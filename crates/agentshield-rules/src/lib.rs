//! 内置风险规则库。
//!
//! 集中维护 shell / file / database 三类规则，供 risk 与 policy 引擎使用。
//! 通过 [`Rule`] trait 支持第三方扩展。

mod database;
mod file;
mod shell;

use agentshield_core::{Action, RiskLevel, ToolCall};

/// 规则分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Shell,
    File,
    Database,
}

/// 规则命中的结果。
#[derive(Debug, Clone)]
pub struct RuleHit {
    pub rule_name: &'static str,
    /// 对风险分的贡献
    pub score_delta: i16,
    pub severity: RiskLevel,
    /// 中文原因，进入 reasons / 报告
    pub reason: String,
}

/// 风险规则统一接口。
pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn category(&self) -> Category;
    fn default_severity(&self) -> RiskLevel;
    fn default_action(&self) -> Action;
    /// 命中返回 `Some`。
    fn evaluate(&self, call: &ToolCall) -> Option<RuleHit>;
}

/// 规则注册表。
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
}

impl RuleRegistry {
    /// 装载全部内置规则。
    pub fn builtin() -> Self {
        let mut rules: Vec<Box<dyn Rule>> = Vec::new();
        shell::register(&mut rules);
        file::register(&mut rules);
        database::register(&mut rules);
        RuleRegistry { rules }
    }

    /// 注册第三方规则。
    pub fn register(&mut self, rule: Box<dyn Rule>) {
        self.rules.push(rule);
    }

    /// 对一次调用评估所有规则，返回全部命中。
    pub fn evaluate_all(&self, call: &ToolCall) -> Vec<RuleHit> {
        self.rules.iter().filter_map(|r| r.evaluate(call)).collect()
    }

    /// 内置规则数量。
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

/// 把严重级别映射到分数下界，规则命中时用于抬分。
pub fn severity_floor(level: RiskLevel) -> i16 {
    match level {
        RiskLevel::Low => 10,
        RiskLevel::Medium => 30,
        RiskLevel::High => 60,
        RiskLevel::Critical => 80,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_not_empty() {
        let reg = RuleRegistry::builtin();
        assert!(reg.len() >= 15, "内置规则数量应覆盖三类常见风险");
    }
}
