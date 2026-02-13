//! Message channel trait definition

use async_trait::async_trait;

use crate::error::Result;

/// Trait for message channels supporting pub/sub communication
///
/// This trait provides an abstraction over different transport protocols:
/// - MQTT (implemented)
/// - IPC (future)
/// - HTTP (future)
#[async_trait]
pub trait MessageChannel: Send + Sync {
    /// Send a message to the specified topic
    ///
    /// # Arguments
    /// * `topic` - The topic to publish to
    /// * `data` - The message payload as a string
    async fn send(&self, topic: &str, data: &str) -> Result<()>;

    /// Start the channel and establish connection
    async fn start(&self) -> Result<()>;

    /// Stop the channel and close connection
    async fn stop(&self) -> Result<()>;

    /// Add a topic subscription
    async fn add_topic(&self, topic: &str) -> Result<()>;

    /// Check if the channel is connected
    fn is_connected(&self) -> bool;
}

/// Callback trait for receiving messages
pub trait MessageReceiveCallback: Send + Sync {
    /// Called when a message is received
    ///
    /// # Arguments
    /// * `topic` - The topic the message was received on
    /// * `data` - The message payload as a string
    fn receive(&self, topic: &str, data: &str);
}

/// Function-based callback wrapper
pub struct FnCallback<F>
where
    F: Fn(&str, &str) + Send + Sync,
{
    callback: F,
}

impl<F> FnCallback<F>
where
    F: Fn(&str, &str) + Send + Sync,
{
    /// Create a new function-based callback
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> MessageReceiveCallback for FnCallback<F>
where
    F: Fn(&str, &str) + Send + Sync,
{
    fn receive(&self, topic: &str, data: &str) {
        (self.callback)(topic, data);
    }
}
