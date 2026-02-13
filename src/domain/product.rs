use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::device::ProductType;

/// Product entity.
///
/// Maps to table `product`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Option<i64>,
    pub product_key: String,
    #[serde(default)]
    pub product_secret: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub product_version: Option<String>,
    #[serde(default)]
    pub product_type: Option<ProductType>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub gmt_create: Option<DateTime<Utc>>,
    #[serde(default)]
    pub gmt_modified: Option<DateTime<Utc>>,
}

impl Product {
    pub fn new(product_key: String) -> Self {
        Self {
            id: None,
            product_key,
            product_secret: None,
            name: None,
            product_version: None,
            product_type: None,
            description: None,
            gmt_create: None,
            gmt_modified: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_new() {
        let product = Product::new("pk001".to_string());
        assert_eq!(product.product_key, "pk001");
        assert!(product.product_secret.is_none());
    }
}
