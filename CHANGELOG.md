# Changelog

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

