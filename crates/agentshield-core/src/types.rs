//! 核心数据类型。各模块之间传递的统一语言。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 一次被拦截的工具调用，所有决策与审计都围绕它展开。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 事件唯一 id（uuid v4）
    pub id: String,
    /// 会话 id
    pub session_id: String,
    /// 来源客户端，如 "Codex CLI"
    pub client_name: String,
    /// 目标 MCP server 名
    pub server_name: String,
    /// 工具名，如 "fs.read_file"
    pub tool_name: String,
    /// 推断出的事件类型
    pub event_type: EventType,
    /// 目标资源：路径 / 命令 / SQL
    pub target: Option<String>,
    /// 原始参数
    pub arguments: serde_json::Value,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 事件类型。由 Proxy 在解析阶段根据工具名 / 参数推断。
///
/// 序列化采用点号形式（`file.read`、`shell.exec`），与策略文件、文档保持一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "file.read")]
    FileRead,
    #[serde(rename = "file.write")]
    FileWrite,
    #[serde(rename = "file.delete")]
    FileDelete,
    #[serde(rename = "file.rename")]
    FileRename,
    #[serde(rename = "shell.exec")]
    ShellExec,
    #[serde(rename = "db.query")]
    DbQuery,
    #[serde(rename = "network.request")]
    NetworkRequest,
    #[serde(rename = "mcp.tool_call")]
    McpToolCall,
    #[serde(rename = "other")]
    Other,
}

impl EventType {
    /// 是否属于写 / 删 / 执行这类有副作用的操作。
    pub fn is_mutating(self) -> bool {
        matches!(
            self,
            EventType::FileWrite
                | EventType::FileDelete
                | EventType::FileRename
                | EventType::ShellExec
                | EventType::DbQuery
        )
    }
}

/// 风险等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    /// 根据 0-100 的分数映射等级。
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=29 => RiskLevel::Low,
            30..=59 => RiskLevel::Medium,
            60..=79 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}

/// 决策动作。
///
/// 派生的 `Ord` 顺序刻意排成 `Allow < Log < Sandbox < Confirm < Block`，
/// 于是“取更严格者”就是 `a.max(b)`，决策合成无需写 if-else 链。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Log,
    Sandbox,
    Confirm,
    Block,
}

/// 风险评分结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// 0-100
    pub score: u8,
    pub level: RiskLevel,
    pub reasons: Vec<String>,
    pub recommended_action: Action,
}

/// 合成后的最终决策。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub action: Action,
    pub risk: RiskAssessment,
    /// 命中的策略规则名
    pub matched_rule: Option<String>,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_ord_takes_stricter() {
        // 取更严格者就是 max
        assert_eq!(Action::Allow.max(Action::Block), Action::Block);
        assert_eq!(Action::Log.max(Action::Confirm), Action::Confirm);
        assert_eq!(Action::Confirm.max(Action::Allow), Action::Confirm);
    }

    #[test]
    fn risk_level_boundaries() {
        assert_eq!(RiskLevel::from_score(29), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(30), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(59), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(60), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(79), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(80), RiskLevel::Critical);
    }

    #[test]
    fn mutating_classification() {
        assert!(EventType::FileDelete.is_mutating());
        assert!(!EventType::FileRead.is_mutating());
    }
}
