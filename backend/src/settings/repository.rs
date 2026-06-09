use crate::config::AppConfig;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingGroup {
    Dingtalk,
    Rustfs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingSource {
    Database,
    Env,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub group: SettingGroup,
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SettingItem {
    pub key: String,
    pub label: String,
    pub group: SettingGroup,
    pub sensitive: bool,
    pub configured: bool,
    pub source: SettingSource,
    pub value_masked: Option<String>,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SettingsConnectionStatus {
    pub rustfs: &'static str,
    pub dingtalk: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SettingsResponse {
    pub settings: Vec<SettingItem>,
    pub connection_status: SettingsConnectionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSetting {
    pub key: String,
    pub value_encrypted: Option<String>,
    pub value_masked: Option<String>,
    pub updated_by: Option<Uuid>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUpdate {
    pub key: String,
    pub value: String,
    pub updated_by: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsRepositoryError {
    #[error("settings validation failed")]
    Validation,
    #[error("database operation failed")]
    Database,
}

#[async_trait]
pub trait SettingsRepository: Clone + Send + Sync + 'static {
    async fn list_settings(&self) -> Result<Vec<StoredSetting>, SettingsRepositoryError>;

    async fn upsert_settings(
        &self,
        updates: Vec<SettingsUpdate>,
    ) -> Result<Vec<StoredSetting>, SettingsRepositoryError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemorySettingsRepository {
    inner: Arc<Mutex<HashMap<String, StoredSetting>>>,
}

#[derive(Debug, Clone)]
pub struct SqlxSettingsRepository {
    pool: PgPool,
}

impl SqlxSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsRepository for MemorySettingsRepository {
    async fn list_settings(&self) -> Result<Vec<StoredSetting>, SettingsRepositoryError> {
        Ok(self
            .inner
            .lock()
            .expect("memory settings repository lock")
            .values()
            .cloned()
            .collect())
    }

    async fn upsert_settings(
        &self,
        updates: Vec<SettingsUpdate>,
    ) -> Result<Vec<StoredSetting>, SettingsRepositoryError> {
        let mut settings = self.inner.lock().expect("memory settings repository lock");
        for update in updates {
            let definition =
                find_definition(&update.key).ok_or(SettingsRepositoryError::Validation)?;
            let stored = build_stored_setting(definition, &update.value, update.updated_by);
            settings.insert(update.key, stored);
        }
        Ok(settings.values().cloned().collect())
    }
}

#[async_trait]
impl SettingsRepository for SqlxSettingsRepository {
    async fn list_settings(&self) -> Result<Vec<StoredSetting>, SettingsRepositoryError> {
        let keys = setting_definitions()
            .iter()
            .map(|definition| definition.key)
            .collect::<Vec<_>>();
        sqlx::query(
            "select key, value_encrypted, value_masked, updated_by, updated_at
             from system_settings
             where key = any($1)",
        )
        .bind(keys)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| SettingsRepositoryError::Database)?
        .into_iter()
        .map(row_to_stored_setting)
        .collect()
    }

    async fn upsert_settings(
        &self,
        updates: Vec<SettingsUpdate>,
    ) -> Result<Vec<StoredSetting>, SettingsRepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| SettingsRepositoryError::Database)?;

        for update in updates {
            let definition =
                find_definition(&update.key).ok_or(SettingsRepositoryError::Validation)?;
            let stored = build_stored_setting(definition, &update.value, update.updated_by);
            sqlx::query(
                "insert into system_settings (
                    key, value_encrypted, value_masked, updated_by, updated_at
                 )
                 values ($1, $2, $3, $4, now())
                 on conflict (key) do update set
                    value_encrypted = excluded.value_encrypted,
                    value_masked = excluded.value_masked,
                    updated_by = excluded.updated_by,
                    updated_at = now()",
            )
            .bind(stored.key)
            .bind(stored.value_encrypted)
            .bind(stored.value_masked)
            .bind(stored.updated_by)
            .execute(&mut *tx)
            .await
            .map_err(|_| SettingsRepositoryError::Database)?;
        }

        tx.commit()
            .await
            .map_err(|_| SettingsRepositoryError::Database)?;
        self.list_settings().await
    }
}

pub fn build_settings_response(stored: Vec<StoredSetting>, config: &AppConfig) -> SettingsResponse {
    let stored_by_key = stored
        .into_iter()
        .map(|setting| (setting.key.clone(), setting))
        .collect::<HashMap<_, _>>();
    let settings = setting_definitions()
        .into_iter()
        .map(|definition| {
            let env_value = env_value_for(definition, config);
            build_setting_item(definition, stored_by_key.get(definition.key), env_value)
        })
        .collect::<Vec<_>>();

    SettingsResponse {
        settings,
        connection_status: SettingsConnectionStatus {
            rustfs: "configured",
            dingtalk: "not_checked",
        },
    }
}

fn build_setting_item(
    definition: SettingDefinition,
    stored: Option<&StoredSetting>,
    env_value: Option<String>,
) -> SettingItem {
    if let Some(stored) = stored {
        return SettingItem {
            key: definition.key.to_string(),
            label: definition.label.to_string(),
            group: definition.group,
            sensitive: definition.sensitive,
            configured: stored
                .value_masked
                .as_deref()
                .is_some_and(|value| !value.is_empty()),
            source: SettingSource::Database,
            value_masked: stored.value_masked.clone(),
            updated_by: stored.updated_by,
            updated_at: stored.updated_at,
        };
    }

    let value_masked = env_value.map(|value| {
        if definition.sensitive {
            mask_value(&value)
        } else {
            value
        }
    });

    SettingItem {
        key: definition.key.to_string(),
        label: definition.label.to_string(),
        group: definition.group,
        sensitive: definition.sensitive,
        configured: value_masked
            .as_deref()
            .is_some_and(|value| !value.is_empty()),
        source: if value_masked.is_some() {
            SettingSource::Env
        } else {
            SettingSource::Empty
        },
        value_masked,
        updated_by: None,
        updated_at: None,
    }
}

fn build_stored_setting(
    definition: SettingDefinition,
    value: &str,
    updated_by: Uuid,
) -> StoredSetting {
    let trimmed = value.trim();
    StoredSetting {
        key: definition.key.to_string(),
        value_encrypted: definition
            .sensitive
            .then(|| encrypt_for_storage(definition.key, trimmed)),
        value_masked: Some(if definition.sensitive {
            mask_value(trimmed)
        } else {
            trimmed.to_string()
        }),
        updated_by: Some(updated_by),
        updated_at: Some(Utc::now()),
    }
}

pub fn validate_updates(updates: &[SettingsUpdate]) -> Result<(), SettingsRepositoryError> {
    for update in updates {
        let Some(_) = find_definition(&update.key) else {
            return Err(SettingsRepositoryError::Validation);
        };
        if update.value.trim().is_empty() {
            return Err(SettingsRepositoryError::Validation);
        }
    }
    Ok(())
}

pub fn setting_definitions() -> Vec<SettingDefinition> {
    vec![
        SettingDefinition {
            key: "dingtalk.corp_id",
            label: "CorpId",
            group: SettingGroup::Dingtalk,
            sensitive: false,
        },
        SettingDefinition {
            key: "dingtalk.app_id",
            label: "App ID",
            group: SettingGroup::Dingtalk,
            sensitive: false,
        },
        SettingDefinition {
            key: "dingtalk.client_id",
            label: "Client ID",
            group: SettingGroup::Dingtalk,
            sensitive: false,
        },
        SettingDefinition {
            key: "dingtalk.client_secret",
            label: "Client Secret",
            group: SettingGroup::Dingtalk,
            sensitive: true,
        },
        SettingDefinition {
            key: "dingtalk.agent_id",
            label: "AgentId",
            group: SettingGroup::Dingtalk,
            sensitive: false,
        },
        SettingDefinition {
            key: "dingtalk.callback_url",
            label: "回调地址",
            group: SettingGroup::Dingtalk,
            sensitive: false,
        },
        SettingDefinition {
            key: "rustfs.endpoint",
            label: "Endpoint",
            group: SettingGroup::Rustfs,
            sensitive: false,
        },
        SettingDefinition {
            key: "rustfs.region",
            label: "Region",
            group: SettingGroup::Rustfs,
            sensitive: false,
        },
        SettingDefinition {
            key: "rustfs.bucket",
            label: "Bucket",
            group: SettingGroup::Rustfs,
            sensitive: false,
        },
        SettingDefinition {
            key: "rustfs.access_key_id",
            label: "Access Key",
            group: SettingGroup::Rustfs,
            sensitive: true,
        },
        SettingDefinition {
            key: "rustfs.secret_access_key",
            label: "Secret Key",
            group: SettingGroup::Rustfs,
            sensitive: true,
        },
        SettingDefinition {
            key: "rustfs.use_https",
            label: "使用 HTTPS",
            group: SettingGroup::Rustfs,
            sensitive: false,
        },
        SettingDefinition {
            key: "rustfs.public_base_url",
            label: "外部访问地址",
            group: SettingGroup::Rustfs,
            sensitive: false,
        },
    ]
}

fn find_definition(key: &str) -> Option<SettingDefinition> {
    setting_definitions()
        .into_iter()
        .find(|definition| definition.key == key)
}

fn env_value_for(definition: SettingDefinition, config: &AppConfig) -> Option<String> {
    match definition.key {
        "dingtalk.corp_id" => config.dingtalk.as_ref().map(|value| value.corp_id.clone()),
        "dingtalk.client_id" => config
            .dingtalk
            .as_ref()
            .map(|value| value.client_id.clone()),
        "dingtalk.client_secret" => config
            .dingtalk
            .as_ref()
            .map(|value| value.client_secret.clone()),
        "dingtalk.agent_id" => config
            .dingtalk
            .as_ref()
            .map(|value| value.agent_id.to_string()),
        "rustfs.endpoint" => Some(config.rustfs.endpoint.clone()),
        "rustfs.region" => Some(config.rustfs.region.clone()),
        "rustfs.bucket" => Some(config.rustfs.bucket.clone()),
        "rustfs.access_key_id" => Some(config.rustfs.access_key_id.clone()),
        "rustfs.secret_access_key" => Some(config.rustfs.secret_access_key.clone()),
        _ => None,
    }
}

fn row_to_stored_setting(
    row: sqlx::postgres::PgRow,
) -> Result<StoredSetting, SettingsRepositoryError> {
    Ok(StoredSetting {
        key: row
            .try_get("key")
            .map_err(|_| SettingsRepositoryError::Database)?,
        value_encrypted: row
            .try_get("value_encrypted")
            .map_err(|_| SettingsRepositoryError::Database)?,
        value_masked: row
            .try_get("value_masked")
            .map_err(|_| SettingsRepositoryError::Database)?,
        updated_by: row
            .try_get("updated_by")
            .map_err(|_| SettingsRepositoryError::Database)?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|_| SettingsRepositoryError::Database)?,
    })
}

fn mask_value(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= 4 {
        return "****".to_string();
    }

    let prefix = chars.iter().take(2).collect::<String>();
    let suffix = chars
        .iter()
        .skip(chars.len().saturating_sub(2))
        .collect::<String>();
    format!("{prefix}***{suffix}")
}

fn encrypt_for_storage(key: &str, value: &str) -> String {
    let key_bytes = key.as_bytes();
    let encrypted = value
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(index, byte)| byte ^ key_bytes[index % key_bytes.len()])
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("v1:{encrypted}")
}

impl Serialize for SettingGroup {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Dingtalk => "dingtalk",
            Self::Rustfs => "rustfs",
        })
    }
}

impl Serialize for SettingSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            Self::Database => "database",
            Self::Env => "env",
            Self::Empty => "empty",
        })
    }
}
