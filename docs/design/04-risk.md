# 设计 · agentshield-risk

风险评分引擎。纯函数：输入 `ToolCall` + 上下文，输出 `RiskAssessment`。无 IO、无副作用、可单测。用户文档见 [risk-engine.md](../risk-engine.md)。

## 职责

- 按多个维度对调用打分，汇总为 0–100。
- 映射出风险等级和建议动作。
- 收集每个加分项的人话原因。
- 依据内置规则命中调整分数。

## 模块结构

```text
agentshield-risk/src/
├── lib.rs
├── engine.rs       # RiskEngine：组合各维度
├── dimensions/     # 各评分维度
│   ├── operation.rs    # 操作类型基础分
│   ├── path.rs         # 敏感路径
│   ├── command.rs      # 危险命令模式
│   ├── network.rs      # 陌生域名
│   ├── database.rs     # 破坏性 SQL
│   ├── trust.rs        # server 信任等级
│   └── history.rs      # 历史行为（P2）
└── score.rs        # 汇总、裁剪、等级映射
```

## 上下文

```rust
pub struct Context<'a> {
    pub server_trust: u8,            // 0-5
    pub weights: &'a RiskWeights,    // 来自 config
    pub recent: &'a RecentActivity,  // 历史行为窗口（P2 用）
}
```

## 维度 trait

```rust
pub trait Dimension {
    /// 返回该维度的贡献分与原因，未命中返回 None
    fn assess(&self, call: &ToolCall, ctx: &Context) -> Option<DimScore>;
}

pub struct DimScore {
    pub points: i16,        // 该维度原始贡献
    pub reason: String,
}
```

## 汇总算法

```text
base = operation_base(event_type)      // read=10 write=30 delete=50 exec=50 ...
total = base
for dim in dimensions:
    if let Some(s) = dim.assess(call, ctx):
        total += s.points * weights[dim]
        reasons.push(s.reason)
score = clamp(total, 0, 100)
level = match score { 0..=29 Low, 30..=59 Medium, 60..=79 High, _ Critical }
action = default_action_of(level)
```

- **权重可配置**（RISK-005）：`RiskWeights` 来自 config，缺省全 1.0。
- **规则命中抬分**（RISK-004）：命中 `agentshield-rules` 的规则时，把分至少抬到该 severity 的下界（critical→80，high→60，medium→30）。
- **可解释**（RISK-003 / 007）：每个命中维度都往 `reasons` 追加中文说明。

## 各维度要点

| 维度 | 命中条件 | 典型贡献 |
|---|---|---|
| operation | 由 event_type 决定基础分 | read 10 / write 30 / delete·exec 50 |
| path | 命中敏感路径表（`.env`、`~/.ssh`、`/etc`、`*.pem`） | +30 ~ +50 |
| command | 命中危险命令正则（`rm -rf`、`curl\|bash`、`sudo`...） | +20 ~ +50 |
| network | 访问非白名单外部域名 | +20 |
| database | 破坏性 SQL（DROP/TRUNCATE/无 WHERE 的 DELETE/UPDATE） | +40 ~ +60 |
| trust | server untrusted（trust_level 低） | +10 ~ +20 |
| history | 短时间大量删除/写入（P2） | +15 |

具体的敏感路径表、命令正则、SQL 模式都委托给 `agentshield-rules`，本 crate 只负责组合与计分，规则数据与匹配在 rules crate 维护（见 [05-rules](05-rules.md)）。

## 与策略引擎的关系

风险引擎只产出 `RiskAssessment`（含建议动作）。最终动作由 policy 引擎合成（取更严格者）。risk 不知道 policy 的存在，保持单向。

## 测试要点

- 每个维度的命中与计分。
- 边界分数的等级映射（29/30、59/60、79/80）。
- 权重调整对总分的影响。
- 规则命中抬分逻辑。
- 示例输出与需求文档 7.2.5 一致。

## 相关需求

7.2 RISK-001 ~ RISK-007；第 13 节内置规则。
