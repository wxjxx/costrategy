use actix_web::cookie::Cookie;
use actix_web::{http::StatusCode, test, web, App};
use chrono::NaiveDate;
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::{SessionStore, UserRole};
use costrategy_backend::dingtalk::{DingTalkLoginIdentity, MockDingTalkClient};
use costrategy_backend::error::{ApiErrorCode, ApiErrorResponse};
use costrategy_backend::notifications::{MemoryNotificationRepository, NotificationRepository};
use costrategy_backend::projects::{
    CreateProject, MemoryProjectRepository, ProjectRepository, ProjectStatus,
};
use costrategy_backend::storage::MemoryAttachmentStorage;
use costrategy_backend::tasks::MemoryTaskRepository;
use costrategy_backend::users::{MemoryUserRepository, NewUser, UserStatus};
use serde_json::json;
use uuid::Uuid;

#[actix_web::test]
async fn logged_in_user_can_upload_download_and_manager_can_delete_attachment() {
    let fixture = AttachmentRouteFixture::new().await;
    let app = attachment_test_app(&fixture).await;
    let manager_cookie = login_cookie(&app, "manager-code").await;
    let employee_cookie = login_cookie(&app, "employee-code").await;
    let other_cookie = login_cookie(&app, "other-code").await;
    let task_id = create_task(
        &app,
        manager_cookie.clone(),
        fixture.task_payload(fixture.employee_id),
    )
    .await;
    assert_eq!(fixture.notifications.list_records().await.unwrap().len(), 1);

    let upload_response = test::call_service(
        &app,
        multipart_request(
            &format!("/api/tasks/{task_id}/attachments"),
            employee_cookie.clone(),
        ),
    )
    .await;
    assert_eq!(upload_response.status(), StatusCode::OK);
    let attachment: serde_json::Value = test::read_body_json(upload_response).await;
    assert_eq!(attachment["file_name"], "需求说明.txt");
    assert_eq!(attachment["file_size"], 12);
    assert_eq!(attachment["uploader_name"], "员工");
    assert_eq!(fixture.notifications.list_records().await.unwrap().len(), 1);
    let attachment_id = attachment["id"].as_str().unwrap();

    let detail_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/tasks/{task_id}"))
            .cookie(employee_cookie.clone())
            .to_request(),
    )
    .await;
    let detail: serde_json::Value = test::read_body_json(detail_response).await;
    assert_eq!(detail["attachments"][0]["file_name"], "需求说明.txt");

    let download_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!(
                "/api/tasks/{task_id}/attachments/{attachment_id}/download"
            ))
            .cookie(employee_cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(download_response.status(), StatusCode::OK);
    let downloaded = test::read_body(download_response).await;
    assert_eq!(&downloaded[..], "附件内容".as_bytes());

    let forbidden_delete = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri(&format!("/api/tasks/{task_id}/attachments/{attachment_id}"))
            .cookie(other_cookie)
            .to_request(),
    )
    .await;
    assert_eq!(forbidden_delete.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(forbidden_delete).await;
    assert_eq!(body.error.code, ApiErrorCode::AttachmentDeleteForbidden);

    let manager_delete = test::call_service(
        &app,
        test::TestRequest::delete()
            .uri(&format!("/api/tasks/{task_id}/attachments/{attachment_id}"))
            .cookie(manager_cookie)
            .to_request(),
    )
    .await;
    assert_eq!(manager_delete.status(), StatusCode::OK);

    let detail_after_delete = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/tasks/{task_id}"))
            .cookie(employee_cookie)
            .to_request(),
    )
    .await;
    let detail: serde_json::Value = test::read_body_json(detail_after_delete).await;
    assert_eq!(detail["attachments"].as_array().unwrap().len(), 0);
}

#[actix_web::test]
async fn upload_without_file_returns_validation_failed() {
    let fixture = AttachmentRouteFixture::new().await;
    let app = attachment_test_app(&fixture).await;
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
            .uri(&format!("/api/tasks/{task_id}/attachments"))
            .cookie(manager_cookie)
            .insert_header(("Content-Type", "multipart/form-data; boundary=missing-file"))
            .set_payload("--missing-file--\r\n")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::ValidationFailed);
}

struct AttachmentRouteFixture {
    dingtalk: MockDingTalkClient,
    users: MemoryUserRepository,
    projects: MemoryProjectRepository,
    tasks: MemoryTaskRepository,
    notifications: MemoryNotificationRepository,
    storage: MemoryAttachmentStorage,
    project_id: Uuid,
    employee_id: Uuid,
}

impl AttachmentRouteFixture {
    async fn new() -> Self {
        let users = MemoryUserRepository::default();
        users
            .insert_user(new_user("manager", "管理人员", UserRole::Manager))
            .await;
        let employee = users
            .insert_user(new_user("employee", "员工", UserRole::Employee))
            .await;
        users
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

        Self {
            dingtalk,
            users,
            projects,
            tasks: MemoryTaskRepository::default(),
            notifications: MemoryNotificationRepository::default(),
            storage: MemoryAttachmentStorage::new("test-bucket"),
            project_id: project.id,
            employee_id: employee.id,
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

async fn attachment_test_app(
    fixture: &AttachmentRouteFixture,
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
            .app_data(web::Data::new(fixture.storage.clone()))
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
            >)
            .configure(costrategy_backend::routes::configure_attachment_routes::<
                MockDingTalkClient,
                MemoryUserRepository,
                MemoryTaskRepository,
                MemoryAttachmentStorage,
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
    let created: serde_json::Value = test::read_body_json(response).await;
    created["id"].as_str().unwrap().to_string()
}

fn multipart_request(uri: &str, cookie: Cookie<'static>) -> actix_http::Request {
    let boundary = "attachment-boundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"需求说明.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         附件内容\r\n\
         --{boundary}--\r\n"
    );

    test::TestRequest::post()
        .uri(uri)
        .cookie(cookie)
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        ))
        .set_payload(body)
        .to_request()
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
