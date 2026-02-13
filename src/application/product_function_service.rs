use std::sync::Arc;

use crate::domain::product_function::ProductFunction;
use crate::error::Result;
use crate::infrastructure::database::module_repo::ModuleRepository;
use crate::infrastructure::database::product_function_repo::ProductFunctionRepository;
use crate::infrastructure::database::product_repo::ProductRepository;

/// Application service for product function (thing model) management.
pub struct ProductFunctionService {
    product_repo: Arc<ProductRepository>,
    module_repo: Arc<ModuleRepository>,
    function_repo: Arc<ProductFunctionRepository>,
}

impl ProductFunctionService {
    pub fn new(
        product_repo: Arc<ProductRepository>,
        module_repo: Arc<ModuleRepository>,
        function_repo: Arc<ProductFunctionRepository>,
    ) -> Self {
        Self {
            product_repo,
            module_repo,
            function_repo,
        }
    }

    pub async fn list_functions(
        &self,
        product_key: &str,
    ) -> Result<Option<Vec<ProductFunction>>> {
        let product = self.product_repo.get(product_key).await?;
        let product_id = match product.and_then(|p| p.id) {
            Some(id) => id,
            None => return Ok(None),
        };

        let functions = self.function_repo.list_by_product_id(product_id).await?;
        Ok(Some(functions))
    }

    pub async fn create_function(
        &self,
        product_key: &str,
        func: &ProductFunction,
    ) -> Result<Option<i64>> {
        let product = self.product_repo.get(product_key).await?;
        let product_id = match product.and_then(|p| p.id) {
            Some(id) => id,
            None => return Ok(None),
        };

        let module_id = self.module_repo.ensure_default_module(product_id).await?;
        let function_id = self.function_repo.create(module_id, func).await?;
        Ok(Some(function_id))
    }

    pub async fn delete_function(&self, function_id: i64) -> Result<()> {
        self.function_repo.delete(function_id).await
    }
}
