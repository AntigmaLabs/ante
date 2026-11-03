---
title: "Video Models & API"
description: "Generate videos with Seedance through Antix, or use MiniMax H3 with your own provider key."
sidebar_position: 5
---

# Video Models & API

Video generation is asynchronous: submit a task, poll for its result, then save the video. Antix exposes two separate APIs with different credentials and response formats.

| | Seedance on BytePlus ModelArk | MiniMax H3 / H3 Max |
| --- | --- | --- |
| Create | `POST /v1/videos` | `POST /v1/minimax/video_generation` |
| Poll | `GET /v1/videos/{id}` | `GET /v1/minimax/query/video_generation/{task_id}` |
| Credential | Antix virtual key or Ante CLI OAuth access token | MiniMax provider key and `X-Antix-Provider: minimax` |
| Billing | Antix account balance | MiniMax bills your provider account; Antix does not re-bill |
| Download | Authenticated Antix `/content` URL | Native MiniMax response at `task.content.url` |

These are not Chat Completions endpoints. The Seedance route accepts JSON in an OpenRouter-shaped subset; do not assume OpenAI's video SDK or multipart request format works unchanged.

## Seedance availability and discovery

Use your Antix credential to discover the video models available to it:

```bash
curl --fail-with-body https://antix.antigma.ai/v1/videos/models \
  -H "Authorization: Bearer $ANTIX_KEY"
```

The response is a `data` array. Each model includes `supported_resolutions`, `supported_aspect_ratios`, `supported_sizes`, `supported_durations`, `supported_frame_images`, `allowed_passthrough_parameters`, and `pricing_skus`.

The current enabled model is **`seedance-2.5`**. `seedance-2.0-fast` and `seedance-2.0-mini` have request specifications but remain disabled. The live list is authoritative: a model also needs a configured provider credential, pricing, and permission under your key's access policy. Video discovery is separate from the text [Models API](/antix/concepts/models).

Provider BYOK keys are refused on `/v1/videos` with `400 video_requires_antix_key`.

## Seedance request

```json
{
  "model": "seedance-2.5",
  "prompt": "A red kite over a beach at sunset",
  "duration": 5,
  "resolution": "720p",
  "aspect_ratio": "16:9"
}
```

| Field | Rules |
| --- | --- |
| `model` | Required; use an ID from `/v1/videos/models`. |
| `prompt` | Required unless a first-frame image is supplied; at most 4,000 characters. |
| `duration` | Required integer seconds. Seedance 2.5 supports 4–30 seconds. |
| `resolution` | Defaults to `720p`; Seedance 2.5 supports `480p`, `720p`, and `1080p`. |
| `aspect_ratio` | Defaults to `16:9` for text-to-video. Supported ratios are `16:9`, `4:3`, `1:1`, `3:4`, `9:16`, and `21:9`. |
| `size` | A `WIDTHxHEIGHT` value from `supported_sizes`, as an alternative to resolution and aspect ratio. |
| `generate_audio` | Boolean; defaults to `true`. |
| `seed` | Optional integer from -1 to 2147483647. |
| `frame_images` | First/last-frame image inputs. With Seedance 2.5, omit `aspect_ratio` and `size`: the output follows the first frame. |
| `input_references` | Up to nine reference images; cannot be combined with `frame_images`. |
| `provider.options.byteplus` | Only the model's advertised passthrough options; Seedance 2.5 allows `watermark` and `output_format`. |
| `user`, `session_id` | Optional tracing labels; not forwarded to the provider. |

For first-frame input, use this complete request. Omit `aspect_ratio` and `size` so the output follows the first frame:

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

To constrain the final frame, add a second entry with `frame_type: "last_frame"`. A last frame requires a first frame; it cannot be supplied alone. Reference images use `input_references` entries with `type: "image_url"` and `image_url.url`, without `frame_type`. Image URLs may use HTTPS or inline `data:image/{png,jpeg,webp};base64,` content. The whole create request is limited to **16 MiB**.

Callbacks (`callback_url`), continuation (`previous_job_id`), and reference audio/video are unsupported. Unknown fields can be rejected by the account's request gate or the video validator; use only the documented fields. There are no public task-list, cancel, delete, or remix endpoints.

## Create, poll, and save a Seedance video

This Python example uses only the standard library. Set `ANTIX_KEY` in your environment before running it. `ANTIX_BASE_URL`, if set, is the gateway root without `/v1`.

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

Create returns **202** with `id`, a relative `polling_url`, and `status: "pending"`. Polling returns `pending`, `in_progress`, `completed`, `failed`, `cancelled`, or `expired`. `in_progress` includes provider queuing, execution, and unresolved tasks awaiting operator review. The provider is checked on a schedule, initially every 15 seconds and backing off to two minutes; faster polling may return the same state.

Save the task ID when create succeeds. If a later poll or download fails transiently, retry that operation for the same task. A create request can continue after a client disconnect, and there is no client idempotency contract: blindly retrying create may generate and charge for another video.

Completed results contain relative **`unsigned_urls`** and `expires_at` (Unix seconds). Despite the field name, these URLs require an Antix credential. The provider's signed `urls` and `last_frame_url` are not exposed. Any valid Antix key belonging to the account that created the task can read/download it; a different account receives 404.

### Download limits

- Download with `GET /v1/videos/{id}/content?index=0`; omitting `index` selects `0`. Other indexes are not available.
- Save the output before `expires_at`, normally 24 hours after provider completion. Antix streams from the provider and does not store the video file.
- Every GET fetches the whole file. Range requests are ignored, `Accept-Ranges` is `none`, and `Content-Length` is omitted. Save a local copy for playback and seeking.
- Each video has a 100-download budget. An attempt that the provider never served does not consume it.
- HEAD checks the Antix task, expiry and remaining download budget without fetching or consuming a download. A successful HEAD does not guarantee the provider can still serve a later GET.
- Downloads have a ten-minute deadline. Concurrent downloads are limited to two per account and 32 total **per gateway instance**; excess requests receive `429 video_download_busy` with `Retry-After`.

### Billing and task limits

At creation, Antix holds funds for the requested output plus one second. For an adaptive first-frame ratio, the hold uses the largest frame at the selected resolution. Insufficient available balance returns `402 ledger_budget_exceeded`.

Final charging normally uses provider-reported **video tokens**, with the unused hold released. If a successful result omits usage, Antix charges the full reserved amount instead. The reservation is not a charge ceiling: reported usage above the hold is charged in full at the request's frozen rate. The output estimate is `width × height × 24 fps × seconds / 1024`, rounded up. A 720p 16:9 five-second output estimates 108,000 tokens. Discovery's `pricing_skus` values are decimal strings in **USD per video token**, with a separate `video_tokens_1080p` rate. Read current prices rather than hardcoding them.

`usage` appears once billed. Failed, cancelled and expired tasks are not charged. Successful billed tasks enter account request history and key usage asynchronously; video tokens are separate from text prompt/completion tokens. Video tokens count toward a key's token quota. See [Trace Export & Analytics](/antix/concepts/analytics) for reporting limits.

By default, an account can have three unfinished tasks. Further creates return `429 video_task_limit` with `Retry-After`. Provider execution is configured to expire after an hour; an unfinished task stops consuming an admission slot no later than 75 minutes after creation. An unresolved outcome can remain `in_progress` with funds held beyond that window until operator review. Releasing the task slot does not release the balance hold. See [Video problems](/antix/concepts/troubleshooting#video-problems) for escalation.

| Status / code | Action |
| --- | --- |
| 400 `invalid_request`, `unsupported_parameter`, `upstream_rejected` | Correct the request; a provider refusal is not charged. |
| 403 `model_not_allowed`, `provider_not_allowed`, `quota_exceeded` | Check the key's access policy or token quota. |
| 404 `model_not_found`, `not_found` | Check discovery, the task ID and owning account. |
| 409 `video_not_ready` | Continue polling before downloading. |
| 410 `video_expired`, `video_download_limit`, `video_unavailable` | This output is no longer downloadable. |
| 429 `video_task_limit`, `video_download_busy`, `upstream_rate_limited` | Back off; respect `Retry-After` when present. |
| 502/503 `upstream_unavailable`, `video_generation_unavailable` | Retry reads/downloads later; do not automatically repeat create. |

## MiniMax H3: native BYOK API

MiniMax uses its native `content[]` request and response, not the Seedance schema. Send your MiniMax key on **both** create and poll requests:

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

# Replace the placeholder with the numeric task ID from MiniMax's response.
curl --fail-with-body https://antix.antigma.ai/v1/minimax/query/video_generation/TASK_ID \
  -H "Authorization: Bearer $MINIMAX_API_KEY" \
  -H "X-Antix-Provider: minimax"
```

| Model | Integer duration | Resolution |
| --- | --- | --- |
| `MiniMax-H3` | 4–15 seconds | `768P`, `2K` |
| `MiniMax-H3-Max` | 5–15 seconds | `480P`, `768P` |

`content` must contain a non-empty text prompt. Text-only requests require a concrete `ratio`: `21:9`, `16:9`, `4:3`, `1:1`, `3:4`, or `9:16`. H3 accepts native image/video/audio content; H3 Max does not support reference images, reference video, or reference audio. The create body limit is 64 MiB.

Poll the numeric task ID until MiniMax reports success or failure; a successful result contains `task.content.url`. Provider response fields and statuses remain native. The Antix `/content` route, Seedance holds, expiry and download-budget rules do not apply to this URL.

Antix platform keys are unsupported on these routes and return **501 `platform_key_not_supported`**; the wrong BYOK provider returns **403 `provider_mismatch`**. A model appearing here does not imply it is listed in `/v1/videos/models` or usable through Chat Completions.

For failed tasks, missing usage or download problems, see [Troubleshooting & Support](/antix/concepts/troubleshooting).
