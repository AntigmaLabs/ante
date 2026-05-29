---
title: "Models API"
description: "List the models your Antix gateway can serve — drop-in compatible with the Anthropic and OpenAI Models APIs."
sidebar_position: 4
---

# Models API

List the models your Antix gateway can serve, and look up a single model. The endpoints are **drop-in compatible** with both the Anthropic and OpenAI Models APIs, so existing SDKs work unchanged — just point their base URL at Antix.

- `GET /v1/models` — list available models
- `GET /v1/models/{model_id}` — retrieve a single model

The `id` returned here is exactly the string you pass as the `model` field when calling the inference endpoints (`/v1/messages`, `/v1/chat/completions`). See [Routing & BYOK](/antix/concepts/routing) for the full list of supported endpoints.

## Quick start

```bash
# OpenAI-style listing (default)
curl -s "https://antix.antigma.ai/v1/models"

# Anthropic-style listing (note the anthropic-version header)
curl -s "https://antix.antigma.ai/v1/models" -H "anthropic-version: 2023-06-01"

# Retrieve one model
curl -s "https://antix.antigma.ai/v1/models/claude-sonnet-4-6" -H "anthropic-version: 2023-06-01"
```

:::note About the URL and the version header
- **Base URL** — the catalog lives at the gateway root (`https://antix.antigma.ai`), so unlike inference traffic it needs **no** [endpoint URL](/antix/concepts/endpoints) UUID and no [Virtual Key](/antix/concepts/virtual-keys). The examples on this page use the bare host on purpose; for actual `/v1/messages` or `/v1/chat/completions` calls, point your SDK at your endpoint URL instead.
- **`anthropic-version`** — `2023-06-01` is the current stable Anthropic API version that the official SDKs send by default. These public endpoints only check whether the header is *present*, not its value, so any version string (or `x-api-key`) selects the Anthropic shape just as well.
:::

## Authentication

These endpoints are **public** — no API key or [endpoint URL](/antix/concepts/endpoints) is required to call them. They return only the catalog of model names the gateway can serve; they never expose credentials, pricing, or routing internals.

:::note
The model **id** returned here is the value you pass as the `model` field to the inference endpoints (`/v1/messages`, `/v1/chat/completions`), which **do** require a [Virtual Key](/antix/concepts/virtual-keys) or BYOK credential.
:::

## Response format & content negotiation

`/v1/models` serves **two response shapes** from the same URL, chosen by the request headers:

| If the request includes…                         | You get the…           |
| ------------------------------------------------- | ---------------------- |
| `anthropic-version` **or** `x-api-key` header     | **Anthropic** shape    |
| neither (e.g. only `Authorization: Bearer …`, or no auth header) | **OpenAI** shape |

This mirrors how the official SDKs send requests, so:

- the **Anthropic SDK** (which always sends `anthropic-version`) automatically receives the Anthropic shape, and
- the **OpenAI SDK** automatically receives the OpenAI shape.

You can force a shape from `curl` by adding or omitting the `anthropic-version` header. (Header **presence** is what matters — the value is not validated by these public endpoints.)

## Which models are returned

A model appears **only if all three** of the following are true:

1. **Configured** — it is defined in the gateway's model configuration.
2. **Credentialed** — the platform API key for its provider is loaded at startup. For example, if the gateway starts without `XAI_API_KEY`, no xAI models are listed.
3. **Priced** — it has an active price entry in the gateway's rate card.

The price list is read live (with a short cache, ~5 minutes), so when pricing is refreshed from upstream sources the listing updates **without a redeploy**.

**Ordering.** Models are returned sorted **alphabetically by `id`**.

**`owned_by` / providers.** Depending on which provider keys are loaded, the provider (the `owned_by` field in the OpenAI shape) is one of: `anthropic`, `openai`, `google`, `alibaba`, `deepseek`, `xai`.

**Pagination.** The full supported catalog is returned in a **single page**. There is no cursor pagination — the Anthropic-shape `has_more` is always `false`, and `limit` / `before_id` / `after_id` query parameters are not used. (The Anthropic list envelope is still returned in full so the Anthropic SDK's auto-paginator works correctly over the one page.)

## `GET /v1/models` — List models

List the models the gateway can serve.

### Request

| | |
| --- | --- |
| **Method** | `GET` |
| **Path** | `/v1/models` |
| **Auth** | none |

**Headers**

| Header | Required | Notes |
| --- | --- | --- |
| `anthropic-version` | no | Presence selects the **Anthropic** response shape. Conventional value: `2023-06-01`. |
| `x-api-key` | no | Also selects the Anthropic shape (presence only). |

**Query parameters**

| Param | Type | Default | Notes |
| --- | --- | --- | --- |
| `return_wildcard_routes` | boolean | `false` | **OpenAI shape only.** When truthy (`True`/`true`/`1`/`yes`), returns one `"{provider}/*"` wildcard entry per provider instead of one entry per model. Used by LiteLLM-style clients. |

### Response — Anthropic shape

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

| Field | Type | Description |
| --- | --- | --- |
| `data` | array | The list of [model objects](#anthropic-model-object). |
| `has_more` | boolean | Always `false` — the full catalog is returned in one page. |
| `first_id` | string \| null | `id` of the first item in `data`, or `null` if empty. |
| `last_id` | string \| null | `id` of the last item in `data`, or `null` if empty. |

#### Anthropic model object {#anthropic-model-object}

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | The model identifier — pass this as the `model` field to the inference API. |
| `type` | string | Always `"model"`. |
| `display_name` | string | Human-readable label derived from the id. |
| `created_at` | string (RFC 3339) | Release timestamp. Antix does not track release dates, so this is the epoch fallback `"1970-01-01T00:00:00Z"`. |

### Response — OpenAI shape

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

| Field | Type | Description |
| --- | --- | --- |
| `object` | string | Always `"list"`. |
| `data` | array | The list of [model objects](#openai-model-object). |

#### OpenAI model object {#openai-model-object}

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | The model identifier — pass this as the `model` field to the inference API. |
| `object` | string | Always `"model"`. |
| `created` | integer | Unix timestamp. Antix does not track release dates, so this is `0`. |
| `owned_by` | string | The provider that owns the model (e.g. `anthropic`, `openai`). |

**With `?return_wildcard_routes=True`:**

```json
{
  "object": "list",
  "data": [
    { "id": "anthropic/*", "object": "model", "created": 0, "owned_by": "anthropic" },
    { "id": "openai/*",    "object": "model", "created": 0, "owned_by": "openai" }
  ]
}
```

### Examples

**curl — OpenAI shape**

```bash
curl -s "https://antix.antigma.ai/v1/models"
```

**curl — Anthropic shape**

```bash
curl -s "https://antix.antigma.ai/v1/models" -H "anthropic-version: 2023-06-01"
```

**Anthropic Python SDK**

```python
import anthropic

client = anthropic.Anthropic(
    base_url="https://antix.antigma.ai",
    api_key="sk-antix-<your-key>",  # not validated by this endpoint
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

## `GET /v1/models/{model_id}` — Retrieve a model

Fetch a single model by its id.

### Request

| | |
| --- | --- |
| **Method** | `GET` |
| **Path** | `/v1/models/{model_id}` |
| **Auth** | none |

**Path parameters**

| Param | Type | Description |
| --- | --- | --- |
| `model_id` | string | The model id, e.g. `claude-sonnet-4-6`. Must be a single path segment (ids never contain `/`). |

The same content-negotiation rule applies: send `anthropic-version` (or `x-api-key`) for the Anthropic shape, otherwise the OpenAI shape.

### Response

`200 OK` — a single model object in the negotiated shape (no list envelope).

**Anthropic shape**

```json
{
  "id": "claude-sonnet-4-6",
  "type": "model",
  "display_name": "Claude Sonnet 4 6",
  "created_at": "1970-01-01T00:00:00Z"
}
```

**OpenAI shape**

```json
{
  "id": "claude-sonnet-4-6",
  "object": "model",
  "created": 0,
  "owned_by": "anthropic"
}
```

The object fields are identical to the per-model objects documented under [List models](#response--anthropic-shape).

### Errors

A model that is not served — unknown, its provider key is not loaded, or it is unpriced — returns **`404 Not Found`** in the negotiated error shape. (This is intentional: the endpoint does not distinguish "not configured" from "not currently serveable", so it never reveals that a model merely exists in config.)

**Anthropic shape**

```json
{
  "type": "error",
  "error": {
    "type": "not_found_error",
    "message": "model: grok-3"
  }
}
```

**OpenAI shape**

```json
{
  "error": {
    "message": "The model `does-not-exist` does not exist or you do not have access to it.",
    "type": "invalid_request_error",
    "code": "model_not_found"
  }
}
```

### Examples

**curl**

```bash
# Anthropic shape
curl -s "https://antix.antigma.ai/v1/models/claude-sonnet-4-6" -H "anthropic-version: 2023-06-01"

# OpenAI shape
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

## Status & error reference

| Status | When | Body |
| --- | --- | --- |
| `200 OK` | List request (always), or retrieve of a served model | List envelope / single model object |
| `404 Not Found` | Retrieve of a model that is not served | Negotiated error envelope (see above) |

The list endpoint always returns `200`; if nothing is serveable the `data` array is empty (`has_more: false`, `first_id`/`last_id` `null` in the Anthropic shape). For the full error-code reference across all endpoints, see [Error Handling](/antix/concepts/error-handling).

## Notes & FAQ

**Why is a model I expected missing?**
It must be (1) in the gateway's model configuration, (2) backed by a loaded provider key, and (3) priced. A model configured but missing any of these will not appear, and retrieving it returns `404`.

**Why is `created_at` the epoch (`1970-01-01`) / `created` `0`?**
Antix does not track per-model release dates. The Anthropic API explicitly sanctions an epoch fallback when the release date is unknown.

**Why is `display_name` only in the Anthropic shape?**
The OpenAI model object has no `display_name` field; we omit it there to stay faithful to that API. The Anthropic value is derived from the id (Antix has no curated display names).

**Does ordering match Anthropic's API?**
Anthropic orders by release date (newest first). Antix has no release dates, so it orders **alphabetically by id**.

**How fresh is the list?**
Pricing is read through a ~5-minute cache, so changes from upstream pricing imports appear within a few minutes without a redeploy. The set of configured models and loaded provider keys is fixed at gateway startup.
