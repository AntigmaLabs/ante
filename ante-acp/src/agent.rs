//! The ACP agent: answers the client's requests over stdio.

use std::path::PathBuf;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{Implementation, InitializeRequest, InitializeResponse};
use agent_client_protocol::{Agent, Error, ErrorCode, Stdio, on_receive_request};

use crate::ante_bin;

/// Serve ACP on this process's stdin/stdout until the client closes them.
pub async fn run(executable: PathBuf) -> Result<(), Error> {
    Agent
        .builder()
        .name("ante-acp")
        .on_receive_request(
            async move |_request: InitializeRequest, responder, _connection| {
                match ante_bin::check_version(&executable).await {
                    Ok(()) => responder
                        .respond(InitializeResponse::new(ProtocolVersion::V1).agent_info(
                            Implementation::new("ante-acp", env!("CARGO_PKG_VERSION")),
                        )),
                    Err(error) => responder.respond_with_error(Error::new(
                        ErrorCode::InternalError.into(),
                        format!("{error:#}"),
                    )),
                }
            },
            on_receive_request!(),
        )
        .connect_to(Stdio::new())
        .await
}
