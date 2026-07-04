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

Org Admins can invite members, assign roles, and configure organization settings (like default rate limits and allowed providers) directly from the Antix portal dashboard at [https://portal.antigma.ai](https://portal.antigma.ai).

**Revocation**
Access can be revoked instantly via the portal, which immediately invalidates all associated keys and active sessions for the offboarded member.
