# ante-acp

[Agent Client Protocol](https://agentclientprotocol.com) agent for
[Ante](https://github.com/AntigmaLabs/ante). ACP clients such as Zed and the
JetBrains IDEs launch it over stdio; it drives an installed `ante` binary.

Status: work in progress. It answers `initialize`, runs sessions
(`session/new`, `session/set_mode`, `session/cancel`), each on its own
`ante serve --stdio` child, and runs prompts: text, `@`-mentioned files,
embedded selections, and images reach Ante, and the reply streams back as
message and thought chunks. A prompt sent while a turn is running steers that
turn. Tool calls show up as they run, with their kind, the file they touch,
a diff for Edit and Write, progress lines, and their result. When Ante
needs approval for a call, the client gets a permission request with Allow,
Always allow in this session, and Reject; cancelling the turn denies it. Ante's
permission modes (strict, auto, yolo) appear as the session's modes,
starting from the user's settings.

Not yet: MCP servers and additional directories passed by the client are
ignored.

## Run

```bash
cargo install --path ante-acp
ante acp   # dispatches to ante-acp with ANTE pointing at that ante binary
```

Launched directly, `ante-acp` uses `--executable <PATH>`, else `ante` on `PATH`.

Zed, in `settings.json`:

```json
{ "agent_servers": { "ante": { "type": "custom", "command": "ante", "args": ["acp"] } } }
```

## Requirements

`ante` 0.2.1 or newer, checked with `ante --version` when the client sends
`initialize`. Set `ANTE_ACP_SKIP_VERSION_CHECK=1` to skip the check
(development builds of `ante` report `0.1.0`).

Logs go to stderr at `info`; set `RUST_LOG` to change the level.
