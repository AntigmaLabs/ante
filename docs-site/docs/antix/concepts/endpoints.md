---
title: "Endpoints"
description: "Stable per-application URLs that tag traffic for spend, traces, and agent sessions."
sidebar_position: 4
---

# Endpoints

An **Endpoint** is a stable, unique URL (e.g., `https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>/...`) that acts as a drop-in replacement for a provider's base URL. Every request that flows through it is automatically tagged with the endpoint's ID, giving you per-application visibility into:

- **Spend & usage** — token counts and dollar cost, per endpoint.
- **Request traces** — full request/response lifecycle, including latency and per-call cost.
- **Agent sessions** — multi-turn requests grouped into cohesive sessions for debugging.

Endpoints are independent of authentication: they decide *where* a request lands and *how it's tagged*, while [Virtual Keys](/antix/concepts/virtual-keys) (or BYOK) decide *who pays* and *what limits apply*.

## Creating an endpoint

Endpoints are created from the **Endpoints** tab of the [Antix Portal](https://portal.antigma.ai).

### Scope: Personal vs. Organization

When creating an endpoint, you choose its scope:

- **Personal** — visible only to you. Useful for local development or experimentation. Capped at **10 personal endpoints per user, per organization**.
- **Org-shared** — visible to all members of the organization. Only **organization Admins** can create Org-shared endpoints. Org-shared endpoints share a separate, larger cap (50 by default) configured per organization.

### Steps

1. Open the **Endpoints** tab in the portal.
2. In the **Create Endpoint** card, pick the organization the endpoint belongs to.
3. Select **Personal** or **Org-shared**.
4. Enter a **Display Name** (e.g., `CI Runner`, `Frontend Production`).
5. Click **Create**.

:::note
The Display Name can be edited later. The generated URL — which embeds a UUID — is **fixed for the lifetime of the endpoint** and will never change.
:::

## Using your endpoint URL

Click into an endpoint to see its **Provider base URLs**. Antix supports multiple AI providers behind a single endpoint UUID; you append the provider name and the native API path to the endpoint's base URL.

### URL structure

```
https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>/<native_path>
```

In the portal, find the row matching your SDK, click **Copy**, and paste it into your application's `base_url` configuration.

### By provider SDK

- **OpenAI SDK (and OpenAI-compatible SDKs)**
  - Base URL: `https://antix.antigma.ai/v1/<endpoint_uuid>/openai`
  - The SDK appends `/v1/chat/completions` automatically.
- **Anthropic SDK / Claude Code**
  - Base URL: `https://antix.antigma.ai/v1/<endpoint_uuid>/anthropic`
  - The SDK appends `/v1/messages` automatically.
- **Gemini (Google AI Studio)**
  - Base URL: `https://antix.antigma.ai/v1/<endpoint_uuid>/gemini`
- **xAI, Alibaba (Qwen), DeepSeek, Zai**
  - Base URL: `https://antix.antigma.ai/v1/<endpoint_uuid>/<xai|alibaba|deepseek|zai>`
  - Append the provider's own native path, or use the equivalent OpenAI-compatible SDK against these providers.
- **Universal / multi-provider**
  - Base URL: `https://antix.antigma.ai/v1/<endpoint_uuid>/multi`
  - With `/multi`, Antix routes the request based on the model name and headers rather than a fixed provider. See [Routing & BYOK](/antix/concepts/routing) for the routing rules and accepted `X-Antix-Provider` values.

:::warning
Treat endpoint URLs as **secrets**. They identify your traffic and are easy to leak via committed config files. Do not push them to public repositories.
:::

## Monitoring & analytics

The endpoint detail page has four tabs.

### Overview

A 30-day summary of activity:

- **Requests** — total requests processed.
- **Tokens** — total prompt and completion tokens.
- **Cost (Antix billed)** — total cost incurred via Antix platform credentials.
- If the endpoint receives BYOK traffic, an estimated passthrough cost is shown beneath the main figure.

### Traces

A log explorer for recent requests:

- **Recent requests** — table with time, auth mode (Virtual Key, BYOK, OAuth), model, token counts, cost, and latency.
- **Filter & search** — collapsible filter strip narrows by Time Range, Auth Mode, Model, or Virtual Key ID, plus substring search across input and output bodies.
- **Observation drawer** — click a row to slide open the full JSON request and response with syntax highlighting, exact TTFT, and per-call cost.

:::note
Some sensitive fields (e.g., request headers, auth tokens) are omitted from the trace body entirely rather than shown, based on gateway policy.
:::

### Activity (Agent Sessions)

This tab is **experimental** and only appears on endpoints that receive traffic supporting session signatures (`/v1/messages`, `/v1/responses`) — typically multi-turn agent frameworks like Claude Code.

- **Session grouping** — Antix automatically groups related requests into sessions keyed off the initial user prompt. No client instrumentation is required.
- **Gantt chart** — pick a session from the sidebar to view a timeline of inference calls.
- **Deep dive** — clicking a bar opens the same observation drawer used in the Traces tab.

### Settings

Manage the endpoint's lifecycle and metadata:

- **Edit name** — update the display name.
- **Duplicate as Org Endpoint** *(personal endpoints only)* — Org Admins can promote a personal endpoint into a brand-new Org-shared endpoint with a *new* URL. The original personal endpoint stays active until you archive it.
- **Archive** — soft-delete the endpoint. Any client still pointing at the archived URL will immediately receive `410 Gone` (see [Error Handling](/antix/concepts/error-handling)). Historical traces and spend data remain visible in the portal. **Archiving cannot be undone.**

## Authentication

The endpoint URL routes traffic; you still authenticate the request via the `Authorization` header.

- **Virtual Keys (recommended)** — an Antix-issued key (`sk-antix-…`). The request is billed to your Antix organization under the limits set on that key. See [Virtual Keys](/antix/concepts/virtual-keys).
- **Bring Your Own Key (BYOK)** — send your raw provider key (e.g., `sk-ant-…`). Antix passes the request to the provider unchanged and does not bill you for it; the portal labels the cost with an *(est.)* badge.

Endpoints work with either auth mode — the choice is per-request, not per-endpoint.
