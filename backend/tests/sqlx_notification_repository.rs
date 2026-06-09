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
    cleanup(&pool, &dingtalk_user_id).await;

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

    cleanup(&pool, &dingtalk_user_id).await;
}

async fn test_pool() -> PgPool {
    let config =
        AppConfig::from_local_env_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_ENV"))
            .unwrap();
    PgPool::connect(&config.database.url()).await.unwrap()
}

async fn cleanup(pool: &PgPool, dingtalk_user_id: &str) {
    sqlx::query(
        "delete from notification_records
         where receiver_id in (select id from users where dingtalk_user_id = $1)",
    )
    .bind(dingtalk_user_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("delete from users where dingtalk_user_id = $1")
        .bind(dingtalk_user_id)
        .execute(pool)
        .await
        .unwrap();
}
