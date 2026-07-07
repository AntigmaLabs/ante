---
title: "端点"
description: "稳定的应用程序级 URL，用于标记流量以进行支出分析、追踪和智能体回话监控。"
sidebar_position: 4
---

# 端点

**端点（Endpoint）** 是一个稳定且唯一的 URL（例如 `https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>/...`），它可以直接作为提供商基础 URL 的替代。流经它的每一个请求都会自动用该端点的 ID 进行标记，从而为您提供应用程序级别的可见性：

- **支出和使用情况（Spend & usage）** — 每个端点的代币（token）数量和美元成本。
- **请求追踪（Request traces）** — 完整的请求/响应生命周期，包括延迟和每次调用的成本。
- **智能体回话（Agent sessions）** — 将多轮请求分组为连贯的会话，以便于调试。

端点独立于身份验证：它们决定请求*发送到哪里*以及*如何标记*，而 [虚拟密钥](/zh-Hans/antix/concepts/virtual-keys)（或 BYOK）决定*谁来买单*以及*适用哪些限制*。

## 创建端点

端点是从 [Antix 门户网站](https://portal.antigma.ai) 的 **Endpoints**（端点）选项卡中创建的。

### 作用域：个人与组织

创建端点时，您需要选择其作用域：

- **个人（Personal）** — 仅对您可见。适用于本地开发或实验。上限为**每个用户每个组织10个个人端点**。
- **组织共享（Org-shared）** — 对组织的所有成员可见。只有**组织管理员（Admins）**才能创建组织共享的端点。组织共享端点共享一个单独的、更大的上限（默认每个组织50个）。

### 步骤

1. 打开门户网站中的 **Endpoints**（端点）选项卡。
2. 在 **Create Endpoint**（创建端点）卡片中，选择该端点所属的组织。
3. 选择 **Personal**（个人）或 **Org-shared**（组织共享）。
4. 输入 **Display Name**（显示名称，例如 `CI Runner`, `Frontend Production`）。
5. 点击 **Create**（创建）。

:::note
显示名称稍后可以编辑。生成的 URL（其中内嵌了 UUID）在**该端点的生命周期内是固定的**，永远不会改变。
:::

## 使用您的端点 URL

点击进入某个端点以查看其 **Provider base URLs**（提供商基础 URL）。Antix 在单一端点 UUID 背后支持多个 AI 提供商；您只需将提供商名称和原生 API 路径附加到端点的基础 URL 上即可。

### URL 结构

```
https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>/<native_path>
```

在门户网站中，找到与您的 SDK 匹配的行，点击 **Copy**（复制），并将其粘贴到应用程序的 `base_url` 配置中。

### 按提供商 SDK 分类

- **OpenAI SDK（以及兼容 OpenAI 的 SDK）**
  - 基础 URL：`https://antix.antigma.ai/v1/<endpoint_uuid>/openai`
  - SDK 会自动附加 `/v1/chat/completions`。
- **Anthropic SDK / Claude Code**
  - 基础 URL：`https://antix.antigma.ai/v1/<endpoint_uuid>/anthropic`
  - SDK 会自动附加 `/v1/messages`。
- **Gemini (Google AI Studio)**
  - 基础 URL：`https://antix.antigma.ai/v1/<endpoint_uuid>/gemini`
- **xAI, Alibaba (Qwen), DeepSeek, Zai**
  - 基础 URL：`https://antix.antigma.ai/v1/<endpoint_uuid>/<xai|alibaba|deepseek|zai>`
  - 附加提供商自己的原生路径，或针对这些提供商使用兼容 OpenAI 的同等 SDK。
- **通用（Universal）/ 多提供商（multi-provider）**
  - 基础 URL：`https://antix.antigma.ai/v1/<endpoint_uuid>/multi`
  - 使用 `/multi` 时，Antix 会根据模型名称和请求头而不是固定的提供商来路由请求。参见 [路由和自带密钥 (BYOK)](/zh-Hans/antix/concepts/routing) 了解路由规则和支持的 `X-Antix-Provider` 值。

:::warning
请将端点 URL 视为**机密**。它们能够识别您的流量，并且很容易通过提交的配置文件泄露。请勿将它们推送到公共仓库中。
:::

## 监控与分析

端点详情页面包含四个选项卡。

### 概览（Overview）

30 天内的活动摘要：

- **请求（Requests）** — 处理的请求总数。
- **代币（Tokens）** — 提示词（prompt）和完成词（completion）代币总数。
- **成本（Cost, Antix billed）** — 通过 Antix 平台凭据产生的总成本。
- 如果端点接收到 BYOK 流量，估计的直通成本（passthrough cost）会显示在主要数据的下方。

### 追踪（Traces）

用于近期请求的日志资源管理器：

- **近期请求（Recent requests）** — 包含时间、身份验证模式（Virtual Key、BYOK、OAuth）、模型、代币数、成本和延迟的表格。
- **过滤与搜索（Filter & search）** — 可折叠的过滤栏可通过时间范围、身份验证模式、模型或虚拟密钥 ID 缩小范围，并在输入和输出主体中支持子字符串搜索。
- **观察抽屉（Observation drawer）** — 点击一行以滑出完整的 JSON 请求和响应，带有语法高亮、精确的首次代币时间 (TTFT) 以及每次调用的成本。

:::note
根据网关策略，某些敏感字段（例如请求头、身份验证令牌）将从追踪记录中完全省略，而不会显示。
:::

### 活动（Activity - Agent Sessions）

此选项卡是**实验性**的，仅出现在接收支持会话签名的流量（如 `/v1/messages`、`/v1/responses`）的端点上——这通常是诸如 Claude Code 的多轮智能体框架。

- **会话分组（Session grouping）** — Antix 会根据最初的用户提示词自动将相关的请求分组为连贯的会话。无需进行客户端代码埋点（instrumentation）。
- **甘特图（Gantt chart）** — 从侧边栏中选择一个会话以查看推理调用的时间轴。
- **深度剖析（Deep dive）** — 点击某个条形图会打开追踪（Traces）选项卡中使用的同一个观察抽屉。

### 设置（Settings）

管理端点的生命周期和元数据：

- **编辑名称（Edit name）** — 更新显示名称。
- **复制为组织端点（Duplicate as Org Endpoint）** *（仅限个人端点）* — 组织管理员可以将个人端点提升为拥有*新* URL 的全新组织共享端点。原始个人端点保持活动状态，直到您将其归档。
- **归档（Archive）** — 软删除该端点。任何仍指向已归档 URL 的客户端都会立即收到 `410 Gone`（参见 [错误处理](/zh-Hans/antix/concepts/error-handling)）。历史追踪和支出数据在门户网站中仍然可见。**归档操作无法撤销。**

## 身份验证

端点 URL 负责路由流量；您仍然需要通过 `Authorization` 请求头对请求进行身份验证。

- **虚拟密钥（Virtual Keys，推荐）** — Antix 签发的密钥（`sk-antix-…`）。请求将根据该密钥设置的限制向您的 Antix 组织计费。参见 [虚拟密钥](/zh-Hans/antix/concepts/virtual-keys)。
- **自带密钥 (BYOK)** — 发送您的原始提供商密钥（例如 `sk-ant-…`）。Antix 会将请求原封不动地传递给提供商，并且不会为此向您计费；门户网站会为该成本打上 *(est.)*（预估）标签。

端点适用于任何一种身份验证模式——这是在每个请求级别做出的选择，而不是在端点级别。
