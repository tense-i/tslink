use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Function parameter definition for IoT product functions.
/// Maps to `iot_function_param` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParam {
    pub id: Option<i64>,
    pub product_id: i64,
    #[serde(default)]
    pub module_id: Option<i64>,
    pub function_id: i64,
    pub param_name: String,
    pub param_identifier: String,
    /// Parameter type: input, output, properties, sub
    pub param_type: String,
    /// Data type: boolean, int, float, double, string, array, enum, json
    pub data_type: String,
    /// Parameter specs/constraints as JSON string
    #[serde(default)]
    pub specs: Option<String>,
    /// Related param ID for nested types (when param_type = "sub")
    #[serde(default)]
    pub rel_param_id: Option<i64>,
    /// Required flag: 0 = optional, 1 = required
    #[serde(default)]
    pub required: i32,
    #[serde(default)]
    pub gmt_create: Option<DateTime<Utc>>,
    #[serde(default)]
    pub gmt_modified: Option<DateTime<Utc>>,
    #[serde(default)]
    pub gmt_create_by: Option<String>,
    #[serde(default)]
    pub gmt_modified_by: Option<String>,
}

impl FunctionParam {
    pub fn new(
        product_id: i64,
        function_id: i64,
        param_name: String,
        param_identifier: String,
        param_type: String,
        data_type: String,
    ) -> Self {
        Self {
            id: None,
            product_id,
            module_id: None,
            function_id,
            param_name,
            param_identifier,
            param_type,
            data_type,
            specs: None,
            rel_param_id: None,
            required: 0,
            gmt_create: None,
            gmt_modified: None,
            gmt_create_by: None,
            gmt_modified_by: None,
        }
    }

    pub fn with_specs(mut self, specs: String) -> Self {
        self.specs = Some(specs);
        self
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = if required { 1 } else { 0 };
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_param_new() {
        let param = FunctionParam::new(
            1,
            2,
            "temperature".to_string(),
            "temp".to_string(),
            "output".to_string(),
            "float".to_string(),
        );
        assert_eq!(param.product_id, 1);
        assert_eq!(param.function_id, 2);
        assert_eq!(param.param_name, "temperature");
        assert_eq!(param.param_identifier, "temp");
        assert_eq!(param.param_type, "output");
        assert_eq!(param.data_type, "float");
        assert_eq!(param.required, 0);
    }

    #[test]
    fn test_function_param_with_specs() {
        let param = FunctionParam::new(
            1,
            2,
            "level".to_string(),
            "level".to_string(),
            "input".to_string(),
            "int".to_string(),
        )
        .with_specs(r#"{"min":0,"max":100}"#.to_string())
        .with_required(true);

        assert_eq!(param.specs.as_deref(), Some(r#"{"min":0,"max":100}"#));
        assert_eq!(param.required, 1);
    }
}
