// Copyright 2026 Antigma Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{HeadTailBuffer, OutputChunk};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::{RecvError, TryRecvError};

/// Ergonomic wrapper around a broadcast receiver of process output.
pub struct OutputReceiver {
    rx: broadcast::Receiver<OutputChunk>,
}

impl OutputReceiver {
    pub fn new(rx: broadcast::Receiver<OutputChunk>) -> Self {
        Self { rx }
    }

    /// Receive the next available chunk, skipping lagged messages.
    pub async fn recv(&mut self) -> Option<OutputChunk> {
        loop {
            match self.rx.recv().await {
                Ok(chunk) => return Some(chunk),
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => return None,
            }
        }
    }

    /// Drain all immediately available chunks without blocking.
    pub fn try_drain(&mut self) -> Vec<OutputChunk> {
        let mut drained = Vec::new();
        self.drain_with(|chunk| drained.push(chunk));
        drained
    }

    pub(crate) fn drain_with<F>(&mut self, mut on_chunk: F)
    where
        F: FnMut(OutputChunk),
    {
        loop {
            match self.rx.try_recv() {
                Ok(chunk) => on_chunk(chunk),
                Err(TryRecvError::Lagged(_)) => {}
                Err(TryRecvError::Empty | TryRecvError::Closed) => return,
            }
        }
    }

    /// Drain all immediately available chunks into a buffer, merging streams.
    pub fn drain_into(&mut self, buf: &mut HeadTailBuffer) {
        self.drain_with(|chunk| buf.push_chunk(chunk.data));
    }
}

impl From<broadcast::Receiver<OutputChunk>> for OutputReceiver {
    fn from(rx: broadcast::Receiver<OutputChunk>) -> Self {
        Self::new(rx)
    }
}
