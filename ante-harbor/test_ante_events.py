from __future__ import annotations

import importlib
import json
import sys
import types
import unittest
from typing import Any


class _Model:
    def __init__(self, **kwargs: Any):
        self.__dict__.update(kwargs)


def _install_harbor_stubs() -> None:
    """Keep parser tests runnable without installing Harbor's full dependency tree."""
    try:
        import harbor.models.trajectories  # noqa: F401
    except ModuleNotFoundError:
        modules = {
            "harbor": types.ModuleType("harbor"),
            "harbor.models": types.ModuleType("harbor.models"),
            "harbor.models.trajectories": types.ModuleType(
                "harbor.models.trajectories"
            ),
        }
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
        sys.modules.update(modules)


_install_harbor_stubs()
ante_events = importlib.import_module("ante_events")


def _event(name: str, data: Any, *, timestamp: str | None = None) -> dict[str, Any]:
    event = {"event": {name: data}}
    if timestamp is not None:
        event["timestamp"] = timestamp
    return event


def _usage(**values: Any) -> dict[str, Any]:
    return _event("UsageUpdate", {"usage": values})


class EventStreamTests(unittest.TestCase):
    def test_mixed_output_keeps_only_json_objects(self):
        first = _event("UserInput", "inspect the repository")
        second = _event("Info", "ready")
        output = "\n".join(
            (
                "installing ante...",
                json.dumps(first),
                "{malformed json",
                "[1, 2, 3]",
                f"  {json.dumps(second)}  ",
                "done",
            )
        )

        self.assertEqual(ante_events.events_from_text(output), [first, second])

    def test_usage_aggregation_preserves_optional_cache_creation(self):
        events = [
            _usage(
                input_tokens=10,
                output_tokens=2,
                cache_read_tokens=4,
                cache_creation_tokens=3,
            ),
            _event("Info", "between calls"),
            _usage(
                input_tokens=7,
                output_tokens=5,
                cache_read_tokens=True,
                cache_creation_tokens=0,
            ),
        ]

        self.assertEqual(
            ante_events.accumulate_usage_from_events(events),
            {
                "n_input_tokens": 17,
                "n_output_tokens": 7,
                "n_cache_tokens": 4,
                "n_cache_creation_tokens": 3,
            },
        )
        self.assertIsNone(
            ante_events.accumulate_usage_from_events(
                [_usage(input_tokens=1, output_tokens=2)]
            )["n_cache_creation_tokens"]
        )

    def test_final_metrics_accept_one_shot_iterables(self):
        events = [
            _usage(
                input_tokens=10,
                output_tokens=2,
                cache_read_tokens=4,
                cache_creation_tokens=3,
            ),
            _usage(input_tokens=7, output_tokens=5, cache_creation_tokens=0),
        ]

        metrics = ante_events._usage_totals(event for event in events)

        self.assertEqual(metrics.total_prompt_tokens, 17)
        self.assertEqual(metrics.total_completion_tokens, 7)
        self.assertEqual(metrics.total_cached_tokens, 4)
        self.assertEqual(metrics.extra, {"total_cache_creation_tokens": 3})


class FailureAndStepTests(unittest.TestCase):
    def test_final_turn_failure_uses_latest_terminal_turn(self):
        failed = _event(
            "TurnEnd",
            {
                "status": {
                    "Error": {
                        "kind": "rate_limited",
                        "headline": "rate limited",
                        "details": ["HTTP 429"],
                    }
                },
                "steps": 1,
            },
        )
        recovered = _event("TurnEnd", {"status": "Completed", "steps": 2})

        self.assertIsNone(ante_events.final_turn_failure([failed, recovered]))

        failure = ante_events.final_turn_failure([failed])
        self.assertEqual(failure.kind, "rate_limited")
        self.assertEqual(failure.failure_class, "rate_limited")
        self.assertEqual(failure.exception_kind, "rate_limited")
        self.assertEqual(
            failure.detail_text,
            "kind: rate_limited\nrate limited\nHTTP 429",
        )

    def test_future_failure_kind_has_stable_fallback(self):
        failure = ante_events.final_turn_failure(
            [
                _event(
                    "TurnEnd",
                    {
                        "status": {
                            "Error": {
                                "kind": "provider_changed_shape",
                                "headline": "request failed",
                                "details": [],
                            }
                        }
                    },
                )
            ]
        )

        self.assertEqual(failure.failure_class, "model_error")
        self.assertEqual(failure.exception_kind, "unknown_api")

    def test_incomplete_step_bound_skips_compaction_usage(self):
        events = [
            _event("TurnEnd", {"status": "Completed", "steps": 2}),
            _event("TurnStart", {"turn_id": "turn-2"}),
            _usage(input_tokens=10, output_tokens=1),
            _event("CompactEnd", {"summary": "summary"}),
            _usage(input_tokens=3, output_tokens=1),
            _usage(input_tokens=6, output_tokens=2),
        ]

        self.assertEqual(ante_events.total_steps_from_events(events), 2)
        self.assertEqual(
            ante_events.incomplete_steps_lower_bound_from_events(events), 5
        )
        self.assertIsNone(
            ante_events.incomplete_steps_lower_bound_from_events(
                [_event("TurnEnd", {"status": "Completed", "steps": 0})]
            )
        )


class TrajectoryTests(unittest.TestCase):
    def test_trajectory_preserves_messages_tools_usage_and_metadata(self):
        events = [
            _event(
                "SessionStart",
                {
                    "session_id": "session-123",
                    "model": {"id": "claude-sonnet-5", "effort": "high"},
                    "provider": {"id": "anthropic"},
                    "cwd": "/workspace/task",
                    "permission_mode": "yolo",
                },
                timestamp="2026-08-23T01:00:00Z",
            ),
            _event("UserInput", "read both files", timestamp="2026-08-23T01:00:01Z"),
            _event("Thinking", "I should inspect them."),
            _event("AgentMessage", "I'll inspect both files."),
            _event(
                "ToolStart",
                {
                    "id": "call-read",
                    "name": "Read",
                    "args": {"file_path": "README.md"},
                    "signature": "signed",
                },
            ),
            _event(
                "ToolStart",
                {"id": "call-shell", "name": "Bash", "args": "pwd"},
            ),
            _event(
                "ToolUpdate",
                {"tool_use_id": "call-shell", "seq": 0, "message": "running"},
                timestamp="2026-08-23T01:00:02Z",
            ),
            # Finish in reverse order to ensure results retain tool-call order.
            _event(
                "ToolEnd",
                {
                    "tool_use_id": "call-shell",
                    "status": "Failed",
                    "result_json": {"error": "not allowed"},
                },
            ),
            _event(
                "ToolEnd",
                {
                    "tool_use_id": "call-read",
                    "status": "Completed",
                    "result_json": {"content": "Ante"},
                },
            ),
            _usage(
                input_tokens=100,
                output_tokens=20,
                cache_read_tokens=40,
                cache_creation_tokens=5,
            ),
            _event("AgentMessage", "Done."),
            _usage(input_tokens=30, output_tokens=4, cache_read_tokens=10),
            _event("Info", "turn complete"),
        ]

        trajectory = ante_events.trajectory_from_events(
            events,
            agent_name="ante",
            agent_version="0.preview.86",
            model_name="fallback-model",
        )

        self.assertEqual(trajectory.schema_version, "ATIF-v1.7")
        self.assertEqual(trajectory.session_id, "session-123")
        self.assertEqual(trajectory.agent.name, "ante")
        self.assertEqual(trajectory.agent.version, "0.preview.86")
        self.assertEqual(trajectory.agent.model_name, "claude-sonnet-5")
        self.assertEqual(
            trajectory.agent.extra,
            {
                "provider_name": "anthropic",
                "cwd": "/workspace/task",
                "permission_mode": "yolo",
            },
        )

        user_step, tool_step, final_step, info_step = trajectory.steps
        self.assertEqual(user_step.source, "user")
        self.assertEqual(user_step.message, "read both files")
        self.assertEqual(tool_step.reasoning_content, "I should inspect them.")
        self.assertEqual(tool_step.message, "I'll inspect both files.")
        self.assertEqual(
            [call.tool_call_id for call in tool_step.tool_calls],
            ["call-read", "call-shell"],
        )
        self.assertEqual(tool_step.tool_calls[0].arguments, {"file_path": "README.md"})
        self.assertEqual(tool_step.tool_calls[0].extra, {"signature": "signed"})
        self.assertEqual(tool_step.tool_calls[1].arguments, {"value": "pwd"})
        self.assertEqual(
            [result.source_call_id for result in tool_step.observation.results],
            ["call-read", "call-shell"],
        )
        self.assertEqual(
            tool_step.observation.results[1].extra,
            {
                "status": "Failed",
                "updates": [
                    {
                        "seq": 0,
                        "message": "running",
                        "timestamp": "2026-08-23T01:00:02Z",
                    }
                ],
            },
        )
        self.assertEqual(tool_step.metrics.prompt_tokens, 100)
        self.assertEqual(tool_step.metrics.extra, {"cache_creation_tokens": 5})
        self.assertEqual(tool_step.llm_call_count, 1)
        self.assertEqual(final_step.message, "Done.")
        self.assertEqual(final_step.llm_call_count, 1)
        self.assertEqual(info_step.extra, {"ante_event": "Info"})

        self.assertEqual(trajectory.final_metrics.total_prompt_tokens, 130)
        self.assertEqual(trajectory.final_metrics.total_completion_tokens, 24)
        self.assertEqual(trajectory.final_metrics.total_cached_tokens, 50)
        self.assertEqual(trajectory.final_metrics.total_steps, 4)
        self.assertEqual(
            trajectory.final_metrics.extra,
            {"total_cache_creation_tokens": 5},
        )

    def test_legacy_events_use_fallback_model_and_omit_new_metadata(self):
        events = [
            _event("AgentMessage", "legacy response"),
            _usage(input_tokens=8, output_tokens=2),
        ]

        trajectory = ante_events.trajectory_from_events(
            events,
            agent_name="ante",
            agent_version="legacy",
            model_name="legacy-model",
        )

        self.assertEqual(trajectory.agent.model_name, "legacy-model")
        self.assertIsNone(trajectory.agent.extra)
        self.assertIsNone(trajectory.final_metrics.extra)
        self.assertIsNone(ante_events.resolved_model_effort_from_events(events))
        self.assertIsNone(ante_events.total_steps_from_events(events))

    def test_metadata_only_events_do_not_create_empty_trajectory(self):
        trajectory = ante_events.trajectory_from_events(
            [_event("SessionStart", {"session_id": "session-123"})],
            agent_name="ante",
            agent_version="test",
            model_name=None,
        )

        self.assertIsNone(trajectory)


if __name__ == "__main__":
    unittest.main()
