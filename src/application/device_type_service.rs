use std::sync::Arc;

use crate::domain::device_type::DeviceType;
use crate::error::{Result, TsLinkError};
use crate::infrastructure::database::device_type_repo::DeviceTypeRepository;

/// Application service for device type management.
pub struct DeviceTypeService {
    repo: Arc<DeviceTypeRepository>,
}

impl DeviceTypeService {
    pub fn new(repo: Arc<DeviceTypeRepository>) -> Self {
        Self { repo }
    }

    /// List all device types.
    pub async fn list(&self) -> Result<Vec<DeviceType>> {
        self.repo.list().await
    }

    /// Get a device type by code.
    pub async fn get_by_code(&self, code: &str) -> Result<Option<DeviceType>> {
        self.repo.find_by_code(code).await
    }

    /// Create a new device type.
    pub async fn create(&self, code: &str, name: &str, description: Option<&str>) -> Result<DeviceType> {
        if let Some(_existing) = self.repo.find_by_code(code).await? {
            return Err(TsLinkError::Internal(format!(
                "Device type with code '{}' already exists",
                code
            )));
        }

        let device_type = DeviceType {
            id: None,
            code: code.to_string(),
            name: name.to_string(),
            description: description.map(String::from),
            gmt_create: None,
            gmt_modified: None,
        };

        self.repo.create(&device_type).await?;
        Ok(device_type)
    }

    /// Update an existing device type.
    pub async fn update(
        &self,
        code: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<bool> {
        if self.repo.find_by_code(code).await?.is_none() {
            return Err(TsLinkError::Internal(format!(
                "Device type with code '{}' not found",
                code
            )));
        }

        self.repo.update(code, name, description).await
    }

    /// Delete a device type by code.
    pub async fn delete(&self, code: &str) -> Result<bool> {
        self.repo.delete(code).await
    }
}
