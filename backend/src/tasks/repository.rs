use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Blocked,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSort {
    DueDate,
    Priority,
    Status,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub project_name: Option<String>,
    pub project_owner_id: Option<Uuid>,
    pub title: String,
    pub assignee_id: Uuid,
    pub assignee_name: Option<String>,
    pub assignees: Vec<TaskAssignee>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub start_date: NaiveDate,
    pub due_date: NaiveDate,
    pub description_json: Value,
    pub creator_id: Uuid,
    pub creator_name: Option<String>,
    pub archived: bool,
    pub is_overdue: bool,
    pub display_status: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct TaskAssignee {
    pub id: Uuid,
    pub name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct TaskDetail {
    pub task: Task,
    pub comments: Vec<TaskComment>,
    pub attachments: Vec<TaskAttachmentSummary>,
    pub activity_logs: Vec<TaskActivityLog>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct TaskComment {
    pub id: Uuid,
    pub task_id: Uuid,
    pub author_id: Uuid,
    pub author_name: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct TaskAttachmentSummary {
    pub id: Uuid,
    pub task_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub uploader_id: Uuid,
    pub uploader_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAttachmentRecord {
    pub id: Uuid,
    pub task_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub bucket: String,
    pub object_key: String,
    pub uploader_id: Uuid,
    pub uploader_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct TaskActivityLog {
    pub id: Uuid,
    pub task_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub action: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateTask {
    pub project_id: Uuid,
    pub title: String,
    pub assignee_id: Uuid,
    pub assignee_ids: Vec<Uuid>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub start_date: NaiveDate,
    pub due_date: NaiveDate,
    pub description_json: Value,
    pub creator_id: Uuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateTask {
    pub project_id: Uuid,
    pub title: String,
    pub assignee_id: Uuid,
    pub assignee_ids: Vec<Uuid>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub start_date: NaiveDate,
    pub due_date: NaiveDate,
    pub description_json: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskComment {
    pub task_id: Uuid,
    pub author_id: Uuid,
    pub author_name: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskAttachment {
    pub task_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub bucket: String,
    pub object_key: String,
    pub uploader_id: Uuid,
    pub uploader_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskFilter {
    pub keyword: Option<String>,
    pub project_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub include_archived: bool,
    pub sort: TaskSort,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskRepositoryError {
    #[error("task not found")]
    NotFound,
    #[error("invalid task status transition")]
    InvalidStatusTransition,
    #[error("invalid task date range")]
    DateRangeInvalid,
    #[error("task validation failed")]
    Validation,
    #[error("database operation failed")]
    Database,
}

#[async_trait]
pub trait TaskRepository: Clone + Send + Sync + 'static {
    async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, TaskRepositoryError>;
    async fn list_tasks_due_on(
        &self,
        due_date: NaiveDate,
    ) -> Result<Vec<Task>, TaskRepositoryError>;
    async fn list_overdue_tasks(
        &self,
        local_date: NaiveDate,
    ) -> Result<Vec<Task>, TaskRepositoryError>;
    async fn get_task(&self, id: Uuid) -> Result<Task, TaskRepositoryError>;
    async fn get_task_detail(&self, id: Uuid) -> Result<TaskDetail, TaskRepositoryError>;
    async fn create_task(&self, task: CreateTask) -> Result<Task, TaskRepositoryError>;
    async fn create_comment(
        &self,
        comment: CreateTaskComment,
    ) -> Result<TaskComment, TaskRepositoryError>;
    async fn create_attachment(
        &self,
        attachment: CreateTaskAttachment,
    ) -> Result<TaskAttachmentSummary, TaskRepositoryError>;
    async fn get_attachment(
        &self,
        task_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<TaskAttachmentRecord, TaskRepositoryError>;
    async fn delete_attachment(
        &self,
        task_id: Uuid,
        attachment_id: Uuid,
        actor_id: Uuid,
    ) -> Result<TaskAttachmentSummary, TaskRepositoryError>;
    async fn update_task(
        &self,
        id: Uuid,
        actor_id: Uuid,
        task: UpdateTask,
    ) -> Result<Task, TaskRepositoryError>;
    async fn update_task_status(
        &self,
        id: Uuid,
        actor_id: Uuid,
        status: TaskStatus,
    ) -> Result<Task, TaskRepositoryError>;
    async fn archive_task(&self, id: Uuid, actor_id: Uuid) -> Result<Task, TaskRepositoryError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryTaskRepository {
    inner: Arc<Mutex<MemoryTaskState>>,
}

#[derive(Debug, Default)]
struct MemoryTaskState {
    tasks: HashMap<Uuid, StoredTask>,
    comments: Vec<StoredTaskComment>,
    attachments: Vec<StoredTaskAttachment>,
    activity_logs: Vec<MemoryActivityLog>,
}

#[derive(Debug, Clone, PartialEq)]
struct StoredTask {
    id: Uuid,
    project_id: Uuid,
    project_owner_id: Option<Uuid>,
    title: String,
    assignee_id: Uuid,
    assignee_ids: Vec<Uuid>,
    status: TaskStatus,
    priority: TaskPriority,
    start_date: NaiveDate,
    due_date: NaiveDate,
    description_json: Value,
    creator_id: Uuid,
    archived: bool,
}

#[derive(Debug, Clone)]
struct StoredTaskComment {
    id: Uuid,
    task_id: Uuid,
    author_id: Uuid,
    author_name: Option<String>,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct StoredTaskAttachment {
    id: Uuid,
    task_id: Uuid,
    file_name: String,
    file_size: i64,
    mime_type: Option<String>,
    bucket: String,
    object_key: String,
    uploader_id: Uuid,
    uploader_name: Option<String>,
    created_at: DateTime<Utc>,
    deleted: bool,
}

#[derive(Debug, Clone)]
struct MemoryActivityLog {
    id: Uuid,
    task_id: Uuid,
    actor_id: Option<Uuid>,
    actor_name: Option<String>,
    action: String,
    created_at: DateTime<Utc>,
}

#[async_trait]
impl TaskRepository for MemoryTaskRepository {
    async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, TaskRepositoryError> {
        let mut tasks = self
            .inner
            .lock()
            .expect("memory task repository lock")
            .tasks
            .values()
            .filter(|task| filter.include_archived || !task.archived)
            .filter(|task| matches_filter(task, &filter))
            .cloned()
            .map(Task::from)
            .collect::<Vec<_>>();
        sort_tasks(&mut tasks, filter.sort);
        Ok(tasks)
    }

    async fn list_tasks_due_on(
        &self,
        due_date: NaiveDate,
    ) -> Result<Vec<Task>, TaskRepositoryError> {
        let mut tasks = self
            .inner
            .lock()
            .expect("memory task repository lock")
            .tasks
            .values()
            .filter(|task| {
                !task.archived && task.status != TaskStatus::Done && task.due_date == due_date
            })
            .cloned()
            .map(Task::from)
            .collect::<Vec<_>>();
        tasks.sort_by(|left, right| left.title.cmp(&right.title));
        Ok(tasks)
    }

    async fn list_overdue_tasks(
        &self,
        local_date: NaiveDate,
    ) -> Result<Vec<Task>, TaskRepositoryError> {
        let mut tasks = self
            .inner
            .lock()
            .expect("memory task repository lock")
            .tasks
            .values()
            .filter(|task| {
                !task.archived && task.status != TaskStatus::Done && task.due_date < local_date
            })
            .cloned()
            .map(Task::from)
            .collect::<Vec<_>>();
        tasks.sort_by(|left, right| left.due_date.cmp(&right.due_date));
        Ok(tasks)
    }

    async fn get_task(&self, id: Uuid) -> Result<Task, TaskRepositoryError> {
        self.inner
            .lock()
            .expect("memory task repository lock")
            .tasks
            .get(&id)
            .filter(|task| !task.archived)
            .cloned()
            .map(Task::from)
            .ok_or(TaskRepositoryError::NotFound)
    }

    async fn get_task_detail(&self, id: Uuid) -> Result<TaskDetail, TaskRepositoryError> {
        let state = self.inner.lock().expect("memory task repository lock");
        let task = state
            .tasks
            .get(&id)
            .filter(|task| !task.archived)
            .cloned()
            .map(Task::from)
            .ok_or(TaskRepositoryError::NotFound)?;
        let comments = state
            .comments
            .iter()
            .filter(|comment| comment.task_id == id)
            .cloned()
            .map(TaskComment::from)
            .collect();
        let attachments = state
            .attachments
            .iter()
            .filter(|attachment| attachment.task_id == id && !attachment.deleted)
            .cloned()
            .map(TaskAttachmentSummary::from)
            .collect();
        let activity_logs = state
            .activity_logs
            .iter()
            .filter(|log| log.task_id == id)
            .cloned()
            .map(TaskActivityLog::from)
            .collect();

        Ok(TaskDetail {
            task,
            comments,
            attachments,
            activity_logs,
        })
    }

    async fn create_task(&self, task: CreateTask) -> Result<Task, TaskRepositoryError> {
        validate_date_range(task.start_date, task.due_date)?;
        let stored = StoredTask {
            id: Uuid::new_v4(),
            project_id: task.project_id,
            project_owner_id: None,
            title: task.title,
            assignee_id: task.assignee_id,
            assignee_ids: normalize_assignee_ids(task.assignee_id, task.assignee_ids),
            status: task.status,
            priority: task.priority,
            start_date: task.start_date,
            due_date: task.due_date,
            description_json: task.description_json,
            creator_id: task.creator_id,
            archived: false,
        };
        let mut state = self.inner.lock().expect("memory task repository lock");
        state.activity_logs.push(MemoryActivityLog {
            id: Uuid::new_v4(),
            task_id: stored.id,
            actor_id: Some(stored.creator_id),
            actor_name: None,
            action: "task_created".to_string(),
            created_at: Utc::now(),
        });
        state.tasks.insert(stored.id, stored.clone());
        Ok(stored.into())
    }

    async fn create_comment(
        &self,
        comment: CreateTaskComment,
    ) -> Result<TaskComment, TaskRepositoryError> {
        let content = normalize_comment_content(&comment.content)?;
        let mut state = self.inner.lock().expect("memory task repository lock");
        if state
            .tasks
            .get(&comment.task_id)
            .filter(|task| !task.archived)
            .is_none()
        {
            return Err(TaskRepositoryError::NotFound);
        }

        let stored = StoredTaskComment {
            id: Uuid::new_v4(),
            task_id: comment.task_id,
            author_id: comment.author_id,
            author_name: comment.author_name,
            content,
            created_at: Utc::now(),
        };
        state.comments.push(stored.clone());
        state.activity_logs.push(MemoryActivityLog {
            id: Uuid::new_v4(),
            task_id: stored.task_id,
            actor_id: Some(stored.author_id),
            actor_name: stored.author_name.clone(),
            action: "comment_created".to_string(),
            created_at: Utc::now(),
        });

        Ok(stored.into())
    }

    async fn create_attachment(
        &self,
        attachment: CreateTaskAttachment,
    ) -> Result<TaskAttachmentSummary, TaskRepositoryError> {
        validate_attachment_metadata(&attachment)?;
        let mut state = self.inner.lock().expect("memory task repository lock");
        if state
            .tasks
            .get(&attachment.task_id)
            .filter(|task| !task.archived)
            .is_none()
        {
            return Err(TaskRepositoryError::NotFound);
        }

        let stored = StoredTaskAttachment {
            id: Uuid::new_v4(),
            task_id: attachment.task_id,
            file_name: attachment.file_name,
            file_size: attachment.file_size,
            mime_type: attachment.mime_type,
            bucket: attachment.bucket,
            object_key: attachment.object_key,
            uploader_id: attachment.uploader_id,
            uploader_name: attachment.uploader_name,
            created_at: Utc::now(),
            deleted: false,
        };
        state.attachments.push(stored.clone());
        state.activity_logs.push(MemoryActivityLog {
            id: Uuid::new_v4(),
            task_id: stored.task_id,
            actor_id: Some(stored.uploader_id),
            actor_name: stored.uploader_name.clone(),
            action: "attachment_uploaded".to_string(),
            created_at: Utc::now(),
        });

        Ok(stored.into())
    }

    async fn get_attachment(
        &self,
        task_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<TaskAttachmentRecord, TaskRepositoryError> {
        self.inner
            .lock()
            .expect("memory task repository lock")
            .attachments
            .iter()
            .find(|attachment| {
                attachment.task_id == task_id
                    && attachment.id == attachment_id
                    && !attachment.deleted
            })
            .cloned()
            .map(TaskAttachmentRecord::from)
            .ok_or(TaskRepositoryError::NotFound)
    }

    async fn delete_attachment(
        &self,
        task_id: Uuid,
        attachment_id: Uuid,
        actor_id: Uuid,
    ) -> Result<TaskAttachmentSummary, TaskRepositoryError> {
        let mut state = self.inner.lock().expect("memory task repository lock");
        let Some(existing) = state.attachments.iter_mut().find(|attachment| {
            attachment.task_id == task_id && attachment.id == attachment_id && !attachment.deleted
        }) else {
            return Err(TaskRepositoryError::NotFound);
        };
        existing.deleted = true;
        let cloned = existing.clone();
        state.activity_logs.push(MemoryActivityLog {
            id: Uuid::new_v4(),
            task_id,
            actor_id: Some(actor_id),
            actor_name: None,
            action: "attachment_deleted".to_string(),
            created_at: Utc::now(),
        });

        Ok(cloned.into())
    }

    async fn update_task(
        &self,
        id: Uuid,
        actor_id: Uuid,
        task: UpdateTask,
    ) -> Result<Task, TaskRepositoryError> {
        validate_date_range(task.start_date, task.due_date)?;
        let mut state = self.inner.lock().expect("memory task repository lock");
        let Some(existing) = state.tasks.get_mut(&id).filter(|task| !task.archived) else {
            return Err(TaskRepositoryError::NotFound);
        };

        existing.project_id = task.project_id;
        existing.title = task.title;
        existing.assignee_id = task.assignee_id;
        existing.assignee_ids = normalize_assignee_ids(task.assignee_id, task.assignee_ids);
        existing.status = task.status;
        existing.priority = task.priority;
        existing.start_date = task.start_date;
        existing.due_date = task.due_date;
        existing.description_json = task.description_json;
        let cloned = existing.clone();
        state.activity_logs.push(MemoryActivityLog {
            id: Uuid::new_v4(),
            task_id: id,
            actor_id: Some(actor_id),
            actor_name: None,
            action: "schedule_changed".to_string(),
            created_at: Utc::now(),
        });
        Ok(cloned.into())
    }

    async fn update_task_status(
        &self,
        id: Uuid,
        actor_id: Uuid,
        status: TaskStatus,
    ) -> Result<Task, TaskRepositoryError> {
        let mut state = self.inner.lock().expect("memory task repository lock");
        let Some(existing) = state.tasks.get_mut(&id).filter(|task| !task.archived) else {
            return Err(TaskRepositoryError::NotFound);
        };
        ensure_status_transition(existing.status, status)?;
        existing.status = status;
        let cloned = existing.clone();
        state.activity_logs.push(MemoryActivityLog {
            id: Uuid::new_v4(),
            task_id: id,
            actor_id: Some(actor_id),
            actor_name: None,
            action: "status_changed".to_string(),
            created_at: Utc::now(),
        });
        Ok(cloned.into())
    }

    async fn archive_task(&self, id: Uuid, actor_id: Uuid) -> Result<Task, TaskRepositoryError> {
        let mut state = self.inner.lock().expect("memory task repository lock");
        let Some(existing) = state.tasks.get_mut(&id).filter(|task| !task.archived) else {
            return Err(TaskRepositoryError::NotFound);
        };
        existing.archived = true;
        let cloned = existing.clone();
        state.activity_logs.push(MemoryActivityLog {
            id: Uuid::new_v4(),
            task_id: id,
            actor_id: Some(actor_id),
            actor_name: None,
            action: "task_archived".to_string(),
            created_at: Utc::now(),
        });
        Ok(cloned.into())
    }
}

impl MemoryTaskRepository {
    pub async fn activity_logs(&self) -> Vec<(Uuid, Uuid, String)> {
        self.inner
            .lock()
            .expect("memory task repository lock")
            .activity_logs
            .iter()
            .filter_map(|log| {
                log.actor_id
                    .map(|actor_id| (log.task_id, actor_id, log.action.clone()))
            })
            .collect()
    }

    pub async fn insert_task_with_project_owner(
        &self,
        task: CreateTask,
        project_owner_id: Option<Uuid>,
    ) -> Result<Task, TaskRepositoryError> {
        validate_date_range(task.start_date, task.due_date)?;
        let stored = StoredTask {
            id: Uuid::new_v4(),
            project_id: task.project_id,
            project_owner_id,
            title: task.title,
            assignee_id: task.assignee_id,
            assignee_ids: normalize_assignee_ids(task.assignee_id, task.assignee_ids),
            status: task.status,
            priority: task.priority,
            start_date: task.start_date,
            due_date: task.due_date,
            description_json: task.description_json,
            creator_id: task.creator_id,
            archived: false,
        };
        self.inner
            .lock()
            .expect("memory task repository lock")
            .tasks
            .insert(stored.id, stored.clone());
        Ok(stored.into())
    }
}

#[derive(Debug, Clone)]
pub struct SqlxTaskRepository {
    pool: PgPool,
}

impl SqlxTaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn task_select_sql(from_clause: &str) -> String {
    format!(
        "select t.id, t.project_id, p.name as project_name, p.owner_id as project_owner_id, t.title,
                t.assignee_id, u.name as assignee_name,
                (
                    select array_agg(ta.user_id order by ta.position)
                    from task_assignees ta
                    where ta.task_id = t.id
                ) as assignee_ids,
                (
                    select array_agg(assignee_user.name order by ta.position)
                    from task_assignees ta
                    join users assignee_user on assignee_user.id = ta.user_id
                    where ta.task_id = t.id
                ) as assignee_names,
                t.status, t.priority, t.start_date, t.due_date, t.description_json,
                t.creator_id, creator.name as creator_name, t.archived_at
         {from_clause}
         join projects p on p.id = t.project_id
         join users u on u.id = t.assignee_id
         join users creator on creator.id = t.creator_id"
    )
}

#[async_trait]
impl TaskRepository for SqlxTaskRepository {
    async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, TaskRepositoryError> {
        let order_by = task_order_by(filter.sort);
        let query = format!(
            "{} 
             where ($8::bool = true or t.archived_at is null)
               and ($1::uuid is null or t.project_id = $1)
               and ($2::uuid is null or exists (
                    select 1 from task_assignees filter_assignee
                    where filter_assignee.task_id = t.id and filter_assignee.user_id = $2
               ))
               and ($3::text is null or t.status = $3)
               and ($4::text is null or t.priority = $4)
               and ($5::date is null or t.due_date >= $5)
               and ($6::date is null or t.start_date <= $6)
               and ($7::text is null or t.title ilike ('%' || $7 || '%'))
             order by {order_by}",
            task_select_sql("from tasks t"),
        );
        let rows = sqlx::query(&query)
            .bind(filter.project_id)
            .bind(filter.assignee_id)
            .bind(filter.status.map(TaskStatus::as_str))
            .bind(filter.priority.map(TaskPriority::as_str))
            .bind(filter.date_from)
            .bind(filter.date_to)
            .bind(filter.keyword)
            .bind(filter.include_archived)
            .fetch_all(&self.pool)
            .await
            .map_err(|_| TaskRepositoryError::Database)?;

        rows.into_iter().map(row_to_task).collect()
    }

    async fn list_tasks_due_on(
        &self,
        due_date: NaiveDate,
    ) -> Result<Vec<Task>, TaskRepositoryError> {
        let rows = sqlx::query(&format!(
            "{} 
             where t.archived_at is null
               and t.status <> 'done'
               and t.due_date = $1
             order by t.due_date asc, t.title asc",
            task_select_sql("from tasks t"),
        ))
        .bind(due_date)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;

        rows.into_iter().map(row_to_task).collect()
    }

    async fn list_overdue_tasks(
        &self,
        local_date: NaiveDate,
    ) -> Result<Vec<Task>, TaskRepositoryError> {
        let rows = sqlx::query(&format!(
            "{} 
             where t.archived_at is null
               and t.status <> 'done'
               and t.due_date < $1
             order by t.due_date asc, t.title asc",
            task_select_sql("from tasks t"),
        ))
        .bind(local_date)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;

        rows.into_iter().map(row_to_task).collect()
    }

    async fn get_task(&self, id: Uuid) -> Result<Task, TaskRepositoryError> {
        let row = sqlx::query(&format!(
            "{} where t.id = $1 and t.archived_at is null",
            task_select_sql("from tasks t"),
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;

        row.map(row_to_task)
            .transpose()?
            .ok_or(TaskRepositoryError::NotFound)
    }

    async fn get_task_detail(&self, id: Uuid) -> Result<TaskDetail, TaskRepositoryError> {
        let task = self.get_task(id).await?;
        let comments = sqlx::query(
            "select c.id, c.task_id, c.author_id, u.name as author_name, c.content, c.created_at
             from task_comments c
             join users u on u.id = c.author_id
             where c.task_id = $1
             order by c.created_at asc",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| TaskRepositoryError::Database)?
        .into_iter()
        .map(row_to_comment)
        .collect::<Result<Vec<_>, _>>()?;
        let attachments = sqlx::query(
            "select a.id, a.task_id, a.file_name, a.file_size, a.mime_type,
                    a.bucket, a.object_key, a.uploader_id, u.name as uploader_name, a.created_at
             from task_attachments a
             join users u on u.id = a.uploader_id
             where a.task_id = $1 and a.deleted_at is null
             order by a.created_at asc",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| TaskRepositoryError::Database)?
        .into_iter()
        .map(row_to_attachment)
        .collect::<Result<Vec<_>, _>>()?;
        let activity_logs = sqlx::query(
            "select l.id, l.task_id, l.actor_id, u.name as actor_name, l.action, l.created_at
             from task_activity_logs l
             left join users u on u.id = l.actor_id
             where l.task_id = $1
             order by l.created_at asc",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| TaskRepositoryError::Database)?
        .into_iter()
        .map(row_to_activity_log)
        .collect::<Result<Vec<_>, _>>()?;

        Ok(TaskDetail {
            task,
            comments,
            attachments,
            activity_logs,
        })
    }

    async fn create_task(&self, task: CreateTask) -> Result<Task, TaskRepositoryError> {
        validate_date_range(task.start_date, task.due_date)?;
        let assignee_ids = normalize_assignee_ids(task.assignee_id, task.assignee_ids);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let task_id = Uuid::new_v4();
        let row = sqlx::query(&format!(
            "with inserted as (
                insert into tasks (
                    id, project_id, title, assignee_id, status, priority, start_date, due_date,
                    description_json, creator_id, updated_at
                )
                values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
                returning id, project_id, title, assignee_id, status, priority, start_date,
                          due_date, description_json, creator_id, archived_at
             )
             {}",
            task_select_sql("from inserted t"),
        ))
        .bind(task_id)
        .bind(task.project_id)
        .bind(task.title)
        .bind(task.assignee_id)
        .bind(task.status.as_str())
        .bind(task.priority.as_str())
        .bind(task.start_date)
        .bind(task.due_date)
        .bind(task.description_json)
        .bind(task.creator_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;
        replace_task_assignees(&mut tx, task_id, &assignee_ids).await?;
        insert_activity_log(&mut tx, task_id, task.creator_id, "task_created").await?;
        tx.commit()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let created = row_to_task(row)?;
        self.get_task(created.id).await
    }

    async fn create_comment(
        &self,
        comment: CreateTaskComment,
    ) -> Result<TaskComment, TaskRepositoryError> {
        let content = normalize_comment_content(&comment.content)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let comment_id = Uuid::new_v4();
        let row = sqlx::query(
            "with inserted as (
                insert into task_comments (id, task_id, author_id, content)
                select $1, t.id, $3, $4
                from tasks t
                where t.id = $2 and t.archived_at is null
                returning id, task_id, author_id, content, created_at
             )
             select c.id, c.task_id, c.author_id, u.name as author_name, c.content, c.created_at
             from inserted c
             join users u on u.id = c.author_id",
        )
        .bind(comment_id)
        .bind(comment.task_id)
        .bind(comment.author_id)
        .bind(content)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;
        let Some(row) = row else {
            return Err(TaskRepositoryError::NotFound);
        };
        insert_activity_log(
            &mut tx,
            comment.task_id,
            comment.author_id,
            "comment_created",
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;

        row_to_comment(row)
    }

    async fn create_attachment(
        &self,
        attachment: CreateTaskAttachment,
    ) -> Result<TaskAttachmentSummary, TaskRepositoryError> {
        validate_attachment_metadata(&attachment)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let attachment_id = Uuid::new_v4();
        let row = sqlx::query(
            "with inserted as (
                insert into task_attachments (
                    id, task_id, file_name, file_size, mime_type, bucket, object_key, uploader_id
                )
                select $1, t.id, $3, $4, $5, $6, $7, $8
                from tasks t
                where t.id = $2 and t.archived_at is null
                returning id, task_id, file_name, file_size, mime_type, bucket, object_key,
                          uploader_id, created_at
             )
             select a.id, a.task_id, a.file_name, a.file_size, a.mime_type, a.bucket,
                    a.object_key, a.uploader_id, u.name as uploader_name, a.created_at
             from inserted a
             join users u on u.id = a.uploader_id",
        )
        .bind(attachment_id)
        .bind(attachment.task_id)
        .bind(attachment.file_name)
        .bind(attachment.file_size)
        .bind(attachment.mime_type)
        .bind(attachment.bucket)
        .bind(attachment.object_key)
        .bind(attachment.uploader_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;
        let Some(row) = row else {
            return Err(TaskRepositoryError::NotFound);
        };
        insert_activity_log(
            &mut tx,
            attachment.task_id,
            attachment.uploader_id,
            "attachment_uploaded",
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;

        row_to_attachment_record(row).map(TaskAttachmentSummary::from)
    }

    async fn get_attachment(
        &self,
        task_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<TaskAttachmentRecord, TaskRepositoryError> {
        let row = sqlx::query(
            "select a.id, a.task_id, a.file_name, a.file_size, a.mime_type, a.bucket,
                    a.object_key, a.uploader_id, u.name as uploader_name, a.created_at
             from task_attachments a
             join users u on u.id = a.uploader_id
             where a.task_id = $1 and a.id = $2 and a.deleted_at is null",
        )
        .bind(task_id)
        .bind(attachment_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;

        row.map(row_to_attachment_record)
            .transpose()?
            .ok_or(TaskRepositoryError::NotFound)
    }

    async fn delete_attachment(
        &self,
        task_id: Uuid,
        attachment_id: Uuid,
        actor_id: Uuid,
    ) -> Result<TaskAttachmentSummary, TaskRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let row = sqlx::query(
            "with deleted as (
                update task_attachments set deleted_at = now()
                where task_id = $1 and id = $2 and deleted_at is null
                returning id, task_id, file_name, file_size, mime_type, bucket, object_key,
                          uploader_id, created_at
             )
             select a.id, a.task_id, a.file_name, a.file_size, a.mime_type, a.bucket,
                    a.object_key, a.uploader_id, u.name as uploader_name, a.created_at
             from deleted a
             join users u on u.id = a.uploader_id",
        )
        .bind(task_id)
        .bind(attachment_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;
        let Some(row) = row else {
            return Err(TaskRepositoryError::NotFound);
        };
        insert_activity_log(&mut tx, task_id, actor_id, "attachment_deleted").await?;
        tx.commit()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;

        row_to_attachment_record(row).map(TaskAttachmentSummary::from)
    }

    async fn update_task(
        &self,
        id: Uuid,
        actor_id: Uuid,
        task: UpdateTask,
    ) -> Result<Task, TaskRepositoryError> {
        validate_date_range(task.start_date, task.due_date)?;
        let assignee_ids = normalize_assignee_ids(task.assignee_id, task.assignee_ids);
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let row = sqlx::query(&format!(
            "with updated as (
                update tasks set
                    project_id = $2,
                    title = $3,
                    assignee_id = $4,
                    status = $5,
                    priority = $6,
                    start_date = $7,
                    due_date = $8,
                    description_json = $9,
                    updated_at = now()
                where id = $1 and archived_at is null
                returning id, project_id, title, assignee_id, status, priority, start_date,
                          due_date, description_json, creator_id, archived_at
             )
             {}",
            task_select_sql("from updated t"),
        ))
        .bind(id)
        .bind(task.project_id)
        .bind(task.title)
        .bind(task.assignee_id)
        .bind(task.status.as_str())
        .bind(task.priority.as_str())
        .bind(task.start_date)
        .bind(task.due_date)
        .bind(task.description_json)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;
        let Some(row) = row else {
            return Err(TaskRepositoryError::NotFound);
        };
        replace_task_assignees(&mut tx, id, &assignee_ids).await?;
        insert_activity_log(&mut tx, id, actor_id, "schedule_changed").await?;
        tx.commit()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let updated = row_to_task(row)?;
        self.get_task(updated.id).await
    }

    async fn update_task_status(
        &self,
        id: Uuid,
        actor_id: Uuid,
        status: TaskStatus,
    ) -> Result<Task, TaskRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let current_status: Option<String> =
            sqlx::query_scalar("select status from tasks where id = $1 and archived_at is null")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| TaskRepositoryError::Database)?;
        let Some(current_status) = current_status else {
            return Err(TaskRepositoryError::NotFound);
        };
        ensure_status_transition(
            current_status
                .parse()
                .map_err(|_| TaskRepositoryError::Database)?,
            status,
        )?;

        let row = sqlx::query(&format!(
            "with updated as (
                update tasks set status = $2, updated_at = now()
                where id = $1 and archived_at is null
                returning id, project_id, title, assignee_id, status, priority, start_date,
                          due_date, description_json, creator_id, archived_at
             )
             {}",
            task_select_sql("from updated t"),
        ))
        .bind(id)
        .bind(status.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;
        insert_activity_log(&mut tx, id, actor_id, "status_changed").await?;
        tx.commit()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        row_to_task(row)
    }

    async fn archive_task(&self, id: Uuid, actor_id: Uuid) -> Result<Task, TaskRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        let row = sqlx::query(&format!(
            "with archived as (
                update tasks set archived_at = now(), updated_at = now()
                where id = $1 and archived_at is null
                returning id, project_id, title, assignee_id, status, priority, start_date,
                          due_date, description_json, creator_id, archived_at
             )
             {}",
            task_select_sql("from archived t"),
        ))
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;
        let Some(row) = row else {
            return Err(TaskRepositoryError::NotFound);
        };
        insert_activity_log(&mut tx, id, actor_id, "task_archived").await?;
        tx.commit()
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
        row_to_task(row)
    }
}

impl From<StoredTask> for Task {
    fn from(task: StoredTask) -> Self {
        let is_overdue = compute_overdue(task.status, task.due_date);
        Self {
            id: task.id,
            project_id: task.project_id,
            project_name: None,
            project_owner_id: task.project_owner_id,
            title: task.title,
            assignee_id: task.assignee_id,
            assignee_name: None,
            assignees: task
                .assignee_ids
                .into_iter()
                .map(|id| TaskAssignee { id, name: None })
                .collect(),
            status: task.status,
            priority: task.priority,
            start_date: task.start_date,
            due_date: task.due_date,
            description_json: task.description_json,
            creator_id: task.creator_id,
            creator_name: None,
            archived: task.archived,
            is_overdue,
            display_status: display_status(task.status, is_overdue).to_string(),
        }
    }
}

impl From<StoredTaskComment> for TaskComment {
    fn from(comment: StoredTaskComment) -> Self {
        Self {
            id: comment.id,
            task_id: comment.task_id,
            author_id: comment.author_id,
            author_name: comment.author_name,
            content: comment.content,
            created_at: comment.created_at,
        }
    }
}

impl From<StoredTaskAttachment> for TaskAttachmentSummary {
    fn from(attachment: StoredTaskAttachment) -> Self {
        Self {
            id: attachment.id,
            task_id: attachment.task_id,
            file_name: attachment.file_name,
            file_size: attachment.file_size,
            mime_type: attachment.mime_type,
            uploader_id: attachment.uploader_id,
            uploader_name: attachment.uploader_name,
            created_at: attachment.created_at,
        }
    }
}

impl From<StoredTaskAttachment> for TaskAttachmentRecord {
    fn from(attachment: StoredTaskAttachment) -> Self {
        Self {
            id: attachment.id,
            task_id: attachment.task_id,
            file_name: attachment.file_name,
            file_size: attachment.file_size,
            mime_type: attachment.mime_type,
            bucket: attachment.bucket,
            object_key: attachment.object_key,
            uploader_id: attachment.uploader_id,
            uploader_name: attachment.uploader_name,
            created_at: attachment.created_at,
        }
    }
}

impl From<TaskAttachmentRecord> for TaskAttachmentSummary {
    fn from(attachment: TaskAttachmentRecord) -> Self {
        Self {
            id: attachment.id,
            task_id: attachment.task_id,
            file_name: attachment.file_name,
            file_size: attachment.file_size,
            mime_type: attachment.mime_type,
            uploader_id: attachment.uploader_id,
            uploader_name: attachment.uploader_name,
            created_at: attachment.created_at,
        }
    }
}

impl From<MemoryActivityLog> for TaskActivityLog {
    fn from(log: MemoryActivityLog) -> Self {
        Self {
            id: log.id,
            task_id: log.task_id,
            actor_id: log.actor_id,
            actor_name: log.actor_name,
            action: log.action,
            created_at: log.created_at,
        }
    }
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
        }
    }
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl TaskSort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DueDate => "due_date",
            Self::Priority => "priority",
            Self::Status => "status",
        }
    }
}

impl serde::Serialize for TaskStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl serde::Serialize for TaskPriority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for TaskStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl<'de> serde::Deserialize<'de> for TaskPriority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl Default for TaskSort {
    fn default() -> Self {
        Self::DueDate
    }
}

impl FromStr for TaskStatus {
    type Err = TaskStatusParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "todo" => Ok(Self::Todo),
            "in_progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "done" => Ok(Self::Done),
            other => Err(TaskStatusParseError(other.to_string())),
        }
    }
}

impl FromStr for TaskPriority {
    type Err = TaskPriorityParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(TaskPriorityParseError(other.to_string())),
        }
    }
}

impl FromStr for TaskSort {
    type Err = TaskSortParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "due_date" => Ok(Self::DueDate),
            "priority" => Ok(Self::Priority),
            "status" => Ok(Self::Status),
            other => Err(TaskSortParseError(other.to_string())),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown task status: {0}")]
pub struct TaskStatusParseError(String);

#[derive(Debug, thiserror::Error)]
#[error("unknown task priority: {0}")]
pub struct TaskPriorityParseError(String);

#[derive(Debug, thiserror::Error)]
#[error("unknown task sort: {0}")]
pub struct TaskSortParseError(String);

fn validate_date_range(
    start_date: NaiveDate,
    due_date: NaiveDate,
) -> Result<(), TaskRepositoryError> {
    if start_date > due_date {
        Err(TaskRepositoryError::DateRangeInvalid)
    } else {
        Ok(())
    }
}

fn normalize_comment_content(content: &str) -> Result<String, TaskRepositoryError> {
    let trimmed = content.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 2000 {
        return Err(TaskRepositoryError::Validation);
    }

    Ok(trimmed.to_string())
}

fn validate_attachment_metadata(
    attachment: &CreateTaskAttachment,
) -> Result<(), TaskRepositoryError> {
    if attachment.file_name.trim().is_empty()
        || attachment.file_size <= 0
        || attachment.bucket.trim().is_empty()
        || attachment.object_key.trim().is_empty()
    {
        return Err(TaskRepositoryError::Validation);
    }

    Ok(())
}

fn ensure_status_transition(from: TaskStatus, to: TaskStatus) -> Result<(), TaskRepositoryError> {
    if from == to {
        return Ok(());
    }
    let allowed = matches!(
        (from, to),
        (TaskStatus::Todo, TaskStatus::InProgress)
            | (TaskStatus::Todo, TaskStatus::Blocked)
            | (TaskStatus::Todo, TaskStatus::Done)
            | (TaskStatus::InProgress, TaskStatus::Todo)
            | (TaskStatus::InProgress, TaskStatus::Blocked)
            | (TaskStatus::InProgress, TaskStatus::Done)
            | (TaskStatus::Blocked, TaskStatus::Todo)
            | (TaskStatus::Blocked, TaskStatus::InProgress)
            | (TaskStatus::Blocked, TaskStatus::Done)
            | (TaskStatus::Done, TaskStatus::Blocked)
            | (TaskStatus::Done, TaskStatus::InProgress)
    );
    if allowed {
        Ok(())
    } else {
        Err(TaskRepositoryError::InvalidStatusTransition)
    }
}

fn matches_filter(task: &StoredTask, filter: &TaskFilter) -> bool {
    if let Some(keyword) = &filter.keyword {
        if !task.title.contains(keyword) {
            return false;
        }
    }
    if let Some(project_id) = filter.project_id {
        if task.project_id != project_id {
            return false;
        }
    }
    if let Some(assignee_id) = filter.assignee_id {
        if !task.assignee_ids.contains(&assignee_id) {
            return false;
        }
    }
    if let Some(status) = filter.status {
        if task.status != status {
            return false;
        }
    }
    if let Some(priority) = filter.priority {
        if task.priority != priority {
            return false;
        }
    }
    if let Some(date_from) = filter.date_from {
        if task.due_date < date_from {
            return false;
        }
    }
    if let Some(date_to) = filter.date_to {
        if task.start_date > date_to {
            return false;
        }
    }
    true
}

fn sort_tasks(tasks: &mut [Task], sort: TaskSort) {
    tasks.sort_by(|left, right| {
        match sort {
            TaskSort::DueDate => left.due_date.cmp(&right.due_date),
            TaskSort::Priority => priority_rank(right.priority).cmp(&priority_rank(left.priority)),
            TaskSort::Status => status_rank(left.status).cmp(&status_rank(right.status)),
        }
        .then(left.due_date.cmp(&right.due_date))
        .then(left.title.cmp(&right.title))
    });
}

fn task_order_by(sort: TaskSort) -> &'static str {
    match sort {
        TaskSort::DueDate => "t.due_date asc, t.title asc",
        TaskSort::Priority => {
            "case t.priority when 'high' then 1 when 'medium' then 2 else 3 end asc, t.due_date asc, t.title asc"
        }
        TaskSort::Status => {
            "case t.status when 'todo' then 1 when 'in_progress' then 2 when 'blocked' then 3 else 4 end asc, t.due_date asc, t.title asc"
        }
    }
}

fn priority_rank(priority: TaskPriority) -> u8 {
    match priority {
        TaskPriority::Low => 1,
        TaskPriority::Medium => 2,
        TaskPriority::High => 3,
    }
}

fn status_rank(status: TaskStatus) -> u8 {
    match status {
        TaskStatus::Todo => 1,
        TaskStatus::InProgress => 2,
        TaskStatus::Blocked => 3,
        TaskStatus::Done => 4,
    }
}

fn row_to_task(row: sqlx::postgres::PgRow) -> Result<Task, TaskRepositoryError> {
    let status: String = row
        .try_get("status")
        .map_err(|_| TaskRepositoryError::Database)?;
    let priority: String = row
        .try_get("priority")
        .map_err(|_| TaskRepositoryError::Database)?;
    let status = status.parse().map_err(|_| TaskRepositoryError::Database)?;
    let due_date = row
        .try_get("due_date")
        .map_err(|_| TaskRepositoryError::Database)?;
    let is_overdue = compute_overdue(status, due_date);
    let assignee_id: Uuid = row
        .try_get("assignee_id")
        .map_err(|_| TaskRepositoryError::Database)?;
    let assignee_name: Option<String> = row
        .try_get("assignee_name")
        .map_err(|_| TaskRepositoryError::Database)?;
    let assignee_ids = row
        .try_get::<Option<Vec<Uuid>>, _>("assignee_ids")
        .map_err(|_| TaskRepositoryError::Database)?
        .unwrap_or_else(|| vec![assignee_id]);
    let assignee_names = row
        .try_get::<Option<Vec<String>>, _>("assignee_names")
        .map_err(|_| TaskRepositoryError::Database)?
        .unwrap_or_else(|| {
            assignee_name
                .clone()
                .map(|name| vec![name])
                .unwrap_or_default()
        });
    let assignees = assignee_ids
        .iter()
        .enumerate()
        .map(|(index, id)| TaskAssignee {
            id: *id,
            name: assignee_names.get(index).cloned(),
        })
        .collect();

    Ok(Task {
        id: row
            .try_get("id")
            .map_err(|_| TaskRepositoryError::Database)?,
        project_id: row
            .try_get("project_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        project_name: row
            .try_get("project_name")
            .map_err(|_| TaskRepositoryError::Database)?,
        project_owner_id: row
            .try_get("project_owner_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        title: row
            .try_get("title")
            .map_err(|_| TaskRepositoryError::Database)?,
        assignee_id,
        assignee_name,
        assignees,
        status,
        priority: priority
            .parse()
            .map_err(|_| TaskRepositoryError::Database)?,
        start_date: row
            .try_get("start_date")
            .map_err(|_| TaskRepositoryError::Database)?,
        due_date,
        description_json: row
            .try_get("description_json")
            .map_err(|_| TaskRepositoryError::Database)?,
        creator_id: row
            .try_get("creator_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        creator_name: row
            .try_get("creator_name")
            .map_err(|_| TaskRepositoryError::Database)?,
        archived: row
            .try_get::<Option<chrono::DateTime<Utc>>, _>("archived_at")
            .map_err(|_| TaskRepositoryError::Database)?
            .is_some(),
        is_overdue,
        display_status: display_status(status, is_overdue).to_string(),
    })
}

fn normalize_assignee_ids(primary_assignee_id: Uuid, assignee_ids: Vec<Uuid>) -> Vec<Uuid> {
    let mut normalized = if assignee_ids.is_empty() {
        vec![primary_assignee_id]
    } else {
        assignee_ids
    };
    if !normalized.contains(&primary_assignee_id) {
        normalized.insert(0, primary_assignee_id);
    }
    let mut seen = HashSet::new();
    normalized.retain(|assignee_id| seen.insert(*assignee_id));
    normalized
}

async fn replace_task_assignees(
    tx: &mut Transaction<'_, Postgres>,
    task_id: Uuid,
    assignee_ids: &[Uuid],
) -> Result<(), TaskRepositoryError> {
    sqlx::query("delete from task_assignees where task_id = $1")
        .bind(task_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| TaskRepositoryError::Database)?;

    for (position, assignee_id) in assignee_ids.iter().enumerate() {
        sqlx::query("insert into task_assignees (task_id, user_id, position) values ($1, $2, $3)")
            .bind(task_id)
            .bind(*assignee_id)
            .bind(position as i32)
            .execute(&mut **tx)
            .await
            .map_err(|_| TaskRepositoryError::Database)?;
    }
    Ok(())
}

fn row_to_comment(row: sqlx::postgres::PgRow) -> Result<TaskComment, TaskRepositoryError> {
    Ok(TaskComment {
        id: row
            .try_get("id")
            .map_err(|_| TaskRepositoryError::Database)?,
        task_id: row
            .try_get("task_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        author_id: row
            .try_get("author_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        author_name: row
            .try_get("author_name")
            .map_err(|_| TaskRepositoryError::Database)?,
        content: row
            .try_get("content")
            .map_err(|_| TaskRepositoryError::Database)?,
        created_at: row
            .try_get("created_at")
            .map_err(|_| TaskRepositoryError::Database)?,
    })
}

fn row_to_attachment(
    row: sqlx::postgres::PgRow,
) -> Result<TaskAttachmentSummary, TaskRepositoryError> {
    row_to_attachment_record(row).map(TaskAttachmentSummary::from)
}

fn row_to_attachment_record(
    row: sqlx::postgres::PgRow,
) -> Result<TaskAttachmentRecord, TaskRepositoryError> {
    Ok(TaskAttachmentRecord {
        id: row
            .try_get("id")
            .map_err(|_| TaskRepositoryError::Database)?,
        task_id: row
            .try_get("task_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        file_name: row
            .try_get("file_name")
            .map_err(|_| TaskRepositoryError::Database)?,
        file_size: row
            .try_get("file_size")
            .map_err(|_| TaskRepositoryError::Database)?,
        mime_type: row
            .try_get("mime_type")
            .map_err(|_| TaskRepositoryError::Database)?,
        bucket: row
            .try_get("bucket")
            .map_err(|_| TaskRepositoryError::Database)?,
        object_key: row
            .try_get("object_key")
            .map_err(|_| TaskRepositoryError::Database)?,
        uploader_id: row
            .try_get("uploader_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        uploader_name: row
            .try_get("uploader_name")
            .map_err(|_| TaskRepositoryError::Database)?,
        created_at: row
            .try_get("created_at")
            .map_err(|_| TaskRepositoryError::Database)?,
    })
}

fn row_to_activity_log(row: sqlx::postgres::PgRow) -> Result<TaskActivityLog, TaskRepositoryError> {
    Ok(TaskActivityLog {
        id: row
            .try_get("id")
            .map_err(|_| TaskRepositoryError::Database)?,
        task_id: row
            .try_get("task_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        actor_id: row
            .try_get("actor_id")
            .map_err(|_| TaskRepositoryError::Database)?,
        actor_name: row
            .try_get("actor_name")
            .map_err(|_| TaskRepositoryError::Database)?,
        action: row
            .try_get("action")
            .map_err(|_| TaskRepositoryError::Database)?,
        created_at: row
            .try_get("created_at")
            .map_err(|_| TaskRepositoryError::Database)?,
    })
}

async fn insert_activity_log(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    task_id: Uuid,
    actor_id: Uuid,
    action: &'static str,
) -> Result<(), TaskRepositoryError> {
    sqlx::query(
        "insert into task_activity_logs (id, task_id, actor_id, action)
         values ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(task_id)
    .bind(actor_id)
    .bind(action)
    .execute(&mut **tx)
    .await
    .map_err(|_| TaskRepositoryError::Database)?;
    Ok(())
}

fn compute_overdue(_status: TaskStatus, due_date: NaiveDate) -> bool {
    Utc::now().date_naive() > due_date
}

fn display_status(status: TaskStatus, _is_overdue: bool) -> &'static str {
    status.as_str()
}
