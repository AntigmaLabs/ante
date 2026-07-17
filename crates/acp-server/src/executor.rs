use serde_json::Value;

/// Execute Ante in headless mode and return the output text.
///
/// Spawns: `ante -p "<prompt>" --json`
/// Collects stdout until the process exits, then parses the JSON output.
pub async fn execute_ante(
    prompt: &str,
    provider: Option<&str>,
    model: Option<&str>,
) -> Result<AnteExecution, AnteExecutionError> {
    let mut cmd = tokio::process::Command::new("ante");
    cmd.arg("-p").arg(prompt);
    cmd.arg("--json");

    if let Some(provider) = provider {
        cmd.arg("--provider").arg(provider);
    }
    if let Some(model) = model {
        cmd.arg("--model").arg(model);
    }

    // Run headless, capture stdout
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let output = cmd.output().await.map_err(|e| AnteExecutionError {
        kind: AnteErrorKind::SpawnFailed,
        message: format!("failed to spawn `ante` process: {e}"),
        details: vec![format!("Command: ante -p \"<prompt>\" --json")],
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();

    if !output.status.success() {
        return Err(AnteExecutionError {
            kind: AnteErrorKind::ProcessFailed,
            message: format!("`ante` exited with code {exit_code:?}"),
            details: vec![stderr],
        });
    }

    // Try to parse the JSON output
    let parsed: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        // If it's not valid JSON, treat the whole output as plain text
        serde_json::json!({ "result": stdout.trim() })
    });

    // Extract the result text from the parsed output
    let result_text = extract_result_text(&parsed);

    Ok(AnteExecution {
        result_text,
        raw_output: parsed,
        exit_code,
    })
}

fn extract_result_text(value: &Value) -> String {
    // Try common output shapes:
    // 1. { "result": "..." }
    // 2. { "output": "..." }
    // 3. { "message": { "content": "..." } }
    // 4. Plain text fallback
    if let Some(result) = value.get("result").and_then(|v| v.as_str()) {
        return result.to_string();
    }
    if let Some(output) = value.get("output").and_then(|v| v.as_str()) {
        return output.to_string();
    }
    if let Some(content) = value.pointer("/message/content").and_then(|v| v.as_str()) {
        return content.to_string();
    }
    // Last resort: stringify the whole thing
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    serde_json::to_string_pretty(value).unwrap_or_default()
}

#[derive(Debug)]
pub struct AnteExecution {
    pub result_text: String,
    pub raw_output: Value,
    pub exit_code: Option<i32>,
}

#[derive(Debug)]
pub struct AnteExecutionError {
    pub kind: AnteErrorKind,
    pub message: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnteErrorKind {
    SpawnFailed,
    ProcessFailed,
    Timeout,
}

impl std::fmt::Display for AnteExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AnteExecutionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_result_text_from_result_key() {
        let v = serde_json::json!({ "result": "hello world" });
        assert_eq!(extract_result_text(&v), "hello world");
    }

    #[test]
    fn extract_result_text_from_output_key() {
        let v = serde_json::json!({ "output": "hello world" });
        assert_eq!(extract_result_text(&v), "hello world");
    }

    #[test]
    fn extract_result_text_from_message_content() {
        let v = serde_json::json!({ "message": { "content": "hello world" } });
        assert_eq!(extract_result_text(&v), "hello world");
    }

    #[test]
    fn extract_result_text_plain_string() {
        let v = serde_json::json!("just a string");
        assert_eq!(extract_result_text(&v), "just a string");
    }

    #[test]
    fn extract_result_text_fallback() {
        let v = serde_json::json!({ "foo": "bar", "baz": 42 });
        let result = extract_result_text(&v);
        assert!(result.contains("foo"));
        assert!(result.contains("bar"));
    }

    #[test]
    fn extract_result_text_empty_object_fallback() {
        let v = serde_json::json!({});
        let result = extract_result_text(&v);
        // Should fall through to stringify
        assert_eq!(result, "{}");
    }

    #[test]
    fn ante_execution_error_display() {
        let err = AnteExecutionError {
            kind: AnteErrorKind::SpawnFailed,
            message: "failed to spawn".into(),
            details: vec![],
        };
        assert_eq!(format!("{err}"), "failed to spawn");
    }

    #[test]
    fn ante_error_kind_equality() {
        assert_eq!(AnteErrorKind::SpawnFailed, AnteErrorKind::SpawnFailed);
        assert_ne!(AnteErrorKind::SpawnFailed, AnteErrorKind::ProcessFailed);
        assert_ne!(AnteErrorKind::ProcessFailed, AnteErrorKind::Timeout);
    }
}
