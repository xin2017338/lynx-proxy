use serde::{Deserialize, Serialize};

/// Modify request handler configuration
#[derive(Debug, Serialize, Deserialize, Default, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModifyResponseConfig {
    pub modify_headers: Option<std::collections::HashMap<String, String>>,
    pub modify_body: Option<String>,
    pub modify_status_code: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_modify_method_field_is_ignored() {
        let config: ModifyResponseConfig = serde_json::from_str(
            r#"{"modifyHeaders":null,"modifyBody":null,"modifyMethod":"GET","modifyStatusCode":201}"#,
        )
        .unwrap();

        assert_eq!(config.modify_status_code, Some(201));
    }
}
