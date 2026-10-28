---
title: "Ante 控制平面"
description: "通过 PKCE、归属与撤销来管控本地编码代理。"
sidebar_position: 1
---

# 原生代理集成 {#native-agent-integration}

Antix 是 **Ante CLI** 的集中式控制平面，为本地编码代理带来企业级的管理能力。

## 将 Ante 连接到 Antix {#connecting-ante-to-antix}

开发人员将他们的本地 Ante 实例验证连接到您的 Antix 服务器：

```bash
ante auth login antix
```

这会启动 Antix OAuth 流程，但不会启动代理会话。在远程机器上，使用 `ante auth login antix --no-browser`，Ante 会打印授权 URL 而不是打开浏览器。

也可以在 TUI 中登录：

1. 启动 `ante` CLI。
2. 输入 `/providers`。
3. 选中 **Antix** 并按 `m`，然后选择 **Sign in to Antix**。

这会将您的本地代理安全地验证到 Antix 控制平面，授予它一个在需要时自动刷新的临时访问令牌。

## 优势 {#benefits}

- **成本归属** — 工程师在 Ante CLI 中运行的每个提示（prompt）都会在计费分类账和分析时间线上归属到其用户 ID。
- **模型治理** — 通过组织级别的默认模型设置，限制本地代理可以使用的模型。
- **部分离职（Partial offboarding）** — 在门户网站中移除成员会将其个人端点归档，几秒内即可切断通过这些 URL 的流量（请参阅[组织](/antix/concepts/organizations)）。这**不会**撤销成员的虚拟密钥或活动的 OAuth 会话——如需彻底切断，请撤销其密钥或删除其账户。
