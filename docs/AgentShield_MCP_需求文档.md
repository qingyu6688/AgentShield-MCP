# AgentShield MCP 项目需求文档

> 项目名称：AgentShield MCP
> 项目类型：开源开发者工具 / AI Agent 安全工具 / MCP 安全防火墙
> 项目定位：面向 MCP 工具、AI Agent、AI 编程助手的运行时安全防火墙与审计系统
> 适用对象：Cursor、Codex、Gemini CLI、MCP Server、AI Coding Agent 使用者

---

## 1. 项目背景

随着 AI Agent、MCP Server、AI 编程助手快速发展，AI 已经不再只是回答问题，而是能够直接执行真实操作，例如：

- 读取本地项目文件
- 修改代码
- 删除文件
- 执行 Shell 命令
- 调用外部 API
- 创建 GitHub Issue / Pull Request
- 访问数据库
- 操作 Docker、Kubernetes、服务器环境

这些能力虽然提升了开发效率，但也带来了新的安全风险：

- AI 误删重要文件
- AI 读取 `.env`、SSH 私钥、Token 等敏感信息
- AI 执行 `rm -rf`、`curl | bash` 等高危命令
- AI 调用 MCP Server 执行未经确认的写入操作
- AI 自动修改数据库结构或删除数据
- 用户无法追踪 AI 到底做了哪些操作
- 团队无法审计 AI Agent 的工具调用行为

因此，需要一个位于 AI Agent 与 MCP Server / 本地工具之间的安全中间层，用于监控、评分、拦截和审计 AI 的工具调用行为。

AgentShield MCP 的目标就是成为：

> AI Agent Runtime Firewall
> AI Agent 运行时安全防火墙

---

## 2. 项目简介

AgentShield MCP 是一个本地优先的 AI Agent 工具调用安全防火墙。

它位于 Cursor、Codex、Gemini CLI 等 AI 编程助手与 MCP Server、本地 Shell、文件系统、数据库、API 服务之间，负责对 AI 的工具调用进行：

- 实时监控
- 风险评分
- 策略匹配
- 危险操作拦截
- 用户确认
- 权限分级
- 审计记录
- 报告生成

英文介绍：

```text
A firewall and audit system for MCP tools, AI agents, and coding assistants.
```

中文介绍：

```text
一个面向 Cursor、Codex、Gemini CLI 和 MCP Server 的 AI Agent 安全防火墙，监控、拦截、审计 AI 对文件、命令、接口、数据库的操作。
```

---

## 3. 项目目标

### 3.1 核心目标

AgentShield MCP 需要解决三个核心问题：

```text
Watch：看见 AI 做了什么
Stop：阻止 AI 做危险操作
Prove：证明 AI 做过什么
```

具体目标如下：

1. 监控 AI Agent 的 MCP 工具调用行为。
2. 记录 AI 执行过的命令、访问过的文件、调用过的接口。
3. 对危险操作进行风险评分。
4. 对高危行为进行阻止或二次确认。
5. 为不同 MCP Server 设置权限等级。
6. 生成完整的 AI 操作审计报告。
7. 提供 CLI 与桌面端可视化监控面板。
8. 支持 Cursor、Codex、Gemini CLI 等主流 AI Coding 工具。

### 3.2 开源目标

项目应具备较强的 GitHub 开源传播能力：

- README 清晰
- 一键运行 Demo
- 支持跨平台安装
- 提供真实风险拦截示例
- 有完整文档
- 有规则库贡献机制
- 有可视化界面
- 有明确的安全定位

---

## 4. 用户角色

### 4.1 个人开发者

使用 Cursor、Codex、Gemini CLI 等 AI 工具辅助开发，希望避免 AI 误删文件、泄露密钥或执行危险命令。

### 4.2 开源项目维护者

希望知道 AI Agent 对项目做了哪些改动，是否执行了高风险操作，并希望生成审计记录。

### 4.3 团队技术负责人

希望给团队统一配置 AI 工具调用权限，限制 AI 访问敏感文件、数据库和服务器命令。

### 4.4 企业安全人员

希望对 AI Agent 工具调用进行审计，发现高危行为、越权访问和异常操作。

---

## 5. 项目范围

### 5.1 项目包含内容

AgentShield MCP 主要包含以下模块：

```text
agentshield-core        核心安全模型与通用能力
agentshield-proxy       MCP 代理与工具调用拦截
agentshield-policy      安全策略引擎
agentshield-risk        风险评分引擎
agentshield-audit       审计日志与报告生成
agentshield-cli         命令行工具
agentshield-desktop     桌面端监控面板
agentshield-rules       内置风险规则库
```

### 5.2 项目不包含内容

AgentShield MCP 不是完整的操作系统级安全软件，因此不承诺：

- 替代杀毒软件或 EDR
- 阻止所有绕过 MCP 的本地进程行为
- 绝对保证恶意 MCP Server 无法造成损害
- 替代密钥管理系统
- 替代数据库备份系统
- 阻止用户手动执行危险命令

---

## 6. 总体架构

### 6.1 架构图

```text
┌──────────────────────────────────────────┐
│        AI Coding Clients                  │
│      Cursor / Codex / Gemini CLI       │
└───────────────────┬──────────────────────┘
                    │ MCP / Shell / File / API
                    ▼
┌──────────────────────────────────────────┐
│          AgentShield Gateway              │
│  MCP Proxy / Command Proxy / File Watcher  │
└───────────────────┬──────────────────────┘
                    ▼
┌──────────────────────────────────────────┐
│          Security Decision Layer          │
│ Policy Engine / Risk Engine / Approval UI │
└───────────────────┬──────────────────────┘
                    ▼
┌──────────────────────────────────────────┐
│          Execution Layer                  │
│ MCP Server / Sandbox / Shell / DB / HTTP  │
└───────────────────┬──────────────────────┘
                    ▼
┌──────────────────────────────────────────┐
│          Audit Layer                      │
│ SQLite / JSONL / Dashboard / Reports      │
└──────────────────────────────────────────┘
```

### 6.2 调用流程

```text
AI Client 发起工具调用
        ↓
AgentShield MCP Proxy 接收请求
        ↓
解析工具名称、参数、来源、上下文
        ↓
Policy Engine 匹配安全策略
        ↓
Risk Engine 计算风险评分
        ↓
根据评分决定 allow / confirm / block / sandbox
        ↓
若需要确认，则弹出 CLI 或桌面端确认窗口
        ↓
用户允许后转发给真实 MCP Server
        ↓
记录执行结果
        ↓
生成审计日志
```

---

## 7. 功能性需求

## 7.1 MCP Proxy 模块

### 7.1.1 功能描述

AgentShield MCP 应能够作为 MCP Proxy 运行在 AI Client 和真实 MCP Server 之间。

### 7.1.2 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| MCP-001 | 支持接收 AI Client 的 MCP 请求 | P0 |
| MCP-002 | 支持转发 `tools/list` 请求 | P0 |
| MCP-003 | 支持拦截 `tools/call` 请求 | P0 |
| MCP-004 | 支持连接真实 MCP Server | P0 |
| MCP-005 | 支持 stdio 传输模式 | P0 |
| MCP-006 | 支持 SSE / Streamable HTTP 传输模式 | P1 |
| MCP-007 | 支持隐藏被禁用的工具 | P1 |
| MCP-008 | 支持修改工具描述，加入风险提示 | P2 |
| MCP-009 | 支持多 MCP Server 聚合 | P1 |
| MCP-010 | 支持 MCP Server 权限配置 | P0 |

### 7.1.3 验收标准

- AI Client 可以连接 AgentShield。
- AgentShield 可以连接真实 MCP Server。
- AI Client 可以正常获取工具列表。
- AI 调用工具时，AgentShield 能拦截并记录。
- 高危工具调用可以被阻止或要求确认。

---

## 7.2 风险评分模块

### 7.2.1 功能描述

系统需要根据操作类型、目标资源、命令内容、MCP Server 权限等级、上下文等因素计算风险评分。

### 7.2.2 风险等级

| 分数范围 | 等级 | 默认动作 |
|---|---|---|
| 0 - 29 | Low | Allow |
| 30 - 59 | Medium | Log |
| 60 - 79 | High | Confirm |
| 80 - 100 | Critical | Block / Confirm |

### 7.2.3 评分维度

| 维度 | 示例 |
|---|---|
| 操作类型 | read / write / delete / execute / network / database |
| 目标路径 | `.env`、`~/.ssh`、`/etc`、项目源码 |
| 命令内容 | `rm -rf`、`sudo`、`curl | bash` |
| 网络访问 | 是否访问陌生外部域名 |
| 数据库操作 | 是否 `DROP`、`DELETE`、`TRUNCATE` |
| MCP Server 信任等级 | trusted / untrusted |
| 文件敏感度 | 密钥文件、配置文件、代码文件 |
| 历史行为 | 是否短时间内大量删除或写入 |

### 7.2.4 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| RISK-001 | 支持对每次工具调用生成风险分数 | P0 |
| RISK-002 | 支持输出风险等级 | P0 |
| RISK-003 | 支持输出风险原因 | P0 |
| RISK-004 | 支持根据规则调整风险分 | P0 |
| RISK-005 | 支持自定义评分权重 | P1 |
| RISK-006 | 支持异常行为检测 | P2 |
| RISK-007 | 支持风险评分解释说明 | P1 |

### 7.2.5 示例输出

```json
{
  "score": 95,
  "level": "critical",
  "reasons": [
    "Downloads remote script",
    "Pipes remote content into shell",
    "Executes unverified code"
  ],
  "recommended_action": "block"
}
```

---

## 7.3 策略引擎模块

### 7.3.1 功能描述

系统应支持通过 YAML / JSON 配置安全策略，根据策略判断工具调用是否允许、确认、阻止或进入沙箱。

### 7.3.2 策略动作

| 动作 | 含义 |
|---|---|
| allow | 直接放行 |
| log | 放行但记录 |
| confirm | 执行前需要用户确认 |
| block | 阻止执行 |
| sandbox | 在沙箱中执行 |

### 7.3.3 策略示例

```yaml
version: 1

default_action: allow

rules:
  - name: block-env-read
    description: Block reading environment files
    match:
      type: file.read
      path:
        contains:
          - ".env"
          - "id_rsa"
          - "id_ed25519"
    action: block
    severity: critical

  - name: confirm-recursive-delete
    match:
      type: shell.exec
      command:
        regex: "rm\\s+-rf|rmdir\\s+/s"
    action: confirm
    severity: high

  - name: block-curl-bash
    match:
      type: shell.exec
      command:
        regex: "curl.*\\|\\s*(bash|sh)|wget.*\\|\\s*(bash|sh)"
    action: block
    severity: critical
```

### 7.3.4 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| POLICY-001 | 支持 YAML 策略文件 | P0 |
| POLICY-002 | 支持规则启用 / 禁用 | P0 |
| POLICY-003 | 支持根据工具名称匹配 | P0 |
| POLICY-004 | 支持根据命令正则匹配 | P0 |
| POLICY-005 | 支持根据文件路径匹配 | P0 |
| POLICY-006 | 支持根据 MCP Server 名称匹配 | P0 |
| POLICY-007 | 支持 allow / confirm / block | P0 |
| POLICY-008 | 支持 sandbox | P2 |
| POLICY-009 | 支持策略热更新 | P1 |
| POLICY-010 | 支持策略测试命令 | P1 |

---

## 7.4 命令审计模块

### 7.4.1 功能描述

系统需要记录 AI Agent 或用户通过 AgentShield 执行的命令。

### 7.4.2 记录字段

```text
command
cwd
agent_name
client_name
start_time
end_time
exit_code
stdout
stderr
risk_score
decision
reason
```

### 7.4.3 高危命令示例

```text
rm -rf
sudo
curl | bash
wget | sh
chmod -R 777
docker rm
docker volume rm
kubectl delete
git push --force
npm publish
```

### 7.4.4 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| CMD-001 | 支持记录执行命令 | P0 |
| CMD-002 | 支持记录命令执行目录 | P0 |
| CMD-003 | 支持记录命令返回码 | P0 |
| CMD-004 | 支持记录 stdout / stderr | P1 |
| CMD-005 | 支持高危命令确认 | P0 |
| CMD-006 | 支持命令黑名单 | P0 |
| CMD-007 | 支持命令白名单 | P1 |
| CMD-008 | 支持命令执行超时设置 | P1 |

---

## 7.5 文件监控模块

### 7.5.1 功能描述

系统需要监控 AI 对文件系统的读取、写入、删除、重命名等操作。

### 7.5.2 重点保护文件

```text
.env
.env.local
.env.production
*.pem
id_rsa
id_ed25519
package.json
pom.xml
docker-compose.yml
nginx.conf
application.yml
settings.xml
.git/config
```

### 7.5.3 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| FILE-001 | 支持记录文件读取行为 | P1 |
| FILE-002 | 支持记录文件写入行为 | P0 |
| FILE-003 | 支持记录文件删除行为 | P0 |
| FILE-004 | 支持记录文件重命名行为 | P1 |
| FILE-005 | 支持生成文件 diff | P1 |
| FILE-006 | 支持敏感文件保护 | P0 |
| FILE-007 | 支持项目目录访问限制 | P0 |
| FILE-008 | 支持文件变更快照 | P2 |
| FILE-009 | 支持恢复被修改文件 | P2 |

---

## 7.6 数据库保护模块

### 7.6.1 功能描述

系统应支持对数据库操作进行风险识别，重点拦截危险 SQL。

### 7.6.2 高危 SQL

```sql
DROP TABLE users;
DROP DATABASE app;
TRUNCATE TABLE logs;
DELETE FROM users;
UPDATE users SET role = 'admin';
ALTER TABLE users DROP COLUMN password;
```

### 7.6.3 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| DB-001 | 支持识别 DROP TABLE | P1 |
| DB-002 | 支持识别 DROP DATABASE | P1 |
| DB-003 | 支持识别 TRUNCATE TABLE | P1 |
| DB-004 | 支持识别无 WHERE 的 DELETE | P1 |
| DB-005 | 支持识别无 WHERE 的 UPDATE | P1 |
| DB-006 | 支持识别 ALTER TABLE DROP COLUMN | P2 |
| DB-007 | 支持生成 SQL 风险说明 | P1 |
| DB-008 | 支持数据库操作确认 | P1 |
| DB-009 | 支持生成回滚建议 | P2 |

---

## 7.7 MCP Server 权限管理模块

### 7.7.1 功能描述

系统应允许用户为不同 MCP Server 配置不同权限等级。

### 7.7.2 权限等级

| 等级 | 名称 | 描述 |
|---|---|---|
| Level 0 | Blocked | 完全禁用 |
| Level 1 | Read Only | 只读 |
| Level 2 | Confirm Write | 写入前确认 |
| Level 3 | Trusted | 低风险自动放行 |
| Level 4 | Sandboxed | 只能在沙箱执行 |
| Level 5 | Admin | 全部放行但记录日志 |

### 7.7.3 配置示例

```yaml
servers:
  github:
    trust_level: 2
    allow_tools:
      - get_file_contents
      - search_repositories
    confirm_tools:
      - create_issue
      - create_pull_request
    block_tools:
      - delete_repository

  filesystem:
    trust_level: 1
    allowed_paths:
      - "./src"
      - "./docs"
    blocked_paths:
      - ".env"
      - "~/.ssh"
      - "/etc"
```

### 7.7.4 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| PERM-001 | 支持 MCP Server 权限等级 | P0 |
| PERM-002 | 支持工具白名单 | P0 |
| PERM-003 | 支持工具黑名单 | P0 |
| PERM-004 | 支持路径白名单 | P0 |
| PERM-005 | 支持路径黑名单 | P0 |
| PERM-006 | 支持写操作确认 | P0 |
| PERM-007 | 支持只读模式 | P0 |
| PERM-008 | 支持权限模板 | P1 |

---

## 7.8 用户确认模块

### 7.8.1 功能描述

当 AI Agent 执行高危操作时，系统应要求用户确认。

### 7.8.2 确认方式

- CLI 终端确认
- 桌面端弹窗确认
- Web 面板确认
- 配置文件自动策略确认

### 7.8.3 确认选项

```text
允许一次
始终允许
拒绝
加入黑名单
加入白名单
查看详情
```

### 7.8.4 CLI 示例

```text
AgentShield Approval Required

Source:
Codex CLI

Command:
rm -rf dist

Risk:
High 82/100

Reason:
Recursive delete operation.

Approve?
[y] allow once
[a] always allow
[n] deny
[b] block forever
```

### 7.8.5 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| APPROVAL-001 | 支持 CLI 确认 | P0 |
| APPROVAL-002 | 支持桌面端弹窗确认 | P1 |
| APPROVAL-003 | 支持允许一次 | P0 |
| APPROVAL-004 | 支持始终允许 | P1 |
| APPROVAL-005 | 支持拒绝 | P0 |
| APPROVAL-006 | 支持加入黑名单 | P1 |
| APPROVAL-007 | 支持确认超时 | P1 |
| APPROVAL-008 | 支持记录用户确认结果 | P0 |

---

## 7.9 审计日志模块

### 7.9.1 功能描述

系统需要完整记录 AI Agent 的工具调用行为。

### 7.9.2 存储方式

- SQLite
- JSONL
- Markdown 报告
- HTML 报告

### 7.9.3 数据表设计

```sql
CREATE TABLE security_events (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  agent_name TEXT,
  client_name TEXT,
  event_type TEXT,
  target TEXT,
  arguments_json TEXT,
  result_json TEXT,
  risk_score INTEGER,
  risk_level TEXT,
  decision TEXT,
  reason TEXT,
  created_at TEXT
);

CREATE TABLE file_changes (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  path TEXT,
  operation TEXT,
  diff TEXT,
  risk_score INTEGER,
  created_at TEXT
);

CREATE TABLE approvals (
  id TEXT PRIMARY KEY,
  event_id TEXT,
  user_decision TEXT,
  remember BOOLEAN,
  created_at TEXT
);

CREATE TABLE mcp_servers (
  id TEXT PRIMARY KEY,
  name TEXT,
  command TEXT,
  args_json TEXT,
  env_json TEXT,
  trust_level INTEGER,
  enabled BOOLEAN
);
```

### 7.9.4 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| AUDIT-001 | 支持记录安全事件 | P0 |
| AUDIT-002 | 支持记录 MCP 工具调用 | P0 |
| AUDIT-003 | 支持记录命令执行 | P0 |
| AUDIT-004 | 支持记录文件变更 | P1 |
| AUDIT-005 | 支持记录用户确认行为 | P0 |
| AUDIT-006 | 支持按时间查询 | P1 |
| AUDIT-007 | 支持按风险等级查询 | P1 |
| AUDIT-008 | 支持导出 JSON | P0 |
| AUDIT-009 | 支持导出 Markdown | P1 |
| AUDIT-010 | 支持导出 HTML | P2 |

---

## 7.10 审计报告模块

### 7.10.1 功能描述

系统应支持根据审计日志生成报告。

### 7.10.2 报告内容

```text
项目名称
会话时间
AI Client 名称
MCP Server 列表
工具调用总数
命令执行总数
文件修改总数
高危操作数量
被阻止操作数量
用户确认操作数量
风险最高的 10 个事件
被访问的敏感文件
策略命中情况
安全建议
```

### 7.10.3 报告示例

```md
# AgentShield Audit Report

Project: fullstack-demo
Agent: Codex CLI
Time: 2026-06-19 20:31

## Summary

- Total tool calls: 42
- File reads: 18
- File writes: 7
- Shell commands: 11
- Blocked actions: 3
- High-risk actions: 5
```

### 7.10.4 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| REPORT-001 | 支持生成 Markdown 报告 | P1 |
| REPORT-002 | 支持生成 HTML 报告 | P2 |
| REPORT-003 | 支持生成 JSON 报告 | P0 |
| REPORT-004 | 支持风险汇总 | P0 |
| REPORT-005 | 支持高危事件排行 | P1 |
| REPORT-006 | 支持安全建议 | P1 |
| REPORT-007 | 支持按项目生成报告 | P1 |
| REPORT-008 | 支持按会话生成报告 | P1 |

---

## 7.11 CLI 模块

### 7.11.1 功能描述

提供命令行工具，方便开发者初始化项目、启动代理、管理 MCP Server、查看审计日志。

### 7.11.2 命令设计

```bash
agentshield init

agentshield mcp add github \
  --command github-mcp-server

agentshield mcp list

agentshield proxy start

agentshield audit list

agentshield report generate

agentshield policy test

agentshield demo
```

### 7.11.3 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| CLI-001 | 支持初始化配置 | P0 |
| CLI-002 | 支持添加 MCP Server | P0 |
| CLI-003 | 支持查看 MCP Server | P0 |
| CLI-004 | 支持启动 MCP Proxy | P0 |
| CLI-005 | 支持查看审计日志 | P1 |
| CLI-006 | 支持生成报告 | P1 |
| CLI-007 | 支持测试策略规则 | P1 |
| CLI-008 | 支持运行 Demo | P0 |
| CLI-009 | 支持版本查看 | P0 |
| CLI-010 | 支持配置校验 | P1 |

---

## 7.12 桌面端监控面板

### 7.12.1 功能描述

提供桌面端可视化界面，用于展示实时事件、风险分布、MCP Server 权限、策略配置和审计报告。

### 7.12.2 页面设计

```text
Dashboard        总览
Live Events      实时事件
MCP Servers      MCP Server 管理
Policies         策略管理
Approvals        待确认操作
Audit Logs       审计日志
Reports          报告导出
Settings         系统设置
```

### 7.12.3 Dashboard 指标

```text
今日工具调用次数
今日高危操作次数
被阻止次数
活跃 MCP Server 数量
最近 10 条风险事件
风险等级分布
```

### 7.12.4 需求列表

| 编号 | 需求 | 优先级 |
|---|---|---|
| DESKTOP-001 | 支持实时事件展示 | P1 |
| DESKTOP-002 | 支持风险统计看板 | P1 |
| DESKTOP-003 | 支持 MCP Server 管理 | P1 |
| DESKTOP-004 | 支持策略编辑 | P2 |
| DESKTOP-005 | 支持弹窗确认 | P1 |
| DESKTOP-006 | 支持审计日志查询 | P1 |
| DESKTOP-007 | 支持报告导出 | P2 |
| DESKTOP-008 | 支持暗色模式 | P2 |

---

## 8. 非功能性需求

### 8.1 性能需求

| 编号 | 需求 |
|---|---|
| NF-001 | 单次 MCP 工具调用代理延迟应尽量控制在 100ms 内，不包含真实工具执行时间 |
| NF-002 | 审计日志写入不能明显阻塞工具调用 |
| NF-003 | 桌面端实时事件刷新延迟应低于 1 秒 |
| NF-004 | 支持单会话至少 10,000 条事件记录 |
| NF-005 | 支持本地 SQLite 数据自动归档 |

### 8.2 安全需求

| 编号 | 需求 |
|---|---|
| SEC-001 | 默认本地运行，不上传用户数据 |
| SEC-002 | 不明文展示敏感 Token |
| SEC-003 | 审计日志中敏感字段需要脱敏 |
| SEC-004 | 默认阻止读取 `.env`、SSH 私钥等敏感文件 |
| SEC-005 | 默认阻止 `curl | bash` 等远程脚本执行 |
| SEC-006 | 配置文件权限应限制为当前用户可读写 |
| SEC-007 | 支持敏感字段匹配与脱敏 |

### 8.3 兼容性需求

| 编号 | 需求 |
|---|---|
| COMP-001 | 支持 Windows |
| COMP-002 | 支持 macOS |
| COMP-003 | 支持 Linux |
| COMP-005 | 支持 Cursor |
| COMP-006 | 支持 Codex CLI |
| COMP-007 | 支持 Gemini CLI |
| COMP-008 | 支持常见 MCP Server |
| COMP-009 | 支持 stdio 模式 MCP |
| COMP-010 | 支持 HTTP 类 MCP 传输方式 |

### 8.4 可维护性需求

| 编号 | 需求 |
|---|---|
| MAINT-001 | 规则库应独立维护 |
| MAINT-002 | 核心模块应支持单元测试 |
| MAINT-003 | 每条内置规则应有测试用例 |
| MAINT-004 | CLI 命令应提供帮助说明 |
| MAINT-005 | 文档应包含快速开始、架构说明、策略说明 |
| MAINT-006 | 支持插件化扩展新的风险规则 |

---

## 9. 推荐技术选型

### 9.1 核心层

| 模块 | 推荐技术 |
|---|---|
| 核心代理 | Rust / Go |
| MCP 适配 | TypeScript / Rust |
| CLI | Rust clap / Go cobra |
| 策略解析 | YAML / JSON |
| 审计存储 | SQLite + JSONL |
| 实时通信 | WebSocket |
| 桌面端 | Tauri + Vue 3 |
| UI 组件 | Ant Design Vue |
| 图表 | ECharts |
| 打包 | GitHub Actions |

### 9.2 推荐理由

- Rust / Go 适合开发本地代理、CLI 和跨平台二进制工具。
- Tauri 适合开发轻量桌面端。
- SQLite 适合本地审计数据存储，不需要用户安装数据库。
- YAML 策略对开发者友好，方便贡献规则。
- Vue 3 + Ant Design Vue 适合快速做出好看的桌面管理界面。

---

## 10. 推荐项目目录

```text
agentshield-mcp
├── README.md
├── LICENSE
├── SECURITY.md
├── CONTRIBUTING.md
├── docs
│   ├── quick-start.md
│   ├── architecture.md
│   ├── policy.md
│   ├── mcp-proxy.md
│   ├── risk-engine.md
│   └── examples.md
├── crates
│   ├── agentshield-core
│   ├── agentshield-cli
│   ├── agentshield-proxy
│   ├── agentshield-policy
│   ├── agentshield-risk
│   └── agentshield-audit
├── desktop
│   ├── src
│   ├── src-tauri
│   └── package.json
├── packages
│   ├── mcp-adapter
│   ├── policy-sdk
│   └── shared-types
├── examples
│   ├── github-mcp
│   ├── filesystem-mcp
│   ├── shell-guard
│   └── dangerous-demo
├── policies
│   ├── default.yaml
│   ├── strict.yaml
│   ├── read-only.yaml
│   └── enterprise.yaml
└── tests
    ├── fixtures
    ├── policy-tests
    ├── mcp-proxy-tests
    └── risk-tests
```

---

## 11. MVP 版本需求

### 11.1 MVP 目标

第一版目标不是做全，而是做出最能证明价值的版本：

```text
AI Client → AgentShield MCP Proxy → Real MCP Server
```

并且可以拦截危险工具调用。

### 11.2 MVP 必做功能

| 编号 | 功能 | 优先级 |
|---|---|---|
| MVP-001 | MCP Proxy 基础转发 | P0 |
| MVP-002 | 拦截 `tools/call` | P0 |
| MVP-003 | 内置风险规则 | P0 |
| MVP-004 | 风险评分 | P0 |
| MVP-005 | CLI 确认 | P0 |
| MVP-006 | SQLite / JSONL 审计日志 | P0 |
| MVP-007 | 一键 Demo | P0 |
| MVP-008 | README 使用文档 | P0 |
| MVP-009 | GitHub Topics 和项目介绍 | P0 |
| MVP-010 | 示例策略文件 | P0 |

### 11.3 MVP 暂不实现

```text
桌面端
数据库代理
复杂沙箱
企业用户系统
云端同步
插件市场
VS Code 插件
```

### 11.4 MVP 验收标准

- 可以通过命令启动 AgentShield MCP Proxy。
- AI Client 可以连接 AgentShield。
- AgentShield 可以正常转发 MCP 工具列表。
- 调用危险工具时可以触发风险评分。
- 高危操作可以被 CLI 确认或阻止。
- 所有工具调用都可以写入审计日志。
- 可以运行 `agentshield demo` 展示拦截效果。

---

## 12. 版本规划

### v0.1：MCP Proxy 版本

```text
MCP Proxy
tools/list 转发
tools/call 拦截
风险评分
CLI 确认
JSONL 审计
Demo 示例
```

### v0.2：策略引擎版本

```text
YAML Policy
规则测试
工具白名单
工具黑名单
路径白名单
路径黑名单
权限等级
```

### v0.3：桌面端版本

```text
Tauri 桌面端
实时事件流
风险看板
弹窗确认
MCP Server 管理
审计日志查看
```

### v0.4：Shell Guard 版本

```text
agentshield exec
危险命令识别
命令执行记录
命令确认
命令黑名单
```

### v0.5：File Guard 版本

```text
文件变更监听
敏感文件保护
文件 diff
删除文件拦截
```

### v1.0：完整开源版本

```text
MCP Proxy
Shell Guard
File Guard
Risk Engine
Policy Engine
Audit Report
Desktop Dashboard
多客户端配置模板
GitHub Action
插件系统
```

---

## 13. 内置风险规则

### 13.1 Shell 规则

| 规则 | 风险等级 | 默认动作 |
|---|---|---|
| `rm -rf` | High | Confirm |
| `curl | bash` | Critical | Block |
| `wget | sh` | Critical | Block |
| `chmod -R 777` | High | Confirm |
| `sudo` | Medium | Confirm |
| `docker rm` | High | Confirm |
| `docker volume rm` | Critical | Confirm |
| `kubectl delete` | High | Confirm |
| `git push --force` | High | Confirm |

### 13.2 文件规则

| 文件 | 风险等级 | 默认动作 |
|---|---|---|
| `.env` | Critical | Block |
| `.env.local` | Critical | Block |
| `id_rsa` | Critical | Block |
| `id_ed25519` | Critical | Block |
| `*.pem` | Critical | Block |
| `package.json` | Medium | Log |
| `pom.xml` | Medium | Log |
| `docker-compose.yml` | High | Confirm |
| `nginx.conf` | High | Confirm |

### 13.3 数据库规则

| SQL | 风险等级 | 默认动作 |
|---|---|---|
| `DROP TABLE` | Critical | Confirm |
| `DROP DATABASE` | Critical | Block |
| `TRUNCATE TABLE` | Critical | Confirm |
| `DELETE` 无 WHERE | High | Confirm |
| `UPDATE` 无 WHERE | High | Confirm |
| `ALTER TABLE DROP COLUMN` | High | Confirm |

---

## 14. 安装与使用需求

### 14.1 安装方式

项目应支持以下安装方式：

```bash
npm install -g agentshield
```

或：

```bash
cargo install agentshield
```

或下载二进制文件：

```text
Windows: agentshield.exe
macOS: agentshield
Linux: agentshield
```

### 14.2 初始化

```bash
agentshield init
```

生成：

```text
.agentshield
├── config.yaml
├── policy.yaml
└── audit.db
```

### 14.3 启动代理

```bash
agentshield proxy start
```

### 14.4 添加 MCP Server

```bash
agentshield mcp add github \
  --command github-mcp-server
```

### 14.5 生成报告

```bash
agentshield report generate
```

---

## 15. 开源文档需求

项目至少需要包含以下文档：

```text
README.md
docs/quick-start.md
docs/architecture.md
docs/policy.md
docs/risk-engine.md
docs/mcp-proxy.md
docs/client-config.md
docs/examples.md
docs/faq.md
SECURITY.md
CONTRIBUTING.md
CHANGELOG.md
```

README 必须包含：

- 项目介绍
- 项目架构图
- 快速开始
- 安装方式
- Demo GIF
- 支持的 AI Client
- 支持的 MCP Server
- 风险规则示例
- 审计报告示例
- Roadmap
- License

---

## 16. GitHub 开源运营需求

### 16.1 仓库名称

```text
agentshield-mcp
```

### 16.2 仓库描述

```text
A firewall and audit system for MCP tools, AI agents, and coding assistants.
```

### 16.3 GitHub Topics

```text
mcp
model-context-protocol
ai-agent
ai-security
coding-agent
codex
cursor
gemini-cli
developer-tools
security-tools
mcp-server
agent-safety
```

### 16.4 README 展示 Demo

建议 README 首页展示以下 Demo：

```text
Demo 1：AI 尝试读取 .env，被阻止
Demo 2：AI 尝试 rm -rf，被要求确认
Demo 3：AI 调用 GitHub MCP 创建 PR，被记录
Demo 4：AI 尝试 DROP TABLE，被标记 critical
Demo 5：生成完整审计报告
```

---

## 17. 验收标准总表

| 模块 | 验收标准 |
|---|---|
| MCP Proxy | 可以连接 AI Client 和真实 MCP Server |
| 工具拦截 | 可以拦截 `tools/call` |
| 风险评分 | 每次调用都有风险分数和风险原因 |
| 策略引擎 | 可以通过 YAML 控制 allow / confirm / block |
| 用户确认 | 高危操作可以在 CLI 中确认 |
| 审计日志 | 所有工具调用都能记录到本地 |
| 报告生成 | 可以导出 JSON / Markdown 报告 |
| 权限管理 | 可以配置 MCP Server 权限等级 |
| CLI | 可以完成初始化、启动代理、查看日志 |
| Demo | 可以一键演示危险操作拦截 |

---

## 18. 项目宣传文案

### 18.1 英文版

```text
AgentShield MCP is a local-first firewall for AI agents and MCP tools.

It sits between your coding assistant and your tools, monitors every tool call, scores risky actions, asks for approval before dangerous operations, and generates a complete audit report of what your AI agent did.
```

### 18.2 中文版

```text
AgentShield MCP 是一个本地优先的 AI Agent 安全防火墙。

它位于 Cursor、Codex、Gemini CLI 和 MCP Server 之间，实时监控 AI 的工具调用，对危险操作进行风险评分，在执行删除文件、运行命令、调用接口、修改数据库之前弹出确认，并生成完整审计报告。
```

---

## 19. 总结

AgentShield MCP 的核心定位是：

```text
Runtime security firewall for AI agents and MCP tools.
```

它的价值不是事后扫描，而是在 AI Agent 执行危险操作之前进行运行时拦截。

项目核心能力可以总结为：

```text
Watch：看见 AI 做了什么
Stop：阻止 AI 做危险操作
Prove：证明 AI 做过什么
```

优先开发顺序建议：

```text
1. MCP Proxy
2. 风险评分
3. CLI 确认
4. 审计日志
5. 一键 Demo
6. 策略引擎
7. 桌面端
8. Shell Guard
9. File Guard
10. DB Guard
```

第一版不要追求大而全，先做出能够演示价值的 MVP：

```text
AI Client → AgentShield MCP Proxy → Real MCP Server
```

只要能够成功拦截 `.env` 读取、`rm -rf`、`curl | bash`、危险 GitHub 写操作，并生成审计日志，这个项目就已经具备开源传播价值。
