---
title: "快速入门"
description: "在5分钟内向 Antix 发起您的第一个请求。"
sidebar_position: 2
---

# 快速入门

Antix 在同一个代理上支持四种网络协议——OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 和 Gemini 原生协议。将任何现有的 SDK 指向 Antix 基础 URL 并使用虚拟密钥（Virtual Key）进行身份验证。

## 基础 URL

- **生产环境：** `https://antix.antigma.ai/v1`
- **本地开发：** `http://127.0.0.1:8080/v1`

## 登录

Antix 门户网站 [https://portal.antigma.ai](https://portal.antigma.ai) 默认采用无密码登录。输入您的电子邮件并点击 **Send link**（发送链接）——门户网站会向您发送一封带有魔法链接的电子邮件，点击该链接即可登录（无需输入密码或验证码）。同一登录页面上还提供 Google 和 GitHub OAuth 作为一键登录的替代选项。

## 获取密钥

登录后，从您的仪表板创建一个虚拟密钥。门户网站签发的密钥以 **`sk-antix-…`** 开头。

密钥被安全地存储；您只会在创建时看到一次明文。

## 第一个请求 — curl

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

## 第一个请求 — OpenAI SDK

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

## 第一个请求 — Anthropic SDK

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

## 第一个请求 — Claude Code

Claude Code 使用 Anthropic Messages 协议，因此只需一行代码即可将其指向 Antix：

```bash
export ANTHROPIC_BASE_URL="https://antix.antigma.ai"
```

就是这样——Claude Code 的 SDK 会读取该环境变量，并将所有 `/v1/messages` 流量路由到 Antix，而 Antix 会将其传递给 Anthropic 并替换为您的平台密钥。

## 支持的端点

| 端点 | 方法 | 目的 |
|---|---|---|
| `/v1/chat/completions` | POST | OpenAI Chat Completions |
| `/v1/responses` | POST | OpenAI Responses API |
| `/v1/messages` | POST | Anthropic Messages |
| `/v1/messages/count_tokens` | POST | Anthropic token counter |
| `/v1/models/{action}` | POST | Gemini `:generateContent` / `:streamGenerateContent` |
| `/v1beta/models/{action}` | POST | Gemini v1beta 路径 |
| `/v1/models`, `/models` | GET | 公开模型目录 |
| `/v2/model/info` | GET | 包含定价的目录 |

不支持的内容：`/v1/embeddings`, `/v1/audio/*`, `/v1/images/*`, `/v1/files`, 微调（fine-tuning）, 批处理 API（batch API）。

## 身份验证模式

- **虚拟密钥（Virtual Key）** — 代理路由上的 `Authorization: Bearer sk-antix-…`。
- **自带密钥（BYOK）** — 在 `Authorization` 中发送您自己的提供商密钥，并设置 `X-Antix-Provider`。参见 [路由](/antix/concepts/routing)。

## 使用端点标记流量

上述基础 URL 在您的组织内是共享的。要获取**每个应用程序**的花费、追踪和智能体回话分析，请在门户网站中创建一个 **端点（Endpoint）** 并使用其 URL。端点 URL 的格式如下：

```
https://antix.antigma.ai/v1/<endpoint_uuid>/<provider>
```

每个通过该 URL 发送的请求都会自动标记为该端点的 ID，因此门户网站可以按端点细分成本、延迟和追踪。身份验证仍然使用您的虚拟密钥（或 BYOK）——端点决定了*流量的去向*，而不是*谁来买单*。

参见 [端点](/antix/concepts/endpoints) 了解创建方法、作用域以及分析选项卡。

## 下一步

- [端点](/antix/concepts/endpoints) — 包含追踪、花费和智能体回话的每个应用程序级 URL。
- [路由和自带密钥（BYOK）](/antix/concepts/routing) — 提供商选择和 OpenAI 兼容语义。
- [虚拟密钥](/antix/concepts/virtual-keys) — 签发具有硬性支出预算的密钥。
- [错误处理](/antix/concepts/error-handling) — 跨提供商的标准化错误代码。
