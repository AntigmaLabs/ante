use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Copy)]
pub struct Id {
    prefix: [u8; Self::PREFIX_SIZE],
    id: Ulid,
}

impl Id {
    pub const PREFIX_SIZE: usize = 4;

    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Self { prefix: Self::clamp_prefix(id), id: Ulid::generate() }
    }

    fn clamp_prefix<S: AsRef<str>>(id: S) -> [u8; Self::PREFIX_SIZE] {
        let mut prefix = [0u8; Self::PREFIX_SIZE];
        let bytes = id.as_ref().as_bytes();
        let len = std::cmp::min(bytes.len(), Self::PREFIX_SIZE);
        prefix[..len].copy_from_slice(&bytes[..len]);
        prefix
    }

    pub fn op() -> Self {
        Self::new("op")
    }

    pub fn evt() -> Self {
        Self::new("evt")
    }

    pub fn ses() -> Self {
        Self::new("ses")
    }

    pub fn step() -> Self {
        Self::new("step")
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &b in self.prefix.iter().take_while(|&&b| b != 0) {
            write!(f, "{}", b as char)?;
        }
        write!(f, "_{}", self.id)
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseIdError {
    MissingSeparator,
    InvalidUlid,
}

impl std::fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSeparator => write!(f, "missing separator"),
            Self::InvalidUlid => write!(f, "invalid ULID"),
        }
    }
}

impl std::error::Error for ParseIdError {}

impl std::str::FromStr for Id {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (prefix_str, ulid_str) = s.split_once('_').ok_or(ParseIdError::MissingSeparator)?;
        let id = Ulid::from_string(ulid_str).map_err(|_| ParseIdError::InvalidUlid)?;
        Ok(Self { prefix: Self::clamp_prefix(prefix_str), id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_id_serde_roundtrip() {
        let id = Id::new("evt");
        let serialized = serde_json::to_string(&id).expect("serialize id");
        assert_eq!(serialized, format!("\"{id}\""));

        let deserialized: Id = serde_json::from_str(&serialized).expect("deserialize id");
        assert_eq!(deserialized, id);
    }

    #[test]
    fn test_id_from_str() {
        let id = Id::new("ses");
        let parsed = Id::from_str(&id.to_string()).expect("parse id");
        assert_eq!(parsed, id);
    }
}
