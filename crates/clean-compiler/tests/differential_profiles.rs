//! M9 differential: the same request under `debug`, `release`, and `size`
//! optimization profiles must behave identically (§14.4.2[8]: optimization
//! is required to be semantics-preserving).
//!
//! Today the pin is the strongest form: pass [8] declares "no optimizations
//! run — `debug` and `release` emit the same code until the conformance
//! suite exists to prove semantics preservation" (`src/mir/mod.rs`), so the
//! emitted component must be **byte-identical** across profiles, and the
//! manifests may differ only in `request_sha256` and
//! `resolved_config.build.optimization`. When pass [8] grows real
//! optimizations, relaxing this test from byte-identity to behavioral
//! equivalence is a deliberate design change, made here in the same commit.
//!
//! The nightly job re-runs this over a wider generated set via
//! `DIFFERENTIAL_FUZZ_SEED` / `DIFFERENTIAL_FUZZ_COUNT`.

mod common;
#[path = "grammar_fuzz/ebnf.rs"]
mod ebnf;
#[path = "grammar_fuzz/generate.rs"]
mod generate;

use clean_compiler::{compile, CompileError};
use clean_compiler_types::request::CompileRequest;
use std::path::Path;

fn request_with(source: &str, optimization: &str) -> CompileRequest {
    let mut request = common::minimal_valid_request();
    request.build.optimization = optimization.to_string();
    request.sources[0].content = source.to_string();
    request.sources[0].sha256 = common::sha256_hex(source.as_bytes());
    request
}

/// Normalizes a manifest for cross-profile comparison: the only fields
/// allowed to differ between profiles today.
fn normalized_manifest(
    manifest: &clean_compiler_types::manifest::BuildManifest,
) -> serde_json::Value {
    let mut value = serde_json::to_value(manifest).expect("manifest serializes");
    value["request_sha256"] = serde_json::Value::String(String::new());
    value["resolved_config"]["build"]["optimization"] = serde_json::Value::String(String::new());
    value
}

fn assert_profiles_agree(source: &str, label: &str) {
    let outcomes: Vec<_> = ["debug", "release", "size"]
        .iter()
        .map(|profile| (profile, compile(request_with(source, profile))))
        .collect();

    let (_, baseline) = &outcomes[0];
    for (profile, outcome) in &outcomes[1..] {
        match (baseline, outcome) {
            (Ok(a), Ok(b)) => {
                assert_eq!(
                    a.wasm, b.wasm,
                    "{label}: component bytes diverge between debug and {profile}"
                );
                assert_eq!(
                    normalized_manifest(&a.manifest),
                    normalized_manifest(&b.manifest),
                    "{label}: manifests diverge beyond the allowed fields \
                     between debug and {profile}"
                );
                assert_eq!(
                    serde_json::to_string(&a.diagnostics).expect("diagnostics serialize"),
                    serde_json::to_string(&b.diagnostics).expect("diagnostics serialize"),
                    "{label}: warning/info diagnostics diverge between debug and {profile}"
                );
            }
            (Err(CompileError::Rejected(a)), Err(CompileError::Rejected(b))) => {
                assert_eq!(
                    serde_json::to_string(a).expect("diagnostics serialize"),
                    serde_json::to_string(b).expect("diagnostics serialize"),
                    "{label}: rejection diagnostics diverge between debug and {profile}"
                );
            }
            (Err(CompileError::Unsupported(a)), Err(CompileError::Unsupported(b))) => {
                assert_eq!(
                    a.len(),
                    b.len(),
                    "{label}: unsupported-construct sets diverge between debug and {profile}"
                );
            }
            (a, b) => panic!(
                "{label}: outcome kind diverges between debug ({}) and {profile} ({})",
                kind(a),
                kind(b)
            ),
        }
    }
}

fn kind(outcome: &Result<clean_compiler::CompileArtifact, CompileError>) -> &'static str {
    match outcome {
        Ok(_) => "ok",
        Err(CompileError::Rejected(_)) => "rejected",
        Err(CompileError::Unsupported(_)) => "unsupported",
        Err(CompileError::Incomplete { .. }) => "incomplete",
    }
}

/// Real programs: every `.cln` under `tests/corpus/core/`, the corpus most
/// likely to reach codegen.
#[test]
fn corpus_core_agrees_across_profiles() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/core");
    let mut files: Vec<_> = walk(&root);
    files.sort();
    assert!(!files.is_empty(), "corpus/core is populated");
    for path in &files {
        let source = std::fs::read_to_string(path).expect("corpus file reads");
        assert_profiles_agree(&source, &path.display().to_string());
    }
    println!(
        "differential profiles: {} corpus/core programs agree",
        files.len()
    );
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).expect("corpus dir reads") {
        let path = entry.expect("readable dir entry").path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else if path.extension().is_some_and(|e| e == "cln") {
            files.push(path);
        }
    }
    files
}

/// Generated programs: the grammar fuzzer's output through all three
/// profiles.
#[test]
fn generated_programs_agree_across_profiles() {
    let base: u64 = std::env::var("DIFFERENTIAL_FUZZ_SEED")
        .map(|v| v.parse().expect("DIFFERENTIAL_FUZZ_SEED is a u64"))
        .unwrap_or(0);
    let count: u64 = std::env::var("DIFFERENTIAL_FUZZ_COUNT")
        .map(|v| v.parse().expect("DIFFERENTIAL_FUZZ_COUNT is a u64"))
        .unwrap_or(64);
    let generator = generate::Generator::new(ebnf::Grammar::load(&vendored_files()));
    for seed in base..base + count {
        let source = generator.program(seed, 400);
        assert_profiles_agree(&source, &format!("fuzz seed {seed}"));
    }
}

fn vendored_files() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/grammar");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("vendored grammar directory exists")
        .map(|e| {
            e.expect("readable dir entry")
                .file_name()
                .into_string()
                .expect("utf-8")
        })
        .filter(|n| n.ends_with(".ebnf.md"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let content =
                std::fs::read_to_string(dir.join(&name)).expect("vendored grammar file reads");
            (name, content)
        })
        .collect()
}
