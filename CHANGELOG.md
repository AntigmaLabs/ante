# Changelog

## v0.preview.93 - 2026-09-02

### Changed
- Tool activity in the TUI shows project-relative paths (or `~` for other home-directory paths), labels completed shell commands as `Ran`, and keeps header punctuation visually neutral
- Compaction learns a provider's actual context window after an overflow, scales summaries and anchors to fit, and retries once instead of walking a shrinking retry ladder

### Fixed
- Parallel tool results share one context cap, preventing oversized batches from entering a repeated clear-and-rerun loop; cleared-result markers now identify the original tool call
- Buffered provider requests stop at the configured idle timeout instead of potentially hanging until the connection fails
- A second Cerebras turn after reasoning and tool use no longer fails because replayed `reasoning_content` is unsupported
- Goal evaluation and compaction retry once without `tool_choice` when an OpenAI-compatible provider rejects that parameter
- OpenCode zen usage-window errors are classified as terminal quota errors instead of being retried as transient rate limits

### Wire
- `SessionInitialized` adds `skills` and `subagents`, so `SessionStart` and `SessionUpdated` announce the capabilities equipped for the session; both fields default to empty when absent

## v0.preview.92 - 2026-08-31

### Added
- `/add-provider` — a bundled skill that walks you through adding a custom OpenAI-compatible provider: it discovers the endpoint's models, probes which reasoning effort levels it accepts, and writes the entry into `~/.ante/catalog.json`
- `catalog.json` can declare which reasoning effort levels a model supports, so a custom endpoint shows its real ladder instead of one guessed from the model's name

### Changed
- Grep and Glob page correctly: a page is cut at a whole entry and the "more results" marker points at the cut. Grep previously dropped results past its character limit while claiming there were more. Glob is now paginated too, returns partial results when it times out, and both tools list files in path order
- File search in the TUI is much faster on large projects, and no longer hides files nested more than six levels deep
- Resuming a long session is faster
- Reasoning effort is consistent everywhere — the selector, the session, and the provider all agree on the level you picked
- Compaction skips a fold that cannot free any space, instead of repeatedly compacting to no effect on small context windows
- Large tool results reach you and the model whole, instead of being replaced by a "trimmed, please retry" error that forced tools to be re-run one at a time
- A slow or huge error page from WebFetch no longer eats the whole request timeout

### Fixed
- Anthropic subscription sign-in works again against Anthropic's current OAuth endpoints, and verifies it was granted inference access before reporting a connection
- Headless `--resume` no longer fails to start when the provider comes from the saved session, and keeps its memory setting
- `/offline-mode` no longer crashes the TUI on terminals shorter than 12 rows

### Wire
- `Op::StartSession`'s payload is renamed `SessionOverrides` → `SessionRequest`. Field names are unchanged, but an unset field now means "use the host's default" rather than "leave unchanged"
- `ModelSpec.effort_options` is replaced by `supported_efforts`. Clients that read the effort ladder need a coordinated bump

## v0.preview.91 - 2026-08-30

### Added
- Anthropic `xhigh` reasoning effort — available on Claude Sonnet 5 and Opus 5, Opus 4.7/4.8, and always on Fable/Mythos 5; Opus/Sonnet 4.6 keep their four-level adaptive ladder
- `gpt-5.4-mini`, `gpt-5.4-nano`, and `glm-5.3-flash` in the Antix catalog
- Completed reasoning blocks show how long the model thought — `Thought for <duration>`, derived from protocol event timestamps so replays report the same number

### Changed
- Compaction tuning: manual `/compact` folds more history than automatic compaction, so running it twice in a row no longer re-folds a fresh summary; an overflow retry stops once it stops making progress instead of spending more provider calls; and a single oversized tool result is trimmed rather than costing a rejected request
- GLM 5.3 defaults to max reasoning effort when an OpenAI-compatible profile supplies no effort, and Anthropic stream errors carrying the `invalid_request` provider alias are classified instead of surfacing raw
- Telemetry uses the standard `OTEL_EXPORTER_OTLP_ENDPOINT` and `OTEL_EXPORTER_OTLP_HEADERS` environment variables

### Fixed
- Markdown link URLs render in the design steel blue instead of hardcoded ANSI blue, which was nearly unreadable on dark backgrounds

## v0.preview.90 - 2026-08-27

### Added
- `/compact` accepts instructions that steer the handoff summary — `/compact focus on the API changes` passes trailing arguments into the fold prompt

### Changed
- Redesigned compaction: one reducer with graded decay — aged tool results are evicted before anything is summarized, the newest slice of the dialog and your own messages survive verbatim, and the summary covers only what the model can no longer see. Compaction is decided at one point in the turn instead of several, so it is more consistent and no longer costs the model its working state
- Bash results lead with the exit status as plain text instead of JSON-escaped output: stdout and stderr stay separate, empty sections are omitted, background handles are named, and signal-terminated jobs report the shell's `128 + signal` convention
- The offline llama engine tracks the official llama.app artifact matrices, pinned at `b10612` (was `b10217`): curated ARM and x86 CPU/Vulkan feature tiers with the strongest compatible tier selected at install time, CUDA-first GPU selection with Vulkan fallback, and every mirrored artifact dependency-audited before upload
- Dependency updates

### Wire
- `Op::Compact` changes from a unit variant to a struct variant carrying optional `instructions`. External clients on the old shape (sending bare `"Compact"`) need a coordinated bump

## v0.preview.89 - 2026-08-26

### Added
- GLM 5.3 Flash on Z.ai (`glm-5.3-flash`) and OpenRouter (`z-ai/glm-5.3-flash`), with multimodal input, a 1M context window, and graded low/high/max reasoning effort; GLM 5.3 remains the default Z.ai model

### Changed
- Run arguments supplied alongside a subcommand are now rejected instead of silently ignored (`ante --model X catalog` errors)
- Tool filters take one value per occurrence — `--tools Read,Write` or a repeated flag; space-separated lists now error, and `--allowed-tools` is removed in favor of `--tools`
- `--profile` is process-scoped: it rides before or after any subcommand, and `serve`, `gateway`, `doctor`, `update`, `rage`, and external apps honor it instead of reading default settings; an unknown profile name falls back to `settings.json` rather than creating one
- WebFetch drops its unused `prompt` argument — Ante has no extraction sub-model, so the interface is URL-only

### Fixed
- WebFetch identifies itself with an `ante/<version>` User-Agent — GitHub's API rejects UA-less requests with an opaque 403 — and a failed request now carries up to 2KB of the response body instead of a bare status line
- `ante serve` handles a client that closes stdout as a logged disconnect instead of hanging, stops reading stdin after an explicit `Shutdown`, and bounds runtime teardown

## v0.preview.88 - 2026-08-25

### Added
- `ante <name>` runs an external `ante-<name>` executable, resolved from `~/.ante/bin` then `$PATH`; built-in subcommands always take precedence, and an unrecognized name still gets a did-you-mean suggestion
- Antix OAuth catalog refresh: Gemini 3.7 Flash, Claude Sonnet 5, Claude Opus 5, and GLM 5.3 replace Gemini 3.5 Flash, Sonnet 4.6, Opus 4.8, and GLM 5.2

### Changed
- `ante --help` and usage errors no longer create a log file or boot telemetry — the CLI now parses before the async runtime, logging, and crash hook are initialized
- The bundled `skill-creator` skill is model-invocable only and no longer appears as a user-facing command

### Fixed
- The "no provider is configured" error names the credential environment variables Ante actually reads and points at `ante auth login` and offline mode
- A terminal that never answers the cursor-position query now explains that the inline UI query went unanswered and suggests launching outside the current wrapper or multiplexer, instead of reporting a bare crossterm timeout
- MCP tool schemas no longer carry `pattern` onto the wire, so an unsupported regex construct (such as lookaround) in one server's schema can no longer make a provider reject the entire tool set; non-regex guidance like `format: email` is preserved

## v0.preview.87 - 2026-08-23

### Added
- GLM 5.3 on Z.ai and OpenRouter, with graded low/high/max reasoning effort (Antix keeps GLM 5.2 until its catalog exposes 5.3)
- `/models <model_id>` selects a model directly on the current provider, including IDs not in the catalog listing
- Denying a tool call can carry a feedback message: the reason reaches the model as the failed tool result and is shown in the transcript under the decision line
- `/term` opens a native split inside Ghostty 1.3+

### Changed
- Successful file reads render as a compact one-liner (`Read (path) ·110 lines`) and failed calls carry a red `Failed` suffix with the error as a dim child row; grouped runs list reads and failures per call instead of folding them into the neutral counts summary
- The default status line renders `model(effort)` and `cwd(⑂ branch)` as compound items, with the same independent `/statusline` toggles
- Markdown blockquotes use the muted theme foreground instead of bright green, so quoted lines read as asides
- The chat input, diff approval, credential entry, and model settings share an in-house grapheme-aware text buffer; combining marks, skin tones, ZWJ emoji, and wide glyphs now wrap and move the cursor correctly (removes the `tui-input`, `ratatui-macros`, and `critical-section` dependencies)

### Fixed
- A mid-turn compaction (context-overflow retry) no longer ends the turn by printing the summary as the answer — the summary is re-entered as a user-role continuation so the model resumes the task
- Launching the TUI with a redirected stdout fails fast with an actionable message instead of hanging two seconds and dying with a cursor-position error
- A persistently empty provider response fails the turn after three retries instead of resubmitting the same prompt up to the step limit, and tool calls whose stream never delivered a name are failed back to the model instead of being silently dropped
- Queued keystrokes can no longer approve a tool approval prompt that was just swapped in
- The public repository's adapter tests are kept in sync with the source of record, which previously failed deterministically in the published checkout

## v0.preview.86 - 2026-08-20

### Added
- `/usage` dialog reporting per-model token usage, accounted client-side
- Double-click selects a word and triple-click selects a line in the transcript; alt-screen pager views (ctrl+o) copy on select

### Changed
- Thinking collapses to a pulsing marker while it streams, with the full text kept in the ctrl+o transcript; Anthropic adaptive thinking displays summarized by default
- Providers accept `stream_idle_timeout_secs` to override the 300-second default used for the first-event, SSE-idle, and WebSocket-idle deadlines
- Background job files persist until an explicit discard or the startup sweep, and waiting on a job is now a re-budgetable observation — an expired wait window changes nothing and can be extended; abandoning a shell call interrupts the process
- The experimental Browser tool and its optional Chromium dependency are removed
- `@agent-<name>` mentions no longer inject an "invoke the agent" reminder; the `@` popup keeps its completion rows as a typing convenience
- Dependency updates

### Fixed
- Copying a multi-line selection rejoins soft-wrapped lines instead of inserting hard breaks mid-sentence
- A permission-mode change (Shift+Tab) made while a turn is settling is no longer reverted when the turn commits

## v0.preview.85 - 2026-08-19

### Changed
- The `Abort` approval decision is removed from the wire protocol; "deny and stop" composes as a deny plus an interrupt

### Fixed
- Invalid permission rules in settings are surfaced as settings notices instead of being silently skipped, while the valid rules keep loading
- A background job's directory is cleaned up on any startup failure, instead of a failed output-file creation stranding it until the 24-hour GC sweep

## v0.preview.84 - 2026-08-18

### Changed
- Qwen 3.8 models send graded `reasoning_effort` levels instead of collapsing every level to thinking on/off, and the effort picker lists only the distinct supported levels
- MCP servers are discovered in parallel (one slow server no longer delays the rest), and MCP tool calls time out after 10 minutes — overridable via `ANTE_MCP_TOOL_TIMEOUT` — instead of pinning a turn until interrupt

### Fixed
- Headless (`ante -p`) sessions wait for MCP warm-up before the first turn, so MCP tools are present in the model's schema instead of every MCP call failing silently
- llama.cpp context overflows are recognized as context-full and trigger the one-time compaction retry instead of failing the turn
- Writing to a closed or broken output pipe (consumer exits, macOS EIO) is handled leniently instead of panicking during terminal teardown

## v0.preview.83 - 2026-08-18

### Added
- `include_skills` / `exclude_skills` settings control which skills a session equips; exclude wins over include

### Changed
- The `local` provider exists only while a live model server is registered; a saved `local` selection with no server running gates with a clear notice instead of retrying for ~17 seconds per turn
- Skill and agent lists moved from the system prompt to a first-message reminder, making the prompt byte-stable across sessions; subagents inherit the parent's skills
- The stable channel and installer only promote after artifacts pass an installer smoke test

### Fixed
- Long TUI sessions no longer retain render caches for archived scrollback, and transcripts beyond 65,535 rows stay complete

## v0.preview.82 - 2026-08-17

### Added
- `ante offline install` downloads and installs the bundled offline inference engine; running an offline model on a machine without the engine now fails fast with that instruction instead of an opaque error

### Changed
- Grep and Glob permission rules and session approvals now scope to the search pattern instead of blanketing the whole tool, and their transcript rows show just the pattern
- Tool calls in the transcript show wider, more balanced argument previews
- Markdown output is restyled: headings, ordered-list markers, inline code, tables, and fenced code blocks on a dark panel background
- `ante catalog` output is wrapped in a `{"providers": [...]}` envelope

## v0.preview.81 - 2026-08-15

### Added
- Qwen 3.8 27B as a verified offline model

### Changed
- macOS release binaries are signed and notarized, so Gatekeeper no longer blocks first launch
- Sent messages are restyled with a muted margin bar, and the composer prompt shows a chevron
- Quiet tool calls are grouped into one collapsing activity cell instead of stacking individual rows
- Log files are written to per-day directories (`logs/<date>/ante.<pid>.log`)
- Background job handles live under `run/jobs/<proc_id>/`, and the Ante home gains a `tmp/` scratch tier

## v0.preview.80 - 2026-08-14

### Changed
- The bundled `ante-guide` skill routes to the documentation corpus instead of inlining reference tables that drift, and no longer suggests the nonexistent `/permissions` command

### Fixed
- Empty streaming deltas from providers (Qwen-style reasoning placeholders, OpenRouter-style empty text) no longer fragment messages into broken parts

## v0.preview.79 - 2026-08-14

### Added
- Gemini 3.7 Flash in the native Gemini, Vertex AI Gemini, and OpenRouter catalogs

### Changed
- The bundled `ante-guide` skill answers configuration questions and edits Ante's own settings from a built-in reference, without cloning the docs repository

### Fixed
- The composer cursor no longer blinks, which broke scrollback in terminals that scroll on output
- WebSearch calls are capped at 90 seconds instead of hanging a tool batch

## v0.preview.78 - 2026-08-13

### Added
- Grok 4.6 in the Antix OAuth catalog
- DeepSeek V4 Pro 0813 on OpenRouter, replacing the superseded preview build, with the GA model's native `max` reasoning effort

### Changed
- Dependency updates

### Fixed
- xAI turns no longer fail with `Argument not supported: search_context_size` — the field is dropped from the provider-native web search tool, so xAI works without `--exclude-tools WebSearch`
- OAuth credentials refresh against the active preset's endpoint, and token issuer URLs are validated
- Antix Grok 4.5 is marked text-only, matching what the live route actually accepts
- `ante doctor` no longer errors when its output is piped to a consumer that closes the pipe early

## v0.preview.77 - 2026-08-12

### Added
- Grok 4.6 in the xAI and OpenRouter catalogs, replacing Grok 4.5

### Changed
- Thinking renders as markdown in the thinking grey, instead of showing raw `**` markers

### Fixed
- Hitting a usage limit no longer triggers repeated compaction attempts
- OpenAI server-overload errors are retried instead of failing the turn
- Background job handle files are cleaned up at startup once the job has finished, instead of accumulating
- `/term` reports tmux as missing instead of opening an empty picker
- An empty `ANTE_ENV` is treated as unset, so telemetry keeps its default environment label

## v0.preview.76 - 2026-08-11

### Added
- Qwen 3.8 Max, Qwen 3.7 Flash, and Muse Spark 1.2 in the OpenRouter catalog; the superseded Qwen 3.7 Max and Muse Spark 1.1 entries are pruned

### Changed
- Tool headers color their arguments — file paths, commands, search patterns — in the design blue instead of plain gray
- The composer cursor blinks
- Skill and agent frontmatter goes through one shared reader with size and nesting-depth limits, so oversized or malformed frontmatter fails with a clear error instead of being partially accepted

### Fixed
- Bundled skills are installed before the daemon starts serving, so they are usable on the very first launch after an update instead of only from the second launch on
- Multiline Grep searches over a directory run one worker at a time, so concurrent search buffers can no longer multiply peak memory
- The Grep header separates its pattern and path — `Search (foo in /tmp/src)` instead of `Search (foo)/tmp/src`

## v0.preview.75 - 2026-08-10

### Added
- Custom providers in the user catalog accept an `extra_body` map — provider-specific fields merged into chat and streaming request bodies, for gateways and proxies that require extra parameters. Keys that collide with Ante-owned request fields are dropped with a notice at catalog load instead of rejecting the whole provider

### Changed
- `SessionStart` and `SessionUpdated` no longer carry the active provider's full model list on the wire; that catalog data is available from `ante catalog`. Clients still receive the provider id, display name, and effective base URL

### Fixed
- Anthropic requests are compatible with proxies and gateways again: the unsupported context-management field is no longer sent, extended-thinking budgets respect the 1024-token minimum, and a temperature other than 1 with thinking enabled now fails with a clear message instead of a confusing provider error

## v0.preview.74 - 2026-08-10

### Changed
- When telemetry is configured, Ante uses a random, resettable installation ID and per-process run IDs; operator identity is sent only when explicitly configured

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
