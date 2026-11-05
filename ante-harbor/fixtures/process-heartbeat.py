#!/usr/bin/env python3
"""Harmless fake Ante with detached descendants for Docker lifecycle tests."""

import json
import os
import signal
import subprocess
import sys
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

if "--version" in sys.argv:
    print("ante heartbeat-test")
    sys.exit(0)

role = (
    sys.argv[1]
    if len(sys.argv) > 1 and sys.argv[1] in {"child", "grandchild", "service"}
    else "parent"
)
mode = os.environ.get("ANTE_HEARTBEAT_MODE", "normal")
logs = Path("/logs/agent")
logs.mkdir(parents=True, exist_ok=True)
if role == "parent":
    (logs / "mode").write_text(mode)
if role == "service":
    (logs / "service.pid").write_text(str(os.getpid()))
else:
    fd = os.open(logs / "pids.jsonl", os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    os.write(fd, (json.dumps({"role": role, "pid": os.getpid()}) + "\n").encode())
    os.close(fd)


def usage(tokens):
    print(
        json.dumps(
            {
                "event": {
                    "UsageUpdate": {
                        "usage": {"input_tokens": tokens, "output_tokens": 1}
                    }
                }
            }
        ),
        flush=True,
    )


def stop(_signal, _frame):
    if role == "parent":
        usage(13)
    sys.exit(0)


signal.signal(
    signal.SIGTERM, signal.SIG_IGN if mode == "kill" and role != "parent" else stop
)
if role in {"parent", "child"}:
    subprocess.Popen(
        [sys.executable, __file__, "child" if role == "parent" else "grandchild"],
        start_new_session=True,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


class TaskService(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"task-service")

    def log_message(self, *_args):
        pass


server = None
if role == "child":
    server = HTTPServer(("127.0.0.1", 0), TaskService)
    server.timeout = 0.02
    (logs / "task-service.port").write_text(str(server.server_port))

started = time.monotonic()
while True:
    with (logs / f"{role}.heartbeat").open("a") as stream:
        stream.write(f"{time.monotonic()}\n")
    if role == "parent":
        usage(2)
        if (
            mode in {"normal", "nonzero"}
            and time.monotonic() - started >= 0.4
            and (logs / "task-service.port").exists()
            and (logs / "grandchild.heartbeat").exists()
        ):
            usage(17)
            if mode == "nonzero":
                sys.exit(7)
            print(
                json.dumps({"event": {"TurnEnd": {"status": "Completed", "steps": 1}}}),
                flush=True,
            )
            break
    if server is not None:
        server.handle_request()
    time.sleep(0.05)
