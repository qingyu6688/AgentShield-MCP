# 设计 · agentshield-core

公共词汇表。定义所有模块共享的类型、配置、错误和脱敏能力。**不依赖任何业务 crate**，只依赖 `serde`、`thiserror`、`uuid`、`chrono` 等基础库。

## 职责

- 定义跨模块流转的核心类型（`ToolCall`、`Decision`、`RiskAssessment` 等）。
- 加载与校验配置文件（`config.yaml`）。
- 统一错误类型。
- 敏感字段脱敏（Token、密码、密钥）。

## 模块结构

```text
agentshield-core/src/
├── lib.rs
├── types.rs       # ToolCall / Decision / Action / EventType / RiskLevel ...
├── config.rs      # Config、ServerConfig、加载与校验
├── error.rs       # CoreError（thiserror）
├── redact.rs      # 脱敏器
└── ids.rs         # 事件 id / 会话 id 生成
```

## 核心类型

```rust
use serde::{Deserialize, Serialize};

/// 一次被拦截的工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub session_id: String,
    pub client_name: String,
    pub server_name: String,
    pub tool_name: String,
    pub event_type: EventType,
    pub target: Option<String>,
    pub arguments: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    FileRead, FileWrite, FileDelete, FileRename,
    ShellExec, DbQuery, NetworkRequest, McpToolCall, Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub score: u8,                  // 0-100
    pub level: RiskLevel,
    pub reasons: Vec<String>,
    pub recommended_action: Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel { Low, Medium, High, Critical }

/// 注意 Ord 的顺序用于“取更严格者”
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action { Allow, Log, Sandbox, Confirm, Block }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub action: Action,
    pub risk: RiskAssessment,
    pub matched_rule: Option<String>,
    pub reason: String,
}
```

> `Action` 的 `Ord` 派生顺序刻意排成 `Allow < Log < Sandbox < Confirm < Block`，于是“取更严格者”就是 `a.max(b)`，决策合成不用写 if-else 链。

## 配置

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub servers: std::collections::BTreeMap<String, ServerConfig>,
    #[serde(default)]
    pub approval: ApprovalConfig,
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub risk_weights: RiskWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    #[serde(default = "default_trust")]
    pub trust_level: u8,          // 0-5
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
}
```

加载入口 `Config::load(path) -> Result<Config, CoreError>`：解析 YAML → 校验（trust_level 范围、命令非空、路径合法）→ 返回。校验失败给出明确的中文错误信息。

## 错误

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("配置文件读取失败: {0}")]
    ConfigIo(#[from] std::io::Error),
    #[error("配置解析失败: {0}")]
    ConfigParse(String),
    #[error("配置校验失败: {0}")]
    ConfigInvalid(String),
}
```

库内一律返回 `Result`，不 `unwrap`/`panic`。

## 脱敏

`redact.rs` 提供：

```rust
/// 对字符串中的敏感片段脱敏，用于审计落盘前处理
pub fn redact(input: &str) -> String;

/// 对 json 值递归脱敏（按 key 名命中：token/password/secret/key/authorization）
pub fn redact_json(value: &mut serde_json::Value);
```

策略：

- 按 **key 名**命中敏感字段（`token`、`password`、`secret`、`api_key`、`authorization` 等），值整体替换为 `***`。
- 按 **值模式**命中（看起来像私钥块 `-----BEGIN ...`、长 base64、`sk-` 前缀等），保留前后各 2 字符，中间打码。
- 脱敏在审计写入路径上调用，确保 SEC-002 / SEC-003 / SEC-007。

## 测试要点

- `Action` 的 Ord 合成行为（`Allow.max(Block) == Block`）。
- 配置加载的正例与各类非法配置的报错。
- 脱敏：常见密钥格式都被打码，正常文本不被误伤。
