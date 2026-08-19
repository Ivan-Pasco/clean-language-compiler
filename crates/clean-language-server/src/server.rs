//! The server loop: initialize handshake, message dispatch, shutdown.
//!
//! One synchronous loop over a [`Connection`] (local ADR 0006): requests and
//! notifications are handled in arrival order, so everything the server
//! publishes is ordered and reproducible. Capabilities are declared here and
//! only here, and only once the handler behind them exists — the server never
//! advertises what it cannot serve (LSP-04's rationale: wrong information is
//! worse than none).

use lsp_server::{Connection, ErrorCode, Message, Request, Response};
use lsp_types::{
    InitializeParams, PositionEncodingKind, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("protocol: {0}")]
    Protocol(#[from] lsp_server::ProtocolError),
    #[error("malformed payload: {0}")]
    Payload(#[from] serde_json::Error),
}

/// What this build of the server can do. Grows stage by stage with the
/// handlers themselves; hover and definition enter when their handlers land.
fn capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // The compiler consumes whole `sources[].content` values (CMP-01);
        // full-document sync keeps the overlay model identical to the
        // request-document model instead of maintaining a divergent
        // incremental buffer.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        // Positions on the wire are UTF-16 code units, the LSP baseline every
        // client must support; the compiler's character columns (Platform 13
        // §2) are converted at the emission boundary (Platform 13 §7).
        position_encoding: Some(PositionEncodingKind::UTF16),
        ..ServerCapabilities::default()
    }
}

/// Drives one editor session over `connection` until `exit`.
pub fn run(connection: Connection) -> Result<(), ServerError> {
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let _params: InitializeParams = serde_json::from_value(initialize_params)?;
    let initialize_result = serde_json::json!({
        "capabilities": capabilities(),
        "serverInfo": {
            "name": "clean-language-server",
            "version": env!("CARGO_PKG_VERSION"),
        },
    });
    connection.initialize_finish(initialize_id, initialize_result)?;

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                respond_method_not_found(&connection, request)?;
            }
            // Unknown notifications are dropped by protocol rule; the
            // document-lifecycle notifications get handlers in the session
            // stage.
            Message::Notification(_) => {}
            // The server sends no requests yet, so no response is expected.
            Message::Response(_) => {}
        }
    }
    Ok(())
}

/// Every request without a handler gets the protocol's MethodNotFound error —
/// never a fabricated empty result, which would misreport a capability.
fn respond_method_not_found(connection: &Connection, request: Request) -> Result<(), ServerError> {
    let response = Response::new_err(
        request.id,
        ErrorCode::MethodNotFound as i32,
        format!("method not implemented: {}", request.method),
    );
    connection.sender.send(Message::Response(response)).ok();
    Ok(())
}
