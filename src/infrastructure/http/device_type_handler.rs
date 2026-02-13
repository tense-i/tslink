use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::application::device_type_service::DeviceTypeService;
use crate::domain::device_type::DeviceType;

/// Shared state for device type handlers.
#[derive(Clone)]
pub struct DeviceTypeState {
    pub service: Arc<DeviceTypeService>,
}

#[derive(Debug, Serialize)]
pub struct DeviceTypeResponse {
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<DeviceType> for DeviceTypeResponse {
    fn from(dt: DeviceType) -> Self {
        Self {
            code: dt.code,
            name: dt.name,
            description: dt.description,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceTypeRequest {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceTypeRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// GET /api/v1/device-types
pub async fn list_device_types(
    State(state): State<DeviceTypeState>,
) -> impl IntoResponse {
    match state.service.list().await {
        Ok(types) => {
            let responses: Vec<DeviceTypeResponse> = types.into_iter().map(Into::into).collect();
            (StatusCode::OK, Json(ApiResponse::ok(responses)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<DeviceTypeResponse>>::err(e.to_string())),
        ),
    }
}

/// GET /api/v1/device-types/:code
pub async fn get_device_type(
    State(state): State<DeviceTypeState>,
    Path(code): Path<String>,
) -> impl IntoResponse {
    match state.service.get_by_code(&code).await {
        Ok(Some(dt)) => {
            let response: DeviceTypeResponse = dt.into();
            (StatusCode::OK, Json(ApiResponse::ok(response)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<DeviceTypeResponse>::err(format!(
                "Device type '{}' not found",
                code
            ))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<DeviceTypeResponse>::err(e.to_string())),
        ),
    }
}

/// POST /api/v1/device-types
pub async fn create_device_type(
    State(state): State<DeviceTypeState>,
    Json(req): Json<CreateDeviceTypeRequest>,
) -> impl IntoResponse {
    match state
        .service
        .create(&req.code, &req.name, req.description.as_deref())
        .await
    {
        Ok(dt) => {
            let response: DeviceTypeResponse = dt.into();
            (StatusCode::CREATED, Json(ApiResponse::ok(response)))
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<DeviceTypeResponse>::err(e.to_string())),
        ),
    }
}

/// PUT /api/v1/device-types/:code
pub async fn update_device_type(
    State(state): State<DeviceTypeState>,
    Path(code): Path<String>,
    Json(req): Json<UpdateDeviceTypeRequest>,
) -> impl IntoResponse {
    match state
        .service
        .update(&code, req.name.as_deref(), req.description.as_deref())
        .await
    {
        Ok(true) => (
            StatusCode::OK,
            Json(ApiResponse::ok(serde_json::json!({"updated": true}))),
        ),
        Ok(false) => (
            StatusCode::OK,
            Json(ApiResponse::ok(serde_json::json!({"updated": false}))),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        ),
    }
}

/// DELETE /api/v1/device-types/:code
pub async fn delete_device_type(
    State(state): State<DeviceTypeState>,
    Path(code): Path<String>,
) -> impl IntoResponse {
    match state.service.delete(&code).await {
        Ok(true) => (
            StatusCode::OK,
            Json(ApiResponse::ok(serde_json::json!({"deleted": true}))),
        ),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::err(format!(
                "Device type '{}' not found",
                code
            ))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<serde_json::Value>::err(e.to_string())),
        ),
    }
}
