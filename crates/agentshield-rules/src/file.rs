//! 文件风险规则。匹配文件类事件的目标路径。

use agentshield_core::{Action, EventType, RiskLevel, ToolCall};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{Rule, RuleHit};

/// 基于路径正则的文件规则。
struct PathRule {
    name: &'static str,
    re: Lazy<Regex>,
    severity: RiskLevel,
    action: Action,
    reason: &'static str,
}

impl Rule for PathRule {
    fn name(&self) -> &'static str {
        self.name
    }
    fn category(&self) -> crate::Category {
        crate::Category::File
    }
    fn default_severity(&self) -> RiskLevel {
        self.severity
    }
    fn default_action(&self) -> Action {
        self.action
    }
    fn evaluate(&self, call: &ToolCall) -> Option<RuleHit> {
        let is_file_event = matches!(
            call.event_type,
            EventType::FileRead
                | EventType::FileWrite
                | EventType::FileDelete
                | EventType::FileRename
        );
        if !is_file_event {
            return None;
        }
        let path = call.target.as_deref()?;
        if !self.re.is_match(path) {
            return None;
        }
        // 写 / 删敏感文件比读更危险，额外加分
        let extra = if call.event_type.is_mutating() { 10 } else { 0 };
        Some(RuleHit {
            rule_name: self.name,
            score_delta: crate::severity_floor(self.severity) + extra,
            severity: self.severity,
            reason: self.reason.to_string(),
        })
    }
}

macro_rules! path_rule {
    ($name:literal, $pat:literal, $sev:expr, $act:expr, $reason:literal) => {
        Box::new(PathRule {
            name: $name,
            re: Lazy::new(|| Regex::new($pat).expect("内置 file 正则应当合法")),
            severity: $sev,
            action: $act,
            reason: $reason,
        })
    };
}

pub(crate) fn register(rules: &mut Vec<Box<dyn Rule>>) {
    use Action::*;
    use RiskLevel::*;
    rules.push(path_rule!(
        "file-env",
        r"(^|[/\\])\.env(\.[A-Za-z0-9_]+)?$",
        Critical,
        Block,
        "访问环境变量文件，可能泄露密钥"
    ));
    rules.push(path_rule!(
        "file-ssh-key",
        r"(^|[/\\])(id_rsa|id_ed25519)$",
        Critical,
        Block,
        "访问 SSH 私钥"
    ));
    rules.push(path_rule!(
        "file-pem",
        r"\.pem$",
        Critical,
        Block,
        "访问证书 / 私钥文件"
    ));
    rules.push(path_rule!(
        "file-compose",
        r"(^|[/\\])docker-compose\.ya?ml$",
        High,
        Confirm,
        "访问 Docker 编排文件"
    ));
    rules.push(path_rule!(
        "file-nginx",
        r"(^|[/\\])nginx\.conf$",
        High,
        Confirm,
        "访问 Nginx 配置"
    ));
    rules.push(path_rule!(
        "file-manifest",
        r"(^|[/\\])(package\.json|pom\.xml)$",
        Medium,
        Log,
        "访问项目清单文件"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuleRegistry;
    use chrono::Utc;

    fn file_call(path: &str, et: EventType) -> ToolCall {
        ToolCall {
            id: "t".into(),
            session_id: "s".into(),
            client_name: "test".into(),
            server_name: "fs".into(),
            tool_name: "read".into(),
            event_type: et,
            target: Some(path.into()),
            arguments: serde_json::Value::Null,
            created_at: Utc::now(),
        }
    }

    fn hits(reg: &RuleRegistry, path: &str, et: EventType, rule: &str) -> bool {
        reg.evaluate_all(&file_call(path, et))
            .iter()
            .any(|h| h.rule_name == rule)
    }

    #[test]
    fn env_hits_but_similar_does_not() {
        let reg = RuleRegistry::builtin();
        assert!(hits(&reg, "./.env", EventType::FileRead, "file-env"));
        assert!(hits(
            &reg,
            "/app/.env.production",
            EventType::FileRead,
            "file-env"
        ));
        // 不能误伤 environment.ts
        assert!(!hits(
            &reg,
            "./environment.ts",
            EventType::FileRead,
            "file-env"
        ));
    }

    #[test]
    fn write_secret_scores_higher_than_read() {
        let reg = RuleRegistry::builtin();
        let read = reg.evaluate_all(&file_call("./.env", EventType::FileRead));
        let write = reg.evaluate_all(&file_call("./.env", EventType::FileWrite));
        let rs = read
            .iter()
            .find(|h| h.rule_name == "file-env")
            .unwrap()
            .score_delta;
        let ws = write
            .iter()
            .find(|h| h.rule_name == "file-env")
            .unwrap()
            .score_delta;
        assert!(ws > rs);
    }
}
