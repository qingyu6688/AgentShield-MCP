//! 数据库风险规则。匹配 `DbQuery` 事件的 SQL 文本。

use agentshield_core::{Action, EventType, RiskLevel, ToolCall};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{Rule, RuleHit};

/// 简单 SQL 规则：正则匹配，可选“无 WHERE”约束。
struct SqlRule {
    name: &'static str,
    re: Lazy<Regex>,
    /// 为 true 时，仅当 SQL 中不含 WHERE 才命中
    require_no_where: bool,
    severity: RiskLevel,
    action: Action,
    reason: &'static str,
}

static WHERE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\bwhere\b").unwrap());

impl Rule for SqlRule {
    fn name(&self) -> &'static str {
        self.name
    }
    fn category(&self) -> crate::Category {
        crate::Category::Database
    }
    fn default_severity(&self) -> RiskLevel {
        self.severity
    }
    fn default_action(&self) -> Action {
        self.action
    }
    fn evaluate(&self, call: &ToolCall) -> Option<RuleHit> {
        if call.event_type != EventType::DbQuery {
            return None;
        }
        let sql = call.target.as_deref()?;
        if !self.re.is_match(sql) {
            return None;
        }
        if self.require_no_where && WHERE_RE.is_match(sql) {
            return None; // 有 WHERE，不算高危
        }
        Some(RuleHit {
            rule_name: self.name,
            score_delta: crate::severity_floor(self.severity),
            severity: self.severity,
            reason: self.reason.to_string(),
        })
    }
}

macro_rules! sql_rule {
    ($name:literal, $pat:literal, $no_where:literal, $sev:expr, $act:expr, $reason:literal) => {
        Box::new(SqlRule {
            name: $name,
            re: Lazy::new(|| Regex::new($pat).expect("内置 sql 正则应当合法")),
            require_no_where: $no_where,
            severity: $sev,
            action: $act,
            reason: $reason,
        })
    };
}

pub(crate) fn register(rules: &mut Vec<Box<dyn Rule>>) {
    use Action::*;
    use RiskLevel::*;
    rules.push(sql_rule!(
        "db-drop-database",
        r"(?i)\bdrop\s+database\b",
        false,
        Critical,
        Block,
        "删除整个数据库，不可逆"
    ));
    rules.push(sql_rule!(
        "db-drop-table",
        r"(?i)\bdrop\s+table\b",
        false,
        Critical,
        Confirm,
        "删除数据表，不可逆"
    ));
    rules.push(sql_rule!(
        "db-truncate",
        r"(?i)\btruncate\s+table\b",
        false,
        Critical,
        Confirm,
        "清空数据表"
    ));
    rules.push(sql_rule!(
        "db-drop-column",
        r"(?i)\balter\s+table\b.*\bdrop\s+column\b",
        false,
        High,
        Confirm,
        "删除数据表字段"
    ));
    rules.push(sql_rule!(
        "db-delete-no-where",
        r"(?i)\bdelete\s+from\b",
        true,
        High,
        Confirm,
        "无 WHERE 条件的 DELETE，将删除全表数据"
    ));
    rules.push(sql_rule!(
        "db-update-no-where",
        r"(?i)\bupdate\b.+\bset\b",
        true,
        High,
        Confirm,
        "无 WHERE 条件的 UPDATE，将更新全表数据"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuleRegistry;
    use chrono::Utc;

    fn sql_call(sql: &str) -> ToolCall {
        ToolCall {
            id: "t".into(),
            session_id: "s".into(),
            client_name: "test".into(),
            server_name: "db".into(),
            tool_name: "query".into(),
            event_type: EventType::DbQuery,
            target: Some(sql.into()),
            arguments: serde_json::Value::Null,
            created_at: Utc::now(),
        }
    }

    fn hit(reg: &RuleRegistry, sql: &str, rule: &str) -> bool {
        reg.evaluate_all(&sql_call(sql))
            .iter()
            .any(|h| h.rule_name == rule)
    }

    #[test]
    fn drop_table_hits() {
        let reg = RuleRegistry::builtin();
        assert!(hit(&reg, "DROP TABLE users", "db-drop-table"));
    }

    #[test]
    fn delete_without_where_hits_but_with_where_does_not() {
        let reg = RuleRegistry::builtin();
        assert!(hit(&reg, "DELETE FROM users", "db-delete-no-where"));
        assert!(!hit(
            &reg,
            "DELETE FROM users WHERE id = 1",
            "db-delete-no-where"
        ));
    }

    #[test]
    fn update_with_where_is_safe() {
        let reg = RuleRegistry::builtin();
        assert!(!hit(
            &reg,
            "UPDATE users SET role = 'admin' WHERE id = 1",
            "db-update-no-where"
        ));
    }
}
