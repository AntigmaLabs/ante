# ante-acp

[Agent Client Protocol](https://agentclientprotocol.com) agent for
[Ante](https://github.com/AntigmaLabs/ante). ACP clients such as Zed and the
JetBrains IDEs launch it over stdio; it drives an installed `ante` binary.

Status: work in progress. It answers `initialize` today; sessions and prompts
follow.

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
