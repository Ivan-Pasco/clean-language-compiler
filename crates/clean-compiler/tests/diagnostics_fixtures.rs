//! The DIA-06 harness (Platform 13 §12), self-discovering over the code
//! registry: every compiler-emittable code MUST have either a fixture
//! triple under `tests/cln/diagnostics/` —
//!
//!   `<CODE>.cln`         minimal failing source
//!   `<CODE>.stdout.txt`  byte-exact CLI output (§4.2 blocks, §10.3 order)
//!   `<CODE>.json`        byte-exact NDJSON (§6.1, emission order)
//!
//! — or a line in `unimplemented.txt`, the shrink-only ledger of codes no
//! input can trigger yet. A code may never appear in both; a code with a
//! fixture can never return to the ledger (the file's header states the
//! decrease-only rule the review enforces).
//!
//! Request-level codes (`RQD`) replace the synthesized request with a
//! committed `<CODE>.request.json`, exercising the same intake path the
//! binary uses. Regenerate snapshots deliberately with
//! `UPDATE_DIAG_FIXTURES=1 cargo test --test diagnostics_fixtures`.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clean_compiler::diag::{DiagnosticSink, SourceCache};
use clean_compiler_types::codes;
use clean_compiler_types::Diagnostic;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/cln/diagnostics")
}

fn emittable_codes() -> BTreeSet<&'static str> {
    codes::REGISTRY
        .iter()
        .filter(|info| info.is_compiler_emittable())
        .map(|info| info.code)
        .collect()
}

fn read_unimplemented(dir: &Path) -> Vec<String> {
    let path = dir.join("unimplemented.txt");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn fixture_codes(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "cln"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect()
}

/// The partition law: fixtures ∪ unimplemented == compiler-emittable codes,
/// with no overlap and no strays. `unimplemented.txt` is kept sorted and
/// unique so its diffs read as pure shrinkage.
#[test]
fn every_emittable_code_has_a_fixture_or_a_ledger_line() {
    let dir = fixtures_dir();
    let emittable = emittable_codes();
    let fixtures = fixture_codes(&dir);
    let ledger = read_unimplemented(&dir);

    let mut sorted = ledger.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        ledger, sorted,
        "unimplemented.txt must be sorted and duplicate-free"
    );
    let ledger: BTreeSet<String> = ledger.into_iter().collect();

    for code in &fixtures {
        assert!(
            emittable.contains(code.as_str()),
            "{code} has a fixture but is not a compiler-emittable registry code"
        );
        assert!(
            !ledger.contains(code),
            "{code} has a fixture; its unimplemented.txt line must be deleted (the ledger only decreases)"
        );
    }
    for code in &ledger {
        assert!(
            emittable.contains(code.as_str()),
            "unimplemented.txt lists {code}, which is not a compiler-emittable registry code"
        );
    }
    for code in &emittable {
        assert!(
            fixtures.contains(*code) || ledger.contains(*code),
            "{code} is compiler-emittable but has neither a fixture triple nor an unimplemented.txt line (DIA-06)"
        );
    }
}

/// Runs one fixture through the same path the binary's `--check` takes and
/// returns the finalized diagnostics.
fn diagnostics_for(code: &str, dir: &Path) -> Vec<Diagnostic> {
    let request_override = dir.join(format!("{code}.request.json"));
    let request = if request_override.exists() {
        let json = std::fs::read_to_string(&request_override).expect("request override reads");
        let mut intake = DiagnosticSink::new();
        match clean_compiler::request::from_json(&json, &mut intake) {
            Some(request) => request,
            None => {
                return clean_compiler::diag::finalize(
                    intake.into_diagnostics(),
                    &SourceCache::empty(),
                )
            }
        }
    } else {
        let path = dir.join(format!("{code}.cln"));
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let mut request = common::minimal_valid_request();
        request.sources[0] = clean_compiler_types::request::SourceFile {
            path: format!("tests/cln/diagnostics/{code}.cln"),
            sha256: common::sha256_hex(content.as_bytes()),
            content,
        };
        request
    };
    clean_compiler::check(request)
        .unwrap_or_else(|e| panic!("{code}: check must run passes 1-9, got: {e}"))
}

/// CLI text: §4.2 blocks sorted by file, then line, then column (§10.3),
/// separated by one blank line.
fn stdout_text(diagnostics: &[Diagnostic]) -> String {
    let mut sorted: Vec<&Diagnostic> = diagnostics.iter().collect();
    sorted.sort_by(|a, b| {
        (
            &a.primary_span.file,
            a.primary_span.start.line,
            a.primary_span.start.column,
        )
            .cmp(&(
                &b.primary_span.file,
                b.primary_span.start.line,
                b.primary_span.start.column,
            ))
    });
    sorted
        .iter()
        .map(|d| d.rendered.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// NDJSON: one object per line, emission order (§6.1, §10.3).
fn json_text(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        out.push_str(&serde_json::to_string(diagnostic).expect("diagnostic serializes"));
        out.push('\n');
    }
    out
}

#[test]
fn fixture_triples_are_byte_exact() {
    let dir = fixtures_dir();
    let update = std::env::var_os("UPDATE_DIAG_FIXTURES").is_some();
    let mut failures = Vec::new();

    for code in fixture_codes(&dir) {
        let diagnostics = diagnostics_for(&code, &dir);
        if diagnostics.is_empty() {
            failures.push(format!("{code}: fixture produced no diagnostics"));
            continue;
        }
        if !diagnostics.iter().any(|d| d.code == code) {
            failures.push(format!(
                "{code}: fixture produced {:?} but never {code}",
                diagnostics
                    .iter()
                    .map(|d| d.code.as_str())
                    .collect::<Vec<_>>()
            ));
            continue;
        }
        let stdout = stdout_text(&diagnostics);
        let json = json_text(&diagnostics);
        let stdout_path = dir.join(format!("{code}.stdout.txt"));
        let json_path = dir.join(format!("{code}.json"));
        if update {
            std::fs::write(&stdout_path, &stdout).expect("write stdout snapshot");
            std::fs::write(&json_path, &json).expect("write json snapshot");
            continue;
        }
        for (path, actual) in [(stdout_path, stdout), (json_path, json)] {
            match std::fs::read_to_string(&path) {
                Ok(expected) if expected == actual => {}
                Ok(_) => failures.push(format!(
                    "{}: snapshot differs (regenerate deliberately with UPDATE_DIAG_FIXTURES=1)",
                    path.display()
                )),
                Err(_) => failures.push(format!(
                    "{}: missing snapshot (DIA-06 triple incomplete)",
                    path.display()
                )),
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
