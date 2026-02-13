use std::sync::Arc;

use serde_json::{json, Value};
use tracing::debug;

use crate::application::config_service::ConfigService;
use crate::domain::topic::TopicInfo;
use crate::error::Result;

/// Handler for device configuration MQTT operations.
pub struct ConfigHandler {
    config_service: Arc<ConfigService>,
}

impl ConfigHandler {
    pub fn new(config_service: Arc<ConfigService>) -> Self {
        Self { config_service }
    }

    pub async fn handle_query(&self, topic: &TopicInfo) -> Result<Value> {
        let product_key = &topic.product_key;
        let device_id = &topic.device_id;

        debug!(
            product_key = %product_key,
            device_id = %device_id,
            "Handling config query"
        );

        let config = self
            .config_service
            .query_config(product_key, device_id)
            .await?;

        match config {
            Some(cfg) => Ok(json!({
                "version": cfg.version.unwrap_or(0),
                "config": cfg.config_json.unwrap_or(json!({}))
            })),
            None => Ok(json!({
                "version": 0,
                "config": {}
            })),
        }
    }

    pub async fn handle_update(&self, topic: &TopicInfo, payload: &Value) -> Result<Value> {
        let product_key = &topic.product_key;
        let device_id = &topic.device_id;

        debug!(
            product_key = %product_key,
            device_id = %device_id,
            "Handling config update"
        );

        let config_data = payload.get("config").cloned().unwrap_or(json!({}));

        let new_version = self
            .config_service
            .update_config(product_key, device_id, config_data)
            .await?;

        Ok(json!({ "version": new_version }))
    }

    pub async fn handle_version_query(&self, topic: &TopicInfo) -> Result<Value> {
        let product_key = &topic.product_key;
        let device_id = &topic.device_id;

        debug!(
            product_key = %product_key,
            device_id = %device_id,
            "Handling config version query"
        );

        let version = self
            .config_service
            .get_version(product_key, device_id)
            .await?;

        Ok(json!({ "version": version.unwrap_or(0) }))
    }
}
