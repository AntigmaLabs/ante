---
title: "路由与 BYOK"
description: "端点、多协议 SDK 兼容性以及自带密钥（Bring Your Own Key）语义。"
sidebar_position: 1
---

# 路由与 BYOK {#routing--byok}

Antix 是一个多协议网关。它接受 OpenAI、Anthropic 和 Gemini 数据结构的请求，并将跨越上游提供商的流式传输规范化，包括 Anthropic、Google Gemini、Alibaba Qwen、DeepSeek、Zai (GLM)、xAI 和 OpenAI。

## 支持的端点 {#supported-endpoints}

| 端点 | 方法 | 协议 |
|---|---|---|
| `/v1/chat/completions` | POST | OpenAI Chat Completions |
| `/v1/responses` | POST | OpenAI Responses API |
| `/v1/messages` | POST | Anthropic Messages |
| `/v1/messages/count_tokens` | POST | Anthropic token counter |
| `/v1/models/{action}` | POST | Gemini 原生（`:generateContent`, `:streamGenerateContent`） |
| `/v1beta/models/{action}` | POST | Gemini v1beta 原生路径 |
| `/v1/models`, `/models` | GET | 公开模型目录（无身份验证） |
| `/v2/model/info` | GET | 包含定价的目录 |

**不**支持 `/v1/embeddings`、音频、图像、文件、微调（fine-tuning）和批处理（batch）API。

## 即插即用的 SDK 兼容性 {#drop-in-sdk-compatibility}

通过更改基地址（base URL）并换上您的虚拟密钥，将任何 OpenAI、Anthropic 或 Gemini SDK 指向 Antix。代理会转换特定于提供商的特性，并保持 SSE 流的传输可预测。

```python
from openai import OpenAI

client = OpenAI(
    base_url="https://antix.antigma.ai/v1",
    api_key="sk-antix-<your-key>",
)

response = client.chat.completions.create(
    model="claude-sonnet-4-6",
    messages=[{"role": "user", "content": "Hello!"}],
    stream=True,
)
```

流式传输管道将跨提供商的 SSE 事件标准化，因此无论上游情况如何，代币增量、工具调用和停止原因都将以一致的数据结构到达。

### Claude Code {#claude-code}

Claude Code 内部使用 Anthropic SDK，因此将其重定向到 Antix 只需要一个环境变量：

```bash
export ANTHROPIC_BASE_URL="https://antix.antigma.ai"
```

就是这样——Claude Code 的 SDK 会读取这两个配置，并将所有的 `/v1/messages` 流量路由到 Antix，Antix 在替换您的平台密钥后将其原样传递给 Anthropic。

## 自带密钥（Bring Your Own Key, BYOK） {#bring-your-own-key-byok}

如果您已经与提供商协商了直接费率，但仍然想要 Antix 的可观测性和路由功能，请使用 BYOK。在 `Authorization` 中发送您的提供商密钥，并通过 `X-Antix-Provider` 声明提供商：

```bash
curl -X POST https://antix.antigma.ai/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_ALIBABA_DASHSCOPE_KEY" \
  -H "X-Antix-Provider: alibaba" \
  -d '{
    "model": "qwen-max",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

Antix 不会对 BYOK 流量重新计费。为了可观测性，它仍会被追踪记录。

### 支持的提供商 {#supported-providers}

Antix 在这些上游提供商之间将请求标准化。您可以通过模型名称（Antix 会推断提供商）或通过 `X-Antix-Provider` 明确地路由到其中的任何一个：

| 提供商 | 显著的模型 | 可接受的 `X-Antix-Provider` 值 | 备注 |
|---|---|---|---|
| **Anthropic** | Claude Sonnet, Opus, Haiku | `anthropic` | 原生 `/v1/messages` 支持。 |
| **OpenAI** | GPT-5.x | `openai` | 原生 `/v1/chat/completions` 和 `/v1/responses` 支持。 |
| **Google Gemini** | Gemini 3.x | `google`, `gemini`, `google_ai_studio_gemini` | `/v1/models/{action}` 和 `/v1beta` 的原生 Gemini 协议。 |
| **xAI** | Grok | `xai`, `x-ai` | 兼容 OpenAI 的上游。 |
| **Alibaba Qwen** | `qwen3-max`, `qwen3-coder-plus`, `qwen3-coder-flash`, `qwq-plus`, `qwen-plus`, `qwen-max`, 等等 | `alibaba`, `qwen`, `dashscope` | 通过 DashScope 路由。没有独特的密钥前缀——您**必须**在 BYOK 调用中设置 `X-Antix-Provider`。 |
| **DeepSeek** | `deepseek-chat` (V3), `deepseek-reasoner` (R1), `deepseek-v4-pro`, `deepseek-v4-flash` | `deepseek` | 兼容 OpenAI 的上游 API。`deepseek-reasoner` 会在最终答案旁返回 `reasoning_content` 字段。 |
| **Zai (GLM)** | `glm-5.2`, `glm-5.1`, `glm-4.7` | `zai` | 智谱 AI 的 GLM 模型系列，兼容 OpenAI 的上游。没有独特的密钥前缀——您**必须**在 BYOK 调用中设置 `X-Antix-Provider`。 |

:::note 提供商推断
当标头被省略时，Antix 会从密钥前缀推断提供商（例如，`sk-ant-…` → Anthropic，`sk-…` → OpenAI）。某些密钥（如 Alibaba/DashScope 或 Zai）可能没有明显的前缀——您**必须**为这些请求设置 `X-Antix-Provider`，否则它们在上游将因 `401 Unauthorized` 失败。
:::

请参阅[模型 API](/antix/concepts/models)以准确列出您的网关目前提供服务的模型。
