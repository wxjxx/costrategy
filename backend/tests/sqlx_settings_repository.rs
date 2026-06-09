use costrategy_backend::auth::UserRole;
use costrategy_backend::config::AppConfig;
use costrategy_backend::settings::{SettingsRepository, SettingsUpdate, SqlxSettingsRepository};
use costrategy_backend::users::{NewUser, SqlxUserRepository, UserRepository, UserStatus};
use sqlx::PgPool;
use std::path::Path;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::test]
async fn sqlx_settings_repository_upserts_masked_sensitive_values() {
    let pool = test_pool().await;
    MIGRATOR.run(&pool).await.unwrap();
    let users = SqlxUserRepository::new(pool.clone());
    let settings = SqlxSettingsRepository::new(pool.clone());
    let suffix = Uuid::new_v4().simple().to_string();
    let dingtalk_user_id = format!("settings-admin-{suffix}");
    cleanup(&pool, &dingtalk_user_id).await;

    users
        .upsert_synced_user(NewUser {
            dingtalk_user_id: dingtalk_user_id.clone(),
            union_id: None,
            name: "系统设置管理员".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Admin,
            status: UserStatus::Active,
        })
        .await
        .unwrap();
    let admin = users
        .find_by_dingtalk_user_id(&dingtalk_user_id)
        .await
        .unwrap()
        .unwrap();

    settings
        .upsert_settings(vec![
            SettingsUpdate {
                key: "dingtalk.corp_id".to_string(),
                value: "ding-corp".to_string(),
                updated_by: admin.id,
            },
            SettingsUpdate {
                key: "dingtalk.client_secret".to_string(),
                value: "super-secret-value".to_string(),
                updated_by: admin.id,
            },
        ])
        .await
        .unwrap();

    let listed = settings.list_settings().await.unwrap();
    let client_secret = listed
        .iter()
        .find(|setting| setting.key == "dingtalk.client_secret")
        .unwrap();
    assert_eq!(client_secret.value_masked.as_deref(), Some("su***ue"));
    assert_ne!(
        client_secret.value_encrypted.as_deref(),
        Some("super-secret-value")
    );
    assert!(client_secret
        .value_encrypted
        .as_deref()
        .unwrap()
        .starts_with("v1:"));

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
        "delete from system_settings
         where updated_by in (select id from users where dingtalk_user_id = $1)",
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
