# Contributing to Ante

The `ante` agent ships as a closed prebuilt binary under the [Binary Preview Terms](BINARY-TERMS.md). We are refactoring it and open-sourcing it piece by piece; [#21](https://github.com/AntigmaLabs/ante/issues/21) tracks the progress. Everything that is here is open under [Apache-2.0](LICENSE) and takes contributions.

For now, the most useful contribution is a [GitHub issue](https://github.com/AntigmaLabs/ante/issues/new/choose): bugs, rough edges, and what you wish Ante did. Issues shape what we open and fix next.

## What's open

| Component | Path | PRs |
|---|---|---|
| SDK, protocol, core primitives | [`crates/`](crates) | Ported by a maintainer |
| Harbor adapter | [`ante-harbor/`](ante-harbor) | Ported by a maintainer |
| ACP server | [`ante-acp/`](ante-acp) | Merged here |
| Slack and Discord gateway | [`examples/ante-gateway/`](examples/ante-gateway) | Merged here |
| Docs | [`docs-site/`](docs-site) | Merged here |
| Curated profiles and skills | [`curated/`](curated) | Merged here |
| Examples | [`examples/`](examples) | Merged here |

`crates/`, `ante-harbor/`, and `CHANGELOG.md` are synced from internal repositories, and the next sync overwrites direct edits. Open your PR here as usual. A maintainer ports it to the source repository, and it lands with the following sync.

## Bugs

Bugs in the agent belong in this repository's [issues](https://github.com/AntigmaLabs/ante/issues/new/choose), even though its source lives elsewhere. Attach the archive that `ante rage` writes: recent logs and your environment, with secrets redacted. Security problems go through [SECURITY.md](SECURITY.md).

## Build and test

Rust crates need Rust 1.88 or newer, and each one builds on its own:

```sh
cargo build --manifest-path ante-acp/Cargo.toml --all-targets
cargo test --manifest-path ante-acp/Cargo.toml
cargo fmt --manifest-path ante-acp/Cargo.toml --check
```

Harbor adapter:

```sh
uv run --no-project --python 3.12 --with "harbor==0.27.0" \
  python -m unittest discover -s ante-harbor -p "test_*.py" -v
```

Docs site, on Node 24. When you change a page, update its `zh-Hans` translation under `docs-site/i18n/` too.

```sh
cd docs-site
npm ci && npm run build
```

## Curated profiles

Before you open a PR for a profile in [`curated/`](curated):

- It holds no secrets: no API keys, tokens, absolute paths, or private URLs.
- `jq empty <name>.settings.json` passes.
- Its section in `curated/README.md` states the minimum Ante version.
- `ante doctor --profile <name>` is clean with the profile copied into `~/.ante/`.
- The PR discloses any affiliation with a vendor the profile points at.

## Pull requests

- Commit messages and PR titles follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). PRs are squash-merged.
- AI-assisted work is welcome. Say which tool you used, and submit only what you have read and run yourself.
- We aim to respond to issues and PRs within 3 business days.

<!-- TODO(team): confirm the response target and the AI-assisted contribution policy. -->

Questions go to [Discussions](https://github.com/AntigmaLabs/ante/discussions) or [Discord](https://discord.gg/CbAsUR434B). The [Code of Conduct](CODE_OF_CONDUCT.md) applies everywhere.
