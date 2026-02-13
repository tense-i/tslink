use std::sync::Arc;

use crate::domain::device::ProductType;
use crate::domain::product::Product;
use crate::error::Result;
use crate::infrastructure::database::device_repo::DeviceRepository;
use crate::infrastructure::database::product_repo::ProductRepository;

/// Application service for product management.
pub struct ProductService {
    product_repo: Arc<ProductRepository>,
    device_repo: Arc<DeviceRepository>,
}

impl ProductService {
    pub fn new(product_repo: Arc<ProductRepository>, device_repo: Arc<DeviceRepository>) -> Self {
        Self {
            product_repo,
            device_repo,
        }
    }

    pub async fn create_product(&self, product: &Product) -> Result<()> {
        self.product_repo.create(product).await
    }

    pub async fn get_product(&self, product_key: &str) -> Result<Option<Product>> {
        self.product_repo.get(product_key).await
    }

    pub async fn list_products(
        &self,
        name: Option<&str>,
        product_type: Option<&ProductType>,
        page: i64,
        size: i64,
    ) -> Result<Vec<Product>> {
        let limit = size.max(1).min(200);
        let offset = (page.max(1) - 1) * limit;
        self.product_repo
            .list(name, product_type, limit, offset)
            .await
    }

    pub async fn update_product(&self, product_key: &str, product: &Product) -> Result<()> {
        self.product_repo.update(product_key, product).await
    }

    pub async fn delete_product(&self, product_key: &str) -> Result<bool> {
        let count = self.device_repo.count_by_product_key(product_key).await?;
        if count > 0 {
            return Ok(false);
        }
        self.product_repo.delete(product_key).await?;
        Ok(true)
    }
}
