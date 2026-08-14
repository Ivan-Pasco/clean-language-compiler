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
    let content = "functions:\n\tinit()\n";
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
