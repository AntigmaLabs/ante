use crate::{
    CommandOptions, ExecRequest, PollRequest, PoolConfig, ProcessPool, StdinRequest, Stream,
    kill_by_pid, run_with_timeout, subprocess,
};
use anyhow::Result;
use std::time::Duration;
use tempfile::tempdir;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::broadcast::error::{RecvError, TryRecvError};
use tokio::time::{sleep, timeout};

fn test_pool_config() -> PoolConfig {
    PoolConfig {
        max_processes: 8,
        max_output_bytes: 16 * 1024,
        default_yield_ms: 25,
        max_yield_ms: 2_000,
        background_timeout_ms: 5_000,
    }
}

#[cfg(unix)]
#[tokio::test]
async fn spawn_captures_tagged_stdout_and_stderr() -> Result<()> {
    let cwd = tempdir()?;
    let args = vec!["-c".to_string(), "printf 'hello'; printf 'error' >&2".to_string()];

    let (handle, mut rx) =
        subprocess::spawn(CommandOptions::new("sh", cwd.path()).args(args)).await?;
    timeout(Duration::from_secs(5), handle.wait_for_exit()).await?;
    sleep(Duration::from_millis(50)).await;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    loop {
        match rx.try_recv() {
            Ok(chunk) => match chunk.stream {
                Stream::Stdout => stdout.extend_from_slice(&chunk.data),
                Stream::Stderr => stderr.extend_from_slice(&chunk.data),
            },
            Err(TryRecvError::Empty | TryRecvError::Closed) => break,
            Err(TryRecvError::Lagged(_)) => continue,
        }
    }

    assert_eq!(stdout, b"hello".to_vec());
    assert_eq!(stderr, b"error".to_vec());
    assert_eq!(handle.exit_code(), Some(0));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn write_stdin_round_trip() -> Result<()> {
    let cwd = tempdir()?;
    let args = vec!["-c".to_string(), "read line; printf 'seen:%s' \"$line\"".to_string()];

    let (handle, mut rx) =
        subprocess::spawn(CommandOptions::new("sh", cwd.path()).args(args)).await?;
    handle.write_stdin(b"ping\n").await?;
    timeout(Duration::from_secs(5), handle.wait_for_exit()).await?;
    sleep(Duration::from_millis(50)).await;

    let mut stdout = Vec::new();
    while let Ok(chunk) = rx.try_recv() {
        if chunk.stream == Stream::Stdout {
            stdout.extend_from_slice(&chunk.data);
        }
    }

    assert_eq!(stdout, b"seen:ping".to_vec());
    assert_eq!(handle.exit_code(), Some(0));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn spawn_no_stdin_rejects_stdin_writes() -> Result<()> {
    let cwd = tempdir()?;
    let args = vec!["-c".to_string(), "echo done".to_string()];

    let (handle, _) =
        subprocess::spawn(CommandOptions::new("sh", cwd.path()).args(args).no_stdin()).await?;
    let error = handle.write_stdin(b"input").await.expect_err("stdin should be unavailable");

    assert!(error.to_string().contains("stdin"));
    timeout(Duration::from_secs(5), handle.wait_for_exit()).await?;
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn terminate_stops_long_running_process() -> Result<()> {
    let cwd = tempdir()?;
    let args = vec!["-c".to_string(), "sleep 30".to_string()];

    let (handle, _) = subprocess::spawn(CommandOptions::new("sh", cwd.path()).args(args)).await?;
    sleep(Duration::from_millis(200)).await;

    handle.terminate();
    timeout(Duration::from_secs(5), handle.wait_for_exit()).await?;

    assert!(handle.has_exited());
    assert_ne!(handle.exit_code(), Some(0));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn kill_by_pid_stops_process_group_descendants() -> Result<()> {
    let cwd = tempdir()?;
    let marker = cwd.path().join("process_group_marker");
    let mut command = Command::new("sh");
    command
        .args(["-c", "(sleep 0.5; printf marker > process_group_marker) & sleep 30"])
        .current_dir(cwd.path())
        .kill_on_drop(false);
    unsafe {
        command.pre_exec(|| {
            crate::process_group::detach_from_tty()?;
            Ok(())
        });
    }

    let mut child = command.spawn()?;
    let pid = child.id().expect("spawned process should have a pid");

    kill_by_pid(pid)?;
    timeout(Duration::from_secs(5), child.wait()).await??;
    sleep(Duration::from_millis(800)).await;

    assert!(!marker.exists(), "descendant survived kill_by_pid and wrote {marker:?}");
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn terminate_child_process_group_stops_descendants() -> Result<()> {
    let cwd = tempdir()?;
    let marker = cwd.path().join("terminate_group_marker");
    let mut command = Command::new("sh");
    command
        .args(["-c", "(sleep 0.5; printf marker > terminate_group_marker) & sleep 30"])
        .current_dir(cwd.path())
        .kill_on_drop(false);
    unsafe {
        command.pre_exec(|| {
            crate::process_group::detach_from_tty()?;
            Ok(())
        });
    }

    let mut child = command.spawn()?;

    subprocess::terminate_child_process_group(&mut child);
    timeout(Duration::from_secs(5), child.wait()).await??;
    sleep(Duration::from_millis(800)).await;

    assert!(!marker.exists(), "descendant survived group termination and wrote {marker:?}");
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn dropping_process_handle_terminates_running_process() -> Result<()> {
    let cwd = tempdir()?;
    let marker = cwd.path().join("drop_marker");
    let args = vec!["-c".to_string(), "sleep 0.5; printf marker > drop_marker".to_string()];

    let (handle, _) = subprocess::spawn(CommandOptions::new("sh", cwd.path()).args(args)).await?;
    drop(handle);
    sleep(Duration::from_millis(800)).await;

    assert!(!marker.exists(), "process survived handle drop and wrote {marker:?}");
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn background_file_ipc_pattern() -> Result<()> {
    let cwd = tempdir()?;
    let args =
        vec!["-c".to_string(), "for i in 1 2 3; do echo line$i; sleep 0.05; done".to_string()];

    let (handle, mut rx) =
        subprocess::spawn(CommandOptions::new("sh", cwd.path()).args(args)).await?;
    let output_path = cwd.path().join("output.log");
    let output_path_for_task = output_path.clone();

    let file_task = tokio::spawn(async move {
        let mut file = tokio::fs::File::create(&output_path_for_task).await?;
        loop {
            match rx.recv().await {
                Ok(chunk) => file.write_all(&chunk.data).await?,
                Err(RecvError::Closed) => break,
                Err(RecvError::Lagged(_)) => continue,
            }
        }
        file.flush().await?;
        Ok::<(), anyhow::Error>(())
    });

    timeout(Duration::from_secs(5), handle.wait_for_exit()).await?;
    drop(handle);
    let _ = timeout(Duration::from_secs(5), file_task).await?;

    let content = tokio::fs::read_to_string(&output_path).await?;
    assert!(content.contains("line1"));
    assert!(content.contains("line2"));
    assert!(content.contains("line3"));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn run_with_timeout_collects_output() -> Result<()> {
    let cwd = tempdir()?;
    let args = vec!["-c".to_string(), "printf 'hello'; printf 'error' >&2".to_string()];

    let result = run_with_timeout(
        CommandOptions::new("sh", cwd.path()).args(args),
        Duration::from_secs(5),
        1024,
    )
    .await?;

    assert_eq!(result.stdout, b"hello".to_vec());
    assert_eq!(result.stderr, b"error".to_vec());
    assert_eq!(result.stdout_omitted_bytes, 0);
    assert_eq!(result.stderr_omitted_bytes, 0);
    assert_eq!(result.exit_code, Some(0));
    assert!(!result.timed_out);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn run_with_timeout_terminates_process() -> Result<()> {
    let cwd = tempdir()?;
    let args = vec!["-c".to_string(), "echo start; sleep 30".to_string()];

    let result = run_with_timeout(
        CommandOptions::new("sh", cwd.path()).args(args),
        Duration::from_millis(100),
        1024,
    )
    .await?;

    assert_eq!(result.stdout, b"start\n".to_vec());
    assert!(result.timed_out);
    assert_eq!(result.exit_code, None);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn run_with_timeout_tracks_stdout_and_stderr_truncation_independently() -> Result<()> {
    let cwd = tempdir()?;
    let args = vec!["-c".to_string(), "printf 'abcdefghij'; printf 'klmnopqrst' >&2".to_string()];

    let result = run_with_timeout(
        CommandOptions::new("sh", cwd.path()).args(args),
        Duration::from_secs(5),
        6,
    )
    .await?;

    assert_eq!(result.stdout, b"abchij".to_vec());
    assert_eq!(result.stderr, b"klmrst".to_vec());
    assert_eq!(result.stdout_omitted_bytes, 4);
    assert_eq!(result.stderr_omitted_bytes, 4);
    assert_eq!(result.stdout_head_bytes, 3);
    assert_eq!(result.stderr_head_bytes, 3);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn process_pool_supports_background_polling() -> Result<()> {
    let cwd = tempdir()?;
    let pool = ProcessPool::new(test_pool_config());
    let request = ExecRequest::new(
        CommandOptions::new("sh", cwd.path())
            .args(["-c".to_string(), "echo started; sleep 0.4; echo finished".to_string()]),
    )
    .with_yield_time_ms(25);

    let initial = pool.exec(request).await?;
    assert!(String::from_utf8_lossy(&initial.output).contains("started"));
    let process_id = initial.process_id.expect("process should still be running");

    let later = pool.poll_output(PollRequest::new(&process_id).with_yield_time_ms(1_000)).await?;

    assert!(String::from_utf8_lossy(&later.output).contains("finished"));
    assert_eq!(later.process_id, None);
    assert_eq!(later.exit_code, Some(0));
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn process_pool_supports_interactive_stdin() -> Result<()> {
    let cwd = tempdir()?;
    let pool = ProcessPool::new(test_pool_config());
    let request = ExecRequest::new(CommandOptions::new("sh", cwd.path()).args([
        "-c".to_string(),
        "while IFS= read -r line; do printf 'seen:%s\\n' \"$line\"; done".to_string(),
    ]))
    .with_yield_time_ms(25);

    let initial = pool.exec(request).await?;
    assert!(initial.output.is_empty());
    let process_id = initial.process_id.expect("interactive process should stay alive");

    let echoed =
        pool.write_stdin(StdinRequest::new(&process_id, b"ping\n").with_yield_time_ms(500)).await?;

    assert!(String::from_utf8_lossy(&echoed.output).contains("seen:ping"));
    assert_eq!(echoed.process_id, Some(process_id.clone()));

    pool.kill(&process_id).await?;
    Ok(())
}
