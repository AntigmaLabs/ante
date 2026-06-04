
<p align="center">
  <img src="docs-site/static/assets/ante.png" width="80" alt="Ante" />
</p>

<p align="center">
  <a href="https://discord.gg/CbAsUR434B"><img src="https://img.shields.io/badge/Discord-Join%20Us-5865F2?logo=discord&logoColor=white" /></a>
  <a href="https://docs.antigma.ai"><img src="https://img.shields.io/badge/Docs-docs.antigma.ai-orange?logo=safari&logoColor=white" /></a>
  <a href="https://twitter.com/antigma_labs"><img src="https://img.shields.io/badge/X-@antigma__labs-black?logo=x&logoColor=white" /></a>
  <a href="https://huggingface.co/Antigma"><img src="https://img.shields.io/badge/HuggingFace-Antigma-yellow?logo=huggingface&logoColor=white" /></a>
</p>

# Ante

> **⚠️ Alpha Preview**
> Ante is currently in alpha and provided as a research preview. Expect breaking changes and incomplete functionality. macOS and Linux only.

Ante is an **AI-native, cloud-native, local-first agent runtime** built by [Antigma Labs](https://antigma.ai) — a single ~15MB Rust binary with zero runtime dependencies.

Today it's the lightest, fastest terminal agent you can run: bring your own key, switch between 12+ providers freely, or go fully offline with local models — no account required. That same tiny core is the point. It's light enough to run by the *thousands* — the substrate we're building for self-organizing intelligence.

> **We care about the harness, not the model or the prompts.**

Most agents ship a Node or Python runtime and a tree of dependencies — fine for a single chat, ruinous when you want a hundred running at once. Ante is hand-written Rust with the heavy parts — `Grep`, `git`, and local inference — embedded in the same binary and run in the same process. The result: **~7× less memory** than Claude Code across 20 parallel tasks, **#1 on the public [Terminal Bench](docs-site/docs/benchmarks/eval.mdx) 1.0 leaderboard**, and a deliberate resistance to AI-generated slop.

## Key Features

- **Lightweight agent core** — ~15MB binary, zero dependencies. Built for minimal overhead and maximum throughput.
- **Native local models** — Built-in local inference integration. No API keys, no internet, no data leaving your device.
- **Zero vendor lock-in** — Bring your own API key or local model. Switch between 12+ providers freely. No account required.
- **Client-daemon architecture** — Run as an interactive TUI, headless CLI, or long-lived server (`ante serve`).
- **Channel integrations** — Run Ante as a Slack or Discord bot with `ante gateway`.
- **Multi-agent orchestration** — Spawn sub-agents and coordinate complex tasks across independent, decentralized, and centralized architectures. [See the patterns →](https://docs.antigma.ai/experimental/agent-org)
- **Extensible** — Custom skills, sub-agents, and persistent memory across sessions.

> [!NOTE]
> **Benchmark proven.** Public, [reproducible evals](https://docs.antigma.ai/benchmarks/eval) on user-facing builds.
> - 🥇 #1 on **Terminal Bench 1.0**
> - 🥇 #1 on **Terminal Bench 2.0** (verified agent)
> - More to come

## Performance

Ante is designed for the **cellular-native** thesis: agents lightweight enough to run hundreds of replicas in parallel on a single machine. Its ~15MB Rust core uses a fraction of the memory, CPU, and disk I/O of comparable agents — making mass parallelism practical without specialized infrastructure.

Docker resource usage across 20 parallel tasks (Ante vs Claude Code vs Opencode):

![Resource Usage Comparison](docs-site/docs/benchmarks/compare_animated.gif)

Across 20 parallel tasks, Ante uses **~7× less peak memory**, **~9× less average CPU**, and generates **~5× less total disk I/O** than Claude Code — while completing the same workload. See the [comparison table](docs-site/docs/benchmarks/compare_table.md) and the [benchmark details](docs-site/docs/benchmarks/eval.mdx) for the evaluation methodology and results.

## Quick Start

### Installation

Ante is distributed as a single, self-contained binary with no external dependencies — just download and run.

```sh
curl -fsSL https://ante.run/install.sh | bash

# Install a specific release channel
curl -fsSL https://ante.run/install.sh | bash -s -- nightly
```

### Interactive TUI

```sh
ante
```

### Headless Mode

```sh
# Fix a bug
ante -p "find and fix the failing test in src/auth"

# Review a diff
git diff | ante -p "review this for security issues"

# Use a different provider
ante --provider openai --model gpt-5.5 -p "refactor the database module"

# Resume a saved session
ante --resume ses_01ARZ3NDEKTSV4RRFFQ69G5FAV -p "now add tests"

# Run fully offline with a local GGUF model
ante --offline-model ~/.ante/models/Qwen3.5-9B-Q4_K_M.gguf \
  -p "add error handling to src/main.rs"
```

### Server Mode

```sh
ante serve
```

### Gateway Mode

```sh
ante gateway
```

### Update Ante

```sh
ante update

# One-off update from a different channel
ante update --channel nightly
```

## Example Usages with TUI

<table>
<tr>
<td width="50%">

**[Providing Context: Files & Folders](https://docs.antigma.ai/cookbook/providing-context)**

![Adding file context with @ mentions](docs-site/static/assets/cookbook/files.gif)

</td>
<td width="50%">

**[Interrupting & Steering](https://docs.antigma.ai/cookbook/steering)**

![Interrupting the agent with Escape](docs-site/static/assets/cookbook/interrupt.gif)

</td>
</tr>
<tr>
<td width="50%">

**[Models, Providers & Thinking](https://docs.antigma.ai/cookbook/models-and-thinking)**

![Selecting a model and provider](docs-site/static/assets/cookbook/model.gif)

</td>
<td width="50%">

**[Subscription Login](https://docs.antigma.ai/cookbook/login)**

![Connecting to a provider via /connect](docs-site/static/assets/cookbook/connect.gif)

</td>
</tr>
</table>

[See all cookbook guides](https://docs.antigma.ai/cookbook/login)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Clients                             │
│                                                             │
│   ┌───────────┐    ┌───────────┐    ┌────────────────────┐  │
│   │    TUI    │    │ Headless  │    │    ante serve      │  │
│   │  (ante)   │    │ (ante -p) │    │  (stdio / ws)      │  │
│   └─────┬─────┘    └─────┬─────┘    └─────────┬──────────┘  │
└─────────┼────────────────┼─────────────────────┼────────────┘
          │                │                     │
          ▼                ▼                     ▼
┌─────────────────────────────────────────────────────────────┐
│                         Daemon                              │
│                                                             │
│   Session ──▶ Turn ──▶ Step                                │
│                                                             │
│   ┌──────────┐  ┌──────────────┐  ┌───────────────────┐     │
│   │  Tools   │  │  Permission  │  │  Skills / Agents  │     │
│   └──────────┘  └──────────────┘  └───────────────────┘     │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                     LLM Providers                           │
│                                                             │
│   Anthropic · OpenAI · Gemini · Grok · Open Router · Local  │
└─────────────────────────────────────────────────────────────┘
```

## Supported Providers

Ante works with 12+ providers out of the box:

| Provider | Example Models |
|----------|---------------|
| Anthropic | Claude Sonnet 4.5, Opus 4.6 |
| OpenAI | GPT-5 family |
| Google Gemini | Gemini 3 family |
| Grok (xAI) | Grok 4 |
| Open Router | Multiple providers |
| Local (GGUF) | Any GGUF model via built-in llama.cpp |
| ...and more | Vertex AI, Zai, Antix, OpenAI-compatible |

Configure providers via environment variables (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.) or OAuth. Add custom providers in `~/.ante/catalog.json`.

## FAQ

### Why <span style="color: #f59e0b;">An</span>other <span style="color: #f59e0b;">Te</span>rminal agent?

The name is the answer: **An**other **Te**rminal agent — and *ante*, the stake you put on the table to play. Ante is fast, lightweight, and the only terminal agent with native local inference built in. We believe a self-contained agent core that self-organizes is the foundation of the coming agent economy.

<details>
<summary><b>How is Ante different than other agents</b></summary>
On the high level, it has most of your favorite features (Multi-agents, skills, etc.) of your favorite agents (like Claude Code, Codex, etc.) 

- Ante is built from scratch in native Rust. We're obsessed with being self-contained — only essential libraries, no framework or runtime dependencies. 

- You only need an LLM provider configured to run it. And if you have the hardware, you don't even need one — Ante natively supports a private inference engine. 

- This resulted in ~15MB self-contained binary and multi-agent orchestration designed to run hundreds of replicas in parallel at scale.
See the [benchmark details](docs-site/docs/benchmarks/eval.mdx) across 20 parallel tasks for concrete numbers.

- No vendor lock-in, not even to ourselves. You don't need an account and can reuse your favorite API credentials. 

</details>

<details>
<summary><b>What's your advantage over similar projects?</b></summary>

Most projects in this space are written in TypeScript or Python and carry heavy runtime dependencies (Node.js, CPython). In practice that usually means an order-of-magnitude larger resource footprint (often ~10×).

We genuinely mean it when we say the agent should be self-contained:

- Core components like `Grep` (fully rebuilt and customized) and `git` are **embedded into the same binary** (while maintaining ~15MB size) and run **in the same process** at runtime — not shelled out to external processes — to prevent accidental resource leakage.
- We've built our own inference engine from the ground up. (See [nanochat-rs](https://github.com/AntigmaLabs/nanochat-rs), a toy version of the kind of work that goes into it.)
- There's an opt-in, fully integrated server-side experience at [antix.antigma.ai](https://antix.antigma.ai).
- And much more in the pipeline — including a multi-agent platform.

Beyond the footprint, it comes down to agent architecture — and ultimately to *who* is building it, and with what philosophy. Anyone can fork a binary; taste and engineering rigor don't copy. Those differences leak into every detail of the product.

</details>

<details>
<summary><b>Why care about runtime optimization like memory and I/O if model inference is usually the biggest bottleneck?</b></summary>

For one-on-one agent interactions, runtime overhead like memory usage and I/O is often less important than model inference.

But our vision is much bigger: millions of agents self-organizing and communicating at massive scale. At that point, even small inefficiencies get multiplied millions or billions of times, so runtime optimization becomes economically significant.
</details>

<details>
<summary><b>Can I run Ante completely offline?</b></summary>

Yes. Ante has a built-in llama.cpp engine that runs GGUF models locally. It handles engine installation, model discovery, and memory management automatically. No API keys or internet connection required.
</details>

<details>
<summary><b>Can I use my own custom models or providers?</b></summary>

Yes. Create a `~/.ante/catalog.json` file to add or override providers and models with custom endpoints, API keys, and configurations. Any OpenAI-compatible API works.
</details>

<details>
<summary><b>What is the <code>ante serve</code> mode for?</b></summary>

Server mode runs Ante as a long-lived daemon that communicates over a structured JSONL protocol. It's ideal for building editor plugins, web UIs, and custom integrations on top of Ante.
</details>

## Documentation

Full documentation is available at [docs.antigma.ai](https://docs.antigma.ai).
The source code is in `docs-site/docs`
