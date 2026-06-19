# 设计 · agentshield-policy

策略引擎。加载 YAML 策略，对 `ToolCall` 匹配规则给出动作，并和风险引擎结果合成最终决策。用户文档见 [policy.md](../policy.md)。

## 职责

- 解析 `policy.yaml` 为内存规则集。
- 对每次调用按顺序匹配规则，返回命中规则与动作。
- 应用 MCP server 权限等级与白/黑名单。
- 与风险评分合成最终 `Decision`（取更严格者，支持 override）。
- 监听策略文件变更，热更新。

## 模块结构

```text
agentshield-policy/src/
├── lib.rs
├── model.rs        # Policy / Rule / Match 数据结构（serde）
├── matcher.rs      # 各匹配条件实现（contains/regex/glob）
├── engine.rs       # PolicyEngine：匹配 + 决策合成
├── permission.rs   # server 权限等级与白黑名单
├── watch.rs        # 文件热更新
└── error.rs
```

## 数据模型

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Policy {
    pub version: u32,
    #[serde(default = "default_allow")]
    pub default_action: Action,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub r#match: Match,
    pub action: Action,
    #[serde(default)]
    pub severity: Option<RiskLevel>,
    #[serde(default)]
    pub r#override: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Match {
    pub r#type: Option<EventType>,
    pub tool: Option<StringMatch>,
    pub server: Option<StringMatch>,
    pub path: Option<StringMatch>,
    pub command: Option<StringMatch>,
    pub sql: Option<StringMatch>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StringMatch {
    #[serde(default)]
    pub equals: Option<String>,
    #[serde(default)]
    pub contains: Vec<String>,
    #[serde(default)]
    pub regex: Option<String>,
    #[serde(default)]
    pub glob: Option<String>,
    #[serde(default)]
    pub r#in: Vec<String>,
}
```

正则在加载时**预编译**并缓存（用 `once`/`regex::Regex` 持有），匹配阶段不重复编译。

## 匹配规则

- 一个 `Match` 内多个字段是 **AND**：全部满足才命中。
- 一个 `StringMatch` 内多种方式是 **OR**：命中任一即可。
- `rules` 列表**按顺序**，第一条 `enabled && 命中` 的规则胜出。
- 没有规则命中时用 `default_action`。

## 权限等级（先于规则）

`PermissionEngine` 按 server 的 `trust_level` 和白黑名单先做一轮裁决：

```text
trust_level == 0 (Blocked)        -> Block
tool in block_tools               -> Block
event 是写/删/执行 且 level==1     -> Block（Read Only）
tool in confirm_tools             -> Confirm
path 命中 blocked_paths           -> Block
event 是写/删 且 level==2          -> Confirm（Confirm Write）
否则                              -> 交给规则与风险引擎
```

## 决策合成

```rust
impl PolicyEngine {
    pub fn decide(&self, call: &ToolCall, risk: RiskAssessment) -> Decision {
        // 1. 权限等级裁决
        let perm = self.permission.evaluate(call);
        // 2. 规则匹配
        let rule_hit = self.match_rule(call);     // Option<&Rule>
        let rule_action = rule_hit.map(|r| r.action).unwrap_or(self.default_action);

        // 3. override 规则可强制覆盖
        if let Some(r) = rule_hit {
            if r.r#override {
                return Decision { action: r.action, risk, matched_rule: Some(r.name.clone()),
                                  reason: r.description.clone() };
            }
        }

        // 4. 取更严格者：权限 / 规则 / 风险建议
        let action = perm.max(rule_action).max(risk.recommended_action);
        Decision { action, risk, matched_rule: rule_hit.map(|r| r.name.clone()), reason: ... }
    }
}
```

利用 `Action` 的 `Ord`（`Allow < Log < Sandbox < Confirm < Block`），合成就是 `max`。

## 热更新

`watch.rs` 用 `notify` 监听 `policy.yaml`。变更时重新加载、重新编译正则，原子替换 `PolicyEngine` 内部的 `Arc<PolicyInner>`。加载失败则保留旧策略并打印警告，不让坏文件搞挂代理。

## policy test 支持

暴露 `PolicyEngine::explain(call) -> Explanation`，返回命中规则、各匹配条件结果、最终动作，供 `agentshield policy test` 输出。

## 测试要点

- 每条内置规则的命中/不命中用例（MAINT-003）。
- AND / OR 组合匹配。
- override 覆盖、default_action 兜底。
- 权限等级与规则的合成优先级。
- 热更新：坏文件不影响运行中的策略。

## 相关需求

7.3 POLICY-001 ~ POLICY-010；7.7 PERM-001 ~ PERM-008。
