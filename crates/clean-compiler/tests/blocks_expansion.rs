//! Pass-[6] expansion end to end (chapter 21, ADR-0004, ADR-0003 wire
//! ABI): a handler's IR splices into the program and compiles; the same
//! request expands byte-identically (CMP-02 — the M5 exit criterion);
//! envelope defects are `BLOCK004`; BLK-03 diagnostics surface as
//! `LIB010`; re-validation errors inside a block attribute to the handler.

mod common;

use clean_compiler_types::request::{CompileRequest, Dependency, SourceFile};
use clean_compiler_types::Level;

const BLOCK_SRC: &str = "data UserData:\n\tinteger id primary\n";

/// A request whose one file (at `app/main.cln`) is `content`, with library
/// `alpha` folder-scoped over `app/` and handled by `envelope`.
fn request_with_handler(content: &str, envelope: &str) -> CompileRequest {
    let mut request = common::minimal_valid_request();
    request.sources[0] = SourceFile {
        path: "app/main.cln".to_string(),
        sha256: common::sha256_hex(content.as_bytes()),
        content: content.to_string(),
    };
    request.dependencies.insert(
        "alpha".to_string(),
        Dependency {
            version: "1.0.0".to_string(),
            resolved_from: "registry".to_string(),
        },
    );
    request
        .folders
        .insert("app".to_string(), vec!["alpha".to_string()]);
    request.library_manifests = vec![common::handler_manifest("alpha", &["data"], envelope)];
    request
}

fn error_codes(request: CompileRequest) -> Vec<String> {
    match clean_compiler::check(request) {
        Ok(diagnostics) => diagnostics
            .into_iter()
            .filter(|d| d.level == Level::Error)
            .map(|d| d.code)
            .collect(),
        Err(err) => panic!("expected diagnostics, got {err:?}"),
    }
}

#[test]
fn emitted_function_is_callable_from_user_code() {
    // The handler turns `data UserData:` into a function the file's own
    // code calls — splice, re-resolve, re-check, and codegen all cross
    // the expansion boundary.
    let envelope = r#"{"ir":{"kind":"function","name":"answer","params":[],"return":{"kind":"integer"},"body":{"kind":"return","expression":{"kind":"literal_integer","value":42}}},"diagnostics":[]}"#;
    let content =
        "data UserData:\n\tinteger id primary\n\nfunctions:\n\tvoid init()\n\t\tinteger x = answer()\n\t\treturn\n";
    let artifact = clean_compiler::compile(request_with_handler(content, envelope))
        .expect("expanded program compiles");
    assert!(!artifact.wasm.is_empty());
}

#[test]
fn expansion_is_byte_deterministic() {
    // The M5 exit criterion: same request ⇒ byte-identical expansion,
    // through to the emitted component and manifest (CMP-02).
    let envelope = r#"{"ir":{"kind":"function","name":"answer","params":[],"return":{"kind":"integer"},"body":{"kind":"return","expression":{"kind":"literal_integer","value":42}}},"diagnostics":[]}"#;
    let content =
        "data UserData:\n\tinteger id primary\n\nfunctions:\n\tvoid init()\n\t\tinteger x = answer()\n\t\treturn\n";
    let first = clean_compiler::compile(request_with_handler(content, envelope))
        .expect("first compile succeeds");
    let second = clean_compiler::compile(request_with_handler(content, envelope))
        .expect("second compile succeeds");
    assert_eq!(first.wasm, second.wasm, "component bytes must be identical");
    assert_eq!(
        serde_json::to_string(&first.manifest).unwrap(),
        serde_json::to_string(&second.manifest).unwrap(),
        "build manifests must be identical"
    );
}

#[test]
fn unknown_ir_kind_is_block004() {
    let envelope = r#"{"ir":{"kind":"bogus"},"diagnostics":[]}"#;
    let codes = error_codes(request_with_handler(BLOCK_SRC, envelope));
    assert_eq!(codes, vec!["BLOCK004".to_string()], "unexpected: {codes:?}");
}

#[test]
fn with_span_outside_the_block_is_block004() {
    // A synthetic span (wrong file) must not re-anchor anything.
    let envelope = r#"{"ir":{"kind":"with_span","span":{"file":"elsewhere.cln","line":1,"column":1,"byteRange":[0,5]},"node":{"kind":"empty"}},"diagnostics":[]}"#;
    let codes = error_codes(request_with_handler(BLOCK_SRC, envelope));
    assert_eq!(codes, vec!["BLOCK004".to_string()], "unexpected: {codes:?}");
}

#[test]
fn handler_error_diagnostic_is_lib010_and_halts() {
    // BLK-03: `error(...)` surfaces as LIB010 at the handler-supplied
    // span (a real span inside the block) and fails the compilation.
    let envelope = r#"{"ir":{"kind":"empty"},"diagnostics":[{"severity":"error","code":"field-missing-type","message":"the field `id` needs a type","span":{"file":"app/main.cln","line":2,"column":2,"byteRange":[16,34]}}]}"#;
    let diagnostics =
        clean_compiler::check(request_with_handler(BLOCK_SRC, envelope)).expect("check runs");
    let lib010 = diagnostics
        .iter()
        .find(|d| d.code == "LIB010")
        .expect("LIB010 emitted");
    assert_eq!(lib010.level, Level::Error);
    assert!(
        lib010.message.contains("[via alpha::data]"),
        "attribution missing: {}",
        lib010.message
    );
    assert_eq!(lib010.primary_span.start.line, 2, "handler span preserved");
}

#[test]
fn handler_warning_does_not_halt() {
    let envelope = r#"{"ir":{"kind":"empty"},"diagnostics":[{"severity":"warning","code":"deprecated-form","message":"prefer `fields:`","span":{"file":"app/main.cln","line":1,"column":1,"byteRange":[0,14]}}]}"#;
    let diagnostics =
        clean_compiler::check(request_with_handler(BLOCK_SRC, envelope)).expect("check runs");
    assert!(
        diagnostics.iter().all(|d| d.level != Level::Error),
        "warning must not fail the build: {diagnostics:?}"
    );
    assert!(diagnostics.iter().any(|d| d.code == "LIB010"));
}

#[test]
fn synthetic_diagnostic_span_is_block004() {
    let envelope = r#"{"ir":{"kind":"empty"},"diagnostics":[{"severity":"error","code":"x","message":"y","span":{"file":"not-a-source.cln","line":1,"column":1,"byteRange":[0,4]}}]}"#;
    let codes = error_codes(request_with_handler(BLOCK_SRC, envelope));
    assert_eq!(codes, vec!["BLOCK004".to_string()], "unexpected: {codes:?}");
}

#[test]
fn undefined_symbol_in_generated_code_is_block004() {
    // §21.4: IR referencing an undefined symbol → diagnostic at the user
    // span, filed under BLOCK004 (the handler's defect, not the user's).
    let envelope = r#"{"ir":{"kind":"function","name":"broken","params":[],"return":{"kind":"integer"},"body":{"kind":"return","expression":{"kind":"call","qualified_name":"missing","arguments":[]}}},"diagnostics":[]}"#;
    let codes = error_codes(request_with_handler(BLOCK_SRC, envelope));
    assert!(
        codes.contains(&"BLOCK004".to_string()),
        "expected BLOCK004 attribution, got {codes:?}"
    );
}

#[test]
fn node_budget_is_lib014() {
    // 500 001 `empty` fragments under one flat `concat` — the LIB014
    // generated-IR cap, exercised at the lowerer (a wasm fixture carrying
    // an 8 MB envelope lands with the DIA-06 triple).
    let mut fragments = Vec::with_capacity(500_001);
    for _ in 0..500_001 {
        fragments.push(serde_json::json!({"kind": "empty"}));
    }
    let ir = serde_json::json!({"kind": "concat", "fragments": fragments});
    let mut lowerer =
        clean_compiler::blocks::ir::Lowerer::new(clean_compiler::source::ByteSpan::new(0, 10), "a");
    match lowerer.items(&ir) {
        Err(clean_compiler::blocks::ir::LowerError::NodeLimit) => {}
        Ok(_) => panic!("node budget did not fire"),
        Err(clean_compiler::blocks::ir::LowerError::Malformed(reason)) => {
            panic!("wrong failure: {reason}")
        }
    }
}

#[test]
fn pathological_nesting_is_malformed_not_a_crash() {
    // 200 nested concats exceed MAX_IR_DEPTH; the lowerer must refuse,
    // not overflow the stack (ICE prevention).
    let mut node = serde_json::json!({"kind": "empty"});
    for _ in 0..200 {
        node = serde_json::json!({"kind": "concat", "fragments": [node]});
    }
    let mut lowerer =
        clean_compiler::blocks::ir::Lowerer::new(clean_compiler::source::ByteSpan::new(0, 10), "a");
    match lowerer.items(&node) {
        Err(clean_compiler::blocks::ir::LowerError::Malformed(reason)) => {
            assert!(reason.contains("depth"), "unexpected reason: {reason}")
        }
        _ => panic!("depth guard did not fire"),
    }
}

#[test]
fn class_with_fields_expands_and_compiles() {
    // ir.class with two fields; the program only declares it (M1 surface
    // for classes is limited, so no instantiation here) — the point is
    // that a class declaration crosses splice + re-check.
    let envelope = r#"{"ir":{"kind":"class","name":"UserData","fields":[{"name":"id","type":{"kind":"integer"}},{"name":"email","type":{"kind":"string"}}],"methods":[]},"diagnostics":[]}"#;
    let diagnostics =
        clean_compiler::check(request_with_handler(BLOCK_SRC, envelope)).expect("check runs");
    assert!(
        diagnostics.iter().all(|d| d.level != Level::Error),
        "expected clean expansion, got {diagnostics:?}"
    );
}
