use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Product function definition (function_info).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFunction {
    pub id: Option<i64>,
    #[serde(default)]
    pub module_id: Option<i64>,
    pub identifier: String,
    pub method: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub call_type: Option<String>,
    #[serde(default)]
    pub function_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub gmt_create: Option<DateTime<Utc>>,
    #[serde(default)]
    pub gmt_modified: Option<DateTime<Utc>>,
}

impl ProductFunction {
    pub fn new(identifier: String, method: String) -> Self {
        Self {
            id: None,
            module_id: None,
            identifier,
            method,
            name: None,
            call_type: None,
            function_type: None,
            description: None,
            gmt_create: None,
            gmt_modified: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_function_new() {
        let func = ProductFunction::new("reboot".to_string(), "thing.service.reboot".to_string());
        assert_eq!(func.identifier, "reboot");
        assert_eq!(func.method, "thing.service.reboot");
    }
}
