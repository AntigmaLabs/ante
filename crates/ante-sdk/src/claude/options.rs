//! Claude Code process and session options.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::stdio::StdioClientOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    Plan,
    BypassPermissions,
}

impl PermissionMode {
    pub fn as_cli_arg(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::Plan => "plan",
            Self::BypassPermissions => "bypassPermissions",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolConfig {
    Default,
    Custom(Vec<String>),
}

#[derive(Debug, Clone, Default)]
pub struct ClaudeOptions {
    pub cli_path: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    /// Pinned session UUID passed to the CLI via `--session-id`. Leave `None`
    /// to let the CLI assign one (reported back on the `system(init)` frame).
    pub session_id: Option<String>,
    pub system_prompt: Option<String>,
    pub append_system_prompt: Option<String>,
    pub tools: Option<ToolConfig>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub permission_mode: Option<PermissionMode>,
    pub continue_conversation: bool,
    pub resume: Option<String>,
    pub max_turns: Option<u32>,
    pub max_budget_usd: Option<f64>,
    pub model: Option<String>,
    pub fallback_model: Option<String>,
    pub settings: Option<String>,
    pub add_dirs: Vec<PathBuf>,
    pub extra_args: BTreeMap<String, Option<String>>,
}

impl ClaudeOptions {
    pub(crate) fn resolve_cli_path(&self) -> Result<PathBuf, which::Error> {
        match &self.cli_path {
            Some(cli_path) => Ok(cli_path.clone()),
            None => which::which("claude"),
        }
    }

    pub(crate) fn build_args(&self) -> Vec<String> {
        let mut args = vec![
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
            "--system-prompt".to_string(),
            self.system_prompt.clone().unwrap_or_default(),
        ];

        if let Some(append_system_prompt) = &self.append_system_prompt {
            args.push("--append-system-prompt".to_string());
            args.push(append_system_prompt.clone());
        }

        if let Some(tools) = &self.tools {
            args.push("--tools".to_string());
            args.push(match tools {
                ToolConfig::Default => "default".to_string(),
                ToolConfig::Custom(tools) => tools.join(","),
            });
        }

        if !self.allowed_tools.is_empty() {
            args.push("--allowedTools".to_string());
            args.push(self.allowed_tools.join(","));
        }

        if let Some(max_turns) = self.max_turns {
            args.push("--max-turns".to_string());
            args.push(max_turns.to_string());
        }

        if let Some(max_budget_usd) = self.max_budget_usd {
            args.push("--max-budget-usd".to_string());
            args.push(max_budget_usd.to_string());
        }

        if !self.disallowed_tools.is_empty() {
            args.push("--disallowedTools".to_string());
            args.push(self.disallowed_tools.join(","));
        }

        if let Some(model) = &self.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        if let Some(fallback_model) = &self.fallback_model {
            args.push("--fallback-model".to_string());
            args.push(fallback_model.clone());
        }

        if let Some(permission_mode) = &self.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(permission_mode.as_cli_arg().to_string());
        }

        if self.continue_conversation {
            args.push("--continue".to_string());
        }

        if let Some(resume) = &self.resume {
            args.push("--resume".to_string());
            args.push(resume.clone());
        }

        if let Some(session_id) = &self.session_id {
            args.push("--session-id".to_string());
            args.push(session_id.clone());
        }

        if let Some(settings) = &self.settings {
            args.push("--settings".to_string());
            args.push(settings.clone());
        }

        for add_dir in &self.add_dirs {
            args.push("--add-dir".to_string());
            args.push(add_dir.display().to_string());
        }

        for (flag, value) in &self.extra_args {
            if flag.starts_with('-') {
                args.push(flag.clone());
            } else {
                args.push(format!("--{flag}"));
            }

            if let Some(value) = value {
                args.push(value.clone());
            }
        }

        args.push("--input-format".to_string());
        args.push("stream-json".to_string());
        args
    }

    pub(crate) fn stdio_options(&self) -> StdioClientOptions {
        let env = self.env.clone();

        StdioClientOptions { cwd: self.cwd.clone(), env }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn build_args_matches_expected_cli_shape() {
        let mut options = ClaudeOptions {
            system_prompt: Some("You are helpful".to_string()),
            append_system_prompt: Some("Use concise output".to_string()),
            tools: Some(ToolConfig::Default),
            allowed_tools: vec!["Read".to_string(), "Write".to_string()],
            disallowed_tools: vec!["Bash".to_string()],
            permission_mode: Some(PermissionMode::AcceptEdits),
            continue_conversation: true,
            resume: Some("session-123".to_string()),
            session_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            max_turns: Some(3),
            max_budget_usd: Some(1.25),
            model: Some("claude-sonnet-4-5".to_string()),
            fallback_model: Some("claude-haiku-4-5".to_string()),
            settings: Some("{\"theme\":\"plain\"}".to_string()),
            add_dirs: vec![PathBuf::from("/tmp/project"), PathBuf::from("/tmp/docs")],
            ..ClaudeOptions::default()
        };
        options.extra_args.insert("replay-user-messages".to_string(), None);
        options.extra_args.insert("output-style".to_string(), Some("plain".to_string()));

        let args = options.build_args();

        assert_eq!(
            args,
            vec![
                "--output-format",
                "stream-json",
                "--verbose",
                "--system-prompt",
                "You are helpful",
                "--append-system-prompt",
                "Use concise output",
                "--tools",
                "default",
                "--allowedTools",
                "Read,Write",
                "--max-turns",
                "3",
                "--max-budget-usd",
                "1.25",
                "--disallowedTools",
                "Bash",
                "--model",
                "claude-sonnet-4-5",
                "--fallback-model",
                "claude-haiku-4-5",
                "--permission-mode",
                "acceptEdits",
                "--continue",
                "--resume",
                "session-123",
                "--session-id",
                "550e8400-e29b-41d4-a716-446655440000",
                "--settings",
                "{\"theme\":\"plain\"}",
                "--add-dir",
                "/tmp/project",
                "--add-dir",
                "/tmp/docs",
                "--output-style",
                "plain",
                "--replay-user-messages",
                "--input-format",
                "stream-json",
            ]
        );
    }
}
