use costrategy_backend::auth::UserRole;
use costrategy_backend::dingtalk::{
    DingTalkDepartment, DingTalkUser, DingtalkSyncService, MockDingTalkClient,
};
use costrategy_backend::error::ApiErrorCode;
use costrategy_backend::users::{MemoryUserRepository, UserStatus};

#[tokio::test]
async fn sync_contacts_upserts_departments_users_relations_and_success_log() {
    let users = MemoryUserRepository::default();
    let client = MockDingTalkClient::default()
        .with_departments(vec![
            DingTalkDepartment {
                dingtalk_dept_id: 1,
                parent_dingtalk_dept_id: None,
                name: "总部".to_string(),
                order_no: Some(1),
            },
            DingTalkDepartment {
                dingtalk_dept_id: 2,
                parent_dingtalk_dept_id: Some(1),
                name: "研发部".to_string(),
                order_no: Some(2),
            },
        ])
        .with_department_users(
            2,
            vec![
                DingTalkUser {
                    dingtalk_user_id: "ding-user-1".to_string(),
                    union_id: Some("union-1".to_string()),
                    name: "张三".to_string(),
                    avatar_url: None,
                    mobile: Some("13800000000".to_string()),
                },
                DingTalkUser {
                    dingtalk_user_id: "ding-user-2".to_string(),
                    union_id: Some("union-2".to_string()),
                    name: "李四".to_string(),
                    avatar_url: None,
                    mobile: None,
                },
            ],
        );
    let service = DingtalkSyncService::new(client, users.clone());

    let result = service.sync_contacts().await.unwrap();

    assert_eq!(result.created_users, 2);
    assert_eq!(result.updated_users, 0);
    assert_eq!(result.disabled_users, 0);
    assert_eq!(users.department_count().await, 2);
    assert_eq!(users.department_user_count().await, 2);

    let synced_user = users.find_by_dingtalk_user_id("ding-user-1").await.unwrap();
    assert_eq!(synced_user.name, "张三");
    assert_eq!(synced_user.role, UserRole::Employee);
    assert_eq!(synced_user.status, UserStatus::Active);

    let logs = users.sync_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].status, "success");
    assert_eq!(logs[0].created_users, 2);
}

#[tokio::test]
async fn sync_contacts_updates_existing_user_and_disables_missing_user() {
    let users = MemoryUserRepository::default();
    let first_client = MockDingTalkClient::default()
        .with_departments(vec![DingTalkDepartment {
            dingtalk_dept_id: 2,
            parent_dingtalk_dept_id: None,
            name: "研发部".to_string(),
            order_no: None,
        }])
        .with_department_users(
            2,
            vec![
                DingTalkUser {
                    dingtalk_user_id: "ding-user-1".to_string(),
                    union_id: Some("union-1".to_string()),
                    name: "张三".to_string(),
                    avatar_url: None,
                    mobile: None,
                },
                DingTalkUser {
                    dingtalk_user_id: "ding-user-2".to_string(),
                    union_id: Some("union-2".to_string()),
                    name: "李四".to_string(),
                    avatar_url: None,
                    mobile: None,
                },
            ],
        );
    DingtalkSyncService::new(first_client, users.clone())
        .sync_contacts()
        .await
        .unwrap();

    let second_client = MockDingTalkClient::default()
        .with_departments(vec![DingTalkDepartment {
            dingtalk_dept_id: 2,
            parent_dingtalk_dept_id: None,
            name: "研发部".to_string(),
            order_no: None,
        }])
        .with_department_users(
            2,
            vec![DingTalkUser {
                dingtalk_user_id: "ding-user-1".to_string(),
                union_id: Some("union-1".to_string()),
                name: "张三丰".to_string(),
                avatar_url: Some("https://example.test/new.png".to_string()),
                mobile: Some("13900000000".to_string()),
            }],
        );

    let result = DingtalkSyncService::new(second_client, users.clone())
        .sync_contacts()
        .await
        .unwrap();

    assert_eq!(result.created_users, 0);
    assert_eq!(result.updated_users, 1);
    assert_eq!(result.disabled_users, 1);

    let active_user = users.find_by_dingtalk_user_id("ding-user-1").await.unwrap();
    assert_eq!(active_user.name, "张三丰");
    assert_eq!(active_user.status, UserStatus::Active);

    let disabled_user = users.find_by_dingtalk_user_id("ding-user-2").await.unwrap();
    assert_eq!(disabled_user.status, UserStatus::Disabled);
}

#[tokio::test]
async fn sync_contacts_records_failed_log_when_dingtalk_call_fails() {
    let users = MemoryUserRepository::default();
    let service = DingtalkSyncService::new(
        MockDingTalkClient::default().with_sync_failure(),
        users.clone(),
    );

    let error = service.sync_contacts().await.unwrap_err();

    assert_eq!(error.code(), ApiErrorCode::DingtalkSyncFailed);
    let logs = users.sync_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].status, "failed");
    assert_eq!(logs[0].created_users, 0);
    assert_eq!(logs[0].updated_users, 0);
    assert_eq!(logs[0].disabled_users, 0);
    assert!(logs[0]
        .failure_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("dingtalk sync failed")));
}
