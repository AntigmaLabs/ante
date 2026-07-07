---
title: "路由和自带密钥 (BYOK)"
description: "端点、多协议 SDK 兼容性以及自带密钥（Bring Your Own Key）语义。"
sidebar_position: 1
---

# 路由和自带密钥 (BYOK)

Antix 是一个多协议网关。它接收具有 OpenAI、Anthropic 和 Gemini 结构形式的请求，并在包括 Anthropic、Google Gemini、Alibaba Qwen、DeepSeek、Zai (GLM)、xAI 和 OpenAI 在内的上游提供商之间规范化流式传输。

## 支持的端点

| 端点 | 方法 | 协议 |
|---|---|---|
| `/v1/chat/completions` | POST | OpenAI Chat Completions |
| `/v1/responses` | POST | OpenAI Responses API |
| `/v1/messages` | POST | Anthropic Messages |
| `/v1/messages/count_tokens` | POST | Anthropic token counter |
| `/v1/models/{action}` | POST | Gemini 原生 (`:generateContent`, `:streamGenerateContent`) |
| `/v1beta/models/{action}` | POST | Gemini v1beta 原生路径 |
| `/v1/models`, `/models` | GET | 公开模型目录（无身份验证） |
| `/v2/model/info` | GET | 包含定价的目录 |

**不支持** `/v1/embeddings`、音频（audio）、图像（images）、文件（files）、微调（fine-tuning）和批处理 API。

## 即插即用的 SDK 兼容性

通过更改基础 URL 并替换为您的虚拟密钥，即可将任何 OpenAI、Anthropic 或 Gemini SDK 指向 Antix。代理会转换特定于提供商的差异，并保持 SSE 流式传输的可预测性。

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

流式传输管道在各个提供商之间规范化了 SSE 事件，因此无论上游情况如何，令牌增量（token deltas）、工具调用和停止原因（stop reasons）都能以一致的结构到达。

### Claude Code

Claude Code 内部使用 Anthropic SDK，因此将其重定向至 Antix 只需要一个环境变量：

```bash
export ANTHROPIC_BASE_URL="https://antix.antigma.ai"
```

就是这样——Claude Code 的 SDK 会读取该环境变量并将所有 `/v1/messages` 流量路由至 Antix，而 Antix 会将其传递给 Anthropic，并替换为您的平台密钥。

## 自带密钥 (BYOK)

如果您已经与提供商协商了直接费率，但仍然希望获得 Antix 的可观测性和路由功能，请使用 BYOK。在 `Authorization` 中发送您的提供商密钥，并使用 `X-Antix-Provider` 声明提供商：

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

Antix 不会对 BYOK 流量重新计费。不过它仍然会被追踪以用于可观测性。

### 支持的提供商

Antix 在以下上游提供商中规范化请求。您可以按模型名称路由到其中任何一个（Antix 会推断提供商），或者通过 `X-Antix-Provider` 显式路由：

| 提供商 | 知名模型 | 可接受的 `X-Antix-Provider` 值 | 备注 |
|---|---|---|---|
| **Anthropic** | Claude Sonnet, Opus, Haiku | `anthropic` | 原生支持 `/v1/messages`。 |
| **OpenAI** | GPT-5.x | `openai` | 原生支持 `/v1/chat/completions` 和 `/v1/responses`。 |
| **Google Gemini** | Gemini 3.x | `google`, `gemini`, `google_ai_studio_gemini` | `/v1/models/{action}` 和 `/v1beta` 处的原生 Gemini 协议。 |
| **xAI** | Grok | `xai`, `x-ai` | 兼容 OpenAI 的上游。 |
| **Alibaba Qwen** | `qwen3-max`, `qwen3-coder-plus`, `qwen3-coder-flash`, `qwq-plus`, `qwen-plus`, `qwen-max` 等 | `alibaba`, `qwen`, `dashscope` | 通过 DashScope 路由。没有独特的前缀——进行 BYOK 调用时，您**必须**设置 `X-Antix-Provider`。 |
| **DeepSeek** | `deepseek-chat` (V3), `deepseek-reasoner` (R1), `deepseek-v4-pro`, `deepseek-v4-flash` | `deepseek` | 兼容 OpenAI 的上游 API。`deepseek-reasoner` 会在最终答案旁返回一个 `reasoning_content` 字段。 |
| **Zai (GLM)** | `glm-5.2`, `glm-5.1`, `glm-4.7` | `zai` | 智谱 AI 的 GLM 模型家族，兼容 OpenAI 的上游。没有独特的前缀——进行 BYOK 调用时，您**必须**设置 `X-Antix-Provider`。 |

:::note 提供商推断
当省略该请求头时，Antix 会从密钥前缀中推断提供商（例如，`sk-ant-…` → Anthropic，`sk-…` → OpenAI）。某些密钥（如 Alibaba/DashScope 或 Zai）可能没有独特的前缀——您**必须**为这些请求设置 `X-Antix-Provider`，否则它们在上游将以 `401 Unauthorized` 失败。
:::

参见 [模型 API](/zh-Hans/antix/concepts/models) 以确切列出您的网关当前提供的模型。
