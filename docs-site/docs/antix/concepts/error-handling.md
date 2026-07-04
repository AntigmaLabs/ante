---
title: "Error Handling"
description: "Standardized error codes."
sidebar_position: 4
---

# Error Handling

Because Antix sits on the critical path of your applications, understanding its error codes is essential.

## Standardized error codes {#errors}

Antix intercepts and standardizes errors across providers:

- **`400 Bad Request`** — malformed payload or requesting a model unsupported by the upstream.
- **`401 Unauthorized`** — missing or malformed `Authorization` header, unknown virtual key, revoked or deleted key, expired JWT (past the 15-minute TTL), or JWT `jti` found in the blocklist.
- **`402 Payment Required`** — the Virtual Key has exceeded its `max_budget`.
- **`403 Forbidden`** — the caller's credentials don't have access to the requested [Endpoint](/antix/concepts/endpoints) or organization scope.
- **`404 Not Found`** — unknown endpoint UUID, or (on the [Models API](/antix/concepts/models)) a model id that isn't currently served.
- **`413 Payload Too Large`** — the request body exceeded the upstream provider's size limit.
- **`410 Gone`** — the request targeted an [Endpoint](/antix/concepts/endpoints) that has been archived. Archiving is permanent; this URL will never become active again.
- **`429 Too Many Requests`** — the gateway's request-rate limit was exceeded. This is currently a global per-IP limit, not a per-Virtual-Key quota.
- **`502 Bad Gateway`** — every upstream provider attempt for the request failed.
- **`503 Service Unavailable`** — Antix's fail-closed path. Triggered when the billing backend is unreachable, or when a requested model has no pricing row (`model_not_priced`).
