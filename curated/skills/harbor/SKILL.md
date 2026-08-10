---
name: harbor
description: >-
  Run Terminal-Bench or any Harbor dataset with Ante as the agent, using the
  Harbor adapter in the ante repo's ante-harbor/ directory. Use when the user
  wants to benchmark Ante on Harbor or reproduce Ante's published eval results.
metadata:
  argument-hint: "<provider> <model> [task ...]"
---

# Run Harbor with Ante

Run [Harbor](https://github.com/laude-institute/harbor) with Ante as the agent. The adapter (`ante_agent.py`) lives in the `ante-harbor/` directory of the [ante repo](https://github.com/AntigmaLabs/ante). Harbor imports it and installs Ante inside each task sandbox from the published install script.

## Prerequisites

- A checkout of [AntigmaLabs/ante](https://github.com/AntigmaLabs/ante), for `ante-harbor/`
- [uv](https://docs.astral.sh/uv/) with Python 3.12
- Docker running: Harbor executes each task in a container
- The provider API key exported in the shell

## Run

Resolve the model and provider from the user's request. From the repo's `ante-harbor/` directory (so `ante_agent:AnteAgent` is importable):

```bash
uv run --python 3.12 --with harbor harbor run \
  --agent ante_agent:AnteAgent \
  --model "<model_name>" \
  --ak provider=anthropic \
  --ak install_args= \
  --ae 'ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}' \
  --dataset terminal-bench/terminal-bench-2-1 \
  --n-attempts 1
```

Adjust for the request:

- `provider` selects which key Ante reads inside the sandbox. For `openai` or `gemini`, forward `OPENAI_API_KEY` or `GEMINI_API_KEY` with `--ae` instead.
- `install_args` picks the Ante build installed in each sandbox: empty for the latest release, or a version to pin.
- Scope: add `-i <task-id>` (repeatable) to run specific tasks. Smoke-test one task before a full run unless the user asks otherwise.
- Throughput: `--n-concurrent <n>` caps parallel sandboxes. `--n-attempts <n>` sets attempts per task; published leaderboard runs use 5.
- Custom endpoint: add `--ae 'MODEL_BASE_URL=${MODEL_BASE_URL}'` when routing through a proxy.

## Read results

Harbor prints a per-task summary and writes a run directory with per-trial output; each trial captures Ante's raw event log from `/logs/agent/ante.txt`. Task failures do not make `harbor` exit non-zero, so judge the run by the summary, not the exit code.
