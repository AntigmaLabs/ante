---
title: "模型 API"
description: "列出您的 Antix 网关可以提供服务的模型 —— 与 Anthropic 和 OpenAI 的模型 API 即插即用兼容。"
sidebar_position: 4
---

# 模型 API {#models-api}

列出您的 Antix 网关可以提供服务的模型，并查询单个模型。这些端点与 Anthropic 和 OpenAI 的模型 API **即插即用兼容**，因此现有的 SDK 无需任何修改即可工作 —— 只需将它们的基地址（base URL）指向 Antix 即可。

- `GET /v1/models` — 列出可用模型
- `GET /v1/models/{model_id}` — 检索单个模型

这里返回的 `id` 与您在调用推理端点（`/v1/messages`, `/v1/chat/completions`）时作为 `model` 字段传递的字符串完全一致。有关支持端点的完整列表，请参阅[路由与 BYOK](/antix/concepts/routing)。

## 快速入门 {#quick-start}

```bash
# OpenAI 样式的列表（默认）
curl -s "https://antix.antigma.ai/v1/models"

# Anthropic 样式的列表（注意 anthropic-version 标头）
curl -s "https://antix.antigma.ai/v1/models" -H "anthropic-version: 2023-06-01"

# 检索单个模型
curl -s "https://antix.antigma.ai/v1/models/claude-sonnet-4-6" -H "anthropic-version: 2023-06-01"
```

:::note 关于 URL 和版本标头
- **基地址（Base URL）** — 目录驻留在网关根目录（`https://antix.antigma.ai`），因此与推理流量不同，它**不需要**[端点 URL](/antix/concepts/endpoints) UUID 和[虚拟密钥](/antix/concepts/virtual-keys)。本页的示例特意使用了纯主机名；对于实际的 `/v1/messages` 或 `/v1/chat/completions` 调用，请将您的 SDK 指向您的端点 URL。
- **`anthropic-version`** — `2023-06-01` 是官方 SDK 默认发送的当前稳定的 Anthropic API 版本。这些公共端点只检查标头是否*存在*，而不检查其值，因此任何版本字符串（或 `x-api-key`）都能同样好地选择 Anthropic 的数据结构。
:::

## 身份验证 {#authentication}

这些端点是**公开的** — 调用它们不需要 API 密钥或[端点 URL](/antix/concepts/endpoints)。它们仅返回网关可提供服务的模型名称目录；它们绝不会暴露凭据、定价或路由内部信息。

:::note
此处返回的模型 **id** 是您作为 `model` 字段传递给推理端点（`/v1/messages`, `/v1/chat/completions`）的值，这些端点**确实**需要[虚拟密钥](/antix/concepts/virtual-keys)或 BYOK 凭据。
:::

## 响应格式与内容协商 {#response-format--content-negotiation}

`/v1/models` 从同一个 URL 提供**两种响应数据结构（shapes）**，具体由请求标头决定：

| 如果请求包含…                         | 您将获得…           |
| ------------------------------------------------- | ---------------------- |
| `anthropic-version` 标头（不考虑其他标头） | **Anthropic** 数据结构    |
| `x-api-key` **且没有** `Authorization` 标头     | **Anthropic** 数据结构    |
| 以上皆无（例如，仅有 `Authorization: Bearer …`、`x-api-key` 和 `Authorization` 同时存在，或没有验证标头） | **OpenAI** 数据结构 |

这反映了官方 SDK 发送请求的方式，因此：

- **Anthropic SDK**（总是发送 `anthropic-version`）自动接收 Anthropic 数据结构，以及
- **OpenAI SDK** 自动接收 OpenAI 数据结构。

您可以通过添加或省略 `anthropic-version` 标头，从 `curl` 强制指定一种数据结构。（标头**是否存在**才是关键 —— 这些公开端点不会验证其值。）

## 返回哪些模型 {#which-models-are-returned}

只有当以下**所有三个条件为真**时，模型才会显示：

1. **已配置** — 它在网关的模型配置中定义。
2. **已授权凭据** — 其提供商的平台 API 密钥在启动时被加载。例如，如果网关启动时没有 `XAI_API_KEY`，则不列出任何 xAI 模型。
3. **已定价** — 它在网关的费率表中有活跃的定价条目。

价格列表是实时读取的（带有短暂的缓存，约 5 分钟），因此当从上游来源刷新定价时，列表更新**无需重新部署**。

**排序**。返回的模型按 `id` **字母顺序排序**。

**`owned_by` / 提供商**。根据加载了哪些提供商密钥，提供商（OpenAI 结构中的 `owned_by` 字段）是以下之一：`anthropic`、`openai`、`google`、`alibaba`、`deepseek`、`zai`、`xai`。

**分页**。所有受支持的目录都在**单页**中返回。没有游标分页 — Anthropic 结构中的 `has_more` 总是 `false`，而且不使用 `limit` / `before_id` / `after_id` 查询参数。（Anthropic 的列表包络结构仍然会完整返回，以使 Anthropic SDK 的自动分页器能在这一页上正常工作。）

## `GET /v1/models` — 列出模型 {#get-v1models--list-models}

列出网关能提供服务的模型。

### 请求 {#request}

| | |
| --- | --- |
| **方法** | `GET` |
| **路径** | `/v1/models` |
| **认证** | 无 |

**标头**

| 标头 | 必填 | 备注 |
| --- | --- | --- |
| `anthropic-version` | 否 | 存在即选择 **Anthropic** 响应数据结构。常规值：`2023-06-01`。 |
| `x-api-key` | 否 | 也选择 Anthropic 数据结构，但仅当请求上**没有**同时存在 `Authorization` 时生效 —— 如果同时发送两者，则 OpenAI 数据结构胜出。 |

**查询参数**

| 参数 | 类型 | 默认值 | 备注 |
| --- | --- | --- | --- |
| `return_wildcard_routes` | 布尔值 | `false` | **仅限 OpenAI 结构。** 为真时（`True`/`true`/`1`/`yes`），为每个提供商返回一个 `"{provider}/*"` 的通配符条目，而不是为每个模型返回一个条目。由 LiteLLM 样式的客户端使用。 |

### 响应 — Anthropic 数据结构 {#response--anthropic-shape}

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
| `data` | 数组 | [模型对象](#anthropic-model-object)的列表。 |
| `has_more` | 布尔值 | 总是 `false` — 完整目录在一页中返回。 |
| `first_id` | 字符串 \| null | `data` 中第一项的 `id`，如果为空则为 `null`。 |
| `last_id` | 字符串 \| null | `data` 中最后一项的 `id`，如果为空则为 `null`。 |

#### Anthropic 模型对象 {#anthropic-model-object}

| 字段 | 类型 | 描述 |
| --- | --- | --- |
| `id` | 字符串 | 模型标识符 — 将此作为 `model` 字段传递给推理 API。 |
| `type` | 字符串 | 总是 `"model"`。 |
| `display_name` | 字符串 | 从 id 派生的人类可读标签。 |
| `created_at` | 字符串 (RFC 3339) | 发布时间戳。Antix 不追踪发布日期，因此这是纪元回退值 `"1970-01-01T00:00:00Z"`。 |

### 响应 — OpenAI 数据结构 {#response--openai-shape}

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
| `object` | 字符串 | 总是 `"list"`。 |
| `data` | 数组 | [模型对象](#openai-model-object)的列表。 |

#### OpenAI 模型对象 {#openai-model-object}

| 字段 | 类型 | 描述 |
| --- | --- | --- |
| `id` | 字符串 | 模型标识符 — 将此作为 `model` 字段传递给推理 API。 |
| `object` | 字符串 | 总是 `"model"`。 |
| `created` | 整数 | Unix 时间戳。Antix 不追踪发布日期，因此这是 `0`。 |
| `owned_by` | 字符串 | 拥有该模型的提供商（例如 `anthropic`、`openai`）。 |

**带有 `?return_wildcard_routes=True`：**

```json
{
  "object": "list",
  "data": [
    { "id": "anthropic/*", "object": "model", "created": 0, "owned_by": "anthropic" },
    { "id": "openai/*",    "object": "model", "created": 0, "owned_by": "openai" }
  ]
}
```

### 示例 {#examples}

**curl — OpenAI 数据结构**

```bash
curl -s "https://antix.antigma.ai/v1/models"
```

**curl — Anthropic 数据结构**

```bash
curl -s "https://antix.antigma.ai/v1/models" -H "anthropic-version: 2023-06-01"
```

**Anthropic Python SDK**

```python
import anthropic

client = anthropic.Anthropic(
    base_url="https://antix.antigma.ai",
    api_key="sk-antix-<your-key>",  # 未被此端点验证
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

## `GET /v1/models/{model_id}` — 检索模型 {#get-v1modelsmodel_id--retrieve-a-model}

根据其 id 获取单个模型。

### 请求 {#request-1}

| | |
| --- | --- |
| **方法** | `GET` |
| **路径** | `/v1/models/{model_id}` |
| **认证** | 无 |

**路径参数**

| 参数 | 类型 | 描述 |
| --- | --- | --- |
| `model_id` | 字符串 | 模型 id，例如 `claude-sonnet-4-6`。必须是单路径段（id 绝不包含 `/`）。 |

相同的内容协商规则适用：发送 `anthropic-version`（或 `x-api-key`）以获取 Anthropic 数据结构，否则获取 OpenAI 数据结构。

### 响应 {#response}

`200 OK` — 协商数据结构中的单个模型对象（无列表包络）。

**Anthropic 数据结构**

```json
{
  "id": "claude-sonnet-4-6",
  "type": "model",
  "display_name": "Claude Sonnet 4 6",
  "created_at": "1970-01-01T00:00:00Z"
}
```

**OpenAI 数据结构**

```json
{
  "id": "claude-sonnet-4-6",
  "object": "model",
  "created": 0,
  "owned_by": "anthropic"
}
```

对象字段与在[列出模型](#response--anthropic-shape)下记录的每个模型对象相同。

### 错误 {#errors}

未提供服务的模型 — 未知、其提供商密钥未加载或未定价 — 会以协商的错误数据结构返回 **`404 Not Found`**。（这是有意为之的：该端点不区分“未配置”和“当前无法提供服务”，因此它绝不会透露模型仅仅在配置中存在。）

**Anthropic 数据结构**

```json
{
  "type": "error",
  "error": {
    "type": "not_found_error",
    "message": "model: grok-3"
  }
}
```

**OpenAI 数据结构**

```json
{
  "error": {
    "message": "The model `does-not-exist` does not exist or you do not have access to it.",
    "type": "invalid_request_error",
    "code": "model_not_found"
  }
}
```

### 示例 {#examples-1}

**curl**

```bash
# Anthropic 数据结构
curl -s "https://antix.antigma.ai/v1/models/claude-sonnet-4-6" -H "anthropic-version: 2023-06-01"

# OpenAI 数据结构
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

## 状态和错误参考 {#status--error-reference}

| 状态 | 何时 | 主体 |
| --- | --- | --- |
| `200 OK` | 列表请求（始终），或对提供服务的模型的检索 | 列表包络 / 单个模型对象 |
| `404 Not Found` | 对未提供服务的模型的检索 | 协商的错误包络（见上文） |

列表端点始终返回 `200`；如果没有可服务的项，`data` 数组为空（Anthropic 结构中 `has_more: false`，`first_id`/`last_id` 为 `null`）。有关所有端点的完整错误代码参考，请参阅[错误处理](/antix/concepts/error-handling)。

## 备注与常见问题 {#notes--faq}

**为什么我期待的模型丢失了？**
它必须 (1) 在网关的模型配置中，(2) 由已加载的提供商密钥支持，且 (3) 已定价。如果模型已配置但缺少上述任一条件，它将不会出现，且检索它将返回 `404`。

**为什么 `created_at` 是纪元（`1970-01-01`） / `created` 是 `0`？**
Antix 不追踪每个模型的发布日期。当发布日期未知时，Anthropic API 明确认可使用纪元回退值。

**为什么 `display_name` 仅在 Anthropic 数据结构中？**
OpenAI 模型对象没有 `display_name` 字段；我们在那里省略它是为了忠实于该 API。Anthropic 的值是从 id 派生出来的（Antix 没有人工策划的显示名称）。

**排序是否匹配 Anthropic 的 API？**
Anthropic 按发布日期（最新的优先）排序。Antix 没有发布日期，因此它按 `id` **字母顺序排序**。

**列表有多新？**
定价通过大约 5 分钟的缓存读取，因此来自上游定价导入的更改会在几分钟内显示，无需重新部署。配置的模型和已加载提供商密钥的集合在网关启动时固定。
