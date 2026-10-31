"""Supervisor output must remain drainable during controller cancellation."""

import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import ante_process
from ante_process import OutputCapture


class OutputCaptureTests(unittest.TestCase):
    def exercise_capture(self, log_path, sink, *, stopping):
        child = subprocess.Popen(
            [
                sys.executable,
                "-c",
                "import sys; sys.stdout.buffer.write(b'x' * 200000); sys.exit(3)",
            ],
            stdout=subprocess.PIPE,
        )
        try:
            with mock.patch("ante_process.sys.stdout", sink):
                output = OutputCapture(child, log_path)
                deadline = time.monotonic() + 3
                while child.poll() is None:
                    output.drain(stopping=stopping)
                    if time.monotonic() >= deadline:
                        self.fail("Output backpressure blocked process shutdown")
                    time.sleep(0.005)
                output.close(stopping=stopping)
            self.assertEqual(child.returncode, 3)
        finally:
            if child.poll() is None:
                child.kill()
            child.wait()
            child.stdout.close()

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
