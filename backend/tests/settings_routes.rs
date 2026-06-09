use actix_web::cookie::Cookie;
use actix_web::{http::StatusCode, test, web, App};
use costrategy_backend::app_state::AppState;
use costrategy_backend::auth::{SessionStore, UserRole};
use costrategy_backend::config::{AppConfig, DatabaseConfig, DingtalkConfig, RustfsConfig};
use costrategy_backend::dingtalk::{DingTalkLoginIdentity, MockDingTalkClient};
use costrategy_backend::error::{ApiErrorCode, ApiErrorResponse};
use costrategy_backend::settings::MemorySettingsRepository;
use costrategy_backend::users::{MemoryUserRepository, NewUser, UserStatus};
use serde_json::json;

#[actix_web::test]
async fn employee_cannot_read_system_settings() {
    let fixture = SettingsRouteFixture::new().await;
    let app = settings_test_app(&fixture).await;
    let cookie = login_cookie(&app, "employee-code").await;

    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/settings")
            .cookie(cookie)
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body: ApiErrorResponse = test::read_body_json(response).await;
    assert_eq!(body.error.code, ApiErrorCode::AuthForbidden);
}

#[actix_web::test]
async fn admin_can_read_and_update_masked_system_settings() {
    let fixture = SettingsRouteFixture::new().await;
    let app = settings_test_app(&fixture).await;
    let cookie = login_cookie(&app, "admin-code").await;

    let initial_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/settings")
            .cookie(cookie.clone())
            .to_request(),
    )
    .await;
    assert_eq!(initial_response.status(), StatusCode::OK);
    let initial: serde_json::Value = test::read_body_json(initial_response).await;
    let rustfs_endpoint = find_setting(&initial, "rustfs.endpoint");
    assert_eq!(rustfs_endpoint["group"], "rustfs");
    assert_eq!(rustfs_endpoint["value_masked"], "10.0.0.4:9000");
    assert_eq!(rustfs_endpoint["source"], "env");
    assert_eq!(initial["connection_status"]["rustfs"], "configured");
    assert_eq!(
        find_setting(&initial, "dingtalk.corp_id")["group"],
        "dingtalk"
    );
    assert_eq!(
        find_setting(&initial, "dingtalk.corp_id")["value_masked"],
        "ding-corp"
    );
    assert_eq!(
        find_setting(&initial, "dingtalk.client_secret")["value_masked"],
        "di***et"
    );

    let update_response = test::call_service(
        &app,
        test::TestRequest::put()
            .uri("/api/settings")
            .cookie(cookie.clone())
            .set_json(json!({
                "settings": [
                    { "key": "dingtalk.corp_id", "value": "ding-corp" },
                    { "key": "dingtalk.client_secret", "value": "super-secret-value" }
                ]
            }))
            .to_request(),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let update_body = test::read_body(update_response).await;
    let update_text = std::str::from_utf8(&update_body).unwrap();
    assert!(!update_text.contains("super-secret-value"));
    let updated: serde_json::Value = serde_json::from_slice(&update_body).unwrap();
    assert_eq!(
        find_setting(&updated, "dingtalk.client_secret")["value_masked"],
        "su***ue"
    );
    assert_eq!(
        find_setting(&updated, "dingtalk.client_secret")["source"],
        "database"
    );

    let read_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/settings")
            .cookie(cookie)
            .to_request(),
    )
    .await;
    assert_eq!(read_response.status(), StatusCode::OK);
    let read_body = test::read_body(read_response).await;
    let read_text = std::str::from_utf8(&read_body).unwrap();
    assert!(!read_text.contains("super-secret-value"));
}

struct SettingsRouteFixture {
    dingtalk: MockDingTalkClient,
    users: MemoryUserRepository,
    settings: MemorySettingsRepository,
    config: AppConfig,
}

impl SettingsRouteFixture {
    async fn new() -> Self {
        let users = MemoryUserRepository::default();
        users
            .insert_user(new_user("admin", "管理员", UserRole::Admin))
            .await;
        users
            .insert_user(new_user("employee", "普通员工", UserRole::Employee))
            .await;
        let dingtalk = MockDingTalkClient::default()
            .with_login_identity("admin-code", identity("admin"))
            .with_login_identity("employee-code", identity("employee"));

        Self {
            dingtalk,
            users,
            settings: MemorySettingsRepository::default(),
            config: test_config(),
        }
    }
}

async fn settings_test_app(
    fixture: &SettingsRouteFixture,
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
            .app_data(web::Data::new(fixture.settings.clone()))
            .app_data(web::Data::new(fixture.config.clone()))
            .configure(costrategy_backend::routes::configure_app::<
                MockDingTalkClient,
                MemoryUserRepository,
            >)
            .configure(costrategy_backend::routes::configure_settings_routes::<
                MockDingTalkClient,
                MemoryUserRepository,
                MemorySettingsRepository,
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

fn find_setting<'a>(body: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    body["settings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|setting| setting["key"] == key)
        .unwrap()
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

fn test_config() -> AppConfig {
    AppConfig {
        database: DatabaseConfig {
            host: "127.0.0.1".to_string(),
            port: 5432,
            user: "user".to_string(),
            password: "password".to_string(),
            db: "db".to_string(),
        },
        rustfs: RustfsConfig {
            endpoint: "10.0.0.4:9000".to_string(),
            region: "cn-east-1".to_string(),
            bucket: "costrategy-files".to_string(),
            access_key_id: "rustfs-access".to_string(),
            secret_access_key: "rustfs-secret".to_string(),
        },
        dingtalk: Some(DingtalkConfig {
            corp_id: "ding-corp".to_string(),
            client_id: "ding-client".to_string(),
            client_secret: "ding-secret".to_string(),
            agent_id: 123456,
            oapi_base_url: "https://oapi.dingtalk.com".to_string(),
        }),
        admin_auth_token: None,
    }
}
