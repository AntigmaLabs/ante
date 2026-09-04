//! Where a host is reachable.

use std::{fmt, path::PathBuf, str::FromStr};

use thiserror::Error;

/// The address of an Ante host. It names a host, never a session: which
/// session a connection drives is decided by the ops sent over it.
///
/// String form: `stdio`, `unix:<path>`, `ws://<addr>`. Each variant carries
/// exactly what its connector dials — a path, a whole URL. Credentials are
/// not part of an endpoint; they travel in connect options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// Spawn `ante serve --stdio` as a child and talk over its pipes. The
    /// child belongs to the connection: dropping the client's op side closes
    /// the child's stdin, which ends the child.
    Stdio,
    /// A Unix domain socket file served by `ante serve --sock`.
    Unix(PathBuf),
    /// A WebSocket URL (`ws://host:port`) served by `ante serve --ws`.
    Ws(String),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EndpointParseError {
    #[error("unknown endpoint `{0}`; expected `stdio`, `unix:<path>`, or `ws://<addr>`")]
    UnknownScheme(String),
    #[error("endpoint `{0}` is missing its address")]
    MissingAddress(String),
}

const UNIX_PREFIX: &str = "unix:";
const WS_PREFIX: &str = "ws://";

impl FromStr for Endpoint {
    type Err = EndpointParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "stdio" {
            return Ok(Self::Stdio);
        }
        if let Some(path) = s.strip_prefix(UNIX_PREFIX) {
            return if path.is_empty() {
                Err(EndpointParseError::MissingAddress(s.to_string()))
            } else {
                Ok(Self::Unix(PathBuf::from(path)))
            };
        }
        if let Some(addr) = s.strip_prefix(WS_PREFIX) {
            return if addr.is_empty() {
                Err(EndpointParseError::MissingAddress(s.to_string()))
            } else {
                Ok(Self::Ws(s.to_string()))
            };
        }
        Err(EndpointParseError::UnknownScheme(s.to_string()))
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdio => f.write_str("stdio"),
            Self::Unix(path) => write!(f, "{UNIX_PREFIX}{}", path.display()),
            Self::Ws(url) => f.write_str(url),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_scheme_and_round_trips() {
        for (text, expected) in [
            ("stdio", Endpoint::Stdio),
            ("unix:/tmp/ante/serve.sock", Endpoint::Unix(PathBuf::from("/tmp/ante/serve.sock"))),
            ("ws://127.0.0.1:4242", Endpoint::Ws("ws://127.0.0.1:4242".to_string())),
        ] {
            let parsed: Endpoint = text.parse().expect("valid endpoint");
            assert_eq!(parsed, expected);
            assert_eq!(parsed.to_string(), text);
        }
    }

    #[test]
    fn rejects_unknown_schemes_and_empty_addresses() {
        assert_eq!(
            "tcp://127.0.0.1:1".parse::<Endpoint>(),
            Err(EndpointParseError::UnknownScheme("tcp://127.0.0.1:1".to_string()))
        );
        assert_eq!(
            "unix:".parse::<Endpoint>(),
            Err(EndpointParseError::MissingAddress("unix:".to_string()))
        );
        assert_eq!(
            "ws://".parse::<Endpoint>(),
            Err(EndpointParseError::MissingAddress("ws://".to_string()))
        );
    }
}
