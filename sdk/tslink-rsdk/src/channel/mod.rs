//! Message channel abstractions and implementations

mod message_channel;
mod multi_channel;

#[cfg(feature = "mqtt")]
mod mqtt_channel;
#[cfg(feature = "ipc")]
mod ipc_channel;

pub use message_channel::{MessageChannel, MessageReceiveCallback};
pub use multi_channel::MultiChannel;

#[cfg(feature = "mqtt")]
pub use mqtt_channel::{MqttChannel, MqttConfig};

#[cfg(feature = "ipc")]
pub use ipc_channel::{IpcChannel, IpcConfig, IpcPayload};
