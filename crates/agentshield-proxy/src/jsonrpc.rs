//! 最小化 JSON-RPC 2.0 消息类型，够用于 MCP 转发。

use serde::{Deserialize, Serialize};

/// 一条 JSON-RPC 消息（请求 / 响应 / 通知通用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcMessage {
    /// 构造一条被 AgentShield 阻止的错误响应。
    pub fn blocked(id: Option<serde_json::Value>, reason: &str) -> Self {
        JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id,
            method: None,
            params: None,
            result: None,
            error: Some(JsonRpcError {
                // 自定义错误码区间：-32000 ~ -32099 为实现保留
                code: -32010,
                message: format!("operation blocked by AgentShield policy: {reason}"),
                data: None,
            }),
        }
    }

    /// 是否为 tools/call 请求。
    pub fn is_tools_call(&self) -> bool {
        self.method.as_deref() == Some("tools/call")
    }
}
