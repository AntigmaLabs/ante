# Curated

A shared space for the team and community to exchange reusable Ante pieces: settings profiles, skills, and whatever proves useful enough to pass around.

The layout mirrors `~/.ante/`, so installing a piece is a straight copy: profiles (`<name>.settings.json`) sit at the top level, skills under `skills/`.

## Profiles

Settings profiles for `ante --profile <name>`. Copy one to `~/.ante/`:

```sh
cp pi.settings.json ~/.ante/
ante --profile pi
```

Profiles are whole-file replacements: values a profile omits fall back to Ante defaults, not to your `settings.json`. Explicit CLI flags still override the profile. See the [preferences docs](https://docs.antigma.ai/configuration/preference#named-profiles).

### pi

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

## Skills

[Agent Skills](https://agentskills.io) folders, each with a `SKILL.md`. Copy a folder into `~/.ante/skills/` for all projects, or `.ante/skills/` for one project. See the [skills docs](https://docs.antigma.ai/extend/skills).

## Contributing

Open a PR that adds your profile or skill where it would live in `~/.ante/`. Include a line or two on what it does and when to reach for it: in the skill's `description` frontmatter, or a short section in this README for a profile. New categories are welcome; add a folder and explain it here.

Strip anything personal before submitting: API keys, tokens, absolute paths, private URLs.

Questions and ideas: [Discord](https://discord.gg/CbAsUR434B) or contact@antigma.ai.
