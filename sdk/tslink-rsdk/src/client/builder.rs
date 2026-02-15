//! Builder for TslinkClient

use crate::enums::QoS;
use crate::error::{Error, Result};

use super::DefaultTslinkClient;

/// Builder for creating TslinkClient instances
#[derive(Default)]
pub struct TslinkClientBuilder {
    endpoint: Option<String>,
    product_key: Option<String>,
    device_id: Option<String>,
    device_secret: Option<String>,
    username: Option<String>,
    password: Option<String>,
    publish_qos: QoS,
    subscribe_qos: QoS,
}

impl TslinkClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            publish_qos: QoS::AtMostOnce,
            subscribe_qos: QoS::AtMostOnce,
            ..Default::default()
        }
    }

    /// Set the MQTT endpoint
    ///
    /// # Arguments
    /// * `endpoint` - MQTT broker endpoint (e.g., "mqtt://broker:1883")
    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the product key
    pub fn product_key(mut self, product_key: impl Into<String>) -> Self {
        self.product_key = Some(product_key.into());
        self
    }

    /// Set the device ID
    pub fn device_id(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = Some(device_id.into());
        self
    }

    /// Set the device secret (optional, used for auto-generating credentials)
    pub fn device_secret(mut self, device_secret: impl Into<String>) -> Self {
        self.device_secret = Some(device_secret.into());
        self
    }

    /// Set the username for MQTT authentication
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password for MQTT authentication
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set the QoS level for publishing messages
    pub fn publish_qos(mut self, qos: QoS) -> Self {
        self.publish_qos = qos;
        self
    }

    /// Set the QoS level for subscribing to topics
    pub fn subscribe_qos(mut self, qos: QoS) -> Self {
        self.subscribe_qos = qos;
        self
    }

    /// Build the TslinkClient
    ///
    /// # Errors
    /// Returns an error if required fields are missing
    pub fn build(self) -> Result<DefaultTslinkClient> {
        let endpoint = self
            .endpoint
            .ok_or_else(|| Error::Configuration("endpoint is required".to_string()))?;

        let product_key = self
            .product_key
            .ok_or_else(|| Error::Configuration("product_key is required".to_string()))?;

        let device_id = self
            .device_id
            .ok_or_else(|| Error::Configuration("device_id is required".to_string()))?;

        let username = self
            .username
            .ok_or_else(|| Error::Configuration("username is required".to_string()))?;

        let password = self
            .password
            .ok_or_else(|| Error::Configuration("password is required".to_string()))?;

        Ok(DefaultTslinkClient::new(
            endpoint,
            product_key,
            device_id,
            username,
            password,
            self.publish_qos,
            self.subscribe_qos,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_missing_endpoint() {
        let result = TslinkClientBuilder::new()
            .product_key("pk")
            .device_id("did")
            .username("user")
            .password("pass")
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("endpoint"));
    }

    #[test]
    fn test_builder_success() {
        let result = TslinkClientBuilder::new()
            .endpoint("mqtt://localhost:1883")
            .product_key("pk")
            .device_id("did")
            .username("user")
            .password("pass")
            .build();

        assert!(result.is_ok());
    }
}
