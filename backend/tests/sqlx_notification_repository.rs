use chrono::NaiveDate;
use costrategy_backend::auth::UserRole;
use costrategy_backend::config::AppConfig;
use costrategy_backend::notifications::{
    NewNotificationRecord, NotificationRepository, NotificationStatus, NotificationType,
    SqlxNotificationRepository,
};
use costrategy_backend::users::{NewUser, SqlxUserRepository, UserRepository, UserStatus};
use sqlx::PgPool;
use std::path::Path;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::test]
async fn sqlx_notification_repository_creates_and_lists_records() {
    let pool = test_pool().await;
    MIGRATOR.run(&pool).await.unwrap();
    let users = SqlxUserRepository::new(pool.clone());
    let notifications = SqlxNotificationRepository::new(pool.clone());
    let suffix = Uuid::new_v4().simple().to_string();
    let dingtalk_user_id = format!("notification-receiver-{suffix}");
    let project_code = format!("NOTIFY-PROJECT-{suffix}");
    cleanup(&pool, &dingtalk_user_id, &project_code).await;

    users
        .upsert_synced_user(NewUser {
            dingtalk_user_id: dingtalk_user_id.clone(),
            union_id: None,
            name: "通知接收人".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await
        .unwrap();
    let receiver = users
        .find_by_dingtalk_user_id(&dingtalk_user_id)
        .await
        .unwrap()
        .unwrap();

    let created = notifications
        .create_record(NewNotificationRecord {
            notification_type: NotificationType::TaskAssigned,
            receiver_id: receiver.id,
            task_id: None,
            jump_url: None,
            content_summary: "新任务分配".to_string(),
            status: NotificationStatus::Success,
            failure_reason: None,
            dedupe_date: None,
        })
        .await
        .unwrap();
    assert_eq!(created.notification_type, NotificationType::TaskAssigned);
    assert_eq!(created.status, NotificationStatus::Success);

    let listed = notifications.list_records().await.unwrap();
    let record = listed
        .iter()
        .find(|record| record.id == created.id)
        .unwrap();
    assert_eq!(record.content_summary, "新任务分配");
    assert!(record.read_at.is_none());
    assert_eq!(record.jump_url, None);
    let receiver_records = notifications
        .list_records_for_receiver(receiver.id)
        .await
        .unwrap();
    assert!(receiver_records
        .iter()
        .any(|record| record.id == created.id));
    let read_record = notifications
        .mark_record_read(created.id, receiver.id)
        .await
        .unwrap()
        .unwrap();
    assert!(read_record.read_at.is_some());

    let project_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    sqlx::query(
        "insert into projects (id, code, name, status, updated_at)
         values ($1, $2, $3, 'active', now())",
    )
    .bind(project_id)
    .bind(&project_code)
    .bind("通知测试项目")
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into tasks (
            id, project_id, title, assignee_id, status, priority,
            start_date, due_date, description_json, creator_id, updated_at
         )
         values ($1, $2, '通知去重任务', $3, 'todo', 'medium',
                 $4, $5, '{}'::jsonb, $3, now())",
    )
    .bind(task_id)
    .bind(project_id)
    .bind(receiver.id)
    .bind(NaiveDate::from_ymd_opt(2026, 6, 9).unwrap())
    .bind(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap())
    .execute(&pool)
    .await
    .unwrap();
    let dedupe_date = NaiveDate::from_ymd_opt(2026, 6, 9).unwrap();
    let reminder_record = notifications
        .create_record(NewNotificationRecord {
            notification_type: NotificationType::DueTomorrow,
            receiver_id: receiver.id,
            task_id: Some(task_id),
            jump_url: Some(format!("/tasks/{task_id}")),
            content_summary: "截止前一天提醒".to_string(),
            status: NotificationStatus::Success,
            failure_reason: None,
            dedupe_date: Some(dedupe_date),
        })
        .await
        .unwrap();
    assert_eq!(reminder_record.dedupe_date, Some(dedupe_date));
    assert!(notifications
        .has_record(
            NotificationType::DueTomorrow,
            task_id,
            receiver.id,
            dedupe_date
        )
        .await
        .unwrap());

    let rules = notifications.list_rules().await.unwrap();
    assert_eq!(rules.len(), 4);
    assert!(rules
        .iter()
        .any(|rule| rule.rule_type == NotificationType::TaskOverdue && rule.enabled));

    let updated_rule = notifications
        .update_rule(NotificationType::TaskOverdue, false, receiver.id)
        .await
        .unwrap();
    assert_eq!(updated_rule.rule_type, NotificationType::TaskOverdue);
    assert!(!updated_rule.enabled);
    assert_eq!(updated_rule.updated_by, Some(receiver.id));

    cleanup(&pool, &dingtalk_user_id, &project_code).await;
}

async fn test_pool() -> PgPool {
    let config =
        AppConfig::from_local_env_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_ENV"))
            .unwrap();
    PgPool::connect(&config.database.url()).await.unwrap()
}

async fn cleanup(pool: &PgPool, dingtalk_user_id: &str, project_code: &str) {
    sqlx::query(
        "delete from notification_rules
         where updated_by in (select id from users where dingtalk_user_id = $1)",
    )
    .bind(dingtalk_user_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "delete from notification_records
         where receiver_id in (select id from users where dingtalk_user_id = $1)",
    )
    .bind(dingtalk_user_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "delete from notification_records
         where task_id in (
            select t.id from tasks t
            join projects p on p.id = t.project_id
            where p.code = $1
         )",
    )
    .bind(project_code)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("delete from tasks where project_id in (select id from projects where code = $1)")
        .bind(project_code)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("delete from projects where code = $1")
        .bind(project_code)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("delete from users where dingtalk_user_id = $1")
        .bind(dingtalk_user_id)
        .execute(pool)
        .await
        .unwrap();
}
