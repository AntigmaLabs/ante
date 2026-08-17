
<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs-site/static/assets/ante-readme-banner-dark-1280x320.png">
    <img src="docs-site/static/assets/ante-readme-banner-light-1280x320.png" alt="Ante — substrate for self-organizing intelligence">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/AntigmaLabs/ante/releases"><img src="https://img.shields.io/github/v/release/AntigmaLabs/ante?include_prereleases&label=release&color=blueviolet" /></a>
  <a href="https://antigma.ai/eval"><img src="https://img.shields.io/badge/Terminal--Bench_2.1-live_results-2ea44f?logo=speedtest&logoColor=white" /></a>
  <a href="https://docs.antigma.ai"><img src="https://img.shields.io/badge/Docs-docs.antigma.ai-orange?logo=safari&logoColor=white" /></a>
  <a href="https://discord.gg/CbAsUR434B"><img src="https://img.shields.io/badge/Discord-Join%20Us-5865F2?logo=discord&logoColor=white" /></a>
  <a href="https://twitter.com/antigma_labs"><img src="https://img.shields.io/badge/X-@antigma__labs-black?logo=x&logoColor=white" /></a>
  <a href="https://huggingface.co/Antigma"><img src="https://img.shields.io/badge/HuggingFace-Antigma-yellow?logo=huggingface&logoColor=white" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" /></a>
</p>

# Ante

> **Alpha preview**: expect breaking changes and incomplete functionality. macOS and Linux only; on Windows we suggest [WSL](https://learn.microsoft.com/windows/wsl/install).

## Read this first

Two things many people ask about:

**Where is the source?** The core harness currently ships as a prebuilt binary; this repo holds the docs, protocol, SDK, and eval pipeline ([details](#whats-in-this-repo)). We are working out a way to ship the source code along with the binary, to address security and privacy concerns first, while taking the time to figure out how open source should work in the agentic era. Progress and discussion: [issue #21](https://github.com/AntigmaLabs/ante/issues/21). If you have concerns today, run Ante in a sandbox: it is a single binary with minimal runtime dependencies, built to be easy to deploy in a container or on a remote machine.

**Is there telemetry?** Yes, and it is opt-out: set `ANTE_TELEMETRY=off` to disable export entirely. What it sends is anonymous — a random installation label you can delete and re-mint, never your username, hostname, or machine id. The `RUST_LOG` filter also applies to exported logs, a convenience carried over from the Rust ecosystem. A better UX is in the works. [Details →](https://docs.antigma.ai/configuration/preference#telemetry)

---

**A ghost in your shell.** Ante is a self-contained coding agent that lives in your terminal and self-organizes. One ~15MB Rust binary from [Antigma Labs](https://antigma.ai), zero runtime dependencies, built to get the most out of any model.

It works like Claude Code or Codex, with none of their dependencies or model constraints. It can also be the optimized core for [building your own harness](#one-binary-many-agents) and high-performing assistants.

```sh
curl -fsSL https://ante.run/install.sh | bash
ante
```

Every agent claims to be good. Here are numbers you can check:

### 🥇 Continuously evaled and evolved, in public

Ante runs [Terminal-Bench 2.1](https://antigma.ai/eval) continuously under official leaderboard constraints: 89 tasks, 5 trials each. Each result pins the exact build you can download and links the raw Harbor run for independent audit. Latest full run: **82.7%** with open-weight **DeepSeek V4 Flash 0731** (368/445 trials, Ante 0.preview.71, about $68 of inference). DeepSeek [reports](https://deepseek.ai/blog/deepseek-v4-flash-ga-agent-benchmarks) the same 82.7 for this model, measured with its unreleased DeepSeek Harness in minimal mode.

**[Live results →](https://antigma.ai/eval)** · [Methodology →](https://docs.antigma.ai/benchmarks/eval)

### 🪶 A fraction of the footprint

Ante is hand-written Rust: the heavy parts (`Grep`, `git`) are embedded in one binary and one process, and local inference is handled by a pinned, managed llama.cpp. Across the same 20 parallel tasks in Docker, Ante uses **~7× less peak memory**, **~9× less average CPU**, and **~5× less disk I/O** than Claude Code.

![Resource Usage Comparison](docs-site/docs/benchmarks/compare_animated.gif)

**[Raw numbers →](https://docs.antigma.ai/benchmarks/compare_table)** · [Benchmark details →](https://docs.antigma.ai/benchmarks/eval)

### 🔌 Natively offline

Ante's inference engine is a pinned, managed version of [llama.cpp](https://github.com/ggml-org/llama.cpp). Point it at a GGUF file and the whole loop runs on your machine: no API key, no account, no internet.

```sh
ante --offline-model ~/.ante/models/Qwen3.5-9B-Q4_K_M.gguf \
  -p "add error handling to src/main.rs"
```

We think about the engine layer in public too. [**nanochat-rs**](https://github.com/AntigmaLabs/nanochat-rs) is a small GPT inference core we wrote in pure Rust on [candle](https://github.com/huggingface/candle): readable, runnable, and living in the same process as the code that calls it. It is a study project rather than part of the binary, published because in-process inference is where local models get interesting for agents.

**[Offline mode →](https://docs.antigma.ai/local/offline)** · [nanochat-rs →](https://github.com/AntigmaLabs/nanochat-rs) · [Where this is going →](https://docs.antigma.ai/experimental/agent-native-inference)

---

The three are one design decision. An agent you can **verify**, **afford**, and **run anywhere** is light enough to run by the *thousands*: the substrate for self-organizing intelligence.

## See it in action

<table>
<tr>
<td width="50%">

**[Models, Providers & Thinking](https://docs.antigma.ai/usage/models-and-thinking)**

![Switching provider, model, and effort with /providers](docs-site/static/assets/cookbook/providers.gif)

</td>
<td width="50%">

**[Providing Context: Files & Folders](https://docs.antigma.ai/usage/providing-context)**

![Adding file context with @ mentions](docs-site/static/assets/cookbook/files.gif)

</td>
</tr>
<tr>
<td width="50%">

**[Interrupting & Steering](https://docs.antigma.ai/usage/steering)**

![Interrupting the agent with Escape](docs-site/static/assets/cookbook/interrupt.gif)

</td>
<td width="50%">

**[Subscription Login](https://docs.antigma.ai/usage/login)**

![Connecting to a provider via /connect](docs-site/static/assets/cookbook/connect.gif)

</td>
</tr>
</table>

[See all cookbook guides](https://docs.antigma.ai/usage/providers)

## Quick Start

### Installation

Ante is a single, self-contained binary with no external dependencies: download and run.

```sh
curl -fsSL https://ante.run/install.sh | bash

# Install a specific release channel
curl -fsSL https://ante.run/install.sh | bash -s -- nightly

# Install into a directory already on PATH
curl -fsSL https://ante.run/install.sh | ANTE_INSTALL_DIR=/usr/local/bin bash
```

### Modes

| Mode | Command | Use it for |
|------|---------|------------|
| [Interactive TUI](https://docs.antigma.ai/usage/tui) | `ante` | day-to-day work in the terminal |
| [Headless](https://docs.antigma.ai/usage/headless) | `ante -p "..."` | one-shot tasks, scripts, CI |
| [Server](https://docs.antigma.ai/usage/serve) | `ante serve` | editor plugins and integrations, over a JSONL protocol |
| [Gateway](https://docs.antigma.ai/usage/gateway) | `ante gateway` | running Ante as a Slack or Discord bot |

### Headless examples

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

### Update Ante

```sh
ante update

# One-off update from a different channel
ante update --channel nightly

# Roll back or pin to an exact release
ante update --version v0.preview.81
```

## Beyond the headline numbers

- **Zero vendor lock-in**: bring your own API key, subscription, or local model. Switch between 12+ providers freely. No account required, not even with us.
- **Multi-agent orchestration**: spawn sub-agents and coordinate complex tasks across independent, decentralized, and centralized architectures. [See the patterns →](https://docs.antigma.ai/experimental/agent-org)
- **Channel integrations**: run Ante as a Slack or Discord bot with `ante gateway`.
- **Extensible**: custom skills, sub-agents, MCP, and persistent memory across sessions.

## One binary, many agents

Ante's behavior lives in a settings file, and `--profile <name>` swaps that file per run: system prompt, tool set, skills, memory. The same binary can be a full assistant in one terminal and a minimal agent in the next.

The curated `pi` profile is the extreme case. It strips Ante down to four tools (Read, Write, Edit, Bash) and one short replacement system prompt; file search runs through `rg`, subagents through `ante -p "<task>"`, web access through `curl`. The whole agent fits in one JSON file you can read in a minute:

```sh
cp curated/profiles/pi.settings.json ~/.ante/
ante --profile pi
```

A profile replaces the whole settings file, so anything it omits falls back to Ante defaults, and explicit CLI flags still win. Ante also ships a built-in `bare` profile for stripped-down runs: no skills, MCP servers, session saving, or auto-memory. Share what you build in [`curated/profiles`](curated/profiles).

**[Named profiles →](https://docs.antigma.ai/configuration/preference#named-profiles)** · [Curated profiles →](curated/profiles)

## Supported Providers

Provider support comes in two layers.

**Built-in presets we maintain.** 17 presets, each tested and kept current, so the per-provider quirks are already handled: wire dialect, API key and OAuth flows, thinking and streaming behavior.

| Provider | Example Models |
|----------|---------------|
| Anthropic | Claude Sonnet 5, Opus 5, Fable 5 (API key or subscription OAuth) |
| OpenAI | GPT-5.6 family (API key or ChatGPT/Codex OAuth) |
| Google Gemini | Gemini 3.x family (Gemini API or Vertex AI) |
| Grok (xAI) | Grok 4.5 |
| DeepSeek | DeepSeek V4 |
| Open Router | Any Open Router model, over three wire styles |
| Local (GGUF) | Any GGUF model via built-in llama.cpp |
| ...and more | Zai, Ali Coding Plan, Antix, OpenAI-compatible |

**A config layer for everything else.** Your own proxy, gateway, or inference engine is one entry in `~/.ante/catalog.json`: a `wire_style` (Ante speaks four API dialects), an auth style (bearer, header, or query, from an env var or OAuth), plus `http_headers` and `extra_body` for whatever else the endpoint expects. The combinations cover most setups without a plugin or a code change:

```json
{
  "providers": {
    "my-gateway": {
      "base_url": "https://gateway.example.com/v1",
      "wire_style": "OpenAiCompatible",
      "auth": { "bearer": { "env_key": "MY_GATEWAY_API_KEY" } },
      "http_headers": { "X-Org": "my-team" },
      "extra_body": { "service_tier": "priority" }
    }
  }
}
```

[Providers guide →](https://docs.antigma.ai/usage/providers) · [Catalog Reference →](https://docs.antigma.ai/reference/catalog-reference)

## What's in this repo

We open sourced what really matters in the age of agentic coding, all under Apache 2.0:

1. **Detailed documentation, the descriptive truth.** [`docs-site/`](docs-site) is the source for [docs.antigma.ai](https://docs.antigma.ai): a precise description of what the harness does and how to drive it.
2. **The protocol, the algorithm of the core.** [`crates/protocol-shape`](crates/protocol-shape) defines the schema and wire messages spoken by `ante serve`; [`crates/agent-sdk`](crates/agent-sdk) is the Rust SDK and client for building against agent runtimes.
3. **The eval pipeline, constraint and continuous improvement.** [`ante-harbor/`](ante-harbor) is the Harbor agent adapter behind our Terminal-Bench results: use it to reproduce any run at [antigma.ai/eval](https://antigma.ai/eval). [`CHANGELOG.md`](CHANGELOG.md) records the improvement, release by release.

Alongside these, [`curated/`](curated) is a shared space for reusable pieces from the team and community: settings profiles like [`pi`](curated/profiles), and skills.

The core harness itself is developed in a private repository during the alpha and ships as a prebuilt binary via [releases](https://github.com/AntigmaLabs/ante/releases). Core libraries from it are included here progressively as they stabilize; [`crates/exec`](crates/exec), standalone process execution, is the first. Open-sourcing progress is tracked in [issue #21](https://github.com/AntigmaLabs/ante/issues/21).

The protocol surface maps to Ante's client-daemon architecture:

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

## The bigger picture

> **We care about the harness, not the model or the prompts.**
>
> **Documentation is the new source code.**

Ante is designed for **cellular-native** agents: like cells in an organism, tiny, expendable, massively replicated. That thesis is why the three headline claims exist. A cell-scale agent must be *verified* (reliability compounds at scale), *tiny* (every byte is multiplied by thousands), and *self-contained* (no runtime to install, no service to phone home to). Read more in our [philosophy](https://docs.antigma.ai/start/philosophy) and [agent organization patterns](https://docs.antigma.ai/experimental/agent-org).

## FAQ

### Why another terminal agent?

The name is the answer: **An**other **Te**rminal agent, and *ante*, the stake you put on the table to play. Ante is fast, lightweight, and the only terminal agent with native local inference built in. We believe a self-contained agent core that self-organizes is the foundation of the coming agent economy.

<details>
<summary><b>How is Ante different from other agents?</b></summary>

Ante has most of the features you expect from agents like Claude Code or Codex: multi-agents, skills, MCP, persistent memory. The difference is the build philosophy.

- Built from scratch in Rust. Core components like `Grep` (fully rebuilt and customized) and `git` are embedded in the same ~15MB binary and run in the same process at runtime, so nothing is shelled out and no resources leak. Most similar projects ship on Node.js or CPython and carry an order-of-magnitude larger footprint.
- Local inference is built in: the engine is a pinned, managed version of [llama.cpp](https://github.com/ggml-org/llama.cpp), so a local GGUF model is all Ante needs to run without any provider. We also share our own work on the engine layer: [nanochat-rs](https://github.com/AntigmaLabs/nanochat-rs), a small in-process inference core in pure Rust.
- No vendor lock-in, not even to ourselves: no account needed, reuse your existing API credentials. An opt-in, fully integrated server-side experience lives at [antix.antigma.ai](https://antix.antigma.ai).
- Every claim is backed by public, reproducible benchmarks of the exact builds we ship: [antigma.ai/eval](https://antigma.ai/eval).

Beyond the footprint it comes down to agent architecture, and ultimately to *who* is building it and with what philosophy. Anyone can fork a binary; taste and engineering rigor don't copy. Those differences leak into every detail of the product.

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

## License

Source code in this repository (including the SDK and protocol crates) is
licensed under the [Apache License 2.0](LICENSE).

The prebuilt `ante` binary is free to use — including commercially — during
the alpha preview under the [Binary Preview Terms](BINARY-TERMS.md). The core
harness is currently developed in a private repository and shipped as a
binary; the SDK and protocol surface you build against here will remain
permissively licensed.
