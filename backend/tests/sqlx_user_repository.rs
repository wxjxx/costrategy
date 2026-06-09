use costrategy_backend::auth::UserRole;
use costrategy_backend::config::AppConfig;
use costrategy_backend::users::{
    NewDepartment, NewUser, SqlxUserRepository, SyncLogRecord, SyncUserOutcome, UserRepository,
    UserStatus,
};
use sqlx::PgPool;
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::test]
async fn sqlx_user_repository_upserts_contacts_and_sync_log() {
    let pool = test_pool().await;
    MIGRATOR.run(&pool).await.unwrap();
    let repo = SqlxUserRepository::new(pool.clone());
    let suffix = Uuid::new_v4().simple().to_string();
    let user_id = format!("test-ding-{suffix}");
    let department_id = 9_000_000_000_i64;

    cleanup(&pool, &user_id, department_id).await;

    let created = repo
        .upsert_synced_user(NewUser {
            dingtalk_user_id: user_id.clone(),
            union_id: Some(format!("test-union-{suffix}")),
            name: "测试用户".to_string(),
            avatar_url: None,
            mobile: Some("13800000000".to_string()),
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await
        .unwrap();
    assert_eq!(created, SyncUserOutcome::Created);

    let updated = repo
        .upsert_synced_user(NewUser {
            dingtalk_user_id: user_id.clone(),
            union_id: Some(format!("test-union-{suffix}")),
            name: "测试用户更新".to_string(),
            avatar_url: Some("https://example.test/avatar.png".to_string()),
            mobile: Some("13900000000".to_string()),
            role: UserRole::Admin,
            status: UserStatus::Active,
        })
        .await
        .unwrap();
    assert_eq!(updated, SyncUserOutcome::Updated);

    let found = repo
        .find_by_dingtalk_user_id(&user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.name, "测试用户更新");
    assert_eq!(found.role, UserRole::Employee);
    assert_eq!(found.status, UserStatus::Active);

    let listed = repo.list_users().await.unwrap();
    let list_item = listed
        .iter()
        .find(|user| user.dingtalk_user_id == user_id)
        .unwrap();
    assert_eq!(list_item.name, "测试用户更新");
    assert_eq!(list_item.mobile.as_deref(), Some("13900000000"));
    assert_eq!(list_item.role, UserRole::Employee);

    let role_updated = repo
        .update_user_role(found.id, UserRole::Manager)
        .await
        .unwrap();
    assert_eq!(role_updated.role, UserRole::Manager);

    let status_updated = repo
        .update_user_status(found.id, UserStatus::Disabled)
        .await
        .unwrap();
    assert_eq!(status_updated.status, UserStatus::Disabled);

    repo.upsert_department(NewDepartment {
        dingtalk_dept_id: department_id,
        parent_dingtalk_dept_id: None,
        name: "测试部门".to_string(),
        order_no: Some(1),
    })
    .await
    .unwrap();
    repo.replace_department_users(department_id, std::slice::from_ref(&user_id))
        .await
        .unwrap();
    let departments = repo.list_user_departments(found.id).await.unwrap();
    assert_eq!(departments, vec!["测试部门"]);

    let relation_count: i64 = sqlx::query_scalar(
        "select count(*)
         from department_users du
         join departments d on d.id = du.department_id
         join users u on u.id = du.user_id
         where d.dingtalk_dept_id = $1 and u.dingtalk_user_id = $2",
    )
    .bind(department_id)
    .bind(&user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(relation_count, 1);

    let mut active_ids = current_active_dingtalk_user_ids(&pool).await;
    active_ids.insert(user_id.clone());
    assert_eq!(
        repo.disable_users_missing_from_sync(&active_ids)
            .await
            .unwrap(),
        0
    );

    repo.record_sync_log(SyncLogRecord {
        status: "success".to_string(),
        created_users: 1,
        updated_users: 1,
        disabled_users: 0,
        failure_reason: None,
    })
    .await
    .unwrap();
    let sync_logs = repo.list_sync_logs().await.unwrap();
    let sync_log = sync_logs
        .iter()
        .find(|log| log.created_users == 1 && log.updated_users == 1)
        .unwrap();
    assert_eq!(sync_log.status, "success");
    assert_eq!(sync_log.disabled_users, 0);

    cleanup(&pool, &user_id, department_id).await;
}

async fn test_pool() -> PgPool {
    let config =
        AppConfig::from_local_env_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_ENV"))
            .unwrap();
    PgPool::connect(&config.database.url()).await.unwrap()
}

async fn cleanup(pool: &PgPool, user_id: &str, department_id: i64) {
    sqlx::query(
        "delete from department_users
         where user_id in (select id from users where dingtalk_user_id = $1)
            or department_id in (select id from departments where dingtalk_dept_id = $2)",
    )
    .bind(user_id)
    .bind(department_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("delete from users where dingtalk_user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("delete from departments where dingtalk_dept_id = $1")
        .bind(department_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn current_active_dingtalk_user_ids(pool: &PgPool) -> HashSet<String> {
    sqlx::query_scalar::<_, String>("select dingtalk_user_id from users where status = 'active'")
        .fetch_all(pool)
        .await
        .unwrap()
        .into_iter()
        .collect()
}
