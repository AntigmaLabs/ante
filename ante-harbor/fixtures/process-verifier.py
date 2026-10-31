"""Verify that only the trial's process tree stopped, before this sentinel."""

import hashlib
import json
import os
import time
from pathlib import Path

logs = Path("/logs/agent")
processes = [
    json.loads(line) for line in (logs / "pids.jsonl").read_text().split("\n") if line
]
assert {p["role"] for p in processes} == {"parent", "child", "grandchild"}, processes
for process in processes:
    assert not Path(f"/proc/{process['pid']}").exists(), process

paths = [
    logs / "ante.txt",
    *(logs / f"{role}.heartbeat" for role in ("parent", "child", "grandchild")),
]
before = {path.name: path.read_bytes() for path in paths}
sentinel = {
    "at": time.time(),
    "log_sha256": hashlib.sha256(before["ante.txt"]).hexdigest(),
    "pids": processes,
}
Path("/logs/verifier/sentinel.json").write_text(json.dumps(sentinel))
service_pid = int((logs / "service.pid").read_text())
os.kill(service_pid, 0)
service_before = (logs / "service.heartbeat").stat().st_size
time.sleep(0.3)
assert (logs / "service.heartbeat").stat().st_size > service_before
assert {path.name: path.read_bytes() for path in paths} == before
Path("/logs/verifier/reward.txt").write_text("1")
