# 快速开始

5 分钟把 AgentShield MCP 跑起来，并看到它拦下一次危险操作。

## 前置条件

- Rust stable（>= 1.80），`cargo` 可用
- 一个能用的 MCP Server（下面以官方 filesystem server 为例，需要 Node.js）

```bash
rustc --version
cargo --version
```

## 1. 构建

```bash
git clone https://github.com/your-name/agentshield-mcp.git
cd agentshield-mcp
cargo build --release
```

产物在 `target/release/agentshield`（Windows 为 `agentshield.exe`）。可以把它加入 PATH，下文统一写作 `agentshield`。

## 2. 初始化配置

在你的**目标项目目录**下执行：

```bash
agentshield init
```

会生成：

```text
.agentshield/
├── config.yaml    # 主配置
├── policy.yaml    # 策略规则（已内置一套安全默认）
└── audit.db       # 审计数据库
```

## 3. 先看 Demo（不接客户端）

最快的验证方式，不需要配置任何 AI 客户端：

```bash
agentshield demo
```

它会模拟几次典型的危险调用，并展示拦截过程：

```text
[1/4] AI 尝试读取 .env
      → file.read  ".env"
      → 风险 95/100 critical：命中规则 block-env-read
      → 决策 BLOCK ✗ 已阻止

[2/4] AI 尝试 rm -rf dist
      → shell.exec  "rm -rf dist"
      → 风险 82/100 high：递归删除
      → 决策 CONFIRM ⏸ 需要确认

[3/4] AI 调用 GitHub MCP create_pull_request
      → mcp.tool_call  "create_pull_request"
      → 风险 45/100 medium
      → 决策 LOG ✓ 已放行并记录

[4/4] AI 尝试 DROP TABLE users
      → db.query  "DROP TABLE users"
      → 风险 90/100 critical
      → 决策 BLOCK ✗ 已阻止

审计已写入 .agentshield/audit.db
```

## 4. 接入真实 MCP Server

注册一个真实 server：

```bash
agentshield mcp add filesystem \
  --command npx \
  --args "-y,@modelcontextprotocol/server-filesystem,."

agentshield mcp list
```

启动代理（stdio 模式）：

```bash
agentshield proxy start --server filesystem
```

此时 AgentShield 自己成为一个 MCP Server（对客户端而言），背后转发给真实的 filesystem server。

## 5. 把 AI 客户端指向 AgentShield

关键一步：客户端原来直连真实 server，现在改成连 AgentShield。各客户端的具体配置见 [client-config.md](client-config.md)。

以通用 MCP JSON 配置为例，把 command 换成：

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "agentshield",
      "args": ["proxy", "start", "--server", "filesystem"]
    }
  }
}
```

重启客户端，让 AI 去读 `.env`，你会在终端看到确认/拦截提示。

## 6. 查看审计与报告

```bash
agentshield audit list                       # 看最近的事件
agentshield audit list --level critical      # 只看高危
agentshield audit list --server filesystem   # 只看某个 server
agentshield audit list --since 2026-06-19 --until 2026-06-20   # 按时间范围
agentshield report generate --format markdown -o report.md
```

## 附：与真实 filesystem server 的端到端联调（已验证）

下面这段不接客户端，直接用管道把 MCP 消息喂给 AgentShield，背后跑官方
`@modelcontextprotocol/server-filesystem`。用来确认“转发 + 拦截”链路真的通。

```bash
# 准备一个沙箱目录
mkdir -p ~/ashield-e2e
echo "hello" > ~/ashield-e2e/hello.txt
echo "SECRET_TOKEN=x" > ~/ashield-e2e/.env

# 直接指定上游（也可以先 mcp add 再用 --server）
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e","version":"0.0.0"}}}' \
 '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_text_file","arguments":{"path":"<沙箱绝对路径>/hello.txt"}}}' \
 '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_text_file","arguments":{"path":"<沙箱绝对路径>/.env"}}}' \
| agentshield proxy start \
    --command npx --args "-y,@modelcontextprotocol/server-filesystem,<沙箱绝对路径>"
```

预期：

- `initialize`、读 `hello.txt` → 透传到真实 server，客户端拿到真实响应；
- 读 `.env` → 被 AgentShield 拦下，回 `-32010` 错误，真实 server 根本没收到；
- `agentshield audit list` 能看到这两次 `tools/call`（hello.txt → Log，`.env` → Block）。

> **Windows 提示**：上游若是 npm 系命令，`--command` 要写 `npx.cmd`（不是 `npx`）。
> 真实客户端直接以参数方式拉起 AgentShield，不经过 shell，
> 不会有路径转义问题；只有在 Git Bash 里手动测试时需注意 MSYS 的参数路径转换
> （可用 `MSYS_NO_PATHCONV=1` 关闭）。

## 常见问题

- **客户端连不上？** 确认客户端配置里的 command 路径正确，且 `agentshield proxy start` 能独立跑通。
- **想临时摘掉 AgentShield？** 把客户端配置改回直连真实 server 即可，AgentShield 不留残留。
- **规则太严/太松？** 编辑 `.agentshield/policy.yaml`，支持热更新。语法见 [policy.md](policy.md)。

更多见 [faq.md](faq.md)。
