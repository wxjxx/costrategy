use chrono::NaiveDate;
use costrategy_backend::auth::UserRole;
use costrategy_backend::dingtalk::MockDingTalkClient;
use costrategy_backend::notifications::{
    MemoryNotificationRepository, NotificationRepository, NotificationStatus, NotificationType,
    TaskNotificationService,
};
use costrategy_backend::tasks::{Task, TaskPriority, TaskStatus};
use costrategy_backend::users::{MemoryUserRepository, NewUser, UserStatus};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn task_assigned_notification_sends_dingtalk_and_records_success() {
    let users = MemoryUserRepository::default();
    let assignee = users
        .insert_user(NewUser {
            dingtalk_user_id: "employee-ding-id".to_string(),
            union_id: None,
            name: "员工".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    let dingtalk = MockDingTalkClient::default();
    let notifications = MemoryNotificationRepository::default();
    let service = TaskNotificationService::new(dingtalk.clone(), users, notifications.clone());

    service
        .notify_task_assigned(&task(assignee.id, Some("员工".to_string())))
        .await
        .expect("notification should not fail business flow");

    let sent = dingtalk.sent_notifications();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].receiver_dingtalk_user_id, "employee-ding-id");
    assert!(sent[0].message.contains("需求文档确认"));
    assert!(sent[0].message.contains("项目管理系统"));
    assert!(sent[0].message.contains("2026-06-10"));
    assert!(sent[0].message.contains("/workbench?task_id="));

    let records = notifications.list_records().await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].notification_type, NotificationType::TaskAssigned);
    assert_eq!(records[0].status, NotificationStatus::Success);
    assert_eq!(records[0].receiver_id, assignee.id);
    assert!(records[0].failure_reason.is_none());
}

#[tokio::test]
async fn failed_dingtalk_notification_is_recorded_without_failing_business_flow() {
    let users = MemoryUserRepository::default();
    let assignee = users
        .insert_user(NewUser {
            dingtalk_user_id: "employee-ding-id".to_string(),
            union_id: None,
            name: "员工".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    let dingtalk = MockDingTalkClient::default().with_notification_failure();
    let notifications = MemoryNotificationRepository::default();
    let service = TaskNotificationService::new(dingtalk, users, notifications.clone());

    service
        .notify_task_assigned(&task(assignee.id, Some("员工".to_string())))
        .await
        .expect("notification failure should be recorded but not fail task creation");

    let records = notifications.list_records().await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].notification_type, NotificationType::TaskAssigned);
    assert_eq!(records[0].status, NotificationStatus::Failed);
    assert_eq!(
        records[0].failure_reason.as_deref(),
        Some("dingtalk notification failed")
    );
}

fn task(assignee_id: Uuid, assignee_name: Option<String>) -> Task {
    Task {
        id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        project_name: Some("项目管理系统".to_string()),
        title: "需求文档确认".to_string(),
        assignee_id,
        assignee_name,
        status: TaskStatus::Todo,
        priority: TaskPriority::High,
        start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        due_date: NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
        description_json: json!({"type": "doc", "content": []}),
        creator_id: Uuid::new_v4(),
        archived: false,
        is_overdue: false,
        display_status: "todo".to_string(),
    }
}
