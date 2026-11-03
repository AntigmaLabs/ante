---
title: "故障排查与支持"
description: "诊断认证、模型访问、视频、分析与付款问题，并收集有效的支持信息。"
sidebar_position: 7
---

# 故障排查与支持 {#troubleshooting--support}

首先查看 HTTP 状态、响应中机器可读的错误码或类型，以及错误消息。认证与推理路由可能采用不同错误结构，因此请同时保留响应正文和状态。通用参考见[错误处理](/antix/concepts/error-handling)。

## 收集有效的故障报告 {#collect-a-useful-failure-report}

将响应头与正文分开保存。以下只读示例使用 Antix 密钥检查模型发现：

```bash
curl --silent --show-error --fail-with-body \
  --dump-header antix-response-headers.txt \
  --output antix-response.json \
  https://antix.antigma.ai/v1/models \
  -H "Authorization: Bearer $ANTIX_KEY"
```

保留响应中的 **`X-Antix-Request-Id`**、带时区的时间戳、路由、模型 ID、状态及错误正文。该响应标识符由 Antix 生成，发送同名请求头无法设置它。如果可用，也请分别标注历史/追踪标识符或视频任务 ID。

## 认证与模型访问 {#authentication-and-model-access}

| 现象 | 检查 / 下一步 |
| --- | --- |
| 401，缺少或无效令牌 | 提供有效 Antix 密钥；OAuth/会话凭据过期时重新登录。提供商 BYOK 密钥应走对应提供商通道。 |
| `/v1/models` 返回 401 | 此发现路由需要认证，请发送将用于推理的凭据。 |
| 模型缺失或 `model_not_found` | 检查已认证的模型列表，Seedance 则检查 `/v1/videos/models`。模型必须已启用、具备凭据和定价，并获当前凭据许可。 |
| 403 `model_not_allowed` / `provider_not_allowed` | 密钥/账户策略不允许该模型/提供商。有余额并不自动获得所有模型访问权。请选择允许的模型或联系部署管理员。 |
| 403 `quota_exceeded` | 密钥 token 配额已用尽，与资金余额不同，反复请求不会重置配额。 |
| 403 `ledger_request_not_supported` | 路由、请求选项或凭据超出账户支持范围。按错误消息移除不支持的选项，不要循环重试原请求。 |
| 提供商/路径不匹配 | 确保类型化端点的提供商、`X-Antix-Provider` 与 BYOK 凭据一致。使用准确的 SDK 基地址，不要遗漏原生 API 版本路径段。 |
| 端点 URL 返回 410 | 端点已归档，请配置一个活跃端点。重试旧 URL 不会恢复它。 |

## 余额、限流与临时故障 {#balance-throttling-and-temporary-failures}

- **402 `ledger_budget_exceeded`：** 可用余额不足以覆盖请求预留。待处理请求/视频任务可能在最终收费前占用资金。开始更多任务前请检查门户余额和现有任务。
- **429：** 请查看具体错误码。`ledger_admission_busy`、提供商限流、`video_task_limit` 和 `video_download_busy` 原因不同。有 `Retry-After` 时遵循其要求，降低并发并退避。追踪导出限流则在正文中报告 `retry_after_seconds`。
- **502/503：** 保留请求 ID 和错误正文。只读操作可退避重试。对于生成请求，先考虑原请求是否已到达提供商；连接失败不普遍保证生成已取消或未收费。

不要假设 `Idempotency-Key` 能让任意推理或视频创建安全重试。只有 API 明确为该操作记录了幂等合同时才能依赖它。

## 视频问题 {#video-problems}

| 现象 | 检查 / 下一步 |
| --- | --- |
| `video_requires_antix_key` | Seedance `/v1/videos` 需要 Antix 凭据，而非 BytePlus/提供商 BYOK 密钥。 |
| MiniMax `platform_key_not_supported` | H3 路由需要 MiniMax BYOK 密钥和 `X-Antix-Provider: minimax`。 |
| 任务持续 `pending` / `in_progress` | 正常执行窗口内使用已保存的同一任务 ID 轮询。约 75 分钟后仍无法确认结果时，请携带该 ID 联系支持，任务可能需要人工审核。并发名额释放后，资金预留仍可能保留。不要为了收回预留而重新创建任务。 |
| `video_not_ready` | 等待 `completed` 后再下载。 |
| `video_expired`、`video_download_limit`、`video_unavailable` | 输出已不可下载。请保留本地副本，轮询无法恢复旧 URL。 |
| 任务返回 404 | 检查准确的 `vid_…` 标识符及凭据所属账户。 |
| 已完成视频未出现在端点追踪中 | Seedance 目前没有端点归属。请查看账户请求历史/密钥用量，并等待异步用量记录。 |

[视频模型与 API](/antix/concepts/videos)介绍请求字段、任务限制、下载行为和计费。不要仅因为轮询或下载失败就重新创建任务。

## 分析缺失或导出不完整 {#missing-analytics-or-an-incomplete-export}

1. 检查账户、端点、时间窗口/时区、模型、密钥及认证模式筛选。账户用量与端点追踪范围不同。
2. 等待预热/陈旧视图刷新。视图不可用不等于没有用量。
3. 区分文本和视频 token；将估算与账户扣款比较前，先查看 `cost_source`。
4. 收到 `cap_exceeded` 时缩小导出窗口或增加筛选；限流时等待报告的延迟。
5. 检查最终导出 manifest。HTTP 200 但包含 `ch_timeout`、`ch_error` 或 `cap_truncated`，或者缺少 manifest，都不是完整导出。

格式、限制和完整性检查见[追踪导出与分析](/antix/concepts/analytics)。记录持续缺失时，请报告具体样例，不要通过制造更多请求来测试计数器。

## 付款或额度仍待确认 {#payment-or-credit-still-pending}

在**用量与计费**中查看现有充值及状态。从结账返回或关闭浏览器标签页都不能证明付款或取消已完成。不要反复创建新结账来解决可能已成功付款的问题。

付款持续待确认/审核中，或预期额度未到账时，请使用该笔充值提供的账单支持入口。附上充值意图 ID/参考号、大致时间与金额，以及界面显示状态。不要提供银行卡资料。账单工单只是求助记录，不代表退款获批，也不会立即更改余额。

## 联系支持 {#contact-support}

部署启用时，可在[门户](https://portal.antigma.ai)打开**支持**：

- **报告问题**：API/产品故障。
- **功能建议**：需要的能力。
- **账单**：额度未到账、收费或退款。可用时优先使用相关充值的支持入口。

可用性取决于部署。若未提供表单，请使用门户显示的支持链接或联系部署管理员。保留返回的工单参考号。一般问题/功能建议仅支持提交，不要假设存在工单对话或状态页；账单入口启用时支持状态/详情查看。

请提供现象、预期行为、复现步骤、带时区的时间戳、路由/模型、状态/错误码、响应请求 ID，以及相关任务/追踪/充值标识符。必要时仅分享最小化且已清理的请求。删除 API 密钥、Bearer 令牌、Cookie、签名存储 URL 和私密提示内容；支持表单可能拒绝敏感材料。导出问题通常提供 manifest 与筛选条件即可，无需发送包含客户提示的文件。
