use sqlx::{MySqlPool, Row};

use crate::domain::device_type::DeviceType;
use crate::error::{Result, TsLinkError};

/// Repository for device type operations.
pub struct DeviceTypeRepository {
    pool: MySqlPool,
}

impl DeviceTypeRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Find a device type by code.
    pub async fn find_by_code(&self, code: &str) -> Result<Option<DeviceType>> {
        let row = sqlx::query(
            "SELECT id, code, name, description, gmt_create, gmt_modified \
             FROM device_type WHERE code = ? LIMIT 1",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row.map(|r| row_to_device_type(&r)))
    }

    /// List all device types.
    pub async fn list(&self) -> Result<Vec<DeviceType>> {
        let rows = sqlx::query(
            "SELECT id, code, name, description, gmt_create, gmt_modified \
             FROM device_type ORDER BY gmt_modified DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(rows.iter().map(row_to_device_type).collect())
    }

    /// Create a new device type.
    pub async fn create(&self, device_type: &DeviceType) -> Result<()> {
        sqlx::query(
            "INSERT INTO device_type (code, name, description) VALUES (?, ?, ?)",
        )
        .bind(&device_type.code)
        .bind(&device_type.name)
        .bind(&device_type.description)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(())
    }

    /// Update an existing device type.
    pub async fn update(&self, code: &str, name: Option<&str>, description: Option<&str>) -> Result<bool> {
        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(n) = name {
            updates.push("name = ?");
            params.push(n.to_string());
        }
        if let Some(d) = description {
            updates.push("description = ?");
            params.push(d.to_string());
        }

        if updates.is_empty() {
            return Ok(false);
        }

        let sql = format!(
            "UPDATE device_type SET {}, gmt_modified = CURRENT_TIMESTAMP WHERE code = ?",
            updates.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for param in &params {
            query = query.bind(param);
        }
        query = query.bind(code);

        let result = query.execute(&self.pool).await.map_err(TsLinkError::Database)?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete a device type by code.
    pub async fn delete(&self, code: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM device_type WHERE code = ?")
            .bind(code)
            .execute(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        Ok(result.rows_affected() > 0)
    }
}

fn row_to_device_type(row: &sqlx::mysql::MySqlRow) -> DeviceType {
    DeviceType {
        id: row.try_get("id").ok(),
        code: row.try_get::<String, _>("code").unwrap_or_default(),
        name: row.try_get::<String, _>("name").unwrap_or_default(),
        description: row.try_get("description").ok().flatten(),
        gmt_create: row.try_get("gmt_create").ok(),
        gmt_modified: row.try_get("gmt_modified").ok(),
    }
}
