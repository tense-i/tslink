use moka::future::Cache;
use sqlx::MySqlPool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

use crate::domain::thing_model::{
    CallType, DeviceModel, FunctionMethod, MethodField, MethodFieldDataType,
};
use crate::error::Result;

/// Repository for loading device models (物模型) from database with caching.
///
/// Loads from product/module/function/function_param tables.
/// Uses moka cache with 30-second TTL to reduce DB pressure.
pub struct ModelRepository {
    pool: Arc<MySqlPool>,
    cache: Cache<String, Arc<DeviceModel>>,
}

impl ModelRepository {
    pub fn new(pool: Arc<MySqlPool>) -> Self {
        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(30))
            .build();
        Self { pool, cache }
    }

    /// Get device model by product key, using cache.
    pub async fn get_device_model(&self, product_key: &str) -> Result<Option<Arc<DeviceModel>>> {
        let cache_key = product_key.to_string();

        // Try cache first
        if let Some(model) = self.cache.get(&cache_key).await {
            debug!(pk = %product_key, "device model cache hit");
            return Ok(Some(model));
        }

        // Load from DB
        let model = self.load_device_model_from_db(product_key).await?;

        if let Some(ref m) = model {
            let arc_model = Arc::new(m.clone());
            self.cache.insert(cache_key, arc_model.clone()).await;
            info!(pk = %product_key, services = m.services.len(), "device model loaded and cached");
            Ok(Some(arc_model))
        } else {
            Ok(None)
        }
    }

    /// Invalidate cache entry for a product key.
    pub async fn invalidate(&self, product_key: &str) {
        self.cache.invalidate(&product_key.to_string()).await;
    }

    /// Warmup: preload all product models into cache.
    ///
    /// Call this at startup to avoid cold-cache latency on first requests.
    pub async fn warmup(&self) -> Result<usize> {
        let rows = sqlx::query("SELECT product_key FROM product")
            .fetch_all(self.pool.as_ref())
            .await?;

        use sqlx::Row;
        let mut loaded = 0usize;
        for row in &rows {
            let pk: String = row.get("product_key");
            if let Ok(Some(_)) = self.get_device_model(&pk).await {
                loaded += 1;
            }
        }
        info!(count = loaded, "model cache warmup complete");
        Ok(loaded)
    }

    /// Load device model from database tables.
    async fn load_device_model_from_db(&self, product_key: &str) -> Result<Option<DeviceModel>> {
        // Step 1: Load product base info
        let product_row = sqlx::query(
            "SELECT id, product_key, name, product_version, product_type FROM product WHERE product_key = ?"
        )
        .bind(product_key)
        .fetch_optional(self.pool.as_ref())
        .await?;

        let product_row = match product_row {
            Some(r) => r,
            None => return Ok(None),
        };

        use sqlx::Row;
        let product_id: i64 = product_row.get("id");
        let product_name: Option<String> = product_row.get("name");
        let product_version: Option<String> = product_row.get("product_version");
        let product_type: Option<String> = product_row.get("product_type");

        // Step 2: Load function methods (services)
        let function_rows = sqlx::query(
            r#"SELECT f.id, f.identifier, f.method, f.name, f.call_type, f.function_type, f.description
               FROM function_info f
               JOIN module m ON f.module_id = m.id
               WHERE m.product_id = ?"#,
        )
        .bind(product_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut services = std::collections::HashMap::new();
        let mut events = std::collections::HashMap::new();
        let mut properties = Vec::new();

        for row in &function_rows {
            let func_id: i64 = row.get("id");
            let identifier: String = row.get("identifier");
            let method: String = row.get("method");
            let name: Option<String> = row.get("name");
            let call_type_str: Option<String> = row.get("call_type");
            let function_type: Option<String> = row.get("function_type");
            let desc: Option<String> = row.get("description");

            let call_type = call_type_str.and_then(|ct| match ct.to_uppercase().as_str() {
                "SYNC" => Some(CallType::Sync),
                "ASYNC" => Some(CallType::Async),
                _ => None,
            });

            // Load input/output fields for this function
            let param_rows = sqlx::query(
                r#"SELECT name, identifier, data_type, required, description, direction
                   FROM function_param
                   WHERE function_id = ?"#,
            )
            .bind(func_id)
            .fetch_all(self.pool.as_ref())
            .await?;

            let mut input_fields = Vec::new();
            let mut output_fields = Vec::new();

            for param in &param_rows {
                let p_name: String = param.get("name");
                let p_identifier: String = param.get("identifier");
                let p_data_type: Option<String> = param.get("data_type");
                let p_required: bool = param.try_get::<bool, _>("required").unwrap_or(false);
                let p_desc: Option<String> = param.get("description");
                let p_direction: Option<String> = param.get("direction");

                let data_type = p_data_type.map(|dt| MethodFieldDataType {
                    data_type: dt,
                    specs: None,
                });

                let field = MethodField {
                    id: None,
                    name: p_name,
                    identifier: p_identifier,
                    data_type,
                    required: p_required,
                    desc: p_desc,
                    expands: None,
                };

                match p_direction.as_deref() {
                    Some("output") => output_fields.push(field),
                    _ => input_fields.push(field),
                }
            }

            let func_method = FunctionMethod {
                identifier: identifier.clone(),
                method: method.clone(),
                name,
                call_type,
                desc,
                input_fields,
                output_fields,
                expands: None,
            };

            match function_type.as_deref() {
                Some("service") | Some("SERVICE") => {
                    services.insert(method, func_method);
                }
                Some("event") | Some("EVENT") => {
                    events.insert(identifier, func_method);
                }
                Some("property") | Some("PROPERTY") => {
                    // Properties stored as MethodField (simplified)
                    properties.push(MethodField {
                        id: Some(func_id),
                        name: func_method.name.unwrap_or_default(),
                        identifier: func_method.identifier,
                        data_type: None,
                        required: false,
                        desc: func_method.desc,
                        expands: None,
                    });
                }
                _ => {
                    // Default to service
                    services.insert(method, func_method);
                }
            }
        }

        Ok(Some(DeviceModel {
            name: product_name,
            product_key: product_key.to_string(),
            device_id: None,
            product_version,
            product_type,
            properties,
            services,
            events,
            configs: vec![],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_format() {
        let pk = "testProduct123";
        let key = pk.to_string();
        assert_eq!(key, "testProduct123");
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let cache: Cache<String, Arc<DeviceModel>> = Cache::builder()
            .max_capacity(100)
            .time_to_live(Duration::from_secs(1))
            .build();

        let model = Arc::new(DeviceModel {
            name: Some("Test".to_string()),
            product_key: "pk001".to_string(),
            device_id: None,
            product_version: None,
            product_type: None,
            properties: vec![],
            services: std::collections::HashMap::new(),
            events: std::collections::HashMap::new(),
            configs: vec![],
        });

        cache.insert("pk001".to_string(), model.clone()).await;
        assert!(cache.get(&"pk001".to_string()).await.is_some());

        cache.invalidate(&"pk001".to_string()).await;
        assert!(cache.get(&"pk001".to_string()).await.is_none());
    }
}
