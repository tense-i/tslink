use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::application::link_service::LinkService;
use crate::application::shadow_service::ShadowService;
use crate::domain::message::{CommonTopicResponse, ResponseCode};
use crate::error::Result;
use crate::infrastructure::database::model_repo::ModelRepository;
use crate::infrastructure::mqtt::publisher::MessagePublisher;

/// Default timeout for synchronous service calls (10 seconds).
const DEFAULT_SYNC_TIMEOUT: Duration = Duration::from_secs(10);

/// Application service for device thing-model service invocation.
///
/// Orchestrates:
/// - Service invocation (sync/async) via MQTT
/// - Property setting via MQTT
/// - Shadow update after successful service call
/// - Multi-link routing via LinkService
pub struct ThingService {
    model_repo: Arc<ModelRepository>,
    publisher: Arc<MessagePublisher>,
    shadow_service: Arc<ShadowService>,
    link_service: Option<Arc<LinkService>>,
}

impl ThingService {
    pub fn new(
        model_repo: Arc<ModelRepository>,
        publisher: Arc<MessagePublisher>,
        shadow_service: Arc<ShadowService>,
    ) -> Self {
        Self {
            model_repo,
            publisher,
            shadow_service,
            link_service: None,
        }
    }

    /// Set a LinkService for multi-link device support (T047).
    pub fn with_link_service(mut self, link_service: Arc<LinkService>) -> Self {
        self.link_service = Some(link_service);
        self
    }

    /// Resolve the target device ID segment, incorporating link suffix if available.
    async fn resolve_device_id_for_topic(&self, product_key: &str, device_id: &str) -> String {
        if let Some(ref ls) = self.link_service {
            match ls.resolve_target_device_id(product_key, device_id).await {
                Ok(resolved) => resolved,
                Err(e) => {
                    warn!(
                        pk = %product_key,
                        did = %device_id,
                        error = %e,
                        "failed to resolve multi-link target, using plain device_id"
                    );
                    device_id.to_string()
                }
            }
        } else {
            device_id.to_string()
        }
    }

    /// Invoke a service on a device.
    ///
    /// - `sync`: if true, waits for device reply via PostSync pattern
    /// - Returns raw reply payload for sync calls, None for async
    ///
    /// After a successful call, updates the shadow service record.
    pub async fn invoke_service(
        &self,
        product_key: &str,
        device_id: &str,
        method: &str,
        data: serde_json::Value,
        sync: bool,
    ) -> Result<Option<Vec<u8>>> {
        // Validate method exists in device model (optional, warn if not found)
        if let Some(model) = self.model_repo.get_device_model(product_key).await? {
            if model.get_service(method).is_none() {
                warn!(
                    pk = %product_key,
                    did = %device_id,
                    method = %method,
                    "service method not found in device model, proceeding anyway"
                );
            }
        }

        let tid = Uuid::new_v4().to_string();
        let target_did = self
            .resolve_device_id_for_topic(product_key, device_id)
            .await;
        let topic = format!(
            "sys/{}/{}/thing/service/{}/post",
            product_key, target_did, method
        );

        let response = CommonTopicResponse {
            tid: Some(tid.clone()),
            bid: None,
            method: Some(method.to_string()),
            data,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            version: "1.0".to_string(),
            code: Some(ResponseCode::SUCCESS.to_string()),
            message: None,
            product_key: Some(product_key.to_string()),
            device_id: Some(device_id.to_string()),
        };

        let result = if sync {
            info!(
                pk = %product_key,
                did = %device_id,
                method = %method,
                tid = %tid,
                "invoking sync service"
            );
            let reply = self
                .publisher
                .publish_with_reply(&topic, &response, DEFAULT_SYNC_TIMEOUT)
                .await?;
            Some(reply)
        } else {
            info!(
                pk = %product_key,
                did = %device_id,
                method = %method,
                "invoking async service"
            );
            self.publisher.publish(&topic, &response).await?;
            None
        };

        // T040: Update shadow after successful service call
        if let Err(e) = self
            .shadow_service
            .update_service(product_key, device_id, method, &response.data)
            .await
        {
            warn!(
                pk = %product_key,
                did = %device_id,
                method = %method,
                error = %e,
                "failed to update shadow after service call"
            );
            // Don't fail the service call just because shadow update failed
        }

        debug!(
            pk = %product_key,
            did = %device_id,
            method = %method,
            sync = sync,
            "service invocation completed"
        );

        Ok(result)
    }

    /// Set properties on a device.
    ///
    /// Publishes to `sys/{pk}/{did}/thing/properties/set`.
    pub async fn set_properties(
        &self,
        product_key: &str,
        device_id: &str,
        properties: serde_json::Value,
    ) -> Result<()> {
        let tid = Uuid::new_v4().to_string();
        let target_did = self
            .resolve_device_id_for_topic(product_key, device_id)
            .await;
        let topic = format!("sys/{}/{}/thing/properties/set", product_key, target_did);

        let response = CommonTopicResponse {
            tid: Some(tid),
            bid: None,
            method: Some("thing.properties.set".to_string()),
            data: properties,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            version: "1.0".to_string(),
            code: Some(ResponseCode::SUCCESS.to_string()),
            message: None,
            product_key: Some(product_key.to_string()),
            device_id: Some(device_id.to_string()),
        };

        self.publisher.publish(&topic, &response).await?;
        info!(
            pk = %product_key,
            did = %device_id,
            "property set command published"
        );

        Ok(())
    }

    /// Get the model repository reference (for external use).
    pub fn model_repo(&self) -> &Arc<ModelRepository> {
        &self.model_repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_topic_format() {
        let topic = format!(
            "sys/{}/{}/thing/service/{}/post",
            "pk001", "did001", "reboot"
        );
        assert_eq!(topic, "sys/pk001/did001/thing/service/reboot/post");
    }

    #[test]
    fn test_property_set_topic_format() {
        let topic = format!("sys/{}/{}/thing/properties/set", "pk001", "did001");
        assert_eq!(topic, "sys/pk001/did001/thing/properties/set");
    }

    #[test]
    fn test_sync_timeout_default() {
        assert_eq!(DEFAULT_SYNC_TIMEOUT, Duration::from_secs(10));
    }
}
