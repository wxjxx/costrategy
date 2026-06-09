use actix_web::cookie::Cookie;
use actix_web::{http::StatusCode, test, web, App};
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::{SessionStore, UserRole};
use costrategy_backend::dingtalk::{DingTalkLoginIdentity, MockDingTalkClient};
use costrategy_backend::error::{ApiErrorCode, ApiErrorResponse};
use costrategy_backend::notifications::{
    MemoryNotificationRepository, NewNotificationRecord, NotificationRepository,
    NotificationStatus, NotificationType,
};
use costrategy_backend::users::{MemoryUserRepository, NewUser, UserStatus};

#[actix_web::test]
async fn admin_can_list_notification_records_and_manager_cannot() {
    let fixture = NotificationRouteFixture::new().await;
    let app = notification_test_app(&fixture).await;
    let admin_cookie = login_cookie(&app, "admin-code").await;
    let manager_cookie = login_cookie(&app, "manager-code").await;
    fixture
        .notifications
        .create_record(NewNotificationRecord {
            notification_type: NotificationType::TaskAssigned,
            receiver_id: fixture.employee_id,
            task_id: None,
            content_summary: "新任务分配".to_string(),
            status: NotificationStatus::Success,
            failure_reason: None,
            dedupe_date: None,
        })
        .await
        .unwrap();

    let manager_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/notification-records")
            .cookie(manager_cookie)
            .to_request(),
    )
    .await;
    assert_eq!(manager_response.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(manager_response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthForbidden);

    let admin_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/notification-records")
            .cookie(admin_cookie)
            .to_request(),
    )
    .await;
    assert_eq!(admin_response.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(admin_response).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["notification_type"], "task_assigned");
    assert_eq!(body[0]["status"], "success");
    assert_eq!(body[0]["content_summary"], "新任务分配");
}

#[actix_web::test]
async fn admin_can_list_and_update_notification_rules() {
    let fixture = NotificationRouteFixture::new().await;
    let app = notification_test_app(&fixture).await;
    let admin_cookie = login_cookie(&app, "admin-code").await;
    let manager_cookie = login_cookie(&app, "manager-code").await;

    let manager_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/notification-rules")
            .cookie(manager_cookie)
            .to_request(),
    )
    .await;
    assert_eq!(manager_response.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(manager_response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthForbidden);

    let list_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/notification-rules")
            .cookie(admin_cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(list_response.status(), StatusCode::OK);
    let rules: serde_json::Value = test::read_body_json(list_response).await;
    assert_eq!(rules.as_array().unwrap().len(), 4);
    assert!(rules
        .as_array()
        .unwrap()
        .iter()
        .any(|rule| rule["rule_type"] == "task_overdue" && rule["enabled"] == true));

    let update_response = test::call_service(
        &app,
        test::TestRequest::patch()
            .uri("/api/notification-rules/task_overdue")
            .cookie(admin_cookie)
            .set_json(serde_json::json!({ "enabled": false }))
            .to_request(),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: serde_json::Value = test::read_body_json(update_response).await;
    assert_eq!(updated["rule_type"], "task_overdue");
    assert_eq!(updated["enabled"], false);
    assert_eq!(updated["updated_by"], fixture.admin_id.to_string());
}

struct NotificationRouteFixture {
    dingtalk: MockDingTalkClient,
    users: MemoryUserRepository,
    notifications: MemoryNotificationRepository,
    admin_id: uuid::Uuid,
    employee_id: uuid::Uuid,
}

impl NotificationRouteFixture {
    async fn new() -> Self {
        let users = MemoryUserRepository::default();
        let admin = users
            .insert_user(new_user("admin", "管理员", UserRole::Admin))
            .await;
        users
            .insert_user(new_user("manager", "管理人员", UserRole::Manager))
            .await;
        let employee = users
            .insert_user(new_user("employee", "员工", UserRole::Employee))
            .await;
        let dingtalk = MockDingTalkClient::default()
            .with_login_identity("admin-code", identity("admin"))
            .with_login_identity("manager-code", identity("manager"));

        Self {
            dingtalk,
            users,
            notifications: MemoryNotificationRepository::default(),
            admin_id: admin.id,
            employee_id: employee.id,
        }
    }
}

async fn notification_test_app(
    fixture: &NotificationRouteFixture,
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
            .app_data(web::Data::new(fixture.notifications.clone()))
            .configure(costrategy_backend::routes::configure_app::<
                MockDingTalkClient,
                MemoryUserRepository,
            >)
            .configure(costrategy_backend::routes::configure_notification_routes::<
                MockDingTalkClient,
                MemoryUserRepository,
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
            .set_json(serde_json::json!({ "code": code }))
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
