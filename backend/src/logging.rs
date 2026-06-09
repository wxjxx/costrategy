use actix_web::middleware::Logger;
use env_logger::Env;

pub const DEFAULT_LOG_FILTER: &str = "info,actix_web=info,actix_server=info";
pub const REQUEST_LOG_FORMAT: &str = r#"%a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#;

pub fn init_logging() {
    let env = Env::default().filter_or("RUST_LOG", DEFAULT_LOG_FILTER);
    let _ = env_logger::Builder::from_env(env).try_init();
}

pub fn request_logger() -> Logger {
    Logger::new(REQUEST_LOG_FORMAT)
}
