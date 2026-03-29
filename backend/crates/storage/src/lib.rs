//! # Storage Crate
//!
//! S3-compatible object storage for storing and retrieving generated images,
//! assets, and other files.

use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Credentials;
use common::AppResult;
use std::path::Path;

/// Storage configuration.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub region: String,
}

/// S3 storage service for object operations.
#[derive(Clone)]
pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    /// Create a new S3 storage instance.
    pub async fn new(config: StorageConfig) -> AppResult<Self> {
        let creds = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "static",
        );

        let region = aws_types::region::Region::new(config.region.clone());
        let cfg = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(region)
            .load()
            .await;

        let client = Client::new(&cfg);

        Ok(Self {
            client,
            bucket: config.bucket,
        })
    }

    /// Upload a file to storage.
    pub async fn upload_file(&self, key: &str, file_path: impl AsRef<Path>) -> AppResult<String> {
        use std::fs;
        let data = fs::read(file_path)?;
        self.upload(key, data, "application/octet-stream").await
    }

    /// Upload bytes to storage.
    pub async fn upload_bytes(&self, key: &str, data: Vec<u8>, content_type: &str) -> AppResult<String> {
        self.upload(key, data, content_type).await
    }

    /// Internal upload method.
    async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> AppResult<String> {
        let body = ByteStream::from(data);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| common::AppError::Aws(e.to_string()))?;

        Ok(format!("s3://{}/{}", self.bucket, key))
    }

    /// Generate a presigned URL for downloading.
    pub async fn presigned_url(&self, key: &str, expires_in_secs: i64) -> AppResult<String> {
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::builder()
            .expires_in(std::time::Duration::from_secs(expires_in_secs as u64))
            .build()
            .map_err(|e| common::AppError::Aws(e.to_string()))?;

        let presigned = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(presigning_config)
            .await
            .map_err(|e| common::AppError::Aws(e.to_string()))?;

        Ok(presigned.uri().to_string())
    }

    /// Delete an object from storage.
    pub async fn delete(&self, key: &str) -> AppResult<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| common::AppError::Aws(e.to_string()))?;
        Ok(())
    }

    /// Check if an object exists.
    pub async fn exists(&self, key: &str) -> AppResult<bool> {
        Ok(self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .is_ok())
    }
}
