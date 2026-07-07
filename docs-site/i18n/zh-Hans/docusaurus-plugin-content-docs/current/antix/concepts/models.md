---
title: "模型 API"
description: "列出您的 Antix 网关可以提供的模型——与 Anthropic 和 OpenAI 的模型 API 即插即用兼容。"
sidebar_position: 4
---

# 模型 API

列出您的 Antix 网关可以提供的模型，并查找单个模型。这些端点与 Anthropic 和 OpenAI 的模型 API **即插即用兼容**，因此现有的 SDK 无需更改即可工作——只需将其基础 URL 指向 Antix 即可。

- `GET /v1/models` — 列出可用模型
- `GET /v1/models/{model_id}` — 检索单个模型

这里返回的 `id` 与调用推理端点（`/v1/messages`, `/v1/chat/completions`）时作为 `model` 字段传递的字符串完全一致。有关支持的端点的完整列表，请参见 [路由和自带密钥 (BYOK)](/zh-Hans/antix/concepts/routing)。

## 快速开始

```bash
# OpenAI 风格的列表（默认）
curl -s "https://antix.antigma.ai/v1/models"

# Anthropic 风格的列表（请注意 anthropic-version 请求头）
curl -s "https://antix.antigma.ai/v1/models" -H "anthropic-version: 2023-06-01"

# 检索单个模型
curl -s "https://antix.antigma.ai/v1/models/claude-sonnet-4-6" -H "anthropic-version: 2023-06-01"
```

:::note 关于 URL 和版本请求头
- **基础 URL** — 目录位于网关的根路径（`https://antix.antigma.ai`），因此与推理流量不同，它**不**需要 [端点 URL](/zh-Hans/antix/concepts/endpoints) UUID 和 [虚拟密钥](/zh-Hans/antix/concepts/virtual-keys)。本页面上的示例故意使用了裸主机名；对于实际的 `/v1/messages` 或 `/v1/chat/completions` 调用，请将您的 SDK 指向您的端点 URL。
- **`anthropic-version`** — `2023-06-01` 是当前的稳定 Anthropic API 版本，官方 SDK 会默认发送此版本。这些公共端点仅检查该请求头是否*存在*，而不检查其值，因此任何版本字符串（或 `x-api-key`）都能同样地选择 Anthropic 结构形式。
:::

## 身份验证

这些端点是**公开的**——调用它们不需要 API 密钥或 [端点 URL](/zh-Hans/antix/concepts/endpoints)。它们只返回网关可提供的模型名称目录；它们绝不会暴露凭据、定价或内部路由信息。

:::note
这里返回的模型 **id** 就是您作为 `model` 字段传递给推理端点（`/v1/messages`, `/v1/chat/completions`）的值，而推理端点**确实**需要 [虚拟密钥](/zh-Hans/antix/concepts/virtual-keys) 或 BYOK 凭据。
:::

## 响应格式与内容协商

`/v1/models` 从同一个 URL 提供**两种响应结构形式**，由请求头决定：

| 如果请求包含…                         | 您会得到…           |
| ------------------------------------------------- | ---------------------- |
| `anthropic-version` 请求头（无视其他请求头） | **Anthropic** 结构形式    |
| `x-api-key` **并且没有** `Authorization` 请求头     | **Anthropic** 结构形式    |
| 上述两者均没有（例如只有 `Authorization: Bearer …`、`x-api-key` 与 `Authorization` 同时存在、或没有身份验证请求头） | **OpenAI** 结构形式 |

这反映了官方 SDK 发送请求的方式，因此：

- **Anthropic SDK**（总是发送 `anthropic-version`）会自动收到 Anthropic 结构形式，且
- **OpenAI SDK** 会自动收到 OpenAI 结构形式。

您可以通过添加或省略 `anthropic-version` 请求头来在 `curl` 中强制指定响应结构形式。（请求头的**存在性**才是关键——这些公共端点不会验证其值。）

## 返回哪些模型

只有在满足以下**全部三个条件**时，模型才会显示：

1. **已配置（Configured）** — 它在网关的模型配置中定义。
2. **已授权（Credentialed）** — 针对其提供商的平台 API 密钥已在启动时加载。例如，如果网关启动时没有 `XAI_API_KEY`，则不会列出任何 xAI 模型。
3. **已定价（Priced）** — 它在网关的费率表中有活跃的定价条目。

价格列表是实时读取的（带有约 5 分钟的短暂缓存），因此当从上游来源刷新定价时，模型列表会**无需重新部署**地进行更新。

**排序。** 返回的模型按 **`id` 的字母顺序**排序。

**`owned_by` / 提供商。** 根据加载的提供商密钥，提供商（OpenAI 结构形式中的 `owned_by` 字段）将是以下之一：`anthropic`, `openai`, `google`, `alibaba`, `deepseek`, `zai`, `xai`。

**分页。** 完整支持的模型目录在**单页**中返回。没有游标分页——Anthropic 结构形式的 `has_more` 始终为 `false`，且不使用 `limit` / `before_id` / `after_id` 查询参数。（Anthropic 列表包裹层仍然会被完整返回，以便 Anthropic SDK 的自动分页器能够在该单一页面上正常工作。）

## `GET /v1/models` — 列出模型

列出网关可提供的模型。

### 请求

| | |
| --- | --- |
| **方法** | `GET` |
| **路径** | `/v1/models` |
| **身份验证** | 无 |

**请求头**

| 请求头 | 必需 | 备注 |
| --- | --- | --- |
| `anthropic-version` | 否 | 只要存在该请求头，即可选择 **Anthropic** 响应结构形式。常规值：`2023-06-01`。 |
| `x-api-key` | 否 | 也可以选择 Anthropic 结构形式，但前提是请求中**未**同时包含 `Authorization`——如果两者都发送了，则优先使用 OpenAI 结构形式。 |

**查询参数**

| 参数 | 类型 | 默认值 | 备注 |
| --- | --- | --- | --- |
| `return_wildcard_routes` | boolean | `false` | **仅限于 OpenAI 结构形式。** 为真值（`True`/`true`/`1`/`yes`）时，为每个提供商返回一个 `"{provider}/*"` 的通配符条目，而不是每个模型一个条目。被类似 LiteLLM 的客户端使用。 |

### 响应 — Anthropic 结构形式

`200 OK`

```json
{
  "data": [
    {
      "id": "claude-sonnet-4-6",
      "type": "model",
      "display_name": "Claude Sonnet 4 6",
      "created_at": "1970-01-01T00:00:00Z"
    }
  ],
  "has_more": false,
  "first_id": "claude-sonnet-4-6",
  "last_id": "claude-sonnet-4-6"
}
```

| 字段 | 类型 | 描述 |
| --- | --- | --- |
| `data` | array | [模型对象](#anthropic-model-object) 的列表。 |
| `has_more` | boolean | 始终为 `false` — 完整的目录在单页中返回。 |
| `first_id` | string \\| null | `data` 中第一个项目的 `id`，如果为空则为 `null`。 |
| `last_id` | string \\| null | `data` 中最后一个项目的 `id`，如果为空则为 `null`。 |

#### Anthropic 模型对象 {#anthropic-model-object}

| 字段 | 类型 | 描述 |
| --- | --- | --- |
| `id` | string | 模型标识符——在推理 API 中将其作为 `model` 字段传递。 |
| `type` | string | 始终为 `"model"`。 |
| `display_name` | string | 从 ID 派生的便于人类阅读的标签。 |
| `created_at` | string (RFC 3339) | 发布时间戳。Antix 不追踪发布日期，因此这是 fallback 到纪元时间 `"1970-01-01T00:00:00Z"`。 |

### 响应 — OpenAI 结构形式

`200 OK`

```json
{
  "object": "list",
  "data": [
    {
      "id": "claude-sonnet-4-6",
      "object": "model",
      "created": 0,
      "owned_by": "anthropic"
    }
  ]
}
```

| 字段 | 类型 | 描述 |
| --- | --- | --- |
| `object` | string | 始终为 `"list"`。 |
| `data` | array | [模型对象](#openai-model-object) 的列表。 |

#### OpenAI 模型对象 {#openai-model-object}

| 字段 | 类型 | 描述 |
| --- | --- | --- |
| `id` | string | 模型标识符——在推理 API 中将其作为 `model` 字段传递。 |
| `object` | string | 始终为 `"model"`。 |
| `created` | integer | Unix 时间戳。Antix 不追踪发布日期，因此此值为 `0`。 |
| `owned_by` | string | 拥有该模型的提供商（例如 `anthropic`, `openai`）。 |

**使用 `?return_wildcard_routes=True`:**

```json
{
  "object": "list",
  "data": [
    { "id": "anthropic/*", "object": "model", "created": 0, "owned_by": "anthropic" },
    { "id": "openai/*",    "object": "model", "created": 0, "owned_by": "openai" }
  ]
}
```

### 示例

**curl — OpenAI 结构形式**

```bash
curl -s "https://antix.antigma.ai/v1/models"
```

**curl — Anthropic 结构形式**

```bash
curl -s "https://antix.antigma.ai/v1/models" -H "anthropic-version: 2023-06-01"
```

**Anthropic Python SDK**

```python
import anthropic

client = anthropic.Anthropic(
    base_url="https://antix.antigma.ai",
    api_key="sk-antix-<your-key>",  # 此端点不验证该密钥
)

for model in client.models.list():
    print(model.id, "-", model.display_name)
```

**OpenAI Python SDK**

```python
from openai import OpenAI

client = OpenAI(
    base_url="https://antix.antigma.ai/v1",
    api_key="sk-antix-<your-key>",
)

for model in client.models.list().data:
    print(model.id, "-", model.owned_by)
```

## `GET /v1/models/{model_id}` — 检索模型

通过模型 ID 获取单个模型。

### 请求

| | |
| --- | --- |
| **方法** | `GET` |
| **路径** | `/v1/models/{model_id}` |
| **身份验证** | 无 |

**路径参数**

| 参数 | 类型 | 描述 |
| --- | --- | --- |
| `model_id` | string | 模型 id，例如 `claude-sonnet-4-6`。必须是单个路径段（id 永远不包含 `/`）。 |

同样的内容协商规则在此适用：发送 `anthropic-version`（或 `x-api-key`）获取 Anthropic 结构形式，否则获取 OpenAI 结构形式。

### 响应

`200 OK` — 采用经过内容协商后的结构形式的单一模型对象（无列表包裹层）。

**Anthropic 结构形式**

```json
{
  "id": "claude-sonnet-4-6",
  "type": "model",
  "display_name": "Claude Sonnet 4 6",
  "created_at": "1970-01-01T00:00:00Z"
}
```

**OpenAI 结构形式**

```json
{
  "id": "claude-sonnet-4-6",
  "object": "model",
  "created": 0,
  "owned_by": "anthropic"
}
```

对象字段与 [列出模型](#响应--anthropic-结构形式) 中记录的逐个模型对象相同。

### 错误

如果某个模型未被提供——不管是未知的、其提供商密钥未加载还是未定价——将以协商后的错误形式返回 **`404 Not Found`**。（这是设计使然：该端点不区分“未配置”与“当前不可提供”，因此它绝不会泄露某个模型仅仅是存在于配置中的事实。）

**Anthropic 结构形式**

```json
{
  "type": "error",
  "error": {
    "type": "not_found_error",
    "message": "model: grok-3"
  }
}
```

**OpenAI 结构形式**

```json
{
  "error": {
    "message": "The model `does-not-exist` does not exist or you do not have access to it.",
    "type": "invalid_request_error",
    "code": "model_not_found"
  }
}
```

### 示例

**curl**

```bash
# Anthropic 结构形式
curl -s "https://antix.antigma.ai/v1/models/claude-sonnet-4-6" -H "anthropic-version: 2023-06-01"

# OpenAI 结构形式
curl -s "https://antix.antigma.ai/v1/models/claude-sonnet-4-6"
```

**Anthropic Python SDK**

```python
model = client.models.retrieve("claude-sonnet-4-6")
print(model.id, model.display_name)
```

**OpenAI Python SDK**

```python
model = client.models.retrieve("claude-sonnet-4-6")
print(model.id, model.owned_by)
```

## 状态与错误参考

| 状态 | 发生场景 | 主体 |
| --- | --- | --- |
| `200 OK` | 列表请求（始终），或检索可提供的模型 | 列表包裹层 / 单个模型对象 |
| `404 Not Found` | 检索未提供的模型 | 经过内容协商的错误包裹层（见上文） |

列表端点始终返回 `200`；如果没有可提供的模型，`data` 数组将为空（Anthropic 结构形式中的 `has_more: false`，`first_id`/`last_id` 为 `null`）。有关跨所有端点的完整错误代码参考，请参见 [错误处理](/zh-Hans/antix/concepts/error-handling)。

## 备注与常见问题 (FAQ)

**为什么我期望的模型丢失了？**
它必须满足三个条件：(1) 存在于网关的模型配置中，(2) 得到已加载的提供商密钥的支持，(3) 具有定价。如果一个模型被配置了但缺少其中任何一项，都不会显示，并且检索它时会返回 `404`。

**为什么 `created_at` 是纪元时间 (`1970-01-01`) / `created` 是 `0`？**
Antix 不追踪每个模型的发布日期。当发布日期未知时，Anthropic API 明确支持 fallback 为纪元时间。

**为什么 `display_name` 仅在 Anthropic 结构形式中存在？**
OpenAI 模型对象没有 `display_name` 字段；为了忠实于该 API，我们在其中省略了该字段。Anthropic 中的值是从 ID 派生而来的（Antix 没有人工精心编排的显示名称）。

**排序是否匹配 Anthropic 的 API？**
Anthropic 按发布日期（最新的优先）进行排序。Antix 没有发布日期数据，因此按 **ID 的字母顺序** 排序。

**列表的更新频率是多少？**
定价数据通过一个大约 5 分钟的缓存进行读取，因此从上游导入的定价变更可以在几分钟内显现，且无需重新部署。已配置模型与已加载提供商密钥的集合在网关启动时即固定。
