use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::error;

use crate::application::device_service::DeviceService;
use crate::domain::device::Device;

/// Shared application state for device handlers.
#[derive(Clone)]
pub struct DeviceState {
    pub device_service: Arc<DeviceService>,
}

/// Request body for creating a device.
#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    #[serde(rename = "productKey")]
    pub product_key: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(default, rename = "deviceName")]
    pub device_name: Option<String>,
    #[serde(default, rename = "deviceSecret")]
    pub device_secret: Option<String>,
    #[serde(default, rename = "productId")]
    pub product_id: Option<i64>,
    #[serde(default, rename = "orgCode")]
    pub org_code: Option<String>,
}

/// Request body for updating a device.
#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    #[serde(default, rename = "deviceName")]
    pub device_name: Option<String>,
    #[serde(default, rename = "deviceExtend")]
    pub device_extend: Option<String>,
}

/// Query parameters for listing devices.
#[derive(Debug, Deserialize)]
pub struct ListDevicesQuery {
    #[serde(rename = "productKey")]
    pub product_key: String,
}

/// GET /api/v1/devices/{pk}/{did}
///
/// Get device information by product key and device ID.
pub async fn get_device(
    State(state): State<DeviceState>,
    Path((pk, did)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    match state.device_service.get_device(&pk, &did).await {
        Ok(Some(device)) => Ok(Json(serde_json::json!({
            "code": 200,
            "data": device,
        }))),
        Ok(None) => Ok(Json(serde_json::json!({
            "code": 404,
            "message": "device not found",
        }))),
        Err(e) => {
            error!(pk = %pk, did = %did, error = %e, "get_device failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("internal error: {}", e),
            })))
        }
    }
}

/// GET /api/v1/devices?productKey=xxx
///
/// List all devices for a product key.
pub async fn list_devices(
    State(state): State<DeviceState>,
    Query(query): Query<ListDevicesQuery>,
) -> Result<Json<Value>, StatusCode> {
    match state.device_service.list_devices(&query.product_key).await {
        Ok(devices) => Ok(Json(serde_json::json!({
            "code": 200,
            "data": devices,
            "total": devices.len(),
        }))),
        Err(e) => {
            error!(pk = %query.product_key, error = %e, "list_devices failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("internal error: {}", e),
            })))
        }
    }
}

/// POST /api/v1/devices
///
/// Create a new device.
pub async fn create_device(
    State(state): State<DeviceState>,
    Json(req): Json<CreateDeviceRequest>,
) -> Result<Json<Value>, StatusCode> {
    let mut device = Device::new(req.product_key.clone(), req.device_id.clone());
    device.device_name = req.device_name;
    device.device_secret = req.device_secret;
    device.product_id = req.product_id;
    device.org_code = req.org_code;

    match state.device_service.create_device(&device).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "device created",
            "data": {
                "productKey": req.product_key,
                "deviceId": req.device_id,
            }
        }))),
        Err(e) => {
            error!(pk = %req.product_key, did = %req.device_id, error = %e, "create_device failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("create failed: {}", e),
            })))
        }
    }
}

/// PUT /api/v1/devices/{pk}/{did}
///
/// Update device information.
pub async fn update_device(
    State(state): State<DeviceState>,
    Path((pk, did)): Path<(String, String)>,
    Json(req): Json<UpdateDeviceRequest>,
) -> Result<Json<Value>, StatusCode> {
    match state
        .device_service
        .update_device(
            &pk,
            &did,
            req.device_name.as_deref(),
            req.device_extend.as_deref(),
        )
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "device updated",
        }))),
        Err(e) => {
            error!(pk = %pk, did = %did, error = %e, "update_device failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("update failed: {}", e),
            })))
        }
    }
}

/// DELETE /api/v1/devices/{pk}/{did}
///
/// Delete a device.
pub async fn delete_device(
    State(state): State<DeviceState>,
    Path((pk, did)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    match state.device_service.delete_device(&pk, &did).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "device deleted",
        }))),
        Err(e) => {
            error!(pk = %pk, did = %did, error = %e, "delete_device failed");
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("delete failed: {}", e),
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_response_format() {
        let json = serde_json::json!({
            "code": 200,
            "data": {
                "productKey": "pk001",
                "deviceId": "did001",
                "status": "ONLINE",
            }
        });
        assert_eq!(json["code"], 200);
        assert_eq!(json["data"]["productKey"], "pk001");
    }

    #[test]
    fn test_create_request_deserialize() {
        let json = r#"{"productKey":"pk001","deviceId":"did001","deviceName":"test"}"#;
        let req: CreateDeviceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.product_key, "pk001");
        assert_eq!(req.device_id, "did001");
        assert_eq!(req.device_name, Some("test".to_string()));
    }

    #[test]
    fn test_update_request_deserialize() {
        let json = r#"{"deviceName":"new-name","deviceExtend":"{\"key\":\"val\"}"}"#;
        let req: UpdateDeviceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.device_name, Some("new-name".to_string()));
        assert!(req.device_extend.is_some());
    }

    #[test]
    fn test_list_query_struct() {
        let query = ListDevicesQuery {
            product_key: "pk001".to_string(),
        };
        assert_eq!(query.product_key, "pk001");
    }
}
