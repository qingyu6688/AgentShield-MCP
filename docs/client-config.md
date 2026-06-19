# 客户端接入

接入思路只有一句话：**把客户端原本指向真实 MCP Server 的配置，改成指向 AgentShield**，由 AgentShield 在背后转发。

```text
之前： Client ──▶ Real MCP Server
之后： Client ──▶ AgentShield ──▶ Real MCP Server
```

下面假设你已经：

1. 构建出 `agentshield`（在 PATH 中）
2. `agentshield init` 生成了配置
3. `agentshield mcp add <name> ...` 注册了真实 server

## 通用做法

AgentShield 作为 stdio MCP server 启动：

```bash
agentshield proxy start --server <name>
```

任何支持自定义 MCP server 命令的客户端，把 command 设成上面这条即可。


## Cursor

`~/.cursor/mcp.json` 或项目 `.cursor/mcp.json`：

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

## Codex CLI

在 Codex 的 MCP 配置（TOML）里：

```toml
[mcp_servers.filesystem]
command = "agentshield"
args = ["proxy", "start", "--server", "filesystem"]
```

## Gemini CLI

在 `settings.json` 的 `mcpServers` 中：

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

## 多 Server 聚合

如果想用一个 AgentShield 入口管多个 server，在 `config.yaml` 注册多个，然后启动聚合模式：

```bash
agentshield proxy start --all
```

客户端只配一个 `agentshield` 入口即可，工具会按 server 名加前缀列出。

## 确认提示在哪看

`confirm` 决策的提示出现在 **`agentshield proxy start` 所在的终端**。如果客户端是后台拉起 server 的（看不到那个终端），有两种选择：

1. 用桌面端确认（v0.3 起）。
2. 把策略里需要确认的规则改成 `block` 或 `allow`，避免依赖交互（适合 CI / 无人值守）。

## Windows 注意

如果上游是 npm 系命令（如 `npx ...`），在 `mcp add` 时 `--command` 要用 `npx.cmd`
而不是 `npx`——Windows 上 `npx` 是脚本、`npx.cmd` 才是可直接拉起的可执行文件：

```bash
agentshield mcp add filesystem \
  --command npx.cmd \
  --args "-y,@modelcontextprotocol/server-filesystem,."
```

## 验证接入成功

接好后让 AI 做一件会被记录的事（比如读个文件），然后：

```bash
agentshield audit list
```

能看到刚才的调用记录，就说明流量确实穿过了 AgentShield。

## 摘除

想暂时不用，把客户端配置改回直连真实 server 即可。AgentShield 不修改客户端以外的任何东西，无残留。
