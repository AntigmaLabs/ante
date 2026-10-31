"""Linux sandbox supervisor; stdlib only, uploaded alongside each Ante run.

The supervisor is a child subreaper: detached tool jobs are adopted here, not
by container init. Only descendants of this process are stopped. The container,
services, and subsequent verifier execs remain outside this ownership boundary.
"""

from __future__ import annotations

import ctypes
import fcntl
import json
import os
import signal
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path


GRACE_SECONDS = 5.0
KILL_SECONDS = 2.0
STOP_SECONDS = 10.0
POLL_SECONDS = 0.05


class OutputCapture:
    """Drain agent output even after the local exec client disconnects.

    Logging stays in the supervisor, outside the process group being stopped,
    so graceful shutdown can still emit its final usage update. A failed log
    write leaves stdout available, preserving the adapter's normal fallback.
    """

    def __init__(self, child: subprocess.Popen, log_path: Path):
        self.pipe = child.stdout
        os.set_blocking(self.pipe.fileno(), False)
        os.set_blocking(sys.stdout.fileno(), False)
        self.forward = True
        self.pending = bytearray()
        self.log = None
        try:
            log_path.parent.mkdir(parents=True, exist_ok=True)
            self.log = log_path.open("wb", buffering=0)
        except OSError:
            pass

    def flush_stdout(self):
        while self.pending:
            try:
                count = os.write(sys.stdout.fileno(), self.pending)
                del self.pending[:count]
            except BlockingIOError:
                return
            except OSError:
                self.pending.clear()
                self.forward = False

    def drain(self, *, stopping=False):
        self.flush_stdout()
        if self.pending:
            if not stopping:
                return  # Backpressure without blocking the supervisor's loop.
            # A cancelled Docker client may stop consuming stdout without
            # closing it. Keep draining to disk throughout bounded cleanup.
            self.pending.clear()
            self.forward = False
        # Bound each drain so a busy writer cannot starve the stop/deadline check.
        for _ in range(64):
            try:
                chunk = os.read(self.pipe.fileno(), 65536)
            except BlockingIOError:
                return
            if not chunk:
                return
            if self.log is not None:
                try:
                    self.log.write(chunk)
                except OSError:
                    self.log.close()
                    self.log = None
            if self.forward:
                self.pending.extend(chunk)
                self.flush_stdout()
                if self.pending:
                    return

    def close(self, *, stopping):
        deadline = time.monotonic() + KILL_SECONDS
        while True:
            self.drain(stopping=stopping)
            if not self.pending:
                break
            if time.monotonic() >= deadline:
                raise RuntimeError("Ante stdout did not drain after completion")
            time.sleep(POLL_SECONDS)
        self.pipe.close()
        if self.log is not None:
            self.log.close()


@contextmanager
def scope_lock(scope: Path):
    scope.mkdir(mode=0o700, parents=True, exist_ok=True)
    with (scope / "lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


def process_info(pid: int) -> tuple[int, int, str] | None:
    """Return parent, process group, and birth tick; tolerate exiting processes."""
    try:
        # comm can contain spaces and parentheses; fields after its last ')' are
        # state (3), ppid (4), pgrp (5), ... starttime (22).
        fields = (
            Path(f"/proc/{pid}/stat")
            .read_text(encoding="utf-8", errors="replace")
            .rsplit(")", 1)[1]
            .split()
        )
        return int(fields[1]), int(fields[2]), fields[19]
    except (FileNotFoundError, ProcessLookupError):
        return None


def descendants() -> dict[int, tuple[int, int, str]]:
    table = {}
    for entry in Path("/proc").iterdir():
        if entry.name.isdigit():
            pid = int(entry.name)
            info = process_info(pid)
            if info is not None:
                table[pid] = info
    owned = {os.getpid()}
    while True:
        children = {pid for pid, info in table.items() if info[0] in owned}
        added = children - owned
        if not added:
            return {pid: table[pid] for pid in owned if pid != os.getpid()}
        owned.update(added)


def reap(child: subprocess.Popen) -> bool:
    """Reap exits and report whether the kernel still knows of any children."""
    while True:
        try:
            pid, status = os.waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            return False
        if pid == 0:
            return True
        if pid == child.pid:
            child.returncode = (
                os.WEXITSTATUS(status) if os.WIFEXITED(status) else -os.WTERMSIG(status)
            )


def signal_owned(
    processes: dict[int, tuple[int, int, str]],
    child: subprocess.Popen,
    signum: int,
) -> None:
    # Signal the invocation's private group while its leader is still ours.
    # Individually cover detached sessions and surviving orphaned descendants.
    group = None
    leader = processes.get(child.pid)
    if leader is not None and process_info(child.pid) == leader:
        try:
            os.killpg(child.pid, signum)
            group = child.pid
        except ProcessLookupError:
            pass
    for pid, info in processes.items():
        if info[1] == group or process_info(pid) != info:
            continue
        try:
            os.kill(pid, signum)
        except ProcessLookupError:
            pass


def quiesce(
    child: subprocess.Popen, output: OutputCapture | None, *, stopping: bool
) -> None:
    deadline = time.monotonic() + GRACE_SECONDS
    terminated = set()
    while True:
        if output is not None:
            output.drain(stopping=stopping)
        reap(child)
        owned = descendants()
        # /proc is not an atomic snapshot. A child can be reparented between
        # reads; require the kernel's ECHILD as well before acknowledging stop.
        if not owned and not reap(child):
            return
        now = time.monotonic()
        if now >= deadline + KILL_SECONDS:
            raise RuntimeError(f"Ante processes survived SIGKILL: {sorted(owned)}")
        if now >= deadline:
            signal_owned(owned, child, signal.SIGKILL)
        else:
            fresh = {
                pid: info
                for pid, info in owned.items()
                if (pid, info[2]) not in terminated
            }
            signal_owned(fresh, child, signal.SIGTERM)
            terminated.update((pid, info[2]) for pid, info in fresh.items())
        time.sleep(POLL_SECONDS)


def run(scope: Path, command: str, log_path: Path) -> int:
    # PR_SET_CHILD_SUBREAPER, Linux >= 3.4. Fail before launching Ante if the
    # sandbox cannot provide the descendant ownership guarantee.
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(36, 1, 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), "Cannot become an Ante child subreaper")

    stopping = False

    def request_stop(_signum, _frame):
        nonlocal stopping
        stopping = True

    signal.signal(signal.SIGTERM, request_stop)
    signal.signal(signal.SIGINT, request_stop)
    child = None
    output = None
    error = None
    code = 124
    completed = False
    with scope_lock(scope):
        # The stop command may win the race with a delayed environment.exec.
        # Keep its tombstone until sandbox teardown so a late launch cannot run.
        if (scope / "stop").exists():
            return code
        (scope / "started").write_text(str(os.getpid()))
    try:
        child = subprocess.Popen(
            ["bash", "-c", command],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        output = OutputCapture(child, log_path)
        while not stopping and not (scope / "stop").exists():
            output.drain()
            status = child.poll()
            if status is not None:
                code = status if status >= 0 else 128 - status
                completed = True
                break
            time.sleep(POLL_SECONDS)
    finally:
        try:
            if child is not None:
                quiesce(child, output, stopping=not completed)
            if output is not None:
                output.close(stopping=not completed)
        except Exception as exc:
            error = str(exc)
            raise
        finally:
            # Atomic acknowledgement is published only after quiescence, or
            # with an explicit failure which the controller must reject.
            pending = scope / "done.tmp"
            pending.write_text(json.dumps({"error": error}))
            pending.replace(scope / "done")
    return code


def stop(scope: Path) -> None:
    with scope_lock(scope):
        (scope / "stop").touch()
        if not (scope / "started").exists():
            return
    deadline = time.monotonic() + STOP_SECONDS
    while not (scope / "done").exists():
        if time.monotonic() >= deadline:
            raise RuntimeError("Ante supervisor did not acknowledge process cleanup")
        time.sleep(POLL_SECONDS)
    error = json.loads((scope / "done").read_text())["error"]
    if error:
        raise RuntimeError(error)


if __name__ == "__main__":
    mode, directory = sys.argv[1:3]
    if mode == "run":
        sys.exit(run(Path(directory), sys.argv[3], Path(sys.argv[4])))
    elif mode == "stop":
        stop(Path(directory))
    else:
        raise ValueError(f"Unknown supervisor mode: {mode}")
