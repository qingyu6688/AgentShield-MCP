//! 把 risk + policy + rules 组装成一个决策器，并提供 JSONL 审计适配器。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use agentshield_audit::{AuditRecord, JsonlSink, SqliteStore};
use agentshield_core::{
    Action, Config, Decision, DecisionMemory, MemoryVerdict, RiskWeights, ToolCall,
};
use agentshield_policy::PolicyEngine;
use agentshield_proxy::{AuditSink, DecisionMaker};
use agentshield_risk::{Context, RiskEngine};
use agentshield_rules::RuleRegistry;

/// 应用级决策器：规则命中参与风险计分，再由策略合成最终动作。
/// 用户的确认记忆会参与最终动作，但不能绕过当前策略里的阻断决策。
pub struct AppDecisionMaker {
    risk: RiskEngine,
    policy: PolicyEngine,
    rules: RuleRegistry,
    memory: Option<Arc<DecisionMemory>>,
    server_trust: BTreeMap<String, u8>,
    weights: RiskWeights,
}

impl AppDecisionMaker {
    /// 不带记忆（用于 demo / policy test）。
    pub fn new(policy: PolicyEngine) -> Self {
        Self::from_config(policy, &Config::default(), None)
    }

    /// 根据配置构建决策器。
    pub fn from_config(
        policy: PolicyEngine,
        cfg: &Config,
        memory: Option<Arc<DecisionMemory>>,
    ) -> Self {
        AppDecisionMaker {
            risk: RiskEngine::new(),
            policy,
            rules: RuleRegistry::builtin(),
            memory,
            server_trust: cfg
                .servers
                .iter()
                .map(|(name, server)| (name.clone(), server.trust_level))
                .collect(),
            weights: cfg.risk_weights.clone(),
        }
    }

    /// 带持久化决策记忆（用于 proxy 运行）。
    pub fn with_memory(policy: PolicyEngine, cfg: &Config, memory: Arc<DecisionMemory>) -> Self {
        Self::from_config(policy, cfg, Some(memory))
    }
}

impl DecisionMaker for AppDecisionMaker {
    fn decide(&self, call: &ToolCall) -> Decision {
        let hits = self.rules.evaluate_all(call);
        let server_trust = self
            .server_trust
            .get(&call.server_name)
            .copied()
            .unwrap_or(2);
        let ctx = Context {
            server_trust,
            weights: &self.weights,
        };
        let risk = self.risk.assess(call, &ctx, &hits);

        if let Some(mem) = &self.memory {
            if let Some(verdict) = mem.lookup(call) {
                if verdict == MemoryVerdict::Block {
                    return Decision {
                        action: Action::Block,
                        risk,
                        matched_rule: Some("memory:block".to_string()),
                        reason: "用户已永久拉黑".to_string(),
                    };
                }

                let decision = self.policy.decide(call, risk);
                if decision.action == Action::Block {
                    return decision;
                }
                return Decision {
                    action: Action::Allow,
                    risk: decision.risk,
                    matched_rule: Some("memory:allow".to_string()),
                    reason: "用户已设为始终允许".to_string(),
                };
            }
        }

        self.policy.decide(call, risk)
    }
}

/// 双写审计：同时落 JSONL（崩溃安全）与 SQLite（可查询、出报告）。
pub struct DualAudit {
    jsonl: JsonlSink,
    sqlite: SqliteStore,
}

impl DualAudit {
    pub fn new(jsonl_path: PathBuf, db_path: PathBuf) -> anyhow::Result<Self> {
        let sqlite =
            SqliteStore::open(db_path).map_err(|e| anyhow::anyhow!("打开审计数据库失败：{e}"))?;
        Ok(DualAudit {
            jsonl: JsonlSink::new(jsonl_path),
            sqlite,
        })
    }
}

impl AuditSink for DualAudit {
    fn record(&self, call: &ToolCall, decision: &Decision, result: Option<&serde_json::Value>) {
        // 审计失败不应中断转发，仅告警到 stderr（stdout 是 MCP 通道，不能污染）。
        // 记录只构建一次，JSONL 与 SQLite 复用，保证两处内容一致（均已脱敏）。
        let rec = AuditRecord::build(call, decision, result);
        if let Err(e) = self.jsonl.write_record(&rec) {
            eprintln!("[AgentShield] JSONL 审计写入失败：{e}");
        }
        if let Err(e) = self.sqlite.insert(&rec) {
            eprintln!("[AgentShield] SQLite 审计写入失败：{e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentshield_core::{EventType, ServerConfig};
    use chrono::Utc;

    fn call(server: &str, tool: &str, event_type: EventType, target: &str) -> ToolCall {
        ToolCall {
            id: "t".into(),
            session_id: "s".into(),
            client_name: "test".into(),
            server_name: server.into(),
            tool_name: tool.into(),
            event_type,
            target: Some(target.into()),
            arguments: serde_json::Value::Null,
            created_at: Utc::now(),
        }
    }

    fn server(trust_level: u8) -> ServerConfig {
        ServerConfig {
            command: "mock".into(),
            args: vec![],
            env: BTreeMap::new(),
            url: None,
            trust_level,
            allow_tools: vec![],
            confirm_tools: vec![],
            block_tools: vec![],
            allowed_paths: vec![],
            blocked_paths: vec![],
            enabled: true,
        }
    }

    #[test]
    fn low_trust_server_increases_mutating_risk() {
        let policy =
            PolicyEngine::from_yaml("version: 1\ndefault_action: allow\nrules: []").unwrap();
        let mut cfg = Config::default();
        cfg.servers.insert("low".into(), server(1));

        let dm = AppDecisionMaker::from_config(policy, &cfg, None);
        let decision = dm.decide(&call("low", "exec", EventType::ShellExec, "echo ok"));

        assert_eq!(decision.risk.score, 60);
        assert_eq!(decision.action, Action::Confirm);
    }

    #[test]
    fn allow_memory_does_not_override_block_policy() {
        let policy = PolicyEngine::from_yaml(
            r#"
version: 1
default_action: allow
rules:
  - name: block-env-read
    match:
      type: file.read
      path:
        contains: [".env"]
    action: block
"#,
        )
        .unwrap();
        let path =
            std::env::temp_dir().join(format!("agentshield-memory-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let memory = Arc::new(DecisionMemory::load(&path));
        let c = call("fs", "read_file", EventType::FileRead, "./.env");
        memory.remember_allow(&c);

        let dm = AppDecisionMaker::with_memory(policy, &Config::default(), memory);
        let decision = dm.decide(&c);

        assert_eq!(decision.action, Action::Block);
        assert_eq!(decision.matched_rule.as_deref(), Some("block-env-read"));
        let _ = std::fs::remove_file(&path);
    }
}
