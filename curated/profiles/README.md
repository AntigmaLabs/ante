# Profiles

Settings profiles for `ante --profile <name>`. Copy one to `~/.ante/`:

```sh
cp minimal.settings.json ~/.ante/
ante --profile minimal
```

Profiles are whole-file replacements: values a profile omits fall back to Ante defaults, not to your `settings.json`. See the [preferences docs](https://docs.antigma.ai/configuration/preference#named-profiles).

## minimal

A quiet, low-context setup: short prompt on, tips and ambient predictions off, onboarding skipped. No MCP servers, since the profile defines none.

Tool choice is a CLI flag rather than a setting, so pass `--tools` to restrict the session to file operations and bash:

```sh
ante --profile minimal --tools Read Write Edit Glob Grep Bash
```

Add `--no-skills` to drop skills as well.

## bare (built in)

Ante ships a file-less `bare` profile: defaults with onboarding and ambient UI off, plus no skills, no MCP servers, no session saving, and no auto-memory. Nothing to copy here — the name is reserved, and a `bare.settings.json` would be ignored:

```sh
ante --profile bare
```
