use sqlx::{MySqlPool, Row};

use crate::domain::function_param::FunctionParam;
use crate::error::{Result, TsLinkError};

pub struct FunctionParamRepository {
    pool: MySqlPool,
}

impl FunctionParamRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_function(&self, function_id: i64) -> Result<Vec<FunctionParam>> {
        let rows = sqlx::query(
            "SELECT id, product_id, module_id, function_id, param_name, param_identifier, 
             param_type, data_type, specs, rel_param_id, required, 
             gmt_create, gmt_modified, gmt_create_by, gmt_modified_by 
             FROM iot_function_param WHERE function_id = ?",
        )
        .bind(function_id)
        .fetch_all(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(rows.iter().map(|r| self.row_to_param(r)).collect())
    }

    pub async fn list_by_product(&self, product_id: i64) -> Result<Vec<FunctionParam>> {
        let rows = sqlx::query(
            "SELECT id, product_id, module_id, function_id, param_name, param_identifier, 
             param_type, data_type, specs, rel_param_id, required, 
             gmt_create, gmt_modified, gmt_create_by, gmt_modified_by 
             FROM iot_function_param WHERE product_id = ?",
        )
        .bind(product_id)
        .fetch_all(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(rows.iter().map(|r| self.row_to_param(r)).collect())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<FunctionParam>> {
        let row = sqlx::query(
            "SELECT id, product_id, module_id, function_id, param_name, param_identifier, 
             param_type, data_type, specs, rel_param_id, required, 
             gmt_create, gmt_modified, gmt_create_by, gmt_modified_by 
             FROM iot_function_param WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row.map(|r| self.row_to_param(&r)))
    }

    pub async fn find_by_identifier(
        &self,
        function_id: i64,
        param_type: &str,
        identifier: &str,
    ) -> Result<Option<FunctionParam>> {
        let row = sqlx::query(
            "SELECT id, product_id, module_id, function_id, param_name, param_identifier, 
             param_type, data_type, specs, rel_param_id, required, 
             gmt_create, gmt_modified, gmt_create_by, gmt_modified_by 
             FROM iot_function_param 
             WHERE function_id = ? AND param_type = ? AND param_identifier = ? LIMIT 1",
        )
        .bind(function_id)
        .bind(param_type)
        .bind(identifier)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row.map(|r| self.row_to_param(&r)))
    }

    pub async fn create(&self, param: &FunctionParam) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO iot_function_param 
             (product_id, module_id, function_id, param_name, param_identifier, 
              param_type, data_type, specs, rel_param_id, required, gmt_create, gmt_modified) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(), NOW())",
        )
        .bind(param.product_id)
        .bind(param.module_id)
        .bind(param.function_id)
        .bind(&param.param_name)
        .bind(&param.param_identifier)
        .bind(&param.param_type)
        .bind(&param.data_type)
        .bind(&param.specs)
        .bind(param.rel_param_id)
        .bind(param.required)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(result.last_insert_id() as i64)
    }

    pub async fn update(&self, id: i64, param: &FunctionParam) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE iot_function_param SET 
             param_name = ?, param_identifier = ?, param_type = ?, data_type = ?, 
             specs = ?, rel_param_id = ?, required = ?, gmt_modified = NOW() 
             WHERE id = ?",
        )
        .bind(&param.param_name)
        .bind(&param.param_identifier)
        .bind(&param.param_type)
        .bind(&param.data_type)
        .bind(&param.specs)
        .bind(param.rel_param_id)
        .bind(param.required)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM iot_function_param WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_by_function(&self, function_id: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM iot_function_param WHERE function_id = ?")
            .bind(function_id)
            .execute(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        Ok(result.rows_affected())
    }

    fn row_to_param(&self, row: &sqlx::mysql::MySqlRow) -> FunctionParam {
        FunctionParam {
            id: row.try_get("id").ok(),
            product_id: row.try_get("product_id").unwrap_or(0),
            module_id: row.try_get("module_id").ok(),
            function_id: row.try_get("function_id").unwrap_or(0),
            param_name: row.try_get("param_name").unwrap_or_default(),
            param_identifier: row.try_get("param_identifier").unwrap_or_default(),
            param_type: row.try_get("param_type").unwrap_or_default(),
            data_type: row.try_get("data_type").unwrap_or_default(),
            specs: row.try_get("specs").ok(),
            rel_param_id: row.try_get("rel_param_id").ok(),
            required: row.try_get("required").unwrap_or(0),
            gmt_create: row.try_get("gmt_create").ok(),
            gmt_modified: row.try_get("gmt_modified").ok(),
            gmt_create_by: row.try_get("gmt_create_by").ok(),
            gmt_modified_by: row.try_get("gmt_modified_by").ok(),
        }
    }
}
