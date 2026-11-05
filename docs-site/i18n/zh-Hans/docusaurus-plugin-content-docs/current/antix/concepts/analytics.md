---
title: "追踪导出与分析"
description: "查看账户用量、理解成本来源，并以 JSON 或 NDJSON 导出端点追踪。"
sidebar_position: 6
---

# 追踪导出与分析 {#trace-export--analytics}

在 [Antix 门户](https://portal.antigma.ai)中查看账户用量，并深入分析通过[端点](/antix/concepts/endpoints)发送的请求。账户用量与端点追踪的统计范围不同。

## 选择合适的视图 {#find-the-right-view}

| 视图 | 用途 |
| --- | --- |
| **Overview / Usage & billing（概览 / 用量与计费）** | 账户总量与各模型用量。 |
| **Request history（请求历史）** | 账户的单次请求、token 数量和成本。 |
| **API keys（API 密钥）** | 归属于特定密钥的用量。 |
| **Endpoints（端点） → 选择端点 → Overview（概览）** | 该端点的请求数、文本 token 和成本摘要。 |
| **Endpoints（端点） → 选择端点 → Traces（追踪）** | 筛选后的端点请求及观察抽屉（Observation drawer）。 |
| **Endpoint Activity（端点活动）** | 在支持的端点上提供按会话聚合的探索视图（实验性）。 |

对于账户请求历史，可按界面提供的时间、模型、密钥和认证模式进行筛选。端点追踪支持按时间、模型、密钥、认证模式筛选，并支持对已存储的输入和输出正文进行不区分大小写的子字符串搜索。打开单条追踪记录，即可查看获准展示的请求与响应字段、工具调用、token 数量、延迟，以及已记录的首字节时间（TTFT）。缺失延迟或 TTFT 并不代表实际测量值为零。

分析数据的显示可能会滞后于请求完成时间。处于加载中、预热中、未及时刷新或暂不可用状态的结果，均不代表实际用量为零。请等待视图刷新，核对时间窗口和筛选条件，并避免反复刷新暂不可用的视图。如果数据持续缺失，请参阅[故障排查与支持](/antix/concepts/troubleshooting)。

### 成本与单位 {#costs-and-units}

当 API 返回 `cost_source` 时，请结合 `cost_usd` 一并解读：

| `cost_source` | 含义 |
| --- | --- |
| `billing_ledger` | 找到了该请求对应的 Antix 精确扣款记录。 |
| `request_price` | 根据请求发生时存储的定价计算出的成本；不作为最终实际扣款的凭据。 |
| `current_pricing_estimate` | 根据当前模型定价估算出的成本。BYOK 流量成本使用此来源。 |
| `mixed` | 聚合统计中混合了多种不同的成本来源。 |

BYOK 估算值既不是 Antix 实际收取的费用，也不是上游提供商的账单。基于当前价格的估算可能会随定价调整而变化；若模型缺失定价，估算结果也可能不完整。请勿将分析视图中的估算金额作为账户余额已扣款的凭据。

**视频 token 是独立的计量单位。** 账户统计接口返回 `total_video_tokens`，请求历史行返回 `video_tokens`，它们不会计入文本提示或完成 token 的总量中。成功计费的 Seedance 任务在异步用量记录完成后会显示为一条请求记录；失败、已取消和已过期的任务不会生成计费视频用量行。

Seedance 用量目前**不支持按端点归属**。请在账户请求历史和密钥用量中查看，而不要在端点追踪或其导出文件中查找。端点导出文件中的 token 字段仅统计文本和缓存 token，不包含视频 token 总量。详见[视频模型与 API](/antix/concepts/videos)。

## 从门户导出 {#export-from-the-portal}

1. 打开 **Endpoints（端点）** 选项卡，选择您有权查看的端点，然后打开 **Traces（追踪）**。
2. 展开筛选控件并选择明确的时间范围，按需添加模型、密钥、认证模式或内容筛选条件。
3. 使用下载控件选择 **JSON** 或 **NDJSON** 格式，并选择是否包含请求与响应正文。
4. 在将导出文件视为完整结果之前，请先检查文件末尾的 `manifest`（清单）。

导出接口使用当前已登录的门户身份，并执行与在线查看追踪完全相同的端点可见性检查。仅凭端点 URL 本身无法获得导出权限。导出 API 路径为：

```text
GET /api/portal/endpoints/{endpoint_id}/traces/download
```

### 导出参数 {#export-parameters}

| 参数 | 规则 |
| --- | --- |
| `from`、`to` | 均为必填，使用 RFC 3339 时间戳。`to` 必须晚于 `from`；最大时间窗口为 90 天。 |
| `format` | `ndjson`（默认）或 `json`。 |
| `include_bodies` | 布尔值；默认为 `false`。 |
| `model` | 可选，精确的模型 ID。 |
| `auth_mode` | 可选，`virtual_key`、`oauth` 或 `byok`。 |
| `vk_id` | 可选，追踪数据中的密钥标识符（切勿传入密钥明文）。 |
| `content_search` | 可选，不区分大小写的字面子字符串，最多 512 个字符。 |

时间戳起止边界均为闭区间。如果将导出拆分为相邻的时间窗口，请按 `trace_id` 对重叠边界的记录去重。

单次导出的结果上限为 **10,000 行**。若收到 `400 cap_exceeded`，请缩小时间窗口或收紧筛选条件；错误响应中会包含行数上限和预估行数。当前实现还限制**每个网关实例**内每位用户每小时最多发起 5 次获准执行的下载尝试。`429 rate_limited` 响应正文中包含 `retry_after_seconds`；请按该时间等待，切勿循环重试。

### 使用门户会话下载 {#download-through-your-portal-session}

如需通过 API 调用，可在**已登录的门户站点**浏览器控制台中运行以下 JavaScript 示例。请将 `YOUR_ENDPOINT_UUID` 替换为您的端点 ID。该示例会导出最近 24 小时的数据（不含请求和响应正文），并在保存文件前校验 `manifest`：

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

该示例直接复用您当前的门户登录会话；切勿将 API 密钥或会话 Cookie 粘贴到示例代码或导出文件中。

## 读取文件并验证完整性 {#read-the-file-and-verify-completeness}

**JSON** 格式返回一个包含 `traces` 数组和 `manifest` 对象的 JSON 对象。**NDJSON** 格式每行输出一条追踪记录，最后一行输出带有 `_manifest: true` 的对象——该行是元数据，而非追踪记录。

每条追踪记录包含 `trace_id`、时间戳、端点与模型、认证模式、`is_byok`、token 计数、`cost_usd` 以及 `cost_source`。延迟字段可能为 `null`；会话/代理签名仅在可用时出现。对于这些已记录的推理行，导出的 `status` 当前均为 `ok`——导出文件并非所有被拒绝或失败的 HTTP 请求的完整日志。

`manifest` 包含 `schema_version`、筛选条件、`row_count`、`include_bodies`、`format`、`generated_at`、脱敏元数据以及 `error_code`。使用导出数据前请核对以下三项：

- 文件能够正常解析，且包含最终的 `manifest`。
- 追踪记录的实际条数与 `row_count` 一致。
- 在将结果视为完整导出之前，确认 `error_code` 为 `null`。

一旦流式响应已发出 HTTP `200` 状态码，便无法再将其替换为错误状态码。如果 `manifest` 中的 `error_code` 为 `ch_timeout` 或 `ch_error`，说明导出仅包含部分数据，请稍后重试或缩小时间窗口；若为 `cap_truncated`，同样需要缩小查询范围。中途断开或被截断的下载文件可能完全缺失末尾的 `manifest`。

### 正文与可见性 {#bodies-and-visibility}

导出默认不包含请求与响应正文。当设置 `include_bodies=true` 时，获准导出的 `input` 和 `output` 字段会经过门户共用的正文投影与脱敏规则处理。根据网关策略，原始提供商请求/响应字段以及身份验证凭据仍会被排除在外；导出内容并非上游通信的逐字节回放。

:::warning 追踪正文中的敏感数据
脱敏机制并不保证能移除用户提示词或模型回答中的所有机密信息与个人隐私。请将包含正文的导出文件视为敏感数据。导出访问操作及是否包含正文的选项均会记录在读取审计元数据中。90 天查询窗口仅是导出接口的范围限制，而非数据保留期限承诺——请参阅[隐私、安全与数据保留](/antix/concepts/security)。
:::

导出文件中的 `trace_id`、请求历史中的请求标识符，以及响应标头中的 `X-Antix-Request-Id` 是不同的诊断字段——请勿假设它们可以互换使用。在[联系支持](/antix/concepts/troubleshooting#contact-support)时，请附上您手头已有的标识符。
