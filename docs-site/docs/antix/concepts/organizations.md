---
title: "Organizations & RBAC"
description: "Multi-tenant boundaries and roles."
sidebar_position: 2
---

# Organizations & RBAC

Antix is multi-tenant by design. **Organizations** are the top-level billing and access boundary — users, virtual keys, spend, and analytics are all scoped to an organization.

## Role model

There are two organization roles:

- **Admin** — manages members, invitations, virtual keys, budgets, and organization settings.
- **Member** — consumes APIs using provided keys. Cannot manage members, view pending invites, or change organization settings.

Role enforcement ensures that operations are securely gated by checking the caller's role before mutating state.


## Managing members

Org Admins can invite members, assign roles, and remove members directly from the Antix portal dashboard at [https://portal.antigma.ai](https://portal.antigma.ai).

**Revocation**
Removing a member from the portal archives their personal endpoints and removes their organization membership — near-instant, since the resolver cache is invalidated synchronously rather than waiting on a caching TTL.

:::note
Member removal does **not** revoke the member's Virtual Keys or active OAuth session. A former member's key keeps working against org-shared endpoints until it is separately revoked. For a full cutoff, revoke the member's keys explicitly or delete their account.
:::
