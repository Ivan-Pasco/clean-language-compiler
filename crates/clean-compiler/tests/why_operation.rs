//! The M8 stage-1 contract (Platform 14 §14.14.1): `why` is a
//! re-presentation of `diagnostics.json`, never a re-compilation. Over the
//! same request documents the DIA-06 harness synthesizes, every diagnostic
//! the build reports must re-project at its own location losing nothing and
//! inventing nothing — and the report must come out identical whether the
//! diagnostics arrive fresh from `check` or parsed back from the NDJSON the
//! build persisted.

mod common;

use std::path::{Path, PathBuf};

use clean_compiler::diag::{DiagnosticSink, SourceCache};
use clean_compiler::why::{diagnostics_from_ndjson, passes_for, why, WhyQuery};
use clean_compiler_types::{codes, Diagnostic};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/cln/diagnostics")
}

fn fixture_codes(dir: &Path) -> Vec<String> {
    let mut codes: Vec<String> = std::fs::read_dir(dir)
        .expect("fixtures dir reads")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "cln"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    codes.sort();
    codes
}

/// The same diagnostics the DIA-06 harness produces for `code`: the
/// committed `.request.json` override when one exists, else the minimal
/// valid request wrapping the fixture source.
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

/// Re-projection loses nothing and invents nothing: for every diagnostic in
/// every corpus fixture, querying its own primary location returns an entry
/// carrying that diagnostic's exact facts, and no entry in any report lacks
/// a source diagnostic.
#[test]
fn every_corpus_diagnostic_reprojects_at_its_own_location() {
    let dir = fixtures_dir();
    let codes = fixture_codes(&dir);
    assert!(!codes.is_empty(), "the DIA-06 corpus is present");

    for code in &codes {
        let diagnostics = diagnostics_for(code, &dir);
        assert!(
            !diagnostics.is_empty(),
            "{code}: fixture yields diagnostics"
        );
        for diagnostic in &diagnostics {
            let query = WhyQuery {
                file: diagnostic.primary_span.file.clone(),
                line: diagnostic.primary_span.start.line,
                column: Some(diagnostic.primary_span.start.column),
            };
            let report = why(&diagnostics, &query);
            let entry = report
                .entries
                .iter()
                .find(|e| {
                    e.code == diagnostic.code
                        && e.message == diagnostic.message
                        && e.primary_span == diagnostic.primary_span
                })
                .unwrap_or_else(|| {
                    panic!(
                        "{code}: {} at {}:{}:{} did not re-project at its own location",
                        diagnostic.code,
                        query.file,
                        query.line,
                        query.column.unwrap(),
                    )
                });
            // Verbatim carry-over: the re-projection is the diagnostic's own
            // data, re-presented (§14.14.1 — the data is already there).
            assert_eq!(entry.level, diagnostic.level, "{code}: level");
            assert_eq!(
                entry.primary_label, diagnostic.primary_label,
                "{code}: label"
            );
            assert_eq!(entry.secondary, diagnostic.secondary, "{code}: secondary");
            assert_eq!(entry.notes, diagnostic.notes, "{code}: notes");
            assert_eq!(entry.helps, diagnostic.helps, "{code}: helps");
            assert_eq!(
                entry.suggestions, diagnostic.suggestions,
                "{code}: suggestions"
            );
            assert_eq!(entry.doc_url, diagnostic.doc_url, "{code}: doc_url");
            // Nothing invented: every reported entry maps back to an input
            // diagnostic at this location.
            for entry in &report.entries {
                assert!(
                    diagnostics.iter().any(|d| d.code == entry.code
                        && d.message == entry.message
                        && d.primary_span == entry.primary_span),
                    "{code}: report contains an entry with no source diagnostic"
                );
            }
        }
    }
}

/// Re-presentation, not re-compilation: the report built from the persisted
/// NDJSON (what `diagnostics.json` holds after a build) is identical to the
/// report built from the fresh `check` output. `why` needs no request
/// document, no sources, no pipeline.
#[test]
fn report_from_persisted_ndjson_equals_report_from_fresh_check() {
    let dir = fixtures_dir();
    for code in fixture_codes(&dir) {
        let fresh = diagnostics_for(&code, &dir);
        let ndjson: String = fresh
            .iter()
            .map(|d| serde_json::to_string(d).expect("diagnostic serializes") + "\n")
            .collect();
        let persisted = diagnostics_from_ndjson(&ndjson)
            .unwrap_or_else(|e| panic!("{code}: round-trip failed: {e}"));

        for diagnostic in &fresh {
            let query = WhyQuery {
                file: diagnostic.primary_span.file.clone(),
                line: diagnostic.primary_span.start.line,
                column: None,
            };
            assert_eq!(
                why(&persisted, &query),
                why(&fresh, &query),
                "{code}: persisted and fresh diagnostics re-project differently"
            );
        }
    }
}

/// The registry-derived pass map is total: every compiler-emittable code
/// resolves to at least one §14.4 pass, except the codes with no pipeline
/// home, which are named here exhaustively.
#[test]
fn pass_map_covers_every_emittable_code() {
    const NO_PIPELINE_HOME: &[&str] = &["COM013"];
    for info in codes::REGISTRY {
        if !info.is_compiler_emittable() {
            continue;
        }
        let passes = passes_for(info.code);
        if NO_PIPELINE_HOME.contains(&info.code) {
            assert!(
                passes.is_empty(),
                "{}: listed as having no pipeline home but the map assigns one",
                info.code
            );
        } else {
            assert!(
                !passes.is_empty(),
                "{}: compiler-emittable but the why pass map does not cover it",
                info.code
            );
        }
    }
}

/// Deterministic over its inputs (CMP-02 for operations): the same
/// diagnostics and query serialize to the byte-identical report.
#[test]
fn report_serialization_is_deterministic() {
    let dir = fixtures_dir();
    let code = "SEM002";
    let diagnostics = diagnostics_for(code, &dir);
    let query = WhyQuery {
        file: diagnostics[0].primary_span.file.clone(),
        line: diagnostics[0].primary_span.start.line,
        column: None,
    };
    let first = serde_json::to_string(&why(&diagnostics, &query)).unwrap();
    let second = serde_json::to_string(&why(&diagnostics, &query)).unwrap();
    assert_eq!(first, second);
    assert!(first.contains("\"entries\""));
}
