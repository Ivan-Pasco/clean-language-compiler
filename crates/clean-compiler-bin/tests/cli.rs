//! Milestone 1 step 2, process-adapter half: a rejected request exits 1 and
//! leaves `diagnostics.json` (NDJSON) and nothing else in the output
//! directory (CMP-05); a valid request in the pre-v1 pipeline exits 3.

use std::process::Command;

fn run(request_json: &str, out: &std::path::Path) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
        .args(["--out", out.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("binary spawns");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(request_json.as_bytes())
        .unwrap();
    child.wait_with_output().expect("binary runs")
}

#[test]
fn invalid_request_exits_1_with_ndjson_diagnostics_and_no_component() {
    let out = tempdir("rejected");
    let output = run(r#"{"unexpected": true}"#, &out);
    assert_eq!(output.status.code(), Some(1));

    let diagnostics = std::fs::read_to_string(out.join("diagnostics.json")).unwrap();
    let lines: Vec<&str> = diagnostics.lines().collect();
    assert!(!lines.is_empty());
    for line in &lines {
        let value: serde_json::Value =
            serde_json::from_str(line).expect("each line is one JSON object");
        assert_eq!(value["code"], "RQD002");
        assert!(value["rendered"].as_str().is_some());
    }
    assert!(
        !out.join("component.wasm").exists(),
        "no partial component on failure (CMP-05)"
    );
}

#[test]
fn valid_request_writes_the_artifact_set_and_exits_0() {
    let out = tempdir("success");
    let request = valid_request_json();
    let output = run(&request, &out);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wasm = std::fs::read(out.join("component.wasm")).expect("component written");
    assert_eq!(
        &wasm[..8],
        &[0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00]
    );
    assert!(out.join("build-manifest.json").exists());
    assert!(out.join("diagnostics.json").exists());
}

fn valid_request_json() -> String {
    let host_wit = include_str!("../../../tests/fixtures/wit/host.wit");
    let content = "functions:\n\tvoid init()\n\t\treturn\n";
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
    .to_string()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn tempdir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("clean-compiler-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_emit(request_json: &str, out: &std::path::Path) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
        .args(["--out", out.to_str().unwrap(), "--emit", "hir-json"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("binary spawns");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(request_json.as_bytes())
        .unwrap();
    child.wait_with_output().expect("binary runs")
}

#[test]
fn emit_hir_json_writes_hir_and_diagnostics() {
    let out = tempdir("emit-hir");
    let request = valid_request_json();
    let output = run_emit(&request, &out);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let hir = std::fs::read_to_string(out.join("hir.json")).expect("hir.json written");
    let value: serde_json::Value = serde_json::from_str(&hir).expect("hir.json is JSON");
    assert!(value["functions"].is_array(), "HIR carries functions");
    assert!(out.join("diagnostics.json").exists());
    assert!(
        !out.join("component.wasm").exists(),
        "emit never writes a component"
    );
}

/// A request whose only source calls an undefined variable, so `check`
/// rejects it with a diagnostic at a known location.
fn failing_request_json() -> String {
    let host_wit = include_str!("../../../tests/fixtures/wit/host.wit");
    let content = "functions:\n\tvoid init()\n\t\tprintln(missing)\n";
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
    .to_string()
}

/// The adapter half of the §14.14.1 contract: `--why` over the
/// `diagnostics.json` a build wrote prints exactly the report the library
/// produces — the adapter adds transport, never content.
#[test]
fn why_reprojects_the_persisted_diagnostics_verbatim() {
    let out = tempdir("why");
    let request = failing_request_json();
    let output = run_check(&request, &out);
    assert_eq!(
        output.status.code(),
        Some(1),
        "the fixture request fails check"
    );

    let ndjson = std::fs::read_to_string(out.join("diagnostics.json")).unwrap();
    let diagnostics: Vec<clean_compiler::types::Diagnostic> = ndjson
        .lines()
        .map(|l| serde_json::from_str(l).expect("diagnostic line parses"))
        .collect();
    assert!(!diagnostics.is_empty());
    let span = &diagnostics[0].primary_span;
    let location = format!("{}:{}:{}", span.file, span.start.line, span.start.column);

    let why_out = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
        .args([
            "--why",
            &location,
            "--diagnostics",
            out.join("diagnostics.json").to_str().unwrap(),
        ])
        .output()
        .expect("binary runs");
    assert_eq!(
        why_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&why_out.stderr)
    );

    let expected = clean_compiler::why(
        &diagnostics,
        &clean_compiler::WhyQuery {
            file: span.file.clone(),
            line: span.start.line,
            column: Some(span.start.column),
        },
    );
    let printed: serde_json::Value =
        serde_json::from_slice(&why_out.stdout).expect("stdout is the report JSON");
    let expected_value = serde_json::to_value(&expected).expect("report serializes");
    assert_eq!(
        printed, expected_value,
        "adapter output diverges from the library"
    );
    assert!(
        !expected.entries.is_empty(),
        "the report carries the diagnostic at the queried location"
    );
}

/// No diagnostic at the queried location is an answer, not a failure.
#[test]
fn why_with_no_match_exits_0_with_empty_entries() {
    let out = tempdir("why-empty");
    let request = failing_request_json();
    run_check(&request, &out);

    let why_out = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
        .args([
            "--why",
            "app/main.cln:999",
            "--diagnostics",
            out.join("diagnostics.json").to_str().unwrap(),
        ])
        .output()
        .expect("binary runs");
    assert_eq!(why_out.status.code(), Some(0));
    let printed: serde_json::Value = serde_json::from_slice(&why_out.stdout).unwrap();
    assert_eq!(printed["entries"], serde_json::json!([]));
}

fn run_check(request_json: &str, out: &std::path::Path) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
        .args(["--out", out.to_str().unwrap(), "--check"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("binary spawns");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(request_json.as_bytes())
        .unwrap();
    child.wait_with_output().expect("binary runs")
}
