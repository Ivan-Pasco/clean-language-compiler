//! The M8 stage-2 contract (Platform 14 §14.14.3): the compiler's half of
//! watch-mode is the stateless rebuild. A watch loop is `compile()`
//! re-invoked with the updated request document — there is no watch API, no
//! incremental state, no compilation mode: "a watch-mode rebuild produces
//! the same `component.wasm` bytes as a full `debug` build".
//!
//! Pinned here across the library/process boundary: the warm in-process
//! rebuild (what a watching caller holds) is byte-identical to a cold build
//! of the same request through the process adapter, and cycling the loop
//! back to an earlier request reproduces its earlier bytes exactly — the
//! loop leaves no state behind (CMP-02).

use std::process::Command;

use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// The request document a watching caller would lower after seeing `content`
/// in the watched source file. Everything else is held fixed, as a watcher
/// holds its resolved configuration between rebuilds.
fn request_with(content: &str) -> serde_json::Value {
    let host_wit = include_str!("../../../tests/fixtures/wit/host.wit");
    serde_json::json!({
        "spec_version": "1",
        "project": { "name": "watched", "version": "0.0.1" },
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

fn compile_in_process(document: &serde_json::Value) -> clean_compiler::CompileArtifact {
    let request = serde_json::from_value(document.clone()).expect("request deserializes");
    clean_compiler::compile(request).expect("fixture request compiles")
}

/// A cold full build of the same request, through the process adapter.
fn compile_cold(document: &serde_json::Value, tag: &str) -> (Vec<u8>, String) {
    let out =
        std::env::temp_dir().join(format!("clean-compiler-watch-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
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
        .write_all(document.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().expect("binary runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let wasm = std::fs::read(out.join("component.wasm")).expect("component written");
    let manifest =
        std::fs::read_to_string(out.join("build-manifest.json")).expect("manifest written");
    (wasm, manifest)
}

const FIRST: &str = "functions:\n\tvoid init()\n\t\treturn\n";
const EDITED: &str =
    "functions:\n\tvoid init()\n\t\tinteger x = 2\n\t\tinteger y = x + 3\n\t\treturn\n";

/// The watch loop's rebuild is a full debug build: after an edit, the warm
/// in-process rebuild and a cold process-adapter build of the identical
/// request produce byte-identical components and manifests.
#[test]
fn warm_rebuild_equals_cold_full_build() {
    let before = request_with(FIRST);
    let after = request_with(EDITED);

    // The watching caller's sequence: build, edit arrives, rebuild.
    let _first = compile_in_process(&before);
    let rebuilt = compile_in_process(&after);

    let (cold_wasm, cold_manifest) = compile_cold(&after, "cold");
    assert_eq!(
        rebuilt.wasm, cold_wasm,
        "watch rebuild diverges from a full debug build (§14.14.3)"
    );
    let rebuilt_manifest =
        serde_json::to_string_pretty(&rebuilt.manifest).expect("manifest serializes");
    assert_eq!(
        rebuilt_manifest, cold_manifest,
        "watch rebuild manifest diverges from a full debug build"
    );
}

/// The loop leaves no state behind: cycling back to an earlier request
/// reproduces its earlier bytes exactly, and an unchanged request rebuilds
/// to unchanged bytes (CMP-02 inside one process).
#[test]
fn the_loop_is_stateless() {
    let a = request_with(FIRST);
    let b = request_with(EDITED);

    let first_a = compile_in_process(&a);
    let first_b = compile_in_process(&b);
    let again_b = compile_in_process(&b);
    let again_a = compile_in_process(&a);

    assert_eq!(
        first_b.wasm, again_b.wasm,
        "an unchanged request must rebuild to unchanged bytes"
    );
    assert_eq!(
        first_a.wasm, again_a.wasm,
        "compiling an edited request must leave no state that changes an earlier one"
    );
    assert_ne!(
        first_a.wasm, first_b.wasm,
        "the edit is real: the two requests produce different components"
    );
}
