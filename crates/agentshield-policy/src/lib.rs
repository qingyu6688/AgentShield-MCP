//! 策略引擎。加载 YAML 策略，匹配规则，并与风险评分合成最终决策。

pub mod model;

use std::path::Path;

use agentshield_core::{Action, Decision, RiskAssessment, ToolCall};
use regex::Regex;

pub use model::{Match, Policy, Rule, StringMatch};

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("策略文件读取失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("策略解析失败: {0}")]
    Parse(String),
    #[error("规则 `{rule}` 的正则非法: {msg}")]
    BadRegex { rule: String, msg: String },
}

/// 策略引擎。
pub struct PolicyEngine {
    policy: Policy,
}

impl PolicyEngine {
    /// 从 YAML 文本构建，预编译并校验所有正则。
    pub fn from_yaml(text: &str) -> Result<Self, PolicyError> {
        let policy: Policy =
            serde_yaml::from_str(text).map_err(|e| PolicyError::Parse(e.to_string()))?;
        // 预校验正则
        for rule in &policy.rules {
            for sm in [
                &rule.match_.tool,
                &rule.match_.server,
                &rule.match_.path,
                &rule.match_.command,
                &rule.match_.sql,
            ]
            .into_iter()
            .flatten()
            {
                if let Some(re) = &sm.regex {
                    Regex::new(re).map_err(|e| PolicyError::BadRegex {
                        rule: rule.name.clone(),
                        msg: e.to_string(),
                    })?;
                }
            }
        }
        Ok(PolicyEngine { policy })
    }

    /// 从文件加载。
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PolicyError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_yaml(&text)
    }

    /// 找到第一条命中的启用规则。
    pub fn match_rule(&self, call: &ToolCall) -> Option<&Rule> {
        self.policy
            .rules
            .iter()
            .find(|r| r.enabled && rule_matches(&r.match_, call))
    }

    /// 与风险评分合成最终决策。取 规则动作 / 风险建议动作 中更严格的一个；
    /// 若命中规则带 `override: true`，则强制使用规则动作。
    pub fn decide(&self, call: &ToolCall, risk: RiskAssessment) -> Decision {
        let hit = self.match_rule(call);
        let rule_action = hit.map(|r| r.action).unwrap_or(self.policy.default_action);

        if let Some(r) = hit {
            if r.override_ {
                return Decision {
                    action: r.action,
                    risk,
                    matched_rule: Some(r.name.clone()),
                    reason: if r.description.is_empty() {
                        format!("命中规则 {}", r.name)
                    } else {
                        r.description.clone()
                    },
                };
            }
        }

        // 利用 Action 的 Ord：Allow < Log < Sandbox < Confirm < Block
        let action = rule_action.max(risk.recommended_action);
        let reason = build_reason(hit, &risk, action);
        Decision {
            action,
            risk,
            matched_rule: hit.map(|r| r.name.clone()),
            reason,
        }
    }
}

fn build_reason(hit: Option<&Rule>, risk: &RiskAssessment, action: Action) -> String {
    match hit {
        Some(r) if !r.description.is_empty() => r.description.clone(),
        Some(r) => format!("命中规则 {}", r.name),
        None => {
            if let Some(first) = risk.reasons.first() {
                first.clone()
            } else {
                format!("默认动作 {:?}", action)
            }
        }
    }
}

/// 判断一个 Match 是否命中（各字段 AND）。
fn rule_matches(m: &Match, call: &ToolCall) -> bool {
    if let Some(t) = m.type_ {
        if t != call.event_type {
            return false;
        }
    }
    if let Some(sm) = &m.tool {
        if !string_matches(sm, &call.tool_name) {
            return false;
        }
    }
    if let Some(sm) = &m.server {
        if !string_matches(sm, &call.server_name) {
            return false;
        }
    }
    // path / command / sql 都作用在 target 上
    for sm in [&m.path, &m.command, &m.sql].into_iter().flatten() {
        let target = call.target.as_deref().unwrap_or("");
        if !string_matches(sm, target) {
            return false;
        }
    }
    true
}

/// 一个 StringMatch 内多种方式是 OR：命中任一即可。
fn string_matches(sm: &StringMatch, value: &str) -> bool {
    if let Some(eq) = &sm.equals {
        if value == eq {
            return true;
        }
    }
    if sm.contains.iter().any(|c| value.contains(c)) {
        return true;
    }
    if sm.in_.iter().any(|c| value == c) {
        return true;
    }
    if let Some(re) = &sm.regex {
        // 已在构建时校验过，这里失败按不命中处理
        if let Ok(re) = Regex::new(re) {
            if re.is_match(value) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentshield_core::{EventType, RiskLevel};
    use chrono::Utc;

    const POLICY: &str = r#"
version: 1
default_action: allow
rules:
  - name: block-env-read
    description: 阻止读取环境变量文件
    match:
      type: file.read
      path:
        contains: [".env", "id_rsa"]
    action: block
    severity: critical
  - name: confirm-rm
    match:
      type: shell.exec
      command:
        regex: "rm\\s+-rf"
    action: confirm
"#;

    fn call(et: EventType, target: &str) -> ToolCall {
        ToolCall {
            id: "t".into(),
            session_id: "s".into(),
            client_name: "c".into(),
            server_name: "srv".into(),
            tool_name: "x".into(),
            event_type: et,
            target: Some(target.into()),
            arguments: serde_json::Value::Null,
            created_at: Utc::now(),
        }
    }

    fn low_risk() -> RiskAssessment {
        RiskAssessment {
            score: 10,
            level: RiskLevel::Low,
            reasons: vec!["x".into()],
            recommended_action: Action::Allow,
        }
    }

    #[test]
    fn env_read_blocked_even_if_risk_low() {
        let eng = PolicyEngine::from_yaml(POLICY).unwrap();
        let d = eng.decide(&call(EventType::FileRead, "./.env"), low_risk());
        assert_eq!(d.action, Action::Block);
        assert_eq!(d.matched_rule.as_deref(), Some("block-env-read"));
    }

    #[test]
    fn no_match_uses_default() {
        let eng = PolicyEngine::from_yaml(POLICY).unwrap();
        let d = eng.decide(&call(EventType::FileRead, "./src/main.rs"), low_risk());
        assert_eq!(d.action, Action::Allow);
        assert!(d.matched_rule.is_none());
    }

    #[test]
    fn takes_stricter_of_rule_and_risk() {
        let eng = PolicyEngine::from_yaml(POLICY).unwrap();
        // 规则给 confirm，风险建议 block → 取 block
        let mut risk = low_risk();
        risk.recommended_action = Action::Block;
        let d = eng.decide(&call(EventType::ShellExec, "rm -rf dist"), risk);
        assert_eq!(d.action, Action::Block);
    }
}
