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

### plan

A planning profile: the agent researches the codebase and delivers an implementation plan, but cannot edit. Write and Edit are absent from the tool set, so file changes are impossible, and an appended prompt keeps the model in planning posture. Permission mode is `strict`: known-safe read-only shell commands (`git log`, `git diff`, `rg`, and pipelines of vetted programs) run without prompts, while anything mutating asks. WebFetch and WebSearch are allowed by rule so research flows uninterrupted.

Plan under the profile, then execute without it — resuming a session restores the default tool set:

```sh
ante --profile plan       # produce the plan
ante                      # then /resume to execute with full tools
```

Headless runs imply yolo mode, which skips permission rules including the Bash ask; add `--exclude-tools Bash` for unattended planning. The profile omits `model` and `provider`, so it uses your default credentials. Pin a heavier model or effort by adding those fields to your copy.

### parallel-search

Add free web search and page fetching with [Parallel Search MCP](https://docs.parallel.ai/integrations/mcp/search-mcp). Anonymous searches use Fast mode; no Parallel account or API key is required. The free endpoint is intended for exploration and light use and has rate limits.

Requires Ante v0.2.12 or later. The profile connects directly to Parallel's Streamable HTTP endpoint.

```sh
mkdir -p ~/.ante
cp curated/parallel-search.settings.json ~/.ante/
ante --profile parallel-search
```

In `/mcp`, check that `parallel` exposes `web_search` and `web_fetch`. Ask Ante to search for current documentation and fetch a page from the results. Headless runs can use the same profile:

```sh
ante --profile parallel-search -p "Search for Ante's MCP documentation, fetch the MCP page, and summarize its supported transport with a source link."
```

This profile adds `mcp__parallel__web_search` and `mcp__parallel__web_fetch` without restricting built-in tools or selecting a model. Named profiles replace the whole settings file, so your existing settings and other MCP servers are not inherited. To keep your current setup, merge just the `parallel` entry into the `mcp_servers` map in `~/.ante/settings.json` instead, then restart Ante. Configure your usual model/provider as needed.

### sem-code

Optional local code discovery using [Sem](https://github.com/Ataraxy-Labs/sem).
Install Sem following its upstream instructions (`sem --version`); this profile
uses its stdio MCP server and exposes only `sem_find` and `sem_grep` to Ante.
These provide symbol context and source search without selecting a model or
replacing built-in tools.

Copy `curated/sem-code.settings.json` into `~/.ante/`, then launch with an
explicit existing Git checkout:

```sh
SEM_REPO="$(git rev-parse --show-toplevel)" ante --profile sem-code
```

Run this from the checkout you want to inspect. In `/mcp`, check that `sem`
connects and lists `sem_find` and `sem_grep`. Ask for a known symbol using
`mcp__sem__sem_find`, then inspect matching source with `mcp__sem__sem_grep`.
The first query may create a local index. Sem needs no provider credentials;
Ante's model inference still needs your usual provider setup.

Named profiles replace the whole settings file rather than extending it. To
preserve your current setup, copy just the `sem` entry into your existing
`mcp_servers` map instead. If the launcher does not inherit your shell environment,
set `env.SEM_REPO` to your checkout path in your local copy. Use an absolute
`command` path if `sem` is not on Ante's PATH.

The tool allowlist limits what Ante exposes, not OS permissions. Sem may write
its index; `SEM_REPO` is not a sandbox. The full server also has validation tools
that can run commands, so retain the allowlist and use trusted checkouts.

## Skills

[Agent Skills](https://agentskills.io) folders, each with a `SKILL.md`. Copy a folder into `~/.ante/skills/` for all projects, or `.ante/skills/` for one project. See the [skills docs](https://docs.antigma.ai/extend/skills).

## Contributing

Open a PR that adds your profile or skill where it would live in `~/.ante/`. Include a line or two on what it does and when to reach for it: in the skill's `description` frontmatter, or a short section in this README for a profile. New categories are welcome; add a folder and explain it here.

Strip anything personal before submitting: API keys, tokens, absolute paths, private URLs.

Questions and ideas: [Discord](https://discord.gg/CbAsUR434B) or contact@antigma.ai.
