---
title: "虚拟密钥与预算"
description: "带有原子速率限制和硬性支出上限的范围限制密钥。"
sidebar_position: 3
---

# 虚拟密钥与硬预算 {#virtual-keys--hard-budgets}

绝不要分发原始的提供商密钥。Antix 签发的**虚拟密钥（Virtual Keys）**可作为中间件拦截器，在将流量路由到上游之前原子化地验证预算。

## 创建密钥 {#creating-a-key}

可以直接在 Antix 门户仪表板 [https://portal.antigma.ai](https://portal.antigma.ai) 中生成密钥。

创建密钥时，您可以设置：
- **Key Name（密钥名称）**：用于在门户和追踪中标识密钥的标签。
- **Budget（预算）**：可选的硬性支出上限（`max_budget`，单位为美元）。目前没有日/月/生命周期期限选择器——它是一个单一的固定上限。

密钥会被安全存储——明文在创建时仅返回**一次**。由门户签发的密钥以 **`sk-antix-…`** 开头。

:::note
单密钥的模型允许列表和单密钥的速率限制（rpm/tpm）目前尚未在门户的密钥创建流程中暴露。有关网关当前的速率限制（全局而非单密钥），请参阅[错误处理](/antix/concepts/error-handling)。
:::

## 可靠计费 {#reliable-billing}

Antix 严格执行预算以防止超支。在任何请求发送到上游之前，成本都会被估算和预留。如果请求将超出密钥的 `max_budget`，它会被立即拒绝并返回 `402 Payment Required` 错误。

一旦请求完成或被取消，成本就会被准确核算。这种严格的执行消除了双重支出，并确保您的预算上限得到严格遵守，即使在繁重的并发负载下也是如此。
