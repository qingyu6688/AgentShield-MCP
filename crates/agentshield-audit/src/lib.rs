//! 审计层。记录工具调用、决策与确认结果。
//!
//! - JSONL 追加写（崩溃安全、易消费）：[`JsonlSink`]
//! - SQLite 结构化存储与查询：[`sqlite::SqliteStore`]
//! - 报告生成（JSON / Markdown / HTML）：[`report::Report`]
//!
//! 见 docs/design/08-audit.md。

pub mod error;
pub mod report;
pub mod sqlite;

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use agentshield_core::{redact, Decision, ToolCall};
use serde::{Deserialize, Serialize};

pub use error::AuditError;
pub use report::{Format, Report, ReportMeta};
pub use sqlite::{EventQuery, SqliteStore};

/// 一条审计记录（落盘前已脱敏）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: String,
    pub session_id: String,
    pub client_name: String,
    pub server_name: String,
    pub tool_name: String,
    pub event_type: String,
    pub target: Option<String>,
    pub arguments_json: serde_json::Value,
    pub result_json: Option<serde_json::Value>,
    pub risk_score: u8,
    pub risk_level: String,
    pub decision: String,
    pub reason: String,
    pub created_at: String,
}

impl AuditRecord {
    /// 由一次调用与决策构造记录，并对敏感字段脱敏。
    pub fn build(call: &ToolCall, decision: &Decision, result: Option<&serde_json::Value>) -> Self {
        let mut arguments = call.arguments.clone();
        redact::redact_json(&mut arguments);

        let result_json = result.cloned().map(|mut v| {
            redact::redact_json(&mut v);
            v
        });

        AuditRecord {
            id: call.id.clone(),
            session_id: call.session_id.clone(),
            client_name: call.client_name.clone(),
            server_name: call.server_name.clone(),
            tool_name: call.tool_name.clone(),
            event_type: format!("{:?}", call.event_type),
            target: call.target.as_ref().map(|t| redact::redact(t)),
            arguments_json: arguments,
            result_json,
            risk_score: decision.risk.score,
            risk_level: format!("{:?}", decision.risk.level),
            decision: format!("{:?}", decision.action),
            reason: decision.reason.clone(),
            created_at: call.created_at.to_rfc3339(),
        }
    }
}

/// JSONL 审计写入器。追加写，每条一行。
pub struct JsonlSink {
    path: PathBuf,
}

impl JsonlSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        JsonlSink { path: path.into() }
    }

    /// 写入一条记录。先脱敏（在 build 阶段）再落盘。
    pub fn record(
        &self,
        call: &ToolCall,
        decision: &Decision,
        result: Option<&serde_json::Value>,
    ) -> std::io::Result<()> {
        let rec = AuditRecord::build(call, decision, result);
        self.write_record(&rec)
    }

    /// 追加写一条已构建好的记录（供双写场景复用，避免重复构建）。
    pub fn write_record(&self, rec: &AuditRecord) -> std::io::Result<()> {
        let line = serde_json::to_string(rec)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{line}")?;
        file.flush()?;
        Ok(())
    }

    /// 读取全部记录（供报告 / 查询使用）。
    pub fn read_all(path: impl AsRef<Path>) -> std::io::Result<Vec<AuditRecord>> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut out = Vec::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            if let Ok(rec) = serde_json::from_str::<AuditRecord>(line) {
                out.push(rec);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentshield_core::{Action, EventType, RiskAssessment, RiskLevel};
    use chrono::Utc;

    fn sample() -> (ToolCall, Decision) {
        let call = ToolCall {
            id: "id1".into(),
            session_id: "s1".into(),
            client_name: "Codex".into(),
            server_name: "fs".into(),
            tool_name: "read".into(),
            event_type: EventType::FileRead,
            target: Some("./.env".into()),
            arguments: serde_json::json!({ "path": "./.env", "token": "abcdef123456" }),
            created_at: Utc::now(),
        };
        let decision = Decision {
            action: Action::Block,
            risk: RiskAssessment {
                score: 95,
                level: RiskLevel::Critical,
                reasons: vec!["读取环境变量文件".into()],
                recommended_action: Action::Block,
            },
            matched_rule: Some("file-env".into()),
            reason: "阻止读取环境变量文件".into(),
        };
        (call, decision)
    }

    #[test]
    fn record_is_redacted() {
        let (call, decision) = sample();
        let rec = AuditRecord::build(&call, &decision, None);
        assert_eq!(rec.arguments_json["token"], "***");
        assert_eq!(rec.decision, "Block");
    }

    #[test]
    fn write_then_read_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("agentshield-test-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let sink = JsonlSink::new(&path);
        let (call, decision) = sample();
        sink.record(&call, &decision, None).unwrap();
        let all = JsonlSink::read_all(&path).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "id1");
        let _ = std::fs::remove_file(&path);
    }
}
