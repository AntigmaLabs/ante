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

Examples here assume Ante v0.3.0+.

See the [Ante MCP documentation](https://docs.antigma.ai/extend/mcp) for more info.

- No authentication requirement

| Provider | Link to documentation | Note |
| --- | --- | --- |
| [exa](./web-search-MCP/exa.settings.json) | [doc](https://exa.ai/docs/get-started/exa-mcp) | Rate-limited. |
| [parallel](./web-search-MCP/parallel.settings.json) | [doc](https://docs.parallel.ai/integrations/mcp/search-mcp) | Rate-limited. |
| [firecrawl](./web-search-MCP/firecrawl.settings.json) | [github](https://github.com/firecrawl/firecrawl-mcp-server) | A keyless connection gets a limited set of tools, `search` included. |
| [duckduckgo](./web-search-MCP/duckduckgo.settings.json) | [github](https://github.com/nickclyde/duckduckgo-mcp-server) | Unofficial, community maintained. No account or API key. Requires `uvx`. |

- Authentication required

The examples show configurations with API key authentication. Substitute `<your-api-key>` with your actual key. For a remote server, you may use `${KEY_NAME}` as a header value to read the key from the environment, so secrets stay out of the settings file. If you want to use OAuth authentication instead, omit the headers entry and run `ante auth login <provider>` (with `--profile <provider>` if you kept it as a separate profile) after adding the server to your `settings.json`.

| Provider | Link to documentation |
| --- | --- |
| [tavily](./web-search-MCP/tavily.settings.json) | [doc](https://docs.tavily.com/documentation/mcp) |
| [perplexity](./web-search-MCP/perplexity.settings.json) | [doc](https://docs.perplexity.ai/docs/getting-started/integrations/mcp-server) |
| [brave](./web-search-MCP/brave.settings.json) | [github](https://github.com/brave/brave-search-mcp-server) |

### [rea](./rea/)

A reverse-engineering agent backed by [REA](https://github.com/morluto/rea), an MCP server that inspects native binaries, Electron/JavaScript apps, .NET assemblies, APKs, and websites, returning decompiled code with the evidence behind each finding. The profile registers REA's stdio server and appends a short prompt that keeps conclusions tied to that evidence.

```sh
cp rea/rea.settings.json ~/.ante/
npx skills add morluto/rea --skill reverse-engineer-anything   # optional workflow skill, lands in ~/.agents/skills/
ante --profile rea
```

Then ask about any binary, package, or app on disk.

Requires Node.js 22.19+ for `npx`. Native decompilation also needs Hopper, Ghidra, or IDA (see REA's [installation guide](https://github.com/morluto/rea/blob/main/docs/installation.md)); JavaScript and .NET inspection need none. With Ghidra (`brew install ghidra` on macOS), add its paths to the server entry so REA finds them:

```json
"env": {
  "GHIDRA_INSTALL_DIR": "/opt/homebrew/opt/ghidra/libexec",
  "JAVA_HOME": "/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home",
  "REA_GHIDRA_STARTUP_TIMEOUT_MS": "3600000"
}
```

Ghidra's first analysis of a large binary can outlast both REA's 330-second startup default and Ante's 10-minute MCP call deadline; raise the latter with `ANTE_MCP_TOOL_TIMEOUT=3600`.

The server is pinned to one `rea-agents` release, as REA recommends. Bump the version in `args` to upgrade. To keep your own model and settings instead of using a profile, merge the `mcp_servers.rea` entry into `~/.ante/settings.json`.

#### Ante decompiles Ante

Don't trust, verify. The Ante binary is closed source, so point Ante at itself and read what comes out. This walkthrough decompiles the first 200 functions of the v0.3.4 macOS arm64 release. It takes about 20 minutes, nearly all of it Ghidra's one-time analysis.

1. Download the release and confirm the binary's hash:

   ```sh
   mkdir verify && cd verify
   gh release download v0.3.4 --repo AntigmaLabs/ante --pattern 'ante-v0.3.4-darwin-arm64.tar.gz'
   tar xzf ante-v0.3.4-darwin-arm64.tar.gz
   shasum -a 256 ante
   # 426cd33670bd89c8f2fcf5b070fbcf7cb63ce2d6672d9b5a92e9b8407b84f9c0
   ```

2. Ask Ante to decompile it. The profile needs the Ghidra `env` block above, with Ghidra 12.1.4:

   ```sh
   ANTE_MCP_TOOL_TIMEOUT=3600 ante --profile rea \
     -p "Open $PWD/ante with the REA Ghidra provider, then run decompile_all with layout address, limit 200, output directory $PWD/decompiled. Do not write files yourself. Report the manifest counts."
   ```

The pseudocode lands in `decompiled/functions/`, one file per function, and `decompiled/decompile-index.json` records the binary hash, the Ghidra version, and a SHA-256 per function. The model only issues the tool calls; REA and Ghidra write the files.

Our own run of these steps is attached to the release as [`decompiled-ante-v0.3.4-darwin-arm64.tar.gz`](https://github.com/AntigmaLabs/ante/releases/download/v0.3.4/decompiled-ante-v0.3.4-darwin-arm64.tar.gz), as an example of the output and something to diff yours against. With the same Ghidra version, the function files come out byte-identical.

Drop `limit 200` to export all of the roughly 28,000 functions (about 280MB and 18 hours on a 24-core Mac), or add `resume true` to continue an earlier run. From there, ask Ante questions about what you exported, such as which functions reference network hosts.

The output is decompiled pseudocode with generated names like `FUN_1000110e4`. It shows what the binary does, and it is not the original Rust source.

## Skills

[Agent Skills](https://agentskills.io) folders, each with a `SKILL.md`. Copy a folder into `~/.ante/skills/` for all projects, or `.ante/skills/` for one project. See the [skills docs](https://docs.antigma.ai/extend/skills).

[`ante`](./skills/ante/) is for other agents: it teaches them to run Ante as a subagent, one-shot, in a shared tmux session, or through `ante serve`. Copy it into that agent's skills folder instead, such as `~/.claude/skills/` for Claude Code or `~/.agents/skills/`.

## Contributing

Open a PR that adds your profile or skill. Include a line or two on what it does and when to reach for it: in the skill's `description` frontmatter, or a short section in this README for a profile. New categories are welcome; add a folder and explain it here.

Strip anything personal before submitting: API keys, tokens, absolute paths, private URLs.

Questions and ideas: [Discord](https://discord.gg/CbAsUR434B) or contact@antigma.ai.
