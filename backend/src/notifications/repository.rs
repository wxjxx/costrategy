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
    pub jump_url: Option<String>,
    pub content_summary: String,
    pub status: NotificationStatus,
    pub failure_reason: Option<String>,
    pub sent_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub dedupe_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NotificationRule {
    pub rule_type: NotificationType,
    pub enabled: bool,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNotificationRecord {
    pub notification_type: NotificationType,
    pub receiver_id: Uuid,
    pub task_id: Option<Uuid>,
    pub jump_url: Option<String>,
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

    async fn list_records_for_receiver(
        &self,
        receiver_id: Uuid,
    ) -> Result<Vec<NotificationRecord>, NotificationRepositoryError>;

    async fn mark_record_read(
        &self,
        record_id: Uuid,
        receiver_id: Uuid,
    ) -> Result<Option<NotificationRecord>, NotificationRepositoryError>;

    async fn has_record(
        &self,
        notification_type: NotificationType,
        task_id: Uuid,
        receiver_id: Uuid,
        dedupe_date: NaiveDate,
    ) -> Result<bool, NotificationRepositoryError>;

    async fn list_rules(&self) -> Result<Vec<NotificationRule>, NotificationRepositoryError>;

    async fn update_rule(
        &self,
        rule_type: NotificationType,
        enabled: bool,
        updated_by: Uuid,
    ) -> Result<NotificationRule, NotificationRepositoryError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryNotificationRepository {
    inner: Arc<Mutex<MemoryNotificationState>>,
}

#[derive(Debug, Default)]
struct MemoryNotificationState {
    records: Vec<NotificationRecord>,
    rules: Vec<NotificationRule>,
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
            jump_url: record.jump_url,
            content_summary: record.content_summary,
            status: record.status,
            failure_reason: record.failure_reason,
            sent_at: Utc::now(),
            read_at: None,
            dedupe_date: record.dedupe_date,
        };
        self.inner
            .lock()
            .expect("memory notification repository lock")
            .records
            .push(stored.clone());
        Ok(stored)
    }

    async fn list_records(&self) -> Result<Vec<NotificationRecord>, NotificationRepositoryError> {
        let mut records = self
            .inner
            .lock()
            .expect("memory notification repository lock")
            .records
            .clone();
        records.sort_by(|left, right| right.sent_at.cmp(&left.sent_at));
        Ok(records)
    }

    async fn list_records_for_receiver(
        &self,
        receiver_id: Uuid,
    ) -> Result<Vec<NotificationRecord>, NotificationRepositoryError> {
        let mut records = self
            .inner
            .lock()
            .expect("memory notification repository lock")
            .records
            .iter()
            .filter(|record| record.receiver_id == receiver_id)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| right.sent_at.cmp(&left.sent_at));
        Ok(records)
    }

    async fn mark_record_read(
        &self,
        record_id: Uuid,
        receiver_id: Uuid,
    ) -> Result<Option<NotificationRecord>, NotificationRepositoryError> {
        let mut state = self
            .inner
            .lock()
            .expect("memory notification repository lock");
        let Some(record) = state
            .records
            .iter_mut()
            .find(|record| record.id == record_id && record.receiver_id == receiver_id)
        else {
            return Ok(None);
        };
        if record.read_at.is_none() {
            record.read_at = Some(Utc::now());
        }
        Ok(Some(record.clone()))
    }

    async fn has_record(
        &self,
        notification_type: NotificationType,
        task_id: Uuid,
        receiver_id: Uuid,
        dedupe_date: NaiveDate,
    ) -> Result<bool, NotificationRepositoryError> {
        Ok(self
            .inner
            .lock()
            .expect("memory notification repository lock")
            .records
            .iter()
            .any(|record| {
                record.notification_type == notification_type
                    && record.task_id == Some(task_id)
                    && record.receiver_id == receiver_id
                    && record.dedupe_date == Some(dedupe_date)
            }))
    }

    async fn list_rules(&self) -> Result<Vec<NotificationRule>, NotificationRepositoryError> {
        let state = self
            .inner
            .lock()
            .expect("memory notification repository lock");
        Ok(default_notification_types()
            .into_iter()
            .map(|rule_type| {
                state
                    .rules
                    .iter()
                    .find(|rule| rule.rule_type == rule_type)
                    .cloned()
                    .unwrap_or(NotificationRule {
                        rule_type,
                        enabled: true,
                        updated_by: None,
                        updated_at: None,
                    })
            })
            .collect())
    }

    async fn update_rule(
        &self,
        rule_type: NotificationType,
        enabled: bool,
        updated_by: Uuid,
    ) -> Result<NotificationRule, NotificationRepositoryError> {
        let mut state = self
            .inner
            .lock()
            .expect("memory notification repository lock");
        let updated = NotificationRule {
            rule_type,
            enabled,
            updated_by: Some(updated_by),
            updated_at: Some(Utc::now()),
        };

        if let Some(existing) = state
            .rules
            .iter_mut()
            .find(|rule| rule.rule_type == rule_type)
        {
            *existing = updated.clone();
        } else {
            state.rules.push(updated.clone());
        }

        Ok(updated)
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
                id, notification_type, receiver_id, task_id, jump_url, content_summary, status,
                failure_reason, dedupe_date
             )
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             returning id, notification_type, receiver_id, task_id, jump_url, content_summary,
                       status, failure_reason, sent_at, read_at, dedupe_date",
        )
        .bind(Uuid::new_v4())
        .bind(record.notification_type.as_str())
        .bind(record.receiver_id)
        .bind(record.task_id)
        .bind(record.jump_url)
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
            "select id, notification_type, receiver_id, task_id, jump_url, content_summary, status,
                    failure_reason, sent_at, read_at, dedupe_date
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

    async fn list_records_for_receiver(
        &self,
        receiver_id: Uuid,
    ) -> Result<Vec<NotificationRecord>, NotificationRepositoryError> {
        sqlx::query(
            "select id, notification_type, receiver_id, task_id, jump_url, content_summary, status,
                    failure_reason, sent_at, read_at, dedupe_date
             from notification_records
             where receiver_id = $1
             order by sent_at desc",
        )
        .bind(receiver_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| NotificationRepositoryError::Database)?
        .into_iter()
        .map(row_to_record)
        .collect()
    }

    async fn mark_record_read(
        &self,
        record_id: Uuid,
        receiver_id: Uuid,
    ) -> Result<Option<NotificationRecord>, NotificationRepositoryError> {
        let row = sqlx::query(
            "update notification_records
             set read_at = coalesce(read_at, now())
             where id = $1 and receiver_id = $2
             returning id, notification_type, receiver_id, task_id, jump_url, content_summary,
                       status, failure_reason, sent_at, read_at, dedupe_date",
        )
        .bind(record_id)
        .bind(receiver_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| NotificationRepositoryError::Database)?;

        row.map(row_to_record).transpose()
    }

    async fn has_record(
        &self,
        notification_type: NotificationType,
        task_id: Uuid,
        receiver_id: Uuid,
        dedupe_date: NaiveDate,
    ) -> Result<bool, NotificationRepositoryError> {
        sqlx::query_scalar(
            "select exists(
                select 1
                from notification_records
                where notification_type = $1
                  and task_id = $2
                  and receiver_id = $3
                  and dedupe_date = $4
            )",
        )
        .bind(notification_type.as_str())
        .bind(task_id)
        .bind(receiver_id)
        .bind(dedupe_date)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| NotificationRepositoryError::Database)
    }

    async fn list_rules(&self) -> Result<Vec<NotificationRule>, NotificationRepositoryError> {
        let rows = sqlx::query(
            "select rule_type, enabled, updated_by, updated_at
             from notification_rules",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| NotificationRepositoryError::Database)?;
        let stored = rows
            .into_iter()
            .map(row_to_rule)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(default_notification_types()
            .into_iter()
            .map(|rule_type| {
                stored
                    .iter()
                    .find(|rule| rule.rule_type == rule_type)
                    .cloned()
                    .unwrap_or(NotificationRule {
                        rule_type,
                        enabled: true,
                        updated_by: None,
                        updated_at: None,
                    })
            })
            .collect())
    }

    async fn update_rule(
        &self,
        rule_type: NotificationType,
        enabled: bool,
        updated_by: Uuid,
    ) -> Result<NotificationRule, NotificationRepositoryError> {
        let row = sqlx::query(
            "insert into notification_rules (id, rule_type, enabled, updated_by, updated_at)
             values ($1, $2, $3, $4, now())
             on conflict (rule_type) do update set
                enabled = excluded.enabled,
                updated_by = excluded.updated_by,
                updated_at = now()
             returning rule_type, enabled, updated_by, updated_at",
        )
        .bind(Uuid::new_v4())
        .bind(rule_type.as_str())
        .bind(enabled)
        .bind(updated_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| NotificationRepositoryError::Database)?;

        row_to_rule(row)
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

fn default_notification_types() -> Vec<NotificationType> {
    vec![
        NotificationType::TaskAssigned,
        NotificationType::AssigneeChanged,
        NotificationType::DueTomorrow,
        NotificationType::TaskOverdue,
    ]
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
        jump_url: row
            .try_get("jump_url")
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
        read_at: row
            .try_get("read_at")
            .map_err(|_| NotificationRepositoryError::Database)?,
        dedupe_date: row
            .try_get("dedupe_date")
            .map_err(|_| NotificationRepositoryError::Database)?,
    })
}

fn row_to_rule(
    row: sqlx::postgres::PgRow,
) -> Result<NotificationRule, NotificationRepositoryError> {
    let rule_type: String = row
        .try_get("rule_type")
        .map_err(|_| NotificationRepositoryError::Database)?;

    Ok(NotificationRule {
        rule_type: rule_type.parse()?,
        enabled: row
            .try_get("enabled")
            .map_err(|_| NotificationRepositoryError::Database)?,
        updated_by: row
            .try_get("updated_by")
            .map_err(|_| NotificationRepositoryError::Database)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|_| NotificationRepositoryError::Database)?,
    })
}
