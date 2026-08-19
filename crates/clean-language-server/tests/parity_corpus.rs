//! The M7 milestone gate, at corpus scale: for every DIA-06 fixture — the
//! byte-exact triple set covering each compiler-emittable code — the LSP
//! surface publishes exactly the diagnostics `check` produces. Same list,
//! same codes, same spans, over the same request document.

mod common;

use std::path::PathBuf;

use clean_compiler_types::Diagnostic;
use common::{embedded_diagnostics, request_document, Client};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/cln/diagnostics")
}

fn sorted(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort_by_key(|d| {
        (
            d.primary_span.file.clone(),
            d.primary_span.start,
            d.primary_span.end,
            d.code.clone(),
            d.message.clone(),
        )
    });
    diagnostics
}

/// Reference diagnostics plus the publish count the server will emit for
/// this document (one request bucket, plus one bucket per source — or the
/// request bucket alone when intake rejects the document).
fn reference(document: &serde_json::Value) -> (Vec<Diagnostic>, usize) {
    let json = document.to_string();
    let mut intake = clean_compiler::diag::DiagnosticSink::new();
    match clean_compiler::request::from_json(&json, &mut intake) {
        Some(request) if !intake.has_errors() => {
            let buckets = 1 + request.sources.len();
            let diagnostics = clean_compiler::check(request)
                .unwrap_or_else(|e| panic!("check must produce diagnostics, got: {e}"));
            (diagnostics, buckets)
        }
        _ => (
            clean_compiler::diag::finalize(
                intake.into_diagnostics(),
                &clean_compiler::diag::SourceCache::empty(),
            ),
            1,
        ),
    }
}

#[test]
fn every_dia06_fixture_holds_parity_over_lsp() {
    let dir = fixtures_dir();
    let mut codes: Vec<String> = std::fs::read_dir(&dir)
        .expect("fixtures dir reads")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "cln"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    codes.sort();
    assert!(!codes.is_empty(), "the DIA-06 corpus is present");

    for code in &codes {
        // The same request the DIA-06 harness synthesizes: the committed
        // `.request.json` override when the code is request-level, else the
        // minimal valid request wrapping the fixture source.
        let override_path = dir.join(format!("{code}.request.json"));
        let document: serde_json::Value = if override_path.exists() {
            let json = std::fs::read_to_string(&override_path).expect("request override reads");
            serde_json::from_str(&json).expect("request override is JSON")
        } else {
            let source_path = dir.join(format!("{code}.cln"));
            let content = std::fs::read_to_string(&source_path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", source_path.display()));
            request_document(&[(
                &format!("tests/cln/diagnostics/{code}.cln"),
                content.as_str(),
            )])
        };

        let (expected, buckets) = reference(&document);
        let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
        let publishes = client.collect_publishes(buckets);
        let published = embedded_diagnostics(&publishes);
        assert_eq!(
            sorted(published),
            sorted(expected),
            "{code}: LSP diagnostics diverge from `check`"
        );
        client.stop();
    }
}
