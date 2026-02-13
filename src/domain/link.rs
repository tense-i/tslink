use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Multi-link access strategy.
///
/// Maps from Java: `AcsEnum`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LinkAccessStrategy {
    /// Automatic — use highest-weight link
    Acs,
    /// Manual — use designated master link
    Master,
    /// Broadcast — send to all links
    All,
}

/// Device communication link.
///
/// Maps from Java: `Link` BO
/// Represents one physical connection path to a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// Link identifier (e.g., link suffix from topic)
    pub link_id: String,
    /// Product key
    pub product_key: String,
    /// Device ID
    pub device_id: String,
    /// Whether this link is currently active (primary)
    pub is_active: bool,
    /// Link quality weight (higher = better)
    pub weight: f64,
    /// Last message timestamp on this link
    pub last_message_time: Option<DateTime<Utc>>,
}

/// Multi-link method routing configuration.
///
/// Maps from Java: `IotMultilinkMethod` — defines which link strategy
/// to use for specific service methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultilinkMethodConfig {
    pub product_key: String,
    /// Service/event method name
    pub method: String,
    /// Access strategy for this method
    pub acs: LinkAccessStrategy,
    /// Method type (SERVICE, EVENTS, PROPERTIES)
    pub method_type: String,
}

/// Multi-link weight configuration.
///
/// Maps from Java: `IotMultilinkWeight`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultilinkWeightConfig {
    pub product_key: String,
    /// Maximum weight value
    pub max_weight: f64,
    /// Weight name/label
    pub name: String,
    /// Whether automatic computation is enabled
    pub is_acs: bool,
}

impl Link {
    /// Create a new link with default values.
    pub fn new(link_id: String, product_key: String, device_id: String) -> Self {
        Self {
            link_id,
            product_key,
            device_id,
            is_active: false,
            weight: 0.0,
            last_message_time: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_new() {
        let link = Link::new(
            "link1".to_string(),
            "pk001".to_string(),
            "did001".to_string(),
        );
        assert_eq!(link.link_id, "link1");
        assert!(!link.is_active);
        assert_eq!(link.weight, 0.0);
    }

    #[test]
    fn test_link_access_strategy_serde() {
        let acs = LinkAccessStrategy::Acs;
        let json = serde_json::to_string(&acs).unwrap();
        assert_eq!(json, "\"ACS\"");

        let master: LinkAccessStrategy = serde_json::from_str("\"MASTER\"").unwrap();
        assert_eq!(master, LinkAccessStrategy::Master);
    }

    #[test]
    fn test_link_serde_roundtrip() {
        let link = Link::new(
            "link1".to_string(),
            "pk001".to_string(),
            "did001".to_string(),
        );
        let json = serde_json::to_string(&link).unwrap();
        let deserialized: Link = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.link_id, "link1");
        assert_eq!(deserialized.product_key, "pk001");
    }
}
