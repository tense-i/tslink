//! # tslink-rsdk
//!
//! IoT SDK for Rust - Device-side SDK supporting multi-channel pub/sub.
//!
//! ## Features
//!
//! - **MQTT Channel**: Connect to IoT platform via MQTT protocol
//! - **Property Reporting**: Report device properties to cloud
//! - **Event Reporting**: Report device events (info/warning/error)
//! - **Service Handling**: Receive and handle cloud service calls
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use tslink_rsdk::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = TslinkClientBuilder::new()
//!         .endpoint("mqtt://broker:1883")
//!         .product_key("your_pk")
//!         .device_id("your_did")
//!         .username("your_username")
//!         .password("your_password")
//!         .build()?;
//!
//!     client.start().await?;
//!
//!     // Report property
//!     client.thing_property_post(json!({"temperature": 25.5})).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod adapter;
pub mod channel;
pub mod client;
pub mod enums;
pub mod error;
pub mod message;

pub use error::{Error, Result};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::channel::{MessageChannel, MessageReceiveCallback};
    #[cfg(feature = "mqtt")]
    pub use crate::channel::{MqttChannel, MqttConfig};
    pub use crate::client::{
        DefaultTslinkClient, ServiceCallback, ServiceReplyCallback, TslinkClient, TslinkClientBuilder,
    };
    pub use crate::enums::{EventType, QoS};
    pub use crate::error::{Error, Result};
    pub use crate::message::{CommonMessage, ReplyMessage};
    pub use serde_json::json;
}
