use crate::dingtalk::DingTalkClient;
use crate::error::{ApiErrorCode, AppError};
use crate::notifications::{
    NewNotificationRecord, NotificationRepository, NotificationStatus, NotificationType,
};
use crate::tasks::Task;
use crate::users::{User, UserRepository, UserStatus};

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
        let Some(receiver) = self
            .users
            .get_user(task.assignee_id)
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

fn build_task_message(action: &str, task: &Task) -> String {
    let project_label = task
        .project_name
        .clone()
        .unwrap_or_else(|| task.project_id.to_string());
    format!(
        "{action}\n任务：{}\n项目：{}\n截止日期：{}\n进入任务详情：/workbench?task_id={}",
        task.title, project_label, task.due_date, task.id
    )
}
