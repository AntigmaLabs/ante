use serde::Deserialize;

/// Per-channel access policy controlling which sender IDs may interact.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "AllowPolicyWire")]
pub enum AllowPolicy {
    /// Anyone may interact. Serialized as the string `"open"`.
    Open,
    /// Only the listed sender IDs may interact. Serialized as a JSON array.
    List(Vec<String>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AllowPolicyWire {
    Open(String),
    List(Vec<String>),
}

impl TryFrom<AllowPolicyWire> for AllowPolicy {
    type Error = String;

    fn try_from(value: AllowPolicyWire) -> Result<Self, Self::Error> {
        match value {
            AllowPolicyWire::Open(value) if value == "open" => Ok(Self::Open),
            AllowPolicyWire::Open(value) => Err(format!("expected \"open\", got \"{value}\"")),
            AllowPolicyWire::List(ids) => Ok(Self::List(ids)),
        }
    }
}

impl Default for AllowPolicy {
    /// Deny all by default.
    fn default() -> Self {
        Self::List(Vec::new())
    }
}

impl AllowPolicy {
    /// Check whether a sender is allowed by this policy.
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        match self {
            Self::Open => true,
            Self::List(ids) => ids.iter().any(|id| id == sender_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_policy() {
        let policy = AllowPolicy::List(vec!["U123".to_string(), "U456".to_string()]);
        assert!(policy.is_allowed("U123"));
        assert!(policy.is_allowed("U456"));
        assert!(!policy.is_allowed("U999"));
    }

    #[test]
    fn open_policy() {
        let policy = AllowPolicy::Open;
        assert!(policy.is_allowed("anyone"));
    }

    #[test]
    fn default_denies_all() {
        let policy = AllowPolicy::default();
        assert!(!policy.is_allowed("anyone"));
    }

    #[test]
    fn deserialize_list() {
        let policy: AllowPolicy = serde_json::from_str(r#"["U1", "U2"]"#).unwrap();
        assert!(policy.is_allowed("U1"));
        assert!(!policy.is_allowed("U3"));
    }

    #[test]
    fn deserialize_open_string() {
        let policy: AllowPolicy = serde_json::from_str(r#""open""#).unwrap();
        assert!(policy.is_allowed("anyone"));
    }

    #[test]
    fn deserialize_invalid_string_fails() {
        let result = serde_json::from_str::<AllowPolicy>(r#""closed""#);
        assert_eq!(result.unwrap_err().to_string(), "expected \"open\", got \"closed\"");
    }
}
