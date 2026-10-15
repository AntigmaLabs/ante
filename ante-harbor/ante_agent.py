from __future__ import annotations

import json
import os
import re
import shlex
import tempfile
from collections.abc import Sequence
from pathlib import Path
from typing import Annotated, Any

from pydantic import Field, field_validator

from ante_events import (
    CACHE_CREATION_METADATA_KEY,
    EFFORT_METADATA_KEY,
    FAILURE_CLASS_METADATA_KEY,
    STEPS_METADATA_KEY,
    accumulate_usage_from_events,
    events_from_text,
    final_turn_failure,
    has_reported_usage,
    has_turn_end,
    is_number,
    read_event_log_text,
    resolved_model_effort_from_events,
    total_steps_from_events,
    trajectory_from_events,
)

from harbor.agents.capabilities import AgentCapabilities
from harbor.agents.installed.base import (
    AgentAuthenticationError,
    AgentSafetyRefusalError,
    ApiConnectionClosedError,
    ApiInternalServerError,
    ApiOverloadedError,
    ApiRateLimitError,
    ApiUsageLimitError,
    BaseInstalledAgent,
    ContextWindowExceededError,
    NetworkConnectionError,
    NonZeroAgentExitCodeError,
    UnknownApiError,
    with_prompt_template,
)
from harbor.agents.options import Cli, Env, InstalledAgentOptions
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext
from harbor.models.task.config import MCPServerConfig
from harbor.models.trial.result import AgentInfo, ModelInfo
from harbor.utils.trajectory_utils import format_trajectory_json


_AGENT_LOG = Path("/logs/agent/ante.txt")
_SETUP_LOG = _AGENT_LOG.parent / "setup" / "stdout.txt"
_INSTRUCTION_PATH = Path("/tmp/instruction.md")
_INSTALLER_URL = "https://download.ante.run/install.sh"
_INSTALLER_MAX_BYTES = 1024 * 1024

ENABLE_ATIF_ENV = "ANTE_ENABLE_ATIF"

# Extra Ante behavior flags used when the caller passes none. Model/provider are
# structured inputs owned by the adapter, never part of this string. run.py
# imports this so the orchestrator and adapter defaults cannot drift.
DEFAULT_ANTE_ARGS = "--yolo --output-format json --no-session-save --no-skills"

# Skills that ship inside the Ante binary. Evals measure the model plus Ante's
# base harness, so these stay hidden even when a task supplies its own skills.
# This list is a floor: the adapter also reads the bundled skill names from the
# installed binary, so a skill added by a newer Ante is hidden as well.
BUNDLED_ANTE_SKILLS = (
    "add-provider",
    "ante-guide",
    "resume-claude",
    "resume-codex",
    "skill-creator",
    "tmux",
)
# Bundled skills are embedded in the binary under ``skills/<name>/SKILL.md``.
_BUNDLED_SKILL_PATTERN = r"skills/[a-z0-9][a-z0-9-]*/SKILL\.md"

# Task-supplied skills are copied into Ante's user-level skill directory, which
# Ante discovers regardless of the session's working directory.
_ANTE_USER_SKILLS_DIR = "$HOME/.ante/skills"
_ANTE_USER_SETTINGS = "$HOME/.ante/settings.json"
_ANTE_USER_CATALOG = "$HOME/.ante/catalog.json"

# Stdlib helper that merges an uploaded JSON payload into an Ante config file
# inside the sandbox, uploaded next to Harbor's installed-agent files.
_MERGE_JSON_SOURCE = Path(__file__).with_name("ante_merge_json.py")
_MERGE_JSON_PATH = "/installed-agent/ante_merge_json.py"
_MCP_SETTINGS_PAYLOAD_PATH = "/installed-agent/ante-mcp-settings.json"
_CATALOG_PAYLOAD_PATH = "/installed-agent/ante-catalog.json"

# Ante reaches remote MCP servers natively over Streamable HTTP and does not
# implement the older HTTP+SSE transport. A task that declares a server as
# ``sse`` is reconfigured to Streamable HTTP only when its URL is listed here,
# exactly: each entry names an endpoint verified to serve both transports.
# Nothing else is rewritten, so an unverified ``sse`` server reaches Ante as
# declared and Ante reports the unsupported transport at startup.
VERIFIED_STREAMABLE_HTTP_ENDPOINTS = {
    # medical-claims-processing (terminal-bench v4.0.0): @playwright/mcp@0.0.64
    # routes /sse to legacy SSE and /mcp to Streamable HTTP on the same port.
    "http://playwright-mcp:3080/sse": "http://playwright-mcp:3080/mcp",
}

_HARBOR_ERROR_BY_EXCEPTION_KIND = {
    "rate_limited": ApiRateLimitError,
    "overloaded": ApiOverloadedError,
    "internal": ApiInternalServerError,
    "connection_closed": ApiConnectionClosedError,
    "network": NetworkConnectionError,
    "usage_limit": ApiUsageLimitError,
    "authentication": AgentAuthenticationError,
    "context_window_exceeded": ContextWindowExceededError,
    "safety_refusal": AgentSafetyRefusalError,
    "unknown_api": UnknownApiError,
}


def _metadata_dict(context: AgentContext) -> dict[str, Any] | None:
    metadata = getattr(context, "metadata", None)
    if isinstance(metadata, dict):
        return metadata
    try:
        context.metadata = {}
    except Exception:
        return None
    return context.metadata if isinstance(context.metadata, dict) else None


def _populate_context_from_events(
    context: AgentContext,
    events: list[dict[str, Any]],
    diagnostic_failure_class: str | None,
) -> None:
    """Translate Ante events into the small AgentContext metadata interface."""
    usage = accumulate_usage_from_events(events)
    if has_reported_usage(usage):
        for field in ("cost_usd", "n_input_tokens", "n_output_tokens", "n_cache_tokens"):
            value = usage.get(field)
            if is_number(value):
                setattr(context, field, value)

    metadata_fields: dict[str, Any] = {}
    if effort := resolved_model_effort_from_events(events):
        metadata_fields[EFFORT_METADATA_KEY] = effort
    cache_creation = usage.get("n_cache_creation_tokens") if usage else None
    if is_number(cache_creation):
        metadata_fields[CACHE_CREATION_METADATA_KEY] = cache_creation
    if (steps := total_steps_from_events(events)) is not None:
        metadata_fields[STEPS_METADATA_KEY] = steps
    if diagnostic_failure_class:
        metadata_fields[FAILURE_CLASS_METADATA_KEY] = diagnostic_failure_class
    if not metadata_fields:
        return

    metadata = _metadata_dict(context)
    if metadata is not None:
        metadata.update(metadata_fields)


def setup_log_command(command: str, *, append: bool = True) -> str:
    """Mirror setup output to Harbor Hub's conventional setup log path."""
    escaped_setup_dir = shlex.quote(str(_SETUP_LOG.parent))
    escaped_setup_path = shlex.quote(str(_SETUP_LOG))
    tee_args = f"-a {escaped_setup_path}" if append else escaped_setup_path
    return "\n".join(
        [
            f"mkdir -p {escaped_setup_dir}",
            "{",
            command,
            f"}} 2>&1 | tee {tee_args}",
        ]
    )


def install_command_from_args(install_args: str) -> str:
    """Build a bounded, retrying in-sandbox installer command."""
    quoted_args = " ".join(shlex.quote(arg) for arg in shlex.split(install_args or ""))
    execute = (
        'ANTE_INSTALL_DIR=/usr/local/bin NO_MODIFY_PATH=true bash -- "$installer_path"'
    )
    if quoted_args:
        execute = f"{execute} {quoted_args}"
    return "\n".join(
        [
            "set -eu",
            'installer_path="$(mktemp "${TMPDIR:-/tmp}/ante-install.XXXXXX")"',
            "trap 'rm -f \"$installer_path\"' EXIT",
            "curl --fail --silent --show-error --location \\",
            "  --retry 3 --retry-delay 1 --retry-max-time 120 \\",
            "  --connect-timeout 10 --max-time 120 \\",
            f'  --max-filesize {_INSTALLER_MAX_BYTES} --output "$installer_path" \\',
            f"  {shlex.quote(_INSTALLER_URL)}",
            execute,
        ]
    )


def split_extra_ante_args(ante_args: str) -> list[str]:
    """Split extra Ante flags and reject flags owned by structured inputs.

    ``--model``/``--provider``/``--effort`` have dedicated inputs; letting them
    ride in ``ante_args`` too would emit duplicate flags (Ante exits 2) and
    leave provenance recording only the structured value.
    """
    tokens = shlex.split(ante_args or "")
    reserved = ("--model", "--provider", "--effort")
    for token in tokens:
        if token in reserved or token.startswith(tuple(f"{flag}=" for flag in reserved)):
            raise ValueError(
                "ante_args must not include --model, --provider, or --effort; "
                "use model_name, provider, and effort instead"
            )
    return tokens


def with_task_skills(tokens: list[str], hidden_skills: Sequence[str]) -> list[str]:
    """Turn skill discovery on for task-supplied skills only.

    ``--no-skills`` would hide the copied task skills, so it becomes
    ``--skills`` plus an exclusion of every skill bundled with Ante.
    """
    kept = [token for token in tokens if token not in ("--no-skills", "--skills")]
    return [*kept, "--skills", "--exclude-skills", ",".join(hidden_skills)]


def ante_command(
    model_name: str,
    provider: str | None,
    effort: str | None,
    ante_args: str,
    *,
    hidden_skills: Sequence[str] | None = None,
) -> str:
    """Build the final Ante invocation run inside the sandbox.

    ``hidden_skills`` is set when task skills were registered: skills are then
    enabled with these names excluded.
    """
    args = ["ante", "--model", model_name]
    if provider:
        args += ["--provider", provider]
    if effort:
        args += ["--effort", effort]
    extra = split_extra_ante_args(ante_args)
    args += extra if hidden_skills is None else with_task_skills(extra, hidden_skills)
    command = " ".join(shlex.quote(arg) for arg in args)
    escaped_log_dir = shlex.quote(str(_AGENT_LOG.parent))
    escaped_log_path = shlex.quote(str(_AGENT_LOG))
    escaped_instruction_path = shlex.quote(str(_INSTRUCTION_PATH))
    return (
        f"trap 'rm -f {escaped_instruction_path}' EXIT && "
        f"mkdir -p {escaped_log_dir} && "
        "{ "
        f"{command} < {escaped_instruction_path} 2>&1 | tee {escaped_log_path}; "
        'exit "${PIPESTATUS[0]}"; '
        "}"
    )


def register_skills_command(skills_dir: str) -> str:
    """Copy a task's skills into Ante's user-level skill directory.

    The command prints the task's skill names, one per line.
    """
    source = skills_dir.rstrip("/")
    return (
        f'mkdir -p "{_ANTE_USER_SKILLS_DIR}" && '
        f'cp -R {shlex.quote(source + "/.")} "{_ANTE_USER_SKILLS_DIR}/" && '
        f"ls -1A {shlex.quote(source)}"
    )


def bundled_skills_command() -> str:
    """List the skill paths embedded in the installed Ante binary."""
    return f"LC_ALL=C grep -aoE '{_BUNDLED_SKILL_PATTERN}' \"$(command -v ante)\" | sort -u"


def bundled_skill_names(output: str) -> set[str]:
    """Parse ``bundled_skills_command`` output into skill names."""
    return {
        match.group(0).split("/")[1] for match in re.finditer(_BUNDLED_SKILL_PATTERN, output)
    }


def mcp_settings(servers: list[MCPServerConfig]) -> dict[str, Any]:
    """Map Harbor MCP server declarations to Ante's ``mcp_servers`` settings.

    Stdio servers launch directly and Streamable HTTP servers are reached at
    their URL. An ``sse`` declaration becomes Streamable HTTP only through
    ``VERIFIED_STREAMABLE_HTTP_ENDPOINTS``; otherwise it is written as
    declared, which Ante refuses with a message naming the transport.
    """
    entries: dict[str, dict[str, Any]] = {}
    for server in servers:
        if server.transport == "stdio":
            entries[server.name] = {"command": server.command, "args": list(server.args)}
            continue
        transport, url = server.transport, server.url
        if transport == "sse" and url in VERIFIED_STREAMABLE_HTTP_ENDPOINTS:
            transport, url = "streamable-http", VERIFIED_STREAMABLE_HTTP_ENDPOINTS[url]
        entries[server.name] = {"transport": transport, "url": url}
    return {"mcp_servers": entries}


def remote_mcp_servers(servers: list[MCPServerConfig]) -> list[MCPServerConfig]:
    """The declared servers Ante reaches over the network rather than stdio."""
    return [server for server in servers if server.transport != "stdio"]


def legacy_sse_servers(servers: list[MCPServerConfig]) -> list[MCPServerConfig]:
    """The ``sse`` declarations with no verified Streamable HTTP endpoint."""
    return [
        server
        for server in servers
        if server.transport == "sse" and server.url not in VERIFIED_STREAMABLE_HTTP_ENDPOINTS
    ]


def remote_mcp_support_command() -> str:
    """Succeed only if the installed Ante knows the Streamable HTTP transport.

    Native support arrived in Ante 0.2.12. An older Ante skips a
    ``settings.json`` server entry it cannot parse with a log line and nothing
    else, so a remote server would be missing from the run without any error.
    The transport's settings name is read from the binary, the way bundled
    skills are.
    """
    return 'LC_ALL=C grep -aq -- "streamable-http" "$(command -v ante)"'


def write_json_command(target: str, payload_path: str) -> str:
    """Write an uploaded JSON payload into an Ante config file under ``$HOME``.

    A fresh sandbox has no such file, so the payload is copied in place and no
    interpreter is needed. Merging into an existing file needs ``python3``;
    without it the command fails with a message that names the file.
    """
    quoted = f'"{target}"'
    return (
        f"mkdir -p \"$(dirname {quoted})\" && "
        f"if [ ! -s {quoted} ]; then cp {payload_path} {quoted}; "
        "elif command -v python3 >/dev/null 2>&1; then "
        f"python3 {_MERGE_JSON_PATH} {quoted} {payload_path}; "
        f"else echo 'python3 not found: cannot merge into {target}'; exit 127; fi"
    )


def register_mcp_command() -> str:
    """Write the uploaded MCP payload into Ante's user settings file."""
    return write_json_command(_ANTE_USER_SETTINGS, _MCP_SETTINGS_PAYLOAD_PATH)


def catalog_overlay(
    provider: str,
    openrouter_routing: dict[str, Any] | None,
    stream_idle_timeout_secs: int | None,
) -> dict[str, Any] | None:
    """Build a partial ``catalog.json`` that tunes one built-in provider.

    Ante merges a user catalog entry onto the built-in provider of the same id,
    so only the overridden fields are written. ``openrouter_routing`` becomes
    the request body's OpenRouter ``provider`` object (``order``, ``only``,
    ``ignore``, ``quantizations``, ``sort``, ``allow_fallbacks``, ...).
    """
    fields: dict[str, Any] = {}
    if openrouter_routing:
        fields["extra_body"] = {"provider": openrouter_routing}
    if stream_idle_timeout_secs is not None:
        fields["stream_idle_timeout_secs"] = stream_idle_timeout_secs
    return {"providers": {provider: fields}} if fields else None


def missing_catalog_overrides(catalog_output: str, overlay: dict[str, Any]) -> list[str]:
    """Return the overlay fields that ``ante catalog`` does not report as loaded.

    Ante ignores catalog fields it does not know, so an older build would
    otherwise run with its default routing and timeouts without any error.
    """
    try:
        start = catalog_output.index("{")
        loaded, _ = json.JSONDecoder().raw_decode(catalog_output[start:])
        providers = {entry["id"]: entry for entry in loaded["providers"]}
    except (ValueError, KeyError, TypeError):
        return ["<unreadable `ante catalog` output>"]
    missing = []
    for provider, fields in overlay["providers"].items():
        entry = providers.get(provider, {})
        missing += [
            f"{provider}.{name}" for name, value in fields.items() if entry.get(name) != value
        ]
    return missing


async def upload_instruction(environment: BaseEnvironment, instruction: str) -> None:
    """Upload the rendered instruction as a file for a cleaner command."""
    with tempfile.TemporaryDirectory() as temp_dir:
        source_path = Path(temp_dir) / _INSTRUCTION_PATH.name
        source_path.write_text(instruction, encoding="utf-8")
        await environment.upload_file(source_path, str(_INSTRUCTION_PATH))


class AnteOptions(InstalledAgentOptions):
    provider: Annotated[str | None, Cli("--provider")] = None
    reasoning_effort: Annotated[str | None, Cli("--effort")] = None
    enable_atif: Annotated[
        bool | None, Env(ENABLE_ATIF_ENV, fallback=ENABLE_ATIF_ENV)
    ] = False
    ante_args: str | None = Field(
        default=None,
        description=f"Extra Ante flags; defaults to {DEFAULT_ANTE_ARGS!r}.",
    )
    install_command: str | None = Field(
        default=None,
        description="Command that installs Ante inside the sandbox.",
    )
    install_args: str | None = Field(
        default=None,
        description="Arguments to Ante's published install.sh; empty uses its default.",
    )
    openrouter_routing: dict[str, Any] | None = Field(
        default=None,
        description=(
            "OpenRouter provider-routing object (JSON) sent as the request body's "
            "`provider` field, e.g. {\"order\": [\"xiaomi\"], \"allow_fallbacks\": false}."
        ),
    )
    stream_idle_timeout_secs: int | None = Field(
        default=None,
        gt=0,
        description="Seconds a model stream may stay silent before Ante reconnects.",
    )

    @field_validator("prompt_template_path", mode="before")
    @classmethod
    def normalize_prompt_template_path(cls, value: Any) -> Any:
        return str(value) if isinstance(value, Path) else value

    @field_validator("ante_args")
    @classmethod
    def validate_ante_args(cls, value: str | None) -> str | None:
        split_extra_ante_args(value or "")
        return value

    @field_validator("openrouter_routing", mode="before")
    @classmethod
    def parse_openrouter_routing(cls, value: Any) -> Any:
        """Accept the JSON text Harbor passes for ``--ak openrouter_routing=...``."""
        if isinstance(value, str):
            value = json.loads(value) if value.strip() else None
        if value is not None and not isinstance(value, dict):
            raise ValueError("openrouter_routing must be a JSON object")
        return value


def supported_capabilities(**flags: bool) -> AgentCapabilities:
    """Declare only the capability fields this Harbor version defines.

    ``AgentCapabilities`` forbids unknown fields. Harbor 0.24 added ``skills``
    and ``mcp_servers`` and rejects trials that need them when they are unset;
    0.23 has neither field and passes skills and MCP servers unconditionally.
    """
    fields = AgentCapabilities.model_fields
    return AgentCapabilities(**{name: on for name, on in flags.items() if name in fields})


class AnteAgent(BaseInstalledAgent):
    capabilities = supported_capabilities(atif=True, skills=True, mcp_servers=True)
    options_model = AnteOptions
    options: AnteOptions
    _INSTALL_VERSION_COMMAND = "ante --version"

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        # Successful exec output is authoritative over a possibly partial
        # downloaded ante.txt. Timeouts leave this unset and use the file.
        self._event_output: str | None = None

    @staticmethod
    def name() -> str:
        return "ante"

    def to_agent_info(self) -> AgentInfo:
        """Report the provider and model consumed by Ante without re-parsing them."""
        return AgentInfo(
            name=self.name(),
            version=self.version() or "unknown",
            model_info=(
                ModelInfo(name=self.model_name, provider=self.options.provider or None)
                if self.model_name
                else None
            ),
        )

    async def _installed_ante_matches_requested_version(
        self, environment: BaseEnvironment
    ) -> bool:
        if self._version is None:
            return False

        version_command = self.get_version_command()
        if not version_command:
            return False

        version_result = await environment.exec(command=version_command)
        if version_result.return_code != 0:
            return False

        installed_version = self.parse_version(version_result.stdout or "")
        return installed_version == self._version

    async def install(self, environment: BaseEnvironment) -> None:
        if await self._installed_ante_matches_requested_version(environment):
            self.logger.debug("Ante is already available at the requested version")
            return

        install_command = self.options.install_command or None
        if install_command is None and self.options.install_args is not None:
            install_command = install_command_from_args(self.options.install_args)

        if install_command is not None:
            # Provision ante from within the sandbox: a raw command, or install.sh
            # for a published build. No runner-built binary required.
            await self.ensure_system_dependencies(environment, ("curl", "bash"))
            await self.exec_as_root(
                environment, command=setup_log_command(install_command, append=False)
            )
        else:
            # Default mode: upload the binary staged next to this file by the
            # caller.
            binary_path = Path(__file__).parent / "ante"
            if not binary_path.exists():
                raise FileNotFoundError(f"Ante binary not found at: {binary_path}")
            await environment.upload_file(
                source_path=binary_path,
                target_path="/usr/local/bin/ante",
            )
            await self.exec_as_root(
                environment,
                command=setup_log_command("chmod +x /usr/local/bin/ante", append=False),
            )

        version_command = self.get_version_command()
        if version_command:
            version_result = await self.exec_as_root(
                environment, command=setup_log_command(version_command, append=True)
            )
            if self._version is None and (version_result.stdout or "").strip():
                self._version = self.parse_version(version_result.stdout)

    def get_version_command(self) -> str | None:
        return self._INSTALL_VERSION_COMMAND

    def parse_version(self, stdout: str) -> str:
        text = stdout.strip()
        for line in text.splitlines():
            parts = line.strip().split()
            if parts:
                return parts[1] if parts[0] == "ante" and len(parts) >= 2 else parts[0]
        return text

    def _classify_exec_error(self, command: str, result: Any) -> NonZeroAgentExitCodeError:
        """Classify a failed Ante process from its final structured turn.

        Harbor calls this hook only after an exec returns nonzero. Exceptions
        raised by Harbor's own timeout/cancellation boundary bypass it.

        A final ``TurnEnd(Error)`` is authoritative and maps through Ante's kind
        table. A run whose final ``TurnEnd`` completed failed for a reason the
        model did not report, so it stays a plain exit-code error rather than
        letting recovered-turn text drive classification. Only output with no
        ``TurnEnd`` at all defers to Harbor's maintained free-text patterns.
        """
        output = f"{result.stdout or ''}\n{result.stderr or ''}"
        events = events_from_text(output)
        failure = final_turn_failure(events)
        if failure is None:
            if not has_turn_end(events):
                return super()._classify_exec_error(command, result)
            detail = (
                f"Command failed (exit {result.return_code}): {command}\n"
                f"stdout: {self._truncate_output(result.stdout)}\n"
                f"stderr: {self._truncate_output(result.stderr)}"
            )
            return NonZeroAgentExitCodeError(detail)

        detail = (
            f"Command failed (exit {result.return_code}): {command}\n"
            f"Ante TurnEnd(Error):\n{self._truncate_output(failure.detail_text)}"
        )
        error_type = (
            _HARBOR_ERROR_BY_EXCEPTION_KIND.get(failure.exception_kind)
            if failure.exception_kind
            else None
        )
        return (error_type or NonZeroAgentExitCodeError)(detail)

    async def _register_task_extensions(
        self, environment: BaseEnvironment
    ) -> tuple[str, ...] | None:
        """Expose task-supplied skills and MCP servers to Ante.

        Returns the skills to hide when task skills were registered, else None.

        Registration is best effort, as in Harbor's own adapters: a failed step
        is logged and the run continues without that extension. It calls
        ``environment.exec`` directly so a failure is not classified as an
        Ante process error. Steps run as the agent user, which may not be able
        to append to the root-owned setup log, so output is logged here instead.
        """
        hidden_skills = None
        if self.skills_dir:
            output = await self._register_step(
                environment, "skills", register_skills_command(self.skills_dir)
            )
            if output is not None:
                hidden_skills = await self._hidden_skills(environment, output.split())
        if self.mcp_servers:
            await self._check_remote_mcp_support(environment)
            await environment.upload_file(_MERGE_JSON_SOURCE, _MERGE_JSON_PATH)
            await self._upload_config_text(
                environment,
                content=json.dumps(mcp_settings(self.mcp_servers)),
                remote_path=_MCP_SETTINGS_PAYLOAD_PATH,
                filename=Path(_MCP_SETTINGS_PAYLOAD_PATH).name,
            )
            await self._register_step(environment, "MCP servers", register_mcp_command())
        return hidden_skills

    async def _check_remote_mcp_support(self, environment: BaseEnvironment) -> None:
        """Stop the trial when the installed Ante cannot reach a remote server.

        A task that declares a remote server needs it, and an Ante without
        native Streamable HTTP support would drop the entry silently and run
        the task without it. Unverified ``sse`` declarations are warned about
        here and still registered as declared, so Ante's startup output names
        the unsupported transport instead of the server simply being absent.
        """
        remote = remote_mcp_servers(self.mcp_servers)
        if not remote:
            return
        probe = await environment.exec(command=remote_mcp_support_command())
        if probe.return_code != 0:
            names = ", ".join(server.name for server in remote)
            raise RuntimeError(
                f"This Ante build has no native remote MCP support, so the task's remote "
                f"MCP servers ({names}) cannot be registered; use Ante 0.2.12 or newer, "
                "which accepts `url` entries in `mcp_servers`."
            )
        for server in legacy_sse_servers(self.mcp_servers):
            self.logger.warning(
                f"MCP server {server.name} declares the legacy SSE transport at "
                f"{server.url}, which Ante does not implement and which has no verified "
                "Streamable HTTP endpoint; it is registered as declared, so Ante reports "
                "the refusal at startup and its tools stay unavailable."
            )

    async def _apply_catalog_overlay(self, environment: BaseEnvironment) -> None:
        """Write the requested provider overrides into Ante's user catalog.

        Unlike task extensions, these are run settings the operator asked for,
        so a failure stops the trial rather than silently measuring a
        different configuration.
        """
        if not (self.options.openrouter_routing or self.options.stream_idle_timeout_secs):
            return
        if not self.options.provider:
            raise ValueError(
                "openrouter_routing and stream_idle_timeout_secs need an explicit provider"
            )
        overlay = catalog_overlay(
            self.options.provider,
            self.options.openrouter_routing,
            self.options.stream_idle_timeout_secs,
        )
        await environment.upload_file(_MERGE_JSON_SOURCE, _MERGE_JSON_PATH)
        await self._upload_config_text(
            environment,
            content=json.dumps(overlay),
            remote_path=_CATALOG_PAYLOAD_PATH,
            filename=Path(_CATALOG_PAYLOAD_PATH).name,
        )
        result = await environment.exec(
            command=write_json_command(_ANTE_USER_CATALOG, _CATALOG_PAYLOAD_PATH)
        )
        if result.return_code != 0:
            output = f"{result.stdout or ''}{result.stderr or ''}".strip()
            raise RuntimeError(
                f"Could not write Ante catalog overrides (exit {result.return_code}): "
                f"{self._truncate_output(output) or '<none>'}"
            )
        loaded = await environment.exec(command="ante catalog")
        if missing := missing_catalog_overrides(loaded.stdout or "", overlay):
            raise RuntimeError(
                "This Ante build did not load the requested catalog overrides "
                f"({', '.join(missing)}); upgrade Ante or drop the overrides."
            )

    async def _register_step(
        self, environment: BaseEnvironment, label: str, command: str
    ) -> str | None:
        """Run one registration step; return its stdout, or None on failure."""
        result = await environment.exec(command=command)
        output = f"{result.stdout or ''}{result.stderr or ''}".strip()
        if result.return_code != 0:
            self.logger.warning(
                f"Could not register task {label} with Ante "
                f"(exit {result.return_code}); continuing without them. "
                f"Output: {self._truncate_output(output) or '<none>'}"
            )
            return None
        self.logger.debug(f"Registered task {label} with Ante: {output}")
        return result.stdout or ""

    async def _hidden_skills(
        self, environment: BaseEnvironment, task_skills: list[str]
    ) -> tuple[str, ...]:
        """Bundled skills to exclude: the known list plus any found in the binary."""
        result = await environment.exec(command=bundled_skills_command())
        found = bundled_skill_names(result.stdout or "") if result.return_code == 0 else set()
        if not found:
            self.logger.warning(
                "Could not read bundled skills from the Ante binary; "
                "hiding the known bundled skills only."
            )
        hidden = tuple(sorted(set(BUNDLED_ANTE_SKILLS) | found))
        if shadowed := sorted(set(task_skills) & set(hidden)):
            self.logger.warning(
                f"Task skills {', '.join(shadowed)} share a name with a bundled Ante "
                "skill and are excluded with it."
            )
        return hidden

    def populate_context_post_run(self, context: AgentContext) -> None:
        log_text = read_event_log_text(Path(self.logs_dir))
        # A successful exec captured the complete stream even when tee could
        # only write a prefix. Timeouts never set _event_output, so their
        # downloaded on-disk prefix remains the fallback for partial metrics.
        event_text = self._event_output or log_text or ""
        self._event_output = None
        events = events_from_text(event_text)
        failure = final_turn_failure(events)

        _populate_context_from_events(
            context, events, failure.failure_class if failure else None
        )

        if self.options.enable_atif:
            if events:
                try:
                    trajectory = trajectory_from_events(
                        events,
                        agent_name=self.name(),
                        agent_version=self.version() or "unknown",
                        model_name=self.model_name,
                    )
                except Exception:
                    self.logger.exception("Failed to convert Ante events to trajectory")
                else:
                    if trajectory:
                        trajectory_path = Path(self.logs_dir) / "trajectory.json"
                        try:
                            trajectory_path.write_text(
                                format_trajectory_json(trajectory.to_json_dict()),
                                encoding="utf-8",
                            )
                            self.logger.debug(f"Wrote Ante trajectory to {trajectory_path}")
                        except OSError as exc:
                            self.logger.debug(
                                f"Failed to write trajectory file {trajectory_path}: {exc}"
                            )

    @with_prompt_template
    async def run(
        self, instruction: str, environment: BaseEnvironment, context: AgentContext
    ) -> None:
        self._event_output = None

        model_name = self.model_name or os.environ.get("MODEL_NAME")
        if not model_name:
            raise RuntimeError("model must be set (pass `-m <name>` or set MODEL_NAME)")
        # ante reads API keys / MODEL_* from the container env that Harbor
        # populates from --ae/--agent-env. The instruction is uploaded as a
        # file so the command only needs shell escaping for flags and paths.
        await upload_instruction(environment, instruction)
        await self._apply_catalog_overlay(environment)
        hidden_skills = await self._register_task_extensions(environment)
        command = ante_command(
            model_name,
            self.options.provider,
            self.options.reasoning_effort,
            DEFAULT_ANTE_ARGS if self.options.ante_args is None else self.options.ante_args,
            hidden_skills=hidden_skills,
        )

        result = await self.exec_as_agent(environment, command=command)
        # tee mirrors the event stream to stdout. A successful exec's captured
        # stream is authoritative because the downloaded log may be truncated
        # by a tee write failure.
        self._event_output = getattr(result, "stdout", "") or ""
