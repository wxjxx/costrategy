use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub rustfs: RustfsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub db: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustfsConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {message}")]
    ReadFile { path: String, message: String },
    #[error("missing required config fields: {0}")]
    MissingFields(String),
    #[error("invalid config field {field}: {message}")]
    InvalidField {
        field: &'static str,
        message: String,
    },
}

#[derive(Default)]
struct ParsedConfig {
    postgresql_host: Option<String>,
    postgresql_port: Option<String>,
    postgresql_user: Option<String>,
    postgresql_password: Option<String>,
    postgresql_db: Option<String>,
    rustfs_endpoint: Option<String>,
    rustfs_region: Option<String>,
    rustfs_bucket: Option<String>,
    rustfs_access_key: Option<String>,
    rustfs_secret_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    PostgreSql,
    Rustfs,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let env_vars = env::vars().collect::<Vec<_>>();
        let dotenv_iter = match dotenvy::dotenv_iter() {
            Ok(iter) => iter,
            Err(err) if err.not_found() => return Self::from_env_vars(env_vars),
            Err(err) => {
                return Err(ConfigError::ReadFile {
                    path: ".env".to_string(),
                    message: err.to_string(),
                });
            }
        };
        let dotenv_vars = collect_dotenv_vars(".env".to_string(), dotenv_iter)?;

        Self::from_env_vars(dotenv_vars.into_iter().chain(env_vars))
    }

    pub fn from_env_vars<I, K, V>(vars: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        let mut parsed = ParsedConfig::default();
        for (key, value) in vars {
            read_standard_env_pair(key.as_ref(), value.into(), &mut parsed);
        }

        Self::from_parsed(parsed)
    }

    pub fn from_local_env_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let text = fs::read_to_string(path_ref).map_err(|err| ConfigError::ReadFile {
            path: path_ref.display().to_string(),
            message: err.to_string(),
        })?;
        Self::from_local_env_text(&text)
    }

    pub fn from_dotenv_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        let path = path_ref.display().to_string();
        let dotenv_iter =
            dotenvy::from_path_iter(path_ref).map_err(|err| ConfigError::ReadFile {
                path: path.clone(),
                message: err.to_string(),
            })?;
        let dotenv_vars = collect_dotenv_vars(path, dotenv_iter)?;

        Self::from_env_vars(dotenv_vars)
    }

    pub fn from_local_env_text(text: &str) -> Result<Self, ConfigError> {
        let mut parsed = ParsedConfig::default();
        let mut section = Section::None;

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(comment) = line.strip_prefix('#') {
                section = detect_section(comment);
                continue;
            }

            match section {
                Section::PostgreSql => read_postgresql_line(line, &mut parsed),
                Section::Rustfs => read_rustfs_line(line, &mut parsed),
                Section::None => read_standard_env_line(line, &mut parsed),
            }
        }

        Self::from_parsed(parsed)
    }

    fn from_parsed(parsed: ParsedConfig) -> Result<Self, ConfigError> {
        let mut missing = Vec::new();

        let host = required(parsed.postgresql_host, "postgresql.host", &mut missing);
        let port = required(parsed.postgresql_port, "postgresql.port", &mut missing);
        let user = required(parsed.postgresql_user, "postgresql.user", &mut missing);
        let password = required(
            parsed.postgresql_password,
            "postgresql.password",
            &mut missing,
        );
        let db = required(parsed.postgresql_db, "postgresql.db", &mut missing);
        let endpoint = required(parsed.rustfs_endpoint, "rustfs.endpoint", &mut missing);
        let region = required(parsed.rustfs_region, "rustfs.region", &mut missing);
        let bucket = required(parsed.rustfs_bucket, "rustfs.bucket", &mut missing);
        let access_key_id = required(parsed.rustfs_access_key, "rustfs.access_key", &mut missing);
        let secret_access_key =
            required(parsed.rustfs_secret_key, "rustfs.secret_key", &mut missing);

        if !missing.is_empty() {
            return Err(ConfigError::MissingFields(missing.join(", ")));
        }

        let port = port
            .expect("checked missing")
            .parse::<u16>()
            .map_err(|err| ConfigError::InvalidField {
                field: "postgresql.port",
                message: err.to_string(),
            })?;

        Ok(Self {
            database: DatabaseConfig {
                host: host.expect("checked missing"),
                port,
                user: user.expect("checked missing"),
                password: password.expect("checked missing"),
                db: db.expect("checked missing"),
            },
            rustfs: RustfsConfig {
                endpoint: endpoint.expect("checked missing"),
                region: region.expect("checked missing"),
                bucket: bucket.expect("checked missing"),
                access_key_id: access_key_id.expect("checked missing"),
                secret_access_key: secret_access_key.expect("checked missing"),
            },
        })
    }
}

fn collect_dotenv_vars<I>(path: String, vars: I) -> Result<Vec<(String, String)>, ConfigError>
where
    I: IntoIterator<Item = std::result::Result<(String, String), dotenvy::Error>>,
{
    vars.into_iter()
        .map(|item| {
            item.map_err(|err| ConfigError::ReadFile {
                path: path.clone(),
                message: err.to_string(),
            })
        })
        .collect()
}

impl DatabaseConfig {
    pub fn url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            urlencoding::encode(&self.user),
            urlencoding::encode(&self.password),
            self.host,
            self.port,
            urlencoding::encode(&self.db)
        )
    }
}

fn required(
    value: Option<String>,
    name: &'static str,
    missing: &mut Vec<&'static str>,
) -> Option<String> {
    if value.as_deref().is_none_or(str::is_empty) {
        missing.push(name);
    }
    value
}

fn detect_section(comment: &str) -> Section {
    let normalized = normalize_key(comment);
    if normalized.contains("postgresql") || normalized.contains("postgres") {
        Section::PostgreSql
    } else if normalized.contains("rustfs") {
        Section::Rustfs
    } else {
        Section::None
    }
}

fn read_postgresql_line(line: &str, parsed: &mut ParsedConfig) {
    if let Some((key, value)) = split_config_line(line) {
        match normalize_key(&key).as_str() {
            "host" => parsed.postgresql_host = Some(value),
            "port" => parsed.postgresql_port = Some(value),
            "user" | "username" => parsed.postgresql_user = Some(value),
            "password" => parsed.postgresql_password = Some(value),
            "db" | "database" | "database_name" => parsed.postgresql_db = Some(value),
            _ => {}
        }
    }
}

fn read_rustfs_line(line: &str, parsed: &mut ParsedConfig) {
    if let Some(value) = read_known_key(
        line,
        &["Secret Key", "SecretKey", "RUSTFS_SECRET_ACCESS_KEY"],
    ) {
        parsed.rustfs_secret_key = Some(value);
    } else if let Some(value) =
        read_known_key(line, &["Access Key", "AccessKey", "RUSTFS_ACCESS_KEY_ID"])
    {
        parsed.rustfs_access_key = Some(value);
    } else if let Some(value) = read_known_key(line, &["Endpoint", "RUSTFS_ENDPOINT"]) {
        parsed.rustfs_endpoint = Some(value);
    } else if let Some(value) = read_known_key(line, &["Region", "RUSTFS_REGION"]) {
        parsed.rustfs_region = Some(value);
    } else if let Some(value) = read_known_key(line, &["Bucket", "RUSTFS_BUCKET"]) {
        parsed.rustfs_bucket = Some(value);
    }
}

fn read_standard_env_line(line: &str, parsed: &mut ParsedConfig) {
    if let Some((key, value)) = split_config_line(line) {
        read_standard_env_pair(&key, value, parsed);
    }
}

fn read_standard_env_pair(key: &str, value: String, parsed: &mut ParsedConfig) {
    match normalize_key(key).as_str() {
        "database_url" => read_database_url(&value, parsed),
        "postgres_host" | "postgresql_host" | "database_host" => {
            parsed.postgresql_host = Some(value)
        }
        "postgres_port" | "postgresql_port" | "database_port" => {
            parsed.postgresql_port = Some(value)
        }
        "postgres_user" | "postgresql_user" | "database_user" => {
            parsed.postgresql_user = Some(value)
        }
        "postgres_password" | "postgresql_password" | "database_password" => {
            parsed.postgresql_password = Some(value)
        }
        "postgres_db" | "postgresql_db" | "database_name" | "database_db" => {
            parsed.postgresql_db = Some(value)
        }
        "rustfs_endpoint" => parsed.rustfs_endpoint = Some(value),
        "rustfs_region" => parsed.rustfs_region = Some(value),
        "rustfs_bucket" => parsed.rustfs_bucket = Some(value),
        "rustfs_access_key_id" => parsed.rustfs_access_key = Some(value),
        "rustfs_secret_access_key" => parsed.rustfs_secret_key = Some(value),
        _ => {}
    }
}

fn read_known_key(line: &str, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = strip_key(line, key) {
            return Some(value);
        }
    }
    None
}

fn strip_key(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed
        .to_ascii_lowercase()
        .starts_with(&key.to_ascii_lowercase())
    {
        return None;
    }

    let rest = trimmed[key.len()..].trim_start();
    let value = rest
        .strip_prefix('=')
        .or_else(|| rest.strip_prefix(':'))
        .or_else(|| rest.strip_prefix('：'))
        .unwrap_or(rest)
        .trim();

    if value.is_empty() {
        None
    } else {
        Some(unquote(value))
    }
}

fn split_config_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().strip_prefix("export ").unwrap_or(line.trim());
    for separator in ['=', '：', ':'] {
        if let Some((key, value)) = trimmed.split_once(separator) {
            let value = value.trim();
            if !key.trim().is_empty() && !value.is_empty() {
                return Some((key.trim().to_string(), unquote(value)));
            }
        }
    }

    let (key, value) = trimmed.split_once(char::is_whitespace)?;
    let value = value.trim();
    if key.trim().is_empty() || value.is_empty() {
        None
    } else {
        Some((key.trim().to_string(), unquote(value)))
    }
}

fn normalize_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn read_database_url(value: &str, parsed: &mut ParsedConfig) {
    let Some(without_scheme) = value
        .strip_prefix("postgres://")
        .or_else(|| value.strip_prefix("postgresql://"))
    else {
        return;
    };
    let Some((credentials, address_and_db)) = without_scheme.split_once('@') else {
        return;
    };
    let Some((user, password)) = credentials.split_once(':') else {
        return;
    };
    let Some((address, db)) = address_and_db.split_once('/') else {
        return;
    };
    let Some((host, port)) = address.rsplit_once(':') else {
        return;
    };

    parsed.postgresql_user = Some(
        urlencoding::decode(user)
            .unwrap_or_else(|_| user.into())
            .into_owned(),
    );
    parsed.postgresql_password = Some(
        urlencoding::decode(password)
            .unwrap_or_else(|_| password.into())
            .into_owned(),
    );
    parsed.postgresql_host = Some(host.to_string());
    parsed.postgresql_port = Some(port.to_string());
    parsed.postgresql_db = Some(
        urlencoding::decode(db)
            .unwrap_or_else(|_| db.into())
            .into_owned(),
    );
}
