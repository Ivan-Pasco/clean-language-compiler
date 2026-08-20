//! Contract tests for the diagnostics-only build (Platform 14 §14.14.4):
//! `check` runs passes 1–9 over the same request document as `compile`,
//! reports the same diagnostics, and can never produce a component.

mod common;

use clean_compiler_types::request::CompileRequest;

fn request_with_source(content: &str) -> CompileRequest {
    let mut request = common::minimal_valid_request();
    request.sources[0].content = content.to_string();
    request.sources[0].sha256 = common::sha256_hex(content.as_bytes());
    request
}

#[test]
fn check_passes_a_valid_program_with_no_diagnostics() {
    let diagnostics = clean_compiler::check(common::minimal_valid_request())
        .expect("check runs passes 1-9 on the M1 surface");
    assert!(
        diagnostics.is_empty(),
        "clean program produced {diagnostics:?}"
    );
}

#[test]
fn check_reports_the_same_errors_a_build_would() {
    let request = request_with_source("functions:\n\tvoid init()\n\t\treturn undefinedName\n");
    let checked = clean_compiler::check(request.clone()).expect("check completes");
    assert!(
        checked
            .iter()
            .any(|d| d.level == clean_compiler::types::Level::Error),
        "expected at least one error, got {checked:?}"
    );

    let built = match clean_compiler::compile(request) {
        Err(clean_compiler::CompileError::Rejected(diagnostics)) => diagnostics,
        other => panic!("build should reject the same program, got {other:?}"),
    };
    let key = |d: &clean_compiler_types::Diagnostic| (d.code.clone(), d.message.clone());
    assert_eq!(
        checked.iter().map(key).collect::<Vec<_>>(),
        built.iter().map(key).collect::<Vec<_>>(),
        "cln check passing where cln build fails would make the fast path untrustworthy (§14.14.4)"
    );
}

/// §14.14.4: there is no reduced request shape for checking — a request
/// without `target_world` is refused with RQD002 exactly like a build. The
/// field is required at the schema level, so the refusal happens at intake
/// (`from_json`), the same path the `--check` binary invocation takes.
#[test]
fn check_refuses_a_request_without_target_world() {
    let mut json = serde_json::to_value(common::minimal_valid_request()).unwrap();
    json.as_object_mut().unwrap().remove("target_world");
    let mut intake = clean_compiler::diag::DiagnosticSink::new();
    let request = clean_compiler::request::from_json(&json.to_string(), &mut intake);
    assert!(request.is_none(), "intake must refuse the request");
    let diagnostics = intake.into_diagnostics();
    assert!(
        diagnostics.iter().any(|d| d.code == "RQD002"),
        "missing target_world must be RQD002, got {diagnostics:?}"
    );
}

/// 07 §7.8: the request's own `max_nesting_depth` governs — BLD001 names
/// the request's cap, not the default.
#[test]
fn custom_max_nesting_depth_is_honored() {
    let mut request = common::minimal_valid_request();
    request.compile_limits.max_nesting_depth = 8;
    let content = format!(
        "functions:\n\tvoid init()\n\t\tinteger x = {}1{}\n\t\treturn\n",
        "(".repeat(20),
        ")".repeat(20)
    );
    request.sources[0].sha256 = common::sha256_hex(content.as_bytes());
    request.sources[0].content = content;
    let diagnostics = clean_compiler::check(request).expect("check reports, does not fail");
    assert!(
        diagnostics.iter().any(|d| d.code == "BLD001"
            && d.message == "build limit 'max-nesting-depth' exceeded: 9 > 8"),
        "{diagnostics:#?}"
    );
}
