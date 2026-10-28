"""Tests for the JSON merge helper the adapter runs inside task sandboxes."""
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import ante_merge_json


class MergeSettingsTests(unittest.TestCase):
    def test_merge_keeps_existing_settings_and_servers(self):
        with tempfile.TemporaryDirectory() as d:
            settings = Path(d) / "home" / "settings.json"
            settings.parent.mkdir()
            settings.write_text(
                json.dumps({"theme": "dark", "mcp_servers": {"old": {"command": "a"}}})
            )
            payload = Path(d) / "payload.json"
            payload.write_text(json.dumps({"mcp_servers": {"new": {"url": "http://h/mcp"}}}))

            ante_merge_json.merge_json(str(settings), str(payload))

            merged = json.loads(settings.read_text())
        self.assertEqual(merged["theme"], "dark")
        self.assertEqual(set(merged["mcp_servers"]), {"old", "new"})

    def test_merge_combines_nested_objects_key_by_key(self):
        merged = ante_merge_json.deep_merge(
            {"providers": {"openrouter": {"base_url": "u"}, "other": {"id": "o"}}},
            {"providers": {"openrouter": {"stream_idle_timeout_secs": 300}}},
        )

        self.assertEqual(
            merged,
            {
                "providers": {
                    "openrouter": {"base_url": "u", "stream_idle_timeout_secs": 300},
                    "other": {"id": "o"},
                }
            },
        )

    def test_merge_creates_missing_settings_file(self):
        with tempfile.TemporaryDirectory() as d:
            settings = Path(d) / ".ante" / "settings.json"
            payload = Path(d) / "payload.json"
            payload.write_text(json.dumps({"mcp_servers": {"new": {"command": "b"}}}))

            ante_merge_json.merge_json(str(settings), str(payload))

            self.assertEqual(
                json.loads(settings.read_text()), {"mcp_servers": {"new": {"command": "b"}}}
            )

    def test_command_line_takes_target_and_payload_only(self):
        with tempfile.TemporaryDirectory() as d:
            payload = Path(d) / "payload.json"
            payload.write_text("{}")
            target = Path(d) / "settings.json"

            self.assertEqual(ante_merge_json.main(["prog", str(target), str(payload)]), 0)
            self.assertEqual(json.loads(target.read_text()), {})
            self.assertEqual(ante_merge_json.main(["prog", str(target)]), 2)


if __name__ == "__main__":
    unittest.main()
