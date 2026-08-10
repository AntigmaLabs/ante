# Profiles

Settings profiles for `ante --profile <name>`. Copy one to `~/.ante/`:

```sh
cp pi.settings.json ~/.ante/
ante --profile pi
```

Profiles are whole-file replacements: values a profile omits fall back to Ante defaults, not to your `settings.json`. Explicit CLI flags still override the profile. See the [preferences docs](https://docs.antigma.ai/configuration/preference#named-profiles).

## pi

A pi-style agent (requires v0.preview.73+): four tools — Read, Write, Edit, Bash — and a concise replacement system prompt. Everything else routes through Bash:

- `rg` for file search (falling back to `grep`/`find`)
- `ante -p "<task>"` for one-shot subagents, `tmux` for persistent interactive agents (ante, claude, codex)
- small programs (`curl`, python, jq, sed/awk) for web fetch, web search, and large-scale edits

Skills, auto-memory, tips, and ambient predictions are off; the short prompt keeps tool descriptions compact. Sessions still save, so `/resume` works. No MCP servers, since the profile defines none.

The prompt's readable source is `pi.system-prompt.md` — JSON strings can't hold real newlines, so the `system_prompt` field is generated from it. Edit the markdown, then regenerate:

```sh
jq --rawfile prompt pi.system-prompt.md \
  '.system_prompt = ($prompt | rtrimstr("\n"))' \
  pi.settings.json > tmp && mv tmp pi.settings.json
```

## bare (built in)

Ante ships a `bare` profile for stripped-down runs: onboarding and ambient UI off, plus no skills, no MCP servers, no session saving, and no auto-memory. Nothing to copy here — the first `ante --profile bare` run seeds `~/.ante/bare.settings.json`, and from then on it is editable like any other profile:

```sh
ante --profile bare
```
