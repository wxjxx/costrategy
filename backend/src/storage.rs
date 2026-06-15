use crate::config::RustfsConfig;
use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObject {
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage configuration is invalid")]
    Config,
    #[error("failed to upload object")]
    Upload,
    #[error("failed to download object")]
    Download,
    #[error("failed to delete object")]
    Delete,
}

#[async_trait]
pub trait AttachmentStorage: Clone + Send + Sync + 'static {
    fn bucket(&self) -> &str;
    async fn put_object(
        &self,
        object_key: &str,
        bytes: Vec<u8>,
        mime_type: Option<&str>,
    ) -> Result<(), StorageError>;
    async fn get_object(&self, object_key: &str) -> Result<StoredObject, StorageError>;
    async fn delete_object(&self, object_key: &str) -> Result<(), StorageError>;
}

#[derive(Debug, Clone)]
pub struct MemoryAttachmentStorage {
    bucket: String,
    objects: Arc<Mutex<HashMap<String, StoredObject>>>,
}

impl MemoryAttachmentStorage {
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            objects: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl AttachmentStorage for MemoryAttachmentStorage {
    fn bucket(&self) -> &str {
        &self.bucket
    }

    async fn put_object(
        &self,
        object_key: &str,
        bytes: Vec<u8>,
        mime_type: Option<&str>,
    ) -> Result<(), StorageError> {
        self.objects
            .lock()
            .map_err(|_| StorageError::Upload)?
            .insert(
                object_key.to_string(),
                StoredObject {
                    bytes,
                    mime_type: mime_type.map(str::to_string),
                },
            );
        Ok(())
    }

    async fn get_object(&self, object_key: &str) -> Result<StoredObject, StorageError> {
        self.objects
            .lock()
            .map_err(|_| StorageError::Download)?
            .get(object_key)
            .cloned()
            .ok_or(StorageError::Download)
    }

    async fn delete_object(&self, object_key: &str) -> Result<(), StorageError> {
        self.objects
            .lock()
            .map_err(|_| StorageError::Delete)?
            .remove(object_key);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RustfsAttachmentStorage {
    bucket: String,
    client: Client,
}

impl RustfsAttachmentStorage {
    pub async fn from_config(config: &RustfsConfig) -> Result<Self, StorageError> {
        let credentials = Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "local_ENV",
        );
        let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()))
            .credentials_provider(credentials)
            .load()
            .await;
        let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
            .endpoint_url(normalize_endpoint(&config.endpoint))
            .force_path_style(true)
            .build();

        Ok(Self {
            bucket: config.bucket.clone(),
            client: Client::from_conf(s3_config),
        })
    }
}

#[async_trait]
impl AttachmentStorage for RustfsAttachmentStorage {
    fn bucket(&self) -> &str {
        &self.bucket
    }

    async fn put_object(
        &self,
        object_key: &str,
        bytes: Vec<u8>,
        mime_type: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(object_key)
            .body(ByteStream::from(bytes));
        if let Some(mime_type) = mime_type {
            request = request.content_type(mime_type);
        }
        request.send().await.map_err(|error| {
            log::error!(
                "{}",
                storage_operation_error_message(
                    "upload",
                    &self.bucket,
                    object_key,
                    &error.to_string(),
                    &format!("{error:?}"),
                )
            );
            StorageError::Upload
        })?;
        Ok(())
    }

    async fn get_object(&self, object_key: &str) -> Result<StoredObject, StorageError> {
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(object_key)
            .send()
            .await
            .map_err(|_| StorageError::Download)?;
        let mime_type = object.content_type().map(str::to_string);
        let bytes = object
            .body
            .collect()
            .await
            .map_err(|_| StorageError::Download)?
            .into_bytes()
            .to_vec();

        Ok(StoredObject { bytes, mime_type })
    }

    async fn delete_object(&self, object_key: &str) -> Result<(), StorageError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(object_key)
            .send()
            .await
            .map_err(|_| StorageError::Delete)?;
        Ok(())
    }
}

fn normalize_endpoint(endpoint: &str) -> String {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        endpoint.to_string()
    } else {
        format!("http://{endpoint}")
    }
}

fn storage_operation_error_message(
    operation: &str,
    bucket: &str,
    object_key: &str,
    error: &str,
    error_debug: &str,
) -> String {
    format!(
        "rustfs storage operation failed: operation={operation}, bucket={bucket}, object_key={object_key}, error={error}, error_debug={error_debug}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_operation_error_message_includes_safe_diagnostics() {
        let message = storage_operation_error_message(
            "upload",
            "costrategy",
            "rich-text/images/image.png",
            "connection refused",
            "ConnectorError { kind: Io, source: connection refused }",
        );

        assert!(message.contains("operation=upload"));
        assert!(message.contains("bucket=costrategy"));
        assert!(message.contains("object_key=rich-text/images/image.png"));
        assert!(message.contains("error=connection refused"));
        assert!(
            message.contains("error_debug=ConnectorError { kind: Io, source: connection refused }")
        );
    }
}
