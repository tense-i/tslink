use sqlx::{MySqlPool, Row};

use crate::domain::device_config::DeviceConfig;
use crate::error::{Result, TsLinkError};

/// Repository for device configuration operations.
pub struct DeviceConfigRepository {
    pool: MySqlPool,
}

impl DeviceConfigRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_device(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<DeviceConfig>> {
        let row = sqlx::query(
            r#"SELECT id, product_key, device_id, version, config_json, gmt_create, gmt_modified
               FROM device_config WHERE product_key = ? AND device_id = ? LIMIT 1"#,
        )
        .bind(product_key)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row.map(|r| row_to_config(&r)))
    }

    pub async fn upsert(&self, config: &DeviceConfig) -> Result<i64> {
        let new_version = config.version.unwrap_or(0) + 1;
        let config_json_str = config
            .config_json
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "{}".to_string());

        sqlx::query(
            r#"INSERT INTO device_config (product_key, device_id, version, config_json)
               VALUES (?, ?, ?, ?)
               ON DUPLICATE KEY UPDATE 
               config_json = VALUES(config_json),
               version = VALUES(version),
               gmt_modified = CURRENT_TIMESTAMP"#,
        )
        .bind(&config.product_key)
        .bind(&config.device_id)
        .bind(new_version)
        .bind(&config_json_str)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(new_version)
    }

    pub async fn get_version(&self, product_key: &str, device_id: &str) -> Result<Option<i64>> {
        let row = sqlx::query(
            "SELECT version FROM device_config WHERE product_key = ? AND device_id = ? LIMIT 1",
        )
        .bind(product_key)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row.and_then(|r| r.try_get::<i64, _>("version").ok()))
    }
}

fn row_to_config(row: &sqlx::mysql::MySqlRow) -> DeviceConfig {
    let config_json_str: Option<String> = row.try_get("config_json").ok();
    let config_json = config_json_str.and_then(|s| serde_json::from_str(&s).ok());

    DeviceConfig {
        id: row.try_get("id").ok(),
        product_key: row.try_get("product_key").unwrap_or_default(),
        device_id: row.try_get("device_id").unwrap_or_default(),
        version: row.try_get("version").ok(),
        config_json,
        gmt_create: row.try_get("gmt_create").ok(),
        gmt_modified: row.try_get("gmt_modified").ok(),
    }
}
