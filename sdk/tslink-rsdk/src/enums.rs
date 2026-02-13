//! Enumeration types for tslink-rsdk

use serde::{Deserialize, Serialize};

/// Event type enumeration for thing events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    /// Informational event
    Info,
    /// Warning event
    Warning,
    /// Error event
    Error,
}

impl EventType {
    /// Get the string value for the event type (used in topics)
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::Info => "info",
            EventType::Warning => "warning",
            EventType::Error => "error",
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Message QoS level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QoS {
    /// At most once delivery
    #[default]
    AtMostOnce = 0,
    /// At least once delivery
    AtLeastOnce = 1,
    /// Exactly once delivery
    ExactlyOnce = 2,
}

impl From<QoS> for rumqttc::QoS {
    fn from(qos: QoS) -> Self {
        match qos {
            QoS::AtMostOnce => rumqttc::QoS::AtMostOnce,
            QoS::AtLeastOnce => rumqttc::QoS::AtLeastOnce,
            QoS::ExactlyOnce => rumqttc::QoS::ExactlyOnce,
        }
    }
}
