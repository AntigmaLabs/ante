"""Task services survive success; failed/cancelled runs leave settled output."""

import json
import os
import shlex
import signal
import socket
import subprocess
import sys
import tempfile
import unittest
import urllib.request
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import ante_process
from ante_process import OutputCapture


class OutputCaptureTests(unittest.TestCase):
    def exercise_capture(self, log_path, sink, *, stopping, size=200000):
        with tempfile.TemporaryFile(mode="a+b") as capture:
            child = subprocess.Popen(
                [
                    sys.executable,
                    "-c",
                    f"import sys; sys.stdout.buffer.write(b'x' * {size}); sys.exit(3)",
                ],
                stdout=capture,
            )
            try:
                with mock.patch("ante_process.sys.stdout", sink):
                    output = OutputCapture(capture, log_path)
                    # Exercise data already buffered when the root exits,
                    # including more than a single bounded drain batch.
                    child.wait(timeout=3)
                    output.close(stopping=stopping)
                self.assertEqual(child.returncode, 3)
            finally:
                if child.poll() is None:
                    child.kill()
                child.wait()

    def test_final_capture_drains_more_than_one_batch(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryFile() as sink,
        ):
            log = Path(directory) / "ante.txt"
            size = 5 * 1024 * 1024
            self.exercise_capture(log, sink, stopping=False, size=size)
            sink.seek(0)
            self.assertEqual(sink.read(), b"x" * size)
            self.assertEqual(log.read_bytes(), b"x" * size)

    def test_failed_log_open_keeps_complete_stdout_and_exit_status(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryFile() as sink,
        ):
            self.exercise_capture(Path(directory), sink, stopping=False)
            sink.seek(0)
            self.assertEqual(sink.read(), b"x" * 200000)

    def test_cancelled_client_that_stops_reading_cannot_block_log_capture(self):
        read_fd, write_fd = os.pipe()
        try:
            with (
                tempfile.TemporaryDirectory() as directory,
                os.fdopen(write_fd, "wb") as sink,
            ):
                log = Path(directory) / "ante.txt"
                self.exercise_capture(log, sink, stopping=True)
                self.assertEqual(log.read_bytes(), b"x" * 200000)
        finally:
            os.close(read_fd)

    def test_disconnected_client_keeps_complete_log(self):
        read_fd, write_fd = os.pipe()
        os.close(read_fd)
        with (
            tempfile.TemporaryDirectory() as directory,
            os.fdopen(write_fd, "wb") as sink,
        ):
            log = Path(directory) / "ante.txt"
            self.exercise_capture(log, sink, stopping=True)
            self.assertEqual(log.read_bytes(), b"x" * 200000)

    def test_incomplete_proc_snapshot_cannot_acknowledge_live_children(self):
        with (
            mock.patch("ante_process.descendants", return_value={}),
            mock.patch("ante_process.reap", return_value=True),
            mock.patch("ante_process.GRACE_SECONDS", 0),
            mock.patch("ante_process.KILL_SECONDS", 0),
        ):
            with self.assertRaisesRegex(RuntimeError, "survived SIGKILL"):
                ante_process.quiesce(SimpleNamespace(pid=42), None, stopping=True)


@unittest.skipUnless(
    sys.platform == "linux", "Supervisor requires Linux child-subreaper support"
)
class SupervisorProtocolTests(unittest.TestCase):
    def exercise_task_service(self, exit_code):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scope = root / "scope"
            port_file = root / "port"
            pid_file = root / "pid"
            service = """
import http.server, pathlib
class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        print("service still writes after agent exit", flush=True)
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"alive")
    def log_message(self, *args):
        pass
server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
pathlib.Path(PORT_FILE).write_text(str(server.server_port))
server.serve_forever()
""".replace("PORT_FILE", repr(str(port_file)))
            # Inherit stdout/stderr deliberately. The supervisor must finish
            # without EOF, and later service writes must not hit a broken pipe.
            agent = f"""
import pathlib, subprocess, sys, time
child = subprocess.Popen([sys.executable, "-u", "-c", {service!r}], start_new_session=True)
pathlib.Path({str(pid_file)!r}).write_text(str(child.pid))
deadline = time.monotonic() + 2
while not pathlib.Path({str(port_file)!r}).exists():
    assert time.monotonic() < deadline
    time.sleep(0.01)
print("final", flush=True)
sys.exit({exit_code})
"""
            try:
                result = subprocess.run(
                    [
                        sys.executable,
                        ante_process.__file__,
                        "run",
                        str(scope),
                        shlex.join([sys.executable, "-c", agent]),
                        str(root / "ante.txt"),
                    ],
                    capture_output=True,
                    timeout=5,
                )
                self.assertEqual(result.returncode, exit_code, result.stderr)
                self.assertEqual(result.stdout, b"final\n")
                self.assertEqual((root / "ante.txt").read_bytes(), result.stdout)
                ante_process.stop(scope)
                ante_process.stop(scope)
                port = int(port_file.read_text())
                if exit_code == 0:
                    for _ in range(2):
                        with urllib.request.urlopen(
                            f"http://127.0.0.1:{port}", timeout=1
                        ) as response:
                            self.assertEqual(response.read(), b"alive")
                    # Task service output after completion cannot mutate the
                    # settled event log used for usage and verification.
                    self.assertEqual((root / "ante.txt").read_bytes(), result.stdout)
                else:
                    with socket.socket() as connection:
                        connection.settimeout(1)
                        self.assertNotEqual(
                            connection.connect_ex(("127.0.0.1", port)), 0
                        )
                self.assertEqual(
                    json.loads((scope / "done").read_text()), {"error": None}
                )
            finally:
                if pid_file.exists():
                    try:
                        os.kill(int(pid_file.read_text()), signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    def test_success_preserves_task_service_with_inherited_output(self):
        self.exercise_task_service(0)

    def test_nonzero_exit_stops_task_service(self):
        self.exercise_task_service(7)

    def test_stop_before_launch_prevents_late_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scope = root / "scope"
            ante_process.stop(scope)
            result = subprocess.run(
                [
                    sys.executable,
                    ante_process.__file__,
                    "run",
                    str(scope),
                    "touch should-not-exist",
                    str(root / "ante.txt"),
                ],
                cwd=root,
                capture_output=True,
                timeout=3,
            )
            self.assertEqual(result.returncode, 124, result.stderr)
            self.assertFalse((root / "should-not-exist").exists())
            ante_process.stop(scope)

    def test_nonzero_exit_keeps_output_and_cleanup_acknowledgement(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scope = root / "scope"
            log = root / "ante.txt"
            result = subprocess.run(
                [
                    sys.executable,
                    ante_process.__file__,
                    "run",
                    str(scope),
                    "printf final; exit 7",
                    str(log),
                ],
                capture_output=True,
                timeout=3,
            )
            self.assertEqual(result.returncode, 7, result.stderr)
            self.assertEqual(result.stdout, b"final")
            self.assertEqual(log.read_bytes(), result.stdout)
            ante_process.stop(scope)
            ante_process.stop(scope)
