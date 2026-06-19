//! 把 risk + policy + rules 组装成一个决策器，并提供 JSONL 审计适配器。

use std::path::PathBuf;
use std::sync::Arc;

use agentshield_audit::{AuditRecord, JsonlSink, SqliteStore};
use agentshield_core::{Action, Decision, ToolCall};
use agentshield_policy::PolicyEngine;
use agentshield_proxy::{AuditSink, DecisionMaker};
use agentshield_risk::{Context, RiskEngine};
use agentshield_rules::RuleRegistry;

use crate::memory::{DecisionMemory, MemoryVerdict};

/// 应用级决策器：规则命中参与风险计分，再由策略合成最终动作。
/// 若用户此前对同一操作选过「始终允许 / 永久拉黑」，记忆优先于策略。
pub struct AppDecisionMaker {
    risk: RiskEngine,
    policy: PolicyEngine,
    rules: RuleRegistry,
    memory: Option<Arc<DecisionMemory>>,
}

impl AppDecisionMaker {
    /// 不带记忆（用于 demo / policy test）。
    pub fn new(policy: PolicyEngine) -> Self {
        AppDecisionMaker {
            risk: RiskEngine::new(),
            policy,
            rules: RuleRegistry::builtin(),
            memory: None,
        }
    }

    /// 带持久化决策记忆（用于 proxy 运行）。
    pub fn with_memory(policy: PolicyEngine, memory: Arc<DecisionMemory>) -> Self {
        AppDecisionMaker {
            risk: RiskEngine::new(),
            policy,
            rules: RuleRegistry::builtin(),
            memory: Some(memory),
        }
    }
}

impl DecisionMaker for AppDecisionMaker {
    fn decide(&self, call: &ToolCall) -> Decision {
        let hits = self.rules.evaluate_all(call);
        let ctx = Context::default();
        let risk = self.risk.assess(call, &ctx, &hits);

        // 记忆优先：用户确认过的「始终允许 / 永久拉黑」直接生效，不再走策略。
        if let Some(mem) = &self.memory {
            if let Some(verdict) = mem.lookup(call) {
                let (action, reason) = match verdict {
                    MemoryVerdict::Allow => (Action::Allow, "用户已设为始终允许"),
                    MemoryVerdict::Block => (Action::Block, "用户已永久拉黑"),
                };
                return Decision {
                    action,
                    risk,
                    matched_rule: Some(format!("memory:{}", action_tag(action))),
                    reason: reason.to_string(),
                };
            }
        }

        self.policy.decide(call, risk)
    }
}

fn action_tag(action: Action) -> &'static str {
    match action {
        Action::Allow => "allow",
        Action::Block => "block",
        _ => "other",
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
