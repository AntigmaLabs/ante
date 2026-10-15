#!/usr/bin/env python3
"""Merge a JSON payload into an Ante config file such as ``settings.json``.

The adapter uploads this script beside Harbor's installed-agent files and
runs it inside the task sandbox, so it uses only the Python standard library.
Nested objects merge key by key; any other payload value replaces the existing
one. A missing target file is created.

Usage:
  ante_merge_json.py <target.json> <payload.json>
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def deep_merge(base: dict, overlay: dict) -> dict:
    """Return ``base`` with ``overlay`` merged in; nested objects merge by key."""
    merged = dict(base)
    for key, value in overlay.items():
        if isinstance(value, dict) and isinstance(merged.get(key), dict):
            merged[key] = deep_merge(merged[key], value)
        else:
            merged[key] = value
    return merged


def merge_json(target_path: str, payload_path: str) -> int:
    """Merge the JSON object in ``payload_path`` into the file at ``target_path``."""
    target = Path(target_path)
    existing = json.loads(target.read_text(encoding="utf-8")) if target.exists() else {}
    payload = json.loads(Path(payload_path).read_text(encoding="utf-8"))
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(deep_merge(existing, payload), indent=2), encoding="utf-8")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) == 3:
        return merge_json(argv[1], argv[2])
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
