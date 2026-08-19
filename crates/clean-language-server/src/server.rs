//! The server loop: initialize handshake, message dispatch, shutdown.
//!
//! One synchronous loop over a [`Connection`] (local ADR 0006): requests and
//! notifications are handled in arrival order, so everything the server
//! publishes is ordered and reproducible. Capabilities are declared here and
//! only here, and only once the handler behind them exists — the server never
//! advertises what it cannot serve (LSP-04's rationale: wrong information is
//! worse than none).

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, LogMessageParams, MessageType, PositionEncodingKind,
    PublishDiagnosticsParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use thiserror::Error;

use crate::session::Session;

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
        // Platform 04 §4.1: type of the expression under the cursor and
        // function signatures, from the pipeline's own typed program.
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        ..ServerCapabilities::default()
    }
}

/// What `initialize` produced: a live session, or the intake diagnostics
/// explaining why the request document was rejected (published once, right
/// after the handshake, under the request-document URI).
// One State exists per process; the variant size spread is irrelevant.
#[allow(clippy::large_enum_variant)]
enum State {
    Session(Session),
    Rejected {
        uri: lsp_types::Uri,
        diagnostics: Vec<lsp_types::Diagnostic>,
    },
    /// The caller sent no request document. Under CMP-01 that is the
    /// caller's defect, not a prompt to go discover files; the server says
    /// so once and serves protocol lifecycle only.
    NoRequest,
}

impl State {
    #[allow(deprecated)] // `root_uri` is deprecated in LSP but still the common field.
    fn from_initialize(params: InitializeParams) -> Self {
        let root = params
            .root_uri
            .as_ref()
            .map(|uri| uri.as_str().to_string())
            .or_else(|| {
                params
                    .workspace_folders
                    .as_ref()
                    .and_then(|folders| folders.first())
                    .map(|folder| folder.uri.as_str().to_string())
            });
        let options = params.initialization_options.unwrap_or_default();
        let request_uri = options
            .get("requestDocumentUri")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let Some(document) = options.get("requestDocument") else {
            return State::NoRequest;
        };
        // The document goes through the same intake as the batch adapter
        // (`request::from_json`), so a malformed payload yields the same
        // RQD diagnostics `cln check` would report.
        let json = document.to_string();
        let mut intake = clean_compiler::diag::DiagnosticSink::new();
        match clean_compiler::request::from_json(&json, &mut intake) {
            Some(request) if !intake.has_errors() => {
                State::Session(Session::new(request, root, request_uri))
            }
            _ => {
                let diagnostics = clean_compiler::diag::finalize(
                    intake.into_diagnostics(),
                    &clean_compiler::diag::SourceCache::empty(),
                );
                let placeholder = Session::new(empty_request_placeholder(), root, request_uri);
                let publishes = placeholder.bucket_for_intake(&diagnostics);
                State::Rejected {
                    uri: publishes.0,
                    diagnostics: publishes.1,
                }
            }
        }
    }
}

/// Drives one editor session over `connection` until `exit`.
pub fn run(connection: Connection) -> Result<(), ServerError> {
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(initialize_params)?;
    let initialize_result = serde_json::json!({
        "capabilities": capabilities(),
        "serverInfo": {
            "name": "clean-language-server",
            "version": env!("CARGO_PKG_VERSION"),
        },
    });
    connection.initialize_finish(initialize_id, initialize_result)?;

    // `initialize_finish` has already consumed the `initialized`
    // notification, so the first diagnostics push happens here: the request
    // document arrived complete at `initialize`, and Platform 04 §4.1 wants
    // diagnostics streaming without waiting for an edit.
    let mut state = State::from_initialize(params);
    match &mut state {
        State::Session(session) => publish_check(&connection, session)?,
        State::Rejected { uri, diagnostics } => {
            publish(&connection, uri.clone(), diagnostics.clone());
        }
        State::NoRequest => {
            log(
                &connection,
                "no `requestDocument` in initializationOptions; \
                 the request document is the compilation unit (CMP-01) and \
                 diagnostics are unavailable without one"
                    .to_string(),
            );
        }
    }

    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                handle_request(&connection, &state, request);
            }
            Message::Notification(notification) => {
                handle_notification(&connection, &mut state, notification)?;
            }
            // The server sends no requests yet, so no response is expected.
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    state: &mut State,
    notification: Notification,
) -> Result<(), ServerError> {
    match notification.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params)?;
            document_changed(
                connection,
                state,
                params.text_document.uri.as_str(),
                params.text_document.text,
            )
        }
        "textDocument/didChange" => {
            let mut params: DidChangeTextDocumentParams =
                serde_json::from_value(notification.params)?;
            // Full sync (declared capability): the last change carries the
            // whole document.
            let Some(change) = params.content_changes.pop() else {
                return Ok(());
            };
            document_changed(
                connection,
                state,
                params.text_document.uri.as_str(),
                change.text,
            )
        }
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params)?;
            if let State::Session(session) = state {
                session.close(params.text_document.uri.as_str());
                return publish_check(connection, session);
            }
            Ok(())
        }
        // Unknown notifications are dropped, per protocol rule.
        _ => Ok(()),
    }
}

fn document_changed(
    connection: &Connection,
    state: &mut State,
    uri: &str,
    text: String,
) -> Result<(), ServerError> {
    let State::Session(session) = state else {
        return Ok(());
    };
    if session.open_or_change(uri, text) {
        publish_check(connection, session)?;
    } else {
        log(
            connection,
            format!("document is not a request source; not compiled: {uri}"),
        );
    }
    Ok(())
}

/// Re-checks the session's effective request and pushes the result: every
/// bucket, every round (stale diagnostics clear), pre-v1 unsupported notes as
/// log messages — the LSP face of the batch adapter's stderr/exit-3 split.
fn publish_check(connection: &Connection, session: &mut Session) -> Result<(), ServerError> {
    let outcome = session.check();
    for message in outcome.logs {
        log(connection, message);
    }
    for entry in outcome.publishes {
        publish(connection, entry.uri, entry.diagnostics);
    }
    Ok(())
}

fn publish(connection: &Connection, uri: lsp_types::Uri, diagnostics: Vec<lsp_types::Diagnostic>) {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    let notification = Notification::new(
        "textDocument/publishDiagnostics".to_string(),
        serde_json::to_value(params).expect("params serialize"),
    );
    connection
        .sender
        .send(Message::Notification(notification))
        .ok();
}

fn log(connection: &Connection, message: String) {
    let params = LogMessageParams {
        typ: MessageType::WARNING,
        message,
    };
    let notification = Notification::new(
        "window/logMessage".to_string(),
        serde_json::to_value(params).expect("params serialize"),
    );
    connection
        .sender
        .send(Message::Notification(notification))
        .ok();
}

/// Dispatches the request surface. Each handler answers from the session's
/// latest pipeline run; a request that arrives without a session (no request
/// document) gets its capability's empty answer, never an invented one.
fn handle_request(connection: &Connection, state: &State, request: Request) {
    match request.method.as_str() {
        "textDocument/hover" => {
            let id = request.id.clone();
            let params: lsp_types::HoverParams = match serde_json::from_value(request.params) {
                Ok(params) => params,
                Err(err) => return respond_invalid_params(connection, id, err),
            };
            let result = match state {
                State::Session(session) => session.hover(
                    params
                        .text_document_position_params
                        .text_document
                        .uri
                        .as_str(),
                    params.text_document_position_params.position,
                ),
                _ => None,
            };
            respond_ok(connection, id, result);
        }
        _ => respond_method_not_found(connection, request),
    }
}

fn respond_ok(connection: &Connection, id: lsp_server::RequestId, result: impl serde::Serialize) {
    let response = Response::new_ok(id, result);
    connection.sender.send(Message::Response(response)).ok();
}

fn respond_invalid_params(
    connection: &Connection,
    id: lsp_server::RequestId,
    err: serde_json::Error,
) {
    let response = Response::new_err(
        id,
        ErrorCode::InvalidParams as i32,
        format!("invalid params: {err}"),
    );
    connection.sender.send(Message::Response(response)).ok();
}

/// Every request without a handler gets the protocol's MethodNotFound error —
/// never a fabricated empty result, which would misreport a capability.
fn respond_method_not_found(connection: &Connection, request: Request) {
    let response = Response::new_err(
        request.id,
        ErrorCode::MethodNotFound as i32,
        format!("method not implemented: {}", request.method),
    );
    connection.sender.send(Message::Response(response)).ok();
}

/// Placeholder for the rejected-intake path: `Session` only needs the URI
/// plumbing, and there is no valid request to hold.
fn empty_request_placeholder() -> clean_compiler_types::request::CompileRequest {
    serde_json::from_value(serde_json::json!({
        "spec_version": "1",
        "project": { "name": "", "version": "" },
        "build": { "target": "", "optimization": "" },
        "target_world": { "host": "", "version": "", "world": "", "sha256": "", "wit": "" },
        "sources": [],
    }))
    .expect("placeholder matches the request schema")
}
