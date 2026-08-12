from __future__ import annotations

import json
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from harbor.models.trajectories import (
    Agent,
    FinalMetrics,
    Metrics,
    Observation,
    ObservationResult,
    Step,
    ToolCall,
    Trajectory,
)


USAGE_FIELDS = (
    ("input_tokens", "n_input_tokens"),
    ("output_tokens", "n_output_tokens"),
    ("cache_read_tokens", "n_cache_tokens"),
    ("cache_creation_tokens", "n_cache_creation_tokens"),
)

OPTIONAL_USAGE_FIELDS = {"n_cache_creation_tokens"}

EFFORT_METADATA_KEY = "effort"
CACHE_CREATION_METADATA_KEY = "n_cache_creation_tokens"
FAILURE_CLASS_METADATA_KEY = "failure_class"
STEPS_METADATA_KEY = "steps"

DIAGNOSTIC_FAILURE_CLASS_PRIORITY = (
    "invalid_multimodal_payload",
    "vision_unsupported",
    "provider_endpoint_unavailable",
    "rate_limited",
    "timeout",
    "model_error",
)


@dataclass(frozen=True)
class _TurnErrorPolicy:
    failure_class: str
    exception_kind: str


@dataclass(frozen=True)
class FinalTurnFailure:
    """Normalized terminal Ante failure shared by the adapter and archive."""

    kind: str | None
    headline: str | None
    details: tuple[str, ...]
    failure_class: str | None
    exception_kind: str | None

    @property
    def detail_text(self) -> str:
        return "\n".join(
            value
            for value in (
                f"kind: {self.kind}" if self.kind else None,
                self.headline,
                *self.details,
            )
            if value
        )


_TURN_ERROR_POLICIES = {
    "rate_limited": _TurnErrorPolicy("rate_limited", "rate_limited"),
    "timeout": _TurnErrorPolicy("timeout", "unknown_api"),
    "overloaded": _TurnErrorPolicy("model_error", "overloaded"),
    "server_error": _TurnErrorPolicy("model_error", "internal"),
    "transport": _TurnErrorPolicy("model_error", "network"),
    "unexpected_eof": _TurnErrorPolicy("model_error", "connection_closed"),
    "malformed_response": _TurnErrorPolicy("model_error", "unknown_api"),
    "quota": _TurnErrorPolicy("model_error", "usage_limit"),
    "auth": _TurnErrorPolicy("model_error", "authentication"),
    "oauth": _TurnErrorPolicy("model_error", "authentication"),
    "forbidden": _TurnErrorPolicy("model_error", "unknown_api"),
    "context_overflow": _TurnErrorPolicy("model_error", "context_window_exceeded"),
    "invalid_request": _TurnErrorPolicy("model_error", "unknown_api"),
    "content_policy": _TurnErrorPolicy("model_error", "safety_refusal"),
    "unknown": _TurnErrorPolicy("model_error", "unknown_api"),
}

_FUTURE_TURN_ERROR_POLICY = _TurnErrorPolicy("model_error", "unknown_api")


def is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def usage_from_event(event_msg: dict[str, Any]) -> dict[str, Any] | None:
    """Return a UsageUpdate payload from one Ante EventMsg object."""
    event = event_msg.get("event")
    update = event.get("UsageUpdate") if isinstance(event, dict) else None
    usage = update.get("usage") if isinstance(update, dict) else None
    return usage if isinstance(usage, dict) else None


def total_steps_from_events(events: Iterable[dict[str, Any]]) -> int | None:
    """Sum turn-loop steps reported by Ante, or None for legacy event logs."""
    counts = []
    for event_msg in events:
        name, data = _event_name_data(event_msg)
        if name != "TurnEnd" or not isinstance(data, dict):
            continue
        value = data.get("steps")
        if isinstance(value, int) and not isinstance(value, bool) and value >= 0:
            counts.append(value)
    return sum(counts) if counts else None


def incomplete_steps_lower_bound_from_events(
    events: Iterable[dict[str, Any]],
) -> int | None:
    """Bound total steps when a started root turn has no terminal event.

    Each non-compaction ``UsageUpdate`` proves that one model iteration
    finished. An unmatched ``TurnStart`` proves the following iteration was
    attempted, so its minimum is one plus those updates. Completed turns still
    contribute their exact ``TurnEnd.steps`` value. ``None`` means every
    observed root turn ended, so callers should retain the exact total instead.
    """
    total = 0
    active_usage_updates: int | None = None
    skip_compaction_usage = False
    saw_incomplete_turn = False

    for event_msg in events:
        name, data = _event_name_data(event_msg)
        if name == "TurnStart":
            if active_usage_updates is not None:
                total += 1 + active_usage_updates
                saw_incomplete_turn = True
            active_usage_updates = 0
            skip_compaction_usage = False
            continue

        if active_usage_updates is None:
            if name == "TurnEnd" and isinstance(data, dict):
                value = data.get("steps")
                if isinstance(value, int) and not isinstance(value, bool) and value >= 0:
                    total += value
            continue

        if name == "CompactEnd":
            # A successful compaction emits its own UsageUpdate next, but it
            # does not advance the enclosing turn loop's step counter.
            skip_compaction_usage = True
        elif name == "UsageUpdate":
            if skip_compaction_usage:
                skip_compaction_usage = False
            else:
                active_usage_updates += 1
        elif name == "TurnEnd":
            value = data.get("steps") if isinstance(data, dict) else None
            if isinstance(value, int) and not isinstance(value, bool) and value >= 0:
                total += value
            else:
                total += 1 + active_usage_updates
            active_usage_updates = None
            skip_compaction_usage = False

    if active_usage_updates is not None:
        total += 1 + active_usage_updates
        saw_incomplete_turn = True

    return total if saw_incomplete_turn else None


def event_from_line(line: str) -> dict[str, Any] | None:
    """Return one Ante EventMsg JSONL object, if present."""
    line = line.strip()
    if not line.startswith("{"):
        return None
    try:
        event_msg = json.loads(line)
    except json.JSONDecodeError:
        return None
    return event_msg if isinstance(event_msg, dict) else None


def events_from_text(output: str) -> list[dict[str, Any]]:
    """Parse Ante's mixed stdout/log stream into EventMsg objects once."""
    return [event for line in output.splitlines() if (event := event_from_line(line))]


def _is_invalid_multimodal_payload(text: str) -> bool:
    return (
        "multimodal data is corrupted or cannot be processed" in text
        or (
            "multimodal" in text
            and (
                "invalid request" in text
                or "http 400" in text
                or "400 bad request" in text
            )
        )
    )


def _is_vision_unsupported(text: str) -> bool:
    unsupported_phrases = (
        "image input is not supported",
        "image input isn't supported",
        "image input not supported",
        "image inputs are not supported",
        "does not support image",
        "doesn't support image",
        "do not support image",
        "vision is not supported",
        "vision isn't supported",
        "vision not supported",
        "does not support vision",
        "doesn't support vision",
        "do not support vision",
        "text only model",
        "text-only model",
    )
    return any(phrase in text for phrase in unsupported_phrases)


def classify_diagnostic_text(text: str) -> str | None:
    """Normalize diagnostic text into the stable provider/agent classes."""
    text = text.lower()
    if _is_invalid_multimodal_payload(text):
        return "invalid_multimodal_payload"
    if "no endpoints available" in text or "no endpoints found" in text:
        if _is_vision_unsupported(text) or "image input" in text or "vision" in text:
            return "vision_unsupported"
        return "provider_endpoint_unavailable"
    if _is_vision_unsupported(text):
        return "vision_unsupported"
    if (
        "apiratelimiterror" in text
        or "rate limit" in text
        or "ratelimit" in text
        or "too many requests" in text
        or "429" in text
    ):
        return "rate_limited"
    if "timeout" in text:
        return "timeout"
    return None


def final_turn_failure(
    events: Iterable[dict[str, Any]],
) -> FinalTurnFailure | None:
    """Normalize the final root ``TurnEnd(Error)`` emitted by headless Ante.

    Headless output emits ``TurnEnd`` only for the root turn. Stopping at the
    final one is important for ``--check`` recovery: an initial
    failed turn followed by a successful check is a recovered run, not an error.
    """
    for event_msg in reversed(list(events)):
        name, data = _event_name_data(event_msg)
        if name != "TurnEnd":
            continue
        status = data.get("status") if isinstance(data, dict) else None
        error = status.get("Error") if isinstance(status, dict) else None
        if not isinstance(error, dict):
            return None

        kind = error.get("kind") if isinstance(error.get("kind"), str) else None
        headline = (
            error.get("headline") if isinstance(error.get("headline"), str) else None
        )
        raw_details = error.get("details")
        details = (
            tuple(str(value) for value in raw_details)
            if isinstance(raw_details, list)
            else ()
        )
        policy = _TURN_ERROR_POLICIES.get(kind, _FUTURE_TURN_ERROR_POLICY) if kind else None
        diagnostic_text = "\n".join(
            value for value in (kind, headline, *details) if value
        )
        return FinalTurnFailure(
            kind=kind,
            headline=headline,
            details=details,
            failure_class=(
                classify_diagnostic_text(diagnostic_text)
                or (policy.failure_class if policy else None)
            ),
            exception_kind=policy.exception_kind if policy else None,
        )
    return None


def merge_diagnostic_failure_classes(values: Iterable[Any]) -> str | None:
    """Resolve several step-level diagnostic classes with stable precedence."""
    classes = {
        value
        for value in values
        if isinstance(value, str) and value in DIAGNOSTIC_FAILURE_CLASS_PRIORITY
    }
    return next(
        (value for value in DIAGNOSTIC_FAILURE_CLASS_PRIORITY if value in classes),
        None,
    )


def fallback_failure_class(
    error_type: str,
    error_message: str,
    reward: Any,
    phase: str | None = None,
) -> str | None:
    """Classify Harbor data only when the adapter recorded no final failure."""
    if phase in {"environment_setup", "agent_setup"}:
        return "setup_error"
    if error_type:
        if "NonZeroAgentExitCodeError" in error_type:
            return "agent_exit"
        if "TimeoutError" in error_type:
            return "timeout"
        is_model_error = error_type.startswith("Api") or error_type in {
            "AgentAuthenticationError",
            "AgentSafetyRefusalError",
            "ContextWindowExceededError",
            "NetworkConnectionError",
            "NoEndpointsError",
            "UnknownApiError",
        }
        if is_model_error:
            if diagnostic_class := classify_diagnostic_text(
                f"{error_type}\n{error_message}"
            ):
                return diagnostic_class
            return "model_error"
        if "Agent" in error_type:
            return "agent_error"
        return "harness_error"
    return "verifier_fail" if not (is_number(reward) and reward > 0) else None


def _usage_from_payloads(payloads: Iterable[dict[str, Any] | None]) -> dict[str, Any] | None:
    totals = {target: 0 for _, target in USAGE_FIELDS}
    seen_field = {target: False for _, target in USAGE_FIELDS}
    seen = False
    for usage in payloads:
        if usage is None:
            continue
        seen = True
        for source, target in USAGE_FIELDS:
            value = usage.get(source)
            if isinstance(value, int) and not isinstance(value, bool):
                totals[target] += value
                seen_field[target] = True
    if not seen:
        return None
    return {
        target: None
        if target in OPTIONAL_USAGE_FIELDS and not seen_field[target]
        else totals[target]
        for _, target in USAGE_FIELDS
    }


def accumulate_usage_from_text(output: str) -> dict[str, Any] | None:
    """Sum token usage from Ante's JSON event stream text.

    Missing cache-write fields stay None, so providers that do not report them
    do not look like they explicitly reported zero.
    """
    return accumulate_usage_from_events(events_from_text(output))


def accumulate_usage_from_events(events: Iterable[dict[str, Any]]) -> dict[str, Any] | None:
    """Sum token usage from parsed Ante EventMsg objects."""
    return _usage_from_payloads(
        usage_from_event(event) for event in events if isinstance(event, dict)
    )


def has_reported_usage(usage: dict[str, Any] | None) -> bool:
    if not isinstance(usage, dict):
        return False
    return any(
        is_number(usage.get(field))
        for field in (
            "cost_usd",
            "n_input_tokens",
            "n_output_tokens",
            "n_cache_tokens",
            "n_cache_creation_tokens",
        )
    )


def _event_name_data(event_msg: dict[str, Any]) -> tuple[str | None, Any]:
    event = event_msg.get("event") if isinstance(event_msg, dict) else None
    if not isinstance(event, dict) or not event:
        return None, None
    name = next(iter(event))
    return name, event.get(name)


def resolved_model_effort_from_events(
    events: Iterable[dict[str, Any]],
) -> str | None:
    """Return the latest effective model effort reported by Ante.

    Session events carry the fully resolved ModelSpec, so this captures catalog
    defaults as well as explicit ``--effort`` overrides. Older Ante versions
    predate the effort field and return ``None``.
    """
    effort: str | None = None
    for event_msg in events:
        name, data = _event_name_data(event_msg)
        if name not in {"SessionStart", "SessionUpdated"} or not isinstance(data, dict):
            continue
        model = data.get("model")
        value = model.get("effort") if isinstance(model, dict) else None
        if isinstance(value, str) and value:
            effort = value.lower()
    return effort


def _timestamp(event_msg: dict[str, Any]) -> str | None:
    value = event_msg.get("timestamp") if isinstance(event_msg, dict) else None
    return value if isinstance(value, str) else None


def _stringify(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        return value
    try:
        return json.dumps(value, ensure_ascii=False)
    except TypeError:
        return str(value)


def _spec_label(value: Any) -> str | None:
    if isinstance(value, dict):
        for key in ("id", "name", "display_name"):
            label = value.get(key)
            if isinstance(label, str) and label:
                return label
        return None
    if isinstance(value, str) and value:
        return value
    return None


def _id_text(value: Any) -> str | None:
    if isinstance(value, str) and value:
        return value
    if value is not None:
        text = str(value)
        return text if text else None
    return None


def _args_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {"value": value}


def _metrics_from_usage(usage: dict[str, Any] | None) -> Any | None:
    if not isinstance(usage, dict):
        return None
    prompt_tokens = usage.get("input_tokens")
    completion_tokens = usage.get("output_tokens")
    cached_tokens = usage.get("cache_read_tokens")
    cache_creation_tokens = usage.get("cache_creation_tokens")

    metrics: dict[str, Any] = {}
    if isinstance(prompt_tokens, int) and not isinstance(prompt_tokens, bool):
        metrics["prompt_tokens"] = prompt_tokens
    if isinstance(completion_tokens, int) and not isinstance(completion_tokens, bool):
        metrics["completion_tokens"] = completion_tokens
    if isinstance(cached_tokens, int) and not isinstance(cached_tokens, bool):
        metrics["cached_tokens"] = cached_tokens
    if isinstance(cache_creation_tokens, int) and not isinstance(cache_creation_tokens, bool):
        metrics["extra"] = {"cache_creation_tokens": cache_creation_tokens}
    return Metrics(**metrics) if metrics else None


def _usage_totals(events: Iterable[dict[str, Any]]) -> Any | None:
    totals = accumulate_usage_from_events(events)
    if totals is None:
        return None

    cache_creation = totals["n_cache_creation_tokens"]
    saw_cache_creation = any(
        isinstance((usage := usage_from_event(event)), dict)
        and isinstance(usage.get("cache_creation_tokens"), int)
        and not isinstance(usage.get("cache_creation_tokens"), bool)
        for event in events
    )

    return FinalMetrics(
        total_prompt_tokens=totals["n_input_tokens"],
        total_completion_tokens=totals["n_output_tokens"],
        total_cached_tokens=totals["n_cache_tokens"],
        extra={"total_cache_creation_tokens": cache_creation}
        if saw_cache_creation
        else None,
    )


def _append_step(steps: list[Any], **kwargs: Any) -> Any:
    step = Step(step_id=len(steps) + 1, **kwargs)
    steps.append(step)
    return step


def _append_content(existing: str | None, content: str) -> str:
    return f"{existing}\n\n{content}" if existing else content


def trajectory_from_events(
    events: Iterable[dict[str, Any]],
    *,
    agent_name: str,
    agent_version: str,
    model_name: str | None,
) -> Any | None:
    """Convert parsed Ante EventMsg objects into Harbor's validated Trajectory model."""
    events = [event for event in events if isinstance(event, dict)]
    steps: list[Any] = []
    tool_steps: dict[str, Any] = {}
    tool_updates: dict[str, list[dict[str, Any]]] = {}
    active_model_step: Any | None = None
    last_model_step: Any | None = None
    session_id: str | None = None
    default_model_name = model_name
    agent_extra: dict[str, Any] = {}

    def current_model_step(timestamp: str | None) -> Any:
        nonlocal active_model_step
        if active_model_step is None:
            active_model_step = _append_step(
                steps,
                timestamp=timestamp,
                source="agent",
                model_name=default_model_name,
                message="",
            )
        return active_model_step

    for event_msg in events:
        name, data = _event_name_data(event_msg)
        timestamp = _timestamp(event_msg)
        if name is None:
            continue

        if name in {"MessageDelta", "ThinkingDelta"}:
            continue

        if name == "SessionStart" and isinstance(data, dict):
            session_id = _id_text(data.get("session_id")) or session_id
            default_model_name = _spec_label(data.get("model")) or default_model_name
            provider_name = _spec_label(data.get("provider"))
            if provider_name:
                agent_extra["provider_name"] = provider_name
            cwd = data.get("cwd")
            if isinstance(cwd, str) and cwd:
                agent_extra["cwd"] = cwd
            permission_mode = data.get("permission_mode")
            if isinstance(permission_mode, str) and permission_mode:
                agent_extra["permission_mode"] = permission_mode
            continue

        if name == "SessionUpdated" and isinstance(data, dict):
            default_model_name = _spec_label(data.get("model")) or default_model_name
            continue

        if name == "UserInput" and isinstance(data, str) and data.strip():
            _append_step(
                steps,
                timestamp=timestamp,
                source="user",
                message=data,
            )
            continue

        if name == "Thinking" and isinstance(data, str) and data.strip():
            step = current_model_step(timestamp)
            step.reasoning_content = _append_content(
                getattr(step, "reasoning_content", None), data
            )
            continue

        if name == "AgentMessage" and isinstance(data, str):
            step = current_model_step(timestamp)
            step.message = _append_content(getattr(step, "message", None), data)
            continue

        if name == "ToolStart" and isinstance(data, dict):
            call_id = _id_text(data.get("id")) or f"tool-{len(tool_steps) + 1}"
            tool_name = _id_text(data.get("name")) or "unknown"
            signature = data.get("signature")
            tool_extra = {"signature": signature} if signature is not None else None
            step = active_model_step or last_model_step
            if step is None:
                # A truncated log can begin after the response's UsageUpdate.
                # Preserve the tool without claiming an unobserved model call.
                step = _append_step(
                    steps,
                    timestamp=timestamp,
                    source="agent",
                    model_name=default_model_name,
                    message="",
                )
            tool_calls = list(getattr(step, "tool_calls", None) or [])
            tool_calls.append(
                ToolCall(
                    tool_call_id=call_id,
                    function_name=tool_name,
                    arguments=_args_dict(data.get("args", {})),
                    extra=tool_extra,
                )
            )
            step.tool_calls = tool_calls
            tool_steps[call_id] = step
            continue

        if name == "ToolUpdate" and isinstance(data, dict):
            call_id = _id_text(data.get("tool_use_id"))
            if call_id:
                update = {
                    "seq": data.get("seq"),
                    "message": data.get("message"),
                    "timestamp": timestamp,
                }
                tool_updates.setdefault(call_id, []).append(update)
            continue

        if name == "ToolEnd" and isinstance(data, dict):
            call_id = _id_text(data.get("tool_use_id")) or f"tool-{len(tool_steps) + 1}"
            step = tool_steps.get(call_id)
            if step is None:
                step = active_model_step or last_model_step
                if step is None:
                    step = _append_step(
                        steps,
                        timestamp=timestamp,
                        source="agent",
                        model_name=default_model_name,
                        message="",
                    )
                tool_calls = list(getattr(step, "tool_calls", None) or [])
                tool_calls.append(
                    ToolCall(
                        tool_call_id=call_id,
                        function_name="unknown",
                        arguments={},
                    )
                )
                step.tool_calls = tool_calls
                tool_steps[call_id] = step

            result_json = data.get("result_json")
            result_extra: dict[str, Any] = {}
            status = data.get("status")
            if status is not None:
                result_extra["status"] = status
            updates = tool_updates.get(call_id)
            if updates:
                result_extra["updates"] = updates
            result = ObservationResult(
                source_call_id=call_id,
                content=_stringify(result_json),
                extra=result_extra or None,
            )
            results = list(
                getattr(getattr(step, "observation", None), "results", None) or []
            )
            results = [
                existing
                for existing in results
                if getattr(existing, "source_call_id", None) != call_id
            ]
            results.append(result)
            call_order = {
                getattr(tool_call, "tool_call_id", None): index
                for index, tool_call in enumerate(getattr(step, "tool_calls", None) or [])
            }
            results.sort(
                key=lambda existing: call_order.get(
                    getattr(existing, "source_call_id", None), len(call_order)
                )
            )
            step.observation = Observation(results=results)
            continue

        if name == "UsageUpdate":
            usage = data.get("usage") if isinstance(data, dict) else None
            step = current_model_step(timestamp)
            step.metrics = _metrics_from_usage(usage)
            step.llm_call_count = 1
            last_model_step = step
            active_model_step = None
            continue

        if name in {"Info", "Error"} and isinstance(data, str) and data.strip():
            _append_step(
                steps,
                timestamp=timestamp,
                source="system",
                message=data,
                extra={"ante_event": name},
            )
            continue

        if name == "ShellOutput" and isinstance(data, dict):
            command = data.get("command")
            message = f"ShellOutput: {command}" if command else "ShellOutput"
            _append_step(
                steps,
                timestamp=timestamp,
                source="system",
                message=message,
                extra={"ante_event": name, **data},
            )

    if not steps:
        return None

    final_metrics = _usage_totals(events)
    if final_metrics is not None:
        final_metrics.total_steps = len(steps)

    return Trajectory(
        schema_version="ATIF-v1.7",
        session_id=session_id,
        agent=Agent(
            name=agent_name,
            version=agent_version,
            model_name=default_model_name,
            extra=agent_extra or None,
        ),
        steps=steps,
        final_metrics=final_metrics,
    )


def read_event_log_text(logs_dir: Path) -> str | None:
    """Read Harbor's downloaded Ante log without parsing it."""
    log_path = logs_dir / "ante.txt"
    try:
        return log_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
