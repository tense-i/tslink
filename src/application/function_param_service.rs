use std::sync::Arc;

use crate::domain::function_param::FunctionParam;
use crate::error::{Result, TsLinkError};
use crate::infrastructure::database::function_param_repo::FunctionParamRepository;

pub struct FunctionParamService {
    repo: Arc<FunctionParamRepository>,
}

impl FunctionParamService {
    pub fn new(repo: Arc<FunctionParamRepository>) -> Self {
        Self { repo }
    }

    pub async fn list_by_function(&self, function_id: i64) -> Result<Vec<FunctionParam>> {
        self.repo.list_by_function(function_id).await
    }

    pub async fn list_by_product(&self, product_id: i64) -> Result<Vec<FunctionParam>> {
        self.repo.list_by_product(product_id).await
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<FunctionParam>> {
        self.repo.find_by_id(id).await
    }

    pub async fn get_by_identifier(
        &self,
        function_id: i64,
        param_type: &str,
        identifier: &str,
    ) -> Result<Option<FunctionParam>> {
        self.repo
            .find_by_identifier(function_id, param_type, identifier)
            .await
    }

    pub async fn create(&self, param: FunctionParam) -> Result<FunctionParam> {
        if let Some(_existing) = self
            .repo
            .find_by_identifier(param.function_id, &param.param_type, &param.param_identifier)
            .await?
        {
            return Err(TsLinkError::Internal(format!(
                "Function param with identifier '{}' already exists for type '{}'",
                param.param_identifier, param.param_type
            )));
        }

        let id = self.repo.create(&param).await?;
        let mut created = param;
        created.id = Some(id);
        Ok(created)
    }

    pub async fn update(&self, id: i64, param: FunctionParam) -> Result<bool> {
        if self.repo.find_by_id(id).await?.is_none() {
            return Err(TsLinkError::Internal(format!(
                "Function param with id '{}' not found",
                id
            )));
        }
        self.repo.update(id, &param).await
    }

    pub async fn delete(&self, id: i64) -> Result<bool> {
        self.repo.delete(id).await
    }

    pub async fn delete_by_function(&self, function_id: i64) -> Result<u64> {
        self.repo.delete_by_function(function_id).await
    }
}
