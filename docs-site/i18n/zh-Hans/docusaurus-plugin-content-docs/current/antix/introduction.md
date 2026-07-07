---
title: "简介"
description: "Antix 是 Antigma 构建的 LLM 代理、身份提供商 (IdP) 和组织管理器。"
sidebar_position: 1
---

# Antix

**Antix** 是 Antigma 的 LLM 代理、身份提供商和组织管理器——一个为团队提供可扩展、安全且可靠的人工智能协作后端。

虽然 [Ante](/) 为您的本地终端提供了自主的 AI 能力，但 Antix 是控制平面：一个统一的网关，它跨多个网络协议路由模型、管理组织、签发具有预算上限的密钥，并追踪整个公司的 AI 支出。

### 核心功能

<CardGroup cols={3}>
  <Card title="多协议网关" icon="route">
    在同一个基础 URL 上支持 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 和 Gemini 原生协议。只需一行配置即可将任何现有的 SDK 指向 Antix。
  </Card>
  <Card title="硬性预算虚拟密钥" icon="shield-halved">
    签发带有原子级 `max_budget`（最大预算）上限且有作用域的 `sk-antix-…` 密钥。严格的执行机制会在调用上游之前阻止超额。
  </Card>
  <Card title="Ante 控制平面" icon="plug">
    针对本地编程智能体的治理。启动 `ante`，输入 `/connect`，并选择 Antix 来安全地验证您的本地智能体，将每次提示归属到位。
  </Card>
  <Card title="自带密钥 (BYOK)" icon="key">
    在 `Authorization` 中发送您自己的提供商凭据，并使用 `X-Antix-Provider` 声明提供商。Antix 仅进行路由和计量，无需重复计费。
  </Card>
</CardGroup>

### 为什么选择 Antix？

- **默认故障关闭。** 如果计费后端不可达，Antix 会拒绝处理流量。
- **高性能热路径。** 流水线规范化了跨提供商的 SSE，并在并发负载下保证原子级预算执行。
- **从第一天起就支持多租户。** 组织、基于角色的访问控制 (RBAC) (`admin` / `member`) 和具有作用域的虚拟密钥——这并非事后添加的功能。
- **关于留存的坦诚。** Antix 会持久化请求和响应的正文，以用于成本归因和管理员分析——参见 [隐私、安全与数据留存](/antix/concepts/security)。

### 下一步

<CardGroup cols={2}>
  <Card title="快速入门" icon="rocket" href="/antix/quickstart">
    在5分钟内向 Antix 代理发起您的第一个请求。
  </Card>
  <Card title="路由和自带密钥 (BYOK)" icon="route" href="/antix/concepts/routing">
    端点、SDK 兼容性以及提供商覆盖。
  </Card>
  <Card title="虚拟密钥和预算" icon="shield-halved" href="/antix/concepts/virtual-keys">
    签发具有硬性支出上限的、有作用域的密钥。
  </Card>
  <Card title="组织和 RBAC" icon="users" href="/antix/concepts/organizations">
    管理成员、分配角色并在您的组织内划定访问范围。
  </Card>
  <Card title="错误处理" icon="circle-exclamation" href="/antix/concepts/error-handling">
    跨提供商的标准化错误代码。
  </Card>
</CardGroup>
