use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::Value;
use std::sync::Arc;
use tracing::error;

use crate::application::shadow_service::ShadowService;

/// Shared application state for shadow handlers.
#[derive(Clone)]
pub struct ShadowState {
    pub shadow_service: Arc<ShadowService>,
}

/// GET /api/v1/devices/{pk}/{did}/shadow
///
/// Get device shadow (cached properties).
pub async fn get_shadow(
    State(state): State<ShadowState>,
    Path((pk, did)): Path<(String, String)>,
) -> Result<Json<Value>, StatusCode> {
    match state.shadow_service.get_device_properties(&pk, &did).await {
        Ok(Some(props)) => Ok(Json(serde_json::json!({
            "code": 200,
            "data": {
                "productKey": pk,
                "deviceId": did,
                "properties": props,
            }
        }))),
        Ok(None) => Ok(Json(serde_json::json!({
            "code": 200,
            "data": {
                "productKey": pk,
                "deviceId": did,
                "properties": {},
            }
        }))),
        Err(e) => {
            error!(pk = %pk, did = %did, error = %e, "get_shadow failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// PUT /api/v1/devices/{pk}/{did}/shadow
///
/// Update device shadow properties.
pub async fn update_shadow(
    State(state): State<ShadowState>,
    Path((pk, did)): Path<(String, String)>,
    Json(properties): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    match state
        .shadow_service
        .update_properties(&pk, &did, &properties)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({
            "code": 200,
            "message": "shadow updated",
        }))),
        Err(e) => {
            error!(pk = %pk, did = %did, error = %e, "update_shadow failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_shadow_response_format() {
        let json = serde_json::json!({
            "code": 200,
            "data": {
                "productKey": "pk001",
                "deviceId": "did001",
                "properties": {"temperature": 25.5},
            }
        });
        assert_eq!(json["code"], 200);
        assert_eq!(json["data"]["properties"]["temperature"], 25.5);
    }
}
