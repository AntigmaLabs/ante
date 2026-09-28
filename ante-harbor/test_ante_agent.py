"""Unit tests for the Ante adapter against Harbor's real model interfaces."""
from __future__ import annotations

import asyncio
import json
import os
import shlex
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import ante_agent
import ante_events
from harbor.agents.installed.base import (
    AgentAuthenticationError,
    AgentSafetyRefusalError,
    ApiConnectionClosedError,
    ApiInternalServerError,
    ApiOverloadedError,
    ApiRateLimitError,
    ApiUsageLimitError,
    ContextWindowExceededError,
    NetworkConnectionError,
    NonZeroAgentExitCodeError,
    UnknownApiError,
)
from harbor.environments.docker.docker import DockerEnvironment
from harbor.models.agent.context import AgentContext


FIXTURES_DIR = Path(__file__).parent / "fixtures"

ERROR_KIND_CASES = {
    "rate_limited": ("rate_limited", "rate_limited", ApiRateLimitError),
    "timeout": ("timeout", "unknown_api", UnknownApiError),
    "overloaded": ("model_error", "overloaded", ApiOverloadedError),
    "server_error": ("model_error", "internal", ApiInternalServerError),
    "transport": ("model_error", "network", NetworkConnectionError),
    "unexpected_eof": (
        "model_error",
        "connection_closed",
        ApiConnectionClosedError,
    ),
    "malformed_response": ("model_error", "unknown_api", UnknownApiError),
    "auth": ("model_error", "authentication", AgentAuthenticationError),
    "oauth": ("model_error", "authentication", AgentAuthenticationError),
    "forbidden": ("model_error", "unknown_api", UnknownApiError),
    "quota": ("model_error", "usage_limit", ApiUsageLimitError),
    "context_overflow": (
        "model_error",
        "context_window_exceeded",
        ContextWindowExceededError,
    ),
    "invalid_request": ("model_error", "unknown_api", UnknownApiError),
    "content_policy": ("model_error", "safety_refusal", AgentSafetyRefusalError),
    "unknown": ("model_error", "unknown_api", UnknownApiError),
    "future_structured_kind": ("model_error", "unknown_api", UnknownApiError),
}


def fixture_events(name: str) -> list[dict]:
    text = (FIXTURES_DIR / name).read_text(encoding="utf-8")
    return [json.loads(line) for line in text.splitlines() if line.strip()]


class FakeEnvironment:
    def __init__(self):
        self.source_text = None
        self.target_path = None

    async def upload_file(self, source_path, target_path):
        self.source_text = source_path.read_text(encoding="utf-8")
        self.target_path = target_path


class FakeExecResult:
    def __init__(self, return_code=0, stdout="", stderr=""):
        self.return_code = return_code
        self.stdout = stdout
        self.stderr = stderr


class FakeInstallEnvironment:
    def __init__(self, *results):
        self.results = list(results)
        self.commands = []
        self.uploads = []

    async def exec(self, command, **kwargs):
        self.commands.append((command, kwargs))
        if not self.results:
            raise AssertionError(f"unexpected command: {command}")
        return self.results.pop(0)

    async def upload_file(self, source_path, target_path):
        self.uploads.append((source_path, target_path))


class HarborStreamingTests(unittest.IsolatedAsyncioTestCase):
    async def test_stream_collector_preserves_large_event_and_unterminated_tail(self):
        prefix = '{"event":{"AgentMessage":"'
        suffix = '"}}'
        message_size = 200 * 1024 - len(prefix) - len(suffix)
        event = prefix + "x" * message_size + suffix
        tail = "unterminated tail"
        script = (
            "import sys\n"
            f"sys.stdout.write({prefix!r} + 'x' * {message_size} + "
            f"{suffix!r} + '\\n' + {tail!r})\n"
        )
        process = await asyncio.create_subprocess_exec(
            sys.executable,
            "-c",
            script,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.STDOUT,
        )
        output = []

        async def on_output(line, stream):
            output.append((line, stream))

        result = await DockerEnvironment._collect_streamed_output(
            process, timeout_sec=5, on_output=on_output
        )

        self.assertEqual(output, [(event + "\n", "stdout"), (tail, "stdout")])
        self.assertEqual(result.stdout, event + "\n" + tail)
        self.assertIsNone(result.stderr)
        self.assertEqual(result.return_code, 0)


class FinalTurnFailureTests(unittest.TestCase):
    def test_normalizes_archive_and_harbor_failure_policy_once(self):
        events = [
            {
                "event": {
                    "TurnEnd": {
                        "status": {
                            "Error": {
                                "kind": "rate_limited",
                                "headline": "rate limited",
                                "details": ["HTTP 429 Too Many Requests"],
                            }
                        }
                    }
                }
            }
        ]

        failure = ante_events.final_turn_failure(events)

        self.assertIsNotNone(failure)
        self.assertEqual(failure.failure_class, "rate_limited")
        self.assertEqual(failure.exception_kind, "rate_limited")
        self.assertIn("kind: rate_limited", failure.detail_text)
        self.assertIn("HTTP 429 Too Many Requests", failure.detail_text)

    def test_final_completed_turn_clears_an_earlier_failure(self):
        events = [
            {
                "event": {
                    "TurnEnd": {
                        "status": {
                            "Error": {
                                "kind": "overloaded",
                                "headline": "overloaded",
                                "details": [],
                            }
                        }
                    }
                }
            },
            {"event": {"TurnEnd": {"status": "Completed"}}},
        ]

        self.assertIsNone(ante_events.final_turn_failure(events))

    def test_turn_kinds_preserve_archive_and_exception_policy(self):
        for kind, (failure_class, exception_kind, _) in ERROR_KIND_CASES.items():
            with self.subTest(kind=kind):
                failure = ante_events.final_turn_failure(
                    [{"event": {"TurnEnd": {"status": {"Error": {"kind": kind}}}}}]
                )

                self.assertIsNotNone(failure)
                self.assertEqual(
                    (failure.failure_class, failure.exception_kind),
                    (failure_class, exception_kind),
                )

    def test_dedicated_harbor_failures_remain_model_errors_in_fallback_reports(self):
        for error_type in (
            AgentAuthenticationError,
            AgentSafetyRefusalError,
            ContextWindowExceededError,
        ):
            with self.subTest(error_type=error_type.__name__):
                self.assertEqual(
                    ante_events.fallback_failure_class(
                        error_type.__name__, "model call failed", None
                    ),
                    "model_error",
                )


class InstallCommandTests(unittest.TestCase):
    def test_setup_log_command_writes_harbor_setup_stdout(self):
        command = ante_agent.setup_log_command("echo setup", append=False)

        self.assertIn("mkdir -p /logs/agent/setup", command)
        self.assertIn("echo setup", command)
        self.assertIn("2>&1 | tee /logs/agent/setup/stdout.txt", command)
        self.assertNotIn("tee -a", command)

    def test_setup_log_command_can_append(self):
        command = ante_agent.setup_log_command("ante --version")

        self.assertIn("ante --version", command)
        self.assertIn("2>&1 | tee -a /logs/agent/setup/stdout.txt", command)

    def test_install_command_passes_args_without_package_manager_logic(self):
        command = ante_agent.install_command_from_args("nightly")

        self.assertIn("https://download.ante.run/install.sh", command)
        self.assertIn("--retry 3", command)
        self.assertIn("--retry-max-time 120", command)
        self.assertIn("--connect-timeout 10", command)
        self.assertIn("--max-time 120", command)
        self.assertIn("--max-filesize 1048576", command)
        self.assertIn('--output "$installer_path"', command)
        self.assertIn('bash -- "$installer_path" nightly', command)
        self.assertNotIn("curl -fsSL", command)
        self.assertNotIn("| bash", command)
        for package_manager in ("apt-get", "apk", "yum", "dnf"):
            self.assertNotIn(package_manager, command)

    def test_install_command_bounds_transient_curl_retries(self):
        command = ante_agent.install_command_from_args("nightly")

        self.assertIn("--retry 3", command)
        self.assertIn("--retry-delay 1", command)
        self.assertIn("--retry-max-time 120", command)

    def test_install_args_are_shell_quoted(self):
        args = "https://example.com/build manifest.json"
        command = ante_agent.install_command_from_args(args)
        quoted = " ".join(shlex.quote(arg) for arg in shlex.split(args))

        self.assertIn(f'bash -- "$installer_path" {quoted}', command)

    def test_empty_install_args_use_install_script_default(self):
        command = ante_agent.install_command_from_args("")

        self.assertIn('bash -- "$installer_path"', command)
        self.assertNotIn('bash -- "$installer_path" ', command)

    def test_truncated_installer_is_not_executed(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            marker = root / "executed"
            stub_curl = bin_dir / "curl"
            stub_curl.write_text(
                "#!/bin/sh\n"
                "while [ \"$#\" -gt 0 ]; do\n"
                "  if [ \"$1\" = --output ]; then\n"
                "    output=$2\n"
                "    shift 2\n"
                "  else\n"
                "    shift\n"
                "  fi\n"
                "done\n"
                "printf '%s\\n' '#!/bin/sh' "
                f"'touch {shlex.quote(str(marker))}' > \"$output\"\n"
                "exit 18\n",
                encoding="utf-8",
            )
            stub_curl.chmod(0o755)
            env = os.environ.copy()
            env["PATH"] = f"{bin_dir}{os.pathsep}{env.get('PATH', '')}"
            env["TMPDIR"] = str(root)

            result = subprocess.run(
                ["bash", "-c", ante_agent.install_command_from_args("nightly")],
                capture_output=True,
                check=False,
                env=env,
                text=True,
                timeout=5,
            )

            self.assertEqual(result.returncode, 18)
            self.assertFalse(marker.exists())
            self.assertEqual(list(root.glob("ante-install.*")), [])


class AnteInstallTests(unittest.TestCase):
    def test_install_skips_when_requested_version_is_already_available(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(logs_dir=Path(d), version="1.2.3")
            environment = FakeInstallEnvironment(FakeExecResult(stdout="ante 1.2.3\n"))

            asyncio.run(agent.install(environment))

            self.assertEqual(environment.commands, [("ante --version", {})])
            self.assertEqual(environment.uploads, [])

    def test_install_args_validate_and_capture_version_without_precheck(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(logs_dir=Path(d), install_args="nightly")
            environment = FakeInstallEnvironment(
                FakeExecResult(), FakeExecResult(stdout="ante 20260627\n")
            )

            with mock.patch.object(
                agent, "ensure_system_dependencies", new=mock.AsyncMock()
            ) as ensure_system_dependencies:
                asyncio.run(agent.install(environment))

            ensure_system_dependencies.assert_awaited_once_with(
                environment, ("curl", "bash")
            )
            self.assertEqual(len(environment.commands), 2)
            self.assertIn("download.ante.run/install.sh", environment.commands[0][0])
            self.assertNotEqual(environment.commands[0][0], "ante --version")
            self.assertIn("ante --version", environment.commands[1][0])
            self.assertEqual(agent.version(), "20260627")

    def test_custom_install_command_ensures_system_dependencies(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(
                logs_dir=Path(d), install_command="custom installer"
            )
            environment = FakeInstallEnvironment(
                FakeExecResult(), FakeExecResult(stdout="ante 20260627\n")
            )

            with mock.patch.object(
                agent, "ensure_system_dependencies", new=mock.AsyncMock()
            ) as ensure_system_dependencies:
                asyncio.run(agent.install(environment))

            ensure_system_dependencies.assert_awaited_once_with(
                environment, ("curl", "bash")
            )
            self.assertIn("custom installer", environment.commands[0][0])

    def test_unmatched_text_without_turn_end_stays_exit_code_error(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(logs_dir=Path(d))
            result = FakeExecResult(
                return_code=1,
                stderr="fixture says HTTP 429 and timeout but no terminal event",
            )

            error = agent._classify_exec_error("ante", result)

            self.assertIs(type(error), NonZeroAgentExitCodeError)
            self.assertIn("HTTP 429", str(error))

    def test_output_without_turn_end_defers_to_harbor_text_patterns(self):
        # No TurnEnd means Ante never reported a structured verdict (installer
        # failure, crash before the first turn settled), so Harbor's maintained
        # ERROR_PATTERNS classify the free text. The first two needles were
        # added in Harbor 0.22.0.
        cases = {
            "service_unavailable": (
                "litellm.ServiceUnavailableError: upstream returned 503",
                ApiOverloadedError,
            ),
            "anthropic_prompt_overflow": (
                "prompt is too long: 210000 tokens > 200000 maximum",
                ContextWindowExceededError,
            ),
            "installer_curl_failure": (
                "curl: (6) Could not resolve host: download.ante.run",
                NetworkConnectionError,
            ),
        }
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(logs_dir=Path(d))
            for name, (stderr, error_type) in cases.items():
                with self.subTest(case=name):
                    result = FakeExecResult(return_code=1, stderr=stderr)

                    error = agent._classify_exec_error("ante", result)

                    self.assertIs(type(error), error_type)

    def test_structured_failure_kinds_use_harbor_classification_hook(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(logs_dir=Path(d))
            for kind, (_, _, error_type) in ERROR_KIND_CASES.items():
                with self.subTest(kind=kind):
                    result = FakeExecResult(
                        return_code=1,
                        stdout=json.dumps({
                            "event": {
                                "TurnEnd": {
                                    "turn_id": "turn-1",
                                    "status": {
                                        "Error": {
                                            "kind": kind,
                                            "headline": "model call failed",
                                            "details": [],
                                        }
                                    },
                                }
                            }
                        }),
                    )

                    error = agent._classify_exec_error("ante", result)

                    self.assertIs(type(error), error_type)

    def test_recovered_turn_error_does_not_classify_the_process(self):
        # The recovered turn's "rate limited" text would match Harbor's
        # ``rate.?limit`` pattern, so this also proves a completed final
        # TurnEnd blocks the free-text fallback.
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(logs_dir=Path(d))
            events = [
                {
                    "event": {
                        "TurnEnd": {
                            "turn_id": "turn-1",
                            "status": {
                                "Error": {
                                    "kind": "rate_limited",
                                    "headline": "rate limited",
                                    "details": [],
                                }
                            },
                        }
                    }
                },
                {
                    "event": {
                        "TurnEnd": {
                            "turn_id": "turn-2",
                            "status": "Completed",
                        }
                    }
                },
            ]
            result = FakeExecResult(
                return_code=1,
                stdout="\n".join(json.dumps(event) for event in events),
            )

            error = agent._classify_exec_error("ante", result)

            self.assertIsInstance(error, NonZeroAgentExitCodeError)
            self.assertNotIsInstance(error, ApiRateLimitError)

    def test_structured_model_failure_stays_typed(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(logs_dir=Path(d))
            result = FakeExecResult(
                return_code=1,
                stdout=json.dumps({
                    "event": {
                        "TurnEnd": {
                            "turn_id": "turn-1",
                            "status": {
                                "Error": {
                                    "kind": "invalid_request",
                                    "headline": "invalid request",
                                    "details": ["HTTP 400 Bad Request"],
                                }
                            },
                        }
                    }
                }),
            )

            error = agent._classify_exec_error("ante", result)

            self.assertIsInstance(error, UnknownApiError)

    def test_harbor_timeout_and_cancellation_bypass_exec_classification(self):
        class RaisingEnvironment:
            def __init__(self, error):
                self.error = error

            async def exec(self, **_kwargs):
                raise self.error

        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(logs_dir=Path(d))

            with self.assertRaises(asyncio.TimeoutError):
                asyncio.run(
                    agent.exec_as_agent(
                        RaisingEnvironment(asyncio.TimeoutError()), command="ante"
                    )
                )

            async def cancelled_exec():
                await agent.exec_as_agent(
                    RaisingEnvironment(asyncio.CancelledError()), command="ante"
                )

            with self.assertRaises(asyncio.CancelledError):
                asyncio.run(cancelled_exec())


class AnteOptionsTests(unittest.TestCase):
    def test_preflight_accepts_the_existing_options_and_declares_atif(self):
        kwargs = {
            "provider": "openrouter",
            "reasoning_effort": "high",
            "enable_atif": "true",
            "ante_args": "--yolo --output-format json",
            "install_command": "custom installer",
            "install_args": "nightly",
            "version": "1.2.3",
            "prompt_template_path": "/tmp/prompt.j2",
        }

        ante_agent.AnteAgent.preflight(kwargs)

        self.assertTrue(ante_agent.AnteAgent.capabilities.atif)
        self.assertFalse(ante_agent.AnteAgent.capabilities.native_config)
        schema = ante_agent.AnteAgent.options_schema()
        self.assertFalse(schema["additionalProperties"])
        self.assertTrue(kwargs.keys() <= schema["properties"].keys())

    def test_unknown_option_is_rejected_at_preflight_and_construction(self):
        with self.assertRaisesRegex(ValueError, "Unknown option 'enable_attif'"):
            ante_agent.AnteAgent.preflight({"enable_attif": True})
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaisesRegex(ValueError, "Unknown option 'enable_attif'"):
                ante_agent.AnteAgent(logs_dir=Path(d), enable_attif=True)

    def test_preflight_rejects_invalid_types_and_reserved_or_malformed_flags(self):
        for kwargs, message in (
            ({"enable_atif": "sometimes"}, "enable_atif"),
            ({"install_args": ["nightly"]}, "install_args"),
            ({"ante_args": "--model duplicate"}, "must not include"),
            ({"ante_args": "--provider=duplicate"}, "must not include"),
            ({"ante_args": "--effort high"}, "must not include"),
            ({"ante_args": "--flag 'unterminated"}, "No closing quotation"),
        ):
            with self.subTest(kwargs=kwargs):
                with self.assertRaisesRegex(ValueError, message):
                    ante_agent.AnteAgent.preflight(kwargs)

    def test_default_and_explicit_empty_flags_and_install_args_are_preserved(self):
        default_flags = " --yolo --output-format json --no-session-save --no-skills"
        for options, flags, install_args in (
            ({}, default_flags, None),
            ({"ante_args": None, "install_args": None}, default_flags, None),
            ({"ante_args": "", "install_args": ""}, "", ""),
            (
                {"ante_args": "--yolo --check", "install_args": "nightly"},
                " --yolo --check",
                "nightly",
            ),
        ):
            with self.subTest(options=options), tempfile.TemporaryDirectory() as d:
                agent = ante_agent.AnteAgent(
                    logs_dir=Path(d), model_name="test/model", **options
                )
                environment = FakeInstallEnvironment(FakeExecResult())

                asyncio.run(agent.run("test instruction", environment, AgentContext()))

                self.assertEqual(len(environment.commands), 1)
                self.assertIn(
                    f"ante --model test/model{flags} < /tmp/instruction.md",
                    environment.commands[0][0],
                )
                self.assertEqual(agent.options.install_args, install_args)

    def test_path_prompt_template_is_accepted_by_preflight_and_constructor(self):
        with tempfile.TemporaryDirectory() as d:
            template = Path(d) / "prompt.j2"
            ante_agent.AnteAgent.preflight({"prompt_template_path": template})

            for args, kwargs in (
                ((), {"logs_dir": Path(d), "prompt_template_path": template}),
                ((Path(d), template), {}),
            ):
                with self.subTest(positional=bool(args)):
                    agent = ante_agent.AnteAgent(*args, **kwargs)

                    self.assertEqual(agent._prompt_template_path, template)
                    self.assertEqual(agent.options.prompt_template_path, str(template))

    def test_openrouter_agent_info_keeps_full_model_and_runtime_provider(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(
                logs_dir=Path(d),
                model_name="deepseek/deepseek-v4-flash-0731",
                provider="openrouter",
            )

            model_info = agent.to_agent_info().model_info

            self.assertIsNotNone(model_info)
            self.assertEqual(model_info.name, "deepseek/deepseek-v4-flash-0731")
            self.assertEqual(model_info.provider, "openrouter")

    def test_direct_deepseek_agent_info_uses_runtime_provider(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(
                logs_dir=Path(d),
                model_name="deepseek-v4-flash",
                provider="deepseek",
            )

            model_info = agent.to_agent_info().model_info

            self.assertIsNotNone(model_info)
            self.assertEqual(model_info.name, "deepseek-v4-flash")
            self.assertEqual(model_info.provider, "deepseek")

    def test_provider_and_atif_kwargs_use_typed_options(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(
                logs_dir=Path(d),
                provider="openrouter",
                enable_atif="true",
            )

            self.assertEqual(agent.build_cli_flags(), "--provider openrouter")
            self.assertTrue(agent.options.enable_atif)

    def test_reasoning_effort_kwarg_maps_to_ante_effort_flag(self):
        with tempfile.TemporaryDirectory() as d:
            agent = ante_agent.AnteAgent(
                logs_dir=Path(d),
                reasoning_effort="high",
            )

            self.assertEqual(agent.build_cli_flags(), "--effort high")
            self.assertEqual(agent.options.reasoning_effort, "high")

    def test_atif_env_fallback_uses_typed_option(self):
        with tempfile.TemporaryDirectory() as d:
            with mock.patch.dict(os.environ, {"ANTE_ENABLE_ATIF": "true"}):
                agent = ante_agent.AnteAgent(logs_dir=Path(d))

            self.assertTrue(agent.options.enable_atif)

    def test_atif_explicit_option_and_agent_env_precede_process_fallback(self):
        with tempfile.TemporaryDirectory() as d:
            with mock.patch.dict(os.environ, {"ANTE_ENABLE_ATIF": "true"}):
                env_agent = ante_agent.AnteAgent(
                    logs_dir=Path(d), extra_env={"ANTE_ENABLE_ATIF": "false"}
                )
                explicit_agent = ante_agent.AnteAgent(
                    logs_dir=Path(d),
                    extra_env={"ANTE_ENABLE_ATIF": "false"},
                    enable_atif=True,
                )
                disabled_agent = ante_agent.AnteAgent(
                    logs_dir=Path(d), enable_atif=False
                )
                null_agent = ante_agent.AnteAgent(logs_dir=Path(d), enable_atif=None)

            self.assertFalse(env_agent.options.enable_atif)
            self.assertTrue(explicit_agent.options.enable_atif)
            self.assertFalse(disabled_agent.options.enable_atif)
            self.assertFalse(null_agent.options.enable_atif)


class AnteCommandTests(unittest.TestCase):
    def test_upload_instruction_uses_file_transfer_target(self):
        environment = FakeEnvironment()
        instruction = "do 'the' task\nwith $SHELL chars && pipes | untouched"

        asyncio.run(ante_agent.upload_instruction(environment, instruction))

        self.assertEqual(environment.source_text, instruction)
        self.assertEqual(environment.target_path, "/tmp/instruction.md")

    def test_command_renders_model_provider_and_extra_args(self):
        command = ante_agent.ante_command(
            "deepseek/model",
            "openrouter",
            "high",
            "--yolo --check",
        )

        self.assertIn(
            "ante --model deepseek/model --provider openrouter --effort high --yolo --check",
            command,
        )
        self.assertIn("< /tmp/instruction.md", command)
        self.assertIn("trap 'rm -f /tmp/instruction.md' EXIT", command)
        self.assertNotIn("do the task", command)
        self.assertNotIn("ante -p", command)
        self.assertNotIn("--prompt", command)
        self.assertEqual(command.count("--model"), 1)
        self.assertEqual(command.count("--provider"), 1)
        self.assertNotIn("set -o pipefail", command)
        self.assertIn('exit "${PIPESTATUS[0]}"', command)
        self.assertIn("tee /logs/agent/ante.txt", command)

    def test_generated_command_returns_ante_status_not_tee_status(self):
        for ante_exit, tee_exit in ((1, 0), (0, 1)):
            with (
                self.subTest(ante_exit=ante_exit, tee_exit=tee_exit),
                tempfile.TemporaryDirectory() as d,
            ):
                root = Path(d)
                bin_dir = root / "bin"
                bin_dir.mkdir()
                instruction_path = root / "instruction.md"
                instruction_path.write_text("run this task\n", encoding="utf-8")
                log_path = root / "logs" / "agent" / "ante.txt"
                event_text = json.dumps({
                    "event": {
                        "TurnEnd": {
                            "turn_id": "turn-1",
                            "status": "Completed",
                            "steps": 1,
                        }
                    }
                })
                stub_ante = bin_dir / "ante"
                stub_ante.write_text(
                    "#!/usr/bin/env bash\n"
                    f"printf '%s\\n' {shlex.quote(event_text)}\n"
                    f"exit {ante_exit}\n",
                    encoding="utf-8",
                )
                stub_ante.chmod(0o755)
                stub_tee = bin_dir / "tee"
                stub_tee.write_text(
                    "#!/usr/bin/env bash\n"
                    "payload=$(/bin/cat)\n"
                    "printf '%s\\n' \"$payload\" > \"${@: -1}\"\n"
                    "printf '%s\\n' \"$payload\"\n"
                    f"exit {tee_exit}\n",
                    encoding="utf-8",
                )
                stub_tee.chmod(0o755)

                with (
                    mock.patch.object(
                        ante_agent, "_INSTRUCTION_PATH", instruction_path
                    ),
                    mock.patch.object(ante_agent, "_AGENT_LOG", log_path),
                ):
                    command = ante_agent.ante_command(
                        "test/model",
                        "openrouter",
                        None,
                        "",
                    )

                env = os.environ.copy()
                env["PATH"] = f"{bin_dir}{os.pathsep}{env.get('PATH', '')}"
                result = subprocess.run(
                    ["bash", "-c", command],
                    capture_output=True,
                    check=False,
                    env=env,
                    text=True,
                    timeout=5,
                )

                self.assertEqual(result.returncode, ante_exit)
                self.assertEqual(result.stdout.strip(), event_text)
                self.assertEqual(
                    log_path.read_text(encoding="utf-8").strip(), event_text
                )
                self.assertFalse(instruction_path.exists())

    def test_command_omits_provider_and_effort_when_unset(self):
        command = ante_agent.ante_command("deepseek/model", None, None, "--yolo")

        self.assertIn("ante --model deepseek/model --yolo", command)
        self.assertNotIn("--provider", command)
        self.assertNotIn("--effort", command)

    def test_extra_args_reject_reserved_flags(self):
        for args in (
            "--model foo",
            "--model=foo",
            "--provider openrouter",
            "--provider=openrouter",
            "--effort high",
            "--effort=high",
        ):
            with self.subTest(args=args):
                with self.assertRaises(ValueError):
                    ante_agent.split_extra_ante_args(args)


def synthetic_events():
    return [
        {
            "timestamp": "2026-06-24T12:00:00Z",
            "id": "evt-session",
            "event": {
                "SessionStart": {
                    "session_id": "session-1",
                    "model": {"id": "deepseek/test", "effort": "xhigh"},
                    "provider": {"id": "openrouter"},
                    "cwd": "/app",
                    "permission_mode": "Yolo",
                }
            },
        },
        {
            "timestamp": "2026-06-24T12:00:01Z",
            "id": "evt-user",
            "event": {"UserInput": "write result.txt"},
        },
        {
            "timestamp": "2026-06-24T12:00:02Z",
            "id": "evt-thinking",
            "event": {"Thinking": "Need to inspect the workspace."},
        },
        {
            "timestamp": "2026-06-24T12:00:03Z",
            "id": "evt-tool-start",
            "event": {
                "ToolStart": {
                    "id": "tool-1",
                    "name": "Bash",
                    "args": {"command": "ls"},
                }
            },
        },
        {
            "timestamp": "2026-06-24T12:00:04Z",
            "id": "evt-tool-update",
            "event": {
                "ToolUpdate": {
                    "tool_use_id": "tool-1",
                    "seq": 1,
                    "message": "running",
                }
            },
        },
        {
            "timestamp": "2026-06-24T12:00:05Z",
            "id": "evt-tool-end",
            "event": {
                "ToolEnd": {
                    "tool_use_id": "tool-1",
                    "status": "Completed",
                    "result_json": {"stdout": "ok\n", "stderr": "", "exit_code": 0},
                }
            },
        },
        {
            "timestamp": "2026-06-24T12:00:06Z",
            "id": "evt-answer",
            "event": {"AgentMessage": "done"},
        },
        {
            "timestamp": "2026-06-24T12:00:07Z",
            "id": "evt-usage",
            "event": {
                "UsageUpdate": {
                    "usage": {
                        "input_tokens": 100,
                        "output_tokens": 20,
                        "cache_read_tokens": 10,
                        "cache_creation_tokens": 5,
                    }
                }
            },
        },
        {
            "timestamp": "2026-06-24T12:00:08Z",
            "id": "evt-turn-end",
            "event": {
                "TurnEnd": {
                    "turn_id": "turn-1",
                    "status": "Completed",
                    "steps": 3,
                }
            },
        },
    ]


class TrajectoryConversionTests(unittest.TestCase):
    def test_resolved_model_effort_comes_from_session_model(self):
        self.assertEqual(
            ante_events.resolved_model_effort_from_events(synthetic_events()),
            "xhigh",
        )

    def test_usage_accumulator_preserves_cache_creation_tokens(self):
        usage = ante_events.accumulate_usage_from_events(synthetic_events())
        self.assertEqual(
            usage,
            {
                "n_input_tokens": 100,
                "n_output_tokens": 20,
                "n_cache_tokens": 10,
                "n_cache_creation_tokens": 5,
            },
        )

    def test_usage_accumulator_preserves_missing_cache_creation(self):
        output = (
            json.dumps({
                "event": {
                    "UsageUpdate": {
                        "usage": {
                            "input_tokens": 100,
                            "output_tokens": 20,
                            "cache_read_tokens": 10,
                        }
                    }
                }
            })
            + "\n"
        )

        usage = ante_events.accumulate_usage_from_events(
            ante_events.events_from_text(output)
        )

        self.assertEqual(usage["n_input_tokens"], 100)
        self.assertEqual(usage["n_output_tokens"], 20)
        self.assertEqual(usage["n_cache_tokens"], 10)
        self.assertIsNone(usage["n_cache_creation_tokens"])

    def test_turn_end_steps_are_summed_across_check_turns(self):
        events = [
            {"event": {"TurnEnd": {"status": "Completed", "steps": 5}}},
            {"event": {"TurnEnd": {"status": "Completed", "steps": 2}}},
        ]

        self.assertEqual(ante_events.total_steps_from_events(events), 7)

    def test_turn_end_steps_are_absent_for_legacy_events(self):
        events = [{"event": {"TurnEnd": {"status": "Completed"}}}]

        self.assertIsNone(ante_events.total_steps_from_events(events))

    def test_synthetic_eventmsg_converts_to_atif_dict(self):
        trajectory = ante_events.trajectory_from_events(
            synthetic_events(),
            agent_name="ante",
            agent_version="1.2.3",
            model_name="fallback-model",
        )

        self.assertIsNotNone(trajectory)
        trajectory = trajectory.to_json_dict()
        self.assertEqual(trajectory["schema_version"], "ATIF-v1.7")
        self.assertEqual(trajectory["session_id"], "session-1")
        self.assertEqual(trajectory["agent"]["name"], "ante")
        self.assertEqual(trajectory["agent"]["version"], "1.2.3")
        self.assertEqual(trajectory["agent"]["model_name"], "deepseek/test")
        self.assertEqual(trajectory["agent"]["extra"]["provider_name"], "openrouter")
        self.assertEqual(trajectory["final_metrics"]["total_prompt_tokens"], 100)
        self.assertEqual(trajectory["final_metrics"]["total_completion_tokens"], 20)
        self.assertEqual(trajectory["final_metrics"]["total_cached_tokens"], 10)
        self.assertEqual(
            trajectory["final_metrics"]["extra"]["total_cache_creation_tokens"],
            5,
        )

        steps = trajectory["steps"]
        self.assertEqual([step["step_id"] for step in steps], [1, 2])
        self.assertEqual(steps[0]["source"], "user")
        self.assertEqual(steps[0]["message"], "write result.txt")
        self.assertEqual(steps[1]["tool_calls"][0]["function_name"], "Bash")
        self.assertEqual(steps[1]["llm_call_count"], 1)
        self.assertEqual(
            steps[1]["observation"]["results"][0]["source_call_id"],
            "tool-1",
        )
        self.assertIn("ok", steps[1]["observation"]["results"][0]["content"])
        self.assertEqual(steps[1]["reasoning_content"], "Need to inspect the workspace.")
        self.assertEqual(steps[1]["message"], "done")
        self.assertEqual(steps[1]["metrics"]["prompt_tokens"], 100)
        self.assertEqual(steps[1]["metrics"]["completion_tokens"], 20)
        self.assertEqual(steps[1]["metrics"]["cached_tokens"], 10)

    def test_parallel_tools_share_one_model_response_step(self):
        events = fixture_events("ante-parallel-tools.jsonl")
        trajectory = ante_events.trajectory_from_events(
            events,
            agent_name="ante",
            agent_version="1.2.3",
            model_name="test-model",
        )

        self.assertIsNotNone(trajectory)
        data = trajectory.to_json_dict()
        self.assertEqual(len(data["steps"]), 1)
        step = data["steps"][0]
        self.assertEqual(step["message"], "I’ll inspect the related files.")
        self.assertEqual(
            step["reasoning_content"], "Inspect all three files in parallel."
        )
        self.assertEqual(step["llm_call_count"], 1)
        self.assertEqual(step["metrics"]["prompt_tokens"], 120)
        self.assertEqual(
            [call["tool_call_id"] for call in step["tool_calls"]],
            ["tool-a", "tool-b", "tool-c"],
        )
        self.assertEqual(
            [result["source_call_id"] for result in step["observation"]["results"]],
            ["tool-a", "tool-b", "tool-c"],
        )
        self.assertEqual(
            step["observation"]["results"][1]["extra"]["updates"][0]["message"],
            "reading",
        )
        usage_update_count = sum(
            "UsageUpdate" in event.get("event", {}) for event in events
        )
        self.assertEqual(
            sum(item.get("llm_call_count", 0) for item in data["steps"]),
            usage_update_count,
        )

    def test_parallel_tool_status_is_completion_order_independent(self):
        events = fixture_events("ante-parallel-tools.jsonl")
        tool_ends = events[-2:]
        tool_ends[0]["event"]["ToolEnd"]["status"] = "Completed"
        tool_ends[1]["event"]["ToolEnd"]["status"] = "Failed"

        forward = ante_events.trajectory_from_events(
            events,
            agent_name="ante",
            agent_version="1.2.3",
            model_name="test-model",
        ).to_json_dict()
        reversed_completion = ante_events.trajectory_from_events(
            [*events[:-2], *reversed(tool_ends)],
            agent_name="ante",
            agent_version="1.2.3",
            model_name="test-model",
        ).to_json_dict()

        self.assertEqual(forward, reversed_completion)
        step = forward["steps"][0]
        self.assertNotIn("status", step.get("extra", {}))
        self.assertEqual(
            {
                result["source_call_id"]: result["extra"]["status"]
                for result in step["observation"]["results"]
            },
            {
                "tool-a": "Completed",
                "tool-b": "Failed",
                "tool-c": "Completed",
            },
        )

    def test_real_eventmsg_fixture_converts_with_typed_models(self):
        trajectory = ante_events.trajectory_from_events(
            fixture_events("ante-real-run-slice.jsonl"),
            agent_name="ante",
            agent_version="1.2.3",
            model_name="fallback-model",
        )

        self.assertIsNotNone(trajectory)
        data = trajectory.to_json_dict()
        self.assertEqual(data["schema_version"], "ATIF-v1.7")
        self.assertEqual(data["session_id"], "ses_01KVZXFSW6VPAHNFX50NFN07Q6")
        self.assertEqual(data["agent"]["model_name"], "gpt-5-mini")
        self.assertEqual(data["agent"]["extra"]["provider_name"], "openai")
        self.assertEqual(data["agent"]["extra"]["cwd"], "/app")
        self.assertEqual(data["agent"]["extra"]["permission_mode"], "yolo")

        self.assertEqual(data["final_metrics"]["total_steps"], 2)
        self.assertEqual(data["final_metrics"]["total_prompt_tokens"], 25893)
        self.assertEqual(data["final_metrics"]["total_completion_tokens"], 2466)
        self.assertEqual(data["final_metrics"]["total_cached_tokens"], 23552)

        steps = data["steps"]
        self.assertEqual([step["step_id"] for step in steps], [1, 2])
        self.assertEqual(steps[0]["source"], "agent")
        self.assertEqual(steps[0]["llm_call_count"], 1)
        self.assertEqual(steps[0]["metrics"]["prompt_tokens"], 10061)
        self.assertEqual(steps[0]["metrics"]["completion_tokens"], 2084)
        self.assertEqual(steps[0]["metrics"]["cached_tokens"], 9728)
        self.assertEqual(steps[0]["tool_calls"][0]["function_name"], "Bash")
        self.assertIn("grpcio==1.73.0", steps[0]["tool_calls"][0]["arguments"]["command"])
        self.assertNotIn("status", steps[0].get("extra", {}))
        result = steps[0]["observation"]["results"][0]
        self.assertEqual(result["source_call_id"], "call_KDV8NRRMtUThvV1dcQBcC1Om")
        self.assertEqual(result["extra"]["status"], "Completed")
        self.assertIn("grpcio-1.73.0", result["content"])
        self.assertEqual(len(result["extra"]["updates"]), 2)
        self.assertEqual(steps[1]["llm_call_count"], 1)
        self.assertEqual(steps[1]["metrics"]["prompt_tokens"], 15832)
        self.assertEqual(steps[1]["metrics"]["completion_tokens"], 382)
        self.assertEqual(steps[1]["metrics"]["cached_tokens"], 13824)
        self.assertEqual(
            sum(step.get("llm_call_count", 0) for step in steps),
            sum(
                "UsageUpdate" in event.get("event", {})
                for event in fixture_events("ante-real-run-slice.jsonl")
            ),
        )

    def test_populate_context_writes_trajectory_and_uses_final_metrics(self):
        with tempfile.TemporaryDirectory() as d:
            logs_dir = Path(d)
            (logs_dir / "ante.txt").write_text(
                "\n".join(json.dumps(event) for event in synthetic_events()) + "\n",
                encoding="utf-8",
            )
            agent = ante_agent.AnteAgent(
                logs_dir=logs_dir,
                model_name="fallback-model",
                version="1.2.3",
                enable_atif=True,
            )
            context = AgentContext()

            agent.populate_context_post_run(context)

            trajectory_path = logs_dir / "trajectory.json"
            self.assertTrue(trajectory_path.is_file())
            trajectory = json.loads(trajectory_path.read_text(encoding="utf-8"))
            self.assertEqual(trajectory["schema_version"], "ATIF-v1.7")
            self.assertEqual(context.n_input_tokens, 100)
            self.assertEqual(context.n_output_tokens, 20)
            self.assertEqual(context.n_cache_tokens, 10)
            self.assertIsNone(context.cost_usd)
            self.assertEqual(
                context.metadata,
                {"effort": "xhigh", "n_cache_creation_tokens": 5, "steps": 3},
            )

    def test_atif_disabled_by_default_keeps_legacy_usage_without_file(self):
        with tempfile.TemporaryDirectory() as d:
            logs_dir = Path(d)
            (logs_dir / "ante.txt").write_text(
                "\n".join(json.dumps(event) for event in synthetic_events()) + "\n",
                encoding="utf-8",
            )
            agent = ante_agent.AnteAgent(
                logs_dir=logs_dir,
                model_name="fallback-model",
            )
            context = AgentContext()

            agent.populate_context_post_run(context)

            self.assertFalse((logs_dir / "trajectory.json").exists())
            self.assertEqual(context.n_input_tokens, 100)
            self.assertEqual(context.n_output_tokens, 20)
            self.assertEqual(context.n_cache_tokens, 10)
            self.assertEqual(
                context.metadata,
                {"effort": "xhigh", "n_cache_creation_tokens": 5, "steps": 3},
            )

    def test_populate_context_classifies_only_final_structured_failure(self):
        with tempfile.TemporaryDirectory() as d:
            logs_dir = Path(d)
            raw_diagnostic = "provider rate limit exceeded: secret diagnostic detail"
            (logs_dir / "ante.txt").write_text(
                "\n".join(
                    [
                        *(json.dumps(event) for event in synthetic_events()),
                        raw_diagnostic,
                        json.dumps({
                            "event": {
                                "TurnEnd": {
                                    "turn_id": "turn-1",
                                    "status": {
                                        "Error": {
                                            "kind": "rate_limited",
                                            "headline": "rate limited",
                                            "details": ["HTTP 429 Too Many Requests"],
                                        }
                                    },
                                }
                            }
                        }),
                    ]
                )
                + "\n",
                encoding="utf-8",
            )
            agent = ante_agent.AnteAgent(
                logs_dir=logs_dir,
                model_name="fallback-model",
            )
            context = AgentContext()

            agent.populate_context_post_run(context)

            self.assertEqual(
                context.metadata,
                {
                    "effort": "xhigh",
                    "n_cache_creation_tokens": 5,
                    "steps": 3,
                    "failure_class": "rate_limited",
                },
            )
            self.assertNotIn(raw_diagnostic, json.dumps(context.metadata))
            self.assertNotIn("error_diagnostics", context.metadata)

    def test_populate_context_prefers_complete_success_stdout_over_partial_log(self):
        with tempfile.TemporaryDirectory() as d:
            logs_dir = Path(d)
            partial_events = synthetic_events()[:-1]
            (logs_dir / "ante.txt").write_text(
                "\n".join(json.dumps(event) for event in partial_events) + "\n",
                encoding="utf-8",
            )
            agent = ante_agent.AnteAgent(
                logs_dir=logs_dir,
                model_name="fallback-model",
            )
            agent._event_output = (  # noqa: SLF001 - successful exec capture
                "\n".join(json.dumps(event) for event in synthetic_events()) + "\n"
            )
            context = AgentContext()

            agent.populate_context_post_run(context)

            self.assertEqual(context.metadata["steps"], 3)
            self.assertIsNone(agent._event_output)

    def test_populate_context_parses_stdout_when_downloaded_log_is_missing_or_empty(self):
        for empty_log in (False, True):
            with self.subTest(empty_log=empty_log), tempfile.TemporaryDirectory() as d:
                logs_dir = Path(d)
                if empty_log:
                    (logs_dir / "ante.txt").write_text("", encoding="utf-8")
                agent = ante_agent.AnteAgent(
                    logs_dir=logs_dir,
                    model_name="fallback-model",
                )
                agent._event_output = (  # noqa: SLF001 - lifecycle fallback
                    "\n".join(json.dumps(event) for event in synthetic_events()) + "\n"
                )
                context = AgentContext()

                agent.populate_context_post_run(context)

                self.assertEqual(context.n_input_tokens, 100)
                self.assertEqual(context.metadata["effort"], "xhigh")
                self.assertIsNone(agent._event_output)


if __name__ == "__main__":
    unittest.main()
