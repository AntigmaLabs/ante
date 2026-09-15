# mini-tui

A chat TUI on top of [`ante-sdk`](../../crates/ante-sdk) in one file,
[`src/main.rs`](src/main.rs): type a prompt, watch the reply stream in,
approve tool calls with `y`/`n`, Esc to interrupt, Ctrl+C to quit.

It needs a working `ante` on `PATH`; model and provider come from your
`~/.ante/settings.json`.

```sh
cd examples/mini-tui
cargo run
```
