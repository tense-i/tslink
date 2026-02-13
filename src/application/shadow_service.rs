use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::application::event_bus::{DeviceEvent, DeviceEventBus};
use crate::domain::message::CommonTopicResponse;
use crate::error::Result;
use crate::infrastructure::database::shadow_repo::ShadowRepository;
use crate::infrastructure::mqtt::publisher::MessagePublisher;
use crate::infrastructure::redis::shadow::ShadowRedis;

/// Application service for device shadow (property cache) management.
///
/// Orchestrates Redis shadow storage + MySQL shadow config for:
/// - Property reads/writes
/// - Shadow replay on device online
pub struct ShadowService {
    shadow_redis: Arc<ShadowRedis>,
    shadow_repo: Arc<ShadowRepository>,
    publisher: Arc<MessagePublisher>,
    event_bus: Option<DeviceEventBus>,
}

impl ShadowService {
    pub fn new(
        shadow_redis: Arc<ShadowRedis>,
        shadow_repo: Arc<ShadowRepository>,
        publisher: Arc<MessagePublisher>,
    ) -> Self {
        Self {
            shadow_redis,
            shadow_repo,
            publisher,
            event_bus: None,
        }
    }

    /// Attach an event bus for WebSocket push notifications.
    pub fn with_event_bus(mut self, bus: DeviceEventBus) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Get device properties from Redis shadow.
    pub async fn get_device_properties(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        self.shadow_redis
            .get_properties(product_key, device_id)
            .await
    }

    /// Update device properties in Redis shadow (merge).
    pub async fn update_properties(
        &self,
        product_key: &str,
        device_id: &str,
        properties: &serde_json::Value,
    ) -> Result<()> {
        self.shadow_redis
            .merge_properties(product_key, device_id, properties)
            .await?;
        debug!(pk = %product_key, did = %device_id, "shadow properties updated");
        if let Some(ref bus) = self.event_bus {
            bus.publish(DeviceEvent::property_update(
                product_key,
                device_id,
                properties.clone(),
            ));
        }
        Ok(())
    }

    /// Update a shadow service configuration (upsert).
    pub async fn update_service(
        &self,
        product_key: &str,
        device_id: &str,
        method: &str,
        data: &serde_json::Value,
    ) -> Result<()> {
        self.shadow_repo
            .upsert_shadow_service(product_key, device_id, method, data)
            .await
    }

    /// Replay shadow services on device online.
    ///
    /// Reads all enabled ShadowServiceConfig for the product,
    /// then sends each as a service invocation to the device.
    pub async fn replay_shadow_on_online(&self, product_key: &str, device_id: &str) -> Result<()> {
        let services = self.shadow_repo.find_shadow_services(product_key).await?;

        if services.is_empty() {
            debug!(pk = %product_key, did = %device_id, "no shadow services to replay");
            return Ok(());
        }

        info!(
            pk = %product_key,
            did = %device_id,
            count = services.len(),
            "replaying shadow services on device online"
        );

        for svc in &services {
            let topic = format!(
                "sys/{}/{}/thing/service/{}/post",
                product_key, device_id, svc.method
            );

            let response = CommonTopicResponse {
                tid: Some(uuid::Uuid::new_v4().to_string()),
                bid: None,
                version: "1.0".to_string(),
                timestamp: Some(chrono::Utc::now().timestamp_millis()),
                method: Some(format!("thing.service.{}", svc.method)),
                product_key: Some(product_key.to_string()),
                device_id: Some(device_id.to_string()),
                data: serde_json::json!({}),
                code: None,
                message: None,
            };

            if let Err(e) = self.publisher.publish(&topic, &response).await {
                warn!(
                    pk = %product_key,
                    did = %device_id,
                    method = %svc.method,
                    error = %e,
                    "failed to replay shadow service"
                );
            }
        }

        Ok(())
    }
}
