use costrategy_backend::logging::{DEFAULT_LOG_FILTER, REQUEST_LOG_FORMAT};

#[test]
fn default_log_filter_enables_application_and_actix_info_logs() {
    assert_eq!(DEFAULT_LOG_FILTER, "info,actix_web=info,actix_server=info");
}

#[test]
fn request_log_format_includes_core_http_request_fields() {
    for field in ["%a", "%r", "%s", "%b", "%T"] {
        assert!(
            REQUEST_LOG_FORMAT.contains(field),
            "request log format should include {field}"
        );
    }
}
