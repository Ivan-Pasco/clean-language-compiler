//! Milestone 1 step 2 checks: the §14.2 invocation surface with no
//! compilation behind it — schema intake (`RQD002`), integrity (`RQD001`),
//! and well-formed diagnostics (Platform 13).

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::request::{from_json, validate};
use clean_compiler_types::codes;

mod common;

fn parse(
    json: &str,
) -> (
    Option<clean_compiler_types::CompileRequest>,
    Vec<clean_compiler_types::Diagnostic>,
) {
    let mut sink = DiagnosticSink::new();
    let request = from_json(json, &mut sink);
    (request, sink.into_diagnostics())
}

#[test]
fn accepts_minimal_valid_request() {
    let (request, diagnostics) = parse(&common::minimal_valid_request_json());
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut sink = DiagnosticSink::new();
    let validated = validate(request.unwrap(), &mut sink);
    assert!(validated.is_some());
    assert!(sink.is_empty());
}

#[test]
fn rqd002_on_unknown_top_level_key() {
    let mut value: serde_json::Value =
        serde_json::from_str(&common::minimal_valid_request_json()).unwrap();
    value["target"] = serde_json::json!("wasm32-server");
    let (request, diagnostics) = parse(&value.to_string());
    assert!(request.is_none());
    assert_eq!(diagnostics.len(), 1);
    let d = &diagnostics[0];
    assert_eq!(d.code, codes::RQD002);
    assert!(
        d.message.contains("unknown key `target`"),
        "message was: {}",
        d.message
    );
    // Spec example (Platform 10 §16): `unknown top-level key at '$.target'`.
    assert!(
        d.message.ends_with("at '$.target'"),
        "message was: {}",
        d.message
    );
}

#[test]
fn rqd002_on_unknown_section_key() {
    let mut value: serde_json::Value =
        serde_json::from_str(&common::minimal_valid_request_json()).unwrap();
    value["build"]["lto"] = serde_json::json!(true);
    let (request, diagnostics) = parse(&value.to_string());
    assert!(request.is_none());
    assert_eq!(diagnostics[0].code, codes::RQD002);
    assert!(
        diagnostics[0].message.contains("unknown key `lto`"),
        "message was: {}",
        diagnostics[0].message
    );
    assert!(
        diagnostics[0].message.contains("'$.build"),
        "message should scope the error to the build section: {}",
        diagnostics[0].message
    );
}

#[test]
fn rqd002_on_missing_target_world() {
    let mut value: serde_json::Value =
        serde_json::from_str(&common::minimal_valid_request_json()).unwrap();
    value.as_object_mut().unwrap().remove("target_world");
    let (request, diagnostics) = parse(&value.to_string());
    assert!(request.is_none());
    assert_eq!(diagnostics[0].code, codes::RQD002);
    assert!(
        diagnostics[0]
            .message
            .contains("missing required field `target_world`"),
        "message was: {}",
        diagnostics[0].message
    );
}

#[test]
fn rqd002_on_unsupported_spec_version() {
    let mut request = common::minimal_valid_request();
    request.spec_version = "2".to_string();
    let mut sink = DiagnosticSink::new();
    assert!(validate(request, &mut sink).is_none());
    let diagnostics = sink.into_diagnostics();
    assert_eq!(diagnostics[0].code, codes::RQD002);
    assert!(
        diagnostics[0].message.contains("highest supported is '1'"),
        "message was: {}",
        diagnostics[0].message
    );
}

#[test]
fn rqd002_on_target_world_hash_mismatch() {
    let mut request = common::minimal_valid_request();
    request.target_world.sha256 = "0".repeat(64);
    let mut sink = DiagnosticSink::new();
    assert!(validate(request, &mut sink).is_none());
    let diagnostics = sink.into_diagnostics();
    assert_eq!(diagnostics[0].code, codes::RQD002);
    assert!(diagnostics[0].message.contains("target_world.sha256"));
}

#[test]
fn rqd001_on_sha256_mismatch() {
    let mut request = common::minimal_valid_request();
    request.sources[0]
        .content
        .push_str("\nedited-without-rehashing()\n");
    let declared = request.sources[0].sha256.clone();
    let mut sink = DiagnosticSink::new();
    assert!(validate(request, &mut sink).is_none());
    let diagnostics = sink.into_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    let d = &diagnostics[0];
    assert_eq!(d.code, codes::RQD001);
    // Template from Platform 10 §16, verbatim slots: path, declared, actual.
    assert!(d
        .message
        .starts_with("request integrity failure: 'app/main.cln' declares sha256"));
    assert!(d.message.contains(&declared));
}

#[test]
fn rqd001_is_collected_for_every_bad_source() {
    let mut request = common::minimal_valid_request();
    let mut second = request.sources[0].clone();
    second.path = "app/other.cln".to_string();
    second.sha256 = "f".repeat(64);
    request.sources.push(second);
    request.sources[0].sha256 = "e".repeat(64);
    let mut sink = DiagnosticSink::new();
    assert!(validate(request, &mut sink).is_none());
    let diagnostics = sink.into_diagnostics();
    assert_eq!(
        diagnostics.len(),
        2,
        "both bad hashes must report in one run"
    );
    assert!(diagnostics.iter().all(|d| d.code == codes::RQD001));
}

#[test]
fn request_diagnostics_are_well_formed() {
    let (_, diagnostics) = parse("{}");
    let d = &diagnostics[0];
    assert_eq!(
        d.doc_url,
        format!("https://errors.cleanlanguage.dev/E/{}", d.code)
    );
    assert!(
        !d.rendered.is_empty(),
        "rendered CLI text is required (Platform 13 §6.1)"
    );
    assert!(d.rendered.starts_with(&format!("error[{}]:", d.code)));
    assert_eq!(d.primary_span.file, "<request>");
    // NDJSON line round-trips.
    let line = serde_json::to_string(d).unwrap();
    assert!(!line.contains('\n'));
    let back: clean_compiler_types::Diagnostic = serde_json::from_str(&line).unwrap();
    assert_eq!(&back, d);
}
