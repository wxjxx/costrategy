use costrategy_backend::auth::{DingtalkAuthService, UserRole};
use costrategy_backend::dingtalk::{
    ConfiguredDingTalkClient, DingTalkLoginIdentity, MockDingTalkClient,
};
use costrategy_backend::error::ApiErrorCode;
use costrategy_backend::users::{MemoryUserRepository, NewUser, UserStatus};

#[tokio::test]
async fn dingtalk_login_returns_active_synced_user() {
    let users = MemoryUserRepository::default();
    let user = users
        .insert_user(NewUser {
            dingtalk_user_id: "ding-user-1".to_string(),
            union_id: Some("union-1".to_string()),
            name: "张三".to_string(),
            avatar_url: Some("https://example.test/avatar.png".to_string()),
            mobile: Some("13800000000".to_string()),
            role: UserRole::Employee,
            status: UserStatus::Active,
        })
        .await;
    let client = MockDingTalkClient::default().with_login_identity(
        "valid-code",
        DingTalkLoginIdentity {
            dingtalk_user_id: "ding-user-1".to_string(),
            union_id: Some("union-1".to_string()),
        },
    );
    let service = DingtalkAuthService::new(client, users);

    let current_user = service.login_with_code("valid-code").await.unwrap();

    assert_eq!(current_user.id, user.id);
    assert_eq!(current_user.name, "张三");
    assert_eq!(current_user.role, UserRole::Employee);
}

#[tokio::test]
async fn dingtalk_login_rejects_user_missing_from_synced_contacts() {
    let users = MemoryUserRepository::default();
    let client = MockDingTalkClient::default().with_login_identity(
        "valid-code",
        DingTalkLoginIdentity {
            dingtalk_user_id: "not-synced".to_string(),
            union_id: Some("union-missing".to_string()),
        },
    );
    let service = DingtalkAuthService::new(client, users);

    let error = service.login_with_code("valid-code").await.unwrap_err();

    assert_eq!(error.status_code().as_u16(), 403);
    assert_eq!(error.code(), ApiErrorCode::AuthUserNotSynced);
}

#[tokio::test]
async fn dingtalk_login_rejects_disabled_user() {
    let users = MemoryUserRepository::default();
    users
        .insert_user(NewUser {
            dingtalk_user_id: "ding-user-1".to_string(),
            union_id: Some("union-1".to_string()),
            name: "李四".to_string(),
            avatar_url: None,
            mobile: None,
            role: UserRole::Manager,
            status: UserStatus::Disabled,
        })
        .await;
    let client = MockDingTalkClient::default().with_login_identity(
        "valid-code",
        DingTalkLoginIdentity {
            dingtalk_user_id: "ding-user-1".to_string(),
            union_id: Some("union-1".to_string()),
        },
    );
    let service = DingtalkAuthService::new(client, users);

    let error = service.login_with_code("valid-code").await.unwrap_err();

    assert_eq!(error.status_code().as_u16(), 403);
    assert_eq!(error.code(), ApiErrorCode::AuthUserDisabled);
}

#[tokio::test]
async fn dingtalk_login_maps_client_failure_to_stable_error_code() {
    let users = MemoryUserRepository::default();
    let client = MockDingTalkClient::default();
    let service = DingtalkAuthService::new(client, users);

    let error = service.login_with_code("unknown-code").await.unwrap_err();

    assert_eq!(error.status_code().as_u16(), 401);
    assert_eq!(error.code(), ApiErrorCode::AuthDingtalkLoginFailed);
}

#[tokio::test]
async fn dingtalk_login_reports_missing_runtime_config() {
    let users = MemoryUserRepository::default();
    let service = DingtalkAuthService::new(ConfiguredDingTalkClient::missing(), users);

    let error = service.login_with_code("valid-code").await.unwrap_err();

    assert_eq!(error.status_code().as_u16(), 500);
    assert_eq!(error.code(), ApiErrorCode::DingtalkConfigMissing);
}
