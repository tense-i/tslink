use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Call type for service invocation.
///
/// Maps from Java: `CallType`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallType {
    /// Synchronous call — wait for device response
    Sync,
    /// Asynchronous call — fire and forget
    Async,
}

/// Device model type for multi-link routing.
///
/// Maps from Java: `DeviceModelType`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceModelType {
    Service,
    Events,
    Properties,
}

/// Data type descriptor for method fields.
///
/// Maps from Java: `MethodFieldDataType`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodFieldDataType {
    /// Type name (e.g., "int", "float", "text", "bool", "enum", "array", "struct")
    #[serde(rename = "type")]
    pub data_type: String,
    /// Type-specific specs (min, max, unit, etc.)
    #[serde(default)]
    pub specs: Option<serde_json::Value>,
}

/// Method field definition.
///
/// Maps from Java: `DeviceModel.MethodField`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodField {
    pub id: Option<i64>,
    pub name: String,
    pub identifier: String,
    #[serde(default)]
    pub data_type: Option<MethodFieldDataType>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub expands: Option<HashMap<String, serde_json::Value>>,
}

/// Function/service method definition.
///
/// Maps from Java: `DeviceModel.FunctionMethod`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMethod {
    pub identifier: String,
    pub method: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub call_type: Option<CallType>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub input_fields: Vec<MethodField>,
    #[serde(default)]
    pub output_fields: Vec<MethodField>,
    #[serde(default)]
    pub expands: Option<HashMap<String, serde_json::Value>>,
}

/// Complete device model definition (物模型).
///
/// Maps from Java: `DeviceModel`
/// Loaded from product/module/function tables with caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceModel {
    #[serde(default)]
    pub name: Option<String>,
    pub product_key: String,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub product_version: Option<String>,
    #[serde(default)]
    pub product_type: Option<String>,
    /// Property definitions
    #[serde(default)]
    pub properties: Vec<MethodField>,
    /// Service definitions (keyed by method name)
    #[serde(default)]
    pub services: HashMap<String, FunctionMethod>,
    /// Event definitions (keyed by identifier)
    #[serde(default)]
    pub events: HashMap<String, FunctionMethod>,
    /// Config definitions
    #[serde(default)]
    pub configs: Vec<MethodField>,
}

impl DeviceModel {
    /// Lookup a service by method name.
    pub fn get_service(&self, method: &str) -> Option<&FunctionMethod> {
        self.services.get(method)
    }

    /// Lookup an event by identifier.
    pub fn get_event(&self, identifier: &str) -> Option<&FunctionMethod> {
        self.events.get(identifier)
    }

    /// Check if a service is synchronous.
    pub fn is_sync_service(&self, method: &str) -> bool {
        self.services
            .get(method)
            .and_then(|s| s.call_type.as_ref())
            .map(|ct| *ct == CallType::Sync)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_model_get_service() {
        let mut services = HashMap::new();
        services.insert(
            "reboot".to_string(),
            FunctionMethod {
                identifier: "reboot".to_string(),
                method: "reboot".to_string(),
                name: Some("重启设备".to_string()),
                call_type: Some(CallType::Sync),
                desc: None,
                input_fields: vec![],
                output_fields: vec![],
                expands: None,
            },
        );

        let model = DeviceModel {
            name: Some("TestDevice".to_string()),
            product_key: "pk001".to_string(),
            device_id: None,
            product_version: Some("1.0".to_string()),
            product_type: None,
            properties: vec![],
            services,
            events: HashMap::new(),
            configs: vec![],
        };

        assert!(model.get_service("reboot").is_some());
        assert!(model.get_service("nonexistent").is_none());
        assert!(model.is_sync_service("reboot"));
    }

    #[test]
    fn test_call_type_serde() {
        let ct = CallType::Sync;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"SYNC\"");

        let async_ct: CallType = serde_json::from_str("\"ASYNC\"").unwrap();
        assert_eq!(async_ct, CallType::Async);
    }

    #[test]
    fn test_method_field_serde() {
        let field = MethodField {
            id: Some(1),
            name: "temperature".to_string(),
            identifier: "temp".to_string(),
            data_type: Some(MethodFieldDataType {
                data_type: "float".to_string(),
                specs: Some(serde_json::json!({"min": "-40", "max": "85", "unit": "°C"})),
            }),
            required: true,
            desc: Some("Temperature sensor".to_string()),
            expands: None,
        };

        let json = serde_json::to_string(&field).unwrap();
        let deserialized: MethodField = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.identifier, "temp");
        assert!(deserialized.required);
    }
}
