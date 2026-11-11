---
name: ante
description: >-
  Run Ante, a terminal coding agent, as a subagent: one-shot, in a tmux
  session the user can join, or as a server you drive with JSON. Use when the
  user asks to hand work to Ante, wants a second opinion from another model or
  provider, or wants a task run on a local model.
---

# Run Ante as a subagent

Ante starts with no knowledge of your conversation. Run it from the project directory and put the goal, constraints, and the output you want in the prompt. Check its work yourself before reporting it.

Pick the lightest mode that fits:

| Mode | Use it when |
|---|---|
| [Headless](#headless) | One task, one answer |
| [Interactive](#interactive) | The user wants to watch or step in |
| [Server](#server) | You want to approve tool calls, answer questions, or steer mid-turn |

In headless and interactive mode, `--model`, `--provider`, and `--effort` pick the model, `--offline-model` runs a local one, and `--tools Read,Glob,Grep` makes Ante read-only.

## Headless

```sh
ante -p "<task>"
```

This prints the answer and exits. Ante approves its own tool calls here, so it edits files and runs commands, and it cannot ask questions. To follow up in the same conversation, capture the session ID:

```sh
ante --output-format json -p "<task>" > run.jsonl
jq -rs 'map(.event.AgentMessage? // empty) | last' run.jsonl   # answer
jq -r '.event.SessionStart?.session_id // empty' run.jsonl     # session ID
ante --resume <session-id> -p "<follow-up>"
```

### Sensitive data

To keep a file's contents away from your own model, hand the file to Ante on a local model. Pass its path without reading it, and ask for a reply that is safe for you to see, because everything Ante prints comes back to you:

```sh
ANTE_TELEMETRY=off ante --offline-model --no-session-save --tools Read,Write \
  -p "Read tickets.csv. Write tickets.redacted.csv with the same rows and columns, replacing every person name, email address, and phone number with [NAME], [EMAIL], and [PHONE]. Reply with only the number of rows written."
```

`--offline-model` runs the default the user saved with `ante offline use`. `--tools Read,Write` limits Ante to file access, `--no-session-save` leaves no transcript, and `ANTE_TELEMETRY=off` stops log export. The [walkthrough](https://docs.antigma.ai/local/overview#local-subagent) explains the boundary.

## Interactive

Run the TUI in tmux so you and the user share one terminal:

```sh
tmux new-session -d -s ante -c "$PWD"
tmux send-keys -t ante 'ante' Enter
tmux send-keys -t ante -l '<task>'
tmux send-keys -t ante Enter
tmux capture-pane -p -t ante    # read the screen
```

Keep the task on one line; for a long brief, write it to a file and tell Ante to read it. The turn is still going while the screen shows `Running…`. Approval prompts and questions appear on screen: answer them with `send-keys`, or leave them to the user, who joins with `tmux attach -t ante`. Sending `/exit` ends the session and prints its `ante --resume` line.

## Server

`ante serve` speaks JSON lines: you send ops, it sends events. Keep it running in the background, fed from a file:

```sh
: > ops.jsonl
tail -n +1 -f ops.jsonl | ante serve --stdio > events.jsonl &
```

Append one op per line. Each op needs its own `id`, which is `op_` plus a 26-character ULID; zero-padded counters work:

```json
{"op":{"StartSession":{}},"id":"op_00000000000000000000000001"}
{"op":{"UserInput":"<task>"},"id":"op_00000000000000000000000002"}
```

Then read `events.jsonl`:

- `AgentMessage` holds the reply, and `TurnEnd` closes the turn. Send another `UserInput` to continue.
- `TurnPause` means Ante is waiting on you. For an approval, reply with the pause's `turn_id` and each tool's `id`. The decision is `Accept` or `Deny`, and a denial can carry a `message`:

  ```json
  {"op":{"ApprovalResponse":{"turn_id":"<turn_id>","responses":[{"tool_use_id":"<id>","decision":"Accept"}]}},"id":"op_00000000000000000000000003"}
  ```

  For a question, reply with a `QuestionResponse`.
- `{"op":{"Steer":"<guidance>"},"id":…}` redirects a running turn, and `{"op":"Interrupt","id":…}` stops it.

`StartSession` accepts `model`, `provider`, `effort`, `tools`, `cwd`, and `permission_mode` (`strict`, `auto`, or `yolo`). With `"unattended": true`, a call that needs approval is denied instead of pausing. Stop the background job to finish; Ante saves the session and exits when its input closes. The [protocol reference](https://docs.antigma.ai/reference/protocol-reference) lists every op and event.
