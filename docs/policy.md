# 策略配置

策略引擎让你用一份 YAML 决定每类工具调用是放行、记录、确认、阻止还是进沙箱。规则面向开发者，方便阅读和贡献。

## 1. 文件位置

`agentshield init` 生成 `.agentshield/policy.yaml`。项目也提供几套预设：

| 文件 | 适用场景 |
|---|---|
| `policies/default.yaml` | 默认，安全与可用平衡 |
| `policies/strict.yaml` | 严格，多数写操作都要确认 |
| `policies/read-only.yaml` | 只读，禁止一切写/删/执行 |
| `policies/enterprise.yaml` | 团队，按 server 分权限等级 |

## 2. 动作

| 动作 | 含义 |
|---|---|
| `allow` | 直接放行 |
| `log` | 放行但记录 |
| `confirm` | 执行前需要用户确认 |
| `block` | 阻止执行 |
| `sandbox` | 在沙箱中执行（P2，规划中） |

## 3. 基本结构

```yaml
version: 1

# 没有任何规则命中时的兜底动作
default_action: allow

rules:
  - name: block-env-read
    description: 阻止读取环境变量文件
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

## 4. match 匹配条件

一个规则的 `match` 里可组合多个条件，**全部满足才算命中**（AND 关系）。

| 字段 | 作用 | 支持的匹配方式 |
|---|---|---|
| `type` | 事件类型 | 精确，如 `file.read`、`shell.exec`、`db.query` |
| `tool` | 工具名 | `equals` / `contains` / `regex` |
| `server` | 来源 MCP server 名 | `equals` / `in` |
| `path` | 目标路径 | `contains` / `regex` / `glob` |
| `command` | 命令内容 | `contains` / `regex` |
| `sql` | SQL 语句 | `contains` / `regex` |

字符串匹配子结构：

```yaml
path:
  contains: [".env", ".pem"]   # 命中任一即可
  regex: "\\.ssh/"             # 或正则
  glob: "**/secrets/*"         # 或 glob
```

## 5. 优先级与合成

- 规则**按文件顺序**匹配，第一条命中的规则给出 `action`。
- 该 action 再与**风险引擎的建议动作**合成，**默认取更严格者**（block > confirm > log > allow）。
- 若想让某规则强制放行（覆盖风险引擎），加 `override: true`：

```yaml
  - name: allow-trusted-read
    match:
      type: file.read
      path:
        glob: "./src/**"
    action: allow
    override: true   # 即使风险引擎想 confirm，也强制放行
```

严格程度排序：`block > confirm > log > allow`。`sandbox` 视为介于 confirm 和 block 之间。

## 6. MCP Server 权限等级

除了规则，还能给每个 server 配一个整体权限等级（在 `config.yaml`）：

| 等级 | 名称 | 描述 |
|---|---|---|
| 0 | Blocked | 完全禁用 |
| 1 | Read Only | 只读 |
| 2 | Confirm Write | 写入前确认 |
| 3 | Trusted | 低风险自动放行 |
| 4 | Sandboxed | 只能在沙箱执行 |
| 5 | Admin | 全部放行但记录日志 |

配合工具/路径白黑名单：

```yaml
servers:
  github:
    trust_level: 2
    allow_tools: [get_file_contents, search_repositories]
    confirm_tools: [create_issue, create_pull_request]
    block_tools: [delete_repository]

  filesystem:
    trust_level: 1
    allowed_paths: ["./src", "./docs"]
    blocked_paths: [".env", "~/.ssh", "/etc"]
```

权限等级先于规则生效：等级 0 直接 block，等级 1 对写/删直接拒，再轮到具体规则。

## 7. 测试与热更新

```bash
# 用一条模拟调用测试当前策略会怎么裁决
agentshield policy test \
  --type shell.exec \
  --command "rm -rf /"

# 校验策略文件语法
agentshield config check
```

策略文件支持热更新（POLICY-009）：proxy 运行时修改 `policy.yaml` 会被自动重新加载，无需重启。

## 8. 相关需求

覆盖 7.3（POLICY-001 ~ POLICY-010）与 7.7 权限管理。详细设计见 [docs/design/03-policy.md](design/03-policy.md)。
