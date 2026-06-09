use actix_web::cookie::Cookie;
use actix_web::{http::StatusCode, test, web, App};
use chrono::NaiveDate;
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::{SessionStore, UserRole};
use costrategy_backend::dingtalk::{DingTalkLoginIdentity, MockDingTalkClient};
use costrategy_backend::error::{ApiErrorCode, ApiErrorResponse};
use costrategy_backend::projects::{MemoryProjectRepository, ProjectStatus};
use costrategy_backend::users::{MemoryUserRepository, NewUser, UserStatus};
use serde_json::json;

#[actix_web::test]
async fn project_list_requires_login() {
    let app = project_test_app(
        MockDingTalkClient::default(),
        MemoryUserRepository::default(),
    )
    .await;

    let response = test::call_service(
        &app,
        test::TestRequest::get().uri("/api/projects").to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthNotLogin);
}

#[actix_web::test]
async fn employee_cannot_create_project() {
    let users = MemoryUserRepository::default();
    users
        .insert_user(NewUser {
            dingtalk_user_id: "employee".to_string(),
            union_id: None,
            name: "员工".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    let dingtalk = MockDingTalkClient::default().with_login_identity(
        "employee-code",
        DingTalkLoginIdentity {
            dingtalk_user_id: "employee".to_string(),
            union_id: None,
        },
    );
    let app = project_test_app(dingtalk, users).await;
    let cookie = login_cookie(&app, "employee-code").await;

    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/projects")
            .cookie(cookie)
            .set_json(project_payload())
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthForbidden);
}

#[actix_web::test]
async fn manager_can_create_update_list_and_archive_project() {
    let users = MemoryUserRepository::default();
    users
        .insert_user(NewUser {
            dingtalk_user_id: "manager".to_string(),
            union_id: None,
            name: "管理人员".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Manager,
            status: UserStatus::Active,
        })
        .await;
    let dingtalk = MockDingTalkClient::default().with_login_identity(
        "manager-code",
        DingTalkLoginIdentity {
            dingtalk_user_id: "manager".to_string(),
            union_id: None,
        },
    );
    let app = project_test_app(dingtalk, users).await;
    let cookie = login_cookie(&app, "manager-code").await;

    let create_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/projects")
            .cookie(cookie.clone())
            .set_json(project_payload())
            .to_request(),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let created: serde_json::Value = test::read_body_json(create_response).await;
    assert_eq!(created["code"], "PM-001");
    assert_eq!(created["status"], "active");
    let project_id = created["id"].as_str().unwrap();

    let update_response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri(&format!("/api/projects/{project_id}"))
            .cookie(cookie.clone())
            .set_json(json!({
                "name": "项目管理系统一期",
                "description": "更新后的描述",
                "start_date": "2026-06-01",
                "end_date": "2026-08-01",
                "status": "paused"
            }))
            .to_request(),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: serde_json::Value = test::read_body_json(update_response).await;
    assert_eq!(updated["name"], "项目管理系统一期");
    assert_eq!(updated["status"], "paused");

    let list_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/projects")
            .cookie(cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let projects: serde_json::Value = test::read_body_json(list_response).await;
    assert_eq!(projects.as_array().unwrap().len(), 1);

    let archive_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/projects/{project_id}/archive"))
            .cookie(cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(archive_response.status(), StatusCode::OK);
    let archived: serde_json::Value = test::read_body_json(archive_response).await;
    assert_eq!(archived["status"], ProjectStatus::Archived.as_str());
}

async fn project_test_app(
    dingtalk: MockDingTalkClient,
    users: MemoryUserRepository,
) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    test::init_service(
        App::new()
            .app_data(web::Data::new(AppState::new(
                dingtalk,
                users,
                SessionStore::default(),
            )))
            .app_data(web::Data::new(MemoryProjectRepository::default()))
            .configure(costrategy_backend::routes::configure_app::<
                MockDingTalkClient,
                MemoryUserRepository,
            >)
            .configure(costrategy_backend::routes::configure_project_routes::<
                MockDingTalkClient,
                MemoryUserRepository,
                MemoryProjectRepository,
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

fn project_payload() -> serde_json::Value {
    json!({
        "code": "PM-001",
        "name": "项目管理系统",
        "description": "第一版项目管理系统",
        "start_date": NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        "end_date": NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
        "status": "active"
    })
}
