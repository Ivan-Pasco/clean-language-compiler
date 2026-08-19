//! JSON-RPC / MCP adapter (Platform 14 §14.2.3): exposes the library
//! operations over stdio so AI agents and remote hosts can call them
//! without linking against the library. The wire format for compilation is
//! the request document unchanged; every method is a wrapper over the same
//! library entry points the process adapter uses — no compilation logic
//! lives here.
//!
//! Transport: one JSON-RPC 2.0 message per line (the MCP stdio framing).
//! The same dispatch serves two dialects:
//!
//! - **Direct JSON-RPC**: `compile`, `check`, `why`, `reproBuild`,
//!   `replay`, `bridgeStub` — params and results are JSON; component bytes
//!   ride base64.
//! - **MCP**: `initialize`, `notifications/initialized`, `tools/list`,
//!   `tools/call` — each tool wraps the identical handler, so an MCP call
//!   and a direct call cannot diverge.
//!
//! Outcome model: an operation that *ran* returns a result even when the
//! program failed (a rejected compile carries its diagnostics; a diverged
//! replay says so). JSON-RPC errors are reserved for protocol misuse —
//! unknown method, malformed params, undecodable payloads.

use std::io::{BufRead, Write};

use base64::Engine as _;
use serde_json::{json, Value};

const BASE64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// Runs the serve loop until stdin closes. Never touches the filesystem:
/// everything arrives in the message and leaves in the response (CMP-01
/// holds with nothing to point at).
pub fn serve() -> std::process::ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_line(&line) {
            let mut out = stdout.lock();
            let text = serde_json::to_string(&response).expect("response serializes");
            if out
                .write_all(text.as_bytes())
                .and_then(|()| out.write_all(b"\n"))
                .and_then(|()| out.flush())
                .is_err()
            {
                break;
            }
        }
    }
    std::process::ExitCode::SUCCESS
}

/// One message in, at most one response out (notifications get none).
pub fn handle_line(line: &str) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(e) => {
            return Some(error_response(
                Value::Null,
                -32700,
                &format!("parse error: {e}"),
            ))
        }
    };
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str)?.to_string();
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    // Notifications (no id) get handled for effect only; MCP's
    // `notifications/initialized` is a no-op by design.
    let id = id?;

    let outcome = dispatch(&method, &params);
    Some(match outcome {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => error_response(id, code, &message),
    })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

type RpcError = (i64, String);

fn invalid_params(message: impl std::fmt::Display) -> RpcError {
    (-32602, format!("invalid params: {message}"))
}

fn dispatch(method: &str, params: &Value) -> Result<Value, RpcError> {
    match method {
        // ---- MCP dialect --------------------------------------------------
        "initialize" => Ok(json!({
            "protocolVersion": "2025-06-18",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "clean-compiler",
                "version": env!("CARGO_PKG_VERSION"),
            },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list()),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_params("tools/call needs a `name`"))?;
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
            let result = dispatch(name, &arguments)?;
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&result).expect("result serializes"),
                }],
                "structuredContent": result,
                "isError": false,
            }))
        }
        // ---- direct operations -------------------------------------------
        "compile" => op_compile(params),
        "check" => op_check(params),
        "why" => op_why(params),
        "reproBuild" => op_repro_build(params),
        "replay" => op_replay(params),
        "bridgeStub" => op_bridge_stub(params),
        other => Err((-32601, format!("method `{other}` not found"))),
    }
}

fn tools_list() -> Value {
    let request_tool = |name: &str, description: &str| {
        json!({
            "name": name,
            "description": description,
            "inputSchema": {
                "type": "object",
                "properties": {
                    "request": {
                        "type": "object",
                        "description": "The Platform 14 §14.1.1 request document, unchanged.",
                    },
                },
                "required": ["request"],
            },
        })
    };
    json!({
        "tools": [
            request_tool("compile", "Compile a request document to a WebAssembly component (Platform 14 §14.2.1). Returns the component (base64), build manifest, and diagnostics; a rejected program returns its diagnostics."),
            request_tool("check", "Diagnostics-only build (Platform 14 §14.14.4): passes 1-9, no codegen. Returns every diagnostic."),
            json!({
                "name": "why",
                "description": "Diagnostic re-projection (Platform 14 §14.14.1): re-present the diagnostics at a source location with pass provenance.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "diagnostics": { "type": "array", "description": "The diagnostics.json entries of the most recent build." },
                        "file": { "type": "string" },
                        "line": { "type": "integer" },
                        "column": { "type": "integer" },
                    },
                    "required": ["diagnostics", "file", "line"],
                },
            }),
            json!({
                "name": "reproBuild",
                "description": "Build reproduction (Platform 14 §14.14.6): rebuild from a build manifest, inputs served from a request document, and assert the output hash.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "manifest": { "type": "object" },
                        "request": { "type": "object" },
                        "original_wasm_base64": { "type": "string" },
                    },
                    "required": ["manifest", "request"],
                },
            }),
            json!({
                "name": "replay",
                "description": "Request replay (Platform 14 §14.14.6): re-run a captured request against the recorded component with every host value frozen to the trace.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "trace": { "type": "object" },
                        "component_wasm_base64": { "type": "string" },
                    },
                    "required": ["trace", "component_wasm_base64"],
                },
            }),
            json!({
                "name": "bridgeStub",
                "description": "Bridge stub generation (Platform 14 §14.14.5): build the stub component for one bridge interface with a response fixture baked in.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "wit": { "type": "string" },
                        "interface": { "type": "string" },
                        "fixture": { "type": "object" },
                    },
                    "required": ["wit", "interface", "fixture"],
                },
            }),
        ],
    })
}

/// Parses the `request` param through the same intake the process adapter
/// uses. `Err(Ok(diagnostics))` means intake rejected it — an outcome, not
/// a protocol error.
fn intake_request(
    params: &Value,
) -> Result<clean_compiler::types::CompileRequest, Result<Value, RpcError>> {
    let Some(request_value) = params.get("request") else {
        return Err(Err(invalid_params("missing `request`")));
    };
    let json = request_value.to_string();
    let mut intake = clean_compiler::diag::DiagnosticSink::new();
    let request = clean_compiler::request::from_json(&json, &mut intake);
    if intake.has_errors() {
        let diagnostics = clean_compiler::diag::finalize(
            intake.into_diagnostics(),
            &clean_compiler::diag::SourceCache::empty(),
        );
        return Err(Ok(json!({
            "rejected": true,
            "diagnostics": diagnostics,
        })));
    }
    Ok(request.expect("intake produced no request yet raised no error"))
}

fn op_compile(params: &Value) -> Result<Value, RpcError> {
    let request = match intake_request(params) {
        Ok(request) => request,
        Err(outcome) => return outcome,
    };
    match clean_compiler::compile(request) {
        Ok(artifact) => Ok(json!({
            "rejected": false,
            "component_wasm_base64": BASE64.encode(&artifact.wasm),
            "build_manifest": artifact.manifest,
            "diagnostics": artifact.diagnostics,
        })),
        Err(clean_compiler::CompileError::Rejected(diagnostics)) => Ok(json!({
            "rejected": true,
            "diagnostics": diagnostics,
        })),
        // Pre-v1 channels, mirrored from the process adapter's exit 3.
        Err(err) => Err((-32000, err.to_string())),
    }
}

fn op_check(params: &Value) -> Result<Value, RpcError> {
    let request = match intake_request(params) {
        Ok(request) => request,
        Err(outcome) => {
            // Intake rejection: `check`'s outcome shape carries the same
            // diagnostics under `failed`.
            return outcome.map(|value| {
                json!({
                    "failed": true,
                    "diagnostics": value["diagnostics"],
                })
            });
        }
    };
    match clean_compiler::check(request) {
        Ok(diagnostics) => {
            let failed = diagnostics
                .iter()
                .any(|d| d.level == clean_compiler::types::Level::Error);
            Ok(json!({ "failed": failed, "diagnostics": diagnostics }))
        }
        Err(err) => Err((-32000, err.to_string())),
    }
}

fn op_why(params: &Value) -> Result<Value, RpcError> {
    let diagnostics: Vec<clean_compiler::types::Diagnostic> =
        serde_json::from_value(params.get("diagnostics").cloned().unwrap_or(Value::Null))
            .map_err(|e| invalid_params(format!("`diagnostics`: {e}")))?;
    let file = params
        .get("file")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("missing `file`"))?;
    let line = params
        .get("line")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_params("missing `line`"))?;
    let column = params.get("column").and_then(Value::as_u64);
    let query = clean_compiler::WhyQuery {
        file: file.to_string(),
        line: line as u32,
        column: column.map(|c| c as u32),
    };
    let report = clean_compiler::why(&diagnostics, &query);
    Ok(serde_json::to_value(report).expect("report serializes"))
}

fn op_repro_build(params: &Value) -> Result<Value, RpcError> {
    let manifest: clean_compiler::types::BuildManifest =
        serde_json::from_value(params.get("manifest").cloned().unwrap_or(Value::Null))
            .map_err(|e| invalid_params(format!("`manifest`: {e}")))?;
    let request = match intake_request(params) {
        Ok(request) => request,
        Err(outcome) => return outcome,
    };
    let original = match params.get("original_wasm_base64").and_then(Value::as_str) {
        Some(text) => Some(
            BASE64
                .decode(text)
                .map_err(|e| invalid_params(format!("`original_wasm_base64`: {e}")))?,
        ),
        None => None,
    };
    let resolver = clean_compiler::RequestDocumentResolver::new(&request);
    match clean_compiler::repro_build(&manifest, original.as_deref(), &resolver) {
        Ok(artifact) => Ok(json!({
            "reproduced": true,
            "component_wasm_base64": BASE64.encode(&artifact.wasm),
            "build_manifest": artifact.manifest,
        })),
        Err(failure) => {
            if let Some(diagnostic) = clean_compiler::repro::divergence_diagnostic(&failure) {
                let diagnostics = clean_compiler::diag::finalize(
                    vec![diagnostic],
                    &clean_compiler::diag::SourceCache::empty(),
                );
                return Ok(json!({
                    "reproduced": false,
                    "failure": failure.to_string(),
                    "diagnostics": diagnostics,
                }));
            }
            match failure {
                clean_compiler::ReproFailure::Compile(clean_compiler::CompileError::Rejected(
                    diagnostics,
                )) => Ok(json!({
                    "reproduced": false,
                    "failure": "reconstructed request did not compile",
                    "diagnostics": diagnostics,
                })),
                err => Err((-32000, err.to_string())),
            }
        }
    }
}

fn op_replay(params: &Value) -> Result<Value, RpcError> {
    let trace: clean_compiler::replay::RequestTrace =
        serde_json::from_value(params.get("trace").cloned().unwrap_or(Value::Null))
            .map_err(|e| invalid_params(format!("`trace`: {e}")))?;
    let component = BASE64
        .decode(
            params
                .get("component_wasm_base64")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_params("missing `component_wasm_base64`"))?,
        )
        .map_err(|e| invalid_params(format!("`component_wasm_base64`: {e}")))?;
    match clean_compiler::replay::replay(&trace, &component) {
        Ok(report) => Ok(json!({ "matched": true, "report": report })),
        Err(
            failure @ (clean_compiler::replay::ReplayFailure::Diverged { .. }
            | clean_compiler::replay::ReplayFailure::ResponseMismatch { .. }
            | clean_compiler::replay::ReplayFailure::Trap(_)),
        ) => Ok(json!({ "matched": false, "failure": failure.to_string() })),
        Err(err) => Err((-32000, err.to_string())),
    }
}

fn op_bridge_stub(params: &Value) -> Result<Value, RpcError> {
    let wit = params
        .get("wit")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("missing `wit`"))?;
    let interface = params
        .get("interface")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("missing `interface`"))?;
    let fixture: clean_compiler::stub::StubFixture =
        serde_json::from_value(params.get("fixture").cloned().unwrap_or(Value::Null))
            .map_err(|e| invalid_params(format!("`fixture`: {e}")))?;
    match clean_compiler::stub::generate_stub(wit, interface, &fixture) {
        Ok(wasm) => Ok(json!({
            "name": clean_compiler::stub::stub_artifact_name(interface),
            "stub_wasm_base64": BASE64.encode(&wasm),
        })),
        Err(err) => Err((-32000, err.to_string())),
    }
}
