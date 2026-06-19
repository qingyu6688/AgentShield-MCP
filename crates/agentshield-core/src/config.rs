//! 配置加载与校验。对应 `.agentshield/config.yaml`。

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

/// 主配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub servers: BTreeMap<String, ServerConfig>,
    #[serde(default)]
    pub approval: ApprovalConfig,
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub risk_weights: RiskWeights,
}

/// 单个 MCP server 的配置与权限。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// 权限等级 0-5，见 docs/policy.md
    #[serde(default = "default_trust")]
    pub trust_level: u8,
    #[serde(default)]
    pub allow_tools: Vec<String>,
    #[serde(default)]
    pub confirm_tools: Vec<String>,
    #[serde(default)]
    pub block_tools: Vec<String>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub blocked_paths: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// 确认相关配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalConfig {
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// 超时兜底动作：deny 更安全
    #[serde(default = "default_on_timeout")]
    pub on_timeout: String,
}

/// 审计相关配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_true")]
    pub sqlite: bool,
    #[serde(default = "default_true")]
    pub jsonl: bool,
}

/// 风险维度权重。缺省全 1.0。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskWeights {
    #[serde(default = "default_weight")]
    pub path: f32,
    #[serde(default = "default_weight")]
    pub command: f32,
    #[serde(default = "default_weight")]
    pub network: f32,
    #[serde(default = "default_weight")]
    pub database: f32,
    #[serde(default = "default_weight")]
    pub trust: f32,
    #[serde(default = "default_weight")]
    pub history: f32,
}

impl Config {
    /// 从 YAML 文件加载并校验。
    pub fn load(path: impl AsRef<Path>) -> Result<Config> {
        let text = std::fs::read_to_string(path)?;
        let cfg: Config =
            serde_yaml::from_str(&text).map_err(|e| CoreError::ConfigParse(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// 基本校验：版本、命令非空、权限等级范围。
    pub fn validate(&self) -> Result<()> {
        if self.version == 0 {
            return Err(CoreError::ConfigInvalid("version 不能为 0".into()));
        }
        for (name, s) in &self.servers {
            if s.command.trim().is_empty() {
                return Err(CoreError::ConfigInvalid(format!(
                    "server `{name}` 的 command 不能为空"
                )));
            }
            if s.trust_level > 5 {
                return Err(CoreError::ConfigInvalid(format!(
                    "server `{name}` 的 trust_level 必须在 0-5 之间，当前 {}",
                    s.trust_level
                )));
            }
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: 1,
            servers: BTreeMap::new(),
            approval: ApprovalConfig::default(),
            audit: AuditConfig::default(),
            risk_weights: RiskWeights::default(),
        }
    }
}

impl Default for ApprovalConfig {
    fn default() -> Self {
        ApprovalConfig {
            timeout_secs: default_timeout(),
            on_timeout: default_on_timeout(),
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        AuditConfig {
            sqlite: true,
            jsonl: true,
        }
    }
}

impl Default for RiskWeights {
    fn default() -> Self {
        RiskWeights {
            path: 1.0,
            command: 1.0,
            network: 1.0,
            database: 1.0,
            trust: 1.0,
            history: 1.0,
        }
    }
}

fn default_trust() -> u8 {
    2
}
fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    60
}
fn default_on_timeout() -> String {
    "deny".to_string()
}
fn default_weight() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let yaml = r#"
version: 1
servers:
  filesystem:
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "."]
    trust_level: 1
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        cfg.validate().unwrap();
        assert_eq!(cfg.servers["filesystem"].trust_level, 1);
    }

    #[test]
    fn rejects_bad_trust_level() {
        let yaml = r#"
version: 1
servers:
  x:
    command: foo
    trust_level: 9
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_empty_command() {
        let yaml = r#"
version: 1
servers:
  x:
    command: ""
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.validate().is_err());
    }
}
