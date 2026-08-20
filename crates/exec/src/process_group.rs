//! Process-group helpers shared by process execution backends.
//!
//! On non-Unix platforms these helpers are no-ops.

use std::io;

#[cfg(target_os = "linux")]
/// Ensure the child receives SIGTERM when the original parent dies.
///
/// This should run in `pre_exec` and uses `parent_pid` captured before spawn to
/// avoid a race where the parent exits between fork and exec.
pub fn set_parent_death_signal(parent_pid: libc::pid_t) -> io::Result<()> {
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) } == -1 {
        return Err(io::Error::last_os_error());
    }

    if unsafe { libc::getppid() } != parent_pid {
        unsafe {
            libc::raise(libc::SIGTERM);
        }
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
/// No-op on non-Linux platforms.
pub fn set_parent_death_signal(_parent_pid: i32) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
/// Detach from the controlling TTY by starting a new session.
pub fn detach_from_tty() -> io::Result<()> {
    let result = unsafe { libc::setsid() };
    if result == -1 {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EPERM) {
            return set_process_group();
        }
        return Err(err);
    }
    Ok(())
}

#[cfg(not(unix))]
/// No-op on non-Unix platforms.
pub fn detach_from_tty() -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
/// Put the calling process into its own process group.
///
/// Intended for use in `pre_exec` so the child becomes the group leader.
pub fn set_process_group() -> io::Result<()> {
    let result = unsafe { libc::setpgid(0, 0) };
    if result == -1 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

#[cfg(not(unix))]
/// No-op on non-Unix platforms.
pub fn set_process_group() -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
/// Resolve a process group by PID and send SIGKILL.
pub fn kill_by_pid(pid: u32) -> io::Result<()> {
    use std::io::ErrorKind;

    let pid = pid as libc::pid_t;
    let pgid = unsafe { libc::getpgid(pid) };
    if pgid == -1 {
        let err = io::Error::last_os_error();
        if err.kind() != ErrorKind::NotFound {
            return Err(err);
        }
        return Ok(());
    }

    let result = unsafe { libc::killpg(pgid, libc::SIGKILL) };
    if result == -1 {
        let err = io::Error::last_os_error();
        if err.kind() != ErrorKind::NotFound {
            return Err(err);
        }
    }

    Ok(())
}

#[cfg(not(unix))]
/// No-op on non-Unix platforms.
pub fn kill_by_pid(_pid: u32) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
/// Whether any process remains in the group led by `pgid`, including members
/// that outlived the leader. Signal 0 runs `killpg`'s existence and permission
/// checks without delivering anything, so `EPERM` (the group is another user's)
/// counts as alive. Groups 0 and 1 are rejected rather than probed: to `killpg`
/// they mean the caller's own group and init's.
///
/// Answers about the group id, not about the job that created it: a recycled
/// pgid reads as alive.
pub fn process_group_is_alive(pgid: u32) -> bool {
    if pgid <= 1 {
        return false;
    }
    let Ok(pgid) = libc::pid_t::try_from(pgid) else {
        return false;
    };
    if unsafe { libc::killpg(pgid, 0) } == 0 {
        return true;
    }
    io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
/// No portable existence probe, so this reports every group as alive — callers
/// fail safe by treating its processes as still running.
pub fn process_group_is_alive(_pgid: u32) -> bool {
    true
}
