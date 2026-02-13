use sqlx::{MySqlPool, Row};
use tracing::debug;

use crate::domain::shadow::ShadowServiceConfig;
use crate::error::{Result, TsLinkError};

/// Database repository for device shadow service configurations.
pub struct ShadowRepository {
    pool: MySqlPool,
}

impl ShadowRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Find all shadow service configurations for a product.
    pub async fn find_shadow_services(
        &self,
        product_key: &str,
    ) -> Result<Vec<ShadowServiceConfig>> {
        let rows = sqlx::query(
            "SELECT id, product_key, method, payload, is_enabled \
             FROM iot_device_shadow_service WHERE product_key = ?",
        )
        .bind(product_key)
        .fetch_all(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        let configs = rows
            .iter()
            .filter_map(|row| {
                let method: String = row.try_get("method").ok()?;
                let pk: String = row.try_get("product_key").ok()?;
                let is_enabled: Option<bool> = row.try_get("is_enabled").ok();

                if is_enabled != Some(true) {
                    return None;
                }

                Some(ShadowServiceConfig {
                    product_key: pk,
                    method,
                })
            })
            .collect();

        Ok(configs)
    }

    /// Upsert a shadow service configuration.
    pub async fn upsert_shadow_service(
        &self,
        product_key: &str,
        device_id: &str,
        method: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        let payload_str = serde_json::to_string(payload)?;

        sqlx::query(
            "INSERT INTO iot_device_shadow_service (product_key, device_id, method, payload, is_enabled) \
             VALUES (?, ?, ?, ?, 1) \
             ON DUPLICATE KEY UPDATE payload = ?, is_enabled = 1",
        )
        .bind(product_key)
        .bind(device_id)
        .bind(method)
        .bind(&payload_str)
        .bind(&payload_str)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        debug!(pk = %product_key, did = %device_id, method = %method, "shadow service upserted");
        Ok(())
    }
}
