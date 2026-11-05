---
title: "视频模型与 API"
description: "通过 Antix 使用 Seedance 生成视频，或使用自己的提供商密钥调用 MiniMax H3。"
sidebar_position: 5
---

# 视频模型与 API {#video-models--api}

视频生成采用异步流程：提交任务、轮询结果，然后保存视频。Antix 提供两套独立的 API，其凭据与响应格式不同。

| | BytePlus ModelArk 上的 Seedance | MiniMax H3 / H3 Max |
| --- | --- | --- |
| **创建** | `POST /v1/videos` | `POST /v1/minimax/video_generation` |
| **轮询** | `GET /v1/videos/{id}` | `GET /v1/minimax/query/video_generation/{task_id}` |
| **凭据** | Antix 虚拟密钥或 Ante CLI OAuth 访问令牌 | MiniMax 提供商密钥，以及 `X-Antix-Provider: minimax` |
| **计费** | Antix 账户余额 | MiniMax 向您的提供商账户计费；Antix 不重新计费 |
| **下载** | 需身份验证的 Antix `/content` URL | MiniMax 原生响应中的 `task.content.url` |

这些不是 Chat Completions 端点。Seedance 路由接受 OpenRouter 数据结构的 JSON 子集——请勿假设 OpenAI 的视频 SDK 或 multipart 请求格式可以直接原样使用。

## Seedance 可用性与模型发现 {#seedance-availability-and-discovery}

使用您的 Antix 凭据查询当前凭据可用的视频模型：

```bash
curl --fail-with-body https://antix.antigma.ai/v1/videos/models \
  -H "Authorization: Bearer $ANTIX_KEY"
```

响应是一个 `data` 数组。每个模型包含 `supported_resolutions`、`supported_aspect_ratios`、`supported_sizes`、`supported_durations`、`supported_frame_images`、`allowed_passthrough_parameters` 和 `pricing_skus`。

当前已启用的模型为 **`seedance-2.5`**。`seedance-2.0-fast` 和 `seedance-2.0-mini` 已有请求规范定义，但目前尚未启用。请以实时返回的列表为准：模型还必须具备已配置的提供商凭据、有效定价，且在您的密钥访问策略允许范围内。视频模型发现独立于文本[模型 API](/antix/concepts/models)。

在 `/v1/videos` 上使用提供商 BYOK 密钥会被拒绝，并返回 `400 video_requires_antix_key`。

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
| `prompt` | 除非提供了首帧图像，否则必填；最多 4,000 个字符。 |
| `duration` | 必填，整数秒。Seedance 2.5 支持 4–30 秒。 |
| `resolution` | 默认为 `720p`；Seedance 2.5 支持 `480p`、`720p` 和 `1080p`。 |
| `aspect_ratio` | 文生视频默认为 `16:9`。支持的宽高比包括 `16:9`、`4:3`、`1:1`、`3:4`、`9:16` 和 `21:9`。 |
| `size` | 取自 `supported_sizes` 的 `WIDTHxHEIGHT` 值，可替代分辨率和宽高比。 |
| `generate_audio` | 布尔值；默认为 `true`。 |
| `seed` | 可选，-1 至 2147483647 之间的整数。 |
| `frame_images` | 首帧/尾帧图像输入。在 Seedance 2.5 中使用此项时，请省略 `aspect_ratio` 和 `size`：输出将自动跟随首帧比例。 |
| `input_references` | 最多 9 张参考图像；不能与 `frame_images` 同时使用。 |
| `provider.options.byteplus` | 仅限模型声明支持的透传选项；Seedance 2.5 支持 `watermark` 和 `output_format`。 |
| `user`、`session_id` | 可选的追踪标签；不会转发给上游提供商。 |

如需传入首帧图像，请使用以下完整请求，并省略 `aspect_ratio` 和 `size` 以使输出跟随首帧比例：

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

如需约束尾帧，请添加第二个条目并设置 `frame_type: "last_frame"`。尾帧必须与首帧同时提供，不能单独传入。参考图像则放入 `input_references` 中，每个条目包含 `type: "image_url"` 和 `image_url.url`，且不带 `frame_type`。图像 URL 支持 HTTPS 链接或内联 `data:image/{png,jpeg,webp};base64,` 内容。整个创建请求的正文大小上限为 **16 MiB**。

不支持回调（`callback_url`）、续接（`previous_job_id`）以及参考音频/视频。未知字段可能会被账户策略或视频验证器拒绝，因此请仅使用文档中列出的字段。目前不提供公开的任务列表、取消、删除或 remix 端点。

## 创建、轮询并保存 Seedance 视频 {#create-poll-and-save-a-seedance-video}

以下 Python 示例仅使用标准库。运行前请在环境变量中设置 `ANTIX_KEY`。如果设置了 `ANTIX_BASE_URL`，其值应为不含 `/v1` 的网关根地址。

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

创建请求会返回 `202 Accepted`，其中包含 `id`、相对路径 `polling_url` 和 `status: "pending"`。轮询可能返回 `pending`、`in_progress`、`completed`、`failed`、`cancelled` 或 `expired`。`in_progress` 涵盖提供商排队、执行中，以及等待人工审核的未决任务。网关会按计划向提供商查询状态——初始间隔为 15 秒，并逐步退避至 2 分钟——因此更频繁地轮询可能会返回相同的状态。

:::warning 轮询前请先保存任务 ID
创建成功后请立即保存任务 ID。如果后续轮询或下载遇到临时故障，请针对同一任务重试相应操作。即使客户端断开连接，创建请求仍可能继续执行，且 API 不提供客户端幂等性保证——盲目重试创建可能会额外生成一段视频并产生费用。
:::

完成后的结果包含相对路径 **`unsigned_urls`** 和 `expires_at`（Unix 秒）。尽管字段名为 `unsigned_urls`，访问这些 URL 仍须在 `Authorization` 标头中携带 Antix 凭据。提供商原始的签名 `urls` 和 `last_frame_url` 不会对外暴露。创建该任务的账户下的任意有效 Antix 密钥均可读取或下载该任务；其他账户访问时会收到 `404 Not Found`。

### 下载限制 {#download-limits}

- 使用 `GET /v1/videos/{id}/content?index=0` 下载；省略 `index` 时默认选择 `0`，不支持其他索引值。
- 请在 `expires_at`（通常为提供商生成完成后的 24 小时）之前保存输出文件。Antix 直接从提供商流式传输数据，不会存储视频文件。
- 每次 `GET` 请求都会拉取完整文件。`Range` 请求会被忽略，`Accept-Ranges` 为 `none`，且响应不包含 `Content-Length`。如需播放和拖动进度条，请先将视频保存到本地。
- 每段视频有 100 次下载配额。如果提供商未能实际返回文件，该次尝试不会消耗配额。
- `HEAD` 请求仅检查 Antix 任务状态、过期时间和剩余下载配额，不会拉取文件或消耗下载配额。`HEAD` 请求成功并不保证提供商一定能正常响应后续的 `GET` 请求。
- 单次下载的超时上限为 10 分钟。**每个网关实例**最多允许同一账户同时进行 2 路下载、所有账户共 32 路并发下载；超出限制的请求会收到带有 `Retry-After` 标头的 `429 video_download_busy`。

### 计费与任务限制 {#billing-and-task-limits}

创建任务时，Antix 会按请求的输出时长加 1 秒预留资金。若采用自适应首帧宽高比，则会按所选分辨率下的最大画幅预留资金。当可用余额不足时，请求会返回 `402 ledger_budget_exceeded`。

最终扣费通常依据提供商报告的**视频 token** 计算，并释放未使用的预留金额。如果成功结果未返回用量，Antix 将改为收取全部预留金额。预留金额并非收费上限：当报告的实际用量超出预留金额时，系统仍会按请求时冻结的费率全额收取实际用量费用。输出 token 的估算公式为 `宽 × 高 × 24 fps × 秒数 / 1024`（向上取整）。例如，720p、16:9 的 5 秒视频估算为 108,000 个 token。模型发现接口中的 `pricing_skus` 值为十进制字符串，单位为**美元 / 视频 token**，其中 1080p 使用独立的 `video_tokens_1080p` 费率。请动态读取当前价格，切勿在代码中硬编码。

计费完成后，响应中会出现 `usage` 字段。失败、已取消和已过期的任务不会产生费用。成功计费的任务会异步写入账户请求历史和密钥用量；视频 token 独立于文本提示/完成 token 统计，但同样会计入密钥的 token 配额。有关统计与导出的限制，请参阅[追踪导出与分析](/antix/concepts/analytics)。

默认情况下，每个账户最多可同时存在 3 个未完成的任务。继续创建会返回带有 `Retry-After` 的 `429 video_task_limit`。提供商端的执行超时配置为 1 小时；未完成的任务最迟在创建 75 分钟后释放并发名额。若任务结果未能确认，在该时间窗口之后仍可能保持 `in_progress` 状态并继续占用预留资金，直至人工审核完成。释放并发任务名额并不等同于释放余额预留。升级处理方式请参阅[视频问题](/antix/concepts/troubleshooting#video-problems)。

| 状态 / 错误码 | 处理方式 |
| --- | --- |
| `400` `invalid_request`、`unsupported_parameter`、`upstream_rejected` | 修正请求参数；被提供商拒绝的请求不会收费。 |
| `403` `model_not_allowed`、`provider_not_allowed`、`quota_exceeded` | 检查密钥的访问策略或 token 配额。 |
| `404` `model_not_found`、`not_found` | 检查模型发现列表、任务 ID 以及所属账户。 |
| `409` `video_not_ready` | 继续轮询，等待任务完成后再下载。 |
| `410` `video_expired`、`video_download_limit`、`video_unavailable` | 该视频输出已无法再下载。 |
| `429` `video_task_limit`、`video_download_busy`、`upstream_rate_limited` | 进行退避等待；若响应包含 `Retry-After` 标头，请遵循其等待时间。 |
| `502` / `503` `upstream_unavailable`、`video_generation_unavailable` | 稍后重试读取或下载请求；切勿自动重复发起创建请求。 |

## MiniMax H3：原生 BYOK API {#minimax-h3-native-byok-api}

MiniMax 使用其原生的 `content[]` 请求与响应数据结构，而非 Seedance 结构。在创建和轮询请求中**均须**发送您的 MiniMax 密钥：

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

`content` 必须包含非空的文本提示。纯文本请求必须指定具体的 `ratio`：`21:9`、`16:9`、`4:3`、`1:1`、`3:4` 或 `9:16`。`MiniMax-H3` 接受原生图像、视频和音频内容；`MiniMax-H3-Max` 不支持参考图像、参考视频或参考音频。创建请求正文的大小上限为 64 MiB。

请使用数字任务 ID 进行轮询，直到 MiniMax 报告成功或失败；成功结果包含 `task.content.url`。提供商的响应字段与状态均保持原生格式。Antix 的 `/content` 路由、Seedance 资金预留、过期时间及下载配额规则均不适用于该 URL。

这些路由不支持 Antix 平台密钥，若使用将返回 `501`（`platform_key_not_supported`）；若指定的 BYOK 提供商不匹配，则返回 `403`（`provider_mismatch`）。某模型支持此路由并不代表它会出现在 `/v1/videos/models` 中，也不代表可通过 Chat Completions 调用。

有关任务失败、用量缺失或下载问题的排查方法，请参阅[故障排查与支持](/antix/concepts/troubleshooting)。
