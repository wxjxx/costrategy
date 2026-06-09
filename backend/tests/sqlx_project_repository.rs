use chrono::NaiveDate;
use costrategy_backend::config::AppConfig;
use costrategy_backend::projects::{
    CreateProject, ProjectRepository, ProjectStatus, SqlxProjectRepository, UpdateProject,
};
use sqlx::PgPool;
use std::path::Path;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::test]
async fn sqlx_project_repository_crud_and_archive() {
    let pool = test_pool().await;
    MIGRATOR.run(&pool).await.unwrap();
    let repo = SqlxProjectRepository::new(pool.clone());
    let code = format!("TEST-PROJECT-{}", Uuid::new_v4().simple());

    cleanup(&pool, &code).await;

    let created = repo
        .create_project(CreateProject {
            code: code.clone(),
            name: "测试项目".to_string(),
            description: Some("创建描述".to_string()),
            start_date: Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
            end_date: Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            status: ProjectStatus::Active,
        })
        .await
        .unwrap();
    assert_eq!(created.code, code);
    assert_eq!(created.status, ProjectStatus::Active);

    let updated = repo
        .update_project(
            created.id,
            UpdateProject {
                name: "测试项目更新".to_string(),
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

    let listed = repo.list_projects().await.unwrap();
    assert!(listed.iter().any(|project| project.id == created.id));

    let archived = repo.archive_project(created.id).await.unwrap();
    assert_eq!(archived.status, ProjectStatus::Archived);
    let listed_after_archive = repo.list_projects().await.unwrap();
    assert!(!listed_after_archive
        .iter()
        .any(|project| project.id == created.id));

    cleanup(&pool, &code).await;
}

async fn test_pool() -> PgPool {
    let config =
        AppConfig::from_local_env_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_ENV"))
            .unwrap();
    PgPool::connect(&config.database.url()).await.unwrap()
}

async fn cleanup(pool: &PgPool, code: &str) {
    sqlx::query("delete from projects where code = $1")
        .bind(code)
        .execute(pool)
        .await
        .unwrap();
}
