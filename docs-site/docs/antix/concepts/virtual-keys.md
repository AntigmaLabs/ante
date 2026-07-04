---
title: "Virtual Keys & Budgets"
description: "Scoped keys with atomic rate limits and hard spend caps."
sidebar_position: 3
---

# Virtual Keys & Hard Budgets

Never distribute raw provider keys. Antix issues **Virtual Keys** that act as middleware interceptors, atomically validating budgets before routing traffic upstream.

## Creating a key

Keys can be generated directly from the Antix portal dashboard at [https://portal.antigma.ai](https://portal.antigma.ai).

When creating a key, you can set:
- **Key Name:** a label to identify the key in the portal and traces.
- **Budget:** an optional hard spend cap (`max_budget` in USD). There is no day/month/lifetime period picker today — it's a single flat cap.

Keys are stored securely — plaintext is returned **exactly once** at creation. Portal-issued keys start with **`sk-antix-…`**.

:::note
Per-key model allow-lists and per-key rate limits (rpm/tpm) are not yet exposed in the portal's key-creation flow. See [Error Handling](/antix/concepts/error-handling) for the gateway's current (global, not per-key) rate limiting.
:::

## Reliable Billing

Antix strictly enforces budgets to prevent overruns. Costs are estimated and reserved before any request goes upstream. If a request would exceed the key's `max_budget`, it is instantly rejected with a `402 Payment Required` error.

Once a request completes or is cancelled, costs are accurately reconciled. This strict enforcement eliminates double-spending and ensures that your budget caps are strictly adhered to, even under heavy concurrent load.
