use crate::dingtalk::DingTalkClient;
use crate::error::{ApiErrorCode, AppError};
use crate::notifications::{
    NewNotificationRecord, NotificationRepository, NotificationStatus, NotificationType,
};
use crate::tasks::{Task, TaskRepository};
use crate::users::{User, UserRepository, UserStatus};
use chrono::NaiveDate;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct TaskNotificationService<C, U, N> {
    dingtalk: C,
    users: U,
    notifications: N,
}

impl<C, U, N> TaskNotificationService<C, U, N>
where
    C: DingTalkClient,
    U: UserRepository,
    N: NotificationRepository,
{
    pub fn new(dingtalk: C, users: U, notifications: N) -> Self {
        Self {
            dingtalk,
            users,
            notifications,
        }
    }

    pub async fn notify_task_assigned(&self, task: &Task) -> Result<(), AppError> {
        self.notify_task_receiver(NotificationType::TaskAssigned, "新任务分配", task)
            .await
    }

    pub async fn notify_assignee_changed(&self, task: &Task) -> Result<(), AppError> {
        self.notify_task_receiver(NotificationType::AssigneeChanged, "负责人变更", task)
            .await
    }

    async fn notify_task_receiver(
        &self,
        notification_type: NotificationType,
        action: &str,
        task: &Task,
    ) -> Result<(), AppError> {
        let mut receiver_ids = task
            .assignees
            .iter()
            .map(|assignee| assignee.id)
            .collect::<Vec<_>>();
        if receiver_ids.is_empty() {
            receiver_ids.push(task.assignee_id);
        }
        receiver_ids.dedup();
        for receiver_id in receiver_ids {
            self.notify_task_receiver_by_id(notification_type, action, task, receiver_id)
                .await?;
        }
        Ok(())
    }

    async fn notify_task_receiver_by_id(
        &self,
        notification_type: NotificationType,
        action: &str,
        task: &Task,
        receiver_id: uuid::Uuid,
    ) -> Result<(), AppError> {
        let Some(receiver) = self
            .users
            .get_user(receiver_id)
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?
        else {
            return Ok(());
        };
        if receiver.status != UserStatus::Active {
            return Ok(());
        }

        let message = build_task_message(action, task);
        let send_result = self
            .dingtalk
            .send_work_notification(&receiver.dingtalk_user_id, &message)
            .await;
        let (status, failure_reason) = match send_result {
            Ok(()) => (NotificationStatus::Success, None),
            Err(_) => (
                NotificationStatus::Failed,
                Some("dingtalk notification failed".to_string()),
            ),
        };

        self.record_notification(
            notification_type,
            &receiver,
            task,
            message,
            status,
            failure_reason,
        )
        .await
    }

    async fn record_notification(
        &self,
        notification_type: NotificationType,
        receiver: &User,
        task: &Task,
        content_summary: String,
        status: NotificationStatus,
        failure_reason: Option<String>,
    ) -> Result<(), AppError> {
        self.notifications
            .create_record(NewNotificationRecord {
                notification_type,
                receiver_id: receiver.id,
                task_id: Some(task.id),
                jump_url: Some(build_task_jump_url(task)),
                content_summary,
                status,
                failure_reason,
                dedupe_date: None,
            })
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ReminderNotificationService<C, U, N, T> {
    dingtalk: C,
    users: U,
    notifications: N,
    tasks: T,
}

impl<C, U, N, T> ReminderNotificationService<C, U, N, T>
where
    C: DingTalkClient,
    U: UserRepository,
    N: NotificationRepository,
    T: TaskRepository,
{
    pub fn new(dingtalk: C, users: U, notifications: N, tasks: T) -> Self {
        Self {
            dingtalk,
            users,
            notifications,
            tasks,
        }
    }

    pub async fn notify_due_tomorrow(&self, local_date: NaiveDate) -> Result<(), AppError> {
        if !self.rule_enabled(NotificationType::DueTomorrow).await? {
            return Ok(());
        }
        let Some(due_date) = local_date.succ_opt() else {
            return Ok(());
        };
        let tasks = self
            .tasks
            .list_tasks_due_on(due_date)
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?;

        for task in tasks {
            for receiver_id in task_receiver_ids(&task) {
                self.notify_task_receiver_once(
                    NotificationType::DueTomorrow,
                    "截止前一天提醒",
                    &task,
                    receiver_id,
                    local_date,
                )
                .await?;
            }
        }
        Ok(())
    }

    pub async fn notify_overdue(&self, local_date: NaiveDate) -> Result<(), AppError> {
        if !self.rule_enabled(NotificationType::TaskOverdue).await? {
            return Ok(());
        }
        let tasks = self
            .tasks
            .list_overdue_tasks(local_date)
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?;

        for task in tasks {
            let mut receiver_ids = task_receiver_ids(&task);
            if let Some(owner_id) = task.project_owner_id {
                receiver_ids.push(owner_id);
            }
            receiver_ids.dedup();
            for receiver_id in receiver_ids {
                self.notify_task_receiver_once(
                    NotificationType::TaskOverdue,
                    "任务延期",
                    &task,
                    receiver_id,
                    local_date,
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn rule_enabled(&self, notification_type: NotificationType) -> Result<bool, AppError> {
        Ok(self
            .notifications
            .list_rules()
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?
            .into_iter()
            .find(|rule| rule.rule_type == notification_type)
            .is_none_or(|rule| rule.enabled))
    }

    async fn notify_task_receiver_once(
        &self,
        notification_type: NotificationType,
        action: &str,
        task: &Task,
        receiver_id: uuid::Uuid,
        dedupe_date: NaiveDate,
    ) -> Result<(), AppError> {
        let Some(receiver) = self
            .users
            .get_user(receiver_id)
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?
        else {
            return Ok(());
        };
        if receiver.status != UserStatus::Active {
            return Ok(());
        }

        let already_sent = self
            .notifications
            .has_record(notification_type, task.id, receiver.id, dedupe_date)
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?;
        if already_sent {
            return Ok(());
        }

        let message = build_task_message(action, task);
        let send_result = self
            .dingtalk
            .send_work_notification(&receiver.dingtalk_user_id, &message)
            .await;
        let (status, failure_reason) = match send_result {
            Ok(()) => (NotificationStatus::Success, None),
            Err(_) => (
                NotificationStatus::Failed,
                Some("dingtalk notification failed".to_string()),
            ),
        };

        self.notifications
            .create_record(NewNotificationRecord {
                notification_type,
                receiver_id: receiver.id,
                task_id: Some(task.id),
                jump_url: Some(build_task_jump_url(&task)),
                content_summary: message,
                status,
                failure_reason,
                dedupe_date: Some(dedupe_date),
            })
            .await
            .map_err(|_| AppError::internal(ApiErrorCode::DatabaseError))?;
        Ok(())
    }
}

fn task_receiver_ids(task: &Task) -> Vec<uuid::Uuid> {
    let mut seen = HashSet::new();
    let mut receiver_ids = task
        .assignees
        .iter()
        .map(|assignee| assignee.id)
        .filter(|id| seen.insert(*id))
        .collect::<Vec<_>>();
    if receiver_ids.is_empty() {
        receiver_ids.push(task.assignee_id);
    }
    receiver_ids
}

fn build_task_message(action: &str, task: &Task) -> String {
    let project_label = task
        .project_name
        .clone()
        .unwrap_or_else(|| task.project_id.to_string());
    format!(
        "{action}\n任务：{}\n项目：{}\n截止日期：{}",
        task.title, project_label, task.due_date
    )
}

fn build_task_jump_url(task: &Task) -> String {
    format!("/tasks/{}", task.id)
}
