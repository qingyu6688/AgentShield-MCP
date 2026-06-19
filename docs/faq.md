# 常见问题

## 它会拖慢我的 AI 吗？

代理层自身的开销很小——解析、评分、策略匹配都是内存里的纯计算，目标是单次 100ms 以内（不含真实工具执行时间）。审计写入是异步的，不阻塞转发。真正的耗时还是真实工具本身的执行时间，那部分 AgentShield 不碰。

## 我的代码和密钥会被上传吗？

不会。AgentShield 默认**完全本地运行**，不向任何外部服务发数据。审计存在本机的 SQLite / JSONL 里。敏感字段在落盘前还会脱敏。

## 它能 100% 防住恶意操作吗？

不能，也不该这么宣传。它防的是“AI Agent 经 MCP 发起的工具调用”这一层。绕过 MCP 的本地进程、用户手动敲的命令、已被放行的恶意 server 自身行为，都不在范围内。它是**降低风险的运行时防火墙**，不是操作系统级安全软件，不替代杀毒 / EDR / 密钥管理 / 数据库备份。

## 和普通的权限确认（比如客户端自带的确认）有什么区别？

客户端自带确认通常是“要不要执行这个工具”的笼统提示。AgentShield 多了三样：**统一的风险评分**（跨工具、跨 server 一致的标准）、**可编程的策略**（YAML 规则，可版本化、可团队共享）、**完整审计**（事后能证明 AI 做过什么）。而且它对客户端无侵入，换客户端规则不变。

## 误报太多/拦得太狠怎么办？

编辑 `.agentshield/policy.yaml`：把过严的规则改成 `confirm` 或 `allow`，或加 `override: true` 的放行规则。也可以直接换更宽松的预设（`policies/default.yaml`）。改完即时生效，支持热更新。

## 无人值守（CI、自动化）场景下没法交互确认怎么办？

把需要交互的 `confirm` 规则改成明确的 `allow` 或 `block`。`confirm` 本质上需要人在终端，CI 里应当用确定性策略，避免卡住。

## 确认提示出现在哪里？

出现在 `agentshield proxy start` 所在的终端。如果客户端在后台拉起 proxy（你看不到那个终端），用桌面端确认（v0.3 起），或者把策略改成不依赖交互。

## 支持哪些 AI 客户端？

任何走标准 MCP 协议的客户端：Cursor、Codex CLI、Gemini CLI 等。接入方式见 [client-config.md](client-config.md)。

## 支持哪些 MCP Server？

stdio 传输的 MCP server 都支持（MVP）。SSE / Streamable HTTP 在规划中。常见的 filesystem、github、shell 类 server 还做了工具名精确映射，评分更准。

## 为什么用 Rust？

需要的是一个低延迟、跨平台、单文件分发的本地代理和 CLM。Rust 在这几点上很合适：无 GC 的稳定低延迟、跨平台静态二进制、强类型让安全相关逻辑不容易写错。

## 它自己安全吗？

我们对它自己的安全很认真：默认本地、敏感字段脱敏、配置文件限当前用户权限、库代码禁用 `unwrap`/`unsafe`。发现漏洞请走 [SECURITY.md](../SECURITY.md) 的私密渠道。

## 怎么彻底卸载/停用？

把客户端 MCP 配置改回直连真实 server，删掉项目里的 `.agentshield/` 目录即可。它不改客户端以外的系统设置。

## 数据库保护是怎么生效的？

如果 AI 通过某个数据库类 MCP server 执行 SQL，AgentShield 会把它识别为 `db.query`，对破坏性语句（`DROP` / `TRUNCATE` / 无 WHERE 的 `DELETE`/`UPDATE`）评分并拦截。它不直接连你的数据库，只看流经 MCP 的 SQL。
