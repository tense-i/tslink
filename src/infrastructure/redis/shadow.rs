use fred::prelude::*;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

use crate::error::{Result, TsLinkError};

/// Redis key prefix for device property shadow.
const DEVICE_PROPERTIES_PREFIX: &str = "IOT_DEVICE_PROPERTIES_KEY";

/// TTL for shadow property keys (7 days).
const SHADOW_TTL_SECS: i64 = 604800;

/// Manages device property shadow in Redis.
///
/// Key format: `IOT_DEVICE_PROPERTIES_KEY_{product_key}_{device_id}`
/// Value: JSON object of property key-values
pub struct ShadowRedis {
    client: Arc<RedisClient>,
}

impl ShadowRedis {
    pub fn new(client: Arc<RedisClient>) -> Self {
        Self { client }
    }

    fn key(product_key: &str, device_id: &str) -> String {
        format!("{}_{}_{}", DEVICE_PROPERTIES_PREFIX, product_key, device_id)
    }

    /// Get all device properties as a JSON Value.
    pub async fn get_properties(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<Value>> {
        let key = Self::key(product_key, device_id);
        let raw: Option<String> = self
            .client
            .get(&key)
            .await
            .map_err(|e| TsLinkError::Redis(format!("get_properties: {}", e)))?;

        match raw {
            Some(s) => {
                let v: Value = serde_json::from_str(&s)?;
                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    /// Merge new properties into existing shadow.
    ///
    /// Does a shallow merge: new_props keys override existing.
    pub async fn merge_properties(
        &self,
        product_key: &str,
        device_id: &str,
        new_props: &Value,
    ) -> Result<()> {
        let key = Self::key(product_key, device_id);

        // Read existing
        let raw: Option<String> = self
            .client
            .get(&key)
            .await
            .map_err(|e| TsLinkError::Redis(format!("merge_properties read: {}", e)))?;

        let mut current = match raw {
            Some(s) => {
                serde_json::from_str::<Value>(&s).unwrap_or(Value::Object(Default::default()))
            }
            None => Value::Object(Default::default()),
        };

        // Merge
        if let (Some(current_obj), Some(new_obj)) = (current.as_object_mut(), new_props.as_object())
        {
            for (k, v) in new_obj {
                current_obj.insert(k.clone(), v.clone());
            }
        }

        let serialized = serde_json::to_string(&current)?;
        let expiration = Some(Expiration::EX(SHADOW_TTL_SECS));
        self.client
            .set::<(), _, _>(&key, serialized.as_str(), expiration, None, false)
            .await
            .map_err(|e| TsLinkError::Redis(format!("merge_properties write: {}", e)))?;

        debug!(pk = %product_key, did = %device_id, "shadow properties merged");
        Ok(())
    }

    /// Delete shadow for a device.
    pub async fn delete(&self, product_key: &str, device_id: &str) -> Result<()> {
        let key = Self::key(product_key, device_id);
        self.client
            .del::<(), _>(&key)
            .await
            .map_err(|e| TsLinkError::Redis(format!("delete shadow: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_format() {
        let key = ShadowRedis::key("pk001", "dev001");
        assert_eq!(key, "IOT_DEVICE_PROPERTIES_KEY_pk001_dev001");
    }
}
