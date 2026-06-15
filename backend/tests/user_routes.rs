use actix_web::cookie::Cookie;
use actix_web::{http::StatusCode, test, web, App};
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::{SessionStore, UserRole};
use costrategy_backend::dingtalk::{DingTalkLoginIdentity, MockDingTalkClient};
use costrategy_backend::error::{ApiErrorCode, ApiErrorResponse};
use costrategy_backend::users::{MemoryUserRepository, NewUser, UserStatus};
use serde_json::json;

#[actix_web::test]
async fn employee_can_list_users_but_cannot_manage_users() {
    let users = MemoryUserRepository::default();
    let target = users
        .insert_user(NewUser {
            dingtalk_user_id: "target".to_string(),
            union_id: None,
            name: "目标用户".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    users
        .insert_user(NewUser {
            dingtalk_user_id: "employee".to_string(),
            union_id: None,
            name: "普通员工".to_string(),
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
    let app = user_test_app(dingtalk, users).await;
    let cookie = login_cookie(&app, "employee-code").await;

    let list_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/users")
            .cookie(cookie.clone())
            .to_request(),
    )
    .await;

    assert_eq!(list_response.status(), StatusCode::OK);
    let listed: serde_json::Value = test::read_body_json(list_response).await;
    assert!(listed
        .as_array()
        .unwrap()
        .iter()
        .any(|user| user["dingtalk_user_id"] == "target"));

    let role_response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/users/{}/role", target.id))
            .cookie(cookie)
            .set_json(json!({ "role": "manager" }))
            .to_request(),
    )
    .await;

    assert_eq!(role_response.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(role_response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthForbidden);
}

#[actix_web::test]
async fn admin_can_list_users_and_update_role_and_status() {
    let users = MemoryUserRepository::default();
    users
        .insert_user(NewUser {
            dingtalk_user_id: "admin".to_string(),
            union_id: None,
            name: "管理员".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Admin,
            status: UserStatus::Active,
        })
        .await;
    let target = users
        .insert_user(NewUser {
            dingtalk_user_id: "target".to_string(),
            union_id: Some("union-target".to_string()),
            name: "目标用户".to_string(),
            avatar_url: Some("https://example.test/avatar.png".to_string()),
            mobile: Some("13800000000".to_string()),
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    let dingtalk = MockDingTalkClient::default().with_login_identity(
        "admin-code",
        DingTalkLoginIdentity {
            dingtalk_user_id: "admin".to_string(),
            union_id: None,
        },
    );
    let app = user_test_app(dingtalk, users).await;
    let cookie = login_cookie(&app, "admin-code").await;

    let list_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/users")
            .cookie(cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed: serde_json::Value = test::read_body_json(list_response).await;
    let target_row = listed
        .as_array()
        .unwrap()
        .iter()
        .find(|user| user["dingtalk_user_id"] == "target")
        .unwrap();
    assert_eq!(target_row["name"], "目标用户");
    assert_eq!(target_row["role"], "employee");
    assert_eq!(target_row["status"], "active");

    let role_response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/users/{}/role", target.id))
            .cookie(cookie.clone())
            .set_json(json!({ "role": "manager" }))
            .to_request(),
    )
    .await;
    assert_eq!(role_response.status(), StatusCode::OK);
    let role_updated: serde_json::Value = test::read_body_json(role_response).await;
    assert_eq!(role_updated["role"], "manager");

    let status_response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri(&format!("/api/users/{}/status", target.id))
            .cookie(cookie)
            .set_json(json!({ "status": "disabled" }))
            .to_request(),
    )
    .await;
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_updated: serde_json::Value = test::read_body_json(status_response).await;
    assert_eq!(status_updated["status"], "disabled");
}

async fn user_test_app(
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
            .configure(costrategy_backend::routes::configure_app::<
                MockDingTalkClient,
                MemoryUserRepository,
            >)
            .configure(costrategy_backend::routes::configure_user_routes::<
                MockDingTalkClient,
                MemoryUserRepository,
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
