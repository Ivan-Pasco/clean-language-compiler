//! Shared fixture builders for the integration suites. Each test binary
//! compiles this module independently, so not every helper is used by every
//! binary.
#![allow(dead_code)]

use clean_compiler_types::request::CompileRequest;
use sha2::{Digest, Sha256};

pub const HOST_WIT: &str = include_str!("../../../../tests/fixtures/wit/host.wit");

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// The smallest request the schema accepts: one source file, the vendored
/// clean-server contract as the target world, defaults everywhere else.
pub fn minimal_valid_request() -> CompileRequest {
    let content = "functions:\n\tvoid init()\n\t\treturn\n";
    serde_json::from_value(serde_json::json!({
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
        "sources": [{
            "path": "app/main.cln",
            "sha256": sha256_hex(content.as_bytes()),
            "content": content,
        }],
    }))
    .expect("minimal fixture matches the request schema")
}

/// Same request as JSON text, for the intake (`from_json`) tests.
pub fn minimal_valid_request_json() -> String {
    serde_json::to_string(&minimal_valid_request()).expect("request serializes")
}

/// Assembles a fixture handler module (ADR-0003 wire ABI) that ignores its
/// input and returns `envelope` verbatim.
pub fn handler_wasm(envelope: &str) -> Vec<u8> {
    let escaped = envelope.replace('\\', "\\\\").replace('"', "\\\"");
    let pages = 2 + envelope.len() as u32 / 65536;
    let wat = format!(
        r#"(module
  (memory (export "memory") {pages})
  (data (i32.const 1024) "{escaped}")
  (func (export "alloc") (param i32) (result i32) (i32.const 65536))
  (func (export "expand") (param i32 i32) (result i32)
    (i32.store (i32.const 0) (i32.const 1024))
    (i32.store (i32.const 4) (i32.const {len}))
    (i32.const 0)))"#,
        len = envelope.len(),
    );
    wat::parse_str(&wat).expect("fixture handler wat assembles")
}

/// A library manifest whose handler returns `envelope` for every block.
pub fn handler_manifest(
    name: &str,
    handles: &[&str],
    envelope: &str,
) -> clean_compiler_types::request::LibraryManifest {
    use base64::Engine as _;
    let wasm = handler_wasm(envelope);
    clean_compiler_types::request::LibraryManifest {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        wit: String::new(),
        handles_blocks: handles.iter().map(|s| s.to_string()).collect(),
        compiletime_wasm_sha256: sha256_hex(&wasm),
        compiletime_wasm: Some(base64::engine::general_purpose::STANDARD.encode(&wasm)),
    }
}

/// The no-op expansion: the block contributes nothing (`ir.empty()`).
pub const EMPTY_ENVELOPE: &str = r#"{"ir":{"kind":"empty"},"diagnostics":[]}"#;
