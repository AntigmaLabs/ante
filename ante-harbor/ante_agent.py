from __future__ import annotations

import os
import shlex
import tempfile
from pathlib import Path
from typing import Any

from ante_events import (
    CACHE_CREATION_METADATA_KEY,
    EFFORT_METADATA_KEY,
    FAILURE_CLASS_METADATA_KEY,
    STEPS_METADATA_KEY,
    accumulate_usage_from_events,
    events_from_text,
    final_turn_failure,
    has_reported_usage,
    is_number,
    read_event_log_text,
    resolved_model_effort_from_events,
    total_steps_from_events,
    trajectory_from_events,
)

from harbor.agents.installed.base import (
    AgentAuthenticationError,
    AgentSafetyRefusalError,
    ApiConnectionClosedError,
    ApiInternalServerError,
    ApiOverloadedError,
    ApiRateLimitError,
    ApiUsageLimitError,
    BaseInstalledAgent,
    CliFlag,
    ContextWindowExceededError,
    EnvVar,
    NetworkConnectionError,
    NonZeroAgentExitCodeError,
    UnknownApiError,
    with_prompt_template,
)
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext
from harbor.utils.trajectory_utils import format_trajectory_json


_AGENT_LOG = Path("/logs/agent/ante.txt")
_SETUP_LOG = _AGENT_LOG.parent / "setup" / "stdout.txt"
_INSTRUCTION_PATH = Path("/tmp/instruction.md")

ENABLE_ATIF_ENV = "ANTE_ENABLE_ATIF"

# Extra Ante behavior flags used when the caller passes none. Model/provider are
# structured inputs owned by the adapter, never part of this string. run.py
# imports this so the orchestrator and adapter defaults cannot drift.
DEFAULT_ANTE_ARGS = "--yolo --output-format json --check"

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

    effort = resolved_model_effort_from_events(events)
    cache_creation = usage.get("n_cache_creation_tokens") if usage else None
    steps = total_steps_from_events(events)
    if (
        not effort
        and not is_number(cache_creation)
        and steps is None
        and not diagnostic_failure_class
    ):
        return

    metadata = _metadata_dict(context)
    if metadata is None:
        return
    if effort:
        metadata[EFFORT_METADATA_KEY] = effort
    if is_number(cache_creation):
        metadata[CACHE_CREATION_METADATA_KEY] = cache_creation
    if steps is not None:
        metadata[STEPS_METADATA_KEY] = steps
    if diagnostic_failure_class:
        metadata[FAILURE_CLASS_METADATA_KEY] = diagnostic_failure_class


def install_prerequisites_command() -> str:
    """Best-effort sandbox prerequisites for published or custom installs."""
    return "\n".join(
        [
            "set -e",
            "if ! command -v curl >/dev/null 2>&1 || ! command -v bash >/dev/null 2>&1; then",
            "  if command -v apt-get >/dev/null 2>&1; then",
            "    apt-get update && "
            "DEBIAN_FRONTEND=noninteractive apt-get install -y curl ca-certificates bash",
            "  elif command -v apk >/dev/null 2>&1; then",
            "    apk add --no-cache curl ca-certificates bash",
            "  elif command -v yum >/dev/null 2>&1; then",
            "    yum install -y curl ca-certificates bash",
            "  elif command -v dnf >/dev/null 2>&1; then",
            "    dnf install -y curl ca-certificates bash",
            "  else",
            "    echo 'curl or bash missing and no supported package manager found' >&2",
            "    exit 127",
            "  fi",
            "fi",
            "command -v curl >/dev/null",
            "command -v bash >/dev/null",
        ]
    )


def with_install_prerequisites(command: str) -> str:
    return "\n".join([install_prerequisites_command(), command])


def _tee_command(command: str, log_path: Path, *, append: bool) -> str:
    escaped_log_dir = shlex.quote(str(log_path.parent))
    escaped_log_path = shlex.quote(str(log_path))
    tee_args = f"-a {escaped_log_path}" if append else escaped_log_path
    return "\n".join(
        [
            f"mkdir -p {escaped_log_dir} || exit",
            "status_file=$(mktemp) || exit",
            "trap 'rm -f \"$status_file\"' EXIT",
            "{",
            f"  sh -c {shlex.quote(command)}",
            "  command_status=$?",
            "  printf '%s\\n' \"$command_status\" > \"$status_file\" || exit",
            f"}} 2>&1 | tee {tee_args}",
            "tee_status=$?",
            "command_status=$(cat \"$status_file\") || exit",
            "case \"$command_status\" in ''|*[!0-9]*) exit 1 ;; esac",
            "if [ \"$command_status\" -ne 0 ]; then exit \"$command_status\"; fi",
            "exit \"$tee_status\"",
        ]
    )


def setup_log_command(command: str, *, append: bool = True) -> str:
    """Mirror setup output to Harbor Hub's conventional setup log path."""
    logged_command = "\n".join(["set -e", command])
    return _tee_command(logged_command, _SETUP_LOG, append=append)


def install_command_from_args(install_args: str) -> str:
    """Build a robust in-sandbox install.sh command for published Ante builds."""
    quoted_args = " ".join(shlex.quote(arg) for arg in shlex.split(install_args or ""))
    install_line = "\n".join(
        [
            "installer=$(mktemp)",
            "trap 'rm -f \"$installer\"' EXIT",
            "curl -fsSL https://download.ante.run/install.sh -o \"$installer\"",
            (
                "ANTE_INSTALL_DIR=/usr/local/bin NO_MODIFY_PATH=true "
                f'bash "$installer" {quoted_args}'
            ).rstrip(),
        ]
    )
    return with_install_prerequisites(install_line)


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


def ante_command(
    model_name: str,
    provider: str | None,
    effort: str | None,
    ante_args: str,
) -> str:
    """Build the final Ante invocation run inside the sandbox."""
    args = ["ante", "--model", model_name]
    if provider:
        args += ["--provider", provider]
    if effort:
        args += ["--effort", effort]
    args += split_extra_ante_args(ante_args)
    command = " ".join(shlex.quote(arg) for arg in args)
    escaped_instruction_path = shlex.quote(str(_INSTRUCTION_PATH))
    logged_command = "\n".join(
        [
            f"trap 'rm -f {escaped_instruction_path}' EXIT",
            f"{command} < {escaped_instruction_path}",
        ]
    )
    return _tee_command(logged_command, _AGENT_LOG, append=False)


async def upload_instruction(environment: BaseEnvironment, instruction: str) -> None:
    """Upload the rendered instruction as a file for a cleaner command."""
    with tempfile.TemporaryDirectory() as temp_dir:
        source_path = Path(temp_dir) / _INSTRUCTION_PATH.name
        source_path.write_text(instruction, encoding="utf-8")
        await environment.upload_file(source_path, str(_INSTRUCTION_PATH))


class AnteAgent(BaseInstalledAgent):
    SUPPORTS_ATIF: bool = True
    _INSTALL_VERSION_COMMAND = "ante --version"
    CLI_FLAGS = [
        CliFlag("provider", cli="--provider", type="str"),
        CliFlag("reasoning_effort", cli="--effort", type="str"),
    ]
    ENV_VARS = [
        EnvVar(
            "enable_atif",
            env=ENABLE_ATIF_ENV,
            type="bool",
            default=False,
            env_fallback=ENABLE_ATIF_ENV,
        ),
    ]

    def __init__(
        self,
        *args,
        ante_args: str | None = None,
        install_command: str | None = None,
        install_args: str | None = None,
        **kwargs,
    ) -> None:
        super().__init__(*args, **kwargs)
        self._ante_args = DEFAULT_ANTE_ARGS if ante_args is None else ante_args
        self._provider = self._resolved_flags.get("provider") or None
        self._effort = self._resolved_flags.get("reasoning_effort") or None
        self._enable_atif = self.resolve_env_vars().get(ENABLE_ATIF_ENV) == "true"
        # Optional in-sandbox install command (e.g. an install.sh one-liner). When
        # set, install() runs it instead of uploading a runner-built binary, so the
        # same adapter works for both PR builds and published nightly/release
        # builds. It must leave `ante` on PATH for the shared --version check.
        self._install_command = install_command or None
        self._install_args = install_args
        # Success-path fallback retained only until Harbor downloads ante.txt.
        # Event parsing and translation stay in populate_context_post_run.
        self._event_output: str | None = None

    @staticmethod
    def name() -> str:
        return "ante"

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

        if self._install_command is not None:
            # Reuse mode: provision ante from within the sandbox (e.g. download a
            # published build via install.sh). No runner-built binary required.
            await self.exec_as_root(
                environment,
                command=setup_log_command(
                    with_install_prerequisites(self._install_command), append=False
                ),
            )
        elif self._install_args is not None:
            await self.exec_as_root(
                environment,
                command=setup_log_command(
                    install_command_from_args(self._install_args), append=False
                ),
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
        """Classify a failed Ante process from its final structured turn only.

        Harbor calls this hook only after an exec returns nonzero. Exceptions
        raised by Harbor's own timeout/cancellation boundary bypass it.
        """
        output = f"{result.stdout or ''}\n{result.stderr or ''}"
        failure = final_turn_failure(events_from_text(output))
        if failure is None:
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

    def populate_context_post_run(self, context: AgentContext) -> None:
        log_text = read_event_log_text(Path(self.logs_dir))
        event_text = log_text or self._event_output or ""
        self._event_output = None
        events = events_from_text(event_text)
        failure = final_turn_failure(events)

        _populate_context_from_events(
            context, events, failure.failure_class if failure else None
        )

        if self._enable_atif:
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
        command = ante_command(model_name, self._provider, self._effort, self._ante_args)

        result = await self.exec_as_agent(environment, command=command)
        # tee mirrors the event stream to stdout. Harbor normally downloads the
        # log before populating context; retain stdout only as a download fallback.
        self._event_output = getattr(result, "stdout", "") or ""
