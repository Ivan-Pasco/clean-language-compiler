//! The M7 contract: LSP diagnostics ≡ `cln check` diagnostics — same list,
//! same codes, same spans, over the same request document. The reference
//! side runs the library `check` entry point directly; the LSP side drives
//! the real server loop and reads back the compiler `Diagnostic` values the
//! protocol round-trips in `data` (Platform 04 §4.1.1).

mod common;

use clean_compiler_types::Diagnostic;
use common::{embedded_diagnostics, reference_check, request_document, Client, ROOT_URI};

/// Publish payloads arrive bucketed by file; the reference list arrives in
/// emission order. Same-list equality is order-insensitive across buckets,
/// so both sides are compared under one stable key.
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

fn assert_parity(publishes: &[common::ReceivedPublish], document: &serde_json::Value) {
    let lsp_side = sorted(embedded_diagnostics(publishes));
    let reference = sorted(reference_check(document));
    assert_eq!(
        lsp_side, reference,
        "LSP diagnostics and `check` diagnostics diverge"
    );
}

const BROKEN: &str = "functions:\n\tvoid init()\n\t\tinteger n = \"👍!\"\n";
const FIXED: &str = "functions:\n\tvoid init()\n\t\tinteger n = 1\n";

#[test]
fn clean_program_publishes_empty_buckets() {
    let document = request_document(&[("app/main.cln", FIXED)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    // One publish per bucket: the request document, then each source.
    let publishes = client.collect_publishes(2);
    assert_eq!(publishes[0].uri, "clean:request");
    assert_eq!(publishes[1].uri, format!("{ROOT_URI}/app/main.cln"));
    assert!(publishes.iter().all(|p| p.diagnostics.is_empty()));
    assert_parity(&publishes, &document);
    client.stop();
}

#[test]
fn type_error_reaches_the_editor_with_parity() {
    let document = request_document(&[("app/main.cln", BROKEN)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let publishes = client.collect_publishes(2);
    assert_parity(&publishes, &document);

    let file_bucket = &publishes[1];
    assert!(
        !file_bucket.diagnostics.is_empty(),
        "the broken fixture yields at least one diagnostic"
    );
    let embedded = embedded_diagnostics(std::slice::from_ref(file_bucket));
    for (lsp, compiler) in file_bucket.diagnostics.iter().zip(&embedded) {
        // Platform 13 §7 field mapping, spot-checked on the wire shape.
        assert_eq!(
            lsp.code,
            Some(lsp_types::NumberOrString::String(compiler.code.clone()))
        );
        assert_eq!(
            lsp.severity,
            Some(lsp_types::DiagnosticSeverity::ERROR),
            "the fixture's diagnostics are errors"
        );
        assert_eq!(
            lsp.code_description.as_ref().map(|c| c.href.as_str()),
            Some(compiler.doc_url.as_str())
        );
        assert!(lsp.message.starts_with(&compiler.message));
    }
    client.stop();
}

#[test]
fn didchange_updates_and_clears_diagnostics() {
    let document = request_document(&[("app/main.cln", BROKEN)]);
    let uri = format!("{ROOT_URI}/app/main.cln");
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let publishes = client.collect_publishes(2);
    assert_parity(&publishes, &document);
    assert!(!publishes[1].diagnostics.is_empty());

    client.notify(
        "textDocument/didOpen",
        serde_json::json!({
            "textDocument": { "uri": uri, "languageId": "clean", "version": 1, "text": BROKEN }
        }),
    );
    let publishes = client.collect_publishes(2);
    assert!(!publishes[1].diagnostics.is_empty());

    // The fix: every bucket republishes, so the stale error clears.
    client.notify(
        "textDocument/didChange",
        serde_json::json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{ "text": FIXED }]
        }),
    );
    let publishes = client.collect_publishes(2);
    assert!(publishes.iter().all(|p| p.diagnostics.is_empty()));
    let edited = request_document(&[("app/main.cln", FIXED)]);
    assert_parity(&publishes, &edited);

    // Break it again: parity holds against the edited document, proving the
    // overlay (content + recomputed sha256) is what the pipeline sees.
    client.notify(
        "textDocument/didChange",
        serde_json::json!({
            "textDocument": { "uri": uri, "version": 3 },
            "contentChanges": [{ "text": BROKEN }]
        }),
    );
    let publishes = client.collect_publishes(2);
    let edited = request_document(&[("app/main.cln", BROKEN)]);
    assert_parity(&publishes, &edited);
    assert!(!publishes[1].diagnostics.is_empty());

    // didClose drops the overlay, reverting to the base document (broken).
    client.notify(
        "textDocument/didClose",
        serde_json::json!({ "textDocument": { "uri": uri } }),
    );
    let publishes = client.collect_publishes(2);
    let base = request_document(&[("app/main.cln", BROKEN)]);
    assert_parity(&publishes, &base);
    client.stop();
}

#[test]
fn request_integrity_failure_publishes_under_the_request_uri() {
    let mut document = request_document(&[("app/main.cln", FIXED)]);
    document["sources"][0]["sha256"] = serde_json::json!("0".repeat(64));
    let request_uri = "file:///workspace/.cln/request.json";
    let mut client = Client::start(serde_json::json!({
        "requestDocument": document,
        "requestDocumentUri": request_uri,
    }));
    let publishes = client.collect_publishes(2);
    assert_eq!(publishes[0].uri, request_uri);
    assert!(
        !publishes[0].diagnostics.is_empty(),
        "RQD001 lands in the request bucket"
    );
    let embedded = embedded_diagnostics(&publishes);
    assert!(embedded.iter().all(|d| d.code == "RQD001"));
    assert_parity(&publishes, &document);
    client.stop();
}

#[test]
fn malformed_request_document_publishes_intake_diagnostics() {
    // `sources` must be an array; the intake rejects this shape with the
    // same RQD002 the batch adapter reports, JSON path included.
    let document = serde_json::json!({
        "spec_version": "1",
        "project": { "name": "fixture", "version": "0.0.1" },
        "build": { "target": "wasm32-server", "optimization": "debug" },
        "sources": 5,
    });
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let publishes = client.collect_publishes(1);
    assert_eq!(publishes[0].uri, "clean:request");
    let embedded = embedded_diagnostics(&publishes);
    assert!(!embedded.is_empty());
    assert!(embedded.iter().all(|d| d.code == "RQD002"));

    // Reference: the same JSON through the same intake.
    let mut intake = clean_compiler::diag::DiagnosticSink::new();
    assert!(clean_compiler::request::from_json(&document.to_string(), &mut intake).is_none());
    let reference = clean_compiler::diag::finalize(
        intake.into_diagnostics(),
        &clean_compiler::diag::SourceCache::empty(),
    );
    assert_eq!(sorted(embedded), sorted(reference));
    client.stop();
}

#[test]
fn ranges_convert_to_utf16_code_units() {
    // Line 3 is `\t\tstring s = "👍" + nope`: the compiler reports `nope` at
    // columns 20..24, counted in characters (Platform 13 §2). U+1F44D is one
    // character but two UTF-16 code units, so on the wire (0-based UTF-16)
    // the range must start at 20 — a character count would say 19.
    let source = "functions:\n\tvoid init()\n\t\tstring s = \"👍\" + nope\n";
    let document = request_document(&[("app/main.cln", source)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let publishes = client.collect_publishes(2);
    assert_parity(&publishes, &document);

    let embedded = embedded_diagnostics(std::slice::from_ref(&publishes[1]));
    let sem002 = embedded
        .iter()
        .position(|d| d.code == "SEM002")
        .expect("the undefined-variable diagnostic is present");
    let compiler_span = &embedded[sem002].primary_span;
    assert_eq!(
        (compiler_span.start.line, compiler_span.start.column),
        (3, 20)
    );
    assert_eq!((compiler_span.end.line, compiler_span.end.column), (3, 24));

    let wire_range = publishes[1].diagnostics[sem002].range;
    assert_eq!(
        (wire_range.start.line, wire_range.start.character),
        (2, 20),
        "start converts to 0-based UTF-16, counting the surrogate pair"
    );
    assert_eq!((wire_range.end.line, wire_range.end.character), (2, 24));
    client.stop();
}

#[test]
fn foreign_documents_are_not_compiled() {
    let document = request_document(&[("app/main.cln", FIXED)]);
    let mut client = Client::start(serde_json::json!({ "requestDocument": document }));
    let _ = client.collect_publishes(2);
    client.notify(
        "textDocument/didOpen",
        serde_json::json!({
            "textDocument": {
                "uri": format!("{ROOT_URI}/scratch/other.cln"),
                "languageId": "clean", "version": 1, "text": "functions:\n"
            }
        }),
    );
    let log = client.wait_for_log();
    assert!(log.contains("not a request source"), "log names the cause");
    client.stop();
}
