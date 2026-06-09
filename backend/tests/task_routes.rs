use actix_web::cookie::Cookie;
use actix_web::{http::StatusCode, test, web, App};
use chrono::NaiveDate;
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::{SessionStore, UserRole};
use costrategy_backend::dingtalk::{DingTalkLoginIdentity, MockDingTalkClient};
use costrategy_backend::error::{ApiErrorCode, ApiErrorResponse};
use costrategy_backend::notifications::{
    MemoryNotificationRepository, NotificationRepository, NotificationType,
};
use costrategy_backend::projects::{
    CreateProject, MemoryProjectRepository, ProjectRepository, ProjectStatus,
};
use costrategy_backend::tasks::MemoryTaskRepository;
use costrategy_backend::users::{MemoryUserRepository, NewUser, UserStatus};
use serde_json::json;
use uuid::Uuid;

#[actix_web::test]
async fn employee_cannot_create_task() {
    let fixture = TaskRouteFixture::new().await;
    let app = task_test_app(&fixture).await;
    let cookie = login_cookie(&app, "employee-code").await;

    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/tasks")
            .cookie(cookie)
            .set_json(fixture.task_payload(fixture.employee_id))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthForbidden);
}

#[actix_web::test]
async fn manager_can_create_edit_list_and_archive_task() {
    let fixture = TaskRouteFixture::new().await;
    let app = task_test_app(&fixture).await;
    let cookie = login_cookie(&app, "manager-code").await;

    let create_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/tasks")
            .cookie(cookie.clone())
            .set_json(fixture.task_payload(fixture.employee_id))
            .to_request(),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let created: serde_json::Value = test::read_body_json(create_response).await;
    assert_eq!(created["title"], "需求文档确认");
    assert_eq!(created["status"], "todo");
    assert_eq!(created["priority"], "high");
    let task_id = created["id"].as_str().unwrap();

    let update_response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/api/tasks/{task_id}"))
            .cookie(cookie.clone())
            .set_json(json!({
                "title": "需求文档复核",
                "project_id": fixture.project_id,
                "assignee_id": fixture.employee_id,
                "status": "in_progress",
                "priority": "medium",
                "start_date": "2026-06-02",
                "due_date": "2026-06-15",
                "description_json": {"type": "doc", "content": []}
            }))
            .to_request(),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: serde_json::Value = test::read_body_json(update_response).await;
    assert_eq!(updated["title"], "需求文档复核");
    assert_eq!(updated["status"], "in_progress");

    let list_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/tasks?keyword=%E9%9C%80%E6%B1%82")
            .cookie(cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let tasks: serde_json::Value = test::read_body_json(list_response).await;
    assert_eq!(tasks.as_array().unwrap().len(), 1);

    let archive_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/tasks/{task_id}/archive"))
            .cookie(cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(archive_response.status(), StatusCode::OK);

    let list_after_archive = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/tasks")
            .cookie(cookie.clone())
            .to_request(),
    )
    .await;
    let tasks: serde_json::Value = test::read_body_json(list_after_archive).await;
    assert_eq!(tasks.as_array().unwrap().len(), 0);
}

#[actix_web::test]
async fn task_create_and_assignee_change_trigger_dingtalk_personal_notifications() {
    let fixture = TaskRouteFixture::new().await;
    let app = task_test_app(&fixture).await;
    let cookie = login_cookie(&app, "manager-code").await;

    let task_id = create_task(
        &app,
        cookie.clone(),
        fixture.task_payload(fixture.employee_id),
    )
    .await;
    let sent_after_create = fixture.dingtalk.sent_notifications();
    assert_eq!(sent_after_create.len(), 1);
    assert_eq!(sent_after_create[0].receiver_dingtalk_user_id, "employee");
    assert!(sent_after_create[0].message.contains("新任务分配"));

    let update_response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/api/tasks/{task_id}"))
            .cookie(cookie)
            .set_json(json!({
                "title": "需求文档复核",
                "project_id": fixture.project_id,
                "assignee_id": fixture.other_id,
                "status": "in_progress",
                "priority": "medium",
                "start_date": "2026-06-02",
                "due_date": "2026-06-15",
                "description_json": {"type": "doc", "content": []}
            }))
            .to_request(),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);

    let sent_after_update = fixture.dingtalk.sent_notifications();
    assert_eq!(sent_after_update.len(), 2);
    assert_eq!(sent_after_update[1].receiver_dingtalk_user_id, "other");
    assert!(sent_after_update[1].message.contains("负责人变更"));

    let records = fixture.notifications.list_records().await.unwrap();
    assert_eq!(records.len(), 2);
    assert!(records
        .iter()
        .any(|record| record.notification_type == NotificationType::TaskAssigned));
    assert!(records
        .iter()
        .any(|record| record.notification_type == NotificationType::AssigneeChanged));
}

#[actix_web::test]
async fn employee_can_update_own_task_status_but_not_others() {
    let fixture = TaskRouteFixture::new().await;
    let app = task_test_app(&fixture).await;
    let manager_cookie = login_cookie(&app, "manager-code").await;
    let employee_cookie = login_cookie(&app, "employee-code").await;
    let other_cookie = login_cookie(&app, "other-code").await;

    let task_id = create_task(
        &app,
        manager_cookie,
        fixture.task_payload(fixture.employee_id),
    )
    .await;

    let own_update = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/tasks/{task_id}/status"))
            .cookie(employee_cookie)
            .set_json(json!({ "status": "in_progress" }))
            .to_request(),
    )
    .await;
    assert_eq!(own_update.status(), StatusCode::OK);

    let other_update = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/tasks/{task_id}/status"))
            .cookie(other_cookie)
            .set_json(json!({ "status": "done" }))
            .to_request(),
    )
    .await;
    assert_eq!(other_update.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(other_update).await;
    assert_eq!(body.error.code, ApiErrorCode::TaskNotAssignee);
}

#[actix_web::test]
async fn task_status_transition_rejects_done_to_todo() {
    let fixture = TaskRouteFixture::new().await;
    let app = task_test_app(&fixture).await;
    let manager_cookie = login_cookie(&app, "manager-code").await;
    let task_id = create_task(
        &app,
        manager_cookie.clone(),
        json!({
            "title": "已完成任务",
            "project_id": fixture.project_id,
            "assignee_id": fixture.employee_id,
            "status": "done",
            "priority": "low",
            "start_date": "2026-06-01",
            "due_date": "2026-06-02",
            "description_json": {"type": "doc", "content": []}
        }),
    )
    .await;

    let response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/tasks/{task_id}/status"))
            .cookie(manager_cookie)
            .set_json(json!({ "status": "todo" }))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::TaskInvalidStatusTransition);
}

#[actix_web::test]
async fn task_list_marks_unfinished_past_due_task_as_overdue() {
    let fixture = TaskRouteFixture::new().await;
    let app = task_test_app(&fixture).await;
    let manager_cookie = login_cookie(&app, "manager-code").await;
    create_task(
        &app,
        manager_cookie.clone(),
        json!({
            "title": "延期任务",
            "project_id": fixture.project_id,
            "assignee_id": fixture.employee_id,
            "status": "todo",
            "priority": "high",
            "start_date": "2026-05-01",
            "due_date": "2026-06-01",
            "description_json": {"type": "doc", "content": []}
        }),
    )
    .await;

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/tasks")
            .cookie(manager_cookie)
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let tasks: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(tasks[0]["is_overdue"], true);
    assert_eq!(tasks[0]["display_status"], "overdue");
}

#[actix_web::test]
async fn logged_in_user_can_read_task_detail_and_add_plain_text_comment() {
    let fixture = TaskRouteFixture::new().await;
    let app = task_test_app(&fixture).await;
    let manager_cookie = login_cookie(&app, "manager-code").await;
    let employee_cookie = login_cookie(&app, "employee-code").await;
    let task_id = create_task(
        &app,
        manager_cookie.clone(),
        fixture.task_payload(fixture.employee_id),
    )
    .await;

    let comment_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/tasks/{task_id}/comments"))
            .cookie(employee_cookie.clone())
            .set_json(json!({ "content": "今天已完成需求确认。" }))
            .to_request(),
    )
    .await;
    assert_eq!(comment_response.status(), StatusCode::OK);
    let comment: serde_json::Value = test::read_body_json(comment_response).await;
    assert_eq!(comment["content"], "今天已完成需求确认。");
    assert_eq!(comment["author_name"], "员工");

    let detail_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/tasks/{task_id}"))
            .cookie(manager_cookie)
            .to_request(),
    )
    .await;

    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail: serde_json::Value = test::read_body_json(detail_response).await;
    assert_eq!(detail["task"]["title"], "需求文档确认");
    assert_eq!(detail["comments"][0]["content"], "今天已完成需求确认。");
    assert_eq!(detail["comments"][0]["author_name"], "员工");
    assert_eq!(detail["attachments"].as_array().unwrap().len(), 0);
    assert!(detail["activity_logs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|log| log["action"] == "comment_created"));
}

#[actix_web::test]
async fn blank_comment_returns_validation_failed() {
    let fixture = TaskRouteFixture::new().await;
    let app = task_test_app(&fixture).await;
    let manager_cookie = login_cookie(&app, "manager-code").await;
    let task_id = create_task(
        &app,
        manager_cookie.clone(),
        fixture.task_payload(fixture.employee_id),
    )
    .await;

    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/tasks/{task_id}/comments"))
            .cookie(manager_cookie)
            .set_json(json!({ "content": "   " }))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::ValidationFailed);
}

struct TaskRouteFixture {
    dingtalk: MockDingTalkClient,
    users: MemoryUserRepository,
    projects: MemoryProjectRepository,
    tasks: MemoryTaskRepository,
    notifications: MemoryNotificationRepository,
    project_id: Uuid,
    employee_id: Uuid,
    other_id: Uuid,
}

impl TaskRouteFixture {
    async fn new() -> Self {
        let users = MemoryUserRepository::default();
        let manager = users
            .insert_user(new_user("manager", "管理人员", UserRole::Manager))
            .await;
        let employee = users
            .insert_user(new_user("employee", "员工", UserRole::Employee))
            .await;
        let other = users
            .insert_user(new_user("other", "其他员工", UserRole::Employee))
            .await;

        let dingtalk = MockDingTalkClient::default()
            .with_login_identity("manager-code", identity("manager"))
            .with_login_identity("employee-code", identity("employee"))
            .with_login_identity("other-code", identity("other"));

        let projects = MemoryProjectRepository::default();
        let project = projects
            .create_project(CreateProject {
                code: "PM-001".to_string(),
                name: "项目管理系统".to_string(),
                description: None,
                start_date: Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
                end_date: Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap()),
                status: ProjectStatus::Active,
            })
            .await
            .unwrap();

        assert_eq!(manager.role, UserRole::Manager);
        assert_eq!(other.role, UserRole::Employee);

        Self {
            dingtalk,
            users,
            projects,
            tasks: MemoryTaskRepository::default(),
            notifications: MemoryNotificationRepository::default(),
            project_id: project.id,
            employee_id: employee.id,
            other_id: other.id,
        }
    }

    fn task_payload(&self, assignee_id: Uuid) -> serde_json::Value {
        json!({
            "title": "需求文档确认",
            "project_id": self.project_id,
            "assignee_id": assignee_id,
            "status": "todo",
            "priority": "high",
            "start_date": "2026-06-01",
            "due_date": "2026-06-10",
            "description_json": {"type": "doc", "content": []}
        })
    }
}

async fn task_test_app(
    fixture: &TaskRouteFixture,
) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new(
                fixture.dingtalk.clone(),
                fixture.users.clone(),
                SessionStore::default(),
            )))
            .app_data(web::Data::new(fixture.projects.clone()))
            .app_data(web::Data::new(fixture.tasks.clone()))
            .app_data(web::Data::new(fixture.notifications.clone()))
            .configure(costrategy_backend::routes::configure_app::<
                MockDingTalkClient,
                MemoryUserRepository,
            >)
            .configure(costrategy_backend::routes::configure_project_routes::<
                MockDingTalkClient,
                MemoryUserRepository,
                MemoryProjectRepository,
            >)
            .configure(costrategy_backend::routes::configure_task_routes::<
                MockDingTalkClient,
                MemoryUserRepository,
                MemoryTaskRepository,
                MemoryNotificationRepository,
            >),
    )
    .await
}

async fn login_cookie<S>(app: &S, code: &str) -> Cookie<'static>
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let response = test::call_service(
        app,
        test::TestRequest::post()
            .uri("/api/auth/dingtalk/login")
            .set_json(json!({ "code": code }))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response
        .response()
        .cookies()
        .find(|cookie| cookie.name() == "costrategy_session")
        .unwrap()
        .into_owned()
}

async fn create_task<S>(app: &S, cookie: Cookie<'static>, payload: serde_json::Value) -> String
where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    let response = test::call_service(
        app,
        test::TestRequest::post()
            .uri("/api/tasks")
            .cookie(cookie)
            .set_json(payload)
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(response).await;
    body["id"].as_str().unwrap().to_string()
}

fn new_user(dingtalk_user_id: &str, name: &str, role: UserRole) -> NewUser {
    NewUser {
        dingtalk_user_id: dingtalk_user_id.to_string(),
        union_id: None,
        name: name.to_string(),
        avatar_url: None,
        mobile: None,
        role,
        status: UserStatus::Active,
    }
}

fn identity(dingtalk_user_id: &str) -> DingTalkLoginIdentity {
    DingTalkLoginIdentity {
        dingtalk_user_id: dingtalk_user_id.to_string(),
        union_id: None,
    }
}
