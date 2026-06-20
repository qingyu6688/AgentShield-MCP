# 贡献指南

感谢你愿意为 AgentShield MCP 出力。下面是参与开发的约定，读完再动手能省很多来回。

## 开发环境

- Rust **stable** 工具链（建议用 `rustup` 管理）
- 组件：`rustfmt`、`clippy`
- 桌面端（v0.3 起）：Node.js 18+、Tauri 依赖

```bash
rustc --version     # 确认 >= 1.80
cargo --version
rustup component add rustfmt clippy
```

## 本地开发流程

```bash
git clone https://github.com/qingyu6688/AgentShield-MCP.git
cd AgentShield-MCP
cargo build
cargo test
```

提交前**必须**全部通过：

```bash
cargo fmt --all                  # 格式化
cargo clippy --all-targets -- -D warnings   # 静态检查，警告即报错
cargo test --all                 # 运行测试
```

## 代码规范

- 缩进 4 空格；类型 / trait / 枚举用 `PascalCase`，函数 / 变量用 `snake_case`，常量用 `UPPER_SNAKE_CASE`。
- 库 crate 不要用 `unwrap()` / `expect()` / `panic!` 处理可恢复错误，统一返回 `Result`；库用 `thiserror`，二进制入口用 `anyhow`。
- 禁止 `unsafe`，确有必要必须写注释说明安全前提。
- 公共 API 必须有 `///` 文档注释，注释用**中文**，重点解释“为什么这样做”。
- 不要提交调试用的 `dbg!` / `println!`。

## 提交规范

提交信息格式：`<类型>: <简短描述>`

| 类型 | 含义 |
|---|---|
| `feat` | 新功能 |
| `fix` | Bug 修复 |
| `refactor` | 重构，不改变功能 |
| `docs` | 文档更新 |
| `test` | 测试相关 |
| `chore` | 构建、依赖、配置等杂项 |

示例：`feat: MCP Proxy 支持 stdio 传输`

## 贡献风险规则（最欢迎）

规则库在 `crates/agentshield-rules`（内置）与 `policies/`（用户可加载）。新增一条规则请同时：

1. 在对应规则文件里加规则，写清 `name`、`description`、`match`、`action`、`severity`。
2. 在 `tests/` 加**至少一个命中用例和一个不命中用例**——每条内置规则都必须有测试（MAINT-003）。
3. 在 `docs/risk-engine.md` 的规则表里补一行。

规则要避免误报。如果一条规则会频繁拦住正常操作，它的价值是负的。

## 提 PR 前自检

- [ ] `cargo fmt` / `cargo clippy` / `cargo test` 全绿
- [ ] 新功能带了测试
- [ ] 涉及行为变更的更新了 `docs/` 和 `CHANGELOG.md`
- [ ] 没有硬编码密钥、Token、绝对路径
- [ ] commit message 符合规范

## 行为准则

对人友善，对事较真。讨论针对方案不针对人。

有疑问可开 Discussion，或邮件 maorongkang@gmail.com。
