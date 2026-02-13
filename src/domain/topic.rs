use serde::{Deserialize, Serialize};

/// Parsed topic information.
///
/// Extracted from MQTT topic strings like:
/// - `sys/{pk}/{did}/thing/event/property/post`
/// - `region/{region}/sys/{pk}/{did}/thing/service/{method}/post_reply`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicInfo {
    /// Product key extracted from topic
    pub product_key: String,
    /// Device ID extracted from topic (may include link suffix)
    pub device_id: String,
    /// Link suffix for multi-link devices (e.g., "link1" from "did001_link1")
    pub link_suffix: Option<String>,
    /// First-level category: "thing", "platform", or "app"
    pub category: String,
    /// Sub-category path segments (e.g., ["event", "property"])
    pub sub_category: Vec<String>,
    /// Terminal identifier (e.g., "post", "post_reply", "set", "request")
    pub identifier: Option<String>,
    /// Event level for event topics (info, warning, error)
    pub level: Option<String>,
    /// Region prefix if present
    pub region: Option<String>,
}

/// First-level message type classification.
///
/// Maps from Java: `MessageTypeEnum`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    /// Device thing messages: `sys/{pk}/{did}/thing/...`
    Thing,
    /// Platform messages: `sys/{pk}/{did}/platform/...`
    Platform,
    /// App messages: `sys/{pk}/{did}/app/...`
    App,
}

/// Thing message sub-types (second-level routing).
///
/// Maps from Java: `DeviceTopicEnum` (IoT-only subset, no media/video business topics)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThingMessageType {
    /// 属性上报: `thing/event/property/post`
    EventProperty,
    /// 自定义事件: `thing/event/{id}/{info|warning|error}`
    EventCustom,
    /// 属性状态: `thing/properties/state`
    PropertyState,
    /// 服务调用回复: `thing/service/{id}/post_reply`
    ServiceReply,
    /// 属性设置回复: `thing/properties/set_reply`
    PropertySetReply,
    /// 设备注册: `thing/register/post`
    Register,
    /// 动态注册: `thing/dynamic_register/post`
    DynamicRegister,
    /// 心跳回复: `thing/pong/post`
    Pong,
    /// NTP 时间同步: `thing/ntp/post`
    Ntp,
    /// 拓扑更新: `thing/update_topo/...`
    UpdateTopo,
    /// 物模型请求: `thing/device/model/request`
    DeviceModel,
    /// 设备请求: `thing/device/...`
    DeviceRequest,
    /// 配置上报: `thing/config/...`
    Config,
    /// 设备发现: `thing/discovery/...`
    Discovery,
}

/// Platform message sub-types.
///
/// Maps from Java: IoT platform messages only
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlatformMessageType {
    /// 多链路状态: `platform/multilink/state`
    MultiLinkState,
    /// 通用平台服务: `platform/service/{method}/post`
    ServicePost,
}

/// App message sub-types.
///
/// Maps from Java: App interface messages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum AppMessageType {
    /// App 物模型请求: `app/device/model/request`
    DeviceModelRequest,
    /// App 设备发现: `app/device/discovery/request`
    DeviceDiscoveryRequest,
    /// App 服务请求: `app/device/service/post`
    DeviceServiceRequest,
}

/// Topic path constants.
///
/// Maps from Java: `TopicConst`
pub mod topic_const {
    pub const BASIC_PRE: &str = "sys/";
    pub const REGION_PRE: &str = "region/";
    pub const REPLY_SUF: &str = "_reply";
    pub const SERVICE: &str = "/service";
    pub const EVENT: &str = "/event";
    pub const PROPERTIES: &str = "/properties";
    pub const PROPERTY: &str = "/property";
    pub const THING: &str = "thing";
    pub const PLATFORM: &str = "platform";
    pub const APP: &str = "app";
    pub const DEVICE: &str = "/device";
    pub const MODEL: &str = "/model";
    pub const CONFIG: &str = "/config";
    pub const POST: &str = "/post";
    pub const SET: &str = "/set";
    pub const REQUEST: &str = "/request";
    pub const REGISTER: &str = "/register";
    pub const DYNAMIC_REGISTER: &str = "/dynamic_register";
    pub const NTP: &str = "/ntp";
    pub const PONG: &str = "/pong";
    pub const UPDATE_TOPO: &str = "/update_topo";
    pub const STATE: &str = "/state";
    pub const INFO: &str = "info";
    pub const WARNING: &str = "warning";
    pub const ERROR: &str = "error";
}

impl TopicInfo {
    /// Get the raw device ID without link suffix.
    pub fn raw_device_id(&self) -> &str {
        &self.device_id
    }

    /// Get the full device ID including link suffix if present.
    pub fn full_device_id(&self) -> String {
        match &self.link_suffix {
            Some(suffix) => format!("{}_{}", self.device_id, suffix),
            None => self.device_id.clone(),
        }
    }

    /// Determine the first-level message type from the category.
    pub fn message_type(&self) -> Option<MessageType> {
        match self.category.as_str() {
            "thing" => Some(MessageType::Thing),
            "platform" => Some(MessageType::Platform),
            "app" => Some(MessageType::App),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_info_message_type() {
        let topic = TopicInfo {
            product_key: "pk001".to_string(),
            device_id: "did001".to_string(),
            link_suffix: None,
            category: "thing".to_string(),
            sub_category: vec!["event".to_string(), "property".to_string()],
            identifier: Some("post".to_string()),
            level: None,
            region: None,
        };
        assert_eq!(topic.message_type(), Some(MessageType::Thing));
    }

    #[test]
    fn test_topic_info_with_link_suffix() {
        let topic = TopicInfo {
            product_key: "pk001".to_string(),
            device_id: "did001".to_string(),
            link_suffix: Some("link1".to_string()),
            category: "thing".to_string(),
            sub_category: vec![],
            identifier: None,
            level: None,
            region: None,
        };
        assert_eq!(topic.raw_device_id(), "did001");
        assert_eq!(topic.full_device_id(), "did001_link1");
    }

    #[test]
    fn test_message_type_serde() {
        let mt = MessageType::Thing;
        let json = serde_json::to_string(&mt).unwrap();
        assert_eq!(json, "\"Thing\"");

        let deserialized: MessageType = serde_json::from_str("\"Platform\"").unwrap();
        assert_eq!(deserialized, MessageType::Platform);
    }

    #[test]
    fn test_thing_message_type_eq() {
        assert_eq!(
            ThingMessageType::EventProperty,
            ThingMessageType::EventProperty
        );
        assert_ne!(ThingMessageType::EventProperty, ThingMessageType::Pong);
    }
}
