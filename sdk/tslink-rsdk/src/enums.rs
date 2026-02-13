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

/// Communication channel type for message routing
/// 
/// This enum determines which channel(s) to use for publishing or subscribing.
/// Mirrors the `CommunicationChannel` enum from ja-IOT-SDK-cpp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunicationChannel {
    /// All channels - publish/subscribe to both MQTT and IPC
    #[default]
    All,
    /// Remote channel only - MQTT for cloud communication
    Remote,
    /// IPC channel only - local inter-process communication
    Ipc,
}

impl CommunicationChannel {
    /// Get the string representation of the channel
    pub fn as_str(&self) -> &'static str {
        match self {
            CommunicationChannel::All => "all",
            CommunicationChannel::Remote => "remote",
            CommunicationChannel::Ipc => "ipc",
        }
    }

    /// Check if this channel includes MQTT
    pub fn includes_mqtt(&self) -> bool {
        matches!(self, CommunicationChannel::All | CommunicationChannel::Remote)
    }

    /// Check if this channel includes IPC
    pub fn includes_ipc(&self) -> bool {
        matches!(self, CommunicationChannel::All | CommunicationChannel::Ipc)
    }
}

impl std::fmt::Display for CommunicationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
