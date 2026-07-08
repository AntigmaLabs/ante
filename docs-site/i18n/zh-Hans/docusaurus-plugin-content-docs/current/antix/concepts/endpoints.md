---
title: "端点"
description: "稳定的按应用分配的 URL，用于为支出、跟踪和代理会话标记流量。"
sidebar_position: 4
---

# 端点 {#endpoints}

**端点（Endpoint）** 是一个稳定且唯一的 URL（例如，`https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>/...`），它作为提供商基地址（base URL）的直接替代品。通过它流转的每个请求都会自动打上端点 ID 的标签，让您可以在应用层面掌握以下信息：

- **支出与使用量** — 每个端点的代币计数和资金成本。
- **请求跟踪** — 完整的请求/响应生命周期，包含延迟以及每次调用的成本。
- **代理会话** — 将多轮请求分组为连贯的会话以便调试。

端点独立于身份验证：它们决定请求将降落在*何处*以及*如何标记*请求，而[虚拟密钥](/antix/concepts/virtual-keys)（或自带密钥，BYOK）决定*谁付费*以及*适用哪些限制*。

## 创建端点 {#creating-an-endpoint}

端点是从 [Antix 门户](https://portal.antigma.ai) 的 **Endpoints（端点）** 选项卡创建的。

### 范围：个人 vs. 组织 {#scope-personal-vs-organization}

创建端点时，您可以选择其范围：

- **个人（Personal）** — 仅对您可见。适用于本地开发或实验。每个组织中**每个用户的个人端点上限为 10 个**。
- **组织共享（Org-shared）** — 对组织的所有成员可见。只有**组织管理员（Admins）**才能创建组织共享端点。组织共享端点共享一个独立的、更大的上限（默认 50 个），该上限按组织配置。

### 步骤 {#steps}

1. 在门户中打开 **Endpoints（端点）** 选项卡。
2. 在 **Create Endpoint（创建端点）** 卡片中，选择端点所属的组织。
3. 选择 **Personal（个人）** 或 **Org-shared（组织共享）**。
4. 输入 **Display Name（显示名称）**（例如，`CI Runner`，`Frontend Production`）。
5. 点击 **Create（创建）**。

:::note
显示名称可以稍后编辑。生成的 URL（嵌入了一个 UUID）**在端点的整个生命周期内固定不变**，且永远不会更改。
:::

## 使用您的端点 URL {#using-your-endpoint-url}

点击进入一个端点以查看其 **Provider base URLs（提供商基地址）**。Antix 支持在一个单一的端点 UUID 后面接入多个 AI 提供商；您只需将提供商名称和原生 API 路径附加到端点的基地址即可。

### URL 结构 {#url-structure}

```
https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>/<native_path>
```

在门户中，找到与您的 SDK 匹配的行，点击 **Copy（复制）**，然后将其粘贴到您的应用程序的 `base_url` 配置中。

### 按提供商 SDK 分类 {#by-provider-sdk}

- **OpenAI SDK（及兼容 OpenAI 的 SDK）**
  - 基地址：`https://antix.antigma.ai/v1/<endpoint_uuid>/openai`
  - SDK 会自动附加 `/v1/chat/completions`。
- **Anthropic SDK / Claude Code**
  - 基地址：`https://antix.antigma.ai/v1/<endpoint_uuid>/anthropic`
  - SDK 会自动附加 `/v1/messages`。
- **Gemini (Google AI Studio)**
  - 基地址：`https://antix.antigma.ai/v1/<endpoint_uuid>/gemini`
- **xAI、Alibaba (Qwen)、DeepSeek、Zai**
  - 基地址：`https://antix.antigma.ai/v1/<endpoint_uuid>/<xai|alibaba|deepseek|zai>`
  - 附加提供商自己的原生路径，或者在这些提供商上使用对应的兼容 OpenAI 的 SDK。
- **通用 / 多提供商（Universal / multi-provider）**
  - 基地址：`https://antix.antigma.ai/v1/<endpoint_uuid>/multi`
  - 使用 `/multi` 时，Antix 根据模型名称和标头而非固定的提供商来路由请求。有关路由规则和接受的 `X-Antix-Provider` 值，请参阅[路由与 BYOK](/antix/concepts/routing)。

:::warning
请将端点 URL 视为**机密信息**。它们标识了您的流量，并且很容易通过提交的配置文件泄露。请勿将它们推送到公共仓库。
:::

## 监控与分析 {#monitoring--analytics}

端点详情页面有四个选项卡。

### 概览（Overview） {#overview}

30 天内的活动摘要：

- **Requests（请求数）** — 处理的总请求数。
- **Tokens（代币数）** — 提示和完成代币的总数。
- **Cost (Antix billed)（成本（Antix 计费））** — 通过 Antix 平台凭据产生的总成本。
- 如果端点接收 BYOK 流量，则在主要数字下方显示估算的直通（passthrough）成本。

### 跟踪（Traces） {#traces}

近期请求的日志探索器：

- **Recent requests（近期请求）** — 包含时间、认证模式（虚拟密钥、BYOK、OAuth）、模型、代币数量、成本和延迟的表格。
- **Filter & search（过滤与搜索）** — 可折叠的过滤带通过时间范围（Time Range）、认证模式（Auth Mode）、模型（Model）或虚拟密钥 ID（Virtual Key ID）缩小范围，并在输入和输出正文中进行子字符串搜索。
- **Observation drawer（观察抽屉）** — 点击某一行滑出包含完整 JSON 请求和响应的抽屉，包含语法高亮、精准的首字节时间（TTFT）以及每次调用的成本。

:::note
根据网关策略，某些敏感字段（例如请求标头、认证令牌）不会被显示，而是从跟踪正文中完全省略。
:::

### 活动（代理会话）（Activity (Agent Sessions)） {#activity-agent-sessions}

此选项卡是**实验性的**，并且仅出现在接收支持会话签名（`/v1/messages`，`/v1/responses`）流量的端点上 —— 通常是像 Claude Code 这样的多轮代理框架。

- **Session grouping（会话分组）** — Antix 会根据初始用户提示自动将相关请求分组到会话中。不需要在客户端进行注入。
- **Gantt chart（甘特图）** — 从侧边栏选择一个会话以查看推理调用的时间轴。
- **Deep dive（深入探究）** — 点击某个条柱会打开在跟踪选项卡中使用的相同的观察抽屉。

### 设置（Settings） {#settings}

管理端点的生命周期和元数据：

- **Edit name（编辑名称）** — 更新显示名称。
- **Duplicate as Org Endpoint（复制为组织端点）** *（仅限个人端点）* — 组织管理员可以将个人端点提升为一个拥有*新* URL 的全新组织共享端点。原始个人端点在您将其归档之前将保持活动状态。
- **Archive（归档）** — 软删除该端点。任何仍指向被归档 URL 的客户端都会立即收到 `410 Gone`（请参阅[错误处理](/antix/concepts/error-handling)）。门户中仍可查看历史追踪记录和消费数据。**归档操作不可撤销。**

## 身份验证 {#authentication}

端点 URL 只负责路由流量；您仍须通过 `Authorization` 标头验证请求。

- **Virtual Keys（虚拟密钥）（推荐）** — 由 Antix 签发的密钥（`sk-antix-…`）。请求费用按该密钥设置的限额计入您的 Antix 组织账户。请参阅[虚拟密钥](/antix/concepts/virtual-keys)。
- **Bring Your Own Key (BYOK)（自带密钥）** — 发送您原始的提供商密钥（例如 `sk-ant-…`）。Antix 会将请求原封不动地传递给提供商，并且不会为此向您计费；门户网站上会给此类成本打上 *(est.)（估算）* 的徽章。

端点能够处理这两种身份验证模式 — 这是基于每个请求的选择，而不是端点的设置。
