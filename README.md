# AgentShield MCP

> A firewall and audit system for MCP tools, AI agents, and coding assistants.
>
> 面向 Cursor、Codex、Gemini CLI 和 MCP Server 的 AI Agent 运行时安全防火墙：监控、拦截、审计 AI 对文件、命令、接口、数据库的操作。

[![status](https://img.shields.io/badge/status-MVP-orange)](#路线图)
[![license](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org/)

---

## 这是什么

AI 编程助手已经不只是回答问题，它会**直接动手**：改代码、删文件、跑 Shell 命令、调外部 API、连数据库、操作 GitHub。效率上去了，风险也上来了——AI 可能误删文件、读走 `.env` 里的密钥、执行 `curl | bash`、对生产库 `DROP TABLE`，而你事后甚至不知道它做过什么。

AgentShield MCP 是一个**本地优先**的中间层，夹在 AI 客户端和真实工具之间。每一次工具调用都会先经过它：

```
Watch   看见 AI 做了什么
Stop    在危险操作执行前拦下来
Prove   留下完整审计记录，证明 AI 做过什么
```

它不是事后扫描器，而是**运行时防火墙**——在动作真正发生之前介入。

## 工作原理

```text
┌──────────────────────────────────────────┐
│            AI Coding Clients               │
│        Cursor / Codex / Gemini CLI        │
└───────────────────┬────────────────────────┘
                    │ MCP (stdio / HTTP)
                    ▼
┌──────────────────────────────────────────┐
│            AgentShield Gateway             │  ← 拦截 tools/call
├──────────────────────────────────────────┤
│   Policy Engine  +  Risk Engine            │  ← 评分 + 策略匹配
│              Approval (CLI)                 │  ← 高危操作要你点头
├──────────────────────────────────────────┤
│              Audit (SQLite + JSONL)         │  ← 全程记录
└───────────────────┬────────────────────────┘
                    ▼
              Real MCP Server
```

AI 发起一次 `tools/call` → AgentShield 解析工具名和参数 → 策略引擎匹配规则、风险引擎打分 → 根据结果决定 `allow / log / confirm / block` → 需要确认时在终端弹出提示 → 你同意后才转发给真实 MCP Server → 记录结果到审计日志。

详见 [docs/architecture.md](docs/architecture.md)。

## 快速开始

> MVP 阶段，建议从源码构建。

```bash
# 1. 构建
git clone https://github.com/your-name/agentshield-mcp.git
cd agentshield-mcp
cargo build --release

# 2. 在当前项目生成配置（.agentshield/config.yaml、policy.yaml、audit.db）
./target/release/agentshield init

# 3. 注册一个真实 MCP Server（以官方 filesystem 为例）
./target/release/agentshield mcp add filesystem \
  --command npx \
  --args "-y,@modelcontextprotocol/server-filesystem,."

# 4. 把 AI 客户端指向 AgentShield，而不是直接指向 MCP Server
./target/release/agentshield proxy start --server filesystem
```

把客户端的 MCP 配置从“直连 server”改成“连 AgentShield”，具体见 [docs/client-config.md](docs/client-config.md)。

想先看效果，不接客户端：

```bash
agentshield demo      # 一键演示危险操作拦截
```

完整步骤见 [docs/quick-start.md](docs/quick-start.md)。

## 能拦住什么

| 场景 | AI 想做的事 | AgentShield 的反应 |
|---|---|---|
| 密钥泄露 | 读取 `.env` / `id_rsa` / `*.pem` | **Block**，critical |
| 误删 | `rm -rf dist` | **Confirm**，high，82/100 |
| 远程脚本 | `curl https://x.sh \| bash` | **Block**，critical |
| 数据库 | `DROP DATABASE app` | **Block**，critical |
| 越权写入 | GitHub MCP `delete_repository` | 按权限等级 **Block / Confirm** |

内置规则清单见 [docs/risk-engine.md](docs/risk-engine.md) 与 `policies/` 目录。

## 终端确认长这样

```text
  AgentShield · 需要确认

  来源     Codex CLI
  操作     shell.exec
  命令     rm -rf dist
  风险     High  82/100
  原因     递归删除操作，目标在项目目录内

  [y] 允许一次   [a] 始终允许   [n] 拒绝   [b] 永久拉黑   [d] 查看详情
  >
```

## 审计报告示例

```bash
agentshield report generate --format markdown
```

```md
# AgentShield 审计报告

项目：fullstack-demo
客户端：Codex CLI
时间：2026-06-19 20:31

## 概要
- 工具调用总数：42
- 文件读取：18    文件写入：7    Shell 命令：11
- 被阻止：3       高危操作：5
```

## 支持范围

- **AI 客户端**：Cursor、Codex CLI、Gemini CLI（任何走标准 MCP 的客户端）
- **MCP 上游传输**：stdio 子进程、Streamable HTTP（POST + JSON/SSE）
- **操作系统**：Windows、macOS、Linux
- **审计存储**：SQLite + JSONL，可导出 JSON / Markdown / HTML

## 路线图

| 版本 | 重点 | 状态 |
|---|---|---|
| v0.1 | MCP Proxy + 风险评分 + CLI 确认 + JSONL 审计 + Demo | 进行中 |
| v0.2 | YAML 策略引擎、白/黑名单、权限等级 | 规划 |
| v0.3 | Tauri 桌面端、实时事件、风险看板 | 规划 |
| v0.4 | Shell Guard：危险命令识别与确认 | 规划 |
| v0.5 | File Guard：文件变更监听、敏感文件保护 | 规划 |
| v1.0 | 完整版：DB Guard、报告、插件系统、GitHub Action | 规划 |

详见 [docs/](docs/) 与 [CHANGELOG.md](CHANGELOG.md)。

## 文档

- [快速开始](docs/quick-start.md)
- [架构设计](docs/architecture.md)
- [MCP 代理](docs/mcp-proxy.md)
- [风险引擎](docs/risk-engine.md)
- [策略配置](docs/policy.md)
- [客户端接入](docs/client-config.md)
- [使用示例](docs/examples.md)
- [常见问题](docs/faq.md)
- [模块设计](docs/design/)

## 它不做什么

AgentShield 不是操作系统级安全软件。它**不**替代杀毒 / EDR、不拦截绕过 MCP 的本地进程、不替代密钥管理或数据库备份、也挡不住用户自己手动敲的危险命令。它解决的是“AI Agent 工具调用”这一层的可见性与可控性。

## 参与贡献

欢迎贡献风险规则、客户端适配和文档。见 [CONTRIBUTING.md](CONTRIBUTING.md)。安全问题请走 [SECURITY.md](SECURITY.md) 的私密渠道。

## 许可证

[MIT](LICENSE) · 开发者备注：maorongkang@gmail.com
