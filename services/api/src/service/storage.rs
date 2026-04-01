//! S3/MinIO storage service.
//! Handles presigned URL generation for upload/download via aws-sdk-s3.

use std::time::Duration;

use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{
    Client,
    config::{Builder as S3ConfigBuilder, Credentials},
    error::SdkError,
    operation::create_bucket::CreateBucketError,
    presigning::PresigningConfig,
    types::CreateBucketConfiguration,
};

use crate::config::Config;

pub struct StorageService {
    client: Client,
    provider: String,
    bucket: String,
    region: String,
    upload_url_expiry: u64,
    download_url_expiry: u64,
}

impl StorageService {
    /// Construct a `StorageService` and create an authenticated S3 client.
    ///
    /// For MinIO the endpoint is overridden and path-style addressing is forced.
    pub async fn new(config: &Config) -> Self {
        let credentials = Credentials::new(
            &config.storage_access_key_id,
            &config.storage_secret_access_key,
            None,  // session token
            None,  // expiry
            "monobase-static",
        );

        let region = Region::new(config.storage_region.clone());

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region.clone())
            .credentials_provider(credentials)
            .load()
            .await;

        // Build a per-service S3 config so we can set endpoint_url and
        // force_path_style (required for MinIO and other S3-compatible stores).
        let mut s3_config_builder = S3ConfigBuilder::from(&shared_config)
            .endpoint_url(&config.storage_endpoint)
            .force_path_style(true); // MinIO / Ceph need path-style

        // For real AWS S3 we skip forced path-style.
        if config.storage_provider == "s3" {
            s3_config_builder = s3_config_builder.force_path_style(false);
        }

        let s3_config = s3_config_builder.build();
        let client = Client::from_conf(s3_config);

        Self {
            client,
            provider: config.storage_provider.clone(),
            bucket: config.storage_bucket.clone(),
            region: config.storage_region.clone(),
            upload_url_expiry: config.storage_upload_url_expiry,
            download_url_expiry: config.storage_download_url_expiry,
        }
    }

    /// Generate a presigned `PutObject` URL for client-side direct upload.
    pub async fn generate_upload_url(
        &self,
        file_id: &str,
        filename: &str,
        mime_type: &str,
    ) -> Result<String, String> {
        let key = format!("{}/{}", file_id, filename);

        let presigning = PresigningConfig::expires_in(Duration::from_secs(self.upload_url_expiry))
            .map_err(|e| format!("presigning config error: {e}"))?;

        let presigned = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type(mime_type)
            .presigned(presigning)
            .await
            .map_err(|e| format!("PutObject presign failed: {e}"))?;

        tracing::debug!(
            provider = %self.provider,
            bucket = %self.bucket,
            key = %key,
            mime_type = %mime_type,
            expiry_secs = self.upload_url_expiry,
            "Generated upload URL"
        );

        Ok(presigned.uri().to_string())
    }

    /// Generate a presigned `GetObject` URL for client-side direct download.
    pub async fn generate_download_url(
        &self,
        file_id: &str,
        filename: &str,
    ) -> Result<String, String> {
        let key = format!("{}/{}", file_id, filename);

        let presigning =
            PresigningConfig::expires_in(Duration::from_secs(self.download_url_expiry))
                .map_err(|e| format!("presigning config error: {e}"))?;

        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .presigned(presigning)
            .await
            .map_err(|e| format!("GetObject presign failed: {e}"))?;

        tracing::debug!(
            provider = %self.provider,
            bucket = %self.bucket,
            key = %key,
            expiry_secs = self.download_url_expiry,
            "Generated download URL"
        );

        Ok(presigned.uri().to_string())
    }

    /// Delete a file from storage using `DeleteObject`.
    pub async fn delete_file(&self, file_id: &str, filename: &str) -> Result<(), String> {
        let key = format!("{}/{}", file_id, filename);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| format!("DeleteObject failed: {e}"))?;

        tracing::debug!(
            provider = %self.provider,
            bucket = %self.bucket,
            key = %key,
            "Deleted file"
        );

        Ok(())
    }

    /// Check if a file exists using `HeadObject`. Returns `false` on a 404.
    pub async fn file_exists(&self, file_id: &str, filename: &str) -> Result<bool, String> {
        let key = format!("{}/{}", file_id, filename);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(SdkError::ServiceError(err)) if err.err().is_not_found() => Ok(false),
            Err(e) => Err(format!("HeadObject failed: {e}")),
        }
    }

    /// Create the bucket if it does not already exist.
    ///
    /// `BucketAlreadyExists` and `BucketAlreadyOwnedByYou` are treated as
    /// success so startup is idempotent.
    pub async fn initialize_bucket(&self) -> Result<(), String> {
        // us-east-1 is special: passing a CreateBucketConfiguration with it
        // causes an InvalidLocationConstraint error, so we omit it.
        let mut req = self.client.create_bucket().bucket(&self.bucket);

        if self.region != "us-east-1" {
            let cfg = CreateBucketConfiguration::builder()
                .location_constraint(
                    self.region
                        .parse()
                        .map_err(|e| format!("invalid region '{e}'"))?,
                )
                .build();
            req = req.create_bucket_configuration(cfg);
        }

        match req.send().await {
            Ok(_) => {
                tracing::info!(
                    provider = %self.provider,
                    bucket = %self.bucket,
                    "Bucket created"
                );
                Ok(())
            }
            Err(SdkError::ServiceError(err)) => match err.err() {
                CreateBucketError::BucketAlreadyExists(_)
                | CreateBucketError::BucketAlreadyOwnedByYou(_) => {
                    tracing::debug!(bucket = %self.bucket, "Bucket already exists — skipping");
                    Ok(())
                }
                other => Err(format!("CreateBucket failed: {other}")),
            },
            Err(e) => Err(format!("CreateBucket request failed: {e}")),
        }
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn region(&self) -> &str {
        &self.region
    }
}
