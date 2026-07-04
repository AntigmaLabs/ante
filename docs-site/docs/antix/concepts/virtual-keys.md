---
title: "Virtual Keys & Budgets"
description: "Scoped keys with atomic rate limits and hard spend caps."
sidebar_position: 3
---

# Virtual Keys, Rate Limits & Hard Budgets

Never distribute raw provider keys. Antix issues **Virtual Keys** that act as middleware interceptors, atomically validating budgets and rate limits before routing traffic upstream.

## Creating a key

Keys can be generated directly from the Antix portal dashboard at [https://portal.antigma.ai](https://portal.antigma.ai). 

When creating a key, you can configure:
- **Models:** Restrict which models the key can access.
- **Budget:** Hard spend cap (`max_budget`) per day, month, or lifetime.
- **Rate Limits:** Maximum requests or tokens per minute.

Keys are stored securely — plaintext is returned **exactly once** at creation.

## Reliable Billing

Antix strictly enforces budgets to prevent overruns. Costs are estimated and reserved before any request goes upstream. If a request would exceed the key's `max_budget`, it is instantly rejected with a `402 Payment Required` error. 

Once a request completes or is cancelled, costs are accurately reconciled. This strict enforcement eliminates double-spending and ensures that your budget caps are strictly adhered to, even under heavy concurrent load.
