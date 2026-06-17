use chrono::NaiveDate;
use costrategy_backend::auth::UserRole;
use costrategy_backend::config::AppConfig;
use costrategy_backend::projects::{
    CreateProject, ProjectRepository, ProjectStatus, SqlxProjectRepository, UpdateProject,
};
use costrategy_backend::users::{NewUser, SqlxUserRepository, UserRepository, UserStatus};
use sqlx::PgPool;
use std::path::Path;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::test]
async fn sqlx_project_repository_crud_and_archive() {
    let pool = test_pool().await;
    MIGRATOR.run(&pool).await.unwrap();
    let repo = SqlxProjectRepository::new(pool.clone());
    let users = SqlxUserRepository::new(pool.clone());
    let code = format!("TEST-PROJECT-{}", Uuid::new_v4().simple());
    let owner_ding = format!("project-owner-{}", Uuid::new_v4().simple());

    cleanup(&pool, &code, &owner_ding).await;
    users
        .upsert_synced_user(NewUser {
            dingtalk_user_id: owner_ding.clone(),
            union_id: None,
            name: "项目负责人".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Manager,
            status: UserStatus::Active,
        })
        .await
        .unwrap();
    let owner = users
        .find_by_dingtalk_user_id(&owner_ding)
        .await
        .unwrap()
        .unwrap();

    let created = repo
        .create_project(CreateProject {
            code: Some(code.clone()),
            name: "测试项目".to_string(),
            owner_id: Some(owner.id),
            description: Some("创建描述".to_string()),
            start_date: Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
            end_date: Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            status: ProjectStatus::Active,
        })
        .await
        .unwrap();
    assert_eq!(created.code, code);
    assert_eq!(created.status, ProjectStatus::Active);
    assert_eq!(created.owner_id, Some(owner.id));

    let updated = repo
        .update_project(
            created.id,
            UpdateProject {
                name: "测试项目更新".to_string(),
                owner_id: None,
                description: Some("更新描述".to_string()),
                start_date: Some(NaiveDate::from_ymd_opt(2026, 6, 2).unwrap()),
                end_date: Some(NaiveDate::from_ymd_opt(2026, 8, 2).unwrap()),
                status: ProjectStatus::Paused,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "测试项目更新");
    assert_eq!(updated.status, ProjectStatus::Paused);
    assert_eq!(updated.owner_id, None);

    let listed = repo.list_projects().await.unwrap();
    assert!(listed.iter().any(|project| project.id == created.id));

    let archived = repo.archive_project(created.id).await.unwrap();
    assert_eq!(archived.status, ProjectStatus::Archived);
    let listed_after_archive = repo.list_projects().await.unwrap();
    assert!(!listed_after_archive
        .iter()
        .any(|project| project.id == created.id));

    cleanup(&pool, &code, &owner_ding).await;
}

async fn test_pool() -> PgPool {
    let config =
        AppConfig::from_local_env_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_ENV"))
            .unwrap();
    PgPool::connect(&config.database.url()).await.unwrap()
}

async fn cleanup(pool: &PgPool, code: &str, owner_ding: &str) {
    sqlx::query("delete from projects where code = $1")
        .bind(code)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("delete from users where dingtalk_user_id = $1")
        .bind(owner_ding)
        .execute(pool)
        .await
        .unwrap();
}
