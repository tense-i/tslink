use std::sync::Arc;

use serde_json::{json, Value};
use tracing::debug;

use crate::application::discovery_service::DiscoveryService;
use crate::domain::topic::TopicInfo;
use crate::error::Result;

/// Handler for device discovery MQTT operations.
pub struct DiscoveryHandler {
    discovery_service: Arc<DiscoveryService>,
}

impl DiscoveryHandler {
    pub fn new(discovery_service: Arc<DiscoveryService>) -> Self {
        Self { discovery_service }
    }

    /// Handle discovery list request - returns all devices for the product.
    pub async fn handle_list(&self, topic: &TopicInfo) -> Result<Value> {
        let product_key = &topic.product_key;

        debug!(
            product_key = %product_key,
            "Handling discovery list request"
        );

        let devices = self.discovery_service.get_devices(product_key).await?;
        Ok(self.discovery_service.build_response(devices))
    }

    /// Handle sub-device discovery request - returns sub-devices for a gateway.
    pub async fn handle_sub_devices(&self, topic: &TopicInfo) -> Result<Value> {
        let product_key = &topic.product_key;
        let gateway_device_id = &topic.device_id;

        debug!(
            product_key = %product_key,
            gateway_device_id = %gateway_device_id,
            "Handling sub-device discovery request"
        );

        let sub_devices = self
            .discovery_service
            .get_sub_devices(product_key, gateway_device_id)
            .await?;
        Ok(json!({ "devices": sub_devices }))
    }

    /// Handle cache refresh request.
    pub async fn handle_refresh(&self, topic: &TopicInfo) -> Result<Value> {
        let product_key = &topic.product_key;

        debug!(
            product_key = %product_key,
            "Handling discovery cache refresh"
        );

        let devices = self
            .discovery_service
            .refresh_cache_for_product(product_key)
            .await?;
        Ok(json!({
            "refreshed": true,
            "count": devices.len()
        }))
    }
}
