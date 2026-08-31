---
title: "资源占用对比（Resource Footprint）"
sidebar_label: "资源占用对比"
description: "在 Docker 中对 20 个并行任务进行实测的资源占用数据 — Ante vs Claude Code vs Opencode"
---

# 资源占用对比（Resource Footprint）

大规模并发只有在单个智能体足够轻量、低开销时才具有实际可行性。为了量化这一点，我们在相同的约束条件下使用 Docker 分别在 Ante、Claude Code 和 Opencode 上并行运行了 20 个相同任务，并全程记录了 CPU、内存与磁盘占用情况。

核心结论：在相同负载下，Ante 相比 Claude Code 峰值内存减少约 **7 倍**，平均 CPU 占用减少约 **9 倍**，磁盘 I/O 减少约 **5 倍**。

![20 个并行任务的资源占用对比](./compare_animated.gif)

完整测试数据如下。

## 耗时概览（Overview） {#overview}

| 智能体 | 耗时 Wall Time (秒) |
|-------|--------------|
| **Ante** | 940 |
| **Claude** | 627 |
| **Opencode** | 1076 |

## CPU 使用率（CPU Usage (%)） {#cpu-usage-}

| 智能体 | 峰值 Peak | 平均 Avg | P95 | P99 |
|-------|------|-----|-----|-----|
| **Ante** | 94.4 | 1.3 | 6.2 | 12.3 |
| **Claude** | 89.5 | 12.1 | 31.0 | 43.4 |
| **Opencode** | 90.8 | 3.8 | 27.1 | 62.3 |

## 内存占用（Memory Usage (MiB)） {#memory-usage-mib}

| 智能体 | 峰值 Peak | 平均 Avg | P95 | P99 |
|-------|------|-----|-----|-----|
| **Ante** | 1968 | 683 | 1489 | 1550 |
| **Claude** | 13877 | 3685 | 8927 | 9535 |
| **Opencode** | 12944 | 2077 | 11266 | 12852 |

## 磁盘占用（Disk Usage (MiB)） {#disk-usage-mib}

| 智能体 | 峰值 Peak | 平均 Avg | P95 | P99 |
|-------|------|-----|-----|-----|
| **Ante** | 7041 | 3121 | 6975 | 6976 |
| **Claude** | 22467 | 4304 | 10128 | 10193 |
| **Opencode** | 59689 | 6046 | 29108 | 34744 |

## 磁盘读取速率（Disk Read Rate (MB/s)） {#disk-read-rate-mbs}

| 智能体 | 峰值 Peak | P95 | P99 |
|-------|------|-----|-----|
| **Ante** | 3.5 | 0.0 | 0.1 |
| **Claude** | 263.9 | 10.4 | 101.9 |
| **Opencode** | 284.1 | 0.1 | 10.6 |

## 磁盘写入速率（Disk Write Rate (MB/s)） {#disk-write-rate-mbs}

| 智能体 | 峰值 Peak | P95 | P99 |
|-------|------|-----|-----|
| **Ante** | 186.3 | 3.6 | 61.7 |
| **Claude** | 302.3 | 26.6 | 113.0 |
| **Opencode** | 302.9 | 14.5 | 296.6 |

## 磁盘总 I/O 量（Total Disk I/O (MB)） {#total-disk-io-mb}

| 智能体 | 总读取 Total Read | 总写入 Total Write |
|-------|------------|-------------|
| **Ante** | 24 | 2785 |
| **Claude** | 17444 | 15116 |
| **Opencode** | 2224 | 31427 |
