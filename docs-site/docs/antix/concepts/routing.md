---
title: "Routing & BYOK"
description: "Endpoints, multi-protocol SDK compatibility, and Bring Your Own Key semantics."
sidebar_position: 1
---

# Routing & BYOK

Antix is a multi-protocol gateway. It accepts requests in OpenAI, Anthropic, and Gemini shapes and normalizes streaming across upstream providers including Anthropic, Google Gemini, Alibaba Qwen, xAI, and OpenAI.

## Supported endpoints

| Endpoint | Method | Protocol |
|---|---|---|
| `/v1/chat/completions` | POST | OpenAI Chat Completions |
| `/v1/responses` | POST | OpenAI Responses API |
| `/v1/messages` | POST | Anthropic Messages |
| `/v1/messages/count_tokens` | POST | Anthropic token counter |
| `/v1/models/{action}` | POST | Gemini native (`:generateContent`, `:streamGenerateContent`) |
| `/v1beta/models/{action}` | POST | Gemini v1beta native path |
| `/v1/models`, `/models` | GET | Public model catalog (no auth) |
| `/v2/model/info` | GET | Catalog with pricing |

`/v1/embeddings`, audio, images, files, fine-tuning, and the batch API are **not** supported.

## Drop-in SDK compatibility

Point any OpenAI, Anthropic, or Gemini SDK at Antix by changing the base URL and swapping in your Virtual Key. The proxy translates provider-specific quirks and keeps SSE streaming predictable.

```python
from openai import OpenAI

client = OpenAI(
    base_url="https://antix.antigma.ai/v1",
    api_key="sk-antix-<your-key>",
)

response = client.chat.completions.create(
    model="claude-sonnet-4-6",
    messages=[{"role": "user", "content": "Hello!"}],
    stream=True,
)
```

A streaming pipeline normalizes SSE events across providers so token deltas, tool calls, and stop reasons arrive in a consistent shape regardless of upstream.

### Claude Code

Claude Code uses the Anthropic SDK internally, so redirecting it at Antix takes one environment variable:

```bash
export ANTHROPIC_BASE_URL="https://antix.antigma.ai"
```

That's it — Claude Code's SDK reads both and routes all `/v1/messages` traffic to Antix, which passes it through to Anthropic with your platform key substituted.

## Bring Your Own Key (BYOK)

If you have negotiated direct rates with a provider but still want Antix's observability and routing, use BYOK. Send your provider key in `Authorization` and declare the provider with `X-Antix-Provider`:

```bash
curl -X POST https://antix.antigma.ai/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_ALIBABA_DASHSCOPE_KEY" \
  -H "X-Antix-Provider: alibaba" \
  -d '{
    "model": "qwen-max",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

BYOK traffic is not re-billed by Antix. It is still tracked for observability.

### Accepted `X-Antix-Provider` values

| Provider | Accepted values |
|---|---|
| OpenAI | `openai` |
| Anthropic | `anthropic` |
| Google Gemini | `google`, `gemini`, `google_ai_studio_gemini` |
| xAI | `xai`, `x-ai` |
| Alibaba / DashScope | `alibaba`, `qwen`, `dashscope` |

:::note Provider inference
When the header is omitted, Antix infers the provider from the key prefix (e.g., `sk-ant-…` → Anthropic, `sk-…` → OpenAI). Some keys (like Alibaba/DashScope or ZAI) may have no distinctive prefix — you **must** set `X-Antix-Provider` for those requests, otherwise they will fail upstream with `401 Unauthorized`.
:::
Unauthorized`.
:::
