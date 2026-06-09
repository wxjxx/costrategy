use actix_web::{http::StatusCode, test, App};

#[actix_web::test]
async fn openapi_json_documents_current_backend_routes() {
    let app = test::init_service(App::new().configure(costrategy_backend::routes::configure)).await;
    let request = test::TestRequest::get()
        .uri("/api-docs/openapi.json")
        .to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(response).await;
    assert_eq!(body["openapi"], "3.0.3");
    assert_eq!(body["info"]["title"], "项目管理系统 API");
    assert!(body["paths"]["/api/tasks"]["get"].is_object());
    assert!(body["paths"]["/api/tasks"]["post"].is_object());
    assert!(body["paths"]["/api/tasks/{task_id}/attachments"]["post"].is_object());
    assert!(body["paths"]["/api/notification-records"]["get"].is_object());
    assert!(body["components"]["securitySchemes"]["cookieAuth"].is_object());
}

#[actix_web::test]
async fn swagger_ui_serves_page_pointing_to_openapi_json() {
    let app = test::init_service(App::new().configure(costrategy_backend::routes::configure)).await;
    let request = test::TestRequest::get().uri("/swagger-ui").to_request();

    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = test::read_body(response).await;
    let text = std::str::from_utf8(&body).expect("swagger ui should be utf8 html");
    assert!(text.contains("SwaggerUIBundle"));
    assert!(text.contains("/api-docs/openapi.json"));
}
