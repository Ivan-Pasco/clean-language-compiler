//! Milestone 1 step 3 checks: the vendored `clean-server` contract parses,
//! the `server` world exposes its eight interfaces plus the two guest entry
//! points, and a bad selector is a malformed request (`RQD002`).

use clean_compiler::codegen::world;
use clean_compiler::diag::DiagnosticSink;
use clean_compiler_types::codes;

mod common;

#[test]
fn server_world_exposes_eight_interfaces() {
    let request = common::minimal_valid_request();
    let mut sink = DiagnosticSink::new();
    let parsed = world::parse(&request.target_world, &mut sink).expect("host.wit parses");
    assert!(sink.is_empty());

    let interfaces = parsed.exported_interfaces();
    assert_eq!(
        interfaces,
        [
            "routing",
            "request",
            "response",
            "websocket",
            "sse",
            "session-envelope",
            "realtime-sockets",
            "log",
        ],
        "the server world's exported interfaces, in declaration order"
    );

    let entry_points = parsed.imported_functions();
    assert_eq!(entry_points, ["init", "handle"]);
}

#[test]
fn world_selector_mismatch_is_rqd002() {
    let mut request = common::minimal_valid_request();
    request.target_world.world = "browser".to_string();
    let mut sink = DiagnosticSink::new();
    assert!(world::parse(&request.target_world, &mut sink).is_none());
    let diagnostics = sink.into_diagnostics();
    assert_eq!(diagnostics[0].code, codes::RQD002);
    assert!(
        diagnostics[0]
            .message
            .contains("world 'browser' is not declared"),
        "message was: {}",
        diagnostics[0].message
    );
    assert!(diagnostics[0]
        .message
        .ends_with("at '$.target_world.world'"));
}

#[test]
fn malformed_wit_is_rqd002() {
    let mut request = common::minimal_valid_request();
    request.target_world.wit = "package clean:host@0.1.0;\nworld server {".to_string();
    let mut sink = DiagnosticSink::new();
    assert!(world::parse(&request.target_world, &mut sink).is_none());
    let diagnostics = sink.into_diagnostics();
    assert_eq!(diagnostics[0].code, codes::RQD002);
    assert!(diagnostics[0].message.contains("does not parse"));
}

/// The end-to-end pass [1] path: a valid request now carries its parsed
/// world through `validate`.
#[test]
fn validate_returns_parsed_world() {
    let request = common::minimal_valid_request();
    let mut sink = DiagnosticSink::new();
    let validated =
        clean_compiler::request::validate(request, &mut sink).expect("valid request validates");
    assert_eq!(validated.world.exported_interfaces().len(), 8);
}
