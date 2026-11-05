---
title: "故障排查与支持"
description: "诊断认证、模型访问、视频、分析与付款问题，并收集有效的支持信息。"
sidebar_position: 7
---

# 故障排查与支持 {#troubleshooting--support}

排查问题时，请首先查看 HTTP 状态码、响应中机器可读的错误代码（或错误类型）以及错误消息。身份验证与推理路由可能使用不同的错误包络结构，因此请同时保存响应正文和状态码。通用错误代码列表请参阅[错误处理](/antix/concepts/error-handling)。

## 收集有效的故障报告 {#collect-a-useful-failure-report}

建议将响应标头与响应正文分开保存。以下只读示例使用 Antix 密钥检查模型发现接口：

```bash
curl --silent --show-error --fail-with-body \
  --dump-header antix-response-headers.txt \
  --output antix-response.json \
  https://antix.antigma.ai/v1/models \
  -H "Authorization: Bearer $ANTIX_KEY"
```

请保留响应返回的 **`X-Antix-Request-Id`**、带时区的时间戳、请求路由、模型 ID、HTTP 状态码以及错误响应正文。该请求标识符由 Antix 自动生成；自行在请求中发送同名标头无法指定该值。如果有历史记录 ID、追踪 ID 或视频任务 ID，也请一并记录并分别注明类型。

## 认证与模型访问 {#authentication-and-model-access}

| 现象 | 检查 / 下一步 |
| --- | --- |
| `401`，缺少或无效令牌 | 提供有效的 Antix 密钥；若 OAuth 或会话凭据已过期，请重新登录。提供商的 BYOK 密钥仅适用于对应的提供商路由。 |
| `/v1/models` 或 `/v1/videos/models` 返回 `401` | 需身份验证的模型发现路由必须携带有效凭据。请发送您将用于推理请求的凭据。 |
| 模型缺失或返回 `model_not_found` | 检查已认证的模型列表（Seedance 视频模型请检查 `/v1/videos/models`）。目标模型必须已启用、已配置提供商凭据与定价，且在当前凭据的访问策略允许范围内。 |
| `403` `model_not_allowed` / `provider_not_allowed` | 当前密钥或账户策略未授权访问该模型或提供商。账户余额充足并不代表自动拥有所有模型的访问权限。请改用受允许的模型，或联系部署管理员。 |
| `403` `quota_exceeded` | 密钥的 token 配额已耗尽。该配额独立于资金余额，重复发起请求不会重置配额。 |
| `403` `ledger_request_not_supported` | 请求路由、参数选项或凭据超出了当前账户支持的作用域。请根据错误消息移除不支持的选项，切勿循环重试未修改的原请求。 |
| 提供商或路径不匹配 | 确保端点 URL 中的提供商标识、`X-Antix-Provider` 标头与 BYOK 凭据彼此匹配。请使用准确的 SDK 基地址，不要省略原生 API 的版本路径段。 |
| 端点 URL 返回 `410` | 该端点已被归档。请改用处于活动状态的端点；继续重试已归档的 URL 不会使其恢复。 |

## 余额、限流与临时故障 {#balance-throttling-and-temporary-failures}

- **`402` `ledger_budget_exceeded`** — 可用余额不足以覆盖本次请求的预留金额。处理中的推理请求和视频任务在最终结算前会占用预留资金。请在发起新任务前检查门户中的账户余额与进行中的任务。
- **`429`** — 请查看具体的错误代码。`ledger_admission_busy`、上游提供商限流、`video_task_limit` 和 `video_download_busy` 的触发原因各不相同。若响应包含 `Retry-After` 标头，请遵循其等待时间，降低并发并执行退避。追踪导出接口触发限流时，则会在响应正文中返回 `retry_after_seconds`。
- **`502` / `503`** — 请保留请求 ID 和错误响应正文。对于只读操作，可采用退避策略重试；对于生成类请求，请先确认原请求是否可能已到达上游提供商——连接中断并不保证生成任务已被取消或未产生费用。

请勿假设携带 `Idempotency-Key` 标头即可安全地重复发起任意推理或视频创建请求。只有当 API 文档明确声明该操作具备幂等性保证时才可依赖。

## 视频问题 {#video-problems}

| 现象 | 检查 / 下一步 |
| --- | --- |
| `video_requires_antix_key` | Seedance `/v1/videos` 路由需要使用 Antix 凭据，不支持 BytePlus 或其他提供商的 BYOK 密钥。 |
| MiniMax `platform_key_not_supported` | MiniMax H3 路由需要提供 MiniMax BYOK 密钥并设置 `X-Antix-Provider: minimax`。 |
| 任务持续处于 `pending` / `in_progress` | 在正常执行时间窗口内，使用保存的同一任务 ID 继续轮询。若超过约 75 分钟仍未得出结果，请附上该任务 ID 联系支持团队——该任务可能需要人工审核。并发任务名额释放后，对应的资金预留仍可能保留；切勿为了退回预留资金而重新创建任务。 |
| `video_not_ready` | 请等待任务状态变为 `completed` 后再执行下载。 |
| `video_expired`、`video_download_limit`、`video_unavailable` | 该视频输出已无法再下载。请妥善保存已下载的本地副本；继续轮询无法恢复已失效的下载链接。 |
| 查询任务返回 `404` | 请核对完整的 `vid_…` 任务标识符，并确认当前凭据属于创建该任务的账户。 |
| 已完成的视频未出现在端点追踪中 | Seedance 目前不支持按端点归属。请前往账户请求历史或密钥用量页面查看，并留出异步用量入账的时间。 |

有关请求字段、任务并发限制、下载行为与计费规则的详细说明，请参阅[视频模型与 API](/antix/concepts/videos)。切勿仅因轮询或下载失败而重新创建视频任务。

## 分析缺失或导出不完整 {#missing-analytics-or-an-incomplete-export}

1. 核对所选账户、端点、时间窗口与时区、模型、密钥以及认证模式筛选条件。账户用量与端点追踪具有不同的统计范围。
2. 等待处于预热中或未及时刷新的视图完成加载。视图暂不可用并不代表实际用量为零。
3. 区分文本 token 与视频 token 计数；在将估算金额与账户扣款对比之前，请先查看 `cost_source`。
4. 若收到 `cap_exceeded`，请缩小导出时间窗口或收紧筛选条件；若触发限流，请按返回的延迟时间等待。
5. 检查导出文件末尾的 `manifest`。即使 HTTP 状态码为 `200`，若 `manifest` 中包含 `ch_timeout`、`ch_error` 或 `cap_truncated`，或者文件末尾缺失 `manifest`，均说明导出并不完整。

有关导出格式、行数限制与完整性校验的详细说明，请参阅[追踪导出与分析](/antix/concepts/analytics)。如果记录持续缺失，请提取具体请求样例联系支持，而不要为了测试计数器而额外发起测试请求。

## 付款或额度仍待确认 {#payment-or-credit-still-pending}

请前往门户的 **Usage & billing（用量与计费）** 页面查看现有充值记录及其状态。从结账页面返回或关闭浏览器标签页，均不代表付款已成功或已取消。对于可能已经付款成功的订单，请勿通过重复发起结账来尝试解决。

如果付款状态持续显示为待确认或审核中，或者预期额度迟迟未到账，请使用该笔充值记录旁提供的账单支持操作（若已启用），并附上充值意图 ID 或参考号、大致时间与金额，以及界面当前显示的状态。切勿在支持请求中填入银行卡信息。提交账单工单仅代表登记了求助请求——并不代表退款已获批，也不会立即改变账户余额。

## 联系支持 {#contact-support}

当您所在的部署启用了支持功能时，可在 [Antix 门户](https://portal.antigma.ai)中打开 **Support（支持）**：

- **Report an issue（报告问题）** — 用于反馈 API 或产品故障。
- **Feature request（功能建议）** — 用于提交您需要的新能力或改进建议。
- **Billing（账单）** — 用于处理额度未到账、扣费疑问或退款申请。如条件允许，请优先使用对应充值记录上的支持入口。

支持入口的可用性取决于具体部署配置。如果门户未提供内置表单，请使用门户页面展示的支持链接，或直接联系您的部署管理员，并妥善保存返回的工单参考号。一般问题与功能建议表单仅支持单向提交，请勿假设存在工单对话线程或状态追踪页；账单支持入口在启用时支持查看工单状态与详情。

提交时请附上故障现象、预期行为、复现步骤、带时区的时间戳、请求路由与模型、HTTP 状态码与错误代码、响应中的 `X-Antix-Request-Id`，以及相关的任务、追踪或充值标识符。仅在必要时提供最小化且经过脱敏清理的请求样例，并务必移除 API 密钥、Bearer 令牌、Cookie、签名存储 URL 及私密提示词内容——支持表单可能会直接拒绝包含敏感信息的提交。排查导出问题时，提供导出的 `manifest` 与筛选参数通常比上传包含用户提示词的导出文件更有帮助。
