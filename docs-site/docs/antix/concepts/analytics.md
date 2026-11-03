---
title: "Trace Export & Analytics"
description: "Inspect account usage, understand cost sources, and export endpoint traces as JSON or NDJSON."
sidebar_position: 6
---

# Trace Export & Analytics

Use the [Antix Portal](https://portal.antigma.ai) to inspect account usage and drill into requests sent through an [Endpoint](/antix/concepts/endpoints). Account usage and endpoint traces have different scopes.

## Find the right view

| View | Use it for |
| --- | --- |
| **Overview / Usage & billing** | Account totals and model usage. |
| **Request history** | Individual account requests, token counts, and cost. |
| **API keys** | Usage attributed to a particular key. |
| **Endpoints → select endpoint → Overview** | The endpoint's request, text-token, and cost summary. |
| **Endpoints → select endpoint → Traces** | Filtered endpoint requests and the observation drawer. |
| **Endpoint Activity** | Session-oriented exploration where supported (experimental). |

For account history, filter by time, model, key, and authentication mode as available. Endpoint traces support time, model, key, authentication mode, and case-insensitive substring search across stored input and output bodies. Open a trace to inspect the permitted request and response fields, tools, token counts, latency, and time to first token (TTFT), where recorded. Missing latency or TTFT is not a measured zero.

Analytics can lag behind a completed request. A loading, warming, stale, or unavailable result is not proof of zero usage. Wait for the view to refresh, verify the time window and filters, and avoid repeatedly refreshing an unavailable view. For persistent gaps, see [Troubleshooting & Support](/antix/concepts/troubleshooting).

### Costs and units

Where the API exposes `cost_source`, interpret it alongside `cost_usd`:

| `cost_source` | Meaning |
| --- | --- |
| `billing_ledger` | An exact recorded Antix debit was found for the request. |
| `request_price` | Cost calculated using stored request-time pricing; not proof of the final debit. |
| `current_pricing_estimate` | An estimate using current model pricing. BYOK costs use this source. |
| `mixed` | An aggregate combines different cost sources. |

BYOK estimates are not an Antix charge or your provider's invoice. Current-price estimates may change when prices change; missing pricing can also make estimates incomplete. Do not use an analytics estimate as proof that your balance was debited.

**Video tokens are a separate unit.** Account stats expose `total_video_tokens`; history rows expose `video_tokens`. They are not added to text prompt or completion totals. Successful billed Seedance tasks appear as one request after asynchronous usage recording. Failed, cancelled, and expired tasks do not produce billed video usage rows.

Seedance usage currently has **no endpoint attribution**. Look for it in account history and key usage, not endpoint traces or their exports. Endpoint export token fields are text and cache counts, not video-token totals. See [Video Models & API](/antix/concepts/videos).

## Export from the portal

1. Open **Endpoints**, select an endpoint you can view, then open **Traces**.
2. Expand the filter controls and choose a bounded time range. Add model, key, auth-mode, or content filters as needed.
3. Use the download control to choose **JSON** or **NDJSON** and whether to include bodies.
4. Inspect the downloaded file's final manifest before treating it as complete.

Exports use the signed-in portal identity and the same endpoint visibility checks as trace viewing. An endpoint URL alone grants no access. The API is:

```text
GET /api/portal/endpoints/{endpoint_id}/traces/download
```

### Export parameters

| Parameter | Rules |
| --- | --- |
| `from`, `to` | Both required; RFC 3339 timestamps. `to` must be later than `from`; maximum window 90 days. |
| `format` | `ndjson` (default) or `json`. |
| `include_bodies` | Boolean; defaults to `false`. |
| `model` | Optional exact model ID. |
| `auth_mode` | Optional `virtual_key`, `oauth`, or `byok`. |
| `vk_id` | Optional key identifier from trace data; never the plaintext key. |
| `content_search` | Optional case-insensitive literal substring, at most 512 characters. |

Timestamp bounds are inclusive. If you split an export into adjacent windows, deduplicate overlapping rows by `trace_id`.

The result cap is **10,000 rows**. Narrow the window or filters after `400 cap_exceeded`; the response includes the limit and estimated row count. The current implementation also limits exports to five admitted download attempts per user per hour **per gateway instance**. `429 rate_limited` includes `retry_after_seconds`; wait rather than looping.

### Download through your portal session

For an API example, run this JavaScript in the browser console on your **signed-in portal origin**. Replace `YOUR_ENDPOINT_UUID` with the endpoint ID. It exports the last 24 hours without request/response bodies and checks the manifest before saving.

```javascript
const endpointId = 'YOUR_ENDPOINT_UUID';
const to = new Date();
const from = new Date(to.getTime() - 24 * 60 * 60 * 1000);
const query = new URLSearchParams({
  from: from.toISOString(),
  to: to.toISOString(),
  format: 'json',
  include_bodies: 'false',
});
const response = await fetch(
  `/api/portal/endpoints/${encodeURIComponent(endpointId)}/traces/download?${query}`,
  { credentials: 'same-origin' },
);
if (!response.ok) throw new Error(`Export failed (${response.status}): ${await response.text()}`);
const data = await response.json();
if (!data.manifest || data.manifest.error_code !== null) {
  throw new Error(`Export incomplete: ${data.manifest?.error_code ?? 'missing manifest'}`);
}
const url = URL.createObjectURL(new Blob([JSON.stringify(data)], { type: 'application/json' }));
const link = document.createElement('a');
link.href = url;
link.download = 'antix-traces.json';
link.click();
setTimeout(() => URL.revokeObjectURL(url), 1000);
```

This uses your existing portal session; do not paste an API key or session cookie into the example or an export file.

## Read the file and verify completeness

**JSON** returns an object with a `traces` array and a `manifest` object. **NDJSON** returns one trace per line followed by a final line with `_manifest: true`; that last line is metadata, not another trace.

Trace rows contain `trace_id`, timestamp, endpoint and model, auth mode, `is_byok`, token counts, `cost_usd`, and `cost_source`. Latency fields can be `null`; session and agent signatures are present only when available. The exported `status` is currently `ok` for these inference rows — this is not a comprehensive log of rejected or failed HTTP requests.

The manifest includes `schema_version`, filters, `row_count`, `include_bodies`, `format`, `generated_at`, redaction metadata, and `error_code`. Check that:

- The file parses and has a final manifest.
- The number of trace records matches `row_count`.
- `error_code` is `null` before treating the result as complete.

A stream that already sent HTTP `200` cannot replace it with an error status. `ch_timeout` or `ch_error` in the manifest means the export is partial; retry later or with a smaller window. `cap_truncated` also requires a narrower query. A disconnected or truncated download may have no final manifest at all.

### Bodies and visibility

Bodies are omitted by default. With `include_bodies=true`, permitted `input` and `output` fields go through the portal's shared body projection/redaction rules. Raw provider request/response fields and authentication material remain excluded by policy; an export is not a byte-for-byte replay of the upstream exchange.

:::warning Sensitive data in trace bodies
Redaction does not promise removal of every secret or personal detail in a user's prompt or a model's answer. Treat body-bearing files as sensitive. Export access and the bodies option are recorded in read-audit metadata. A 90-day query window is an export limit, not a data-retention guarantee — see [Privacy, Security & Data Retention](/antix/concepts/security).
:::

The export's `trace_id`, history's request identifier, and the response's `X-Antix-Request-Id` header are different diagnostic fields — do not assume they are interchangeable. Include whichever you have when [contacting support](/antix/concepts/troubleshooting#contact-support).
