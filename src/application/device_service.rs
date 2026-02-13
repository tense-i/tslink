use std::sync::Arc;
use tracing::{info, warn};

use crate::application::event_bus::{DeviceEvent, DeviceEventBus};
use crate::domain::device::{Device, DeviceStatus};
use crate::domain::message::{CommonTopicResponse, ResponseCode};
use crate::error::Result;
use crate::infrastructure::database::device_repo::DeviceRepository;
use crate::infrastructure::database::product_repo::ProductRepository;
use crate::infrastructure::redis::device_state::DeviceStateRedis;

/// Application service for device lifecycle management.
///
/// Orchestrates Redis state + MySQL persistence for device
/// online/offline, heartbeat, and registration flows.
pub struct DeviceService {
    device_state: Arc<DeviceStateRedis>,
    device_repo: Arc<DeviceRepository>,
    product_repo: Arc<ProductRepository>,
    event_bus: Option<DeviceEventBus>,
}

impl DeviceService {
    pub fn new(
        device_state: Arc<DeviceStateRedis>,
        device_repo: Arc<DeviceRepository>,
        product_repo: Arc<ProductRepository>,
    ) -> Self {
        Self {
            device_state,
            device_repo,
            product_repo,
            event_bus: None,
        }
    }

    /// Attach an event bus for WebSocket push notifications.
    pub fn with_event_bus(mut self, bus: DeviceEventBus) -> Self {
        self.event_bus = Some(bus);
        self
    }

    /// Handle device coming online.
    ///
    /// 1. Update Redis status to ONLINE
    /// 2. Update DB status to ONLINE
    pub async fn handle_device_online(&self, product_key: &str, device_id: &str) -> Result<()> {
        self.device_state.set_online(product_key, device_id).await?;
        self.device_repo
            .update_status(product_key, device_id, &DeviceStatus::Online)
            .await?;
        info!(pk = %product_key, did = %device_id, "device online");
        if let Some(ref bus) = self.event_bus {
            bus.publish(DeviceEvent::online(product_key, device_id));
        }
        Ok(())
    }

    /// Handle device going offline.
    ///
    /// 1. Update Redis status to OFFLINE
    /// 2. Update DB status to OFFLINE
    pub async fn handle_device_offline(&self, product_key: &str, device_id: &str) -> Result<()> {
        self.device_state
            .set_offline(product_key, device_id)
            .await?;
        self.device_repo
            .update_status(product_key, device_id, &DeviceStatus::Offline)
            .await?;
        info!(pk = %product_key, did = %device_id, "device offline");
        if let Some(ref bus) = self.event_bus {
            bus.publish(DeviceEvent::offline(product_key, device_id));
        }
        Ok(())
    }

    /// Handle heartbeat (pong) from device.
    pub async fn handle_heartbeat(&self, product_key: &str, device_id: &str) -> Result<()> {
        self.device_state
            .refresh_heartbeat(product_key, device_id)
            .await?;
        Ok(())
    }

    /// Handle static device registration.
    ///
    /// Verifies the device secret, then activates the device.
    pub async fn handle_register(
        &self,
        product_key: &str,
        device_id: &str,
        device_secret: &str,
        product_secret: &str,
    ) -> Result<CommonTopicResponse<serde_json::Value>> {
        if !self
            .verify_product_secret(product_key, product_secret)
            .await?
        {
            warn!(pk = %product_key, did = %device_id, "register failed: invalid product secret");
            return Ok(CommonTopicResponse {
                tid: None,
                bid: None,
                version: "1.0".to_string(),
                timestamp: Some(chrono::Utc::now().timestamp_millis()),
                method: Some("thing.register".to_string()),
                product_key: Some(product_key.to_string()),
                device_id: Some(device_id.to_string()),
                data: serde_json::Value::Null,
                code: Some(ResponseCode::REFUSE.to_string()),
                message: Some("invalid product secret".to_string()),
            });
        }

        // Verify device exists and secret matches
        let valid = self
            .device_repo
            .verify_secret(product_key, device_id, device_secret)
            .await?;

        if !valid {
            warn!(pk = %product_key, did = %device_id, "register failed: invalid secret");
            return Ok(CommonTopicResponse {
                tid: None,
                bid: None,
                version: "1.0".to_string(),
                timestamp: Some(chrono::Utc::now().timestamp_millis()),
                method: Some("thing.register".to_string()),
                product_key: Some(product_key.to_string()),
                device_id: Some(device_id.to_string()),
                data: serde_json::Value::Null,
                code: Some(ResponseCode::REFUSE.to_string()),
                message: Some("invalid device secret".to_string()),
            });
        }

        // Activate — set online
        self.handle_device_online(product_key, device_id).await?;
        info!(pk = %product_key, did = %device_id, "device registered successfully");

        Ok(CommonTopicResponse {
            tid: None,
            bid: None,
            version: "1.0".to_string(),
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            method: Some("thing.register".to_string()),
            product_key: Some(product_key.to_string()),
            device_id: Some(device_id.to_string()),
            data: serde_json::Value::Null,
            code: Some(ResponseCode::SUCCESS.to_string()),
            message: Some("success".to_string()),
        })
    }

    async fn verify_product_secret(&self, product_key: &str, product_secret: &str) -> Result<bool> {
        let secret = self.product_repo.get_secret(product_key).await?;
        Ok(secret.is_some_and(|s| s == product_secret))
    }

    /// Handle dynamic registration.
    ///
    /// Creates the device if it doesn't exist, verifies product-level key,
    /// then activates.
    pub async fn handle_dynamic_register(
        &self,
        product_key: &str,
        device_id: &str,
        device_name: Option<&str>,
        product_secret: &str,
    ) -> Result<CommonTopicResponse<serde_json::Value>> {
        if !self
            .verify_product_secret(product_key, product_secret)
            .await?
        {
            warn!(pk = %product_key, did = %device_id, "dynamic register failed: invalid product secret");
            return Ok(CommonTopicResponse {
                tid: None,
                bid: None,
                version: "1.0".to_string(),
                timestamp: Some(chrono::Utc::now().timestamp_millis()),
                method: Some("thing.dynamic_register".to_string()),
                product_key: Some(product_key.to_string()),
                device_id: Some(device_id.to_string()),
                data: serde_json::Value::Null,
                code: Some(ResponseCode::REFUSE.to_string()),
                message: Some("invalid product secret".to_string()),
            });
        }

        // Check if device already exists
        let existing = self
            .device_repo
            .find_by_pk_did(product_key, device_id)
            .await?;

        if existing.is_none() {
            // Create new device
            let secret = uuid::Uuid::new_v4().to_string().replace('-', "")[..16].to_string();
            let mut device = Device::new(product_key.to_string(), device_id.to_string());
            device.device_name = device_name.map(|s| s.to_string());
            device.device_secret = Some(secret);
            self.device_repo.create(&device).await?;
            info!(pk = %product_key, did = %device_id, "device dynamically created");
        }

        // Activate
        self.handle_device_online(product_key, device_id).await?;

        Ok(CommonTopicResponse {
            tid: None,
            bid: None,
            version: "1.0".to_string(),
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
            method: Some("thing.dynamic_register".to_string()),
            product_key: Some(product_key.to_string()),
            device_id: Some(device_id.to_string()),
            data: serde_json::Value::Null,
            code: Some(ResponseCode::SUCCESS.to_string()),
            message: Some("success".to_string()),
        })
    }

    /// Get device status from Redis.
    pub async fn get_device_status(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<DeviceStatus>> {
        self.device_state.get_status(product_key, device_id).await
    }

    /// Get full device info from DB.
    pub async fn get_device(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<Device>> {
        self.device_repo.find_by_pk_did(product_key, device_id).await
    }

    /// List all devices for a product key.
    pub async fn list_devices(&self, product_key: &str) -> Result<Vec<Device>> {
        self.device_repo.find_by_product_key(product_key).await
    }

    /// Create a new device via HTTP API.
    pub async fn create_device(&self, device: &Device) -> Result<()> {
        self.device_repo.create(device).await?;
        info!(pk = %device.product_key, did = %device.device_id, "device created via API");
        Ok(())
    }

    /// Update device name/extend fields.
    pub async fn update_device(
        &self,
        product_key: &str,
        device_id: &str,
        device_name: Option<&str>,
        device_extend: Option<&str>,
    ) -> Result<()> {
        self.device_repo
            .update_info(product_key, device_id, device_name, device_extend)
            .await?;
        info!(pk = %product_key, did = %device_id, "device updated via API");
        Ok(())
    }

    /// Delete a device (DB + Redis cleanup).
    pub async fn delete_device(&self, product_key: &str, device_id: &str) -> Result<()> {
        self.device_state.delete(product_key, device_id).await?;
        self.device_repo.delete(product_key, device_id).await?;
        info!(pk = %product_key, did = %device_id, "device deleted via API");
        Ok(())
    }
}
