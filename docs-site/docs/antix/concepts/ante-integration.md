---
title: "Ante Control Plane"
description: "Govern local coding agents with PKCE, attribution, and revocation."
sidebar_position: 1
---

# Native Agent Integration

Antix is the centralized control plane for the **Ante CLI**, bringing enterprise governance to local coding agents.

## Connecting Ante to Antix

Developers authenticate their local Ante instance against your Antix server:

```bash
ante auth login
```

This starts the Antix OAuth flow without launching an agent session. Use `ante auth login --no-browser` on a remote machine where Ante should print the authorization URL instead of opening a browser.

You can also sign in from the TUI:

1. Start the `ante` CLI.
2. Type `/connect`.
3. Select **Antix** from the menu.

This securely authenticates your local agent with the Antix control plane, granting it a temporary access token that automatically refreshes as needed.

## Benefits

- **Cost attribution** — every prompt run from an engineer's Ante CLI is attributed to their user ID in the billing ledger and analytics timeline.
- **Model governance** — restrict which models local agents can use via organization-level model defaults.
- **Partial offboarding** — removing a member in the portal archives their personal endpoints, cutting off traffic through those URLs within seconds (see [Organizations](/antix/concepts/organizations)). This does **not** revoke the member's Virtual Key or active OAuth session — for a full cutoff, revoke their keys or delete their account.
