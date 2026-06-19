# 风险引擎

风险引擎给每一次工具调用打一个 0–100 的分，并给出等级、原因和建议动作。它是一个**纯函数**：输入 `ToolCall` + 上下文，输出 `RiskAssessment`，不做任何 IO。

## 1. 输出结构

```json
{
  "score": 95,
  "level": "critical",
  "reasons": [
    "下载远程脚本",
    "把远程内容管道进 shell",
    "执行未经验证的代码"
  ],
  "recommended_action": "block"
}
```

## 2. 等级与默认动作

| 分数 | 等级 | 默认动作 |
|---|---|---|
| 0–29 | Low | Allow |
| 30–59 | Medium | Log |
| 60–79 | High | Confirm |
| 80–100 | Critical | Block / Confirm |

## 3. 评分维度

每个维度独立给出“贡献分”，最终分由各维度加权汇总并裁剪到 0–100。

| 维度 | 说明 | 示例 |
|---|---|---|
| 操作类型 | 基础风险 | read < write < delete ≈ execute |
| 目标路径 | 敏感文件加分 | `.env`、`~/.ssh`、`/etc` |
| 命令内容 | 命中危险模式 | `rm -rf`、`sudo`、`curl \| bash` |
| 网络访问 | 陌生外部域名 | 非白名单域名 |
| 数据库操作 | 破坏性 SQL | `DROP` / `TRUNCATE` / 无 WHERE 的 `DELETE` |
| Server 信任等级 | untrusted 加分 | trusted / untrusted |
| 文件敏感度 | 密钥 > 配置 > 源码 | `*.pem` > `nginx.conf` > `*.rs` |
| 历史行为 | 短时大量删除/写入 | 异常突发（P2） |

## 4. 评分模型

伪代码：

```text
score = base(event_type)
for dim in dimensions:
    score += dim.contribution(tool_call, context) * weight[dim]
score = clamp(score, 0, 100)
level = level_of(score)
action = action_of(level)        # 默认动作
```

- **权重可配置**（RISK-005）：`config.yaml` 里可调每个维度的 weight。
- **规则可调分**（RISK-004）：命中内置规则可直接把分抬到规则声明的 severity 对应区间（如 critical 规则把分顶到 90+）。
- **可解释**（RISK-003 / RISK-007）：每个加分项都往 `reasons` 里追加一句人话，确认界面和报告直接展示。

## 5. 内置规则

规则来自 `agentshield-rules` crate，分三类。完整可在 `policies/default.yaml` 查看与覆盖。

### 5.1 Shell 规则

| 规则 | 等级 | 默认动作 |
|---|---|---|
| `rm -rf` | High | Confirm |
| `curl \| bash` | Critical | Block |
| `wget \| sh` | Critical | Block |
| `chmod -R 777` | High | Confirm |
| `sudo` | Medium | Confirm |
| `docker rm` | High | Confirm |
| `docker volume rm` | Critical | Confirm |
| `kubectl delete` | High | Confirm |
| `git push --force` | High | Confirm |

### 5.2 文件规则

| 文件 | 等级 | 默认动作 |
|---|---|---|
| `.env` / `.env.local` | Critical | Block |
| `id_rsa` / `id_ed25519` | Critical | Block |
| `*.pem` | Critical | Block |
| `docker-compose.yml` | High | Confirm |
| `nginx.conf` | High | Confirm |
| `package.json` / `pom.xml` | Medium | Log |

### 5.3 数据库规则

| SQL | 等级 | 默认动作 |
|---|---|---|
| `DROP DATABASE` | Critical | Block |
| `DROP TABLE` | Critical | Confirm |
| `TRUNCATE TABLE` | Critical | Confirm |
| `DELETE` 无 WHERE | High | Confirm |
| `UPDATE` 无 WHERE | High | Confirm |
| `ALTER TABLE DROP COLUMN` | High | Confirm |

## 6. 与策略引擎的关系

风险引擎给的是**建议动作**；策略引擎给的是**规则动作**。两者由 Gateway 合成，**取更严格者**。例如：

- 风险分 40（建议 log）+ 策略命中 block → 最终 **block**
- 风险分 85（建议 block）+ 策略明确 allow 该工具 → 仍倾向 block，但若策略规则显式声明优先级更高的 allow，可被覆盖（见 [policy.md](policy.md) 的优先级说明）

## 7. 扩展自定义规则

规则实现统一 trait：

```rust
pub trait Rule: Send + Sync {
    fn name(&self) -> &str;
    /// 返回 Some 表示命中，附带加分、原因、严重级别
    fn evaluate(&self, call: &ToolCall, ctx: &Context) -> Option<RuleHit>;
}

pub struct RuleHit {
    pub score_delta: i16,
    pub severity: RiskLevel,
    pub reason: String,
}
```

第三方 crate 实现 `Rule` 后注册到引擎即可（MAINT-006）。每条规则必须配命中/不命中测试用例。

## 8. 相关需求

覆盖 7.2（RISK-001 ~ RISK-007）与第 13 节内置规则。详细设计见 [docs/design/04-risk.md](design/04-risk.md) 与 [docs/design/05-rules.md](design/05-rules.md)。
