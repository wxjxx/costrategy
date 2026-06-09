use actix_web::cookie::Cookie;
use actix_web::{http::StatusCode, test, web, App};
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::{SessionStore, UserRole};
use costrategy_backend::dingtalk::{
    DingTalkDepartment, DingTalkLoginIdentity, DingTalkUser, MockDingTalkClient,
};
use costrategy_backend::error::{ApiErrorCode, ApiErrorResponse};
use costrategy_backend::users::{
    MemoryUserRepository, NewDepartment, NewUser, UserRepository, UserStatus,
};
use serde_json::json;

#[actix_web::test]
async fn login_sets_session_cookie_and_me_returns_current_user() {
    let users = MemoryUserRepository::default();
    users
        .insert_user(NewUser {
            dingtalk_user_id: "ding-user-1".to_string(),
            union_id: Some("union-1".to_string()),
            name: "张三".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    users
        .upsert_department(NewDepartment {
            dingtalk_dept_id: 42,
            parent_dingtalk_dept_id: None,
            name: "研发部".to_string(),
            order_no: Some(1),
        })
        .await
        .unwrap();
    users
        .replace_department_users(42, &["ding-user-1".to_string()])
        .await
        .unwrap();
    let dingtalk = MockDingTalkClient::default().with_login_identity(
        "valid-code",
        DingTalkLoginIdentity {
            dingtalk_user_id: "ding-user-1".to_string(),
            union_id: Some("union-1".to_string()),
        },
    );
    let app =
        test::init_service(
            App::new()
                .app_data(web::Data::new(AppState::new(
                    dingtalk,
                    users,
                    SessionStore::default(),
                )))
                .configure(
                    costrategy_backend::routes::configure_app::<
                        MockDingTalkClient,
                        MemoryUserRepository,
                    >,
                ),
        )
        .await;

    let login_request = test::TestRequest::post()
        .uri("/api/auth/dingtalk/login")
        .set_json(json!({ "code": "valid-code" }))
        .to_request();
    let login_response = test::call_service(&app, login_request).await;
    assert_eq!(login_response.status(), StatusCode::OK);
    let session_cookie = login_response
        .response()
        .cookies()
        .find(|cookie| cookie.name() == "costrategy_session")
        .expect("login should set session cookie")
        .into_owned();

    let me_request = test::TestRequest::get()
        .uri("/api/me")
        .cookie(session_cookie)
        .to_request();
    let me_response = test::call_service(&app, me_request).await;
    assert_eq!(me_response.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(me_response).await;
    assert_eq!(body["name"], "张三");
    assert_eq!(body["role"], "employee");
    assert_eq!(body["departments"], serde_json::json!(["研发部"]));
    assert!(body["permissions"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("view_workbench")));
    assert!(body["permissions"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("update_own_task_status")));
}

#[actix_web::test]
async fn me_without_session_returns_auth_not_login() {
    let app = test_app(
        MockDingTalkClient::default(),
        MemoryUserRepository::default(),
    )
    .await;

    let request = test::TestRequest::get().uri("/api/me").to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthNotLogin);
}

#[actix_web::test]
async fn logout_invalidates_session_cookie() {
    let users = MemoryUserRepository::default();
    users
        .insert_user(NewUser {
            dingtalk_user_id: "ding-user-1".to_string(),
            union_id: None,
            name: "张三".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    let dingtalk = MockDingTalkClient::default().with_login_identity(
        "valid-code",
        DingTalkLoginIdentity {
            dingtalk_user_id: "ding-user-1".to_string(),
            union_id: None,
        },
    );
    let app = test_app(dingtalk, users).await;
    let session_cookie = login_cookie(&app, "valid-code").await;

    let logout_request = test::TestRequest::post()
        .uri("/api/auth/logout")
        .cookie(session_cookie.clone())
        .to_request();
    let logout_response = test::call_service(&app, logout_request).await;

    assert_eq!(logout_response.status(), StatusCode::OK);
    let cleared_cookie = logout_response
        .response()
        .cookies()
        .find(|cookie| cookie.name() == "costrategy_session")
        .expect("logout should clear session cookie");
    assert_eq!(cleared_cookie.value(), "");

    let me_request = test::TestRequest::get()
        .uri("/api/me")
        .cookie(session_cookie)
        .to_request();
    let me_response = test::call_service(&app, me_request).await;
    assert_eq!(me_response.status(), StatusCode::UNAUTHORIZED);
    let body: ApiErrorResponse = test::read_body_json(me_response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthNotLogin);
}

#[actix_web::test]
async fn dingtalk_sync_requires_admin_session() {
    let users = MemoryUserRepository::default();
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
    let app = test_app(dingtalk, users).await;
    let session_cookie = login_cookie(&app, "employee-code").await;

    let sync_request = test::TestRequest::post()
        .uri("/api/dingtalk/sync")
        .cookie(session_cookie)
        .to_request();
    let response = test::call_service(&app, sync_request).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthForbidden);
}

#[actix_web::test]
async fn dingtalk_sync_without_session_returns_auth_not_login() {
    let app = test_app(
        MockDingTalkClient::default(),
        MemoryUserRepository::default(),
    )
    .await;

    let sync_request = test::TestRequest::post()
        .uri("/api/dingtalk/sync")
        .to_request();
    let response = test::call_service(&app, sync_request).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthNotLogin);
}

#[actix_web::test]
async fn admin_can_run_dingtalk_sync() {
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
    let dingtalk = MockDingTalkClient::default()
        .with_login_identity(
            "admin-code",
            DingTalkLoginIdentity {
                dingtalk_user_id: "admin".to_string(),
                union_id: None,
            },
        )
        .with_departments(vec![DingTalkDepartment {
            dingtalk_dept_id: 100,
            parent_dingtalk_dept_id: None,
            name: "研发部".to_string(),
            order_no: Some(1),
        }])
        .with_department_users(
            100,
            vec![
                DingTalkUser {
                    dingtalk_user_id: "admin".to_string(),
                    union_id: None,
                    name: "管理员".to_string(),
                    avatar_url: None,
                    mobile: None,
                },
                DingTalkUser {
                    dingtalk_user_id: "synced-user".to_string(),
                    union_id: Some("union-synced".to_string()),
                    name: "同步用户".to_string(),
                    avatar_url: None,
                    mobile: None,
                },
            ],
        );
    let app = test_app(dingtalk, users.clone()).await;
    let session_cookie = login_cookie(&app, "admin-code").await;

    let sync_request = test::TestRequest::post()
        .uri("/api/dingtalk/sync")
        .cookie(session_cookie)
        .to_request();
    let response = test::call_service(&app, sync_request).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["created_users"], 1);
    assert_eq!(body["updated_users"], 0);
    assert_eq!(body["disabled_users"], 0);
    assert!(users
        .find_by_dingtalk_user_id("synced-user")
        .await
        .is_some());
}

#[actix_web::test]
async fn admin_can_view_dingtalk_sync_logs_and_employee_cannot() {
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
    let dingtalk = MockDingTalkClient::default()
        .with_login_identity(
            "admin-code",
            DingTalkLoginIdentity {
                dingtalk_user_id: "admin".to_string(),
                union_id: None,
            },
        )
        .with_login_identity(
            "employee-code",
            DingTalkLoginIdentity {
                dingtalk_user_id: "employee".to_string(),
                union_id: None,
            },
        )
        .with_departments(vec![DingTalkDepartment {
            dingtalk_dept_id: 100,
            parent_dingtalk_dept_id: None,
            name: "研发部".to_string(),
            order_no: Some(1),
        }])
        .with_department_users(
            100,
            vec![DingTalkUser {
                dingtalk_user_id: "admin".to_string(),
                union_id: None,
                name: "管理员".to_string(),
                avatar_url: None,
                mobile: None,
            }],
        );
    let app = test_app(dingtalk, users).await;
    let admin_cookie = login_cookie(&app, "admin-code").await;
    let employee_cookie = login_cookie(&app, "employee-code").await;

    let sync_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/dingtalk/sync")
            .cookie(admin_cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(sync_response.status(), StatusCode::OK);

    let employee_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/dingtalk/sync-logs")
            .cookie(employee_cookie)
            .to_request(),
    )
    .await;
    assert_eq!(employee_response.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(employee_response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthForbidden);

    let logs_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/dingtalk/sync-logs")
            .cookie(admin_cookie)
            .to_request(),
    )
    .await;
    assert_eq!(logs_response.status(), StatusCode::OK);
    let logs: serde_json::Value = test::read_body_json(logs_response).await;
    assert_eq!(logs[0]["status"], "success");
    assert_eq!(logs[0]["created_users"], 0);
    assert_eq!(logs[0]["updated_users"], 0);
}

#[actix_web::test]
async fn dingtalk_sync_failure_is_recorded_in_sync_logs() {
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
    let dingtalk = MockDingTalkClient::default()
        .with_login_identity(
            "admin-code",
            DingTalkLoginIdentity {
                dingtalk_user_id: "admin".to_string(),
                union_id: None,
            },
        )
        .with_sync_failure();
    let app = test_app(dingtalk, users).await;
    let admin_cookie = login_cookie(&app, "admin-code").await;

    let sync_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/dingtalk/sync")
            .cookie(admin_cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(sync_response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body: ApiErrorResponse = test::read_body_json(sync_response).await;
    assert_eq!(body.error.code, ApiErrorCode::DingtalkSyncFailed);

    let logs_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/dingtalk/sync-logs")
            .cookie(admin_cookie)
            .to_request(),
    )
    .await;
    assert_eq!(logs_response.status(), StatusCode::OK);
    let logs: serde_json::Value = test::read_body_json(logs_response).await;
    assert_eq!(logs[0]["status"], "failed");
    assert_eq!(logs[0]["failure_reason"], "dingtalk sync failed");
}

async fn test_app(
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
    let request = test::TestRequest::post()
        .uri("/api/auth/dingtalk/login")
        .set_json(json!({ "code": code }))
        .to_request();
    let response = test::call_service(app, request).await;
    assert_eq!(response.status(), StatusCode::OK);
    response
        .response()
        .cookies()
        .find(|cookie| cookie.name() == "costrategy_session")
        .expect("login should set session cookie")
        .into_owned()
}
