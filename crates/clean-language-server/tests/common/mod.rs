//! Shared driver for the language-server integration suites: request-document
//! builders mirroring the compiler's own fixtures, and an in-process LSP
//! client over the memory transport.
#![allow(dead_code)]

use lsp_server::{Connection, Message, Notification, Request};
use sha2::{Digest, Sha256};

pub const HOST_WIT: &str = include_str!("../../../../tests/fixtures/wit/host.wit");
pub const ROOT_URI: &str = "file:///workspace";

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// A complete request document over the vendored clean-server world, one
/// entry per `(path, content)` pair — the same JSON `cln` would hand
/// `clean-compiler --check`.
pub fn request_document(sources: &[(&str, &str)]) -> serde_json::Value {
    serde_json::json!({
        "spec_version": "1",
        "project": { "name": "fixture", "version": "0.0.1" },
        "build": { "target": "wasm32-server", "optimization": "debug" },
        "target_world": {
            "host": "clean-server",
            "version": "0.7.0",
            "world": "server",
            "sha256": sha256_hex(HOST_WIT.as_bytes()),
            "wit": HOST_WIT,
        },
        "sources": sources.iter().map(|(path, content)| serde_json::json!({
            "path": path,
            "sha256": sha256_hex(content.as_bytes()),
            "content": content,
        })).collect::<Vec<_>>(),
    })
}

/// The reference side of the parity gate: the same document through the same
/// intake and `check` operation the batch adapter dispatches.
pub fn reference_check(document: &serde_json::Value) -> Vec<clean_compiler_types::Diagnostic> {
    let mut intake = clean_compiler::diag::DiagnosticSink::new();
    let request = clean_compiler::request::from_json(&document.to_string(), &mut intake)
        .expect("reference document parses");
    assert!(!intake.has_errors(), "reference intake is clean");
    match clean_compiler::check(request) {
        Ok(diagnostics) => diagnostics,
        Err(clean_compiler::CompileError::Rejected(diagnostics)) => diagnostics,
        Err(other) => panic!("reference check did not produce diagnostics: {other}"),
    }
}

/// In-process LSP client: drives the real `run` loop over the memory
/// transport and collects what the server pushes.
pub struct Client {
    connection: Connection,
    handle: Option<std::thread::JoinHandle<Result<(), clean_language_server::ServerError>>>,
    next_id: i32,
    /// `window/logMessage` texts collected while waiting for other traffic.
    pub logs: Vec<String>,
}

/// One `textDocument/publishDiagnostics` payload, as received.
pub struct ReceivedPublish {
    pub uri: String,
    pub diagnostics: Vec<lsp_types::Diagnostic>,
}

impl Client {
    /// Starts the server and completes the `initialize` handshake with the
    /// given `initializationOptions`.
    pub fn start(initialization_options: serde_json::Value) -> Self {
        let (server, client) = Connection::memory();
        let handle = std::thread::spawn(move || clean_language_server::run(server));
        let mut this = Self {
            connection: client,
            handle: Some(handle),
            next_id: 0,
            logs: Vec::new(),
        };
        let response = this.request(
            "initialize",
            serde_json::json!({
                "capabilities": {},
                "rootUri": ROOT_URI,
                "initializationOptions": initialization_options,
            }),
        );
        response.expect("initialize succeeds");
        this.notify("initialized", serde_json::json!({}));
        this
    }

    pub fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, lsp_server::ResponseError> {
        self.next_id += 1;
        let id = self.next_id;
        self.connection
            .sender
            .send(Message::Request(Request::new(
                id.into(),
                method.to_string(),
                params,
            )))
            .expect("request sends");
        loop {
            match self.recv() {
                Message::Response(response) => {
                    assert_eq!(response.id, id.into(), "responses arrive in order");
                    return response.response_result;
                }
                Message::Notification(n) => self.stash(n),
                Message::Request(r) => panic!("unexpected server request: {}", r.method),
            }
        }
    }

    pub fn notify(&self, method: &str, params: serde_json::Value) {
        self.connection
            .sender
            .send(Message::Notification(Notification::new(
                method.to_string(),
                params,
            )))
            .expect("notification sends");
    }

    /// Collects exactly `count` diagnostic publishes, stashing any log
    /// messages that interleave.
    pub fn collect_publishes(&mut self, count: usize) -> Vec<ReceivedPublish> {
        let mut publishes = Vec::new();
        while publishes.len() < count {
            match self.recv() {
                Message::Notification(n) if n.method == "textDocument/publishDiagnostics" => {
                    let params: lsp_types::PublishDiagnosticsParams =
                        serde_json::from_value(n.params).expect("publish params parse");
                    publishes.push(ReceivedPublish {
                        uri: params.uri.as_str().to_string(),
                        diagnostics: params.diagnostics,
                    });
                }
                Message::Notification(n) => self.stash(n),
                other => panic!("expected publishDiagnostics, got {other:?}"),
            }
        }
        publishes
    }

    /// Waits until at least one `window/logMessage` has arrived.
    pub fn wait_for_log(&mut self) -> String {
        while self.logs.is_empty() {
            match self.recv() {
                Message::Notification(n) => self.stash(n),
                other => panic!("expected notification, got {other:?}"),
            }
        }
        self.logs.remove(0)
    }

    fn stash(&mut self, notification: Notification) {
        if notification.method == "window/logMessage" {
            let params: lsp_types::LogMessageParams =
                serde_json::from_value(notification.params).expect("log params parse");
            self.logs.push(params.message);
        }
        // Anything else (e.g. an unexpected publish) is dropped here;
        // suites that care collect explicitly.
    }

    fn recv(&self) -> Message {
        self.connection
            .receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("server responds within the timeout")
    }

    /// Clean shutdown; joins the server thread.
    pub fn stop(mut self) {
        self.request("shutdown", serde_json::Value::Null)
            .expect("shutdown succeeds");
        self.notify("exit", serde_json::Value::Null);
        self.handle
            .take()
            .expect("server thread present")
            .join()
            .expect("server thread joins")
            .expect("server exits cleanly");
    }
}

/// Decodes the compiler `Diagnostic` values a publish set round-trips in
/// `data` (Platform 04 §4.1.1), in publish order.
pub fn embedded_diagnostics(
    publishes: &[ReceivedPublish],
) -> Vec<clean_compiler_types::Diagnostic> {
    publishes
        .iter()
        .flat_map(|p| p.diagnostics.iter())
        .map(|d| {
            serde_json::from_value(d.data.clone().expect("data field present"))
                .expect("data field holds the §6.1 Diagnostic")
        })
        .collect()
}
