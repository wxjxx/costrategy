use costrategy_backend::config::AppConfig;

#[tokio::main]
async fn main() {
    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("config failed: {error}");
            std::process::exit(1);
        }
    };

    println!("config ok");

    if let Err(error) = check_postgres(&config).await {
        eprintln!("postgres failed: {}", error.safe_message());
        std::process::exit(1);
    }
    println!("postgres ok");

    if let Err(error) = check_rustfs(&config).await {
        eprintln!("rustfs failed: {}", error.safe_message());
        std::process::exit(1);
    }
    println!("rustfs ok");
}

async fn check_postgres(config: &AppConfig) -> Result<(), CheckError> {
    let pool = sqlx::PgPool::connect(&config.database.url())
        .await
        .map_err(|_| CheckError::PostgresConnection)?;
    sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(&pool)
        .await
        .map_err(|_| CheckError::PostgresQuery)?;
    Ok(())
}

async fn check_rustfs(config: &AppConfig) -> Result<(), CheckError> {
    let credentials = aws_credential_types::Credentials::new(
        config.rustfs.access_key_id.clone(),
        config.rustfs.secret_access_key.clone(),
        None,
        None,
        "local_ENV",
    );
    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(config.rustfs.region.clone()))
        .credentials_provider(credentials)
        .load()
        .await;

    let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
        .endpoint_url(normalize_endpoint(&config.rustfs.endpoint))
        .force_path_style(true)
        .build();
    let client = aws_sdk_s3::Client::from_conf(s3_config);

    client
        .head_bucket()
        .bucket(&config.rustfs.bucket)
        .send()
        .await
        .map_err(|_| CheckError::RustfsHeadBucket)?;

    Ok(())
}

fn normalize_endpoint(endpoint: &str) -> String {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        endpoint.to_string()
    } else {
        format!("http://{endpoint}")
    }
}

enum CheckError {
    PostgresConnection,
    PostgresQuery,
    RustfsHeadBucket,
}

impl CheckError {
    fn safe_message(&self) -> &'static str {
        match self {
            Self::PostgresConnection => "connection check failed",
            Self::PostgresQuery => "query check failed",
            Self::RustfsHeadBucket => "bucket check failed",
        }
    }
}
