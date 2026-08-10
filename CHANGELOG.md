# Changelog

## v0.preview.73 - 2026-08-10

### Added
- Coming from Claude Code or Codex: when a session in this directory was active within the last 4 hours, startup offers to pick it up. `/resume-claude` and `/resume-codex` are bundled skills that locate the foreign transcript, read it as data, and reconstruct task, progress, and next step before confirming with you. The hint appears once per project and never in headless runs.
- `/import-claude` copies Claude Code's project memory for this directory into Ante's project memory. Existing Ante files are never overwritten, and re-running is a clean no-op.
- `settings.json` accepts `system_prompt`, `append_system_prompt`, and `tools` as fresh-session defaults, plus `auto_memory`, `skills`, and `session_save` with matching CLI overrides. Explicit CLI flags and wire values stay authoritative, and persisted custom prompt text is redacted from `ante rage` bundles.

### Changed
- Bump the bundled llama.cpp engine to b10217
- The built-in `bare` profile is now a real `bare.settings.json` file, seeded once and editable like any other profile

### Fixed
- Headless runs exit cleanly when their output consumer closes stdout or stderr, instead of panicking with a broken pipe

## v0.preview.72 - 2026-08-09

### Added
- Onboarding "Use API key" now takes a pasted key directly: masked input with provider auto-detect from the key prefix, validated against the provider, and stored owner-only under `~/.ante/auth` (env vars still take precedence)
- On the API-key step with Anthropic selected, `Tab` signs in to the Anthropic Console in the browser and provisions an API key automatically — the key never touches the clipboard
- Compaction results are now visible: a collapsed `* Compacted` marker appears in the conversation (manual and auto compaction), with the full summary in the ctrl+o transcript view; `CompactEnd` now carries the summary text on the wire, and trimmed oversized tool results are reported with an info line

### Changed
- Improved Bash command execution
- Antix OAuth is listed ahead of OpenAI in `/connect` and provider auto-detection

### Fixed
- A fresh install with no credentials no longer lands in a session silently wired to the unreachable built-in `localhost:8080` fallback — the not-connected state points at `/connect`, and a successful sign-in restarts the session on the newly connected provider
- Standalone `ante update` no longer panics with a broken pipe when its launcher closes stdout, and a closed pipe can no longer cancel an update

## v0.preview.71 - 2026-08-07

### Added
- `ante doctor` checks that the Ante home directory (`~/.ante`) is writable

### Changed
- Dependency updates

### Fixed
- Cancelling a running Bash tool call now kills its whole process group, so grandchild processes no longer keep running after Ante reports execution stopped
- Streaming model calls that produce no usable output within 5 minutes fail with a timeout instead of hanging the turn indefinitely
- The installer refuses `sudo` installs that would leave `~/.ante` root-owned, and startup shows a visible notice when the Ante home is unwritable
- Proxy `auth_unavailable` credential failures are classified as terminal auth errors instead of consuming the reconnect budget
- Parameterless tool schemas keep an explicit empty `properties` object on OpenAI-compatible requests, fixing rejections from strict endpoints

## v0.preview.70 - 2026-08-03

### Added
- `--profile <name>`: named settings profiles as whole-file replacement settings files (`<name>.settings.json`), plus the `ANTE_PROFILE` env var and a built-in file-less `bare` profile
- Bare `/term` opens a terminal picker — list, attach/detach, kill, and create sessions

### Changed
- Updates now ride the normal TUI flow: when a new version is available an in-chat notice shows its release notes, and `/update` installs it on exit (`/update skip` dismisses that version, `/update cancel` unschedules); the blocking startup modal is gone, and running sessions now check for updates hourly
- Use DeepSeek V4 Flash 0731 on OpenRouter
- Nightly builds log at debug level by default
- SKILL.md reads are labeled as skill loads in the transcript

### Fixed
- `ante catalog` exits cleanly instead of panicking when its output pipe closes early
- Brighter agent response text; thinking text pinned to foreground gray
- Hidden hardware cursor parked at the composer so OS IME popups appear at the caret
- Z.ai five-hour usage-limit errors (code 1308) are classified as terminal quota instead of being retried

## v0.preview.69 - 2026-08-02

### Added
- Shift+Enter and Ctrl+Enter insert a newline in the composer on terminals with kitty keyboard protocol support (kitty, ghostty, WezTerm, iTerm2 3.5+, Alacritty 0.13+)

### Changed
- Diff View V2 across all diff surfaces — chat Edit/Write details, the `/diff` pager, the approval Tab preview, and the theme dialog: numbered line gutter, full-width added/removed background bands, and `+N -M` change counts inline in headers

### Fixed
- Normalize MCP tool schemas to the portable provider subset at ingestion, so servers using advanced JSON Schema keywords (`exclusiveMinimum`, `$ref`, `oneOf`, …) no longer get requests rejected by strict providers such as Gemini
- Merge system prompt fragments into a single leading system message for unrecognized OpenAI-compatible models, fixing strict local chat templates that error on multiple system messages
- Spinner token count now shows only tokens generated this turn (it previously re-added the full context every generation, inflating wildly), with a ↑/↓ request/stream phase arrow
- Gemini quota errors that mention token limits are classified as quota, not context overflow, so they no longer trigger futile compaction

## v0.preview.68 - 2026-07-31

### Added
- `/term <name> [args...]` launches the named binary in the fresh session — `/term claude --continue` opens a split already running claude; existing sessions still just attach
- `terminals` status-line chip (on by default) showing live `ante-*` tmux sessions by name
- `--no-skills` run flag: skip skill discovery entirely (nothing advertised in the system prompt or dispatchable as commands); `/resume` preserves the choice

### Changed
- Repeated `/term` calls stack viewer splits in one right-hand column, replacing the composed `ante-view-*` viewer — every agent keeps a full-width column and Ante keeps its size
- Two-row footer: identity items (model, dir, branch, …) on top, permission mode and activity chips beneath
- Default DeepSeek V4 Flash to max reasoning effort on all routes (Antix, direct, OpenRouter)
- Dependency updates

### Fixed
- Fully typed commands that take arguments (e.g. bare `/term`) no longer swallow the first Enter

## v0.preview.67 - 2026-07-29

### Added
- `/term <a> <b>` composes a side-by-side viewer over several sessions in one window — watch or compare two terminal agents at once
- Bundled `tmux` skill: session reuse, interactive-program input, and driving other CLI agents such as Claude Code or Codex
- Discover skills from user-level `~/.claude/skills`, and highlight recognized commands in the composer
- `@`-mention completion for `~/`, absolute, `../`, and `./` paths
- Delete sessions from the `/resume` picker

### Changed
- Replace `/pty` with `/term`: the agent drives durable named tmux sessions through ordinary Bash (namespaced `ante-*`), and `/term <name>` opens a native terminal split/window to watch or type; sessions survive Ante restarts
- No subprocesses or file writes before the TUI's first frame (faster, quieter startup)
- Bump the bundled llama.cpp engine to b10107

### Fixed
- Redact the OAuth callback query from logs
- Parse model `weight_class` values case-insensitively

## v0.preview.66 - 2026-07-26

### Added
- Per-category context-window usage reporting over the protocol (`Op::ContextReport`)

### Changed
- Speed up Grep further by parallelizing the directory walk and search (large-corpus searches 40–70% faster)
- Update and prune the built-in model catalogs

### Fixed
- Don't abort OAuth login when stdin closes

## v0.preview.65 - 2026-07-25

### Added
- Add a `/pty` window that attaches a native terminal side panel
- Render LaTeX math in the TUI via a delimiter normalizer and TeX→Unicode converter
- Add an auto compact setting

### Changed
- `--tools` now sets the base tool set and `--include-tools` adds on top of it
- Serialize concurrent Edit/Write calls that target the same file instead of only warning
- Speed up Grep by dropping per-file parallel fan-out (~2× faster)
- Use catalog keys as provider IDs
- Cap skill catalog descriptions rendered into the system prompt
- Extend short-prompt coverage to the system prompt and more tools
- Harden release publication identity and atomicity

## v0.preview.64 - 2026-07-24

### Added
- Add an Antix auth login command

### Changed
- Wrap tool header lines at the viewport edge instead of clipping them
- Dependency updates

### Fixed
- Detect when iTerm2 blocks scrollback clearing: purge by default when verified safe, show a notice when blocked
- Exit the TUI cleanly on broken stdout pipes
- Omit absent optional arguments from tool call displays

## v0.preview.63 - 2026-07-21

### Added
- Add Tencent Hy3 to the OpenRouter model catalog and pricing table
- Include step count in turn end events

### Changed
- Markdown render improved by 93% for long streams
- Remove Qwen 3.7 Max/Plus from the OpenRouter catalog (endpoints no longer accessible); they remain available via Antix
- Render markdown lists directly instead of via tui-markdown
- Inset all TUI rows two columns from the window edge
- Consolidate the ante-guide into a bundled, model-only skill
- Warn when concurrent Edit/Write mutations target the same file path
- Prune older built-in model catalog entries

### Fixed
- Reject tool calls with malformed arguments before execution and return the parse error so the model can retry
- Handle tool calls truncated at the output-token limit gracefully across all providers, continuing the turn instead of surfacing a hard error
- Drop malformed ambient replies instead of leaking model reasoning

## v0.preview.61 - 2026-07-19

### Added
- Client-side PDF reads: page rendering, text extraction, and page bounding
- CLI option to load a system prompt from a file
- WebFetch spills oversized responses to a cache file instead of truncating

### Changed
- Corrective tool errors and no line clipping in Read
- Lenient tool path resolution and normalized permission matching
- TUI rendering performance and composer text contrast improvements

### Fixed
- Fall back to a usable provider instead of localhost for unknown provider ids
- Bound PDF render dimensions against hostile input

## v0.preview.60 - 2026-07-17

- Reconnect and resume when an LLM stream dies mid-response — even before any answer text has arrived — instead of ending the turn with an error
- Fail streams that end without a proper stop signal as retryable instead of hanging the turn
- Honor server `Retry-After` hints, rebalance retry budgets toward the visible reconnect layer, and log every retry decision
- Add Kimi K3 support to Antix and the OpenRouter model catalog
- Add Meta Muse Spark 1.1 to the OpenRouter model catalog
- Add a `short_prompt` toggle to the `/config` dialog
- Show effort next to the model in the default status line order
- Scroll the status line picker on short terminals instead of clipping items, and hint that Claude Code-compatible `status_line_command` scripts are supported
- Exit cleanly when stdout closes mid-output (e.g. when piping into `head`)

## v0.preview.59 - 2026-07-16

- Add a `--no-session-save` CLI flag that skips session event logs and resumable snapshots for TUI and headless sessions, and propagates to subagents
- Add a `--short-prompt` option (CLI flag and `settings.json`) that uses compact tool descriptions to shrink the system prompt
- Add xAI Grok 4.5 support to Antix
- Support GPT-5.6 on OpenAI subscriptions
- Dependency updates

## v0.preview.58 - 2026-07-15

- Add a `/config` settings dialog to the TUI
- Fall back to `CLAUDE.md` for project instructions when no `AGENTS.md` exists
- Fix persisted model effort restoration and ambient settings drift
- Preserve parse error details when loading user model catalogs
- Render agent output text in the theme foreground color
- Improve session shutdown reliability by cancelling and draining background session tasks before the session ends
- Unify TUI state ownership across modals, pickers, the status line, and terminal size to prevent stale rendering after resizes and modal transitions
- Update the pinned offline `llama.cpp` engine to build b9986
- Remove a stale xAI model catalog alias
- Refresh the mascot logo pixel art

## v0.preview.57 - 2026-07-12

- Persist Shift+Tab-selected permission modes as the default for new sessions while keeping Yolo session-only
- Add curated contextual tips beneath the working spinner, with cooldowns and an opt-out setting
- Fix turns that could hang after the model completed by stopping stream collection at `MessageStop`
- Improve LLM error recovery with structured error kinds, actionable `/connect` guidance for expired OAuth, bare 403 classification, and FastAPI detail parsing
- Unify theme picker state and open paths to prevent stale selections and conflicting modal transitions
- Improve Harbor eval correctness and efficiency with accurate parallel tool-call grouping, persisted effective flags, single-turn ad hoc runs, native result rendering, and batched archive reads
- Fix and centralize the production telemetry contract

## v0.preview.56 - 2026-07-10

- Unify model thinking controls as a six-level Effort scale across providers, the CLI, sessions, and eval reporting
- Redesign `/providers` and `/models` with provider-scoped model lists, an effort slider, two-step provider switching, and remembered model preferences
- Add GPT-5.6 Sol, Terra, and Luna to the OpenAI API and OpenRouter Responses catalogs
- Wait for loading llama-compatible servers when attaching offline mode instead of reporting them as unavailable
- Serialize session snapshot writes so older state cannot overwrite newer state
- Dependency updates

## v0.preview.55 - 2026-07-09

- Add offline-mode attachment to existing local llama-compatible servers, with broader localhost detection, `/v1/models` validation, retryable attach errors, UTF-8-safe input, and corrected external-server state handling
- Add Antix Claude Fable 5 support
- Add Grok 4.5 to the xAI and OpenRouter model catalogs
- Fix OAuth provider login started from `/providers` so the hidden picker no longer captures input or makes the TUI appear frozen
- Fix Kimi thinking request compatibility by omitting unsupported `thinking.type = "enabled"` while preserving explicit disabled requests
- Improve chat composer wrapping so English words wrap at boundaries while long tokens, CJK text, cursor movement, and resizes stay aligned
- Normalize ambient thinking phrases to sentence case instead of shouting in all caps
- Compact sessions before the next turn starts to reduce turn-lifecycle and context-management edge cases

## v0.preview.54 - 2026-07-06

- Add OpenRouter web search request support and forward `top_k` in OpenAI-compatible requests
- Update built-in model catalogs
- Fix multibyte truncation in approval previews
- Improve settings and credential storage reliability with race-safe ownership and centralized atomic writes
- Scope permission grants at the permission layer for clearer approval behavior
- Polish TUI sent-message and composer cursor rendering
- Clean up the external protocol shape by removing non-wire behavior and decoupling internal storage/logging details

## v0.preview.53 - 2026-07-04

- Add `/goal` for goal-driven sessions that keep working until a condition is met, cleared, interrupted, or judged unreachable
- Improve TUI resize handling to prevent scrollback duplication and content loss, with an opt-in `resize_reflow = "purge"` setting for terminals that can safely purge and replay
- Install official pinned `llama.cpp` prebuilts with SHA-256 verification, atomic versioned installs, GPU-tier selection, and mirror fallback for offline engine setup
- Improve installer feedback and reliability
- Refine default prompts for task-contract validation and split Ante product guidance from working defaults

## v0.preview.52 - 2026-07-02

- Fix skill slash command injection
- Warn when deprecated tool filter aliases are used
- Broadly shrink and streamline default prompts and tool descriptions across the agent, skills, OAuth, and TodoWrite flows as model capabilities improve
- Polish TUI styling for composer input, status line, agent bullets, and sent-message bars
- Dependency updates

## v0.preview.51 - 2026-07-01

- Add Claude Fable 5 routes to the model catalog
- Remove simple agent mode and its dedicated prompt/config path

## v0.preview.48 - 2026-07-01

- Add an orange grid style for the composer and sent user messages
- Automatically reconnect on transient mid-stream errors, with consolidated provider error parsing
- Rework the session into a self-driving actor with a fan-in mailbox, and fix follow-up turn-lifecycle regressions so exactly one end-of-turn event fires per turn
- Improve offline CUDA detection and local-server diagnostics
- Read the Zai provider API key from `ZAI_API_KEY`
- Fix a UTF-8 truncation bug in the ambient prompt

## v0.preview.46 - 2026-06-28

- Predict a short, task-specific spinner phrase while you type a longer prompt and show it as the spinner label for that turn (best-effort, runs on the cheapest model, off the critical path)
- Suggest a likely next prompt as dim ghost text after a turn ends, acceptable with Tab
- Keep destructive or alarming words out of the predicted spinner phrase
- Apply a permission-mode change (Shift+Tab) to the in-flight turn, so switching to Auto mid-turn takes effect immediately instead of on the next turn
- Make MCP and dynamically registered tools obey tool filtering, fixing a bypass where they ignored `--allowed-tools`/`--disallowed-tools`; rename the flags to `--include-tools`/`--exclude-tools` with the old names kept as hidden aliases
- Send native web search to the model by default on web-search-capable providers
- Add `gpt-5.4` and `gpt-5.4-mini` to the OpenAI subscription models
- Apply `~/.ante/catalog.json` as partial provider overlays, patching existing providers field-by-field instead of overwriting them
- Make skill frontmatter parsing lenient, recovering from common unquoted colons in `description:` values

## v0.preview.45 - 2026-06-25

- Add MiMo models via OpenRouter
- Select flash and default models automatically by weight class (feather/middle/heavy)
- Infer model vision support from model markers, and return image metadata from Read when the model lacks vision
- Make native web search a declarative provider flag
- Rework bash mode to run in the user's shell and reuse the shared crates/exec path
- Surface settings parse notices in `ante doctor`
- Dependency updates

## v0.preview.44 - 2026-06-22

- Add readline-style line editing with undo/redo to the chat composer
- Add bash mode (!cmd) for running shell commands inline
- Make the status line Claude Code-compatible, show the raw context window, and refresh its defaults
- Lay groundwork for offline mode with a self-contained TUI
- Make ante-guide read docs from the ante-preview repo checkout
- Improve auto-memory tool labels in the TUI
- Handle malformed tool-call arguments cleanly
- Read the subagent report from the last model message
- Allow up to 3 max-token continuations per episode
- Charge model-call usage once per step
- Restructure the turn model-step loop for readability

## v0.preview.43 - 2026-06-20

- Add an opt-in auto-memory prompt that teaches the agent to record and recall typed memories across sessions
- Add OpenRouter GLM 5.2 to the model catalog
- Clarify max-output-token truncation handling
- Preserve partially streamed content when a mid-stream error occurs
- Validate model token budgets when loading the catalog
- Make tool approval bookkeeping non-fatal
- Simplify OpenAI-compatible profile policies
- Tighten the headless check prompt
- Detect updater installer download failures

## v0.preview.42 - 2026-06-18

- Add an `ante doctor` command and speed up startup by decoupling TUI session start
- Remove the global Ctrl-D TUI exit shortcut
- Show the Shift+Tab cycle hint in the custom status-line footer
- Guard background bash commands against trailing ampersands
- Clean up the offline installer config and release build workflows
- Dependency updates

## v0.preview.41 - 2026-06-17

- Rename persisted permission settings to the Claude-style `permissions` key and drop the legacy `permission_settings` alias
- Scope permission rules to known primary tool args (Bash, Agent, Read, Edit) and render TUI tool-call args as the primary value when known

## v0.preview.40 - 2026-06-16

- Add scoped session permission grants with a subsumption algebra, collapsing batched approval prompts by re-evaluating them against already-granted scopes
- Cycle permission modes with Shift+Tab
- Rename the `Default` permission mode to `Strict` and drive permission mode through settings and the protocol
- Rework turn interrupt and steer keys: Ctrl-C interrupts, Ctrl-S steers, and Esc is contextual and also interrupts an active turn
- Keep the active turn alive across a session or model update
- Show a live "still working" timer for quiet long-running tools
- Report context usage as percent used in the status line
- Cap compaction output by the remaining context window
- Harden the safe-command classifier for `date`, `file`, `tree`, and abbreviated git options
- Fix conservative model token budgets
- Remove unsupported OpenAI subscription models
- Remove the leading slash glyph from the empty input prompt

## v0.preview.39 - 2026-06-14

- Add layered permission configuration with an Auto mode and hardened settings handling
- Stop wildcard permission rules from matching across sequence stages
- Add a context-window-left metric to the TUI status line
- Surface a startup notice when settings fail to parse instead of silently ignoring them
- Add new antix OAuth models
- Update the model catalog for Anthropic, OpenRouter, and ZAI, and add MiniMax M3 on OpenRouter
- Stop orphaned subagent turns when a turn is interrupted
- Classify in-stream OpenAI-compatible error events instead of failing opaquely
- Improve OpenRouter error detail handling
- Fix OpenRouter DeepSeek max thinking effort

## v0.preview.38 - 2026-06-12

- Always stream responses end to end and retire the session streaming flag; suppress raw deltas in headless output
- Harden Bash safety classifiers
- Add a command-based status line to the TUI (Claude Code-compatible)
- Suggest a starter prompt and highlight slash commands on a fresh session
- Handle terminal stop reasons before retrying empty responses
- Fix OAuth refresh token response parsing
- Harden WebFetch against binary responses
- Fail fast in headless mode when the daemon dies, and explain the stdin EOF wait
- Extend the typed LLM error taxonomy through streaming and provider error classification
- Rework the session/turn lifecycle with explicit states and symmetric exits
- Dependency updates

## v0.preview.37 - 2026-06-10

- Update the Anthropic model catalog for the latest Claude models
- Introduce a typed LLM error taxonomy with per-kind recovery hints in error messages
- Reconnect dropped streams mid-turn and treat cancellation as a first-class turn outcome, including while a stream is being opened
- Surface content-filter stops like output-limit truncation instead of failing silently
- Fix Gemini streaming: emit thought text and signature as one thinking delta, resolve tool-response names from the dialog's tool calls, count thinking tokens in output usage, and classify recitation/blocklist stops
- Harden OpenAI-compatible streaming: accumulate tool-call deltas split across chunks, flush buffered tool calls on premature stream EOF, isolate malformed tool-call arguments, and keep the system prompt when no user message exists
- Forward `max_output_tokens` and `temperature` to OpenAI and preserve truncated output
- Propagate Anthropic message conversion errors instead of dropping messages
- Honor the HTTP-date form of `Retry-After` headers when rate limited

## v0.preview.35 - 2026-06-08

- Add OpenRouter provider profiles
- Show a sign-off message and bug-report hint when exiting the TUI
- Set the terminal window title to "Ante"
- Handle redacted thinking blocks from the latest Claude models
- Prevent the Bash tool from inheriting stdin
- Fix token usage accounting for Anthropic and OpenAI-compatible streaming

## v0.preview.34 - 2026-06-06

- Add OpenAI-compatible provider profiles
- Surface subagent activity as live tool updates instead of separate turn events
- Recover from transient API decode failures instead of crashing the run
- Allow `ANTE_INSTALL_DIR` to override the install location and harden the install script
- Unify the LLM streaming driver across providers for consistent streaming behavior
- Dependency updates

## v0.preview.33 - 2026-06-04

- Add `ante update --version <V>` to pin or roll back to a specific release
- Retire the legacy `latest` update channel and transparently resolve it to `stable`
- Drive vision/image support from model metadata
- Reduce Read and multiline Grep latency
- Fix `@`-mention handling: dedupe repeats, honor `\@` escapes after multibyte characters, and stop dropping large mentioned files

## v0.preview.32 - 2026-06-04

- Add `ante catalog` command to print the merged model catalog as JSON
- Show structured turn errors instead of a raw debug dump
- Recover from transient connection resets instead of failing the run
- Fix Anthropic 400 error from unsigned thinking blocks
- Fix stale OAuth credential cache
- Migrate the Grep tool to a streaming ripgrep-style search engine

## v0.preview.31 - 2026-06-02

- Wrap markdown table content in narrow TUI views
- Fix approval prompt wrapping
- Use bundled webpki TLS roots for all HTTP clients
- Use a blocking HTTP client for OTLP telemetry exporters
- Speed up file searches by pruning VCS directories during traversal

## v0.preview.30 - 2026-05-31

- Add `ante rage` command to bundle a bug report
- Persist tool approvals via "always allow" and store allow/ask/deny rules in settings.json
- Let Edit create a new file via an empty `old_string`
- Suggest a similar path when Edit targets a missing file
- Handle CRLF files correctly in Read/Edit
- Harden Edit/Write filesystem guards
- Allow arbitrary model ids for explicit providers
- Improve OpenRouter provider defaults
- Improve responsiveness of grep/glob searches
- Fix character-based output truncation
- Fix image decode limits
- Dependency updates

## v0.preview.29 - 2026-05-28

- Add Claude Opus 4.7/4.8 and GPT-5.5-pro to the model catalog
- Drop the retired Gemini 3.1 Flash Lite preview model
- Add user model overrides to customize built-in model specs
- Handle malformed user model config entries leniently
- Use explicit OPENAI_COMPATIBLE_API_KEY for OpenAI-compatible providers
- Fix persisting of empty sessions
- Fix status bar truncation and clipping of overflowing hyperlinks
- Dependency updates

## v0.preview.28 - 2026-05-21

- Support global `~/.ante/AGENTS.md` alongside project AGENTS.md
- Update OpenAI model catalog and provider selector fallback
- Add generic LLM model listing across providers
- Re-enable antix smoke test in release workflow

## v0.preview.27 - 2026-05-19

- Add OpenAI Responses WebSocket transport (opt-in)
- Show OpenAI transport in debug panel info tab
- Improve tool call failure logging
- Optimize directory tree traversal
- Tighten OpenAI Response API streaming
- Normalize blank Grep file type
- Dependency updates

## v0.preview.26 - 2026-05-13

- Animate the `/compact` info block header while compaction runs
- Show installer download progress
- Use CDN URLs in release manifests
- Include provider metadata in session events

## v0.preview.25 - 2026-05-13

- Add /compact slash command
- Recover from output-token-limit truncation: keep streamed text and show a hint to send "continue"
- Auto-compact and retry once when OpenAI requests exceed the context window
- Fix pager and resume overlays not resizing with the terminal

## v0.preview.24 - 2026-05-10

- Fix OpenAI subscription streaming requests
- Fix Unicode clipping in diff viewer
- Persist update channel overrides, including on install failure

## v0.preview.23 - 2026-05-08

- Paste images from clipboard with Ctrl+V
- Add update channel override
- Log panic crash reports
- Refactor release artifact publishing and smoke tests
- Refine Dependabot dependency grouping
- Dependency updates

## v0.preview.22 - 2026-05-06

- Add nightly release channel
- Split stable and latest release channels
- Fix OpenRouter streaming for thinking (reasoning) parts

## v0.preview.21 - 2026-05-06

- Add TUI provider selector
- Simplify model selector
- Add DeepSeek support for OpenRouter
- Add random logo variants on startup

## v0.preview.19 - 2026-05-04

- Improve DeepSeek support
- Lazy MCP tool registration so daemon doesn't block on warm-up
- Render MCP tool output as readable text
- Let background bash survive parent exit
- Fix public sync messages derivation from tracked paths
- Fix duplicate auth in public sync
- Dependency updates

## v0.preview.18 - 2026-05-02

- Add MCP (Model Context Protocol) support
- Add browser features
- Replace BashOutput/KillShell tools with status file
- Differentiate Bash foreground and background output
- Add explicit bash background flag
- Unwrap nested bash -lc wrappers before exec and rule matching
- Preserve bash output head and tail with mid-omission marker
- Restore Windows WSL skip and trim bash preview hot path
- Refactor shell detection handling
- Move bash tests to integration suite with isolated shell
- Refine Bash tool description
- Avoid duplicate shell tool updates

## v0.preview.17 - 2026-05-01

- Add Windows compatibility
- Add provider-specific base URL env vars
- Add extra llamacpp args
- Update offline models
- Optimize dialog clone storage
- Trim ToolEnd shims and dedupe assistant-part emission
- Wire runtime protocol to shape types and prune protocol shape crate
- Collapse and tighten protocol helper call sites
- Fix DeepSeek-v4 interruption bug
- Fix empty message deletion on interrupt
- Fix small issues uncovered by DeepSeek testing
- Fix thinking correspondence
- Dependency updates

## v0.preview.16 - 2026-04-26

- Add deepseek-4 model support
- Update OpenAI and Gemini model presets
- Split Antix API-key and subscription providers
- Derive OAuth providers from catalog
- Make local provider the default
- Show and preserve current provider in model selector
- Fix provider fallback resolution
- Fix sync handling for deleted mapped paths

## v0.preview.15 - 2026-04-23

- Enable vision for local GGUF models and refresh offline model catalog
- Fix yolo resume bug
- Support nested skill metadata
- Add read-only bash permission heuristic
- Align headless startup provider handling
- Move message ID generation into OpMsg/EventMsg constructors
- Consolidate llm_smoke around session-based tool-call path
- Split antix into its own catalog module
- Harden release workflow reproducibility and failure recovery
- Move thinking option labels into TUI
- Update connect and model command description

## v0.preview.14 - 2026-04-21

- Add escape example of Ante and fix config reload bug
- Fix shutdown bug for offline serve and headless
- Show changelog on update
- Support symlinked user skill roots
- Scope release concurrency by version

## v0.preview.13 - 2026-04-17

- Add initial Claude Code SDK (agent-sdk)
- Add offline mode support for headless, serve, and channel modes
- Add offline mode loading progress bar
- Promote Evt::UserInput to a protocol-level event
- Refactor agent-sdk so CLI owns session id
- Drop redundant search_incomplete field from GrepResult

## v0.preview.12 - 2026-04-14

- Add `--resume` CLI flag and exit resume hint
- Add Slack/Discord integration
- Add ali-coding-plan builtin support
- Update log analyzer to accept workflow URL as input
- Fix Gemini enum problem
- Improve grep tool: pagination, filtering, glob parsing, count totals, and session cwd resolution
- Clarify TUI connect command description
- Remove user group
- Fix smoke test format
- Dependency updates

## v0.preview.11 - 2026-04-07

- Experimental PTY tmux support
- Update init command description with contextual input
- Add Gemma4 model
- Update eval workflow with new harbor
- Improve offline mode log output
- Update Antix wirestyle to Anthropic and add Qwen models
- Adjust offline mode for new llamacpp version
- Add popular models from OpenRouter
- Implement explicit update command
- Dependency updates

## v0.preview.10 - 2026-04-01

- Update openrouter model name
- Fix git commit authors for GitHub Action

## v0.preview.9 - 2026-03-30

- Add dialog snapshot persistence for session restore
- Add event log persistence and TUI replay on resume

## v0.preview.8 - 2026-03-30

- Add guide subagent
- Add number key shortcuts to approval dialog
- Improve inactive model visibility in model selector
- Refactor TUI modal state handling
- Refactor default prompt assembly for agents
- Update ratatui to 0.30 and tui-input to 0.15
- Dependency updates

## v0.preview.7 - 2026-03-25

- Decouple scheduler from review decisions
- Fix quit bug
- Update eval workflow and scripts
- Make browser tool optional
- Eliminate per-delta buffer cloning in streaming output
- Deserialize tool results from &Value instead of cloning
- Sort model selector by current provider first
- Simplify TUI thinking selector handling

## v0.preview.6 - 2026-03-24

- Add queued message feature for multi-turn input
- Add browser tool
- Fix OpenAI codex backend
- Reduce tool input cloning
- Dependency updates

## v0.preview.5 - 2026-03-22

- Add /statusline command for configurable footer
- Add PR link status line item
- Add thinking level selector to model switcher
- Use theme.secondary for status line text to improve readability
- Refactor skill module into core/skill
- Reorganize agent specs
- Add websocket transport for serve mode
- Add release skill for tagged releases
- Fix assistant messages in OpenAI Responses API
- Dependency updates

## v0.preview.4 - 2026-03-14

- Add Criterion benchmarking for core fs and Bash tools
- Add release benchmark baseline reporting
- Fix update Antix's default URL to public domain
- Fix typos and spelling
- Update calculation for benchmarks
- Move bundled assets to top-level module
- Dependency updates

## v0.preview.3 - 2026-03-11

- Prioritize TUI input over protocol events
- Flatten llm catalog presets
- Move catalog into llm module
- Handle queued steers around approval pauses

## v0.preview.2 - 2026-03-09

- Fix command popup scrolling when selection moves past visible area
- Add Ante terminus
- Add standard OAuth support for Antix
- Fix OAuth callback server cancellation and bind errors
- Adjust OpenAI reasoning effort mapping
- Dependency updates
