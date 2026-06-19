# 设计 · agentshield-rules

内置规则库。集中维护 shell / file / database 三类内置风险规则，供 risk 和 policy 引擎使用。独立成 crate 是为了让规则可单独贡献、单独测试、单独演进（MAINT-001）。

## 职责

- 提供内置规则数据（命令正则、敏感路径、破坏性 SQL）。
- 暴露统一的 `Rule` trait 供引擎调用与第三方扩展。
- 每条规则自带测试用例。

## 模块结构

```text
agentshield-rules/src/
├── lib.rs          # 注册表 + Rule trait
├── shell.rs        # shell 规则
├── file.rs         # 文件规则
├── database.rs     # 数据库规则
└── registry.rs     # RuleRegistry：收集所有规则
```

## Rule trait

```rust
pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn category(&self) -> Category;     // Shell / File / Database
    fn default_severity(&self) -> RiskLevel;
    fn default_action(&self) -> Action;

    /// 命中返回 Some
    fn evaluate(&self, call: &ToolCall) -> Option<RuleHit>;
}

pub struct RuleHit {
    pub rule_name: &'static str,
    pub score_delta: i16,
    pub severity: RiskLevel,
    pub reason: String,         // 中文，进 reasons / 报告
}
```

## 注册表

```rust
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
}

impl RuleRegistry {
    pub fn builtin() -> Self { /* 装载全部内置规则 */ }
    pub fn register(&mut self, rule: Box<dyn Rule>) { ... }  // 第三方扩展
    pub fn evaluate_all(&self, call: &ToolCall) -> Vec<RuleHit> { ... }
}
```

第三方插件实现 `Rule` 后 `register` 即可（MAINT-006）。

## 内置规则清单

### Shell（shell.rs）

| name | 模式 | severity | action |
|---|---|---|---|
| `shell-rm-rf` | `rm\s+-rf` / `rmdir\s+/s` | High | Confirm |
| `shell-curl-bash` | `curl.*\|\s*(bash\|sh)` | Critical | Block |
| `shell-wget-sh` | `wget.*\|\s*(bash\|sh)` | Critical | Block |
| `shell-chmod-777` | `chmod\s+-R\s+777` | High | Confirm |
| `shell-sudo` | `\bsudo\b` | Medium | Confirm |
| `shell-docker-rm` | `docker\s+rm` | High | Confirm |
| `shell-docker-volume-rm` | `docker\s+volume\s+rm` | Critical | Confirm |
| `shell-kubectl-delete` | `kubectl\s+delete` | High | Confirm |
| `shell-git-force-push` | `git\s+push\s+.*--force` | High | Confirm |

### File（file.rs）

| name | 目标 | severity | action |
|---|---|---|---|
| `file-env` | `.env` / `.env.local` / `.env.production` | Critical | Block |
| `file-ssh-key` | `id_rsa` / `id_ed25519` | Critical | Block |
| `file-pem` | `*.pem` | Critical | Block |
| `file-compose` | `docker-compose.yml` | High | Confirm |
| `file-nginx` | `nginx.conf` | High | Confirm |
| `file-manifest` | `package.json` / `pom.xml` | Medium | Log |

文件规则区分操作：读敏感文件 → 按表；写/删敏感文件风险更高（在 file 规则里对 write/delete 额外加分）。

### Database（database.rs）

| name | 模式 | severity | action |
|---|---|---|---|
| `db-drop-database` | `DROP\s+DATABASE` | Critical | Block |
| `db-drop-table` | `DROP\s+TABLE` | Critical | Confirm |
| `db-truncate` | `TRUNCATE\s+TABLE` | Critical | Confirm |
| `db-delete-no-where` | `DELETE\s+FROM` 且无 `WHERE` | High | Confirm |
| `db-update-no-where` | `UPDATE\s+...\s+SET` 且无 `WHERE` | High | Confirm |
| `db-drop-column` | `ALTER\s+TABLE.*DROP\s+COLUMN` | High | Confirm |

SQL 匹配做大小写不敏感、容忍多空白；“无 WHERE”用简单解析（去注释/字符串后检查 `where` 关键字）。

## 正则管理

所有正则在 crate 初始化时用 `once_cell::Lazy` 预编译，避免每次调用重复编译。模式写在常量里，便于审阅与贡献。

## 测试要点（强约束）

每条规则**至少**一个命中用例 + 一个不命中用例（MAINT-003）。重点防误报：

- `db-delete-no-where` 不能误伤带 WHERE 的 DELETE。
- `shell-rm-rf` 不能误伤 `rm file.txt`。
- `file-env` 命中 `.env` 但不命中 `environment.ts`。

## 相关需求

第 13 节内置风险规则；MAINT-001 / 003 / 006。
