//! 策略数据模型，对应 `policy.yaml`。

use agentshield_core::{Action, EventType, RiskLevel};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Policy {
    pub version: u32,
    #[serde(default = "default_allow")]
    pub default_action: Action,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(rename = "match")]
    pub match_: Match,
    pub action: Action,
    #[serde(default)]
    pub severity: Option<RiskLevel>,
    #[serde(default, rename = "override")]
    pub override_: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Match {
    #[serde(rename = "type")]
    pub type_: Option<EventType>,
    pub tool: Option<StringMatch>,
    pub server: Option<StringMatch>,
    pub path: Option<StringMatch>,
    pub command: Option<StringMatch>,
    pub sql: Option<StringMatch>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StringMatch {
    #[serde(default)]
    pub equals: Option<String>,
    #[serde(default)]
    pub contains: Vec<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default, rename = "in")]
    pub in_: Vec<String>,
}

fn default_allow() -> Action {
    Action::Allow
}
fn default_true() -> bool {
    true
}
