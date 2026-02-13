//! Multi-channel manager for routing messages to MQTT and/or IPC channels

use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::enums::CommunicationChannel;
use crate::error::Result;
use crate::channel::{MessageChannel, MessageReceiveCallback};

#[cfg(feature = "mqtt")]
use crate::channel::MqttChannel;

#[cfg(feature = "ipc")]
use crate::channel::IpcChannel;

/// Multi-channel manager that routes messages to MQTT and/or IPC channels
/// based on the specified CommunicationChannel.
pub struct MultiChannel {
    #[cfg(feature = "mqtt")]
    mqtt_channel: Option<Arc<MqttChannel>>,
    
    #[cfg(feature = "ipc")]
    ipc_channel: Option<Arc<IpcChannel>>,
    
    default_channel: CommunicationChannel,
    callback: Arc<RwLock<Option<Arc<dyn MessageReceiveCallback>>>>,
}

impl MultiChannel {
    /// Create a new MultiChannel with optional MQTT and IPC channels
    #[allow(unused_variables)]
    pub fn new(
        #[cfg(feature = "mqtt")] mqtt_channel: Option<Arc<MqttChannel>>,
        #[cfg(feature = "ipc")] ipc_channel: Option<Arc<IpcChannel>>,
        default_channel: CommunicationChannel,
    ) -> Self {
        Self {
            #[cfg(feature = "mqtt")]
            mqtt_channel,
            #[cfg(feature = "ipc")]
            ipc_channel,
            default_channel,
            callback: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the message receive callback for IPC channel
    /// Note: MQTT channel callback is set at construction time
    #[cfg(feature = "ipc")]
    pub fn set_ipc_callback(&self, callback: Arc<dyn MessageReceiveCallback>) {
        if let Some(ipc) = &self.ipc_channel {
            ipc.set_callback(callback.clone());
        }
        
        // Store for future use
        let cb = self.callback.clone();
        tokio::spawn(async move {
            *cb.write().await = Some(callback);
        });
    }

    /// Get the default communication channel
    pub fn default_channel(&self) -> CommunicationChannel {
        self.default_channel
    }

    /// Check if MQTT channel is available and connected
    #[cfg(feature = "mqtt")]
    pub fn mqtt_available(&self) -> bool {
        self.mqtt_channel.as_ref().map(|c| c.is_connected()).unwrap_or(false)
    }

    /// Check if IPC channel is available and connected
    #[cfg(feature = "ipc")]
    pub fn ipc_available(&self) -> bool {
        self.ipc_channel.as_ref().map(|c| c.is_connected()).unwrap_or(false)
    }

    /// Send message using the specified channel
    pub async fn send_with_channel(
        &self,
        topic: &str,
        message: &str,
        channel: CommunicationChannel,
    ) -> Result<()> {
        let mut sent = false;
        
        #[cfg(feature = "mqtt")]
        if channel.includes_mqtt() {
            if let Some(mqtt) = &self.mqtt_channel {
                if mqtt.is_connected() {
                    mqtt.send(topic, message).await?;
                    debug!(topic = %topic, channel = "mqtt", "Message sent via MQTT");
                    sent = true;
                } else {
                    warn!(topic = %topic, "MQTT channel not connected, skipping");
                }
            }
        }
        
        #[cfg(feature = "ipc")]
        if channel.includes_ipc() {
            if let Some(ipc) = &self.ipc_channel {
                if ipc.is_connected() {
                    ipc.send(topic, message).await?;
                    debug!(topic = %topic, channel = "ipc", "Message sent via IPC");
                    sent = true;
                } else {
                    warn!(topic = %topic, "IPC channel not connected, skipping");
                }
            }
        }
        
        if !sent {
            warn!(topic = %topic, channel = %channel, "No channel available for sending");
        }
        
        Ok(())
    }

    /// Subscribe to topic using the specified channel
    pub async fn subscribe_with_channel(
        &self,
        topic: &str,
        channel: CommunicationChannel,
    ) -> Result<()> {
        #[cfg(feature = "mqtt")]
        if channel.includes_mqtt() {
            if let Some(mqtt) = &self.mqtt_channel {
                mqtt.add_topic(topic).await?;
                debug!(topic = %topic, channel = "mqtt", "Subscribed via MQTT");
            }
        }
        
        #[cfg(feature = "ipc")]
        if channel.includes_ipc() {
            if let Some(ipc) = &self.ipc_channel {
                ipc.add_topic(topic).await?;
                debug!(topic = %topic, channel = "ipc", "Subscribed via IPC");
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl MessageChannel for MultiChannel {
    async fn send(&self, topic: &str, message: &str) -> Result<()> {
        self.send_with_channel(topic, message, self.default_channel).await
    }

    async fn add_topic(&self, topic: &str) -> Result<()> {
        self.subscribe_with_channel(topic, self.default_channel).await
    }

    async fn start(&self) -> Result<()> {
        #[cfg(feature = "mqtt")]
        if let Some(mqtt) = &self.mqtt_channel {
            mqtt.start().await?;
        }
        
        #[cfg(feature = "ipc")]
        if let Some(ipc) = &self.ipc_channel {
            ipc.start().await?;
        }
        
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        #[cfg(feature = "mqtt")]
        if let Some(mqtt) = &self.mqtt_channel {
            mqtt.stop().await?;
        }
        
        #[cfg(feature = "ipc")]
        if let Some(ipc) = &self.ipc_channel {
            ipc.stop().await?;
        }
        
        Ok(())
    }

    fn is_connected(&self) -> bool {
        let mut connected = false;
        
        #[cfg(feature = "mqtt")]
        if let Some(mqtt) = &self.mqtt_channel {
            connected = connected || mqtt.is_connected();
        }
        
        #[cfg(feature = "ipc")]
        if let Some(ipc) = &self.ipc_channel {
            connected = connected || ipc.is_connected();
        }
        
        connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_communication_channel_includes() {
        assert!(CommunicationChannel::All.includes_mqtt());
        assert!(CommunicationChannel::All.includes_ipc());
        
        assert!(CommunicationChannel::Remote.includes_mqtt());
        assert!(!CommunicationChannel::Remote.includes_ipc());
        
        assert!(!CommunicationChannel::Ipc.includes_mqtt());
        assert!(CommunicationChannel::Ipc.includes_ipc());
    }

    #[test]
    fn test_default_channel() {
        assert_eq!(CommunicationChannel::default(), CommunicationChannel::All);
    }
}
