use costrategy_backend::config::AppConfig;
use costrategy_backend::error::{ApiErrorCode, AppError};
use std::path::Path;

#[test]
fn parses_standard_environment_variables() {
    let config = AppConfig::from_env_vars([
        (
            "DATABASE_URL",
            "postgres://task_user:p%40ss%20word@10.0.0.2:5432/costrategy",
        ),
        ("RUSTFS_ENDPOINT", "10.0.0.4:9000"),
        ("RUSTFS_REGION", "cn-east-1"),
        ("RUSTFS_BUCKET", "costrategy-files"),
        ("RUSTFS_ACCESS_KEY_ID", "rustfs-access"),
        ("RUSTFS_SECRET_ACCESS_KEY", "rustfs-secret"),
    ])
    .expect("standard env vars should parse");

    assert_eq!(config.database.host, "10.0.0.2");
    assert_eq!(config.database.port, 5432);
    assert_eq!(config.database.user, "task_user");
    assert_eq!(config.database.password, "p@ss word");
    assert_eq!(config.database.db, "costrategy");
    assert_eq!(config.rustfs.endpoint, "10.0.0.4:9000");
    assert_eq!(config.rustfs.region, "cn-east-1");
    assert_eq!(config.rustfs.bucket, "costrategy-files");
    assert_eq!(config.rustfs.access_key_id, "rustfs-access");
    assert_eq!(config.rustfs.secret_access_key, "rustfs-secret");
}

#[test]
fn parses_standard_dotenv_file() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let path = dir.path().join(".env");
    std::fs::write(
        &path,
        r#"
DATABASE_URL=postgres://task_user:p%40ss%20word@10.0.0.2:5432/costrategy
RUSTFS_ENDPOINT=10.0.0.4:9000
RUSTFS_REGION=cn-east-1
RUSTFS_BUCKET=costrategy-files
RUSTFS_ACCESS_KEY_ID=rustfs-access
RUSTFS_SECRET_ACCESS_KEY=rustfs-secret
"#,
    )
    .expect("dotenv file should be written");

    let config = AppConfig::from_dotenv_file(&path).expect("standard .env file should parse");

    assert_eq!(config.database.host, "10.0.0.2");
    assert_eq!(config.database.port, 5432);
    assert_eq!(config.database.user, "task_user");
    assert_eq!(config.database.password, "p@ss word");
    assert_eq!(config.database.db, "costrategy");
    assert_eq!(config.rustfs.endpoint, "10.0.0.4:9000");
    assert_eq!(config.rustfs.region, "cn-east-1");
    assert_eq!(config.rustfs.bucket, "costrategy-files");
    assert_eq!(config.rustfs.access_key_id, "rustfs-access");
    assert_eq!(config.rustfs.secret_access_key, "rustfs-secret");
}

#[test]
fn later_environment_variables_override_earlier_dotenv_values() {
    let config = AppConfig::from_env_vars([
        (
            "DATABASE_URL",
            "postgres://task_user:dotenv-secret@10.0.0.2:5432/costrategy",
        ),
        ("RUSTFS_ENDPOINT", "10.0.0.4:9000"),
        ("RUSTFS_REGION", "cn-east-1"),
        ("RUSTFS_BUCKET", "dotenv-bucket"),
        ("RUSTFS_ACCESS_KEY_ID", "rustfs-access"),
        ("RUSTFS_SECRET_ACCESS_KEY", "rustfs-secret"),
        (
            "DATABASE_URL",
            "postgres://task_user:env-secret@10.0.0.2:5432/costrategy",
        ),
        ("RUSTFS_BUCKET", "env-bucket"),
    ])
    .expect("merged environment variables should parse");

    assert_eq!(config.database.password, "env-secret");
    assert_eq!(config.rustfs.bucket, "env-bucket");
}

#[test]
fn parses_sectioned_local_env_without_standard_equals_syntax() {
    let config = AppConfig::from_local_env_text(
        r#"
# postgreSQL
host 10.0.0.2
port 5432
user task_user
password p@ss word
db costrategy

# Redis
host 10.0.0.3
port 6379
password redis-secret
db 0

# rustfs
Endpoint：10.0.0.4:9000
Region cn-east-1
Bucket costrategy-files
Access Key rustfs-access
Secret Key rustfs-secret
"#,
    )
    .expect("sectioned local_ENV should parse");

    assert_eq!(config.database.host, "10.0.0.2");
    assert_eq!(config.database.port, 5432);
    assert_eq!(config.database.user, "task_user");
    assert_eq!(config.database.password, "p@ss word");
    assert_eq!(config.database.db, "costrategy");
    assert_eq!(
        config.database.url(),
        "postgres://task_user:p%40ss%20word@10.0.0.2:5432/costrategy"
    );
    assert_eq!(config.rustfs.endpoint, "10.0.0.4:9000");
    assert_eq!(config.rustfs.region, "cn-east-1");
    assert_eq!(config.rustfs.bucket, "costrategy-files");
    assert_eq!(config.rustfs.access_key_id, "rustfs-access");
    assert_eq!(config.rustfs.secret_access_key, "rustfs-secret");
}

#[test]
fn config_error_names_missing_fields_without_leaking_values() {
    let error = AppConfig::from_local_env_text(
        r#"
# postgreSQL
host 10.0.0.2
port 5432
user task_user
password super-secret

# rustfs
Endpoint：10.0.0.4:9000
Region cn-east-1
Bucket costrategy-files
Access Key rustfs-access
"#,
    )
    .expect_err("missing db and secret key should fail");

    let message = error.to_string();
    assert!(message.contains("postgresql.db"));
    assert!(message.contains("rustfs.secret_key"));
    assert!(!message.contains("super-secret"));
    assert!(!message.contains("rustfs-access"));
}

#[test]
fn reads_actual_local_env_file_when_present() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../local_ENV");
    let config = AppConfig::from_local_env_file(&path).expect("local_ENV should parse");

    assert!(!config.database.host.is_empty());
    assert!(config.database.port > 0);
    assert!(!config.database.user.is_empty());
    assert!(!config.database.password.is_empty());
    assert!(!config.database.db.is_empty());
    assert!(!config.rustfs.endpoint.is_empty());
    assert!(!config.rustfs.region.is_empty());
    assert!(!config.rustfs.bucket.is_empty());
    assert!(!config.rustfs.access_key_id.is_empty());
    assert!(!config.rustfs.secret_access_key.is_empty());
}

#[test]
fn api_errors_serialize_stable_codes_for_frontend() {
    let error = AppError::forbidden(ApiErrorCode::TaskNotAssignee);
    let body = error.body();
    let json = serde_json::to_value(&body).expect("error body should serialize");

    assert_eq!(json["error"]["code"], "TASK_NOT_ASSIGNEE");
    assert_eq!(json["error"]["message"], "只能更新自己负责的任务");
    assert!(json["error"].get("details").is_none());
    assert_eq!(error.status_code().as_u16(), 403);
}
