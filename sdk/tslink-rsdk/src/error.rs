//! Error types for tslink-rsdk

use thiserror::Error;

/// Result type alias for tslink-rsdk operations
pub type Result<T> = std::result::Result<T, Error>;

/// SDK error types
#[derive(Error, Debug)]
pub enum Error {
    /// MQTT connection error
    #[error("MQTT connection error: {0}")]
    MqttConnection(String),

    /// MQTT publish error
    #[error("MQTT publish error: {0}")]
    MqttPublish(String),

    /// MQTT subscribe error
    #[error("MQTT subscribe error: {0}")]
    MqttSubscribe(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Channel error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Timeout error
    #[error("Operation timeout: {0}")]
    Timeout(String),

    /// Callback not found
    #[error("Callback not found for: {0}")]
    CallbackNotFound(String),

    /// Client not started
    #[error("Client not started")]
    NotStarted,

    /// Client already started
    #[error("Client already started")]
    AlreadyStarted,

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<rumqttc::ClientError> for Error {
    fn from(err: rumqttc::ClientError) -> Self {
        Error::MqttConnection(err.to_string())
    }
}

impl From<rumqttc::ConnectionError> for Error {
    fn from(err: rumqttc::ConnectionError) -> Self {
        Error::MqttConnection(err.to_string())
    }
}
