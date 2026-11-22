// Copyright 2026 Antigma Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::handle::{OutputChunk, ProcessHandle, SpawnedProcess, Stream};
use crate::{CommandOptions, HeadTailBuffer, lock_or_recover, subprocess};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::{Mutex, Notify, broadcast};
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};

const EXIT_DRAIN_GRACE: Duration = Duration::from_millis(50);
const RECENT_PROTECTION_COUNT: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoolConfig {
    pub max_processes: usize,
    pub max_output_bytes: usize,
    pub default_yield_ms: u64,
    pub max_yield_ms: u64,
    pub background_timeout_ms: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_processes: 64,
            max_output_bytes: 1024 * 1024,
            default_yield_ms: 250,
            max_yield_ms: 30_000,
            background_timeout_ms: 300_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExecRequest {
    pub command: CommandOptions,
    pub yield_time_ms: u64,
    pub max_output_bytes: Option<usize>,
}

impl ExecRequest {
    pub fn new(command: CommandOptions) -> Self {
        Self { command, yield_time_ms: 0, max_output_bytes: None }
    }

    pub fn with_yield_time_ms(mut self, yield_time_ms: u64) -> Self {
        self.yield_time_ms = yield_time_ms;
        self
    }

    pub fn with_max_output_bytes(mut self, max_output_bytes: usize) -> Self {
        self.max_output_bytes = Some(max_output_bytes);
        self
    }
}

#[derive(Clone, Debug)]
pub struct PollRequest<'a> {
    pub process_id: &'a str,
    pub yield_time_ms: u64,
    pub max_output_bytes: Option<usize>,
}

impl<'a> PollRequest<'a> {
    pub fn new(process_id: &'a str) -> Self {
        Self { process_id, yield_time_ms: 0, max_output_bytes: None }
    }

    pub fn with_yield_time_ms(mut self, yield_time_ms: u64) -> Self {
        self.yield_time_ms = yield_time_ms;
        self
    }

    pub fn with_max_output_bytes(mut self, max_output_bytes: usize) -> Self {
        self.max_output_bytes = Some(max_output_bytes);
        self
    }
}

#[derive(Clone, Debug)]
pub struct StdinRequest<'a> {
    pub process_id: &'a str,
    pub input: &'a [u8],
    pub yield_time_ms: u64,
    pub max_output_bytes: Option<usize>,
}

impl<'a> StdinRequest<'a> {
    pub fn new(process_id: &'a str, input: &'a [u8]) -> Self {
        Self { process_id, input, yield_time_ms: 0, max_output_bytes: None }
    }

    pub fn with_yield_time_ms(mut self, yield_time_ms: u64) -> Self {
        self.yield_time_ms = yield_time_ms;
        self
    }

    pub fn with_max_output_bytes(mut self, max_output_bytes: usize) -> Self {
        self.max_output_bytes = Some(max_output_bytes);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecResponse {
    pub output: Vec<u8>,
    pub stderr: Vec<u8>,
    pub process_id: Option<String>,
    pub exit_code: Option<i32>,
    pub wall_time: Duration,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExecError {
    SpawnFailed(String),
    UnknownProcess { process_id: String },
    StdinClosed,
    PoolFull,
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpawnFailed(message) => write!(f, "failed to spawn process: {message}"),
            Self::UnknownProcess { process_id } => write!(f, "unknown process: {process_id}"),
            Self::StdinClosed => write!(f, "stdin is closed for this process"),
            Self::PoolFull => write!(f, "process pool is full"),
        }
    }
}

impl Error for ExecError {}

#[derive(Clone)]
pub struct ProcessPool {
    inner: Arc<PoolInner>,
}

struct PoolInner {
    config: PoolConfig,
    entries: Mutex<HashMap<String, Arc<ProcessEntry>>>,
    spawn_gate: Mutex<()>,
    next_process_id: AtomicU64,
}

struct ProcessEntry {
    handle: ProcessHandle,
    output: StdMutex<HeadTailBuffer>,
    stderr: StdMutex<HeadTailBuffer>,
    notify: Arc<Notify>,
    last_used: StdMutex<Instant>,
    interaction: Mutex<()>,
    buffer_task: StdMutex<Option<JoinHandle<()>>>,
}

impl ProcessPool {
    pub fn new(config: PoolConfig) -> Self {
        let default_yield_ms = config.default_yield_ms.max(1);
        let max_yield_ms = config.max_yield_ms.max(default_yield_ms);
        let normalized = PoolConfig {
            max_processes: config.max_processes.max(1),
            max_output_bytes: config.max_output_bytes,
            default_yield_ms,
            max_yield_ms,
            background_timeout_ms: config.background_timeout_ms.max(1),
        };

        Self {
            inner: Arc::new(PoolInner {
                config: normalized,
                entries: Mutex::new(HashMap::new()),
                spawn_gate: Mutex::new(()),
                next_process_id: AtomicU64::new(1000),
            }),
        }
    }

    pub async fn exec(&self, request: ExecRequest) -> Result<ExecResponse, ExecError> {
        let spawn_guard = self.inner.spawn_gate.lock().await;
        self.ensure_capacity().await?;

        let process_id = self.next_process_id();
        let entry = ProcessEntry::new(
            subprocess::spawn(request.command)
                .await
                .map_err(|err| ExecError::SpawnFailed(err.to_string()))?,
            self.inner.config.max_output_bytes,
        );

        self.inner.entries.lock().await.insert(process_id.clone(), Arc::clone(&entry));
        drop(spawn_guard);

        self.interact(&entry, &process_id, None, request.yield_time_ms, request.max_output_bytes)
            .await
    }

    pub async fn poll_output(&self, request: PollRequest<'_>) -> Result<ExecResponse, ExecError> {
        self.prune_expired_entries().await;
        let entry = self.entry(request.process_id).await?;
        self.interact(
            &entry,
            request.process_id,
            None,
            request.yield_time_ms,
            request.max_output_bytes,
        )
        .await
    }

    pub async fn write_stdin(&self, request: StdinRequest<'_>) -> Result<ExecResponse, ExecError> {
        self.prune_expired_entries().await;
        let entry = self.entry(request.process_id).await?;
        self.interact(
            &entry,
            request.process_id,
            Some(request.input),
            request.yield_time_ms,
            request.max_output_bytes,
        )
        .await
    }

    pub async fn kill(&self, process_id: &str) -> Result<(), ExecError> {
        let removed = self.inner.entries.lock().await.remove(process_id);
        let Some(entry) = removed else {
            return Err(ExecError::UnknownProcess { process_id: process_id.to_string() });
        };

        shutdown_entry(entry, true);
        Ok(())
    }

    pub async fn terminate_all(&self) {
        let removed =
            self.inner.entries.lock().await.drain().map(|(_, entry)| entry).collect::<Vec<_>>();
        for entry in removed {
            shutdown_entry(entry, true);
        }
    }

    async fn interact(
        &self,
        entry: &Arc<ProcessEntry>,
        process_id: &str,
        input: Option<&[u8]>,
        yield_time_ms: u64,
        max_output_bytes: Option<usize>,
    ) -> Result<ExecResponse, ExecError> {
        let _interaction_guard = entry.interaction.lock().await;
        entry.touch();

        let stdin_closed = match input {
            Some(input) => entry.handle.write_stdin(input).await.is_err(),
            None => false,
        };

        if stdin_closed {
            if entry.handle.has_exited() {
                self.remove_if_same(process_id, entry, false).await;
            }
            return Err(ExecError::StdinClosed);
        }

        let response =
            self.collect_locked(entry, process_id, yield_time_ms, max_output_bytes).await;

        if response.process_id.is_none() {
            self.remove_if_same(process_id, entry, false).await;
        }

        Ok(response)
    }

    async fn collect_locked(
        &self,
        entry: &Arc<ProcessEntry>,
        process_id: &str,
        yield_time_ms: u64,
        max_output_bytes: Option<usize>,
    ) -> ExecResponse {
        let started = Instant::now();
        let deadline =
            Instant::now() + Duration::from_millis(self.normalize_yield_ms(yield_time_ms));
        let output_limit = self.normalize_output_bytes(max_output_bytes);
        let mut output = HeadTailBuffer::new(output_limit);
        let mut stderr = HeadTailBuffer::new(output_limit);
        let mut exit_grace_deadline = None;

        loop {
            entry.drain_into(&mut output, &mut stderr);

            let now = Instant::now();
            if let Some(grace_deadline) = exit_grace_deadline {
                if now >= grace_deadline {
                    break;
                }
            } else if entry.handle.has_exited() {
                exit_grace_deadline = Some(deadline.min(now + EXIT_DRAIN_GRACE));
            } else if now >= deadline {
                break;
            }

            let wait_until = exit_grace_deadline.unwrap_or(deadline);
            if Instant::now() >= wait_until {
                continue;
            }

            let notified = entry.notify.notified();
            tokio::pin!(notified);

            tokio::select! {
                _ = &mut notified => {}
                _ = entry.handle.wait_for_exit(), if exit_grace_deadline.is_none() => {
                    exit_grace_deadline = Some(deadline.min(Instant::now() + EXIT_DRAIN_GRACE));
                }
                _ = sleep_until(wait_until) => {
                    break;
                }
            }
        }

        entry.touch();
        let exit_code = entry.handle.exit_code();
        let process_id =
            if entry.handle.has_exited() { None } else { Some(process_id.to_string()) };

        ExecResponse {
            output: output.to_bytes(),
            stderr: stderr.to_bytes(),
            process_id,
            exit_code,
            wall_time: started.elapsed(),
        }
    }

    async fn entry(&self, process_id: &str) -> Result<Arc<ProcessEntry>, ExecError> {
        self.inner
            .entries
            .lock()
            .await
            .get(process_id)
            .cloned()
            .ok_or_else(|| ExecError::UnknownProcess { process_id: process_id.to_string() })
    }

    async fn ensure_capacity(&self) -> Result<(), ExecError> {
        let mut removed = self.take_expired_entries().await;
        {
            let mut entries = self.inner.entries.lock().await;
            while entries.len() >= self.inner.config.max_processes {
                let Some(process_id) = eviction_candidate(&entries) else {
                    break;
                };

                if let Some(entry) = entries.remove(&process_id) {
                    removed.push(entry);
                }
            }

            if entries.len() >= self.inner.config.max_processes {
                return Err(ExecError::PoolFull);
            }
        }

        shutdown_entries(removed, true);

        Ok(())
    }

    async fn prune_expired_entries(&self) {
        shutdown_entries(self.take_expired_entries().await, true);
    }

    async fn remove_if_same(
        &self,
        process_id: &str,
        expected: &Arc<ProcessEntry>,
        terminate: bool,
    ) {
        let removed = {
            let mut entries = self.inner.entries.lock().await;
            match entries.get(process_id) {
                Some(entry) if Arc::ptr_eq(entry, expected) => entries.remove(process_id),
                _ => None,
            }
        };

        if let Some(entry) = removed {
            shutdown_entry(entry, terminate);
        }
    }

    fn next_process_id(&self) -> String {
        self.inner.next_process_id.fetch_add(1, Ordering::Relaxed).to_string()
    }

    fn normalize_output_bytes(&self, max_output_bytes: Option<usize>) -> usize {
        max_output_bytes
            .unwrap_or(self.inner.config.max_output_bytes)
            .min(self.inner.config.max_output_bytes)
    }

    fn normalize_yield_ms(&self, yield_time_ms: u64) -> u64 {
        let yield_time_ms =
            if yield_time_ms == 0 { self.inner.config.default_yield_ms } else { yield_time_ms };

        yield_time_ms.clamp(self.inner.config.default_yield_ms, self.inner.config.max_yield_ms)
    }

    async fn take_expired_entries(&self) -> Vec<Arc<ProcessEntry>> {
        let timeout = Duration::from_millis(self.inner.config.background_timeout_ms);
        let now = Instant::now();
        let mut entries = self.inner.entries.lock().await;
        entries
            .extract_if(|_, entry| now.duration_since(entry.last_used()) > timeout)
            .map(|(_, entry)| entry)
            .collect()
    }
}

impl ProcessEntry {
    fn new(spawned: SpawnedProcess, max_output_bytes: usize) -> Arc<Self> {
        let (handle, rx) = spawned;
        let entry = Arc::new(Self {
            handle,
            output: StdMutex::new(HeadTailBuffer::new(max_output_bytes)),
            stderr: StdMutex::new(HeadTailBuffer::new(max_output_bytes)),
            notify: Arc::new(Notify::new()),
            last_used: StdMutex::new(Instant::now()),
            interaction: Mutex::new(()),
            buffer_task: StdMutex::new(None),
        });

        let task_entry = Arc::clone(&entry);
        let task = tokio::spawn(async move {
            task_entry.buffer_output(rx).await;
        });
        *lock_or_recover(&entry.buffer_task) = Some(task);

        entry
    }

    async fn buffer_output(self: Arc<Self>, mut rx: broadcast::Receiver<OutputChunk>) {
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    self.push_chunk(chunk);
                    self.notify.notify_waiters();
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }

    fn push_chunk(&self, chunk: OutputChunk) {
        let OutputChunk { stream, data } = chunk;

        if stream == Stream::Stderr {
            lock_or_recover(&self.stderr).push_chunk(data.clone());
        }

        lock_or_recover(&self.output).push_chunk(data);
    }

    fn drain_into(&self, output: &mut HeadTailBuffer, stderr: &mut HeadTailBuffer) {
        lock_or_recover(&self.output).drain_into(output);
        lock_or_recover(&self.stderr).drain_into(stderr);
    }

    fn touch(&self) {
        *lock_or_recover(&self.last_used) = Instant::now();
    }

    fn last_used(&self) -> Instant {
        *lock_or_recover(&self.last_used)
    }

    fn abort_buffer_task(&self) {
        if let Some(task) = lock_or_recover(&self.buffer_task).take() {
            task.abort();
        }
    }
}

fn eviction_candidate(entries: &HashMap<String, Arc<ProcessEntry>>) -> Option<String> {
    let mut candidates = entries
        .iter()
        .map(|(process_id, entry)| {
            (process_id.clone(), entry.last_used(), entry.handle.has_exited())
        })
        .collect::<Vec<_>>();

    if let Some((process_id, _, _)) = candidates
        .iter()
        .filter(|(_, _, exited)| *exited)
        .min_by_key(|(_, last_used, _)| *last_used)
    {
        return Some(process_id.clone());
    }

    candidates.sort_by_key(|(_, last_used, _)| *last_used);
    if candidates.is_empty() {
        return None;
    }

    let protected = RECENT_PROTECTION_COUNT.min(candidates.len().saturating_sub(1));
    let unprotected_end = candidates.len().saturating_sub(protected);

    candidates.into_iter().take(unprotected_end).next().map(|(process_id, _, _)| process_id)
}

fn shutdown_entry(entry: Arc<ProcessEntry>, terminate: bool) {
    if terminate && !entry.handle.has_exited() {
        entry.handle.terminate();
    }

    entry.abort_buffer_task();
}

fn shutdown_entries(entries: Vec<Arc<ProcessEntry>>, terminate: bool) {
    for entry in entries {
        shutdown_entry(entry, terminate);
    }
}
