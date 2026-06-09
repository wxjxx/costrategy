use actix_web::{http::StatusCode, test, App};
use serde_json::json;

#[actix_web::test]
async fn health_endpoint_returns_ok_status() {
    let app = test::init_service(App::new().configure(costrategy_backend::routes::configure)).await;
    let request = test::TestRequest::get().uri("/api/health").to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body, json!({ "status": "ok" }));
}
