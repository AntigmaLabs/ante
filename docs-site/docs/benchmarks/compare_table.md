---
title: "Resource Footprint"
description: "Docker-measured resource usage across 20 parallel tasks — Ante vs Claude Code vs Opencode"
---

# Resource Footprint

Mass parallelism only pays off if a single agent is cheap enough to multiply. To measure that, we ran the same 20 tasks in parallel through Ante, Claude Code, and Opencode, each inside Docker with identical constraints, and recorded CPU, memory, and disk usage throughout the run.

Headline result: Ante uses **~7× less peak memory**, **~9× less average CPU**, and **~5× less disk I/O** than Claude Code on the same workload.

![Resource usage comparison across 20 parallel tasks](./compare_animated.gif)

The full measurements are below.

## Overview

| Agent | Wall Time (s) |
|-------|--------------|
| **Ante** | 940 |
| **Claude** | 627 |
| **Opencode** | 1076 |

## CPU Usage (%)

| Agent | Peak | Avg | P95 | P99 |
|-------|------|-----|-----|-----|
| **Ante** | 94.4 | 1.3 | 6.2 | 12.3 |
| **Claude** | 89.5 | 12.1 | 31.0 | 43.4 |
| **Opencode** | 90.8 | 3.8 | 27.1 | 62.3 |

## Memory Usage (MiB)

| Agent | Peak | Avg | P95 | P99 |
|-------|------|-----|-----|-----|
| **Ante** | 1968 | 683 | 1489 | 1550 |
| **Claude** | 13877 | 3685 | 8927 | 9535 |
| **Opencode** | 12944 | 2077 | 11266 | 12852 |

## Disk Usage (MiB)

| Agent | Peak | Avg | P95 | P99 |
|-------|------|-----|-----|-----|
| **Ante** | 7041 | 3121 | 6975 | 6976 |
| **Claude** | 22467 | 4304 | 10128 | 10193 |
| **Opencode** | 59689 | 6046 | 29108 | 34744 |

## Disk Read Rate (MB/s)

| Agent | Peak | P95 | P99 |
|-------|------|-----|-----|
| **Ante** | 3.5 | 0.0 | 0.1 |
| **Claude** | 263.9 | 10.4 | 101.9 |
| **Opencode** | 284.1 | 0.1 | 10.6 |

## Disk Write Rate (MB/s)

| Agent | Peak | P95 | P99 |
|-------|------|-----|-----|
| **Ante** | 186.3 | 3.6 | 61.7 |
| **Claude** | 302.3 | 26.6 | 113.0 |
| **Opencode** | 302.9 | 14.5 | 296.6 |

## Total Disk I/O (MB)

| Agent | Total Read | Total Write |
|-------|------------|-------------|
| **Ante** | 24 | 2785 |
| **Claude** | 17444 | 15116 |
| **Opencode** | 2224 | 31427 |
