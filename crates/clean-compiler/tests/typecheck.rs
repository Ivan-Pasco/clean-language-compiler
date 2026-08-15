//! Milestone 1 step 5c checks: resolve + typecheck over multi-file
//! programs, with the world-typed boundary projections of ADR-0002 —
//! enum-by-string-literal, class↔record, width-suffixed integers — and the
//! SEM/FUNC paths with their Platform 10 wording.

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{compile, CompileError};
use clean_compiler_types::codes;

mod common;

/// Builds a request whose sources are (path, content) pairs, with hashes
/// computed so pass [1] accepts them.
fn request_for(sources: &[(&str, &str)]) -> clean_compiler_types::CompileRequest {
    let mut request = common::minimal_valid_request();
    request.sources = sources
        .iter()
        .map(
            |(path, content)| clean_compiler_types::request::SourceFile {
                path: path.to_string(),
                sha256: common::sha256_hex(content.as_bytes()),
                content: content.to_string(),
            },
        )
        .collect();
    request
}

const HOST_BRIDGE: &str = "\
host interface routing version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function register(m: method, path: string, handlerId: integer:u32, opts: options)
\t\tdescription \"Register one route.\"

host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function setStatus(status: integer:u16)
\t\tdescription \"Set the status code.\"
";

const OPTIONS_CLASS: &str = "class Options\n\tboolean csrf\n";

fn rejected(sources: &[(&str, &str)]) -> Vec<clean_compiler_types::Diagnostic> {
    match compile(request_for(sources)) {
        Err(CompileError::Rejected(diagnostics)) => diagnostics,
        other => panic!("expected rejection, got {other:?}"),
    }
}

/// Passes when the program survives typecheck with no diagnostics — later
/// pipeline gaps (`Incomplete`) and constructs beyond the current build
/// (`Unsupported`, e.g. strings before step 6) are not rejections.
fn typechecks(sources: &[(&str, &str)]) {
    match compile(request_for(sources)) {
        Ok(_) | Err(CompileError::Incomplete { .. }) | Err(CompileError::Unsupported(_)) => {}
        Err(CompileError::Rejected(diagnostics)) => {
            panic!("program was rejected: {diagnostics:#?}")
        }
    }
}

#[test]
fn acceptance_shape_typechecks_across_files() {
    let main = "\
functions:
\tvoid init()
\t\tregister(\"get\", \"/\", 0, Options(true))
\tvoid handle(integer handlerId)
\t\tif handlerId == 0
\t\t\tsetStatus(200)
\t\telse
\t\t\tsetStatus(404)
";
    typechecks(&[
        ("app/host_bridge.cln", HOST_BRIDGE),
        ("app/options.cln", OPTIONS_CLASS),
        ("app/main.cln", main),
    ]);
}

#[test]
fn unknown_enum_case_is_sem016_listing_cases() {
    let main = "\
functions:
\tvoid init()
\t\tregister(\"fetch\", \"/\", 0, Options(true))
";
    let diagnostics = rejected(&[
        ("app/host_bridge.cln", HOST_BRIDGE),
        ("app/options.cln", OPTIONS_CLASS),
        ("app/main.cln", main),
    ]);
    let d = &diagnostics[0];
    assert_eq!(d.code, codes::SEM016);
    assert!(d
        .message
        .contains("`\"fetch\"` is not a case of enum `method`"));
    assert!(d
        .primary_label
        .as_deref()
        .unwrap()
        .contains("get, head, post"));
}

#[test]
fn enum_case_requires_a_literal_not_a_variable() {
    let main = "\
functions:
\tvoid init()
\t\tstring m = \"get\"
\t\tregister(m, \"/\", 0, Options(true))
";
    let diagnostics = rejected(&[
        ("app/host_bridge.cln", HOST_BRIDGE),
        ("app/options.cln", OPTIONS_CLASS),
        ("app/main.cln", main),
    ]);
    assert_eq!(diagnostics[0].code, codes::SEM016);
    assert!(diagnostics[0]
        .message
        .contains("argument `1` of `register` has the wrong type"));
}

#[test]
fn out_of_range_literal_for_boundary_width_is_sem026() {
    let main = "\
functions:
\tvoid init()
\t\tsetStatus(70000)
";
    let diagnostics = rejected(&[("app/host_bridge.cln", HOST_BRIDGE), ("app/main.cln", main)]);
    assert_eq!(diagnostics[0].code, codes::SEM026);
    assert_eq!(
        diagnostics[0].message,
        "literal 70000 does not fit integer:u16 (range 0 to 65535)"
    );
}

#[test]
fn negative_literal_folds_before_range_check() {
    // -9223372036854775808 fits integer only after the unary minus folds.
    let main = "\
functions:
\tvoid init()
\t\tinteger floor = -9223372036854775808
\t\treturn
";
    typechecks(&[("app/main.cln", main)]);
}

#[test]
fn condition_must_be_boolean_sem023() {
    let main = "\
functions:
\tvoid f()
\t\tif 1
\t\t\treturn
";
    let diagnostics = rejected(&[("app/main.cln", main)]);
    assert_eq!(diagnostics[0].code, codes::SEM023);
    assert_eq!(
        diagnostics[0].message,
        "Condition must be a boolean expression, found integer"
    );
    assert_eq!(
        diagnostics[0].primary_label.as_deref(),
        Some("expected boolean")
    );
}

#[test]
fn assignment_mismatch_is_sem001_with_labels() {
    let main = "\
functions:
\tvoid f()
\t\tinteger x = \"hello\"
";
    let diagnostics = rejected(&[("app/main.cln", main)]);
    let d = &diagnostics[0];
    assert_eq!(d.code, codes::SEM001);
    assert_eq!(d.message, "type mismatch in assignment");
    assert_eq!(
        d.primary_label.as_deref(),
        Some("`x` is declared with type `integer`")
    );
    assert_eq!(d.secondary[0].label, "this expression has type `string`");
}

#[test]
fn undefined_names_report_sem002_and_sem019() {
    let main = "\
functions:
\tvoid f()
\t\tinteger x = y
\t\tmissing()
";
    let diagnostics = rejected(&[("app/main.cln", main)]);
    assert_eq!(diagnostics[0].code, codes::SEM002);
    assert_eq!(
        diagnostics[0].message,
        "I cannot find a variable named `y` in scope"
    );
    assert_eq!(diagnostics[1].code, codes::SEM019);
    assert_eq!(
        diagnostics[1].message,
        "I cannot find a function named `missing`"
    );
}

#[test]
fn return_type_mismatch_is_sem015() {
    let main = "\
functions:
\tinteger f()
\t\treturn true
";
    let diagnostics = rejected(&[("app/main.cln", main)]);
    assert_eq!(diagnostics[0].code, codes::SEM015);
    assert_eq!(diagnostics[0].message, "return type mismatch in `f`");
}

#[test]
fn wrong_arity_is_func002() {
    let main = "\
functions:
\tinteger add(integer a, integer b)
\t\treturn a + b
\tvoid f()
\t\tinteger x = add(1, 2, 3)
\t\treturn
";
    let diagnostics = rejected(&[("app/main.cln", main)]);
    assert_eq!(diagnostics[0].code, codes::FUNC002);
    assert!(diagnostics[0]
        .message
        .contains("`add` expects 2 argument(s), got 3"));
}

#[test]
fn top_level_redefinition_is_sem003_across_files() {
    let a = "functions:\n\tvoid f()\n\t\treturn\n";
    let b = "functions:\n\tvoid f()\n\t\treturn\n";
    let diagnostics = rejected(&[("app/a.cln", a), ("app/b.cln", b)]);
    assert_eq!(diagnostics[0].code, codes::SEM003);
}

#[test]
fn local_redeclaration_is_scope002() {
    let main = "\
functions:
\tvoid f()
\t\tinteger x = 1
\t\tinteger x = 2
\t\treturn
";
    let diagnostics = rejected(&[("app/main.cln", main)]);
    assert_eq!(diagnostics[0].code, codes::SCOPE002);
}

#[test]
fn unsupported_constructs_surface_as_typed_error_not_diagnostics() {
    let main = "\
functions:
\tvoid f()
\t\tstring s = \"total: {1 + 2}\"
\t\treturn
";
    match compile(request_for(&[("app/main.cln", main)])) {
        Err(CompileError::Unsupported(constructs)) => {
            assert_eq!(constructs[0].construct, "string interpolation");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn kebab_projection_matches_lbs_example() {
    use clean_compiler::typecheck::types::kebab;
    assert_eq!(kebab("setInnerHTML"), "set-inner-html");
    assert_eq!(kebab("getParam"), "get-param");
    assert_eq!(kebab("register"), "register");
    assert_eq!(kebab("handlerId"), "handler-id");
}

/// Silences the unused-helper lint for helpers other suites use.
#[allow(dead_code)]
fn _use_sink(_: DiagnosticSink) {}
