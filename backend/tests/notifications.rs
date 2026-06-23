use chrono::NaiveDate;
use costrategy_backend::auth::UserRole;
use costrategy_backend::dingtalk::MockDingTalkClient;
use costrategy_backend::notifications::{
    MemoryNotificationRepository, NotificationRepository, NotificationStatus, NotificationType,
    ReminderNotificationService, TaskNotificationService,
};
use costrategy_backend::tasks::{
    CreateTask, MemoryTaskRepository, Task, TaskPriority, TaskRepository, TaskStatus,
};
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
    assert!(!sent[0].message.contains("进入任务详情"));
    assert!(!sent[0].message.contains("/tasks/"));

    let records = notifications.list_records().await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].notification_type, NotificationType::TaskAssigned);
    assert_eq!(records[0].status, NotificationStatus::Success);
    assert_eq!(records[0].receiver_id, assignee.id);
    assert!(records[0].failure_reason.is_none());
    let record_json = serde_json::to_value(&records[0]).unwrap();
    assert_eq!(
        record_json["jump_url"],
        format!("/tasks/{}", records[0].task_id.unwrap())
    );
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
        Some("dingtalk notification failed: notify failed")
    );
}

#[tokio::test]
async fn due_tomorrow_reminder_sends_once_per_task_receiver_and_date() {
    let today = NaiveDate::from_ymd_opt(2026, 6, 9).unwrap();
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
    let tasks = MemoryTaskRepository::default();
    let due_task = tasks
        .create_task(CreateTask {
            project_id: Uuid::new_v4(),
            title: "明天交付".to_string(),
            assignee_id: assignee.id,
            assignee_ids: vec![assignee.id],
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            start_date: today,
            due_date: today.succ_opt().unwrap(),
            description_json: json!({"type": "doc", "content": []}),
            creator_id: assignee.id,
        })
        .await
        .unwrap();
    tasks
        .create_task(CreateTask {
            project_id: Uuid::new_v4(),
            title: "已完成任务".to_string(),
            assignee_id: assignee.id,
            assignee_ids: vec![assignee.id],
            status: TaskStatus::Done,
            priority: TaskPriority::High,
            start_date: today,
            due_date: today.succ_opt().unwrap(),
            description_json: json!({"type": "doc", "content": []}),
            creator_id: assignee.id,
        })
        .await
        .unwrap();
    let dingtalk = MockDingTalkClient::default();
    let notifications = MemoryNotificationRepository::default();
    let service =
        ReminderNotificationService::new(dingtalk.clone(), users, notifications.clone(), tasks);

    service.notify_due_tomorrow(today).await.unwrap();
    service.notify_due_tomorrow(today).await.unwrap();

    let sent = dingtalk.sent_notifications();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].message.contains("明天交付"));
    assert!(sent[0].message.contains("截止日期：2026-06-10"));

    let records = notifications.list_records().await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].notification_type, NotificationType::DueTomorrow);
    assert_eq!(records[0].receiver_id, assignee.id);
    assert_eq!(records[0].task_id, Some(due_task.id));
    assert_eq!(records[0].dedupe_date, Some(today));
}

#[tokio::test]
async fn overdue_reminder_sends_to_assignee_and_project_owner_once_per_date() {
    let today = NaiveDate::from_ymd_opt(2026, 6, 9).unwrap();
    let users = MemoryUserRepository::default();
    let assignee = users
        .insert_user(NewUser {
            dingtalk_user_id: "assignee-ding-id".to_string(),
            union_id: None,
            name: "负责人".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    let owner = users
        .insert_user(NewUser {
            dingtalk_user_id: "owner-ding-id".to_string(),
            union_id: None,
            name: "项目负责人".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Manager,
            status: UserStatus::Active,
        })
        .await;
    let tasks = MemoryTaskRepository::default();
    let overdue_task = tasks
        .insert_task_with_project_owner(
            CreateTask {
                project_id: Uuid::new_v4(),
                title: "逾期交付".to_string(),
                assignee_id: assignee.id,
                assignee_ids: vec![assignee.id],
                status: TaskStatus::InProgress,
                priority: TaskPriority::High,
                start_date: today.pred_opt().unwrap(),
                due_date: today.pred_opt().unwrap(),
                description_json: json!({"type": "doc", "content": []}),
                creator_id: owner.id,
            },
            Some(owner.id),
        )
        .await
        .unwrap();
    let dingtalk = MockDingTalkClient::default();
    let notifications = MemoryNotificationRepository::default();
    let service =
        ReminderNotificationService::new(dingtalk.clone(), users, notifications.clone(), tasks);

    service.notify_overdue(today).await.unwrap();
    service.notify_overdue(today).await.unwrap();

    let sent = dingtalk.sent_notifications();
    assert_eq!(sent.len(), 2);
    assert!(sent
        .iter()
        .any(|notification| notification.receiver_dingtalk_user_id == "assignee-ding-id"));
    assert!(sent
        .iter()
        .any(|notification| notification.receiver_dingtalk_user_id == "owner-ding-id"));
    assert!(sent[0].message.contains("任务延期"));

    let records = notifications.list_records().await.unwrap();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|record| {
        record.notification_type == NotificationType::TaskOverdue
            && record.task_id == Some(overdue_task.id)
            && record.dedupe_date == Some(today)
    }));
}

fn task(assignee_id: Uuid, assignee_name: Option<String>) -> Task {
    Task {
        id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        project_name: Some("项目管理系统".to_string()),
        project_owner_id: None,
        title: "需求文档确认".to_string(),
        assignee_id,
        assignee_name: assignee_name.clone(),
        assignees: vec![costrategy_backend::tasks::TaskAssignee {
            id: assignee_id,
            name: assignee_name,
        }],
        status: TaskStatus::Todo,
        priority: TaskPriority::High,
        start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        due_date: NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
        description_json: json!({"type": "doc", "content": []}),
        creator_id: Uuid::new_v4(),
        creator_name: None,
        updated_at: chrono::Utc::now(),
        archived: false,
        is_overdue: false,
        display_status: "todo".to_string(),
    }
}
