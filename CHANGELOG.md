# Changelog

## v0.4.0 - 2026-11-28

Offline mode, rebuilt: name any Hugging Face GGUF and it downloads and runs, from one list, on one engine shared by every Ante. Offline settings and MCP servers get their own files, and Ante updates itself in the background. Offline users have a few steps after upgrading: see Breaking.

### Breaking
These need a step after upgrading, or change a name you may have saved. The Wire changes below are breaking for protocol clients.
- `~/.ante/offline-config.json` is no longer read, and nothing moves over. Offline settings live in `~/.ante/offline.json` (or an `offline` key in `settings.json`): `engine`, `models_dir`, and per model id `source`, `context`, `temperature` and `args`. Set your models folder again (Ctrl+F in `/offline-mode`) and run `ante offline use <MODEL>` again
- `~/.ante/verified_models.json` is no longer read: add a model of your own as an `offline.models` entry with a `source`
- The offline provider is `offline` (was `local`), shown as `llama.cpp`; a saved `local` selection falls back with a notice. `ANTE_LOCAL_PROVIDER_PORT` is `ANTE_OFFLINE_PROVIDER_PORT`, and `catalog.json` can't define `offline`
- Downloads live in the Hugging Face cache (`~/.cache/huggingface/hub`), shared with other tools. Old copies are not reused; to keep them: `mv ~/.ante/models/models--* ~/.cache/huggingface/hub/`
- One id per offline model: the verified short id (`qwen3.8-27b`), else the file name in lower case. The `_local` suffix is gone
- An MCP entry's transport comes from `command` (stdio) or `url` (HTTP); `transport`, `type` and `oauth` are ignored. `ANTE_MCP_TOOL_TIMEOUT` is replaced by the entry's `timeout`
- `ANTE_FILE_SEARCH_TIMEOUT` is no longer read
- `/update` and the install on exit are gone; `ante update` stays. `/models` is `/model`, which matches short names (`/model opus`). The global Ctrl+E stop is gone: Ctrl+S in `/offline-mode` stops the engine and Ctrl+U upgrades it

### Added
- New releases install in the background and run from the next launch; `✓ Update installed · Restart to update` shows above the input. Auto-update in `/config` turns it off. Only published builds update themselves; Homebrew goes through `brew upgrade`
- A first launch on an offline model needs no sign-in, and the sign-in dialog offers "Run a model offline". A headless run with no provider set up and nothing named starts offline on a verified model, installing the engine and downloading the model if needed
- `ante offline fetch <MODEL>` downloads a model and resumes one cut short; `--offline-model` and the selector fetch a model before loading it
- Any GGUF file on Hugging Face works, by URL or as `<owner>/<repo>:<file>.gguf`
- `offline.models.<id>.source` names a model of your own (a path, URL or short form), so `--offline-model <id>` works
- `offline.engine` runs your own llama.cpp `llama` launcher in place of the installed engine
- The offline selector and `ante offline list` mark each model `●` loaded, `○` on disk or `↓` not downloaded. The selector shows each model's size and the memory it needs, and Ctrl+F sets the folder for your own GGUF files
- An idle offline model frees its memory after 30 minutes and wakes on the next request
- `~/.ante/mcp.json` takes MCP servers in the format Claude Code and Cursor use (`mcpServers`). Ante never writes it, and a server of the same name in `settings.json` wins
- MCP entries take `${VAR}` and `${VAR:-default}` in every field, and `timeout` (milliseconds) per tool call
- Turn an MCP server off without deleting it: Space in `/mcp`, or `disabled_mcp_servers` in `settings.json`
- Gemini 4 Carbon, Neon and Hydrogen on Antix

### Changed
- The saved offline model is `provider_model.offline`, set by `ante offline use` or a pick in the TUI. A bare `--offline-model` loads it, else a verified model on disk, else the smallest verified download. The list no longer orders by last use
- One offline list: verified models first, downloaded ones ahead, then your own. A downloaded model is one row
- The offline engine is one router per Ante home, shared by every Ante, and only Ctrl+S or `ante offline stop` ends it. A failed load keeps the router. GPU layers are left to the engine
- `settings.json` holds only the keys that are set
- Attended Anthropic sessions keep the prompt cache for an hour instead of 5 minutes; headless runs and subagents keep 5 minutes
- A reply whose stream drops mid-answer continues from the partial text
- A running compaction shows in the spinner, and MCP warm-up reports only the servers that fail to connect

### Fixed
- A Gemini project at its monthly spending cap is a quota error instead of a retried rate limit
- Native MiniMax requests send at most the newest 200 images
- Gemini 4 Neon and Hydrogen over Antix no longer fail every request with HTTP 400
- `/mcp` Delete no longer restarts the session when it removed nothing
- `ante gateway` gets the normal unknown-command error

### Wire
- `Info`, `Error` and a failed `TurnEnd` carry `Notice { kind, headline, details }`; a bare string is still read
- `InfoBlockStart` and `InfoBlockAppend` are removed: MCP warm-up sends one `Info` with `kind: "mcp_failed"`. A session log with `InfoBlock*` lines resumes with a "skipped N damaged log records" notice
- `RegisterLocalProvider` and `RestoreLocalProvider` are removed

### Crates
- Public crates published as 0.4.0: ante-protocol-shape, ante-llm, ante-sdk. `ante-llm` uses the llama.cpp dialect for provider `offline`

## v0.3.9 - 2026-11-23

### Added
- `compact_at` in the catalog sets the token count at which auto-compact starts for a model. Gemini 4 models start at 500,000 tokens instead of near the end of their 1M window
- Inkling 2 Mini in the offline catalog: `ante --offline-model Inkling-2-Mini`
- The installer shows the release version, platform and archive size before downloading, confirms the SHA256 check, and takes `--dir PATH` to choose the install directory for one run
- Homebrew installs update through `brew upgrade`: `ante update` and the update on exit no longer install a second binary under `~/.ante/bin`

### Changed
- A reply or prompt blocked by the provider is dropped and retried with a note instead of ending the turn. An empty reply is nudged the same way. Both share the limit of 3 in a row with the other nudges
- Headless sends the prompt as typed; the working-directory tree is no longer appended
- Offline models take their context, vision, tool support and effort levels from the engine after a load. The effort picker shows the levels the model's chat template declares, and with no effort chosen the template's default applies. The thinking checkbox and `ante offline use --thinking` are removed
- With no context set, llama.cpp sizes an offline model's context to fit memory, and the session uses the per-slot context the engine reports. `--context` above the model's maximum is refused, and `--context 0` clears a saved value
- An offline load is refused when the fitted context is too small for the agent, and a model whose chat template does not describe tools loads with a warning
- The offline engine is pinned to llama.cpp b12606 on macOS and b12607 on Linux
- Deleted lines in diffs are no longer struck through

### Fixed
- Resuming a session in the TUI no longer takes the permission mode, model, effort, title or MCP servers from replayed history, and no longer reopens old approval prompts
- Headless fails the run when a session start, or a `--provider`/`--model` switch on resume, is refused, instead of hanging or running on the saved provider. MCP warm-up events no longer leak into resume output
- Gemini reports a blocked prompt as a content-policy error instead of a dropped stream
- Compaction summary requests carry no tool definitions, so a local model can no longer answer with a tool call and leave the summary empty
- Linux offline installs check that the CUDA or Vulkan build can see a device, and fall back to the CPU build with the reason. `ante offline install` repairs a broken engine
- The TUI saves offline model preferences when no offline config file exists yet
- A long queued message wraps instead of being cut off along with its `ctrl-s to steer` hint

### Wire
- `ModelSpec` gains optional `compact_at`

### Crates
- Public crates published as 0.3.9: ante-protocol-shape, ante-llm, ante-exec, ante-sdk. `ante-sdk` `Client::send` returns the op's id, and `ante-llm` `openai_compatible::effort_levels` takes the model's declared levels

## v0.3.8 - 2026-11-21

### Added
- Untitled sessions name themselves from the first prompt; `/rename` wins

### Changed
- OpenAI Responses caches the system prompt and hands reasoning back, so the model stops re-reasoning each step
- Gemini uses its own tool-call ids and gets no sampling parameters
- `top_p`, `top_k` and `MODEL_TOP_P` are removed
- Restore points survive a model or provider switch
- Refreshed verified offline models; Linux CUDA prebuilts drop `sm_86` (Vulkan fallback)

### Fixed
- Cache-write tokens on OpenAI Responses are counted and priced

### Wire
- `Usage.cache_creation_tokens` → `cache_write_tokens`; `ModelSpec` drops `top_p`/`top_k`

### Crates
- Public crates published as 0.3.8: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.3.7 - 2026-11-19

### Added
- `/rollover [instructions]` continues the conversation in a new session. The current session writes a handoff and stays resumable; the new session starts from the handoff alone, under current prompts and settings. The resume picker shows which session a session continues
- Gemini 4 Neon on the Gemini API, Vertex AI and OpenRouter, with a 1,048,576-token context and low to xhigh thinking. It defaults to high effort on Gemini API and Vertex

### Changed
- The OpenAI API and subscription offer GPT-6.2 Sol in place of GPT-6.1 Sol, at low effort by default
- Antix offers GPT-6.2 Astra and GPT-6.2 Sol in place of GPT-6 Astra and GPT-6.1 Sol
- Bash can wait up to one hour for a command (was 10 minutes) and explains how to block on a backgrounded command's status file. A job's `status.json` records the session that started it
- A reply that has thinking but no answer and no tool call is no longer stored and no longer ends the turn. The model is asked to continue, up to 3 times in a row, a limit shared with `max_tokens` continuations
- Starting a session, including `/clear`, builds the new session before ending the current one, so a failed start leaves the current session as it was
- Dependency updates

### Fixed
- Long lines in code blocks wrap instead of being cut at the right edge, and copying a wrapped line returns the original text, runs of spaces included
- When Bash output is truncated, the full output stays in the file named on the first line until gc
- The chat column on very narrow windows lines up with scrollback, and the offline model settings dialog no longer breaks at one row of height
- The resume picker no longer adds `…` to a non-ASCII first message that fits

### Wire
- `StartSession` accepts optional `SessionRequest.rollover { instructions }`, and `SessionInfo` carries optional `rolled_from`

### Crates
- Public crates published as 0.3.7: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.3.6 - 2026-11-17

### Added
- Gemini 4 Carbon and Hydrogen on the Gemini API, Vertex AI and OpenRouter. Hydrogen defaults to high effort on Gemini API and Vertex, and both routes support `xhigh`
- OpenRouter offers Qwen4 Max, Tencent Hy4 Flash and Meta Muse Ember

### Changed
- The OpenAI API and subscription offer GPT-6.2 Astra in place of GPT-6 Astra, at low effort by default. The Astra subscription context window is corrected to 272,000 tokens
- Gemini 4 output budgets on Gemini API and Vertex rise to 65,536 tokens. Gemini 3.8 Flash is removed from the pickers; Flash-Lite stays
- Dark and light themes use the v2 palette: higher-contrast greys and blue, code blocks without a background, and explicit colors for notices and approval decisions
- The offline engine is pinned to llama.cpp b12375
- Dependency updates

### Fixed
- `ante serve` waits for MCP tools to register before the first turn. Previously the tool list could change between turns, which broke Claude Fable 5.2 sessions with an invalid thinking `signature` error and missed the prompt cache on other models
- A response cut off at `max_tokens` still runs the tool calls that finished before the cut. Only a call cut mid-arguments is dropped
- An Anthropic web search that pauses its turn is resumed, up to 3 times
- An Agent call fails when the subagent's turn did not complete. The parent sees the reason, the child's last message and the transcript path, instead of a success carrying an `[interrupted]` marker
- A cancellation that arrives after a response completed no longer marks that response as cancelled
- Edit keeps the line endings of untouched lines in files that mix LF and CRLF

### Crates
- Public crates published as 0.3.6: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.3.5 - 2026-11-11

### Changed
- Anthropic requests cache the tool definitions and the shared system prompt separately from the conversation. A new session, including one in another directory, reuses that cached prefix instead of writing it again
- The environment and memory-index details now come at the start of each turn instead of in the system prompt
- Antix offers Claude Fable 5.2 in place of Fable 5.1

### Fixed
- When a model ends its turn with an empty reply, the turn ends with one notice instead of storing `[end_turn]`. Other empty replies are retried through the normal reconnect path
- A resume, including the one that ends every TUI rewind, keeps the client's permission mode instead of switching back to the mode in settings
- A rewind adds a record to the session's event log instead of rewriting the log, so a crash during a rewind no longer leaves the session half-rewound
- An MCP tool whose schema has a JSON-value union with a sibling `items` no longer breaks requests to Gemini

### Wire
- `ResumeSession` accepts an optional `permission_mode`. If it is absent, the session uses the current settings

### Crates
- Public crates published as 0.3.5: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.3.4 - 2026-11-09

### Added
- DeepSeek sessions can use WebSearch through DeepSeek's Search API with the session's existing credential. A `deepseek` provider pointed at a gateway that doesn't forward the search endpoint can turn it off with `"supports_web_search": false` in `catalog.json`

### Changed
- Anthropic API, Anthropic subscription and OpenRouter offer Claude Fable 5.2 in place of Fable 5.1, at high effort by default. OpenRouter offers Tencent Hy4 in place of Hy3. Antix drops GPT-5.4 mini and nano. GPT-6 Astra on the OpenAI subscription gets a 500,000-token context window. Explicitly configured models still work
- A model's declared `supported_efforts` now decide the effort level sent on every wire, not only what the picker offers. The lowest level is sent explicitly (`none` on the Responses API, `MINIMAL` on Gemini) instead of falling back to the provider's default
- Startup notices and the update offer print under the session header instead of above the logo
- The offline engine is pinned to llama.cpp b12182

### Fixed
- Switching models within one provider (for example Gemini to Claude on Antix) no longer breaks later turns with an invalid thinking `signature` error
- A configured temperature is sent on the Responses API when reasoning effort is `none`
- Tool-error, unknown-tool and malformed-call warnings in the log no longer include tool arguments or error output

### Crates
- Public crates published as 0.3.4: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.3.3 - 2026-11-05

### Added
- `ante usage` text output aligns the quota bars
- After you sign in to a provider in the browser, the TUI shows an inline signed-in notice

### Changed
- Antix serves Grok 4.7, GPT-6.1 Terra and GPT-6 Luna in place of Grok 4.6, GPT-5.6 Terra and GPT-5.6 Luna, and adds Gemini 3.5 Flash Lite, GLM-5.3 FlashX Legal and Qwen 3.8 Flash. Explicitly configured models still work
- The session header gets more compact in narrow terminals
- Ante no longer changes the tmux pane's `allow-passthrough` setting
- Published builds send telemetry through the Ante telemetry gateway and no longer embed credentials. Utility commands send no telemetry

### Fixed
- Interrupting a turn before any output puts your prompt back in the composer
- A prompt you withdraw no longer stays in the transcript
- A table that is still streaming waits until it is complete before it goes into inline scrollback
- Resuming a long session is fast again: the event-log replay fast path is restored
- Dependency updates

## v0.3.2 - 2026-11-01

### Added
- `ante usage` reports Claude Code and Codex subscription quotas without opening a session: percentage used, reset times, and per-account expired/unavailable states. Select one stored account with `ante usage openai/work`, or get structured output with `--json` (`provider` is the stable profile id, `anthropic` / `openai`). Exits 1 when any selected sign-in is expired or unavailable, after printing the full report
- Providers accept an optional `max_request_images` cap in `catalog.json`. Over the cap, the oldest images in a request are replaced with a text marker; the saved conversation is unchanged. Unset means uncapped, and `0` sends no images

### Changed
- Built-in model lists are trimmed. OpenAI API offers GPT-6 Astra, GPT-6.1 Sol and Terra, and GPT-6 Luna. OpenAI subscription drops GPT-5.6 Sol, Terra and Luna. Antix drops GPT-5.3-Codex, Gemini 3.1 Pro preview, Gemini 3.7 Flash and Grok 4.5. Explicitly configured models still work
- Sonnet 5.5 and Fable 5.1 default to high effort
- The installer shows the download size on its download line

### Fixed
- Images and files returned by a tool stay attached to the call that produced them on the OpenAI Responses and Chat Completions APIs

## v0.3.1 - 2026-10-30

### Added
- `ante auth list --json` prints the stored credentials as one JSON object. Each entry has `reference`, `kind`, `name`, `issuer`, `account`, `state` and `used_by`, and no secret values. `reference` is the exact string a session can pin
- A session can pin a credential when it starts or resumes, so its first request already uses the chosen account instead of the default
- The TUI reports turn status (running, waiting, done, failed) to the terminal via OSC 7501, including from tmux
- Antix serves Claude Opus 5.5, GPT-6.1 Sol and Kimi K3.1 Pro, replacing Opus 5, GPT-5.6 Sol and Kimi K3

### Changed
- The installer adds `ante` to your shell's PATH and keeps the existing binary if an update fails
- Error messages show home paths as `~`

### Fixed
- `--resume` honors `--tools`, `--include-tools` and `--exclude-tools` instead of exposing the full default tool set
- GPT-6.1 Sol on Antix keeps all five effort levels instead of falling back to token-budget thinking
- After a partial write (e.g. a full disk), later session events are no longer swallowed on replay, and persistence failures are reported
- Copying a reply that is still streaming no longer keeps the screen's line breaks
- A tool row left running at the end of a turn is now cancelled
- Dependency updates

### Wire
- `SessionRequest.credential` and `ResumeSession.credential`: an optional credential reference to pin from the first request. The start is refused if it is unavailable
- `ResumeSession.tools`, `include_tools` and `exclude_tools`: optional tool filters for the resumed session. Omitted uses current settings

### Crates
- Public crates published as 0.3.1: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.3.0 - 2026-10-28

Opens the 0.3 series with one intentional break: credentials are configured with one reference language, shared by providers, MCP servers and `ante auth`. Old shapes are refused by name rather than half-read; see "Upgrading from 0.2" below.

### Upgrading from 0.2
- Provider `auth` is now `credential`, a `scheme:name` reference or a list of them: `{"bearer":{"env_key":"X"}}` becomes `"env:X"`; a key sent in a custom header adds `"credential_header": "H"`. `http_headers` is now `headers`, and `{session_id}` is now `${session_id}`. An entry still using the old keys is skipped with a warning naming the replacement (`ante catalog`, `ante doctor`)
- A custom provider that had no `auth` (a local server) now fails with "No credential for <id>": add `"credential": null`
- The `models` section of `catalog.json` is removed; patch a model under `providers.<id>.preferred_models` instead. A non-empty section is refused at load
- MCP servers: the `oauth` block is retired; use `ante auth login <server>` or an `Authorization` entry in `headers`. `ante mcp login`/`logout` are replaced by `ante auth login`/`logout`
- The `antix-api-key` provider is folded into `antix`, which takes a sign-in, `ANTIX_API_KEY`, or a stored key; a saved `antix-api-key` selection falls back to auto-detect
- Stored sign-ins and `auth/api_keys.json` move to `auth/credentials/` automatically on first run

### Added
- `ante auth list|login|logout|add|rm` manages every stored credential: OAuth sign-ins, API keys, and helper output. `list` shows each credential's state and the connectors that use it
- Credential references: `env:NAME`, `secret:NAME` (stored with `ante auth add`), `oauth:PROFILE[/label]` for one of several accounts, and `helper:NAME`, which runs a command from the new `helpers` section of `catalog.json` and caches its output for `ttl_secs`. A list is tried in order; `null` sends nothing
- `oauth_profiles` in `catalog.json` defines additional OAuth sign-in profiles beside the built-in Anthropic, OpenAI and Antix ones
- Provider `headers` values and `extra_body` strings take `${NAME}`, `${NAME:-default}` and `${session_id}`; a `null` header removes a built-in one
- Agents panel: while a turn runs subagents, a panel under the footer lists each with its type, elapsed time and tokens; select a row and press Enter to follow its transcript live
- `/usage` lists every saved Claude and Codex account, each with its own quota
- GPT-6.1 Sol, GPT-6.1 Terra and GPT-6 Luna in the OpenAI subscription provider (Luna is the default for auxiliary calls); GPT-6.1 Terra and Terra Pro in the OpenAI and OpenRouter catalogs; Kimi K3.1 Pro on OpenRouter

### Changed
- `/providers` is one place for choosing a provider, model and credential, signing in, entering keys, and removing accounts; onboarding and sign-in recovery use the same view, and `/connect` remains as an alias. Each row shows where its credential comes from; ←/→ previews another credential of the same provider
- Antix leads the provider list and is the default when nothing is configured; explicit and saved choices still win, and existing credentials for other providers are detected when Antix is unavailable. Default order: Antix, DeepSeek, OpenAI Subscription, Anthropic Subscription
- OpenRouter: Kimi K3 and MiniMax M3 are pruned in favor of K3.1 Pro and M3.1 Flash, and the DeepSeek V4.1 Flash October 17 snapshot becomes the default
- Commands such as `ante auth` and `ante update` exit promptly; telemetry gets at most 250 ms at shutdown

### Fixed
- Headless `ante --resume <id> --provider <p> --model <m>` uses the given provider and model instead of dropping them, so a session saved on a provider that is no longer connected can be resumed
- An invalid environment value read by a built-in sign-in profile (e.g. `ANTIX_BASE_URL=not-a-url`) is a catalog warning instead of crashing every command
- OAuth sign-ins send the redirect URI where the local callback server actually listens

### Wire
- `SessionInfo.provider.credential` and `SessionUpdate.credential`: an optional credential reference (e.g. `oauth:openai/work`), never a token. Omitted means the configured precedence

### Crates
- Public crates published as 0.3.0: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.15 - 2026-10-23

### Added
- Remote MCP servers can authenticate: `url` servers in `mcp_servers` accept `headers` sent on every request, whose values may reference `${NAME}` from the environment so secrets stay out of the settings file; a bearer token is a plain `Authorization` header
- OAuth sign-in for remote MCP servers with `ante mcp login <name>` / `ante mcp logout <name>`. It uses dynamic client registration and the server's advertised scopes by default; an optional `oauth` block sets a pre-registered `client_id`/`client_secret` or explicit `scopes`. A server that answers 401 names the login command to run
- Background jobs view: while jobs run and the composer is empty, ↓ selects the `N bg jobs` footer pill and Enter opens a list of running commands, with status, runtime, the last stdout lines and stderr for each

### Changed
- The update notice's "What's new" preview links to https://ante.run/changelog instead of pointing at `CHANGELOG.md`
- OpenRouter's MiniMax M3.1 Flash preset uses the paid `minimax/minimax-m3.1-flash` route; the free route was retired upstream
- `thinking_budget` set in an OpenAI-compatible provider's `extra_body` is now sent instead of being dropped as a reserved field
- Dependency updates

### Fixed
- TLS verification rejects the compromised intermediate keys on rustls-webpki's deny list (0.103.16), including cross-signed certificates

### Crates
- Public crates published as 0.2.15: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.14 - 2026-10-21

### Added
- Native MiniMax provider: set `MINIMAX_API_KEY` to use MiniMax M3.1 Flash directly (1M context, vision, `low`–`max` reasoning, `max` by default); `MINIMAX_BASE_URL` overrides the endpoint
- MiniMax M3.1 Flash in the OpenRouter catalog

### Changed
- Offline mode defaults to llama.cpp's port 9931 instead of 8080: managed launches try 9931–9940, and user-started servers are discovered on 9931 and 8080 only (previously 8080–8179). A router already running keeps its recorded port, and a server on any other port can still be attached by endpoint
- A provider `Retry-After` longer than 60s ends the turn and shows the wait (`Provider asks to retry in 48m.`) instead of reconnecting every 60s for about four minutes

### Fixed
- Gemini accepts MCP tool schemas it used to reject: `enum`/`const` values that are not non-empty strings move into the parameter's description, and requests without a forced tool choice use `AUTO` instead of `VALIDATED`, so very large schemas are no longer refused
- OpenAI-compatible streams that carry both `reasoning` and `reasoning_content` on one delta, as gateways such as Bifrost send, no longer fail to decode
- `ante offline use --args` rejects malformed values such as `--ctx invalid` before saving, instead of failing at the next load
- Bundled TLS trust roots refreshed to webpki 1.0.10, which drops the emSign ECC Root CA - G3; `ante-sdk` now requires `webpki-roots` 1.0.10 or later

### Crates
- Public crates published as 0.2.14: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.13 - 2026-10-18

### Added
- Jump-to-bottom pill in fullscreen: while the conversation is scrolled up, its last row shows `N new messages (ctrl+l) ↓`, or `Jump to bottom (ctrl+l) ↓` when nothing new has arrived; Ctrl+L or a click on that row jumps to the latest output, and sending a prompt or a `!command` follows new output again
- Subagents record their own event log under the parent session, at `sessions/<parent>/subagents/<child>/events.jsonl`; a subagent is not a resumable session, so `/resume` does not list it, and deleting the parent removes its logs. The parent's Agent tool result names the child's session id
- A warning when session history cannot be saved: a failed event-log or snapshot write shows one notice per session with the path and disk-space/permission guidance, and the conversation keeps running
- Gemini 4 Argon on Antix; DeepSeek V4.1 Flash 1017, Reflection Beam 1.0 and Muse Spark Open in the OpenRouter catalog

### Changed
- Compaction's fold call gets room for reasoning: its output cap is twice the summary's budget, bounded by the model's declared `max_tokens` and the context window, so a model that thinks before summarizing no longer has its summary cut off at the cap
- The fold runs at reasoning effort `low` at most (`min` stays `min`; a model without an effort setting stays without one), since a summary does not need the session's effort
- Dependency updates

### Fixed
- A Responses API `response.incomplete` with reason `length` is treated as a max-tokens stop: a cut-off compaction summary reports `summary was cut off at N tokens` instead of an opaque `failed to fold dialog`, and a normal turn continues as it does after `max_output_tokens`
- Interrupting a turn before the model produces any output attaches the `Interrupted` row to the prompt you just sent instead of the previous answer, and Ctrl+C / Esc between send and the turn's start interrupts the coming turn instead of acting as if the session were idle
- In fullscreen, a bare click on the conversation (press and release without dragging) no longer stops the view from following new output; a drag selection still holds the view in place

### Crates
- Public crates published as 0.2.13: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.12 - 2026-10-15

### Added
- Remote MCP servers over Streamable HTTP: an `mcpServers` entry takes `url` in place of `command`, with an optional `transport` of `stdio` or `streamable-http` (`http` is a synonym). An entry that mixes `command` and `url`, or names the legacy `sse` transport, fails that server at warm-up with a message saying what to change. `/mcp` shows the URL, and `rage` redacts a URL's query string
- `ante offline use <MODEL>` takes `--context <TOKENS>`, `--thinking on|off`, `--temperature <T>|default` and `--args "<llama.cpp options>"` to save per-model load preferences; each flag replaces only its own value, they are validated at `use`, and every invocation ends with a `Loads with:` line
- Gemini 4 Argon in the native Gemini, Vertex and OpenRouter catalogs, with explicit low/medium/high thinking levels; Muse Wonder in the OpenRouter catalog

### Changed
- The `local` provider is derived from the offline runtime state at the moment a model becomes ready, so a `StartSession` sent on the ready reply always finds it registered
- Headless runs and subagents are told that no one can answer them and the session ends with the turn
- llama.cpp bumped to b11540, picking up the router control-pipe fix and CUDA, Vulkan and Metal correctness fixes

### Fixed
- A session that names a provider without a model now gets that provider's default model instead of the global default, which the named provider could not serve
- `Read` recognizes PNG and other images saved without an image extension (or as `.bin`) and shows them as images
- Sharded GGUF models selected by path or saved default are inspected the same way discovery does, reporting the full set; incomplete sets are rejected at resolution
- `ante offline stop` help and messages say what it does: it stops the home's router whether or not a session is using it

### Wire
- `McpServerInfo.url` added: the endpoint of a remote MCP server; `command` is empty for such a server

### Crates
- Public crates published as 0.2.12: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.11 - 2026-10-12

### Added
- `ante offline status`, `ante offline list` and `ante offline use <MODEL>` set up offline mode without the TUI: `status` reports the engine, model directory, default model, running servers and memory; `list` shows local and verified models with size, context and memory fit; `use` saves the default model. `status` and `list` take `--json`
- `--offline-model [MODEL]` starts the local engine for TUI, headless and `serve` launches. It takes a GGUF path, filename, or verified model name or URL; the bare flag starts the saved default. It can't be combined with `--model` or `--provider`

### Changed
- If you keep the offline router running when you exit, the next launch reuses it instead of starting a second one; `ante offline status` shows it on a `Router:` line and `ante offline stop` ends it
- The skills list in the prompt is sized to fit the model's context window, and is resized at every compaction, so switching models or adding MCP tools is accounted for
- Every subagent prompt is built the same way, with the shared core instructions and the environment block; the bundled Explore agent now learns its working directory
- Agent prompts base fixes on evidence and the task's scope, and handoffs keep track of pending jobs and unknown outcomes so actions aren't blindly rerun
- Antix serves Claude Sonnet 5.5 in place of Sonnet 5
- `ante --resume` lists the session archive once at startup instead of twice
- Dependency updates

### Fixed
- Claude Sonnet 5.5 requests that force a tool choice, and Claude Haiku 5.5 requests with a temperature other than 1, now fail before sending with a clear error
- Poolside "input length exceeds" errors are treated as context overflows and recovered by compaction, including when OpenRouter or a proxy wraps them
- Compaction no longer clears long `AskUser` answers before the summarizer sees them

## v0.2.10 - 2026-10-10

### Added
- Opt-in attention cues for the TUI: with `"attention": { "enabled": true }` in `settings.json`, a turn that finishes or fails, and a pause for approval or a question, ring the terminal bell and, while the terminal reports itself unfocused, post a desktop notification (iTerm2, Ghostty, WezTerm, Kitty, foot, Warp, Windows Terminal and VTE terminals; inside tmux, `focus-events` and `allow-passthrough` must be on). `notifications` and `sound` can each be turned off; interrupted turns, queued follow-ups and replayed history stay silent
- The `/resume` picker shows each session's size beside its message count and age
- Claude Haiku 5.5 on Anthropic (API and subscription), OpenRouter and Antix, replacing Haiku 4.5; the lightweight model role now picks Haiku 5.5

### Changed
- Session logs are read faster on resume and replay; damaged logs still recover line by line
- Dependency updates

### Fixed
- A failed compaction no longer removes earlier inputs from `/rewind` and `/fork`, live or when replaying saved history

### Crates
- Public crates published as 0.2.10: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.9 - 2026-10-05

### Added
- `s` in the `/fork` picker opens the forked child in a terminal split beside the parent instead of switching to it, with the cut prompt ready to edit; uses the same split launchers as `/term` (tmux, iTerm2, WezTerm, Kitty, Ghostty)
- Opt-in `context-tokens` status line item shows the context window as `used/limit ctx` (for example `13.6k/400k ctx`), the same figure `/context` reports; turn it on in `/statusline`
- A provider's `http_headers` value in `catalog.json` may contain `{session_id}`, filled with the conversation's id on each request (a subagent sends its parent's id), for gateways that need a per-session header

### Changed
- `/rewind` and `/fork` reach the prompts before a `/resume` and the prompts a fork copied from its parent, back to the last compaction or provider switch, instead of only the current run's
- Every provider call runs under one timeout owned by the transport: headers, error body, OAuth refresh, retry backoff and the WebSocket-to-HTTP fallback share one budget
- Catalog pruned: GPT-5.4 Pro on direct OpenAI (GPT-5.5 Pro stays)
- llama.cpp engine bumped to b11379 (Metal memory-leak fix, Vulkan cache-stride fix, GGUF overflow checks)
- Compaction logs the fold's configuration and the summary's thinking/text split, to diagnose cut-off summaries on local servers

### Fixed
- Responses API "input exceeds the context window" rejections are treated as context overflows and recovered by compaction

## v0.2.8 - 2026-10-02

### Added
- `/rewind` (or Esc Esc on an empty composer) opens a picker over the inputs still in reach; picking one rewinds the conversation to just before it and puts that prompt back in the composer for editing
- `/fork` saves a copy of the conversation cut at a chosen input, or the whole conversation, and switches to it; a forked session names the session it came from under its header, and `/resume` returns to the parent
- `ante serve --sock` and `--ws` print a readable transcript of connections, sessions, tool calls and turn status; a nonempty `NO_COLOR` disables styling
- GPT-6.1 Sol on OpenAI and OpenRouter

### Changed
- A session resumed under a different OS or shell, or on a later day, is told what changed in one notice on its next turn; resuming in the same environment tells nothing
- The session picker orders sessions by last use instead of start time
- Every provider call streams, so compaction, the goal judge and ambient hints share the turn's timeouts and error handling
- Catalog pruned: GPT-6 Sol on direct OpenAI; GPT-6 Sol, Sol Pro and the GPT-5.6 Sol/Terra/Luna Pro routes on OpenRouter; deprecated GPT-5.3 Codex and GPT-5.4 Nano
- Release binaries are built with full LTO and are about 6% smaller
- Dependency updates

### Fixed
- `/usage` refreshes an expired access token instead of reporting the session expired, labels OpenAI quota windows by their length (a 7-day window is "Weekly limit", not "Current session"), and shows the Anthropic plan name without the rate tier
- Responses API rejections of an oversized request that carry no specific error code are treated as context overflows and recovered by compaction
- A cancelled call on an OpenAI WebSocket session discards its connection, so the next request no longer reads the abandoned response
- A provider that accepts a request and never answers, or sends an error status and then stalls, fails at the idle timeout instead of hanging; the OAuth token refresh has its own timeout

## v0.2.7 - 2026-09-29

### Added
- Claude Sonnet 5.5 on Anthropic and OpenRouter; Qwen 3.8 Max and GLM 5.3 FlashX on Antix; xAI moves to Grok 4.7

### Changed
- Resumed sessions keep the system prompts they were started with (including `--system-prompt`, `--append-system-prompt` and `--short-prompt`) instead of rebuilding them from current settings; if the date has changed, the model is told on the next turn
- Dates given to the model use the local day instead of UTC
- `/resume` inside a running TUI starts on a fresh screen with the resumed session's header
- Offline model preferences are loaded during discovery; edits to `offline-config.json` take effect on the next reopen
- llama.cpp engine bumped to b11200

### Fixed
- Provider errors for an exhausted credit balance are reported as quota errors, with billing guidance

### Wire
- `Op::RewindSession { to }` rewinds the active session to just before input `to` and answers `Evt::SessionRewound { to }`
- `Op::ForkSession { at }` saves a new session from the conversation at input `at` (or from the whole conversation) without starting it, and answers `Evt::SessionForked { forked_from, session_id, at }`
- `SessionInfo.forked_from` names the session a fork came from

### Crates
- Public crates published as 0.2.7: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.6 - 2026-09-27

### Changed
- OpenRouter now defaults to DeepSeek V4.1 Flash; the older V4 Flash preset is gone
- DeepSeek V4 Pro stays in the Antix catalog
- NVIDIA Nemotron 3.5 Lightning removed from the OpenRouter catalog
- The TUI spinner now pulses with a pause between beats, and the mascot has new poses
- The bundled `ante-guide` and `add-provider` skills are now cheat sheets tied to the installed version, including an "After an update" section

### Fixed
- Output requests are capped to the context that remains, so large-context sessions no longer get rejected for asking for too much output
- When an image or document is dropped to fit the content budget, the message now says it was omitted entirely
- Shift+Enter inserts a newline inside tmux, and in zellij after closing fullscreen overlays such as `/resume`, the transcript or `/diff`
- The provider is saved after an OAuth connect
- Raw MCP server stderr is logged at debug level

## v0.2.5 - 2026-09-24

### Added
- Switch a session's provider in place, keeping its ID, messages, title, and permission state

### Changed
- MCP server startup and protocol compatibility are stricter
- WebFetch points the model toward URLs it has already found
- Pruned outdated entries from the built-in model catalog
- TUI shows tabs instead of dropping them
- Grep keeps the match visible when it clips a long line
- Simplified the LLM provider layer
- Dependency updates

### Fixed
- Abandoned MCP calls are cancelled, and rich MCP results are kept
- Throttled web searches recover within the call budget
- Oversized tool results are attributed to the calls that produced them

### Wire
- `SessionUpdate.provider` added: switches the session to a named catalog provider on the next turn. An unknown provider, or a switch while a turn is running, returns `Error`

### Crates
- Public crates published as 0.2.5: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.4 - 2026-09-22

### Added
- GPT-6 Sol and Luna (OpenAI and OpenRouter), Claude Opus 5.5 (Anthropic, subscription presets, and OpenRouter), Grok 4.7, and MiMo 2.6 Pro/Flash in the built-in model catalogs
- `ante update` prints a message when no newer version is available

### Changed
- Offline engine updated to llama.cpp b11046; the removed `--mlock`, `--mmap`/`--no-mmap`, and DirectIO loading flags are rejected with guidance
- Restored the muted spinner and amber tool indicators in the TUI
- TUI modal state refactor: every modal is held as a value in the modal state

### Fixed
- Shared high-detail images are capped at 2000px

### Crates
- Public crates published as 0.2.4: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.3 - 2026-09-21

### Added
- Bare `ante --resume` opens the interactive session picker directly without requiring a session ID or creating a throwaway session
- First-class `System` message role for runtime instructions, serializing natively to Anthropic system and OpenAI developer messages with fallback to system reminders

### Changed
- Preserve subagent deliverables in parent handoffs by returning final task answers directly instead of requesting an extra synthetic summary turn
- Clarify context compaction marker to state that earlier history remains for reference while the model sees only the summary
- Working spinner remains visible continuously throughout active turns (streaming, thinking, and tool execution)
- Active background jobs are highlighted with the theme accent and display without obstructing scrollback
- AskUser accepts single-option questions to avoid unnecessary retries, and surfaces tool errors visibly in chat
- TUI performance and state ownership optimizations: cached row span allocation reductions, single-fact settings ownership in `AppState`, and git branch re-reading only at turn and tool boundaries

### Wire
- `Evt::TaskEnd` event added to notify clients of structured job and background task completions

### Crates
- Public crates published as 0.2.3: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.2 - 2026-09-20

### Added
- `max_concurrent_subagents` setting in `settings.json` to cap parallel Agent calls
- NVIDIA Nemotron (3.5 Lightning, 3 Ultra) and OpenRouter models (GPT-6 Astra, Astra Pro, Qwen 3.8 Flash)
- Ctrl+Shift+V shortcut for clipboard image paste in the composer
- Show Bash descriptions as headlines with command preview and elapsed time

### Changed
- `ante gateway` delegates to the external `ante-gateway` executable
- `ante rage` excludes gateway channel configuration
- TUI architecture and rendering refactors: ChatState state ownership, pure pager frames, first-fit text wrapping, and unified row types
- Show shell commands immediately upon invocation
- Stable TUI spinner with delayed elapsed time display
- Diff views omit unchanged-line indicator rows
- Refined task completion, delegation, and length guidance in system prompts
- Model catalog cleanup: removed obsolete DeepSeek Flash and OpenRouter Opus routes

### Fixed
- Session resume re-renders the complete event log instead of capping at 200 events
- User image and PDF attachments are preserved in Responses wire serialization
- Queued messages return to the composer when replacing a session
- Strip ANSI escapes from external tool and shell output to avoid wrap clipping
- Soft-wrapped notices and shell output rejoin properly when copied

### Removed
- Removed `ante_sdk::claude` and legacy child-process stdio transport; host stdio connections remain supported via `connect(Endpoint::Stdio, options)`

### Crates
- Public crates published as 0.2.2: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.1 - 2026-09-19

### Added
- DeepSeek V4.1 Flash in the Antix catalog (`deepseek-flash`), with native vision support and reasoning effort controls
- Fallback lookup for Cargo-installed external `ante-*` commands when `~/.cargo/bin` is missing from `PATH`

### Changed
- `TodoWrite` is now opt-in via `--include-tools TodoWrite`; default TUI sessions omit the pinned task list
- `ante gateway` connects to a running Ante host (`ante serve --sock` or `--connect`) via the SDK rather than managing an in-process runtime
- TUI working spinner delays appearance until three seconds into a turn, eliminating unnecessary animations on quick replies
- Removed the built-in `ali-coding-plan` preset; users can configure it as a custom provider
- Dependency updates

### Fixed
- Images are filtered out before sending to models that do not support vision, preventing API errors without altering saved history
- Fullscreen TUI mode supports Shift+Enter for newline insertion in terminals with Kitty keyboard protocol support
- Compaction summaries stream through the standard provider transport, inheriting the same timeout and error handling as regular turns
- Provider credit exhaustion errors (HTTP 429) are classified as quota limits rather than transient rate limits, preventing minute-long retry loops
- Bash commands receive the calling session ID in `ANTE_SESSION` instead of inheriting stale parent process values
- Discord gateway replies route back to the original conversation without requiring explicit bot re-mentions

### Crates
- Public crates published as 0.2.1: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## v0.2.0 - 2026-09-17

First numbered release, closing the `v0.preview.N` series. From now on, patch releases within a minor series stay compatible; intentional breaking changes move to a new minor series with migration notes.

### Added
- AskUser tool: the model can ask one to four structured multiple-choice questions mid-turn. The TUI shows them in place of the composer, with option previews, free-text notes, and multi-select; Enter takes the recommended option, Esc skips so the model proceeds on its own judgment, and "Chat about this" returns to conversation. The gateway renders questions as numbered options. Unattended sessions never pause, and subagents do not get the tool
- GPT-6 Astra in the OpenAI subscription provider, now its default model (272K context, image input); GPT-5.5, GPT-5.4, and GPT-5.4 Mini are removed from the subscription picker
- `ante-sdk` connects to `ante serve --ws` over `ws://` (loopback hosts only) or `wss://` (opt-in `wss` feature), passing a bearer token

### Changed
- `ante serve --ws` requires a bearer token from `ANTE_SERVE_TOKEN` and refuses to start without one; clients must send `Authorization: Bearer <token>`, missing or wrong tokens are refused with `401`, and handshakes time out after 10 seconds
- Disposable storage moved out of Ante home: WebFetch overflow goes to the OS cache directory (macOS `~/Library/Caches`, Linux `XDG_CACHE_HOME`, Windows `LOCALAPPDATA`), and intermediate downloads use the OS temp directory
- `ante rage` writes the finished report to `--output` (default: the working directory) in both compressed and uncompressed formats, never overwrites existing output, and falls back to an `.uncompressed` directory if compression fails
- Agent autocomplete in the TUI inserts quoted mentions (`@"name (agent)"`) instead of `@agent-name`

### Fixed
- Resumed sessions use the current permission settings instead of the saved mode, so a session saved in Yolo no longer overrides Strict settings. Older snapshots still load; their saved mode is ignored and dropped on the next save
- The running-tool row wraps long shell commands over up to three rows with a `… (+N chars)` marker instead of cutting them off at the terminal edge

### Wire
- `TurnPauseReason::Question { tool_use_id, questions }` pauses a turn for AskUser; clients resolve it with `Op::QuestionResponse { turn_id, tool_use_id, reply }`, where `reply` is `Answered`, `Dismissed`, or `Discuss { message? }`. New payload types: `QuestionSpec`, `QuestionOption`, `QuestionAnswer`, `QuestionReply`

### Crates
- Public crates published as 0.2.0: ante-protocol-shape, ante-llm, ante-exec, ante-sdk

## Earlier releases

Notes for releases before v0.2.0 (`v0.preview.N`) are on the [GitHub releases page](https://github.com/AntigmaLabs/ante/releases).

