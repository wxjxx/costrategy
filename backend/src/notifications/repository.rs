use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    TaskAssigned,
    AssigneeChanged,
    DueTomorrow,
    TaskOverdue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NotificationRecord {
    pub id: Uuid,
    pub notification_type: NotificationType,
    pub receiver_id: Uuid,
    pub task_id: Option<Uuid>,
    pub content_summary: String,
    pub status: NotificationStatus,
    pub failure_reason: Option<String>,
    pub sent_at: DateTime<Utc>,
    pub dedupe_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNotificationRecord {
    pub notification_type: NotificationType,
    pub receiver_id: Uuid,
    pub task_id: Option<Uuid>,
    pub content_summary: String,
    pub status: NotificationStatus,
    pub failure_reason: Option<String>,
    pub dedupe_date: Option<NaiveDate>,
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationRepositoryError {
    #[error("notification validation failed")]
    Validation,
    #[error("database operation failed")]
    Database,
}

#[async_trait]
pub trait NotificationRepository: Clone + Send + Sync + 'static {
    async fn create_record(
        &self,
        record: NewNotificationRecord,
    ) -> Result<NotificationRecord, NotificationRepositoryError>;

    async fn list_records(&self) -> Result<Vec<NotificationRecord>, NotificationRepositoryError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryNotificationRepository {
    inner: Arc<Mutex<Vec<NotificationRecord>>>,
}

#[derive(Debug, Clone)]
pub struct SqlxNotificationRepository {
    pool: PgPool,
}

impl SqlxNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NotificationRepository for MemoryNotificationRepository {
    async fn create_record(
        &self,
        record: NewNotificationRecord,
    ) -> Result<NotificationRecord, NotificationRepositoryError> {
        validate_new_record(&record)?;
        let stored = NotificationRecord {
            id: Uuid::new_v4(),
            notification_type: record.notification_type,
            receiver_id: record.receiver_id,
            task_id: record.task_id,
            content_summary: record.content_summary,
            status: record.status,
            failure_reason: record.failure_reason,
            sent_at: Utc::now(),
            dedupe_date: record.dedupe_date,
        };
        self.inner
            .lock()
            .expect("memory notification repository lock")
            .push(stored.clone());
        Ok(stored)
    }

    async fn list_records(&self) -> Result<Vec<NotificationRecord>, NotificationRepositoryError> {
        let mut records = self
            .inner
            .lock()
            .expect("memory notification repository lock")
            .clone();
        records.sort_by(|left, right| right.sent_at.cmp(&left.sent_at));
        Ok(records)
    }
}

#[async_trait]
impl NotificationRepository for SqlxNotificationRepository {
    async fn create_record(
        &self,
        record: NewNotificationRecord,
    ) -> Result<NotificationRecord, NotificationRepositoryError> {
        validate_new_record(&record)?;
        let row = sqlx::query(
            "insert into notification_records (
                id, notification_type, receiver_id, task_id, content_summary, status,
                failure_reason, dedupe_date
             )
             values ($1, $2, $3, $4, $5, $6, $7, $8)
             returning id, notification_type, receiver_id, task_id, content_summary, status,
                       failure_reason, sent_at, dedupe_date",
        )
        .bind(Uuid::new_v4())
        .bind(record.notification_type.as_str())
        .bind(record.receiver_id)
        .bind(record.task_id)
        .bind(record.content_summary)
        .bind(record.status.as_str())
        .bind(record.failure_reason)
        .bind(record.dedupe_date)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| NotificationRepositoryError::Database)?;

        row_to_record(row)
    }

    async fn list_records(&self) -> Result<Vec<NotificationRecord>, NotificationRepositoryError> {
        sqlx::query(
            "select id, notification_type, receiver_id, task_id, content_summary, status,
                    failure_reason, sent_at, dedupe_date
             from notification_records
             order by sent_at desc",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| NotificationRepositoryError::Database)?
        .into_iter()
        .map(row_to_record)
        .collect()
    }
}

impl NotificationType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TaskAssigned => "task_assigned",
            Self::AssigneeChanged => "assignee_changed",
            Self::DueTomorrow => "due_tomorrow",
            Self::TaskOverdue => "task_overdue",
        }
    }
}

impl Serialize for NotificationType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl NotificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

impl Serialize for NotificationStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl FromStr for NotificationType {
    type Err = NotificationRepositoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "task_assigned" => Ok(Self::TaskAssigned),
            "assignee_changed" => Ok(Self::AssigneeChanged),
            "due_tomorrow" => Ok(Self::DueTomorrow),
            "task_overdue" => Ok(Self::TaskOverdue),
            _ => Err(NotificationRepositoryError::Database),
        }
    }
}

impl FromStr for NotificationStatus {
    type Err = NotificationRepositoryError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            _ => Err(NotificationRepositoryError::Database),
        }
    }
}

fn validate_new_record(record: &NewNotificationRecord) -> Result<(), NotificationRepositoryError> {
    if record.content_summary.trim().is_empty() {
        Err(NotificationRepositoryError::Validation)
    } else {
        Ok(())
    }
}

fn row_to_record(
    row: sqlx::postgres::PgRow,
) -> Result<NotificationRecord, NotificationRepositoryError> {
    let notification_type: String = row
        .try_get("notification_type")
        .map_err(|_| NotificationRepositoryError::Database)?;
    let status: String = row
        .try_get("status")
        .map_err(|_| NotificationRepositoryError::Database)?;

    Ok(NotificationRecord {
        id: row
            .try_get("id")
            .map_err(|_| NotificationRepositoryError::Database)?,
        notification_type: notification_type.parse()?,
        receiver_id: row
            .try_get("receiver_id")
            .map_err(|_| NotificationRepositoryError::Database)?,
        task_id: row
            .try_get("task_id")
            .map_err(|_| NotificationRepositoryError::Database)?,
        content_summary: row
            .try_get("content_summary")
            .map_err(|_| NotificationRepositoryError::Database)?,
        status: status.parse()?,
        failure_reason: row
            .try_get("failure_reason")
            .map_err(|_| NotificationRepositoryError::Database)?,
        sent_at: row
            .try_get("sent_at")
            .map_err(|_| NotificationRepositoryError::Database)?,
        dedupe_date: row
            .try_get("dedupe_date")
            .map_err(|_| NotificationRepositoryError::Database)?,
    })
}
