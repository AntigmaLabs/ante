# Harbor Test Workflow

This workflow runs Harbor tests against the Ante agent using configurable parameters.

## Default Command

With default parameters, the workflow executes:

```bash
uv run harbor run \
  --agent ante_agent:AnteAgent \
  --model "gemini-3-pro-preview" \
  --ak provider=gemini \
  --ak install_args= \
  --ae 'GEMINI_API_KEY=${GEMINI_API_KEY}' \
  --ae 'MODEL_BASE_URL=${MODEL_BASE_URL}' \
  --ae 'ANTE_ENV=${ANTE_ENV}' \
  --dataset terminal-bench@2.0 \
  --n-attempts 5
```

This directory contains the Ante agent adapter (`ante_agent.py` and
`ante_events.py`), which the workflow puts on `PYTHONPATH` for the harbor
run. Ante itself is installed inside each sandbox from the published
install.sh (`https://download.ante.run/install.sh`) using `install_args`.

## Workflow Parameters

### `install_args`
- **Description**: install.sh args selecting the Ante build to test (e.g. `stable`, `nightly`, a version)
- **Default**: `` (empty — install.sh's default build)
- **Required**: No
- **Usage**: Passed to the published install.sh inside the sandbox to select which Ante build to install.

### `harbor_args`
- **Description**: Arguments to pass to harbor
- **Default**: `--dataset terminal-bench@2.0 --n-attempts 5`
- **Required**: Yes
- **Usage**: Custom Harbor CLI arguments. Common options include:
  - `--dataset <dataset>`: Specify the test dataset (e.g., `terminal-bench@2.0`)
  - `--n-attempts <number>`: Number of attempts per task
  - `--max-concurrent <number>`: Maximum concurrent tasks

### `model_name`
- **Description**: Model name to use
- **Default**: `gemini-3-pro-preview`
- **Required**: No
- **Usage**: The model identifier passed to Harbor and forwarded to `ante --model`.

### `provider`
- **Description**: Model provider (anthropic, openai, gemini)
- **Default**: `gemini`
- **Required**: No
- **Usage**: Passed to `ante --provider`, and selects which API key is forwarded into the sandbox:
  - `anthropic`: `ANTHROPIC_API_KEY`
  - `openai`: `OPENAI_API_KEY`
  - `gemini`: `GEMINI_API_KEY`

### `effort`
- **Description**: Reasoning effort passed to `ante --effort`
- **Default**: `` (empty — Ante's catalog default)
- **Required**: No

### `model_base_url`
- **Description**: Base URL for model API
- **Default**: `https://generativelanguage.googleapis.com/v1beta`
- **Required**: No
- **Usage**: The base URL for the model API endpoint. Used when routing through a proxy or custom endpoint.

### `model_temperature`
- **Description**: Model temperature
- **Default**: `` (empty)
- **Required**: No
- **Usage**: Controls randomness in model outputs (0.0 to 2.0). Lower values make outputs more deterministic. If empty, the model's default temperature is used.

### `model_max_tokens`
- **Description**: Max tokens for model
- **Default**: `` (empty)
- **Required**: No
- **Usage**: Maximum number of tokens in the model's response. If empty, the model's default is used.

### `runs_on`
- **Description**: Runner to use for the test job
- **Default**: `gcp-hosted`
- **Required**: No
- **Usage**: GitHub Actions runner label. Examples:
  - `gcp-hosted`: GCP-hosted runner
  - `ubuntu-22.04-16core`: Self-hosted runner with 16 cores
  - `ubuntu-latest`: GitHub-hosted runner

### `run_count`
- **Description**: Number of times to run Harbor tests (each run is a separate job)
- **Default**: `1`
- **Required**: No
- **Usage**: Number of independent test runs to execute. Each run is a separate job. The workflow is configured with a 120-hour timeout (`timeout-minutes: 7200`), but note that GitHub-hosted runners have a hard 6-hour limit regardless of the configured timeout. Self-hosted runners can utilize the full 120-hour timeout. Runs execute sequentially (`max-parallel: 1`). Useful for long-running tests that exceed a single job's time limit on GitHub-hosted runners.

## Execution Details

- **Job Timeout**: Each test job has a 120-hour timeout configured (`timeout-minutes: 7200`). **Important**: GitHub-hosted runners have a hard 6-hour limit regardless of the configured timeout value. Self-hosted runners can utilize the full 120-hour timeout.
- **Execution Strategy**: Sequential execution (`max-parallel: 1`) - runs execute one after another
- **Artifacts**: Each run uploads its results as a separate artifact named `ante-harbor-results-<run_id>-run-<index>`
- **Artifact Retention**: 30 days


## Example Usage

### Basic test with defaults
```
install_args: stable
harbor_args: --dataset terminal-bench@2.0 --n-attempts 5
model_name: gemini-3-pro-preview
```

### Multiple runs for long tests
```
run_count: 5
harbor_args: --dataset terminal-bench@2.0 --n-attempts 5
```
This creates 5 separate jobs, each configured with a 120-hour timeout. Note: On GitHub-hosted runners, each job is limited to 6 hours regardless of the configured timeout. Self-hosted runners can use the full 120-hour timeout. Jobs run sequentially.
