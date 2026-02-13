use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::error;

use crate::application::thing_service::ThingService;

/// Shared application state for service handlers.
#[derive(Clone)]
pub struct ServiceState {
    pub thing_service: Arc<ThingService>,
}

/// Request body for service invocation.
#[derive(Debug, Deserialize)]
pub struct InvokeServiceRequest {
    pub data: Value,
    #[serde(default)]
    pub sync: bool,
}

/// POST /api/v1/devices/{pk}/{did}/services/{method}
///
/// Invoke a service on a device.
pub async fn invoke_service(
    State(state): State<ServiceState>,
    Path((pk, did, method)): Path<(String, String, String)>,
    Json(req): Json<InvokeServiceRequest>,
) -> Result<Json<Value>, StatusCode> {
    match state
        .thing_service
        .invoke_service(&pk, &did, &method, req.data, req.sync)
        .await
    {
        Ok(Some(reply_bytes)) => {
            // Sync call: parse the reply
            match serde_json::from_slice::<Value>(&reply_bytes) {
                Ok(reply) => Ok(Json(serde_json::json!({
                    "code": 200,
                    "message": "success",
                    "data": reply,
                }))),
                Err(_) => Ok(Json(serde_json::json!({
                    "code": 200,
                    "message": "success",
                    "data": null,
                }))),
            }
        }
        Ok(None) => {
            // Async call: no reply
            Ok(Json(serde_json::json!({
                "code": 200,
                "message": "async invocation sent",
            })))
        }
        Err(e) => {
            error!(
                pk = %pk,
                did = %did,
                method = %method,
                error = %e,
                "invoke_service failed"
            );
            Ok(Json(serde_json::json!({
                "code": 500,
                "message": format!("service invocation failed: {}", e),
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoke_request_deserialize() {
        let json = r#"{"data": {"value": 42}, "sync": true}"#;
        let req: InvokeServiceRequest = serde_json::from_str(json).unwrap();
        assert!(req.sync);
        assert_eq!(req.data["value"], 42);
    }

    #[test]
    fn test_invoke_request_default_sync() {
        let json = r#"{"data": {}}"#;
        let req: InvokeServiceRequest = serde_json::from_str(json).unwrap();
        assert!(!req.sync);
    }
}
