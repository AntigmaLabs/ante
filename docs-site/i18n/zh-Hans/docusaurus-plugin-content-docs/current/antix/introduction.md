---
title: "简介"
description: "Antix 是由 Antigma 构建的 LLM 代理、身份提供商（IdP）和组织管理器。"
sidebar_position: 1
---

# Antix {#antix}

**Antix** 是 Antigma 的 LLM 代理、身份提供商和组织管理器——它是让 AI 为团队提供可扩展、安全且可靠体验的协作后端。

虽然 Ante 为您的本地终端带来了自主 AI 能力，但 Antix 是控制平面：一个统一的网关，可以跨多个网络协议路由模型、管理组织、签发具有预算上限的密钥，并追踪您公司的 AI 支出。

### 核心功能 {#key-features}

<CardGroup cols={3}>
  <Card title="多协议网关" icon="route">
    在相同的基地址上支持 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 和 Gemini 原生协议。通过一行配置即可将任何现有 SDK 指向 Antix。
  </Card>
  <Card title="带有硬预算的虚拟密钥" icon="shield-halved">
    签发具有原子级 `max_budget` 上限且限定范围的 `sk-antix-…` 密钥。严格的执行会在调用上游之前阻断超支。
  </Card>
  <Card title="Ante 控制平面" icon="plug">
    针对本地编码代理的治理。启动 `ante`，输入 `/connect`，并选择 Antix 以安全地验证您的本地代理，记录每个提示的归属。
  </Card>
  <Card title="自带密钥（BYOK）" icon="key">
    在 `Authorization` 中发送您自己的提供商凭据，并通过 `X-Antix-Provider` 声明提供商。Antix 进行路由和计量，且不会重新计费。
  </Card>
</CardGroup>

### 为什么选择 Antix？ {#why-antix}

- **默认故障关闭（Fail-closed by default）。** 如果无法访问计费后端，Antix 拒绝提供服务。
- **高性能热路径（High-performance hot path）。** 流式管道将跨提供商的 SSE 规范化，并保证在并发负载下原子化地执行预算。
- **从第一天起即为多租户设计。** 组织、RBAC（`admin` / `member`）以及带范围限制的虚拟密钥——这些都不是后续强行添加的。
- **在数据保留上诚实透明。** Antix 为成本归属和管理员分析而持久化请求与响应主体——请参阅[隐私、安全与数据保留](/antix/concepts/security)。

### 后续步骤 {#next-steps}

<CardGroup cols={2}>
  <Card title="快速入门" icon="rocket" href="/antix/quickstart">
    在 5 分钟内向 Antix 代理发出您的第一个请求。
  </Card>
  <Card title="路由与 BYOK" icon="route" href="/antix/concepts/routing">
    端点、SDK 兼容性和提供商覆盖。
  </Card>
  <Card title="虚拟密钥与预算" icon="shield-halved" href="/antix/concepts/virtual-keys">
    发放带有硬性支出上限的、作用域受限的密钥。
  </Card>
  <Card title="组织与 RBAC" icon="users" href="/antix/concepts/organizations">
    管理成员、分配角色，并在您的组织内限定访问范围。
  </Card>
  <Card title="错误处理" icon="circle-exclamation" href="/antix/concepts/error-handling">
    跨提供商的标准错误代码。
  </Card>
</CardGroup>
