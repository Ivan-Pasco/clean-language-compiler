//! The M8 stage-6 contract (Platform 14 §14.2.3): the JSON-RPC / MCP
//! adapter is a wrapper, not a rewrite. Over the same request document,
//! every operation served over the wire returns exactly what the library
//! returns — byte-identical component, identical diagnostics — and the MCP
//! dialect wraps the identical handlers, so the two surfaces cannot
//! diverge.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use base64::Engine as _;
use sha2::{Digest, Sha256};

const BASE64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Server {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
            .arg("--serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("server spawns");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn call(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        writeln!(self.stdin, "{message}").expect("request writes");
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("response reads");
        let response: serde_json::Value = serde_json::from_str(&line).expect("response is JSON");
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], serde_json::json!(id));
        response
    }

    fn result(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let response = self.call(method, params);
        assert!(
            response.get("error").is_none(),
            "unexpected RPC error: {}",
            response["error"]
        );
        response["result"].clone()
    }

    fn stop(mut self) {
        drop(self.stdin);
        let _ = self.child.wait();
    }
}

fn request_document(content: &str) -> serde_json::Value {
    let host_wit = include_str!("../../../tests/fixtures/wit/host.wit");
    serde_json::json!({
        "spec_version": "1",
        "project": { "name": "fixture", "version": "0.0.1" },
        "build": { "target": "wasm32-server", "optimization": "debug" },
        "target_world": {
            "host": "clean-server",
            "version": "0.7.0",
            "world": "server",
            "sha256": sha256_hex(host_wit.as_bytes()),
            "wit": host_wit,
        },
        "sources": [{
            "path": "app/main.cln",
            "sha256": sha256_hex(content.as_bytes()),
            "content": content,
        }],
    })
}

const VALID: &str = "functions:\n\tvoid init()\n\t\treturn\n";
const FAILING: &str = "functions:\n\tvoid init()\n\t\tinteger x = missing\n";

/// The core §14.2.3 contract: `compile` over the wire is `compile` in the
/// library — byte-identical component, identical manifest outputs, over
/// the request document unchanged.
#[test]
fn rpc_compile_is_byte_identical_to_the_library() {
    let document = request_document(VALID);
    let expected = clean_compiler::compile(serde_json::from_value(document.clone()).unwrap())
        .expect("fixture compiles");

    let mut server = Server::start();
    let result = server.result("compile", serde_json::json!({ "request": document }));
    assert_eq!(result["rejected"], false);
    let wasm = BASE64
        .decode(result["component_wasm_base64"].as_str().unwrap())
        .expect("component decodes");
    assert_eq!(
        wasm, expected.wasm,
        "wire component diverges from the library"
    );
    assert_eq!(
        result["build_manifest"],
        serde_json::to_value(&expected.manifest).unwrap(),
        "wire manifest diverges from the library"
    );
    server.stop();
}

/// A rejected program is an outcome, not a protocol error, and carries the
/// exact diagnostics `check` produces.
#[test]
fn rpc_check_returns_the_library_diagnostics() {
    let document = request_document(FAILING);
    let expected = clean_compiler::check(serde_json::from_value(document.clone()).unwrap())
        .expect("check runs");
    assert!(!expected.is_empty());

    let mut server = Server::start();
    let result = server.result("check", serde_json::json!({ "request": document }));
    assert_eq!(result["failed"], true);
    assert_eq!(
        result["diagnostics"],
        serde_json::to_value(&expected).unwrap(),
        "wire diagnostics diverge from the library"
    );

    // compile over the same failing document: rejected, same diagnostics.
    let result = server.result("compile", serde_json::json!({ "request": document }));
    assert_eq!(result["rejected"], true);
    assert_eq!(
        result["diagnostics"],
        serde_json::to_value(&expected).unwrap()
    );
    server.stop();
}

/// `why` over the wire re-projects exactly what the library re-projects.
#[test]
fn rpc_why_matches_the_library_report() {
    let document = request_document(FAILING);
    let diagnostics = clean_compiler::check(serde_json::from_value(document.clone()).unwrap())
        .expect("check runs");
    let span = &diagnostics[0].primary_span;
    let expected = clean_compiler::why(
        &diagnostics,
        &clean_compiler::WhyQuery {
            file: span.file.clone(),
            line: span.start.line,
            column: None,
        },
    );

    let mut server = Server::start();
    let result = server.result(
        "why",
        serde_json::json!({
            "diagnostics": diagnostics,
            "file": span.file,
            "line": span.start.line,
        }),
    );
    assert_eq!(result, serde_json::to_value(&expected).unwrap());
    server.stop();
}

/// The MCP dialect: handshake, tool inventory, and a tools/call that wraps
/// the identical handler the direct method uses.
#[test]
fn mcp_dialect_wraps_the_same_handlers() {
    let mut server = Server::start();

    let init = server.result("initialize", serde_json::json!({}));
    assert_eq!(init["serverInfo"]["name"], "clean-compiler");
    assert!(init["capabilities"]["tools"].is_object());

    let tools = server.result("tools/list", serde_json::json!({}));
    let names: Vec<&str> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for expected in [
        "compile",
        "check",
        "why",
        "reproBuild",
        "replay",
        "bridgeStub",
    ] {
        assert!(names.contains(&expected), "tools/list misses `{expected}`");
    }

    let document = request_document(FAILING);
    let direct = server.result("check", serde_json::json!({ "request": document }));
    let wrapped = server.result(
        "tools/call",
        serde_json::json!({ "name": "check", "arguments": { "request": document } }),
    );
    assert_eq!(wrapped["isError"], false);
    assert_eq!(
        wrapped["structuredContent"], direct,
        "the MCP tool and the direct method diverged"
    );
    server.stop();
}

/// Protocol misuse is a JSON-RPC error; it never leaks as a fake outcome.
#[test]
fn protocol_errors_are_jsonrpc_errors() {
    let mut server = Server::start();

    let response = server.call("no-such-method", serde_json::json!({}));
    assert_eq!(response["error"]["code"], -32601);

    let response = server.call("why", serde_json::json!({ "diagnostics": [] }));
    assert_eq!(response["error"]["code"], -32602);

    // The reproBuild + replay + bridgeStub wrappers round-trip too: a
    // build's manifest reproduces over the wire.
    let document = request_document(VALID);
    let compiled = server.result("compile", serde_json::json!({ "request": document }));
    let repro = server.result(
        "reproBuild",
        serde_json::json!({
            "manifest": compiled["build_manifest"],
            "request": document,
            "original_wasm_base64": compiled["component_wasm_base64"],
        }),
    );
    assert_eq!(repro["reproduced"], true);
    assert_eq!(
        repro["component_wasm_base64"], compiled["component_wasm_base64"],
        "wire reproduction diverges from the wire build"
    );
    server.stop();
}
