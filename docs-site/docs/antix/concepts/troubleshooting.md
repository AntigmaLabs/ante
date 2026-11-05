---
title: "Troubleshooting & Support"
description: "Diagnose authentication, model access, video, analytics, and payment problems, and collect useful support details."
sidebar_position: 7
---

# Troubleshooting & Support

Start with the HTTP status, the response's machine-readable error code or type, and its message. Authentication and inference routes can use different error envelopes, so save the body as well as the status. See [Error Handling](/antix/concepts/error-handling) for the general reference.

## Collect a useful failure report

Capture response headers separately from the body. This read-only example checks model discovery with an Antix key:

```bash
curl --silent --show-error --fail-with-body \
  --dump-header antix-response-headers.txt \
  --output antix-response.json \
  https://antix.antigma.ai/v1/models \
  -H "Authorization: Bearer $ANTIX_KEY"
```

Keep the returned **`X-Antix-Request-Id`**, timestamp with timezone, route, model ID, status, and error body. Antix generates this response identifier; sending your own header with that name does not set it. If available, also record the history ID, trace ID, or video task ID, labeled separately.

## Authentication and model access

| Symptom | Check / next step |
| --- | --- |
| `401`, missing or invalid token | Supply a valid Antix key, or sign in again for an expired OAuth or session credential. Provider BYOK keys belong to their provider lane. |
| `/v1/models` or `/v1/videos/models` returns `401` | Authenticated discovery routes require a valid credential. Send the credential you will use for inference. |
| A model is absent, or `model_not_found` | Check the authenticated model list (or `/v1/videos/models` for Seedance). The model must be enabled, credentialed, priced, and permitted for that credential. |
| `403` `model_not_allowed` / `provider_not_allowed` | The key or account policy does not grant that model or provider. A positive balance does not itself grant every model. Use an allowed model or contact the deployment administrator. |
| `403` `quota_exceeded` | A key's token quota is exhausted. This is separate from money balance; repeated requests do not reset it. |
| `403` `ledger_request_not_supported` | The route, request option, or credential is outside the account's supported scope. Follow the error message and remove the unsupported option; do not retry the unchanged request in a loop. |
| Provider or path mismatch | Match the typed endpoint provider, `X-Antix-Provider`, and BYOK credential. Use the exact SDK base path; do not drop the native API version segment. |
| `410` on an endpoint URL | The endpoint was archived. Configure an active endpoint; retrying the archived URL will not restore it. |

## Balance, throttling, and temporary failures

- **`402` `ledger_budget_exceeded`** — available balance cannot cover the request's reservation. Pending requests and video tasks can hold funds before their final charge. Check the portal balance and existing tasks before starting more work.
- **`429`** — inspect the error code. `ledger_admission_busy`, provider throttling, `video_task_limit`, and `video_download_busy` have different causes. Honor `Retry-After` when present, reduce concurrency, and back off. Trace-export throttling instead reports `retry_after_seconds` in its response body.
- **`502` / `503`** — preserve the request ID and error body. Retry read-only operations with backoff. For generation requests, first consider whether the original request could already have reached the provider; a connection failure is not a general guarantee that generation was cancelled or uncharged.

Do not assume an `Idempotency-Key` header makes every inference or video create request safe to repeat. The API must explicitly document that operation's idempotency guarantee.

## Video problems

| Symptom | Check / next step |
| --- | --- |
| `video_requires_antix_key` | Seedance `/v1/videos` requires an Antix credential, not a BytePlus or provider BYOK key. |
| MiniMax `platform_key_not_supported` | H3 routes require a MiniMax BYOK key and `X-Antix-Provider: minimax`. |
| Task remains `pending` / `in_progress` | Poll the same saved task ID during the normal execution window. If still unresolved after roughly 75 minutes, contact support with that ID — the task may require operator review. Its balance hold can remain after the concurrency slot is released. Do not recreate it to recover the hold. |
| `video_not_ready` | Wait for `completed` before downloading. |
| `video_expired`, `video_download_limit`, `video_unavailable` | The output is no longer downloadable. Preserve any local copy; the old URL cannot be restored by polling. |
| Task returns `404` | Check the exact `vid_…` identifier and the credential's owning account. |
| A completed video is missing from endpoint traces | Seedance currently has no endpoint attribution. Check account request history and key usage, and allow time for asynchronous usage recording. |

The [Video Models & API](/antix/concepts/videos) guide covers request fields, task limits, download behavior, and billing. Do not recreate a task merely because polling or downloading failed.

## Missing analytics or an incomplete export

1. Verify the account, endpoint, time window and timezone, model, key, and auth-mode filters. Account usage and endpoint traces have different scopes.
2. Let warming or stale views refresh. An unavailable view is not a zero-usage result.
3. Distinguish text and video token counts, and inspect `cost_source` before comparing an estimate with the account debit.
4. For `cap_exceeded`, narrow the export window or filters. For throttling, wait for the reported delay.
5. Check the final export manifest. HTTP `200` with `ch_timeout`, `ch_error`, or `cap_truncated`, or a missing manifest, is not a complete export.

See [Trace Export & Analytics](/antix/concepts/analytics) for formats, limits, and completeness checks. If records remain missing, report a specific example rather than generating more requests to test the counters.

## Payment or credit still pending

Use **Usage & billing** to review the existing top-up and its status. Returning from checkout, or closing its browser tab, does not establish payment or cancellation. Do not open repeated checkouts to resolve a payment that may already have succeeded.

If a payment stays pending or under review, or the expected credit is missing, use its billing-support action when available. Include the top-up intent ID or reference, approximate time and amount, plus the displayed status. Never include card details. A billing case records a request for help — it does not approve a refund or instantly change the balance.

## Contact support

Open **Support** in the [Antix Portal](https://portal.antigma.ai) when enabled:

- **Report an issue** — for API or product failures.
- **Feature request** — for a capability you need.
- **Billing** — for missing credits, charges, or refunds. Use the affected top-up's support action if available.

Availability depends on the deployment. If a form is not offered, use the support link shown in the portal or contact your deployment administrator. Keep the returned case reference. General issue and feature intake is submit-only; do not assume a case conversation or status page exists. Billing intake supports case status and detail where enabled.

Include the symptom, expected behavior, reproduction steps, timestamp with timezone, route and model, status and error code, response request ID, and relevant task, trace, or top-up identifiers. Share a minimal sanitized request only if needed. Remove API keys, bearer tokens, cookies, signed storage URLs, and private prompt content — support forms can reject sensitive material. For an export problem, its manifest and filters are usually more useful than a file containing customer prompts.
