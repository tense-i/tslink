use sqlx::{MySqlPool, Row};
use tracing::debug;

use crate::domain::device::{Device, DeviceStatus};
use crate::error::{Result, TsLinkError};

/// Database repository for IoT device CRUD operations.
pub struct DeviceRepository {
    pool: MySqlPool,
}

impl DeviceRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Find a device by product_key and device_id.
    pub async fn find_by_pk_did(
        &self,
        product_key: &str,
        device_id: &str,
    ) -> Result<Option<Device>> {
        let row = sqlx::query(
            "SELECT id, product_id, product_key, device_id, device_name, \
             device_secret, device_status, parent_product_key, parent_id, \
             gmt_last_online, register_time, device_extend, org_code \
             FROM iot_device WHERE product_key = ? AND device_id = ? LIMIT 1",
        )
        .bind(product_key)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row.map(|r| row_to_device(&r)))
    }

    /// Find all devices belonging to a product.
    pub async fn find_by_product_key(&self, product_key: &str) -> Result<Vec<Device>> {
        let rows = sqlx::query(
            "SELECT id, product_id, product_key, device_id, device_name, \
             device_secret, device_status, parent_product_key, parent_id, \
             gmt_last_online, register_time, device_extend, org_code \
             FROM iot_device WHERE product_key = ?",
        )
        .bind(product_key)
        .fetch_all(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(rows.iter().map(row_to_device).collect())
    }

    /// Count devices belonging to a product.
    pub async fn count_by_product_key(&self, product_key: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM iot_device WHERE product_key = ?")
            .bind(product_key)
            .fetch_one(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        Ok(row.try_get::<i64, _>("cnt").unwrap_or(0))
    }

    /// Update device status in the database.
    pub async fn update_status(
        &self,
        product_key: &str,
        device_id: &str,
        status: &DeviceStatus,
    ) -> Result<()> {
        let status_str = status_to_str(status);

        sqlx::query(
            "UPDATE iot_device SET device_status = ? WHERE product_key = ? AND device_id = ?",
        )
        .bind(status_str)
        .bind(product_key)
        .bind(device_id)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        debug!(pk = %product_key, did = %device_id, status = %status_str, "device status updated in DB");
        Ok(())
    }

    /// Create a new device record.
    pub async fn create(&self, device: &Device) -> Result<()> {
        let status_str = status_to_str(&device.device_status);

        sqlx::query(
            "INSERT INTO iot_device (product_id, product_key, device_id, device_name, \
             device_secret, device_status, parent_product_key, parent_id, org_code) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(device.product_id)
        .bind(&device.product_key)
        .bind(&device.device_id)
        .bind(&device.device_name)
        .bind(&device.device_secret)
        .bind(status_str)
        .bind(&device.parent_product_key)
        .bind(&device.parent_id)
        .bind(&device.org_code)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        debug!(pk = %device.product_key, did = %device.device_id, "device created in DB");
        Ok(())
    }

    /// Update device info (name, extend).
    pub async fn update_info(
        &self,
        product_key: &str,
        device_id: &str,
        device_name: Option<&str>,
        device_extend: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE iot_device SET device_name = COALESCE(?, device_name), \
             device_extend = COALESCE(?, device_extend) \
             WHERE product_key = ? AND device_id = ?",
        )
        .bind(device_name)
        .bind(device_extend)
        .bind(product_key)
        .bind(device_id)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        debug!(pk = %product_key, did = %device_id, "device info updated in DB");
        Ok(())
    }

    /// Delete a device record.
    pub async fn delete(&self, product_key: &str, device_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM iot_device WHERE product_key = ? AND device_id = ?")
            .bind(product_key)
            .bind(device_id)
            .execute(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        debug!(pk = %product_key, did = %device_id, "device deleted from DB");
        Ok(())
    }

    /// Verify a device's secret.
    pub async fn verify_secret(
        &self,
        product_key: &str,
        device_id: &str,
        secret: &str,
    ) -> Result<bool> {
        let row = sqlx::query(
            "SELECT device_secret FROM iot_device \
             WHERE product_key = ? AND device_id = ? LIMIT 1",
        )
        .bind(product_key)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row
            .and_then(|r| {
                r.try_get::<Option<String>, _>("device_secret")
                    .ok()
                    .flatten()
            })
            .is_some_and(|s| s == secret))
    }
}

fn status_to_str(status: &DeviceStatus) -> &'static str {
    match status {
        DeviceStatus::Online => "ONLINE",
        DeviceStatus::Offline => "OFFLINE",
        DeviceStatus::Fault => "FAULT",
        DeviceStatus::NotActive => "NOT_ACTIVE",
    }
}

fn row_to_device(row: &sqlx::mysql::MySqlRow) -> Device {
    let status_str: Option<String> = row.try_get("device_status").ok();
    let device_status = match status_str.as_deref() {
        Some("ONLINE") => DeviceStatus::Online,
        Some("OFFLINE") => DeviceStatus::Offline,
        Some("FAULT") => DeviceStatus::Fault,
        Some("NOT_ACTIVE") => DeviceStatus::NotActive,
        _ => DeviceStatus::NotActive,
    };

    let gmt_last_online: Option<chrono::NaiveDateTime> = row.try_get("gmt_last_online").ok();
    let register_time: Option<chrono::NaiveDateTime> = row.try_get("register_time").ok();

    Device {
        id: row.try_get("id").ok(),
        product_id: row.try_get("product_id").ok(),
        product_key: row.try_get("product_key").unwrap_or_default(),
        product_version: None,
        device_id: row.try_get("device_id").unwrap_or_default(),
        device_name: row.try_get("device_name").ok(),
        device_secret: row.try_get("device_secret").ok(),
        device_status,
        parent_product_key: row.try_get("parent_product_key").ok(),
        parent_id: row.try_get("parent_id").ok(),
        gmt_last_online: gmt_last_online
            .map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
        register_time: register_time
            .map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
        device_extend: row.try_get("device_extend").ok(),
        org_code: row.try_get("org_code").ok(),
    }
}
