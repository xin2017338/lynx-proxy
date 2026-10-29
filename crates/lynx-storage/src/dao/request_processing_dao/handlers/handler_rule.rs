use serde::{Deserialize, Serialize};

use super::{
    BlockHandlerConfig, DelayHandlerConfig, DelayType, HtmlScriptInjectorConfig, LocalFileConfig,
    ModifyRequestConfig, ThrottleHandlerConfig, ThrottlePreset,
    modify_response_handler::ModifyResponseConfig, proxy_forward_handler::ProxyForwardConfig,
};

#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum HandlerRuleType {
    Block(BlockHandlerConfig),
    ModifyRequest(ModifyRequestConfig),
    LocalFile(LocalFileConfig),
    ModifyResponse(ModifyResponseConfig),
    ProxyForward(ProxyForwardConfig),
    HtmlScriptInjector(HtmlScriptInjectorConfig),
    Delay(DelayHandlerConfig),
    Throttle(ThrottleHandlerConfig),
}

/// Handler rule configuration
#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandlerRule {
    pub id: Option<i32>,
    pub handler_type: HandlerRuleType,
    pub execution_order: i32,
    pub enabled: bool,
}

impl HandlerRule {
    pub fn block_handler(status_code: Option<u16>, reason: Option<String>) -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::Block(BlockHandlerConfig {
                status_code,
                reason,
            }),
            execution_order: 100,
            enabled: true,
        }
    }

    pub fn local_file_handler(
        file_path: String,
        content_type: Option<String>,
        status_code: Option<u16>,
    ) -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::LocalFile(LocalFileConfig {
                file_path,
                content_type,
                status_code,
            }),
            execution_order: 50,
            enabled: true,
        }
    }

    pub fn modify_request_handler(
        modify_headers: Option<std::collections::HashMap<String, String>>,
        modify_body: Option<String>,
        modify_method: Option<String>,
        modify_url: Option<String>,
    ) -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::ModifyRequest(ModifyRequestConfig {
                modify_headers,
                modify_body,
                modify_method,
                modify_url,
            }),
            execution_order: 20,
            enabled: true,
        }
    }

    pub fn modify_response_handler(
        modify_headers: Option<std::collections::HashMap<String, String>>,
        modify_body: Option<String>,
        modify_status_code: Option<u16>,
    ) -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::ModifyResponse(ModifyResponseConfig {
                modify_headers,
                modify_body,
                modify_status_code,
            }),
            execution_order: 80,
            enabled: true,
        }
    }

    pub fn proxy_forward_handler(
        target_scheme: Option<String>,
        target_authority: Option<String>,
        target_path: Option<String>,
    ) -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::ProxyForward(ProxyForwardConfig::new(
                target_scheme,
                target_authority,
                target_path,
            )),
            execution_order: 10,
            enabled: true,
        }
    }

    pub fn html_script_injector_handler(
        content: Option<String>,
        injection_position: Option<String>,
    ) -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::HtmlScriptInjector(HtmlScriptInjectorConfig {
                content,
                injection_position: injection_position.or(Some("body-end".to_string())),
            }),
            execution_order: 85,
            enabled: true,
        }
    }

    pub fn delay_handler(delay_ms: u64, variance_ms: Option<u64>, delay_type: DelayType) -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::Delay(DelayHandlerConfig {
                delay_ms,
                variance_ms,
                delay_type,
            }),
            execution_order: 5, // Execute early to delay before processing
            enabled: true,
        }
    }

    pub fn throttle_handler(preset: ThrottlePreset) -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::Throttle(ThrottleHandlerConfig {
                preset,
                download_kbps: None,
                upload_kbps: None,
                latency_ms: None,
            }),
            execution_order: 5,
            enabled: true,
        }
    }
}

impl Default for HandlerRule {
    fn default() -> Self {
        Self {
            id: None,
            handler_type: HandlerRuleType::Block(BlockHandlerConfig {
                status_code: Some(403),
                reason: Some("block".to_string()),
            }),
            execution_order: 0,
            enabled: true,
        }
    }
}
