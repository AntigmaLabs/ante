// Copyright 2026 Antigma Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Result, anyhow};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Notify, broadcast, mpsc};

use crate::lock_or_recover;

pub(crate) const OUTPUT_CHANNEL_CAPACITY: usize = 256;
pub(crate) const STDIN_CHANNEL_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputChunk {
    pub stream: Stream,
    pub data: Vec<u8>,
}

/// Handle + a pre-subscribed receiver that won't miss early output.
pub type SpawnedProcess = (ProcessHandle, broadcast::Receiver<OutputChunk>);

pub(crate) trait ChildTerminator: Send + 'static {
    fn terminate(&mut self);
}

/// Thin process lifecycle handle.
pub struct ProcessHandle {
    output_tx: broadcast::Sender<OutputChunk>,
    writer: Option<mpsc::Sender<Vec<u8>>>,
    exit_code: Arc<StdMutex<Option<i32>>>,
    exit_notify: Arc<Notify>,
    terminator: StdMutex<Option<Box<dyn ChildTerminator>>>,
}

impl ProcessHandle {
    pub(crate) fn from_parts(
        output_tx: broadcast::Sender<OutputChunk>,
        writer: Option<mpsc::Sender<Vec<u8>>>,
        exit_code: Arc<StdMutex<Option<i32>>>,
        exit_notify: Arc<Notify>,
        terminator: Box<dyn ChildTerminator>,
    ) -> Self {
        Self {
            output_tx,
            writer,
            exit_code,
            exit_notify,
            terminator: StdMutex::new(Some(terminator)),
        }
    }

    /// Write bytes to the child's stdin. Errors if stdin was not piped.
    pub async fn write_stdin(&self, data: &[u8]) -> Result<()> {
        let writer =
            self.writer.as_ref().ok_or_else(|| anyhow!("stdin was not piped for this process"))?;
        writer
            .send(data.to_vec())
            .await
            .map_err(|_| anyhow!("stdin is closed for this process"))?;
        Ok(())
    }

    /// Subscribe to the raw output stream.
    pub fn output_subscribe(&self) -> broadcast::Receiver<OutputChunk> {
        self.output_tx.subscribe()
    }

    /// Wait until the process exits.
    pub async fn wait_for_exit(&self) {
        loop {
            if self.has_exited() {
                return;
            }
            // Acquire the waiter before checking again to avoid missing a notify.
            let notified = self.exit_notify.notified();
            if self.has_exited() {
                return;
            }
            notified.await;
        }
    }

    /// True if the child has exited.
    pub fn has_exited(&self) -> bool {
        lock_or_recover(&self.exit_code).is_some()
    }

    /// Exit code, if exited.
    pub fn exit_code(&self) -> Option<i32> {
        *lock_or_recover(&self.exit_code)
    }

    /// Kill the child and all descendants. Idempotent.
    pub fn terminate(&self) {
        let mut guard = lock_or_recover(&self.terminator);
        if let Some(mut terminator) = guard.take() {
            terminator.terminate();
        }
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if !self.has_exited() {
            self.terminate();
        }
    }
}
