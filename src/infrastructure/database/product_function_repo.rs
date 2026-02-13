use sqlx::{MySqlPool, Row};

use crate::domain::product_function::ProductFunction;
use crate::error::{Result, TsLinkError};

/// Repository for product function definitions.
pub struct ProductFunctionRepository {
    pool: MySqlPool,
}

impl ProductFunctionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_product_id(&self, product_id: i64) -> Result<Vec<ProductFunction>> {
        let rows = sqlx::query(
            r#"SELECT f.id, f.module_id, f.identifier, f.method, f.name,
               f.call_type, f.function_type, f.description, f.gmt_create, f.gmt_modified
               FROM function_info f
               JOIN module m ON f.module_id = m.id
               WHERE m.product_id = ?"#,
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(rows.iter().map(row_to_function).collect())
    }

    pub async fn create(&self, module_id: i64, func: &ProductFunction) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO function_info (module_id, identifier, method, name, call_type, function_type, description) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(module_id)
        .bind(&func.identifier)
        .bind(&func.method)
        .bind(&func.name)
        .bind(&func.call_type)
        .bind(&func.function_type)
        .bind(&func.description)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(result.last_insert_id() as i64)
    }

    pub async fn delete(&self, function_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM function_info WHERE id = ?")
            .bind(function_id)
            .execute(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        Ok(())
    }
}

fn row_to_function(row: &sqlx::mysql::MySqlRow) -> ProductFunction {
    ProductFunction {
        id: row.try_get("id").ok(),
        module_id: row.try_get("module_id").ok(),
        identifier: row.try_get("identifier").unwrap_or_default(),
        method: row.try_get("method").unwrap_or_default(),
        name: row.try_get("name").ok(),
        call_type: row.try_get("call_type").ok(),
        function_type: row.try_get("function_type").ok(),
        description: row.try_get("description").ok(),
        gmt_create: row.try_get("gmt_create").ok(),
        gmt_modified: row.try_get("gmt_modified").ok(),
    }
}
