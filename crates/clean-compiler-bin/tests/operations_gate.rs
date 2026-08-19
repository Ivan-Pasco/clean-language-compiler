//! The M8 milestone gate: every v1 API operation (Platform 14 §14.14 +
//! §14.2.3), over the same request document, through the shipped surfaces.
//! One document flows build → check → why → repro build → replay →
//! bridge stub → JSON-RPC, and every answer agrees with every other:
//! the fast path, the debugging operations, and the wire are framings of
//! one `compile()`, not separate code paths.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use base64::Engine as _;
use sha2::{Digest, Sha256};

const BASE64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn request_document(content: &str) -> serde_json::Value {
    let host_wit = include_str!("../../../tests/fixtures/wit/host.wit");
    serde_json::json!({
        "spec_version": "1",
        "project": { "name": "gate", "version": "0.0.1" },
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

fn run_adapter(stdin: &str, args: &[&str]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary spawns");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().expect("binary runs")
}

fn tempdir(tag: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("clean-compiler-gate-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn every_v1_operation_agrees_over_the_same_request_document() {
    let valid = request_document("functions:\n\tvoid init()\n\t\treturn\n");
    let failing = request_document("functions:\n\tvoid init()\n\t\tinteger x = missing\n");

    // ---- [build] the process adapter produces the artifact set ---------
    let build_out = tempdir("build");
    let output = run_adapter(&valid.to_string(), &["--out", build_out.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "build succeeds");
    let component = std::fs::read(build_out.join("component.wasm")).unwrap();
    let manifest_text = std::fs::read_to_string(build_out.join("build-manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();

    // ---- [check §14.14.4] the fast path fails where the build fails ----
    let check_out = tempdir("check");
    let output = run_adapter(
        &failing.to_string(),
        &["--out", check_out.to_str().unwrap(), "--check"],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "check rejects the bad program"
    );
    let diagnostics_path = check_out.join("diagnostics.json");
    let ndjson = std::fs::read_to_string(&diagnostics_path).unwrap();
    let first: serde_json::Value = serde_json::from_str(ndjson.lines().next().unwrap()).unwrap();
    let file = first["primary_span"]["file"].as_str().unwrap().to_string();
    let line = first["primary_span"]["start"]["line"].as_u64().unwrap();

    // ---- [why §14.14.1] re-projection over the persisted diagnostics ---
    let output = run_adapter(
        "",
        &[
            "--why",
            &format!("{file}:{line}"),
            "--diagnostics",
            diagnostics_path.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "why runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["entries"][0]["code"], first["code"],
        "why re-presents the diagnostic check produced"
    );

    // ---- [repro build §14.14.6] the manifest reproduces the bytes ------
    let repro_out = tempdir("repro");
    let output = run_adapter(
        &valid.to_string(),
        &[
            "--out",
            repro_out.to_str().unwrap(),
            "--repro-build",
            build_out.join("build-manifest.json").to_str().unwrap(),
            "--original",
            build_out.join("component.wasm").to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "reproduction succeeds");
    let reproduced = std::fs::read(repro_out.join("component.wasm")).unwrap();
    assert_eq!(reproduced, component, "reproduction is byte-identical");

    // ---- [replay §14.14.6] the shipped component replays its trace -----
    let trace = serde_json::json!({
        "spec_version": "1",
        "component_sha256": manifest["outputs"]["wasm_sha256"],
        "entry": { "function": "init", "arguments": [] },
        "host_calls": [],
        "response": [],
    });
    let trace_path = build_out.join("trace.json");
    std::fs::write(&trace_path, trace.to_string()).unwrap();
    let output = run_adapter(
        "",
        &[
            "--replay",
            trace_path.to_str().unwrap(),
            "--component",
            build_out.join("component.wasm").to_str().unwrap(),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "replay matches: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // ---- [bridge stub §14.14.5] the test loop's other half -------------
    let wit_path = build_out.join("bridge.wit");
    std::fs::write(
        &wit_path,
        "package clean:bridge@1.0.0;\n\ninterface console {\n    print: func(text: string);\n}\n",
    )
    .unwrap();
    let fixture_path = build_out.join("fixture.json");
    std::fs::write(&fixture_path, r#"{ "responses": { "print": [[]] } }"#).unwrap();
    let stub_out = tempdir("stub");
    let output = run_adapter(
        "",
        &[
            "--out",
            stub_out.to_str().unwrap(),
            "--bridge-stub",
            "console",
            "--wit",
            wit_path.to_str().unwrap(),
            "--fixture",
            fixture_path.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "stub generates");
    assert!(stub_out.join("clean-bridge-console-stub.wasm").exists());

    // ---- [JSON-RPC / MCP §14.2.3] the wire agrees with the adapter -----
    let mut server = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
        .arg("--serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("server spawns");
    let mut stdin = server.stdin.take().unwrap();
    let mut stdout = BufReader::new(server.stdout.take().unwrap());
    let message = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "compile",
        "params": { "request": valid },
    });
    writeln!(stdin, "{message}").unwrap();
    let mut linebuf = String::new();
    stdout.read_line(&mut linebuf).unwrap();
    let response: serde_json::Value = serde_json::from_str(&linebuf).unwrap();
    let wire_wasm = BASE64
        .decode(
            response["result"]["component_wasm_base64"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        wire_wasm, component,
        "the wire, the process adapter, and the manifest all name one build"
    );
    drop(stdin);
    let _ = server.wait();
}
