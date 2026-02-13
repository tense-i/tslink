use sqlx::{MySqlPool, Row};

use crate::domain::device::ProductType;
use crate::domain::product::Product;
use crate::error::{Result, TsLinkError};

/// Database repository for product operations.
pub struct ProductRepository {
    pool: MySqlPool,
}

impl ProductRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get product secret by product_key.
    pub async fn get_secret(&self, product_key: &str) -> Result<Option<String>> {
        let row = sqlx::query(
            "SELECT product_secret FROM product WHERE product_key = ? LIMIT 1",
        )
        .bind(product_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row.and_then(|r| r.try_get::<Option<String>, _>("product_secret").ok().flatten()))
    }

    /// Get product by product_key.
    pub async fn get(&self, product_key: &str) -> Result<Option<Product>> {
        let row = sqlx::query(
            "SELECT id, product_key, product_secret, name, product_version, \
             product_type, description, gmt_create, gmt_modified \
             FROM product WHERE product_key = ? LIMIT 1",
        )
        .bind(product_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(row.map(|r| row_to_product(&r)))
    }

    /// List products with optional name/type filters.
    pub async fn list(
        &self,
        name: Option<&str>,
        product_type: Option<&ProductType>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Product>> {
        let mut sql =
            "SELECT id, product_key, product_secret, name, product_version, \
             product_type, description, gmt_create, gmt_modified \
             FROM product WHERE 1=1"
                .to_string();
        if name.is_some() {
            sql.push_str(" AND name LIKE ?");
        }
        if product_type.is_some() {
            sql.push_str(" AND product_type = ?");
        }
        sql.push_str(" ORDER BY gmt_modified DESC LIMIT ? OFFSET ?");

        let mut query = sqlx::query(&sql);
        if let Some(n) = name {
            query = query.bind(format!("%{}%", n));
        }
        if let Some(pt) = product_type {
            query = query.bind(product_type_to_str(pt));
        }
        let rows = query
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        Ok(rows.iter().map(row_to_product).collect())
    }

    /// Create product record.
    pub async fn create(&self, product: &Product) -> Result<()> {
        sqlx::query(
            "INSERT INTO product (product_key, product_secret, name, product_version, \
             product_type, description) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&product.product_key)
        .bind(&product.product_secret)
        .bind(&product.name)
        .bind(&product.product_version)
        .bind(product.product_type.as_ref().map(product_type_to_str))
        .bind(&product.description)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(())
    }

    /// Update product by product_key.
    pub async fn update(&self, product_key: &str, product: &Product) -> Result<()> {
        sqlx::query(
            "UPDATE product SET product_secret = COALESCE(?, product_secret), \
             name = COALESCE(?, name), product_version = COALESCE(?, product_version), \
             product_type = COALESCE(?, product_type), description = COALESCE(?, description) \
             WHERE product_key = ?",
        )
        .bind(&product.product_secret)
        .bind(&product.name)
        .bind(&product.product_version)
        .bind(product.product_type.as_ref().map(product_type_to_str))
        .bind(&product.description)
        .bind(product_key)
        .execute(&self.pool)
        .await
        .map_err(TsLinkError::Database)?;

        Ok(())
    }

    /// Delete product by product_key.
    pub async fn delete(&self, product_key: &str) -> Result<()> {
        sqlx::query("DELETE FROM product WHERE product_key = ?")
            .bind(product_key)
            .execute(&self.pool)
            .await
            .map_err(TsLinkError::Database)?;

        Ok(())
    }
}

fn row_to_product(row: &sqlx::mysql::MySqlRow) -> Product {
    let product_type = row
        .try_get::<Option<String>, _>("product_type")
        .ok()
        .flatten()
        .and_then(parse_product_type);

    Product {
        id: row.try_get("id").ok(),
        product_key: row
            .try_get::<String, _>("product_key")
            .unwrap_or_default(),
        product_secret: row.try_get("product_secret").ok().flatten(),
        name: row.try_get("name").ok().flatten(),
        product_version: row.try_get("product_version").ok().flatten(),
        product_type,
        description: row.try_get("description").ok().flatten(),
        gmt_create: row.try_get("gmt_create").ok(),
        gmt_modified: row.try_get("gmt_modified").ok(),
    }
}

fn parse_product_type(value: String) -> Option<ProductType> {
    match value.to_lowercase().as_str() {
        "directdevice" => Some(ProductType::DirectDevice),
        "gateway" => Some(ProductType::Gateway),
        "subdevice" => Some(ProductType::SubDevice),
        "unrealdevice" => Some(ProductType::UnrealDevice),
        _ => None,
    }
}

fn product_type_to_str(value: &ProductType) -> String {
    match value {
        ProductType::DirectDevice => "DirectDevice",
        ProductType::Gateway => "Gateway",
        ProductType::SubDevice => "SubDevice",
        ProductType::UnrealDevice => "UnrealDevice",
    }
    .to_string()
}
