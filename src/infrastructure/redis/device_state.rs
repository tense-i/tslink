use fred::prelude::*;
use std::sync::Arc;
use tracing::debug;

use crate::domain::device::DeviceStatus;
use crate::error::{Result, TsLinkError};

/// Redis key prefix for device status.
const DEVICE_STATUS_PREFIX: &str = "DEVICE_STATUS";

/// TTL for device status keys (24 hours).
/// If no heartbeat refreshes the key within this window, it expires automatically.
const DEVICE_STATUS_TTL_SECS: i64 = 86400;

/// Manages device online/offline status in Redis.
///
/// Key format: `DEVICE_STATUS_{product_key}_{device_id}`
/// Value: `ONLINE` | `OFFLINE` | `FAULT` | `NOT_ACTIVE`
pub struct DeviceStateRedis {
    client: Arc<RedisClient>,
}

impl DeviceStateRedis {
    pub fn new(client: Arc<RedisClient>) -> Self {
        Self { client }
    }

    /// Build the Redis key for a device status.
    fn key(product_key: &str, device_id: &str) -> String {
        format!("{}_{}_{}", DEVICE_STATUS_PREFIX, product_key, device_id)
    }

    /// Set device status to ONLINE with TTL.
    pub async fn set_online(&self, product_key: &str, device_id: &str) -> Result<()> {
        let key = Self::key(product_key, device_id);
        let expiration = Some(Expiration::EX(DEVICE_STATUS_TTL_SECS));
        self.client
            .set::<(), _, _>(&key, "ONLINE", expiration, None, false)
            .await
            .map_err(|e| TsLinkError::Redis(format!("set_online failed: {}", e)))?;
        debug!(pk = %product_key, did = %device_id, "device status set to ONLINE (TTL={}s)", DEVICE_STATUS_TTL_SECS);
        Ok(())
    }

    /// Set device status to OFFLINE with TTL.
    pub async fn set_offline(&self, product_key: &str, device_id: &str) -> Result<()> {
        let key = Self::key(product_key, device_id);
        let expiration = Some(Expiration::EX(DEVICE_STATUS_TTL_SECS));
        self.client
            .set::<(), _, _>(&key, "OFFLINE", expiration, None, false)
            .await
            .map_err(|e| TsLinkError::Redis(format!("set_offline failed: {}", e)))?;
        debug!(pk = %product_key, did = %device_id, "device status set to OFFLINE (TTL={}s)", DEVICE_STATUS_TTL_SECS);
        Ok(())
    }

    /// Get device status from Redis.
    pub async fn get_status(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<DeviceStatus>> {
        let key = Self::key(product_key, device_id);
        let value: Option<String> = self
            .client
            .get(&key)
            .await
            .map_err(|e| TsLinkError::Redis(format!("get_status failed: {}", e)))?;

        Ok(value.and_then(|v| match v.as_str() {
            "ONLINE" => Some(DeviceStatus::Online),
            "OFFLINE" => Some(DeviceStatus::Offline),
            "FAULT" => Some(DeviceStatus::Fault),
            "NOT_ACTIVE" => Some(DeviceStatus::NotActive),
            _ => None,
        }))
    }

    /// Refresh heartbeat — reset TTL and ensure status is ONLINE.
    pub async fn refresh_heartbeat(&self, product_key: &str, device_id: &str) -> Result<()> {
        let key = Self::key(product_key, device_id);
        let expiration = Some(Expiration::EX(DEVICE_STATUS_TTL_SECS));
        self.client
            .set::<(), _, _>(&key, "ONLINE", expiration, None, false)
            .await
            .map_err(|e| TsLinkError::Redis(format!("refresh_heartbeat failed: {}", e)))?;
        debug!(pk = %product_key, did = %device_id, "heartbeat refreshed (TTL reset)");
        Ok(())
    }

    /// Delete device status (for cleanup).
    pub async fn delete(&self, product_key: &str, device_id: &str) -> Result<()> {
        let key = Self::key(product_key, device_id);
        self.client
            .del::<(), _>(&key)
            .await
            .map_err(|e| TsLinkError::Redis(format!("delete status failed: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_format() {
        let key = DeviceStateRedis::key("pk001", "dev001");
        assert_eq!(key, "DEVICE_STATUS_pk001_dev001");
    }
}
