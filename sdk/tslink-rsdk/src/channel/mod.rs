//! Message channel abstractions and implementations

mod message_channel;
#[cfg(feature = "mqtt")]
mod mqtt_channel;

pub use message_channel::{MessageChannel, MessageReceiveCallback};

#[cfg(feature = "mqtt")]
pub use mqtt_channel::{MqttChannel, MqttConfig};
