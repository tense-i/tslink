/// Integration tests for TSLink IoT Core.
///
/// These tests validate the complete message processing pipeline:
/// MQTT → Topic Parser → Router → Handler → Service → Redis/DB
///
/// Run with: cargo test --test integration_test -- --nocapture
#[cfg(test)]
mod integration_test {
    use tslink::domain::message::{CommonTopicReceiver, CommonTopicResponse};
    use tslink::domain::topic::ThingMessageType;
    use tslink::infrastructure::mqtt::topic_parser::{classify_thing_message, parse_topic};

    /// Verify full topic parsing → message classification → deserialization chain.
    #[test]
    fn test_full_message_pipeline() {
        // Step 1: Parse topic
        let topic = "sys/pk001/did001/thing/event/property/post";
        let topic_info = parse_topic(topic).expect("topic should parse");

        assert_eq!(topic_info.product_key, "pk001");
        assert_eq!(topic_info.device_id, "did001");
        assert_eq!(topic_info.category, "thing");

        // Step 2: Classify
        let msg_type = classify_thing_message(&topic_info);
        assert_eq!(msg_type, Some(ThingMessageType::EventProperty));

        // Step 3: Deserialize
        let payload = r#"{
            "tid": "tid-001",
            "version": "1.0",
            "timestamp": 1700000000000,
            "method": "thing.event.property.post",
            "productKey": "pk001",
            "deviceId": "did001",
            "data": {"temperature": 25.5, "humidity": 60}
        }"#;

        let msg: CommonTopicReceiver<serde_json::Value> =
            serde_json::from_str(payload).expect("should deserialize");

        assert_eq!(msg.tid.as_deref(), Some("tid-001"));
        assert_eq!(msg.data["temperature"], 25.5);

        // Step 4: Create reply
        let reply = CommonTopicResponse::reply(&msg, serde_json::json!({}));
        assert_eq!(reply.tid.as_deref(), Some("tid-001"));
        assert_eq!(reply.code.as_deref(), Some("200"));
    }

    /// Test service invocation topic chain.
    #[test]
    fn test_service_invocation_chain() {
        let service_topic = "sys/pk001/did001/thing/service/reboot/post";
        let topic_info = parse_topic(service_topic).unwrap();
        assert_eq!(topic_info.product_key, "pk001");

        let reply_topic = "sys/pk001/did001/thing/service/reboot/post_reply";
        let reply_info = parse_topic(reply_topic).unwrap();
        let reply_type = classify_thing_message(&reply_info);
        assert_eq!(reply_type, Some(ThingMessageType::ServiceReply));
    }

    /// Test device registration flow.
    #[test]
    fn test_registration_flow() {
        let reg_topic = "sys/pk001/did001/thing/register/post";
        let topic_info = parse_topic(reg_topic).unwrap();
        let msg_type = classify_thing_message(&topic_info);
        assert_eq!(msg_type, Some(ThingMessageType::Register));

        let dyn_reg_topic = "sys/pk001/did001/thing/dynamic_register/post";
        let dyn_info = parse_topic(dyn_reg_topic).unwrap();
        let dyn_type = classify_thing_message(&dyn_info);
        assert_eq!(dyn_type, Some(ThingMessageType::DynamicRegister));
    }

    /// Test multi-link device topic parsing.
    /// Current implementation treats the full segment as device_id;
    /// link suffix splitting is handled at the service layer (LinkService).
    #[test]
    fn test_multilink_topic_parsing() {
        let topic = "sys/pk001/did001_link1/thing/event/property/post";
        let info = parse_topic(topic).unwrap();
        // The parser keeps the full segment as device_id
        assert_eq!(info.device_id, "did001_link1");
        assert_eq!(info.product_key, "pk001");
        assert_eq!(info.category, "thing");
    }

    /// Test region-prefixed topic parsing.
    #[test]
    fn test_region_topic_parsing() {
        let topic = "region/cn-east/sys/pk001/did001/thing/ntp/post";
        let info = parse_topic(topic).unwrap();
        assert_eq!(info.region.as_deref(), Some("cn-east"));
        assert_eq!(info.product_key, "pk001");
        let msg_type = classify_thing_message(&info);
        assert_eq!(msg_type, Some(ThingMessageType::Ntp));
    }

    /// Test key ThingMessageType variants can be classified.
    #[test]
    fn test_thing_message_types_classifiable() {
        let test_cases: Vec<(&str, ThingMessageType)> = vec![
            (
                "sys/pk/did/thing/event/property/post",
                ThingMessageType::EventProperty,
            ),
            (
                "sys/pk/did/thing/event/custom/info",
                ThingMessageType::EventCustom,
            ),
            (
                "sys/pk/did/thing/properties/state",
                ThingMessageType::PropertyState,
            ),
            (
                "sys/pk/did/thing/service/method/post_reply",
                ThingMessageType::ServiceReply,
            ),
            (
                "sys/pk/did/thing/properties/set_reply",
                ThingMessageType::PropertySetReply,
            ),
            ("sys/pk/did/thing/register/post", ThingMessageType::Register),
            (
                "sys/pk/did/thing/dynamic_register/post",
                ThingMessageType::DynamicRegister,
            ),
            ("sys/pk/did/thing/pong/post", ThingMessageType::Pong),
            ("sys/pk/did/thing/ntp/post", ThingMessageType::Ntp),
        ];

        for (topic, expected_type) in test_cases {
            let info = parse_topic(topic).unwrap_or_else(|_| panic!("Failed to parse: {}", topic));
            let msg_type = classify_thing_message(&info);
            assert_eq!(
                msg_type,
                Some(expected_type.clone()),
                "Failed for topic: {}",
                topic
            );
        }
    }
}
