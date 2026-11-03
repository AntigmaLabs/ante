---
title: "追踪导出与分析"
description: "查看账户用量、理解成本来源，并以 JSON 或 NDJSON 导出端点追踪。"
sidebar_position: 6
---

# 追踪导出与分析 {#trace-export--analytics}

在 [Antix 门户](https://portal.antigma.ai) 中查看账户用量，并深入分析通过[端点](/antix/concepts/endpoints)发送的请求。账户用量与端点追踪的范围不同。

## 选择合适的视图 {#find-the-right-view}

| 视图 | 用途 |
| --- | --- |
| 概览 / 用量与计费 | 账户总量和模型用量。 |
| 请求历史 | 账户的单次请求、token 数量和成本。 |
| API 密钥 | 归属于特定密钥的用量。 |
| 端点 → 选择端点 → 概览 | 该端点的请求、文本 token 和成本摘要。 |
| 端点 → 选择端点 → 追踪 | 筛选后的端点请求及详情抽屉。 |
| 端点活动 | 支持时提供按会话探索的实验性视图。 |

账户历史可按界面提供的时间、模型、密钥和认证模式筛选。端点追踪支持时间、模型、密钥、认证模式，以及对已存储输入/输出进行不区分大小写的子串搜索。打开一条追踪，可查看获准展示的请求/响应字段、工具、token 数、延迟，以及已记录的首 token 时间（TTFT）。缺失延迟或 TTFT 不代表测量值为零。

分析数据可能晚于请求完成。加载、预热、陈旧或不可用的结果都不能证明没有用量。请等待视图刷新，检查时间窗口和筛选条件，避免反复刷新不可用的视图。持续缺失时，参阅[故障排查与支持](/antix/concepts/troubleshooting)。

### 成本与单位 {#costs-and-units}

API 提供 `cost_source` 时，应结合 `cost_usd` 理解：

| `cost_source` | 含义 |
| --- | --- |
| `billing_ledger` | 找到了该请求的 Antix 精确扣款记录。 |
| `request_price` | 按存储的请求时定价计算的成本；不能证明最终扣款金额。 |
| `current_pricing_estimate` | 按当前模型价格估算的成本。BYOK 成本使用此来源。 |
| `mixed` | 汇总值包含不同成本来源。 |

BYOK 估算不是 Antix 收费，也不是提供商账单。当前价格估算可能随价格变动而变化，缺失定价也可能使估算不完整。不要用分析估算证明账户余额已被扣除。

**视频 token 使用独立单位。** 账户统计提供 `total_video_tokens`，历史行提供 `video_tokens`。它们不会加入文本输入/输出总量。成功计费的 Seedance 任务在异步用量记录后计为一次请求；失败、取消和过期任务不生成已计费的视频用量行。

Seedance 用量目前**没有端点归属**。请在账户历史和密钥用量中查看，不要在端点追踪及其导出中查找。端点导出的 token 字段是文本/缓存计数，不是视频 token 总量。详见[视频模型与 API](/antix/concepts/videos)。

## 从门户导出 {#export-from-the-portal}

1. 打开**端点**，选择您有权查看的端点，再打开**追踪**。
2. 展开筛选控件并选择有限的时间范围，按需增加模型、密钥、认证模式或内容筛选。
3. 使用下载控件选择 **JSON** 或 **NDJSON**，并选择是否包含正文。
4. 使用结果前，先检查下载文件的最终 manifest（清单）。

导出使用已登录的门户身份，并执行与查看追踪相同的端点可见性检查。仅持有端点 URL 并不授予访问权限。API 为：

```text
GET /api/portal/endpoints/{endpoint_id}/traces/download
```

### 导出参数 {#export-parameters}

| 参数 | 规则 |
| --- | --- |
| `from`、`to` | 均必填，使用 RFC 3339 时间戳。`to` 必须晚于 `from`，最大窗口为 90 天。 |
| `format` | `ndjson`（默认）或 `json`。 |
| `include_bodies` | 布尔值，默认 `false`。 |
| `model` | 可选，精确模型 ID。 |
| `auth_mode` | 可选，`virtual_key`、`oauth` 或 `byok`。 |
| `vk_id` | 可选，追踪数据中的密钥标识符，绝不是密钥明文。 |
| `content_search` | 可选，不区分大小写的字面子串，最多 512 个字符。 |

时间戳边界均为闭区间。若拆分为相邻窗口，请按 `trace_id` 去除边界重复记录。

结果上限为 **10,000 行**。收到 `400 cap_exceeded` 时应缩小时间范围或增加筛选条件，响应包含上限和估算行数。当前实现还限制**每个网关实例**内每位用户每小时最多五次通过准入的下载尝试。`429 rate_limited` 在响应正文中包含 `retry_after_seconds`，请等待，不要循环重试。

### 使用门户会话下载 {#download-through-your-portal-session}

以下 API 示例可在**已登录的门户源站**浏览器控制台运行。将 `YOUR_ENDPOINT_UUID` 替换为端点 ID。示例导出最近 24 小时的数据，不包含请求/响应正文，并在保存前检查 manifest。

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

该示例使用已有门户会话，无需将 API 密钥或会话 Cookie 粘贴到示例或导出文件中。

## 读取文件并验证完整性 {#read-the-file-and-verify-completeness}

**JSON** 返回包含 `traces` 数组和 `manifest` 对象的结构。**NDJSON** 每行一条追踪，最后一行包含 `_manifest: true`；最后一行是元数据，不是另一条追踪。

追踪行包含 `trace_id`、时间戳、端点/模型、认证模式、`is_byok`、token 计数、`cost_usd` 和 `cost_source`。延迟字段可能为 null，只有可用时才提供会话/代理签名。这些推理行的导出 `status` 当前为 `ok`，并非所有被拒绝或失败 HTTP 请求的完整日志。

Manifest 包含 `schema_version`、筛选条件、`row_count`、`include_bodies`、`format`、`generated_at`、脱敏元数据和 `error_code`。请检查：

- 文件可以解析，并具有最终 manifest。
- 追踪记录数量与 `row_count` 一致。
- 将结果视为完整前，确认 `error_code` 为 null。

流已发送 HTTP 200 后，无法再改为错误状态。Manifest 中的 `ch_timeout` 或 `ch_error` 表示导出不完整，应稍后重试或缩小窗口。`cap_truncated` 也要求缩小查询范围。断开或截断的下载可能根本没有最终 manifest。

### 正文与可见性 {#bodies-and-visibility}

默认不包含正文。使用 `include_bodies=true` 时，获准提供的 `input` 和 `output` 字段会经过门户共享的正文投影/脱敏规则。原始提供商请求/响应字段和认证材料仍按策略排除；导出并非上游交互的逐字节重放。

脱敏不保证移除用户提示或模型回答中的所有秘密与个人信息。请将含正文文件视为敏感数据。读取审计元数据会记录导出访问和正文选项。90 天查询窗口是导出限制，不是数据保留承诺；请参阅[隐私、安全与数据保留](/antix/concepts/security)。

导出的 `trace_id`、历史中的请求标识符，以及响应头 `X-Antix-Request-Id` 是不同诊断字段，不要假设可以互换。[联系支持](/antix/concepts/troubleshooting#contact-support)时可提供您掌握的标识符。
