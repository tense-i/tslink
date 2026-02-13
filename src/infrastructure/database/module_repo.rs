use sqlx::{MySqlPool, Row};

use crate::error::{Result, TsLinkError};

/// Repository for product module (thing model module).
pub struct ModuleRepository {
    pool: MySqlPool,
}

impl ModuleRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_product_id(&self, product_id: i64) -> Result<Option<i64>> {
        let row = sqlx::query("SELECT id FROM module WHERE product_id = ? LIMIT 1")
            .bind(product_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        Ok(row.and_then(|r| r.try_get::<i64, _>("id").ok()))
    }

    pub async fn ensure_default_module(&self, product_id: i64) -> Result<i64> {
        if let Some(id) = self.find_by_product_id(product_id).await? {
            return Ok(id);
        }

        let result = sqlx::query(
            "INSERT INTO module (product_id, name, identifier, description) VALUES (?, ?, ?, ?)",
        )
        .bind(product_id)
        .bind("Default Module")
        .bind("default")
        .bind("auto-created")
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(result.last_insert_id() as i64)
    }
}
