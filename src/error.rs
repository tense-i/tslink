use thiserror::Error;

/// Global error type for TSLink.
///
/// All subsystem errors are unified into this enum to provide
/// consistent error handling throughout the application.
#[derive(Error, Debug)]
pub enum TsLinkError {
    // ── Infrastructure errors ────────────────────────────────────
    #[error("MQTT error: {0}")]
    Mqtt(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Kafka error: {0}")]
    Kafka(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    // ── Configuration & parsing ──────────────────────────────────
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Topic parse error: {message}")]
    TopicParse { message: String },

    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    // ── Runtime errors ───────────────────────────────────────────
    #[error("Operation timed out after {duration_ms}ms: {operation}")]
    Timeout { operation: String, duration_ms: u64 },

    #[error("Device not found: product_key={product_key}, device_id={device_id}")]
    DeviceNotFound {
        product_key: String,
        device_id: String,
    },

    #[error("Authentication failed: {reason}")]
    AuthFailed { reason: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience type alias for Results using TsLinkError.
pub type Result<T> = std::result::Result<T, TsLinkError>;

// ── Conversion helpers ───────────────────────────────────────────

impl From<rumqttc::ClientError> for TsLinkError {
    fn from(e: rumqttc::ClientError) -> Self {
        TsLinkError::Mqtt(e.to_string())
    }
}

impl From<rumqttc::ConnectionError> for TsLinkError {
    fn from(e: rumqttc::ConnectionError) -> Self {
        TsLinkError::Mqtt(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TsLinkError::TopicParse {
            message: "invalid format".to_string(),
        };
        assert_eq!(err.to_string(), "Topic parse error: invalid format");
    }

    #[test]
    fn test_device_not_found_display() {
        let err = TsLinkError::DeviceNotFound {
            product_key: "pk001".to_string(),
            device_id: "did001".to_string(),
        };
        assert!(err.to_string().contains("pk001"));
        assert!(err.to_string().contains("did001"));
    }

    #[test]
    fn test_timeout_display() {
        let err = TsLinkError::Timeout {
            operation: "mqtt_publish".to_string(),
            duration_ms: 5000,
        };
        assert!(err.to_string().contains("5000ms"));
        assert!(err.to_string().contains("mqtt_publish"));
    }
}
