use chrono::NaiveDate;
use costrategy_backend::auth::UserRole;
use costrategy_backend::config::AppConfig;
use costrategy_backend::projects::{
    CreateProject, ProjectRepository, ProjectStatus, SqlxProjectRepository,
};
use costrategy_backend::tasks::{
    CreateTask, CreateTaskAttachment, CreateTaskComment, SqlxTaskRepository, TaskFilter,
    TaskPriority, TaskRepository, TaskSort, TaskStatus,
};
use costrategy_backend::users::{NewUser, SqlxUserRepository, UserRepository, UserStatus};
use serde_json::json;
use sqlx::PgPool;
use std::path::Path;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[tokio::test]
async fn sqlx_task_repository_crud_status_archive_and_activity_logs() {
    let pool = test_pool().await;
    MIGRATOR.run(&pool).await.unwrap();
    let users = SqlxUserRepository::new(pool.clone());
    let projects = SqlxProjectRepository::new(pool.clone());
    let tasks = SqlxTaskRepository::new(pool.clone());
    let suffix = Uuid::new_v4().simple().to_string();
    let manager_ding = format!("test-manager-{suffix}");
    let assignee_ding = format!("test-assignee-{suffix}");
    let project_code = format!("TEST-TASK-PROJECT-{suffix}");

    cleanup(&pool, &manager_ding, &assignee_ding, &project_code).await;

    users
        .upsert_synced_user(NewUser {
            dingtalk_user_id: manager_ding.clone(),
            union_id: None,
            name: "测试经理".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Manager,
            status: UserStatus::Active,
        })
        .await
        .unwrap();
    users
        .upsert_synced_user(NewUser {
            dingtalk_user_id: assignee_ding.clone(),
            union_id: None,
            name: "测试员工".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await
        .unwrap();
    let manager = users
        .find_by_dingtalk_user_id(&manager_ding)
        .await
        .unwrap()
        .unwrap();
    let assignee = users
        .find_by_dingtalk_user_id(&assignee_ding)
        .await
        .unwrap()
        .unwrap();
    let project = projects
        .create_project(CreateProject {
            code: Some(project_code.clone()),
            name: "任务测试项目".to_string(),
            owner_id: Some(manager.id),
            description: None,
            start_date: Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
            end_date: Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
            status: ProjectStatus::Active,
        })
        .await
        .unwrap();

    let created = tasks
        .create_task(CreateTask {
            project_id: project.id,
            title: "任务仓储测试".to_string(),
            assignee_id: assignee.id,
            assignee_ids: vec![assignee.id],
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            due_date: NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
            description_json: json!({"type": "doc", "content": []}),
            creator_id: manager.id,
        })
        .await
        .unwrap();
    assert_eq!(created.status, TaskStatus::Todo);
    assert_eq!(created.project_name.as_deref(), Some("任务测试项目"));
    assert_eq!(created.assignee_name.as_deref(), Some("测试员工"));
    assert!(created.updated_at <= chrono::Utc::now());
    let low_priority = tasks
        .create_task(CreateTask {
            project_id: project.id,
            title: "低优先级仓储测试".to_string(),
            assignee_id: assignee.id,
            assignee_ids: vec![assignee.id],
            status: TaskStatus::Todo,
            priority: TaskPriority::Low,
            start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            due_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            description_json: json!({"type": "doc", "content": []}),
            creator_id: manager.id,
        })
        .await
        .unwrap();

    let listed = tasks
        .list_tasks(TaskFilter {
            keyword: Some("仓储".to_string()),
            ..TaskFilter::default()
        })
        .await
        .unwrap();
    let listed_task = listed.iter().find(|task| task.id == created.id).unwrap();
    assert_eq!(listed_task.project_name.as_deref(), Some("任务测试项目"));
    assert_eq!(listed_task.assignee_name.as_deref(), Some("测试员工"));

    let in_progress = tasks
        .update_task_status(created.id, assignee.id, TaskStatus::InProgress)
        .await
        .unwrap();
    assert_eq!(in_progress.status, TaskStatus::InProgress);
    assert_eq!(in_progress.project_name.as_deref(), Some("任务测试项目"));
    assert_eq!(in_progress.assignee_name.as_deref(), Some("测试员工"));
    assert_eq!(in_progress.project_owner_id, Some(manager.id));
    let blocked = tasks
        .update_task_status(created.id, assignee.id, TaskStatus::Blocked)
        .await
        .unwrap();
    assert_eq!(blocked.status, TaskStatus::Blocked);
    let back_to_progress = tasks
        .update_task_status(created.id, assignee.id, TaskStatus::InProgress)
        .await
        .unwrap();
    assert_eq!(back_to_progress.status, TaskStatus::InProgress);
    let due_tasks = tasks
        .list_tasks_due_on(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap())
        .await
        .unwrap();
    assert!(due_tasks.iter().any(|task| task.id == created.id));
    let overdue_tasks = tasks
        .list_overdue_tasks(NaiveDate::from_ymd_opt(2026, 6, 11).unwrap())
        .await
        .unwrap();
    assert!(overdue_tasks
        .iter()
        .any(|task| task.id == created.id && task.project_owner_id == Some(manager.id)));

    let comment = tasks
        .create_comment(CreateTaskComment {
            task_id: created.id,
            author_id: assignee.id,
            author_name: None,
            content: "  SQLx 评论测试  ".to_string(),
        })
        .await
        .unwrap();
    assert_eq!(comment.content, "SQLx 评论测试");
    assert_eq!(comment.author_name.as_deref(), Some("测试员工"));

    let attachment = tasks
        .create_attachment(CreateTaskAttachment {
            task_id: created.id,
            file_name: "排期说明.txt".to_string(),
            file_size: 12,
            mime_type: Some("text/plain".to_string()),
            bucket: "test-bucket".to_string(),
            object_key: format!("tasks/{}/attachments/test-file", created.id),
            uploader_id: assignee.id,
            uploader_name: None,
        })
        .await
        .unwrap();
    assert_eq!(attachment.file_name, "排期说明.txt");
    assert_eq!(attachment.uploader_name.as_deref(), Some("测试员工"));

    let detail = tasks.get_task_detail(created.id).await.unwrap();
    assert_eq!(detail.task.id, created.id);
    assert_eq!(detail.comments[0].content, "SQLx 评论测试");
    assert_eq!(detail.attachments[0].file_name, "排期说明.txt");
    assert!(detail
        .activity_logs
        .iter()
        .any(|log| log.action == "comment_created"));
    assert!(detail.activity_logs.iter().any(|log| {
        log.action == "status_changed"
            && log
                .after_value
                .as_ref()
                .and_then(|value| value.get("status"))
                .and_then(|value| value.as_str())
                == Some("blocked")
    }));

    let attachment_record = tasks
        .get_attachment(created.id, attachment.id)
        .await
        .unwrap();
    assert_eq!(
        attachment_record.object_key,
        format!("tasks/{}/attachments/test-file", created.id)
    );
    let deleted_attachment = tasks
        .delete_attachment(created.id, attachment.id, manager.id)
        .await
        .unwrap();
    assert_eq!(deleted_attachment.file_name, "排期说明.txt");
    let detail_after_attachment_delete = tasks.get_task_detail(created.id).await.unwrap();
    assert_eq!(detail_after_attachment_delete.attachments.len(), 0);

    let archived = tasks.archive_task(created.id, manager.id).await.unwrap();
    assert!(archived.archived);
    assert_eq!(archived.project_name.as_deref(), Some("任务测试项目"));
    assert_eq!(archived.assignee_name.as_deref(), Some("测试员工"));
    let listed_after_archive = tasks.list_tasks(TaskFilter::default()).await.unwrap();
    assert!(!listed_after_archive
        .iter()
        .any(|task| task.id == created.id));
    let archived_and_sorted = tasks
        .list_tasks(TaskFilter {
            project_id: Some(project.id),
            include_archived: true,
            sort: TaskSort::UpdatedAt,
            ..TaskFilter::default()
        })
        .await
        .unwrap();
    assert_eq!(archived_and_sorted[0].id, created.id);
    assert!(archived_and_sorted[0].archived);
    assert_eq!(archived_and_sorted[1].id, low_priority.id);

    let log_count: i64 =
        sqlx::query_scalar("select count(*) from task_activity_logs where task_id = $1")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(log_count, 8);

    cleanup(&pool, &manager_ding, &assignee_ding, &project_code).await;
}

async fn test_pool() -> PgPool {
    let config =
        AppConfig::from_local_env_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_ENV"))
            .unwrap();
    PgPool::connect(&config.database.url()).await.unwrap()
}

async fn cleanup(pool: &PgPool, manager_ding: &str, assignee_ding: &str, project_code: &str) {
    sqlx::query(
        "delete from task_activity_logs
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
    sqlx::query(
        "delete from tasks
         where project_id in (select id from projects where code = $1)",
    )
    .bind(project_code)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("delete from projects where code = $1")
        .bind(project_code)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("delete from users where dingtalk_user_id = any($1)")
        .bind(vec![manager_ding.to_string(), assignee_ding.to_string()])
        .execute(pool)
        .await
        .unwrap();
}
