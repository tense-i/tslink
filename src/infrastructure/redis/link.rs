use fred::prelude::*;
use std::sync::Arc;
use tracing::debug;

use crate::domain::link::Link;
use crate::error::{Result, TsLinkError};

/// Redis keys for multi-link device management.
///
/// - `DEVICE_LINK_{pk}_{linkId}` — individual link score (ZSet member in multilink)
/// - `DEVICE_MULTILINK_{pk}_{did}` — ZSet of all links for a device, scored by weight
/// - `ACTIVE_DEVICE_LINK_{pk}_{did}` — String, currently active link ID
const MULTILINK_KEY_PREFIX: &str = "DEVICE_MULTILINK";
const ACTIVE_LINK_KEY_PREFIX: &str = "ACTIVE_DEVICE_LINK";

/// Redis-based multi-link storage.
///
/// Stores link weights in a ZSet, and the active link as a String.
/// Uses a sliding-window approach for QPS-based weight calculation.
pub struct LinkRedis {
    client: Arc<RedisClient>,
}

impl LinkRedis {
    pub fn new(client: Arc<RedisClient>) -> Self {
        Self { client }
    }

    fn multilink_key(product_key: &str, device_id: &str) -> String {
        format!("{}_{}_{}", MULTILINK_KEY_PREFIX, product_key, device_id)
    }

    fn active_link_key(product_key: &str, device_id: &str) -> String {
        format!("{}_{}_{}", ACTIVE_LINK_KEY_PREFIX, product_key, device_id)
    }

    /// Update link weight in the multilink ZSet.
    ///
    /// Higher weight = better link quality.
    pub async fn update_link_weight(
        &self,
        product_key: &str,
        device_id: &str,
        link_id: &str,
        weight: f64,
    ) -> Result<()> {
        let key = Self::multilink_key(product_key, device_id);
        self.client
            .zadd::<(), _, _>(
                &key,
                None,
                None,
                false,
                false,
                (weight, link_id.to_string()),
            )
            .await
            .map_err(|e| TsLinkError::Redis(format!("zadd link weight: {}", e)))?;

        debug!(
            pk = %product_key,
            did = %device_id,
            link_id = %link_id,
            weight = weight,
            "link weight updated"
        );
        Ok(())
    }

    /// Get all links for a device, ordered by weight (descending).
    pub async fn get_links(&self, product_key: &str, device_id: &str) -> Result<Vec<Link>> {
        let key = Self::multilink_key(product_key, device_id);
        let members: Vec<(String, f64)> = self
            .client
            .zrevrange(&key, 0, -1, true)
            .await
            .map_err(|e| TsLinkError::Redis(format!("zrevrange links: {}", e)))?;

        let links = members
            .into_iter()
            .map(|(link_id, weight)| {
                let mut link = Link::new(link_id, product_key.to_string(), device_id.to_string());
                link.weight = weight;
                link
            })
            .collect();

        Ok(links)
    }

    /// Get the currently active link ID.
    pub async fn get_active_link(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<String>> {
        let key = Self::active_link_key(product_key, device_id);
        let result: Option<String> = self
            .client
            .get(&key)
            .await
            .map_err(|e| TsLinkError::Redis(format!("get active link: {}", e)))?;
        Ok(result)
    }

    /// Set the active link for a device.
    pub async fn set_active_link(
        &self,
        product_key: &str,
        device_id: &str,
        link_id: &str,
    ) -> Result<()> {
        let key = Self::active_link_key(product_key, device_id);
        self.client
            .set::<(), _, _>(&key, link_id, None, None, false)
            .await
            .map_err(|e| TsLinkError::Redis(format!("set active link: {}", e)))?;

        debug!(
            pk = %product_key,
            did = %device_id,
            link_id = %link_id,
            "active link set"
        );
        Ok(())
    }

    /// Remove a link from the multilink ZSet.
    pub async fn remove_link(
        &self,
        product_key: &str,
        device_id: &str,
        link_id: &str,
    ) -> Result<()> {
        let key = Self::multilink_key(product_key, device_id);
        self.client
            .zrem::<(), _, _>(&key, link_id.to_string())
            .await
            .map_err(|e| TsLinkError::Redis(format!("zrem link: {}", e)))?;
        Ok(())
    }

    /// Get the highest-weight link ID for a device.
    pub async fn get_best_link(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<String>> {
        let key = Self::multilink_key(product_key, device_id);
        let members: Vec<(String, f64)> = self
            .client
            .zrevrange(&key, 0, 0, true)
            .await
            .map_err(|e| TsLinkError::Redis(format!("zrevrange best link: {}", e)))?;

        Ok(members.into_iter().next().map(|(id, _)| id))
    }

    /// Delete all link data for a device.
    pub async fn delete_device_links(&self, product_key: &str, device_id: &str) -> Result<()> {
        let multilink_key = Self::multilink_key(product_key, device_id);
        let active_key = Self::active_link_key(product_key, device_id);

        let _ = self.client.del::<(), _>(&multilink_key).await;
        let _ = self.client.del::<(), _>(&active_key).await;

        debug!(pk = %product_key, did = %device_id, "device links deleted");
        Ok(())
    }

    /// Increment link weight using sliding-window QPS heuristic.
    ///
    /// Each message received on a link increments its weight by a small delta.
    /// This provides an automatic quality metric.
    pub async fn increment_link_weight(
        &self,
        product_key: &str,
        device_id: &str,
        link_id: &str,
        delta: f64,
    ) -> Result<f64> {
        let key = Self::multilink_key(product_key, device_id);
        let new_score: f64 = self
            .client
            .zincrby(&key, delta, link_id.to_string())
            .await
            .map_err(|e| TsLinkError::Redis(format!("zincrby link weight: {}", e)))?;

        Ok(new_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multilink_key_format() {
        let key = LinkRedis::multilink_key("pk001", "did001");
        assert_eq!(key, "DEVICE_MULTILINK_pk001_did001");
    }

    #[test]
    fn test_active_link_key_format() {
        let key = LinkRedis::active_link_key("pk001", "did001");
        assert_eq!(key, "ACTIVE_DEVICE_LINK_pk001_did001");
    }
}
