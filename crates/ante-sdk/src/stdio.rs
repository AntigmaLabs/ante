//! Newline-delimited JSON transport for child processes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio as ProcessStdio};

use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

#[derive(Debug, Clone, Default)]
pub struct StdioClientOptions {
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Error)]
pub enum StdioError {
    #[error("failed to spawn command `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },

    #[error("child stdin unavailable")]
    MissingStdin,

    #[error("child stdout unavailable")]
    MissingStdout,

    #[error("stdin is already closed")]
    InputClosed,

    #[error("failed to write to child stdin: {0}")]
    StdinWrite(#[source] std::io::Error),

    #[error("failed to read child stdout: {0}")]
    StdoutRead(#[source] std::io::Error),

    #[error("failed to query child process status: {0}")]
    ProcessStatus(#[source] std::io::Error),

    #[error("failed to encode JSON payload: {0}")]
    EncodeJson(#[source] serde_json::Error),

    #[error("failed to decode JSON payload: {source}; line={line}")]
    DecodeJson {
        line: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("child stdout closed (exit code: {0:?})")]
    UnexpectedEof(Option<i32>),
}

pub struct Stdio {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout_lines: Lines<BufReader<ChildStdout>>,
    exit_status: Option<ExitStatus>,
}

impl Stdio {
    pub fn spawn(
        command: impl Into<PathBuf>,
        args: Vec<String>,
        options: StdioClientOptions,
    ) -> Result<Self, StdioError> {
        let command = command.into();

        let mut process = Command::new(&command);
        process
            .args(&args)
            .stdin(ProcessStdio::piped())
            .stdout(ProcessStdio::piped())
            .stderr(ProcessStdio::inherit());

        if let Some(cwd) = options.cwd {
            process.current_dir(cwd);
        }

        for (key, value) in options.env {
            process.env(key, value);
        }

        let mut child = process.spawn().map_err(|source| StdioError::Spawn {
            command: command.display().to_string(),
            source,
        })?;

        let stdin = child.stdin.take().ok_or(StdioError::MissingStdin)?;
        let stdout = child.stdout.take().ok_or(StdioError::MissingStdout)?;

        Ok(Self {
            child,
            stdin: Some(stdin),
            stdout_lines: BufReader::new(stdout).lines(),
            exit_status: None,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    pub fn is_input_closed(&self) -> bool {
        self.stdin.is_none()
    }

    pub fn poll_exit_code(&mut self) -> Result<Option<i32>, StdioError> {
        self.refresh_exit_status()?;
        Ok(self.exit_status.as_ref().and_then(ExitStatus::code))
    }

    pub async fn send_json<T>(&mut self, value: &T) -> Result<(), StdioError>
    where
        T: Serialize,
    {
        let mut line = serde_json::to_vec(value).map_err(StdioError::EncodeJson)?;
        line.push(b'\n');

        let stdin = self.stdin.as_mut().ok_or(StdioError::InputClosed)?;
        stdin.write_all(&line).await.map_err(StdioError::StdinWrite)?;
        stdin.flush().await.map_err(StdioError::StdinWrite)?;

        Ok(())
    }

    pub async fn read_json<T>(&mut self) -> Result<T, StdioError>
    where
        T: DeserializeOwned,
    {
        loop {
            match self.stdout_lines.next_line().await.map_err(StdioError::StdoutRead)? {
                Some(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let parsed = serde_json::from_str(trimmed).map_err(|source| {
                        StdioError::DecodeJson { line: trimmed.to_string(), source }
                    })?;
                    return Ok(parsed);
                }
                None => {
                    let exit_code = self.poll_exit_code()?;
                    return Err(StdioError::UnexpectedEof(exit_code));
                }
            }
        }
    }

    pub async fn close_input(&mut self) -> Result<(), StdioError> {
        if let Some(mut stdin) = self.stdin.take() {
            stdin.shutdown().await.map_err(StdioError::StdinWrite)?;
        }
        Ok(())
    }

    pub async fn wait(&mut self) -> Result<Option<i32>, StdioError> {
        if self.exit_status.is_none() {
            let status = self.child.wait().await.map_err(StdioError::ProcessStatus)?;
            self.exit_status = Some(status);
        }

        Ok(self.exit_status.as_ref().and_then(ExitStatus::code))
    }

    pub async fn shutdown(&mut self) -> Result<(), StdioError> {
        let _ = self.close_input().await;

        if self.poll_exit_code()?.is_none() {
            let _ = self.child.start_kill();
            let _ = self.wait().await?;
        }

        Ok(())
    }

    fn refresh_exit_status(&mut self) -> Result<(), StdioError> {
        if self.exit_status.is_none()
            && let Some(status) = self.child.try_wait().map_err(StdioError::ProcessStatus)?
        {
            self.exit_status = Some(status);
        }

        Ok(())
    }
}

impl Drop for Stdio {
    fn drop(&mut self) {
        let _ = self.stdin.take();

        if self.exit_status.is_some() {
            return;
        }

        if let Ok(Some(status)) = self.child.try_wait() {
            self.exit_status = Some(status);
            return;
        }

        let _ = self.child.start_kill();
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    #[tokio::test]
    async fn test_stdio_roundtrip() {
        let mut client =
            Stdio::spawn(PathBuf::from("cat"), Vec::new(), StdioClientOptions::default())
                .expect("spawn cat");

        let payload = json!({ "hello": "world" });
        client.send_json(&payload).await.expect("write json");
        let echoed: Value = client.read_json().await.expect("read json");

        assert_eq!(echoed, payload);
        assert!(!client.is_input_closed());

        client.close_input().await.expect("close input");
        assert!(client.is_input_closed());

        client.shutdown().await.expect("shutdown");
    }
}
