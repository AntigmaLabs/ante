"""Verify success preserves task services while interruption stops owned work."""

import hashlib
import json
import os
import time
from pathlib import Path
from urllib.error import URLError
from urllib.request import urlopen

logs = Path("/logs/agent")
mode = (logs / "mode").read_text()
processes = [
    json.loads(line) for line in (logs / "pids.jsonl").read_text().split("\n") if line
]
assert {p["role"] for p in processes} == {"parent", "child", "grandchild"}, processes
for process in processes:
    alive = Path(f"/proc/{process['pid']}").exists()
    assert alive == (mode == "normal" and process["role"] != "parent"), process

port = int((logs / "task-service.port").read_text())
url = f"http://127.0.0.1:{port}/"
if mode == "normal":
    with urlopen(url, timeout=1) as response:
        assert response.read() == b"task-service"
else:
    try:
        urlopen(url, timeout=1)
    except URLError:
        pass
    else:
        raise AssertionError("Stopped task service is still reachable")

paths = [
    logs / "ante.txt",
    *(logs / f"{role}.heartbeat" for role in ("parent", "child", "grandchild")),
]
before = {path.name: path.read_bytes() for path in paths}
sentinel = {
    "at": time.time(),
    "log_sha256": hashlib.sha256(before["ante.txt"]).hexdigest(),
    "pids": processes,
    "task_service_reachable": mode == "normal",
}
Path("/logs/verifier/sentinel.json").write_text(json.dumps(sentinel))
service_pid = int((logs / "service.pid").read_text())
os.kill(service_pid, 0)
service_before = (logs / "service.heartbeat").stat().st_size
time.sleep(0.3)
assert (logs / "service.heartbeat").stat().st_size > service_before
for path in paths:
    if mode == "normal" and path.name in {"child.heartbeat", "grandchild.heartbeat"}:
        assert len(path.read_bytes()) > len(before[path.name])
    else:
        assert path.read_bytes() == before[path.name]
if mode == "normal":
    with urlopen(url, timeout=1) as response:
        assert response.read() == b"task-service"
Path("/logs/verifier/reward.txt").write_text("1")
