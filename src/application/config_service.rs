use std::sync::Arc;

use crate::domain::device_config::DeviceConfig;
use crate::error::Result;
use crate::infrastructure::database::device_config_repo::DeviceConfigRepository;

/// Application service for device configuration management.
pub struct ConfigService {
    config_repo: Arc<DeviceConfigRepository>,
}

impl ConfigService {
    pub fn new(config_repo: Arc<DeviceConfigRepository>) -> Self {
        Self { config_repo }
    }

    pub async fn query_config(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<DeviceConfig>> {
        self.config_repo.get_by_device(product_key, device_id).await
    }

    pub async fn update_config(
        &self,
        product_key: &str,
        device_id: &str,
        config_json: serde_json::Value,
    ) -> Result<i64> {
        let existing = self
            .config_repo
            .get_by_device(product_key, device_id)
            .await?;

        let config = DeviceConfig {
            id: existing.as_ref().and_then(|c| c.id),
            product_key: product_key.to_string(),
            device_id: device_id.to_string(),
            config_json: Some(config_json),
            version: existing.as_ref().and_then(|c| c.version),
            gmt_create: None,
            gmt_modified: None,
        };

        self.config_repo.upsert(&config).await
    }

    pub async fn get_version(&self, product_key: &str, device_id: &str) -> Result<Option<i64>> {
        self.config_repo.get_version(product_key, device_id).await
    }
}
