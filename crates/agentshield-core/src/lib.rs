//! AgentShield MCP 核心库。
//!
//! 定义跨模块共享的类型、配置、错误与脱敏能力，不依赖任何业务 crate。

pub mod config;
pub mod error;
pub mod ids;
pub mod redact;
pub mod types;

pub use config::{ApprovalConfig, AuditConfig, Config, RiskWeights, ServerConfig};
pub use error::CoreError;
pub use types::{Action, Decision, EventType, RiskAssessment, RiskLevel, ToolCall};
