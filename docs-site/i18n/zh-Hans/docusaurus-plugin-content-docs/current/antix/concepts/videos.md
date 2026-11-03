---
title: "视频模型与 API"
description: "通过 Antix 使用 Seedance 生成视频，或使用自己的提供商密钥调用 MiniMax H3。"
sidebar_position: 5
---

# 视频模型与 API {#video-models--api}

视频生成采用异步流程：提交任务、轮询结果，然后保存视频。Antix 提供两套独立的 API，其凭据与响应格式不同。

| | BytePlus ModelArk 上的 Seedance | MiniMax H3 / H3 Max |
| --- | --- | --- |
| 创建 | `POST /v1/videos` | `POST /v1/minimax/video_generation` |
| 轮询 | `GET /v1/videos/{id}` | `GET /v1/minimax/query/video_generation/{task_id}` |
| 凭据 | Antix 虚拟密钥或 Ante CLI OAuth 访问令牌 | MiniMax 提供商密钥，以及 `X-Antix-Provider: minimax` |
| 计费 | Antix 账户余额 | MiniMax 向您的提供商账户计费；Antix 不重复收费 |
| 下载 | 需要身份验证的 Antix `/content` URL | MiniMax 原生响应中的 `task.content.url` |

这些不是 Chat Completions 端点。Seedance 路由接受 OpenRouter 风格的 JSON 子集；不要假设 OpenAI 视频 SDK 或 multipart 请求格式可以直接使用。

## Seedance 可用性与模型发现 {#seedance-availability-and-discovery}

使用您的 Antix 凭据查询该凭据可用的视频模型：

```bash
curl --fail-with-body https://antix.antigma.ai/v1/videos/models \
  -H "Authorization: Bearer $ANTIX_KEY"
```

响应包含 `data` 数组。每个模型提供 `supported_resolutions`、`supported_aspect_ratios`、`supported_sizes`、`supported_durations`、`supported_frame_images`、`allowed_passthrough_parameters` 和 `pricing_skus`。

当前已启用的模型是 **`seedance-2.5`**。`seedance-2.0-fast` 和 `seedance-2.0-mini` 已定义请求规范，但尚未启用。请以实时列表为准：模型还需要已配置的提供商凭据、定价，以及您密钥访问策略的许可。视频模型发现独立于文本[模型 API](/antix/concepts/models)。

在 `/v1/videos` 使用提供商 BYOK 密钥会收到 `400 video_requires_antix_key`。

## Seedance 请求 {#seedance-request}

```json
{
  "model": "seedance-2.5",
  "prompt": "A red kite over a beach at sunset",
  "duration": 5,
  "resolution": "720p",
  "aspect_ratio": "16:9"
}
```

| 字段 | 规则 |
| --- | --- |
| `model` | 必填；使用 `/v1/videos/models` 返回的 ID。 |
| `prompt` | 除非提供首帧图像，否则必填；最多 4,000 个字符。 |
| `duration` | 必填，整数秒。Seedance 2.5 支持 4–30 秒。 |
| `resolution` | 默认 `720p`；Seedance 2.5 支持 `480p`、`720p` 和 `1080p`。 |
| `aspect_ratio` | 文生视频默认 `16:9`。支持 `16:9`、`4:3`、`1:1`、`3:4`、`9:16` 和 `21:9`。 |
| `size` | `supported_sizes` 中的 `WIDTHxHEIGHT` 值，可代替分辨率和宽高比。 |
| `generate_audio` | 布尔值；默认 `true`。 |
| `seed` | 可选，-1 至 2147483647 之间的整数。 |
| `frame_images` | 首帧/尾帧输入。Seedance 2.5 使用此输入时，请省略 `aspect_ratio` 和 `size`，输出跟随首帧比例。 |
| `input_references` | 最多九张参考图像；不能与 `frame_images` 同时使用。 |
| `provider.options.byteplus` | 仅允许模型公布的透传选项；Seedance 2.5 允许 `watermark` 和 `output_format`。 |
| `user`、`session_id` | 可选追踪标签；不会转发给提供商。 |

如需首帧输入，请使用以下完整请求。省略 `aspect_ratio` 和 `size`，使输出跟随首帧比例：

```json
{
  "model": "seedance-2.5",
  "duration": 5,
  "resolution": "720p",
  "frame_images": [
    {
      "type": "image_url",
      "image_url": {"url": "https://your-host.example/first-frame.png"},
      "frame_type": "first_frame"
    }
  ]
}
```

如需约束尾帧，请增加第二项，使用 `frame_type: "last_frame"`。尾帧必须与首帧一起提供，不能单独使用。参考图像放入 `input_references`，每项使用 `type: "image_url"` 和 `image_url.url`，无需 `frame_type`。图像 URL 支持 HTTPS 或内联 `data:image/{png,jpeg,webp};base64,` 内容。整个创建请求上限为 **16 MiB**。

不支持回调（`callback_url`）、续接（`previous_job_id`）和参考音频/视频。未知字段可能被账户请求限制或视频验证器拒绝，请仅使用已记录的字段。目前没有公开的任务列表、取消、删除或 remix 端点。

## 创建、轮询并保存 Seedance 视频 {#create-poll-and-save-a-seedance-video}

以下 Python 示例仅使用标准库。运行前在环境中设置 `ANTIX_KEY`。如果设置 `ANTIX_BASE_URL`，其值应为不含 `/v1` 的网关根地址。

```python
import json
import os
from pathlib import Path
import shutil
import time
from urllib.parse import urljoin
from urllib.request import Request, urlopen

base = os.environ.get("ANTIX_BASE_URL", "https://antix.antigma.ai").rstrip("/")
headers = {"Authorization": f"Bearer {os.environ['ANTIX_KEY']}"}

def api(path, payload=None):
    request = Request(
        urljoin(base + "/", path),
        data=None if payload is None else json.dumps(payload).encode(),
        headers={**headers, "Content-Type": "application/json"},
    )
    with urlopen(request, timeout=120) as response:
        return json.load(response)

task = api("/v1/videos", {
    "model": "seedance-2.5",
    "prompt": "A red kite over a beach at sunset",
    "duration": 5,
    "resolution": "720p",
    "aspect_ratio": "16:9",
})
polling_url = task["polling_url"]
print("Save this task ID to resume polling:", task["id"], flush=True)
deadline = time.monotonic() + 75 * 60
delay = 15
while task["status"] in {"pending", "in_progress"}:
    if time.monotonic() >= deadline:
        raise SystemExit(f"Polling stopped; contact support with {task['id']}. Do not recreate the task.")
    time.sleep(delay)
    task = api(polling_url)
    delay = min(delay * 2, 120)
if task["status"] != "completed":
    raise SystemExit(f"Video ended: {task['status']}; {task.get('error', '')}")

request = Request(urljoin(base + "/", task["unsigned_urls"][0]), headers=headers)
partial = Path("kite.mp4.part")
with urlopen(request, timeout=120) as response, partial.open("wb") as output:
    shutil.copyfileobj(response, output)
partial.replace("kite.mp4")
print("Saved kite.mp4")
```

创建返回 **202**，包含 `id`、相对路径 `polling_url` 和 `status: "pending"`。轮询状态包括 `pending`、`in_progress`、`completed`、`failed`、`cancelled` 或 `expired`。`in_progress` 涵盖提供商排队、执行，以及等待人工审核的未决任务。提供商检查按计划进行，初始间隔 15 秒，逐步退避至两分钟；更频繁轮询可能返回相同状态。

创建成功后请保存任务 ID。后续轮询或下载遇到临时故障时，应重试同一任务的相应操作。客户端断开连接后，创建流程仍可能继续，而且 API 没有客户端幂等合同：盲目重试创建可能额外生成并收费。

完成结果包含相对路径 **`unsigned_urls`** 和 `expires_at`（Unix 秒）。尽管字段名为 unsigned，这些 URL 仍需要 Antix 凭据。提供商签名 `urls` 和 `last_frame_url` 不会对外提供。创建账户下的任意有效 Antix 密钥均可读取/下载任务；其他账户收到 404。

### 下载限制 {#download-limits}

- 使用 `GET /v1/videos/{id}/content?index=0` 下载；省略 `index` 时默认为 `0`。其他索引不可用。
- 在 `expires_at` 前保存文件，通常是提供商完成后 24 小时。Antix 从提供商流式传输，不保存视频文件。
- 每次 GET 获取完整文件。Range 请求被忽略，`Accept-Ranges` 为 `none`，不提供 `Content-Length`。如需播放和拖动进度，请保存本地副本。
- 每段视频最多下载 100 次。提供商未实际提供文件的尝试不消耗次数。
- HEAD 只检查 Antix 任务、过期时间和剩余次数，不获取文件或消耗下载次数。HEAD 成功不保证提供商仍能响应后续 GET。
- 单次下载最长十分钟。**每个网关实例**最多允许同一账户两路并发下载、总共 32 路；超限返回带 `Retry-After` 的 `429 video_download_busy`。

### 计费与任务限制 {#billing-and-task-limits}

创建时，Antix 按请求输出时长加一秒预留资金。首帧自适应比例按该分辨率下的最大画面预留。可用余额不足时返回 `402 ledger_budget_exceeded`。

通常按提供商报告的**视频 token**最终收费，并释放未使用的预留。成功结果未提供用量时，Antix 会收取全部预留金额。预留金额不是收费上限：报告用量超出预留时，仍按请求冻结的费率收取全部实际用量费用。输出估算公式为 `宽 × 高 × 24 fps × 秒数 / 1024`，向上取整。例如，720p、16:9、五秒输出估算为 108,000 token。模型发现的 `pricing_skus` 使用十进制字符串，单位为**美元/视频 token**，1080p 使用独立的 `video_tokens_1080p` 费率。请读取当前价格，不要硬编码。

计费完成后出现 `usage`。失败、取消和过期任务不收费。成功计费的任务会异步写入账户请求历史和密钥用量；视频 token 独立于文本输入/输出 token，同时会计入密钥的 token 配额。报告限制见[追踪导出与分析](/antix/concepts/analytics)。

默认每个账户最多有三个未完成任务。更多创建请求返回带 `Retry-After` 的 `429 video_task_limit`。提供商执行被配置为一小时后过期；未完成任务最迟在创建后 75 分钟不再占用准入名额。结果无法确认时，任务可能在该窗口后仍保持 `in_progress`，资金也会继续预留，等待人工审核。释放任务名额不代表释放余额预留。升级处理方式见[视频问题](/antix/concepts/troubleshooting#video-problems)。

| 状态 / 错误码 | 处理方式 |
| --- | --- |
| 400 `invalid_request`、`unsupported_parameter`、`upstream_rejected` | 修正请求；提供商拒绝不收费。 |
| 403 `model_not_allowed`、`provider_not_allowed`、`quota_exceeded` | 检查密钥访问策略或 token 配额。 |
| 404 `model_not_found`、`not_found` | 检查模型发现、任务 ID 和所属账户。 |
| 409 `video_not_ready` | 先继续轮询，完成后再下载。 |
| 410 `video_expired`、`video_download_limit`、`video_unavailable` | 该输出已无法下载。 |
| 429 `video_task_limit`、`video_download_busy`、`upstream_rate_limited` | 退避；有 `Retry-After` 时遵循其要求。 |
| 502/503 `upstream_unavailable`、`video_generation_unavailable` | 稍后重试读取/下载；不要自动重复创建。 |

## MiniMax H3：原生 BYOK API {#minimax-h3-native-byok-api}

MiniMax 使用原生 `content[]` 请求和响应，不使用 Seedance 结构。创建和轮询请求**均需**发送您的 MiniMax 密钥：

```bash
curl --fail-with-body https://antix.antigma.ai/v1/minimax/video_generation \
  -H "Authorization: Bearer $MINIMAX_API_KEY" \
  -H "X-Antix-Provider: minimax" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "MiniMax-H3",
    "content": [{"type": "text", "text": "A red kite over a beach"}],
    "duration": 5,
    "resolution": "768P",
    "ratio": "16:9"
  }'

# 将占位符替换为 MiniMax 响应中的数字任务 ID。
curl --fail-with-body https://antix.antigma.ai/v1/minimax/query/video_generation/TASK_ID \
  -H "Authorization: Bearer $MINIMAX_API_KEY" \
  -H "X-Antix-Provider: minimax"
```

| 模型 | 整数时长 | 分辨率 |
| --- | --- | --- |
| `MiniMax-H3` | 4–15 秒 | `768P`、`2K` |
| `MiniMax-H3-Max` | 5–15 秒 | `480P`、`768P` |

`content` 必须包含非空文本提示。纯文本请求必须使用明确的 `ratio`：`21:9`、`16:9`、`4:3`、`1:1`、`3:4` 或 `9:16`。H3 接受原生图像/视频/音频内容；H3 Max 不支持参考图像、参考视频或参考音频。创建请求体上限为 64 MiB。

使用数字任务 ID 轮询，直到 MiniMax 报告成功或失败；成功结果包含 `task.content.url`。提供商响应字段和状态保持原生格式。Antix `/content` 路由、Seedance 预留、过期时间和下载次数规则不适用于该 URL。

这些路由不支持 Antix 平台密钥，使用时返回 **501 `platform_key_not_supported`**；使用错误的 BYOK 提供商会返回 **403 `provider_mismatch`**。此处支持某个模型不代表它会出现在 `/v1/videos/models` 中，或可通过 Chat Completions 使用。

任务失败、用量缺失或下载问题请参阅[故障排查与支持](/antix/concepts/troubleshooting)。
