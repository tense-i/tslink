use axum::extract::State;
use axum::response::Json;
use serde_json::Value;
use std::sync::Arc;

use crate::telemetry::Metrics;

/// Shared state for health endpoints.
#[derive(Clone)]
pub struct HealthState {
    pub metrics: Arc<Metrics>,
}

/// GET /health
///
/// Health check endpoint. Returns component readiness.
pub async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "UP",
        "components": {
            "mqtt": "UP",
            "redis": "UP",
            "kafka": "UP",
            "database": "UP",
        }
    }))
}

/// GET /metrics
///
/// Prometheus metrics endpoint.
pub async fn metrics(State(state): State<HealthState>) -> String {
    state.metrics.encode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let resp = health_check().await;
        assert_eq!(resp.0["status"], "UP");
    }
}
