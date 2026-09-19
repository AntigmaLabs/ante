//! `initialize` over real stdio against the built binary, driving a stand-in
//! `ante` shell script that only has to answer `--version`.

#![cfg(unix)]

use std::path::PathBuf;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{InitializeRequest, InitializeResponse};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client, ConnectionTo, Error};

/// Write `body` as an executable `ante` script in a directory unique to `name`.
fn fake_ante(name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("ante-acp-test-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ante");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

async fn initialize_with(fake: PathBuf) -> Result<InitializeResponse, Error> {
    let config = AcpAgentConfig::new(env!("CARGO_BIN_EXE_ante-acp"))
        .env("ANTE", fake.display().to_string())
        // An empty value re-enables the check if the developer's shell skips it.
        .env("ANTE_ACP_SKIP_VERSION_CHECK", "");
    Client
        .builder()
        .connect_with(AcpAgent::new(config), |cx: ConnectionTo<Agent>| async move {
            cx.send_request(InitializeRequest::new(ProtocolVersion::V1)).block_task().await
        })
        .await
}

#[tokio::test]
async fn initialize_reports_the_adapter() {
    let response = initialize_with(fake_ante("current", "echo 'ante 0.2.1'"))
        .await
        .expect("initialize succeeds");

    assert_eq!(response.protocol_version, ProtocolVersion::V1);
    let info = response.agent_info.expect("agent info");
    assert_eq!(info.name, "ante-acp");
    assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn initialize_refuses_an_old_ante() {
    let error = initialize_with(fake_ante("old", "echo 'ante 0.1.0'"))
        .await
        .expect_err("old ante is refused");

    assert!(error.message.contains("ante 0.1.0"), "{}", error.message);
    assert!(error.message.contains("0.2.1"), "{}", error.message);
}

#[tokio::test]
async fn initialize_refuses_a_failing_ante() {
    let error = initialize_with(fake_ante("failing", "echo 'ante 0.2.1'; exit 3"))
        .await
        .expect_err("a failing ante is refused even if it prints a version");

    assert!(error.message.contains("failed with"), "{}", error.message);
}
