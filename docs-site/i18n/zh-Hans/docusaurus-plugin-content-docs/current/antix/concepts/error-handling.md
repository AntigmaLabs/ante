---
title: "错误处理"
description: "标准化的错误代码。"
sidebar_position: 4
---

# 错误处理

因为 Antix 处于您的应用程序的关键路径上，理解它的错误代码是至关重要的。

## 标准化的错误代码 {#errors}

Antix 在不同的提供商之间拦截并标准化了错误：

- **`400 Bad Request`** — 格式错误的有效载荷，或请求了上游不支持的模型。
- **`401 Unauthorized`** — 缺失或格式错误的 `Authorization` 请求头、未知的虚拟密钥、被撤销或已删除的密钥、过期的 JWT（超过了 15 分钟的 TTL）或 JWT 的 `jti` 存在于黑名单中。
- **`402 Payment Required`** — 虚拟密钥超出了其 `max_budget`（最大预算）。
- **`403 Forbidden`** — 调用者的凭据无权访问所请求的 [端点](/antix/concepts/endpoints) 或组织作用域。
- **`404 Not Found`** — 未知的端点 UUID，或者（在 [模型 API](/antix/concepts/models) 上）一个当前未提供的模型 id。
- **`413 Payload Too Large`** — 请求主体的体积超出了上游提供商的大小限制。
- **`410 Gone`** — 请求指向了一个已被归档的 [端点](/antix/concepts/endpoints)。归档是永久性的；该 URL 永远不会再次被激活。
- **`429 Too Many Requests`** — 超出了网关的请求速率限制。当前这是全局针对每 IP 的限制，而不是每个虚拟密钥的配额。
- **`502 Bad Gateway`** — 针对该请求的所有上游提供商尝试均告失败。
- **`503 Service Unavailable`** — Antix 的故障关闭路径。当计费后端不可达，或者当请求的模型没有定价记录（`model_not_priced`）时触发。
