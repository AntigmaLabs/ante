---
title: "错误处理"
description: "标准化的错误代码。"
sidebar_position: 4
---

# 错误处理 {#error-handling}

由于 Antix 处于您应用程序的关键路径上，因此了解其错误代码至关重要。

## 标准化的错误代码 {#errors}

Antix 拦截并标准化了跨多个提供商的错误：

- **`400 Bad Request`（错误请求）** — 载荷格式错误或请求了上游不支持的模型。
- **`401 Unauthorized`（未授权）** — 缺少或格式错误的 `Authorization` 标头、未知的虚拟密钥、密钥已撤销或删除、JWT 已过期（超过 15 分钟的生存时间（TTL）），或者在阻止列表中找到了 JWT `jti`。
- **`402 Payment Required`（需要付款）** — 虚拟密钥已超出其 `max_budget`（最大预算）。
- **`403 Forbidden`（禁止访问）** — 调用者的凭据无权访问所请求的[端点](/antix/concepts/endpoints)或组织范围。
- **`404 Not Found`（未找到）** — 未知的端点 UUID，或者（在[模型 API](/antix/concepts/models) 上）模型 id 当前未被提供服务。
- **`413 Payload Too Large`（载荷过大）** — 请求主体超过了上游提供商的大小限制。
- **`410 Gone`（已失效）** — 请求针对了一个已被归档的[端点](/antix/concepts/endpoints)。归档是永久性的；此 URL 将永远不会再次变为活动状态。
- **`429 Too Many Requests`（请求过多）** — 超出了网关的请求速率限制。这目前是一个全局的每个 IP 限制，而不是按虚拟密钥（Virtual-Key）分配的配额。
- **`502 Bad Gateway`（错误的网关）** — 针对该请求的所有上游提供商尝试均失败。
- **`503 Service Unavailable`（服务不可用）** — Antix 的故障关闭路径。当无法访问计费后端或所请求的模型没有定价行（`model_not_priced`）时触发。
