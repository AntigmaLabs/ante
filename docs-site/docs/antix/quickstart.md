---
title: "Quickstart"
description: "Make your first request to Antix in under 5 minutes."
sidebar_position: 2
---

# Quickstart

Antix speaks four wire protocols — OpenAI Chat Completions, OpenAI Responses, Anthropic Messages, and Gemini native — over the same proxy. Point any existing SDK at the Antix base URL and authenticate with a Virtual Key.

## Base URLs

- **Production:** `https://antix.antigma.ai/v1`
- **Local development:** `http://127.0.0.1:8080/v1`

## Getting a key

Sign in at the Antix portal at [https://portal.antigma.ai](https://portal.antigma.ai) and create a Virtual Key from your dashboard. Portal-issued keys start with **`sk-antix-…`**.

Keys are stored securely; you see the plaintext exactly once at creation.

## First request — curl

```bash
curl -X POST https://antix.antigma.ai/v1/chat/completions \
  -H "Authorization: Bearer sk-antix-<your-key>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-6",
    "messages": [{"role": "user", "content": "Write a rust function for fibonacci."}],
    "stream": true
  }'
```

## First request — OpenAI SDK

```python
from openai import OpenAI

client = OpenAI(
    base_url="https://antix.antigma.ai/v1",
    api_key="sk-antix-<your-key>",
)

response = client.chat.completions.create(
    model="claude-sonnet-4-6",
    messages=[{"role": "user", "content": "Write a rust function for fibonacci."}],
    stream=True,
)

for chunk in response:
    print(chunk.choices[0].delta.content or "", end="")
```

## First request — Anthropic SDK

Antix implements the Anthropic Messages API natively at `/v1/messages`, so you can point the Anthropic SDK at Antix with no code changes:

```python
from anthropic import Anthropic

client = Anthropic(
    base_url="https://antix.antigma.ai",
    api_key="sk-antix-<your-key>",
)

message = client.messages.create(
    model="claude-sonnet-4-6",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Hello!"}],
)
```

## First request — Claude Code

Claude Code speaks the Anthropic Messages protocol, so pointing it at Antix is a one-liner:

```bash
export ANTHROPIC_BASE_URL="https://antix.antigma.ai"
```

That's it — Claude Code's SDK reads both and routes all `/v1/messages` traffic to Antix, which passes it through to Anthropic with your platform key substituted.

## Supported endpoints

| Endpoint | Method | Purpose |
|---|---|---|
| `/v1/chat/completions` | POST | OpenAI Chat Completions |
| `/v1/responses` | POST | OpenAI Responses API |
| `/v1/messages` | POST | Anthropic Messages |
| `/v1/messages/count_tokens` | POST | Anthropic token counter |
| `/v1/models/{action}` | POST | Gemini `:generateContent` / `:streamGenerateContent` |
| `/v1beta/models/{action}` | POST | Gemini v1beta path |
| `/v1/models`, `/models` | GET | Public model catalog |
| `/v2/model/info` | GET | Catalog with pricing |

Not supported: `/v1/embeddings`, `/v1/audio/*`, `/v1/images/*`, `/v1/files`, fine-tuning, batch API.

## Authentication modes

- **Virtual Key** — `Authorization: Bearer sk-antix-…` on proxy routes.
- **BYOK** — send your own provider key in `Authorization` and set `X-Antix-Provider`. See [Routing](/antix/concepts/routing).

## Tagging traffic with an Endpoint

The base URLs above are shared across your organization. To get **per-application** spend, traces, and agent-session analytics, create an **Endpoint** in the portal and use its URL instead. An endpoint URL looks like:

```
https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>
```

Every request through that URL is automatically tagged with the endpoint's ID, so the portal can break down cost, latency, and traces per endpoint. Authentication still uses your Virtual Key (or BYOK) — endpoints decide *where the traffic lands*, not *who pays*.

See [Endpoints](/antix/concepts/endpoints) for creation, scopes, and the analytics tabs.

## Next steps

- [Endpoints](/antix/concepts/endpoints) — per-application URLs with traces, spend, and agent sessions.
- [Routing & BYOK](/antix/concepts/routing) — provider selection and OpenAI-compatible semantics.
- [Virtual keys](/antix/concepts/virtual-keys) — provision keys with hard budgets and rate limits.
- [Error handling](/antix/concepts/error-handling) — standardized codes across providers.
