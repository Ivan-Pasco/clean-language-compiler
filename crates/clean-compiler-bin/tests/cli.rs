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
fn valid_request_reports_incomplete_pipeline_with_exit_3() {
    let out = tempdir("incomplete");
    let request = valid_request_json();
    let output = run(&request, &out);
    assert_eq!(output.status.code(), Some(3));
    assert!(!out.join("component.wasm").exists());
}

fn valid_request_json() -> String {
    let host_wit = include_str!("../../../tests/fixtures/wit/host.wit");
    let content = "functions:\n\tinit()\n";
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
