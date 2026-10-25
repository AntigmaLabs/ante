# Curated

A shared space for the team and community to exchange reusable Ante pieces: settings profiles, skills, and whatever proves useful enough to pass around.

Profiles (`<name>.settings.json`) are organized into folders by topic, skills under `skills/`.

## Profiles

Settings profiles for `ante --profile <name>`. Copy one to `~/.ante/`:

```sh
cp <name>.settings.json ~/.ante/
ante --profile <name>
```

Profiles are whole-file replacements: values a profile omits fall back to Ante defaults, not to your `settings.json`. Explicit CLI flags still override the profile. See the [preferences docs](https://docs.antigma.ai/configuration/preference#named-profiles).

### [pi](./pi/)

A pi-style agent (requires v0.preview.73+): four tools — Read, Write, Edit, Bash — and a concise replacement system prompt. Everything else routes through Bash:

- `rg` for file search (falling back to `grep`/`find`)
- `ante -p "<task>"` for one-shot subagents, `tmux` for persistent interactive agents (ante, claude, codex)
- small programs (`curl`, python, jq, sed/awk) for web fetch, web search, and large-scale edits

Skills, auto-memory, tips, and ambient predictions are off; the short prompt keeps tool descriptions compact. Sessions still save, so `/resume` works. No MCP servers, since the profile defines none.

The prompt's readable source is [`pi.system-prompt.md`](./pi/pi.system-prompt.md) — JSON strings can't hold real newlines, so the `system_prompt` field is generated from it. Edit the markdown, then regenerate:

```sh
jq --rawfile prompt pi.system-prompt.md \
  '.system_prompt = ($prompt | rtrimstr("\n"))' \
  pi.settings.json > tmp && mv tmp pi.settings.json
```

### [plan](./plan/)

A planning profile: the agent researches the codebase and delivers an implementation plan, but cannot edit. Write and Edit are absent from the tool set, so file changes are impossible, and an appended prompt keeps the model in planning posture. Permission mode is `strict`: known-safe read-only shell commands (`git log`, `git diff`, `rg`, and pipelines of vetted programs) run without prompts, while anything mutating asks. WebFetch and WebSearch are allowed by rule so research flows uninterrupted.

Plan under the profile, then execute without it — resuming a session restores the default tool set:

```sh
ante --profile plan       # produce the plan
ante                      # then /resume to execute with full tools
```

Headless runs imply yolo mode, which skips permission rules including the Bash ask; add `--exclude-tools Bash` for unattended planning. The profile omits `model` and `provider`, so it uses your default credentials. Pin a heavier model or effort by adding those fields to your copy.

### [web-search-MCP](./web-search-MCP/)

A collection of third-party web search providers, each with its MCP configuration, so that models without native web search support can reach the web. Each file follows the naming convention of `<provider>.settings.json`; copy one to `~/.ante/` and run `ante --profile <provider>`, or merge its `mcp_servers` entry into your own settings.

Based on their authentication requirements, the providers fall into the two groups listed below. Most of the listed providers offer a generous free-tier quota when you are logged in with an account. See the documentation for the default configuration used for each, the available knobs to tune for your use case, and CLI + agent skill alternatives.

- No authentication requirement

The `exa`, `parallel`, and `firecrawl` examples connect over Streamable HTTP, which requires Ante v0.2.12 or later. The `duckduckgo` example requires `uvx`.

| Provider | Link to documentation | Note |
| --- | --- | --- |
| [exa](./web-search-MCP/exa.settings.json) | [doc](https://exa.ai/docs/get-started/exa-mcp) | Rate-limited. |
| [parallel](./web-search-MCP/parallel.settings.json) | [doc](https://docs.parallel.ai/integrations/mcp/search-mcp) | Rate-limited. |
| [firecrawl](./web-search-MCP/firecrawl.settings.json) | [github](https://github.com/firecrawl/firecrawl-mcp-server) | A keyless connection gets a limited set of tools, `search` included. |
| [duckduckgo](./web-search-MCP/duckduckgo.settings.json) | [github](https://github.com/nickclyde/duckduckgo-mcp-server) | Unofficial, community maintained. No account or API key. |

- Authentication required

The examples show configurations with API key authentication. Substitute `<your-api-key>` with your actual key. Refer to the documentation if you want to use OAuth authentication instead.

The `tavily` example connects over Streamable HTTP, which requires Ante v0.2.12 or later.

| Provider | Link to documentation |
| --- | --- |
| [tavily](./web-search-MCP/tavily.settings.json) | [doc](https://docs.tavily.com/documentation/mcp) |
| [perplexity](./web-search-MCP/perplexity.settings.json) | [doc](https://docs.perplexity.ai/docs/getting-started/integrations/mcp-server) |
| [brave](./web-search-MCP/brave.settings.json) | [github](https://github.com/brave/brave-search-mcp-server) |

### [publish-MCP](./publish-MCP/)

MCP servers that put what the agent builds (an HTML report, a demo page, a static site from `dist/`) on a public URL, so you can open it on another device or send someone the link. Each file follows the `<provider>.settings.json` convention used above.

The `deed-page` example connects over Streamable HTTP, which requires Ante v0.2.12 or later. It needs no account or API key: the first `publish_site` call returns the site URL and an owner token, and the agent passes that token to `update_site`, `restore_version` or `delete_site` for the same site later. Anything published is public, so don't publish secrets or private data.

| Provider | Link to documentation | Note |
| --- | --- | --- |
| [deed-page](./publish-MCP/deed-page.settings.json) | [doc](https://deed.page/docs) | Keyless. Rate-limited, with size limits on the free plan. |

## Skills

[Agent Skills](https://agentskills.io) folders, each with a `SKILL.md`. Copy a folder into `~/.ante/skills/` for all projects, or `.ante/skills/` for one project. See the [skills docs](https://docs.antigma.ai/extend/skills).

## Contributing

Open a PR that adds your profile or skill. Include a line or two on what it does and when to reach for it: in the skill's `description` frontmatter, or a short section in this README for a profile. New categories are welcome; add a folder and explain it here.

Strip anything personal before submitting: API keys, tokens, absolute paths, private URLs.

Questions and ideas: [Discord](https://discord.gg/CbAsUR434B) or contact@antigma.ai.
