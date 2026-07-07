---
title: "快速入门"
description: "在 5 分钟内向 Antix 发出您的第一个请求。"
sidebar_position: 2
---

# 快速入门 {#quickstart}

Antix 在同一个代理上支持四种网络协议——OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 和 Gemini 原生协议。只需将任何现有 SDK 指向 Antix 基地址，并使用虚拟密钥进行验证即可。

## 基地址（Base URLs） {#base-urls}

- **生产环境：** `https://antix.antigma.ai/v1`
- **本地开发环境：** `http://127.0.0.1:8080/v1`

## 登录 {#signing-in}

在 [https://portal.antigma.ai](https://portal.antigma.ai) 的 Antix 门户默认无密码登录。输入您的邮箱并点击 **Send link（发送链接）**——门户会给您发送一封带有魔术链接的邮件，点击即可登录（无需输入密码或验证码）。Google 和 GitHub OAuth 也作为一键登录选项在同一页面提供。

## 获取密钥 {#getting-a-key}

登录后，从您的仪表板创建一个虚拟密钥。门户发出的密钥以 **`sk-antix-…`** 开头。

密钥会被安全存储；您仅在创建时能看到一次明文。

## 第一个请求 — curl {#first-request--curl}

```bash
curl -X POST https://antix.antigma.ai/v1/chat/completions \
  -H "Authorization: Bearer sk-antix-<your-key>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-6",
    "messages": [{"role": "user", "content": "Write a rust function for fibonacci."}],
    "stream": true
  }'
```

## 第一个请求 — OpenAI SDK {#first-request--openai-sdk}

```python
from openai import OpenAI

client = OpenAI(
    base_url="https://antix.antigma.ai/v1",
    api_key="sk-antix-<your-key>",
)

response = client.chat.completions.create(
    model="claude-sonnet-4-6",
    messages=[{"role": "user", "content": "Write a rust function for fibonacci."}],
    stream=True,
)

for chunk in response:
    print(chunk.choices[0].delta.content or "", end="")
```

## 第一个请求 — Anthropic SDK {#first-request--anthropic-sdk}

Antix 在 `/v1/messages` 原生实现了 Anthropic Messages API，因此您无需更改代码即可将 Anthropic SDK 指向 Antix：

```python
from anthropic import Anthropic

client = Anthropic(
    base_url="https://antix.antigma.ai",
    api_key="sk-antix-<your-key>",
)

message = client.messages.create(
    model="claude-sonnet-4-6",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Hello!"}],
)
```

## 第一个请求 — Claude Code {#first-request--claude-code}

Claude Code 使用 Anthropic Messages 协议，因此将其指向 Antix 只需要一行命令：

```bash
export ANTHROPIC_BASE_URL="https://antix.antigma.ai"
```

就是这样——Claude Code 的 SDK 会读取这个配置并将所有的 `/v1/messages` 流量路由到 Antix，然后 Antix 在替换您的平台密钥后原样传给 Anthropic。

## 支持的端点 {#supported-endpoints}

| 端点 | 方法 | 用途 |
|---|---|---|
| `/v1/chat/completions` | POST | OpenAI Chat Completions |
| `/v1/responses` | POST | OpenAI Responses API |
| `/v1/messages` | POST | Anthropic Messages |
| `/v1/messages/count_tokens` | POST | Anthropic 代币计数器 |
| `/v1/models/{action}` | POST | Gemini `:generateContent` / `:streamGenerateContent` |
| `/v1beta/models/{action}` | POST | Gemini v1beta 路径 |
| `/v1/models`, `/models` | GET | 公开模型目录 |
| `/v2/model/info` | GET | 带定价的目录 |

不支持：`/v1/embeddings`，`/v1/audio/*`，`/v1/images/*`，`/v1/files`，微调，批处理 API。

## 身份验证模式 {#authentication-modes}

- **虚拟密钥（Virtual Key）** — 代理路由上的 `Authorization: Bearer sk-antix-…`。
- **自带密钥（BYOK）** — 在 `Authorization` 中发送您自己的提供商密钥，并设置 `X-Antix-Provider`。见[路由](/antix/concepts/routing)。

## 使用端点标记流量 {#tagging-traffic-with-an-endpoint}

上述基地址在您的整个组织中共享。要获得**按应用程序**分类的支出、追踪和代理会话分析，请在门户中创建一个**端点（Endpoint）**并使用其 URL 代替。端点 URL 的格式如下：

```
https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>
```

流经该 URL 的每个请求都会自动打上该端点 ID 的标签，以便门户能够按端点细分成本、延迟和追踪。身份验证仍然使用您的虚拟密钥（或 BYOK）——端点决定了*流量的去向*，而不是*谁付费*。

关于端点创建、作用域限制和分析选项卡的内容，请参阅[端点](/antix/concepts/endpoints)。

## 后续步骤 {#next-steps}

- [端点](/antix/concepts/endpoints) — 按应用程序分类的 URL，用于追踪、支出和代理会话分析。
- [路由与 BYOK](/antix/concepts/routing) — 提供商选择以及兼容 OpenAI 的语义。
- [虚拟密钥](/antix/concepts/virtual-keys) — 配置带有硬性支出预算的密钥。
- [错误处理](/antix/concepts/error-handling) — 跨提供商的标准代码。
