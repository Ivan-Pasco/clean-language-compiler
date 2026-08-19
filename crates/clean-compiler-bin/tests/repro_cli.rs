//! Adapter half of the §14.14.6 build-reproduction contract: over the same
//! request document, `--repro-build` writes the byte-identical shipped
//! component on match, and on mismatch writes a COM013 `diagnostics.json`
//! with exit 1 and no component (CMP-05).

use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn request_json() -> String {
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

fn tempdir(tag: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("clean-compiler-repro-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(stdin: &str, args: &[&str]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_clean-compiler"))
        .args(args)
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
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().expect("binary runs")
}

fn build(out: &Path) -> (Vec<u8>, String) {
    let output = run(&request_json(), &["--out", out.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wasm = std::fs::read(out.join("component.wasm")).unwrap();
    let manifest = out
        .join("build-manifest.json")
        .to_str()
        .unwrap()
        .to_string();
    (wasm, manifest)
}

#[test]
fn repro_build_writes_the_byte_identical_component() {
    let build_out = tempdir("build");
    let (original_wasm, manifest_path) = build(&build_out);

    let repro_out = tempdir("repro");
    let output = run(
        &request_json(),
        &[
            "--out",
            repro_out.to_str().unwrap(),
            "--repro-build",
            &manifest_path,
            "--original",
            build_out.join("component.wasm").to_str().unwrap(),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reproduced = std::fs::read(repro_out.join("component.wasm")).unwrap();
    assert_eq!(reproduced, original_wasm, "reproduction is byte-identical");
}

#[test]
fn corrupted_manifest_yields_com013_and_no_component() {
    let build_out = tempdir("build-corrupt");
    let (_wasm, manifest_path) = build(&build_out);

    // Corrupt the recorded output hash.
    let text = std::fs::read_to_string(&manifest_path).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
    manifest["outputs"]["wasm_sha256"] = serde_json::Value::String("0".repeat(64));
    let corrupt_path = build_out.join("corrupt-manifest.json");
    std::fs::write(&corrupt_path, manifest.to_string()).unwrap();

    let repro_out = tempdir("repro-corrupt");
    let output = run(
        &request_json(),
        &[
            "--out",
            repro_out.to_str().unwrap(),
            "--repro-build",
            corrupt_path.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "divergence is a rejection");
    assert!(
        !repro_out.join("component.wasm").exists(),
        "no component on divergence (CMP-05)"
    );
    let diagnostics = std::fs::read_to_string(repro_out.join("diagnostics.json")).unwrap();
    let line: serde_json::Value =
        serde_json::from_str(diagnostics.lines().next().unwrap()).unwrap();
    assert_eq!(
        line["code"], "COM013",
        "divergence presents as a compiler bug"
    );
}
