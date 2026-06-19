//! AgentShield MCP 代理。
//!
//! 提供 JSON-RPC 消息类型、工具调用分类（[`classify`]）以及 stdio 传输的
//! 拦截转发主循环（[`gateway::run_stdio`]）。HTTP 传输见 docs/design/02-proxy.md。

pub mod classify;
pub mod gateway;
pub mod jsonrpc;
pub mod transport;

use agentshield_core::{Decision, ToolCall};

pub use classify::classify;
pub use gateway::{run_stdio, ProxyContext};
pub use jsonrpc::{JsonRpcError, JsonRpcMessage};

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("JSON-RPC 编解码失败: {0}")]
    Codec(String),
    #[error("上游 MCP server 错误: {0}")]
    Upstream(String),
    #[error("传输错误: {0}")]
    Transport(String),
}

/// 决策器：由上层（cli）注入，组合 risk + policy。
///
/// MVP 暂用同步签名；接入异步传输时改为 async（见设计文档）。
pub trait DecisionMaker: Send + Sync {
    fn decide(&self, call: &ToolCall) -> Decision;
}

/// 确认器：决策为 confirm 时调用。
pub trait Approver: Send + Sync {
    fn approve(&self, call: &ToolCall, decision: &Decision) -> ApprovalResult;
}

/// 审计接收器。
pub trait AuditSink: Send + Sync {
    fn record(&self, call: &ToolCall, decision: &Decision, result: Option<&serde_json::Value>);
}

/// 用户确认结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalResult {
    AllowOnce,
    AllowAlways,
    Deny,
    BlockForever,
    TimedOut,
}
