use crate::handle::{
    ChildTerminator, OUTPUT_CHANNEL_CAPACITY, OutputChunk, ProcessHandle, STDIN_CHANNEL_CAPACITY,
    SpawnedProcess, Stream,
};
use crate::process_group;
use crate::{HeadTailBuffer, OutputReceiver, lock_or_recover};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Notify, broadcast, mpsc};
use tokio::time::Instant;

const EXIT_OUTPUT_GRACE: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StdinMode {
    #[default]
    Piped,
    Null,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandOptions {
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    env: HashMap<String, String>,
    stdin: StdinMode,
}

impl CommandOptions {
    pub fn new(program: impl Into<String>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: cwd.into(),
            env: HashMap::new(),
            stdin: StdinMode::Piped,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env.extend(envs.into_iter().map(|(key, value)| (key.into(), value.into())));
        self
    }

    pub fn stdin(mut self, stdin: StdinMode) -> Self {
        self.stdin = stdin;
        self
    }

    pub fn no_stdin(self) -> Self {
        self.stdin(StdinMode::Null)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_omitted_bytes: usize,
    pub stderr_omitted_bytes: usize,
    pub stdout_head_bytes: usize,
    pub stderr_head_bytes: usize,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub wall_time: Duration,
}

pub async fn run_with_timeout(
    options: CommandOptions,
    timeout: Duration,
    max_output_bytes: usize,
) -> Result<RunOutput> {
    let started = Instant::now();
    let (handle, rx) = spawn(options).await?;
    let mut receiver = OutputReceiver::from(rx);
    let mut stdout = HeadTailBuffer::new(max_output_bytes);
    let mut stderr = HeadTailBuffer::new(max_output_bytes);
    let deadline = Instant::now() + timeout;
    let timeout_sleep = tokio::time::sleep_until(deadline);
    tokio::pin!(timeout_sleep);
    let mut timed_out = false;
    let mut exit_seen = false;

    loop {
        drain_receiver(&mut receiver, &mut stdout, &mut stderr);

        if exit_seen {
            break;
        }

        if handle.has_exited() {
            exit_seen = true;
            tokio::time::sleep(EXIT_OUTPUT_GRACE).await;
            continue;
        }

        tokio::select! {
            chunk = receiver.recv() => {
                if let Some(chunk) = chunk {
                    push_output_chunk(chunk, &mut stdout, &mut stderr);
                }
            }
            _ = handle.wait_for_exit() => {
                exit_seen = true;
                tokio::time::sleep(EXIT_OUTPUT_GRACE).await;
            }
            _ = &mut timeout_sleep => {
                timed_out = true;
                handle.terminate();
                tokio::time::sleep(EXIT_OUTPUT_GRACE).await;
                break;
            }
        }
    }

    drain_receiver(&mut receiver, &mut stdout, &mut stderr);

    Ok(RunOutput {
        stdout: stdout.to_bytes(),
        stderr: stderr.to_bytes(),
        stdout_omitted_bytes: stdout.omitted_bytes(),
        stderr_omitted_bytes: stderr.omitted_bytes(),
        stdout_head_bytes: stdout.head_bytes(),
        stderr_head_bytes: stderr.head_bytes(),
        exit_code: if timed_out { None } else { handle.exit_code() },
        timed_out,
        wall_time: started.elapsed(),
    })
}

pub async fn spawn(options: CommandOptions) -> Result<SpawnedProcess> {
    let with_stdin = matches!(options.stdin, StdinMode::Piped);
    let mut command = Command::new(&options.program);
    command
        .args(&options.args)
        .current_dir(&options.cwd)
        .envs(&options.env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false);

    if with_stdin {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    #[cfg(unix)]
    {
        let parent_pid = unsafe { libc::getpid() };
        unsafe {
            command.pre_exec(move || {
                process_group::detach_from_tty()?;
                process_group::set_parent_death_signal(parent_pid)?;
                Ok(())
            });
        }
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn process `{}`", options.program))?;
    let pid = child.id().ok_or_else(|| anyhow!("spawned process is missing a pid"))?;

    let writer = if with_stdin { child.stdin.take().map(spawn_stdin_writer) } else { None };
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("failed to capture stdout"))?;
    let stderr = child.stderr.take().ok_or_else(|| anyhow!("failed to capture stderr"))?;

    let (output_tx, output_rx) = broadcast::channel(OUTPUT_CHANNEL_CAPACITY);
    let exit_code = Arc::new(StdMutex::new(None));
    let exit_notify = Arc::new(Notify::new());

    spawn_reader(stdout, Stream::Stdout, output_tx.clone());
    spawn_reader(stderr, Stream::Stderr, output_tx.clone());
    spawn_exit_watcher(child, Arc::clone(&exit_code), Arc::clone(&exit_notify));

    let handle = ProcessHandle::from_parts(
        output_tx,
        writer,
        exit_code,
        exit_notify,
        Box::new(PidTerminator { pid }),
    );

    Ok((handle, output_rx))
}

/// Best-effort termination for a spawned child and its isolated process group.
pub fn terminate_child_process_group(child: &mut Child) {
    if let Some(pid) = child.id() {
        let _ = process_group::kill_by_pid(pid);
    }

    let _ = child.start_kill();
}

fn spawn_stdin_writer(mut stdin: ChildStdin) -> mpsc::Sender<Vec<u8>> {
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(STDIN_CHANNEL_CAPACITY);
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if stdin.write_all(&data).await.is_err() {
                return;
            }
            if stdin.flush().await.is_err() {
                return;
            }
        }
    });
    tx
}

struct PidTerminator {
    pid: u32,
}

impl ChildTerminator for PidTerminator {
    fn terminate(&mut self) {
        let _ = process_group::kill_by_pid(self.pid);
    }
}

fn spawn_reader<R>(mut reader: R, stream: Stream, output_tx: broadcast::Sender<OutputChunk>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => return,
                Ok(n) => {
                    let chunk = OutputChunk { stream, data: buffer[..n].to_vec() };
                    let _ = output_tx.send(chunk);
                }
                Err(_) => return,
            }
        }
    });
}

fn spawn_exit_watcher(
    mut child: Child,
    exit_code: Arc<StdMutex<Option<i32>>>,
    exit_notify: Arc<Notify>,
) {
    tokio::spawn(async move {
        let code = match child.wait().await {
            Ok(status) => Some(normalize_exit_code(status)),
            Err(_) => Some(-1),
        };

        *lock_or_recover(&exit_code) = code;
        exit_notify.notify_waiters();
    });
}

fn normalize_exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return 128 + signal;
    }

    -1
}

fn drain_receiver(
    receiver: &mut OutputReceiver,
    stdout: &mut HeadTailBuffer,
    stderr: &mut HeadTailBuffer,
) {
    receiver.drain_with(|chunk| push_output_chunk(chunk, stdout, stderr));
}

fn push_output_chunk(chunk: OutputChunk, stdout: &mut HeadTailBuffer, stderr: &mut HeadTailBuffer) {
    let OutputChunk { stream, data } = chunk;
    match stream {
        Stream::Stdout => stdout.push_chunk(data),
        Stream::Stderr => stderr.push_chunk(data),
    }
}
