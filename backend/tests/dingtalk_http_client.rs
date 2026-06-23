use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use costrategy_backend::config::DingtalkConfig;
use costrategy_backend::dingtalk::{DingTalkClient, DingtalkHttpClient};
use serde_json::{json, Value};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[actix_web::test]
async fn dingtalk_http_client_exchanges_login_code_and_reuses_access_token() {
    let state = Arc::new(TestDingtalkState::default());
    let (base_url, server) = start_test_server(state.clone());
    let server_handle = server.handle();
    actix_web::rt::spawn(server);
    let client = DingtalkHttpClient::new(DingtalkConfig {
        corp_id: "corp-id".to_string(),
        client_id: "client-id".to_string(),
        client_secret: "client-secret".to_string(),
        agent_id: 123456,
        oapi_base_url: base_url,
    });

    let identity = client.exchange_login_code("login-code").await.unwrap();
    client
        .send_work_notification("ding-user-1", "任务提醒")
        .await
        .unwrap();

    assert_eq!(identity.dingtalk_user_id, "ding-user-1");
    assert_eq!(identity.union_id.as_deref(), Some("union-1"));
    assert_eq!(state.token_requests.load(Ordering::SeqCst), 1);
    assert_eq!(state.login_requests.load(Ordering::SeqCst), 1);
    assert_eq!(state.notification_requests.load(Ordering::SeqCst), 1);
    server_handle.stop(true).await;
}

#[actix_web::test]
async fn dingtalk_http_client_lists_departments_and_department_users() {
    let state = Arc::new(TestDingtalkState::default());
    let (base_url, server) = start_test_server(state.clone());
    let server_handle = server.handle();
    actix_web::rt::spawn(server);
    let client = DingtalkHttpClient::new(DingtalkConfig {
        corp_id: "corp-id".to_string(),
        client_id: "client-id".to_string(),
        client_secret: "client-secret".to_string(),
        agent_id: 123456,
        oapi_base_url: base_url,
    });

    let departments = client.list_departments().await.unwrap();
    let users = client.list_users_by_department(2).await.unwrap();

    assert_eq!(departments.len(), 1);
    assert_eq!(departments[0].dingtalk_dept_id, 2);
    assert_eq!(departments[0].parent_dingtalk_dept_id, Some(1));
    assert_eq!(departments[0].name, "研发部");
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].dingtalk_user_id, "ding-user-1");
    assert_eq!(users[0].union_id.as_deref(), Some("union-1"));
    assert_eq!(state.token_requests.load(Ordering::SeqCst), 1);
    assert_eq!(state.department_requests.load(Ordering::SeqCst), 2);
    assert_eq!(state.user_list_requests.load(Ordering::SeqCst), 1);
    server_handle.stop(true).await;
}

#[actix_web::test]
async fn dingtalk_http_client_preserves_api_failure_details() {
    let state = Arc::new(TestDingtalkState::default());
    let (base_url, server) = start_test_server(state.clone());
    let server_handle = server.handle();
    actix_web::rt::spawn(server);
    let client = DingtalkHttpClient::new(DingtalkConfig {
        corp_id: "corp-id".to_string(),
        client_id: "client-id".to_string(),
        client_secret: "client-secret".to_string(),
        agent_id: 123456,
        oapi_base_url: base_url,
    });

    let error = client
        .send_work_notification("missing-user", "任务提醒")
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "notify failed: errcode=33012, errmsg=userid invalid"
    );
    server_handle.stop(true).await;
}

#[derive(Default)]
struct TestDingtalkState {
    token_requests: AtomicUsize,
    login_requests: AtomicUsize,
    notification_requests: AtomicUsize,
    department_requests: AtomicUsize,
    user_list_requests: AtomicUsize,
}

fn start_test_server(state: Arc<TestDingtalkState>) -> (String, actix_web::dev::Server) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .route("/gettoken", web::get().to(get_token))
            .route("/topapi/v2/user/getuserinfo", web::post().to(get_user_info))
            .route(
                "/topapi/message/corpconversation/asyncsend_v2",
                web::post().to(send_work_notification),
            )
            .route(
                "/topapi/v2/department/listsub",
                web::post().to(list_sub_departments),
            )
            .route("/topapi/v2/user/list", web::post().to(list_users))
    })
    .listen(listener)
    .unwrap()
    .run();

    (format!("http://{addr}"), server)
}

async fn get_token(state: web::Data<Arc<TestDingtalkState>>, request: HttpRequest) -> HttpResponse {
    state.token_requests.fetch_add(1, Ordering::SeqCst);
    if !request.query_string().contains("appkey=client-id")
        || !request.query_string().contains("appsecret=client-secret")
    {
        return HttpResponse::BadRequest().finish();
    }

    HttpResponse::Ok().json(json!({
        "errcode": 0,
        "access_token": "token-1",
        "expires_in": 7200
    }))
}

async fn get_user_info(
    state: web::Data<Arc<TestDingtalkState>>,
    request: HttpRequest,
    body: web::Json<Value>,
) -> HttpResponse {
    state.login_requests.fetch_add(1, Ordering::SeqCst);
    if !request.query_string().contains("access_token=token-1") || body["code"] != "login-code" {
        return HttpResponse::BadRequest().finish();
    }

    HttpResponse::Ok().json(json!({
        "errcode": 0,
        "result": {
            "userid": "ding-user-1",
            "unionid": "union-1"
        }
    }))
}

async fn send_work_notification(
    state: web::Data<Arc<TestDingtalkState>>,
    request: HttpRequest,
    body: web::Json<Value>,
) -> HttpResponse {
    state.notification_requests.fetch_add(1, Ordering::SeqCst);
    if !request.query_string().contains("access_token=token-1")
        || body["agent_id"] != 123456
        || body["msg"]["text"]["content"] != "任务提醒"
    {
        return HttpResponse::BadRequest().finish();
    }
    if body["userid_list"] != "ding-user-1" {
        return HttpResponse::Ok().json(json!({
            "errcode": 33012,
            "errmsg": "userid invalid"
        }));
    }

    HttpResponse::Ok().json(json!({
        "errcode": 0,
        "result": {
            "task_id": 42
        }
    }))
}

async fn list_sub_departments(
    state: web::Data<Arc<TestDingtalkState>>,
    request: HttpRequest,
    body: web::Json<Value>,
) -> HttpResponse {
    state.department_requests.fetch_add(1, Ordering::SeqCst);
    if !request.query_string().contains("access_token=token-1") {
        return HttpResponse::BadRequest().finish();
    }

    let result = match body["dept_id"].as_i64() {
        Some(1) => json!([
            {
                "dept_id": 2,
                "parent_id": 1,
                "name": "研发部",
                "order": 2
            }
        ]),
        Some(2) => json!([]),
        _ => return HttpResponse::BadRequest().finish(),
    };

    HttpResponse::Ok().json(json!({
        "errcode": 0,
        "result": result
    }))
}

async fn list_users(
    state: web::Data<Arc<TestDingtalkState>>,
    request: HttpRequest,
    body: web::Json<Value>,
) -> HttpResponse {
    state.user_list_requests.fetch_add(1, Ordering::SeqCst);
    if !request.query_string().contains("access_token=token-1")
        || body["dept_id"] != 2
        || body["cursor"] != 0
        || body["size"] != 100
    {
        return HttpResponse::BadRequest().finish();
    }

    HttpResponse::Ok().json(json!({
        "errcode": 0,
        "result": {
            "list": [
                {
                    "userid": "ding-user-1",
                    "unionid": "union-1",
                    "name": "张三",
                    "avatar": "https://example.test/avatar.png",
                    "mobile": "13800000000"
                }
            ],
            "has_more": false
        }
    }))
}
