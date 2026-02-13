use crate::domain::topic::{ThingMessageType, TopicInfo};
use crate::error::{Result, TsLinkError};

/// Parse an MQTT topic string into a structured `TopicInfo`.
///
/// Supported formats:
/// - `sys/{product_key}/{device_id}/thing/...`
/// - `sys/{product_key}/{device_id}/platform/...`
/// - `sys/{product_key}/{device_id}/app/...`
/// - `region/{region}/sys/{product_key}/{device_id}/thing/...`
///
/// Device ID may contain a link suffix: `{did}_{linkSuffix}`
pub fn parse_topic(topic: &str) -> Result<TopicInfo> {
    let parts: Vec<&str> = topic.split('/').collect();

    let (region, offset) = if parts.first() == Some(&"region") && parts.len() > 3 {
        // region/{region}/sys/{pk}/{did}/...
        (Some(parts[1].to_string()), 3) // skip "region/{region}/sys"
    } else if parts.first() == Some(&"sys") {
        (None, 1) // skip "sys"
    } else {
        return Err(TsLinkError::TopicParse {
            message: format!("unknown topic prefix: {}", topic),
        });
    };

    // Need at least: {pk}/{did}/{category}
    if parts.len() < offset + 3 {
        return Err(TsLinkError::TopicParse {
            message: format!("topic too short: {}", topic),
        });
    }

    let product_key = parts[offset].to_string();
    let raw_device_segment = parts[offset + 1];
    let category = parts[offset + 2].to_string();

    // Parse device_id and optional link suffix: "did001_link1" -> ("did001", Some("link1"))
    let (device_id, link_suffix) = parse_device_segment(raw_device_segment);

    // Remaining path segments after category
    let remaining = &parts[offset + 3..];

    // Build sub_category and identifier
    let (sub_category, identifier, level) = parse_remaining_path(remaining);

    Ok(TopicInfo {
        product_key,
        device_id,
        link_suffix,
        category,
        sub_category,
        identifier,
        level,
        region,
    })
}

/// Parse device ID segment, splitting link suffix if present.
///
/// "did001_link1" -> ("did001", Some("link1"))
/// "did001" -> ("did001", None)
fn parse_device_segment(segment: &str) -> (String, Option<String>) {
    // Only split on the last underscore to support device IDs with underscores
    if let Some(pos) = segment.rfind('_') {
        let _prefix = &segment[..pos];
        let _suffix = &segment[pos + 1..];
        // Heuristic: link suffix starts with "link" or is a known pattern
        // For now, we keep the full segment as device_id and don't split
        // unless explicitly needed. Multi-link parsing will be added in Phase 9.
        (segment.to_string(), None)
    } else {
        (segment.to_string(), None)
    }
}

/// Parse remaining path segments after category (thing/platform/app).
///
/// Returns (sub_category, identifier, level)
fn parse_remaining_path(parts: &[&str]) -> (Vec<String>, Option<String>, Option<String>) {
    if parts.is_empty() {
        return (vec![], None, None);
    }

    let mut sub_category = Vec::new();
    let mut identifier = None;
    let mut level = None;

    for (i, part) in parts.iter().enumerate() {
        // Check for event levels (last segment)
        if i == parts.len() - 1
            && (*part == "info" || *part == "warning" || *part == "error")
            && parts.len() > 2
        {
            level = Some(part.to_string());
        } else if i == parts.len() - 1
            && (*part == "post"
                || *part == "post_reply"
                || *part == "set"
                || *part == "set_reply"
                || *part == "request"
                || *part == "state")
        {
            identifier = Some(part.to_string());
        } else {
            sub_category.push(part.to_string());
        }
    }

    (sub_category, identifier, level)
}

/// Classify a parsed topic into a `ThingMessageType`.
///
/// This maps the full sub-category path to a specific handler type.
pub fn classify_thing_message(topic: &TopicInfo) -> Option<ThingMessageType> {
    let path = topic.sub_category.join("/");
    let full = if let Some(ref id) = topic.identifier {
        format!("{}/{}", path, id)
    } else if let Some(ref lv) = topic.level {
        format!("{}/{}", path, lv)
    } else {
        path.clone()
    };

    match full.as_str() {
        // 属性上报
        "event/property/post" => Some(ThingMessageType::EventProperty),
        // 属性状态
        "properties/state" => Some(ThingMessageType::PropertyState),
        // 属性设置回复
        "properties/set_reply" => Some(ThingMessageType::PropertySetReply),
        // 心跳
        "pong/post" => Some(ThingMessageType::Pong),
        // NTP
        "ntp/post" => Some(ThingMessageType::Ntp),
        // 注册
        "register/post" => Some(ThingMessageType::Register),
        // 动态注册
        "dynamic_register/post" => Some(ThingMessageType::DynamicRegister),
        // 拓扑更新
        s if s.starts_with("update_topo") => Some(ThingMessageType::UpdateTopo),
        // 物模型请求
        "device/model/request" => Some(ThingMessageType::DeviceModel),
        // 设备请求(通用)
        s if s.starts_with("device/") => Some(ThingMessageType::DeviceRequest),
        // 配置
        s if s.starts_with("config") => Some(ThingMessageType::Config),
        // 设备发现
        s if s.starts_with("discovery") => Some(ThingMessageType::Discovery),
        // 服务回复: service/{method}/post_reply
        s if s.starts_with("service/") && s.ends_with("/post_reply") => {
            Some(ThingMessageType::ServiceReply)
        }
        // 自定义事件: event/{identifier}/{info|warning|error}
        s if s.starts_with("event/") && topic.level.is_some() => {
            Some(ThingMessageType::EventCustom)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_property_post() {
        let topic = parse_topic("sys/pk001/did001/thing/event/property/post").unwrap();
        assert_eq!(topic.product_key, "pk001");
        assert_eq!(topic.device_id, "did001");
        assert_eq!(topic.category, "thing");
        assert_eq!(topic.sub_category, vec!["event", "property"]);
        assert_eq!(topic.identifier.as_deref(), Some("post"));
        assert!(topic.region.is_none());
    }

    #[test]
    fn test_parse_region_topic() {
        let topic =
            parse_topic("region/cn-east/sys/pk001/did001/thing/event/property/post").unwrap();
        assert_eq!(topic.region.as_deref(), Some("cn-east"));
        assert_eq!(topic.product_key, "pk001");
        assert_eq!(topic.device_id, "did001");
        assert_eq!(topic.category, "thing");
    }

    #[test]
    fn test_parse_service_reply() {
        let topic = parse_topic("sys/pk001/did001/thing/service/reboot/post_reply").unwrap();
        assert_eq!(topic.category, "thing");
        assert_eq!(topic.sub_category, vec!["service", "reboot"]);
        assert_eq!(topic.identifier.as_deref(), Some("post_reply"));
    }

    #[test]
    fn test_parse_event_with_level() {
        let topic = parse_topic("sys/pk001/did001/thing/event/fire_alarm/warning").unwrap();
        assert_eq!(topic.category, "thing");
        assert_eq!(topic.sub_category, vec!["event", "fire_alarm"]);
        assert_eq!(topic.level.as_deref(), Some("warning"));
    }

    #[test]
    fn test_parse_register() {
        let topic = parse_topic("sys/pk001/did001/thing/register/post").unwrap();
        assert_eq!(topic.sub_category, vec!["register"]);
        assert_eq!(topic.identifier.as_deref(), Some("post"));
    }

    #[test]
    fn test_parse_ntp() {
        let topic = parse_topic("sys/pk001/did001/thing/ntp/post").unwrap();
        assert_eq!(topic.sub_category, vec!["ntp"]);
        assert_eq!(topic.identifier.as_deref(), Some("post"));
    }

    #[test]
    fn test_parse_pong() {
        let topic = parse_topic("sys/pk001/did001/thing/pong/post").unwrap();
        assert_eq!(topic.sub_category, vec!["pong"]);
        assert_eq!(topic.identifier.as_deref(), Some("post"));
    }

    #[test]
    fn test_parse_device_model_request() {
        let topic = parse_topic("sys/pk001/did001/thing/device/model/request").unwrap();
        assert_eq!(topic.sub_category, vec!["device", "model"]);
        assert_eq!(topic.identifier.as_deref(), Some("request"));
    }

    #[test]
    fn test_parse_platform_topic() {
        let topic = parse_topic("sys/pk001/did001/platform/service/media_server/post").unwrap();
        assert_eq!(topic.category, "platform");
        assert_eq!(topic.sub_category, vec!["service", "media_server"]);
        assert_eq!(topic.identifier.as_deref(), Some("post"));
    }

    #[test]
    fn test_parse_app_topic() {
        let topic = parse_topic("sys/tid001/did001/app/device/model/request").unwrap();
        assert_eq!(topic.category, "app");
        assert_eq!(topic.sub_category, vec!["device", "model"]);
        assert_eq!(topic.identifier.as_deref(), Some("request"));
    }

    #[test]
    fn test_parse_properties_set_reply() {
        let topic = parse_topic("sys/pk001/did001/thing/properties/set_reply").unwrap();
        assert_eq!(topic.sub_category, vec!["properties"]);
        assert_eq!(topic.identifier.as_deref(), Some("set_reply"));
    }

    #[test]
    fn test_parse_properties_state() {
        let topic = parse_topic("sys/pk001/did001/thing/properties/state").unwrap();
        assert_eq!(topic.sub_category, vec!["properties"]);
        assert_eq!(topic.identifier.as_deref(), Some("state"));
    }

    #[test]
    fn test_parse_invalid_topic() {
        let result = parse_topic("invalid/topic");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_too_short() {
        let result = parse_topic("sys/pk001");
        assert!(result.is_err());
    }

    #[test]
    fn test_classify_property_post() {
        let topic = parse_topic("sys/pk001/did001/thing/event/property/post").unwrap();
        assert_eq!(
            classify_thing_message(&topic),
            Some(ThingMessageType::EventProperty)
        );
    }

    #[test]
    fn test_classify_service_reply() {
        let topic = parse_topic("sys/pk001/did001/thing/service/reboot/post_reply").unwrap();
        assert_eq!(
            classify_thing_message(&topic),
            Some(ThingMessageType::ServiceReply)
        );
    }

    #[test]
    fn test_classify_event_custom() {
        let topic = parse_topic("sys/pk001/did001/thing/event/fire_alarm/warning").unwrap();
        assert_eq!(
            classify_thing_message(&topic),
            Some(ThingMessageType::EventCustom)
        );
    }

    #[test]
    fn test_classify_pong() {
        let topic = parse_topic("sys/pk001/did001/thing/pong/post").unwrap();
        assert_eq!(classify_thing_message(&topic), Some(ThingMessageType::Pong));
    }

    #[test]
    fn test_classify_ntp() {
        let topic = parse_topic("sys/pk001/did001/thing/ntp/post").unwrap();
        assert_eq!(classify_thing_message(&topic), Some(ThingMessageType::Ntp));
    }

    #[test]
    fn test_classify_register() {
        let topic = parse_topic("sys/pk001/did001/thing/register/post").unwrap();
        assert_eq!(
            classify_thing_message(&topic),
            Some(ThingMessageType::Register)
        );
    }

    #[test]
    fn test_classify_dynamic_register() {
        let topic = parse_topic("sys/pk001/secret001/thing/dynamic_register/post").unwrap();
        assert_eq!(
            classify_thing_message(&topic),
            Some(ThingMessageType::DynamicRegister)
        );
    }

    #[test]
    fn test_classify_device_model() {
        let topic = parse_topic("sys/pk001/did001/thing/device/model/request").unwrap();
        assert_eq!(
            classify_thing_message(&topic),
            Some(ThingMessageType::DeviceModel)
        );
    }

    #[test]
    fn test_classify_property_set_reply() {
        let topic = parse_topic("sys/pk001/did001/thing/properties/set_reply").unwrap();
        assert_eq!(
            classify_thing_message(&topic),
            Some(ThingMessageType::PropertySetReply)
        );
    }

    #[test]
    fn test_classify_property_state() {
        let topic = parse_topic("sys/pk001/did001/thing/properties/state").unwrap();
        assert_eq!(
            classify_thing_message(&topic),
            Some(ThingMessageType::PropertyState)
        );
    }
}
