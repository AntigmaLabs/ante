//! Locating the `ante` executable and checking its version.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};

/// Set by `ante <name>` dispatch to the absolute path of the dispatching binary.
pub const ANTE_ENV: &str = "ANTE";
/// Set to a non-empty value to skip the version check (development builds
/// of `ante` report `0.1.0`).
pub const SKIP_VERSION_CHECK_ENV: &str = "ANTE_ACP_SKIP_VERSION_CHECK";
/// The oldest `ante` this adapter drives: the release whose wire protocol it
/// is built against.
pub const MIN_VERSION: Version = Version { major: 0, minor: 2, patch: 1 };
/// Bound on `ante --version` so a hung executable cannot hang `initialize`.
const VERSION_CHECK_TIMEOUT: Duration = Duration::from_secs(10);

/// Resolve the `ante` executable: `ANTE` (set when launched as `ante acp`),
/// then `--executable`, then `ante` on `PATH`.
pub fn resolve(env: Option<OsString>, flag: Option<PathBuf>) -> PathBuf {
    env.filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or(flag)
        .unwrap_or_else(|| PathBuf::from("ante"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// Parse `ante --version` output, e.g. `ante 0.2.1`.
    pub fn parse(output: &str) -> Option<Self> {
        let mut parts = output.trim().strip_prefix("ante ")?.split('.').map(str::parse::<u64>);
        let (Some(Ok(major)), Some(Ok(minor)), Some(Ok(patch)), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        Some(Self { major, minor, patch })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Run `<executable> --version` and refuse anything older than [`MIN_VERSION`].
pub async fn check_version(executable: &Path) -> Result<()> {
    if std::env::var_os(SKIP_VERSION_CHECK_ENV).is_some_and(|value| !value.is_empty()) {
        return Ok(());
    }
    let command = format!("{} --version", executable.display());
    let output = tokio::time::timeout(
        VERSION_CHECK_TIMEOUT,
        tokio::process::Command::new(executable).arg("--version").output(),
    )
    .await
    .with_context(|| format!("`{command}` did not finish within {VERSION_CHECK_TIMEOUT:?}"))?
    .with_context(|| format!("could not run `{command}`; install Ante or pass --executable"))?;
    if !output.status.success() {
        bail!("`{command}` failed with {}", output.status);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = Version::parse(&stdout).with_context(|| {
        format!(
            "unrecognized `ante --version` output {stdout:?}; set {SKIP_VERSION_CHECK_ENV}=1 to skip this check"
        )
    })?;
    if version < MIN_VERSION {
        bail!(
            "ante {version} is older than the {MIN_VERSION} this adapter requires; run `ante update`"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_output() {
        assert_eq!(Version::parse("ante 0.2.1\n"), Some(Version { major: 0, minor: 2, patch: 1 }));
    }

    #[test]
    fn rejects_other_output() {
        for output in ["", "0.2.1", "ante 0.2", "ante 0.2.1-rc.1", "ante 20260919-1200-abc"] {
            assert_eq!(Version::parse(output), None, "{output:?}");
        }
    }

    #[test]
    fn orders_by_component() {
        assert!(Version::parse("ante 0.10.0") > Version::parse("ante 0.9.9"));
        assert!(Version::parse("ante 0.1.0") < Some(MIN_VERSION));
    }

    #[test]
    fn env_wins_then_flag_then_path() {
        let flag = Some(PathBuf::from("/flag/ante"));
        assert_eq!(resolve(Some("/env/ante".into()), flag.clone()), PathBuf::from("/env/ante"));
        assert_eq!(resolve(Some("".into()), flag.clone()), PathBuf::from("/flag/ante"));
        assert_eq!(resolve(None, flag), PathBuf::from("/flag/ante"));
        assert_eq!(resolve(None, None), PathBuf::from("ante"));
    }
}
