"""Dependency-free contract tests for the adapter published to AntigmaLabs/ante."""
from __future__ import annotations

import importlib
import os
import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path
from unittest.mock import patch


class _Model:
    def __init__(self, *args, **kwargs):
        self.args = args
        self.__dict__.update(kwargs)

    def to_json_dict(self):
        return self.__dict__


class _BaseInstalledAgent:
    def version(self):
        return getattr(self, "_version", None)


def _install_harbor_stubs() -> None:
    modules = {
        "harbor": types.ModuleType("harbor"),
        "harbor.agents": types.ModuleType("harbor.agents"),
        "harbor.agents.installed": types.ModuleType("harbor.agents.installed"),
        "harbor.agents.installed.base": types.ModuleType("harbor.agents.installed.base"),
        "harbor.environments": types.ModuleType("harbor.environments"),
        "harbor.environments.base": types.ModuleType("harbor.environments.base"),
        "harbor.models": types.ModuleType("harbor.models"),
        "harbor.models.agent": types.ModuleType("harbor.models.agent"),
        "harbor.models.agent.context": types.ModuleType("harbor.models.agent.context"),
        "harbor.models.trial": types.ModuleType("harbor.models.trial"),
        "harbor.models.trial.result": types.ModuleType("harbor.models.trial.result"),
        "harbor.models.trajectories": types.ModuleType("harbor.models.trajectories"),
        "harbor.utils": types.ModuleType("harbor.utils"),
        "harbor.utils.trajectory_utils": types.ModuleType("harbor.utils.trajectory_utils"),
    }
    base = modules["harbor.agents.installed.base"]
    for name in (
        "AgentAuthenticationError",
        "AgentSafetyRefusalError",
        "ApiConnectionClosedError",
        "ApiInternalServerError",
        "ApiOverloadedError",
        "ApiRateLimitError",
        "ApiUsageLimitError",
        "ContextWindowExceededError",
        "NetworkConnectionError",
        "NonZeroAgentExitCodeError",
        "UnknownApiError",
    ):
        setattr(base, name, type(name, (Exception,), {}))
    base.BaseInstalledAgent = _BaseInstalledAgent
    base.CliFlag = _Model
    base.EnvVar = _Model
    base.with_prompt_template = lambda function: function
    modules["harbor.environments.base"].BaseEnvironment = _Model
    modules["harbor.models.agent.context"].AgentContext = _Model
    modules["harbor.models.trial.result"].AgentInfo = _Model
    modules["harbor.models.trial.result"].ModelInfo = _Model
    trajectories = modules["harbor.models.trajectories"]
    for name in (
        "Agent",
        "FinalMetrics",
        "Metrics",
        "Observation",
        "ObservationResult",
        "Step",
        "ToolCall",
        "Trajectory",
    ):
        setattr(trajectories, name, _Model)
    modules["harbor.utils.trajectory_utils"].format_trajectory_json = str
    sys.modules.update(modules)


_install_harbor_stubs()
sys.path.insert(0, str(Path(__file__).parent))
ante_agent = importlib.import_module("ante_agent")


class ShellCommandTests(unittest.TestCase):
    def run_shell(
        self, command: str, *, path: Path | None = None
    ) -> subprocess.CompletedProcess:
        env = os.environ.copy()
        if path is not None:
            env["PATH"] = f"{path}{os.pathsep}{env['PATH']}"
        return subprocess.run(
            ["bash", "-c", command], text=True, capture_output=True, env=env
        )

    def test_setup_log_command_overwrites_setup_log(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "setup" / "stdout.txt"
            with patch.object(ante_agent, "_SETUP_LOG", log):
                command = ante_agent.setup_log_command(
                    "printf 'fresh setup output\\n'", append=False
                )

            result = self.run_shell(command)

            self.assertEqual(result.returncode, 0)
            self.assertEqual(log.read_text(), "fresh setup output\n")

    def test_setup_log_command_appends_to_setup_log(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "setup" / "stdout.txt"
            log.parent.mkdir()
            log.write_text("earlier setup output\n")
            with patch.object(ante_agent, "_SETUP_LOG", log):
                command = ante_agent.setup_log_command("printf 'next setup output\\n'")

            result = self.run_shell(command)

            self.assertEqual(result.returncode, 0)
            self.assertEqual(
                log.read_text(), "earlier setup output\nnext setup output\n"
            )

    def test_ante_command_preserves_agent_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "ante"
            binary.write_text(
                "#!/usr/bin/env bash\ncat >/dev/null\nprintf 'agent output\\n'\nexit 19\n"
            )
            binary.chmod(0o755)
            instruction = root / "instruction.md"
            instruction.write_text("test instruction")
            log = root / "logs" / "ante.txt"

            with (
                patch.object(ante_agent, "_AGENT_LOG", log),
                patch.object(ante_agent, "_INSTRUCTION_PATH", instruction),
            ):
                command = ante_agent.ante_command("test-model", None, None, "")
            result = self.run_shell(command, path=root)

            self.assertEqual(result.returncode, 19)
            self.assertEqual(log.read_text(), "agent output\n")
            self.assertFalse(instruction.exists())


class AdapterMetadataTests(unittest.TestCase):
    def test_default_args_disable_session_persistence_and_skills(self):
        self.assertEqual(
            ante_agent.DEFAULT_ANTE_ARGS,
            "--yolo --output-format json --no-session-save --no-skills",
        )

    def test_agent_info_preserves_runtime_provider_and_full_model_name(self):
        agent = object.__new__(ante_agent.AnteAgent)
        agent._version = "1.2.3"
        agent.model_name = "deepseek/deepseek-v4-flash-0731"
        agent._provider = "openrouter"

        info = agent.to_agent_info()

        self.assertEqual(info.name, "ante")
        self.assertEqual(info.version, "1.2.3")
        self.assertEqual(info.model_info.name, "deepseek/deepseek-v4-flash-0731")
        self.assertEqual(info.model_info.provider, "openrouter")


if __name__ == "__main__":
    unittest.main()
