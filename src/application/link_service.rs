use std::sync::Arc;
use tracing::{debug, info};

use crate::domain::link::Link;
use crate::error::Result;
use crate::infrastructure::redis::link::LinkRedis;

/// Weight increment per message received on a link.
const WEIGHT_INCREMENT: f64 = 1.0;

/// Application service for multi-link device communication.
///
/// Orchestrates link weight management, active link selection,
/// and link routing for service invocations.
pub struct LinkService {
    link_redis: Arc<LinkRedis>,
}

impl LinkService {
    pub fn new(link_redis: Arc<LinkRedis>) -> Self {
        Self { link_redis }
    }

    /// Check and update a link when a message is received.
    ///
    /// Extracts linkSuffix from clientId/deviceId, increments weight.
    /// Returns the link suffix if present.
    pub async fn check_link(
        &self,
        product_key: &str,
        device_id: &str,
        link_suffix: Option<&str>,
    ) -> Result<Option<String>> {
        let link_id = match link_suffix {
            Some(suffix) if !suffix.is_empty() => suffix,
            _ => return Ok(None),
        };

        // Increment weight for this link (sliding window heuristic)
        let new_weight = self
            .link_redis
            .increment_link_weight(product_key, device_id, link_id, WEIGHT_INCREMENT)
            .await?;

        debug!(
            pk = %product_key,
            did = %device_id,
            link_id = %link_id,
            weight = new_weight,
            "link weight incremented"
        );

        Ok(Some(link_id.to_string()))
    }

    /// Select the best link for sending a message to a device.
    ///
    /// Priority: manual active link > highest weight link.
    /// Returns the link suffix to use in the topic device ID segment.
    pub async fn select_link(&self, product_key: &str, device_id: &str) -> Result<Option<String>> {
        // Check for manually set active link first
        if let Some(active) = self
            .link_redis
            .get_active_link(product_key, device_id)
            .await?
        {
            debug!(
                pk = %product_key,
                did = %device_id,
                link_id = %active,
                "using manually set active link"
            );
            return Ok(Some(active));
        }

        // Fall back to highest-weight link
        if let Some(best) = self
            .link_redis
            .get_best_link(product_key, device_id)
            .await?
        {
            debug!(
                pk = %product_key,
                did = %device_id,
                link_id = %best,
                "using highest-weight link"
            );
            return Ok(Some(best));
        }

        // No links known — single link device
        Ok(None)
    }

    /// Get all links for a device, ordered by weight.
    pub async fn get_links(&self, product_key: &str, device_id: &str) -> Result<Vec<Link>> {
        self.link_redis.get_links(product_key, device_id).await
    }

    /// Set the active link manually.
    pub async fn set_active_link(
        &self,
        product_key: &str,
        device_id: &str,
        link_id: &str,
    ) -> Result<()> {
        self.link_redis
            .set_active_link(product_key, device_id, link_id)
            .await?;
        info!(
            pk = %product_key,
            did = %device_id,
            link_id = %link_id,
            "active link manually set"
        );
        Ok(())
    }

    /// Remove a link when disconnected.
    pub async fn remove_link(
        &self,
        product_key: &str,
        device_id: &str,
        link_id: &str,
    ) -> Result<()> {
        self.link_redis
            .remove_link(product_key, device_id, link_id)
            .await
    }

    /// Clean up all link data for a device.
    pub async fn cleanup_device_links(&self, product_key: &str, device_id: &str) -> Result<()> {
        self.link_redis
            .delete_device_links(product_key, device_id)
            .await
    }

    /// Build the device ID segment for MQTT topic, including link suffix if needed.
    ///
    /// Returns `did` or `did_linkSuffix` depending on selected link.
    pub async fn resolve_target_device_id(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<String> {
        match self.select_link(product_key, device_id).await? {
            Some(link_id) => Ok(format!("{}_{}", device_id, link_id)),
            None => Ok(device_id.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_device_id_with_suffix() {
        let did = "did001";
        let link_id = "link1";
        let result = format!("{}_{}", did, link_id);
        assert_eq!(result, "did001_link1");
    }

    #[test]
    fn test_weight_increment_constant() {
        assert_eq!(WEIGHT_INCREMENT, 1.0);
    }
}
