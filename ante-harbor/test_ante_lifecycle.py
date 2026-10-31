"""Controller cancellation and Harbor's capture/verification boundary."""

from __future__ import annotations

import asyncio
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

from ante_agent import AgentCleanupError, AnteAgent
from harbor.models.agent.context import AgentContext
from harbor.trial.errors import AgentTimeoutError
from harbor.trial.single_step import SingleStepTrial


class ControlledEnvironment:
    def __init__(self, logs: Path, *, normal=False, fail_cleanup=False):
        self.logs = logs
        self.normal = normal
        self.fail_cleanup = fail_cleanup
        self.launched = asyncio.Event()
        self.stopping = asyncio.Event()
        self.allow_stop = asyncio.Event()
        self.stopped = False

    async def upload_file(self, *_args):
        pass

    async def exec(self, command, **_kwargs):
        if " stop " in command:
            self.stopping.set()
            await self.allow_stop.wait()
            self.stopped = not self.fail_cleanup
            # The final usage update arrives during graceful shutdown.
            (self.logs / "ante.txt").write_text(
                json.dumps(
                    {
                        "event": {
                            "UsageUpdate": {
                                "usage": {"input_tokens": 17, "output_tokens": 3}
                            }
                        }
                    }
                )
                + "\n"
            )
            return SimpleNamespace(
                return_code=int(self.fail_cleanup), stdout="cleanup", stderr=None
            )
        self.launched.set()
        if not self.normal:
            await asyncio.Future()
        return SimpleNamespace(return_code=0, stdout="complete stream\n", stderr=None)


class LifecycleTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.logs = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.agent = AnteAgent(logs_dir=self.logs, model_name="test/model")

    async def test_timeout_waits_for_cleanup_despite_repeated_cancellation(self):
        env = ControlledEnvironment(self.logs)
        running = asyncio.create_task(self.agent.run("test", env, AgentContext()))
        controller = asyncio.create_task(asyncio.wait_for(running, timeout=0.02))
        await asyncio.wait_for(env.stopping.wait(), timeout=1)
        running.cancel("cancel again during cleanup")
        await asyncio.sleep(0)
        self.assertFalse(controller.done())
        self.assertFalse(env.stopped)
        env.allow_stop.set()
        with self.assertRaises(asyncio.TimeoutError):
            await controller
        self.assertTrue(env.stopped)
        context = AgentContext()
        self.agent.populate_context_post_run(context)
        self.assertEqual((context.n_input_tokens, context.n_output_tokens), (17, 3))

    async def test_cancellation_during_normal_cleanup_is_replayed_after_stop(self):
        env = ControlledEnvironment(self.logs, normal=True)
        running = asyncio.create_task(self.agent.run("test", env, AgentContext()))
        await asyncio.wait_for(env.stopping.wait(), timeout=1)
        running.cancel("user cancellation")
        await asyncio.sleep(0)
        self.assertFalse(running.done())
        env.allow_stop.set()
        with self.assertRaisesRegex(asyncio.CancelledError, "user cancellation"):
            await running
        self.assertTrue(env.stopped)

    async def test_normal_completion_keeps_stdout_only_after_cleanup(self):
        env = ControlledEnvironment(self.logs, normal=True)
        running = asyncio.create_task(self.agent.run("test", env, AgentContext()))
        await asyncio.wait_for(env.stopping.wait(), timeout=1)
        self.assertIsNone(self.agent._event_output)
        env.allow_stop.set()
        await running
        self.assertTrue(env.stopped)
        self.assertEqual(self.agent._event_output, "complete stream\n")

    async def test_cleanup_failure_preserves_timeout_and_blocks_harbor_capture(self):
        env = ControlledEnvironment(self.logs, fail_cleanup=True)
        env.allow_stop.set()
        with self.assertLogs(self.agent.logger, level="ERROR"):
            with self.assertRaises(asyncio.TimeoutError):
                await asyncio.wait_for(
                    self.agent.run("test", env, AgentContext()), timeout=0.02
                )
        with self.assertRaises(AgentCleanupError):
            self.agent.populate_context_post_run(AgentContext())

    async def test_harbor_timeout_captures_after_stop_before_verifier(self):
        # Exercise the consumed Harbor method that catches AgentTimeoutError and
        # then calls _sync_agent_output. Its continuation must see settled logs.
        env = ControlledEnvironment(self.logs)
        order = []
        target = SimpleNamespace(agent_result=AgentContext())

        async def phase(**_kwargs):
            try:
                await asyncio.wait_for(
                    self.agent.run("test", env, target.agent_result), timeout=0.02
                )
            except asyncio.TimeoutError as exc:
                raise AgentTimeoutError("original trial deadline") from exc

        async def capture(_target):
            self.assertTrue(env.stopped)
            order.append("capture")
            self.agent.populate_context_post_run(target.agent_result)

        harness = SimpleNamespace(
            result=target,
            task=SimpleNamespace(
                instruction="test",
                config=SimpleNamespace(agent=SimpleNamespace(user=None)),
            ),
            _agent_timeout_sec=0.02,
            _load_trajectory=None,
            _run_agent_phase=phase,
            _sync_agent_output=capture,
            _record_exception=lambda exc: order.append(type(exc).__name__),
        )
        running = asyncio.create_task(SingleStepTrial._run_agent(harness))
        await asyncio.wait_for(env.stopping.wait(), timeout=1)
        self.assertEqual(order, [])
        env.allow_stop.set()
        await running
        order.append("verifier")
        self.assertEqual(order, ["AgentTimeoutError", "capture", "verifier"])
        self.assertEqual(target.agent_result.n_input_tokens, 17)

    async def test_cancelled_cleanup_task_also_blocks_verification(self):
        env = ControlledEnvironment(self.logs)
        with (
            mock.patch.object(
                self.agent,
                "_stop_process_scope",
                side_effect=asyncio.CancelledError("backend cancelled cleanup"),
            ),
            self.assertLogs(self.agent.logger, level="ERROR"),
        ):
            with self.assertRaises(asyncio.TimeoutError):
                await asyncio.wait_for(
                    self.agent.run("test", env, AgentContext()), timeout=0.02
                )
        with self.assertRaises(AgentCleanupError):
            self.agent.populate_context_post_run(AgentContext())

    async def test_cleanup_deadline_is_bounded(self):
        env = ControlledEnvironment(self.logs, normal=True)
        with mock.patch("ante_agent._CLEANUP_TIMEOUT_SEC", 0.02):
            with self.assertRaises(AgentCleanupError):
                await self.agent.run("test", env, AgentContext())
        with self.assertRaises(AgentCleanupError):
            self.agent.populate_context_post_run(AgentContext())
