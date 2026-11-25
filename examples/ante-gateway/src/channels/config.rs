use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::access::AllowPolicy;

/// Per-channel instance configuration, keyed by platform id in the top-level map.
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelInstanceConfig {
    pub enabled: bool,
    /// Access policy. A JSON array of sender IDs deserializes as
    /// `AllowPolicy::List`. The literal string `"open"` deserializes as
    /// `AllowPolicy::Open`. Defaults to deny-all when absent.
    #[serde(default)]
    pub allow_from: AllowPolicy,
    /// All remaining fields are platform-specific secrets / settings.
    /// Values prefixed with `"env:"` are resolved from the environment.
    #[serde(flatten)]
    pub secrets: HashMap<String, serde_json::Value>,
}

impl ChannelInstanceConfig {
    /// Resolve a secret value. If the stored value starts with `"env:"`, look
    /// it up from the environment. Returns `None` if the key is missing or the
    /// value is not a string.
    pub fn resolve_secret(&self, key: &str) -> Option<String> {
        let val = self.secrets.get(key)?.as_str()?;
        if let Some(var) = val.strip_prefix("env:") {
            std::env::var(var).ok()
        } else {
            Some(val.to_string())
        }
    }
}

/// Top-level channels configuration: `{ "slack": { ... }, "discord": { ... } }`.
pub type ChannelsConfig = HashMap<String, ChannelInstanceConfig>;

/// Load and parse the channels configuration file.
pub fn load_channels_config(path: &Path) -> Result<ChannelsConfig> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read channels config: {}", path.display()))?;
    let config: ChannelsConfig =
        serde_json::from_str(&data).context("failed to parse channels config")?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let json = r#"{
            "slack": {
                "enabled": true,
                "bot_token": "xoxb-test",
                "allow_from": ["U123"]
            }
        }"#;
        let config: ChannelsConfig = serde_json::from_str(json).unwrap();
        let slack = config.get("slack").unwrap();
        assert!(slack.enabled);
        assert_eq!(slack.resolve_secret("bot_token"), Some("xoxb-test".to_string()));
        assert!(slack.allow_from.is_allowed("U123"));
        assert!(!slack.allow_from.is_allowed("U999"));
    }

    #[test]
    fn parse_open_allow_policy() {
        let json = r#"{
            "test": {
                "enabled": true,
                "allow_from": "open"
            }
        }"#;
        let config: ChannelsConfig = serde_json::from_str(json).unwrap();
        let test = config.get("test").unwrap();
        assert!(test.allow_from.is_allowed("anyone"));
    }

    #[test]
    fn default_allow_policy_denies_all() {
        let json = r#"{ "test": { "enabled": true } }"#;
        let config: ChannelsConfig = serde_json::from_str(json).unwrap();
        let test = config.get("test").unwrap();
        assert!(!test.allow_from.is_allowed("anyone"));
    }

    #[test]
    fn resolve_env_secret() {
        // SAFETY: test runs serially (via serial_test or single-threaded test runner).
        unsafe {
            std::env::set_var("TEST_CHANNEL_TOKEN_89012", "secret-value");
        }
        let json = r#"{
            "test": {
                "enabled": true,
                "token": "env:TEST_CHANNEL_TOKEN_89012"
            }
        }"#;
        let config: ChannelsConfig = serde_json::from_str(json).unwrap();
        let test = config.get("test").unwrap();
        assert_eq!(test.resolve_secret("token"), Some("secret-value".to_string()));
        unsafe {
            std::env::remove_var("TEST_CHANNEL_TOKEN_89012");
        }
    }
}
