"""Opt-in, disposable trials against the pinned Harbor Docker lifecycle.

ANTE_DOCKER_TESTS=1 python -m unittest test_ante_docker_lifecycle -v
Set ANTE_TEST_DOCKER_IMAGE to reuse a local Linux image with Python 3 and Bash.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

import ante_events
from harbor.models.trial.config import TrialConfig
from harbor.trial.trial import Trial


@unittest.skipUnless(
    os.environ.get("ANTE_DOCKER_TESTS") == "1",
    "Set ANTE_DOCKER_TESTS=1 for disposable Docker trials",
)
class DockerLifecycleTests(unittest.IsolatedAsyncioTestCase):
    async def run_trial(self, mode, *, streamed=False):
        with tempfile.TemporaryDirectory(prefix="ante-lifecycle-") as directory:
            root = Path(directory)
            task = root / "heartbeat"
            environment = task / "environment"
            tests = task / "tests"
            environment.mkdir(parents=True)
            tests.mkdir()
            fixtures = Path(__file__).parent / "fixtures"
            shutil.copy(fixtures / "process-heartbeat.py", environment / "ante")
            shutil.copy(fixtures / "process-verifier.py", tests / "verify.py")
            (tests / "test.sh").write_text(
                "#!/bin/bash\nset -eu\npython3 /tests/verify.py\n"
            )
            image = os.environ.get("ANTE_TEST_DOCKER_IMAGE", "python:3.12-slim")
            (environment / "Dockerfile").write_text(
                f"FROM {image}\nCOPY ante /usr/local/bin/ante\n"
                "RUN chmod +x /usr/local/bin/ante\nWORKDIR /workspace\n"
            )
            (task / "instruction.md").write_text(
                "Emit harmless heartbeats. No model or network calls.\n"
            )
            (task / "task.toml").write_text(
                "[environment]\ncpus = 1\nmemory_mb = 256\n"
                '[verifier]\ntimeout_sec = 15\nenvironment_mode = "shared"\n'
            )
            config = TrialConfig(
                task={"path": task},
                trials_dir=root / "trials",
                agent={
                    "import_path": "ante_agent:AnteAgent",
                    "model_name": "test/heartbeat",
                    "override_timeout_sec": 15 if mode == "normal" else 2,
                    "kwargs": {
                        "install_command": "mkdir -p /logs/agent; nohup ante service >/dev/null 2>&1 &"
                    },
                    "env": {"ANTE_HEARTBEAT_MODE": mode},
                },
                environment={"type": "docker", "delete": True},
            )
            trial = await Trial.create(config)
            if streamed:

                async def on_log(_entry):
                    pass

                trial.add_log_callback(on_log)

            captured = []
            original_sync = trial._sync_agent_output

            async def sync(target):
                await original_sync(target)
                captured.append((trial.paths.agent_dir / "ante.txt").read_bytes())

            started = time.monotonic()
            with mock.patch.object(trial, "_sync_agent_output", side_effect=sync):
                result = await trial.run()
            detail = result.model_dump_json(indent=2)
            self.assertIsNotNone(result.verifier_result, detail)
            self.assertEqual(result.verifier_result.rewards, {"reward": 1.0}, detail)
            if mode == "normal":
                self.assertIsNone(result.exception_info, detail)
            else:
                self.assertEqual(
                    result.exception_info.exception_type, "AgentTimeoutError", detail
                )
                self.assertIn("2.0 seconds", result.exception_info.exception_message)
            if mode == "kill":
                self.assertGreaterEqual(time.monotonic() - started, 5)

            log = (trial.paths.agent_dir / "ante.txt").read_bytes()
            self.assertEqual(captured, [log])
            sentinel = json.loads(
                (trial.paths.verifier_dir / "sentinel.json").read_text()
            )
            self.assertEqual(hashlib.sha256(log).hexdigest(), sentinel["log_sha256"])
            usage = ante_events.accumulate_usage_from_events(
                ante_events.events_from_text(log.decode())
            )
            if mode != "normal":
                updates = [
                    ante_events.usage_from_event(event)
                    for event in ante_events.events_from_text(log.decode())
                ]
                self.assertIn({"input_tokens": 13, "output_tokens": 1}, updates)
            self.assertGreater(usage["n_input_tokens"], 0)
            self.assertEqual(
                result.agent_result.n_input_tokens, usage["n_input_tokens"]
            )
            self.assertEqual(
                result.agent_result.n_output_tokens, usage["n_output_tokens"]
            )

    async def test_normal_completion_reaps_detached_children(self):
        await self.run_trial("normal")

    async def test_timeout_graceful_termination(self):
        await self.run_trial("graceful")

    async def test_timeout_kill_fallback_for_detached_children(self):
        await self.run_trial("kill")

    async def test_streamed_timeout_kills_children_after_exec_client_termination(self):
        await self.run_trial("kill", streamed=True)
