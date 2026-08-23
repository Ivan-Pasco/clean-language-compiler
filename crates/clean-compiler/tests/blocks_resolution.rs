//! Block-name resolution behaviors (chapter 21 §21.2, LEX-05) beyond the
//! DIA-06 fixtures: explicit-import precedence over folder ambiguity,
//! exact qualified-name lookup (no prefix fallback), the `core.` reserved
//! prefix, and whole-segment folder matching. Every fixture handler
//! returns `ir.empty()`, so a successful resolution is simply a clean
//! check of the expanded program.

mod common;

use clean_compiler_types::request::{CompileRequest, Dependency, LibraryManifest};

fn manifest(name: &str, handles: &[&str]) -> LibraryManifest {
    common::handler_manifest(name, handles, common::EMPTY_ENVELOPE)
}

fn dependency() -> Dependency {
    Dependency {
        version: "1.0.0".to_string(),
        resolved_from: "registry".to_string(),
    }
}

/// One source at `path`, with the given libraries as dependencies+manifests
/// and a single folder mapping.
fn block_request(
    path: &str,
    content: &str,
    folders: &[(&str, &[&str])],
    manifests: Vec<LibraryManifest>,
) -> CompileRequest {
    let mut request = common::minimal_valid_request();
    request.sources[0] = clean_compiler_types::request::SourceFile {
        path: path.to_string(),
        sha256: common::sha256_hex(content.as_bytes()),
        content: content.to_string(),
    };
    for m in &manifests {
        request.dependencies.insert(m.name.clone(), dependency());
    }
    for (folder, libs) in folders {
        request.folders.insert(
            folder.to_string(),
            libs.iter().map(|s| s.to_string()).collect(),
        );
    }
    request.library_manifests = manifests;
    request
}

fn codes_of(request: CompileRequest) -> Vec<String> {
    match clean_compiler::check(request) {
        Ok(diagnostics) => diagnostics.into_iter().map(|d| d.code).collect(),
        Err(err) => panic!("expected diagnostics, got {err:?}"),
    }
}

/// A resolved block expands (here to nothing) and the program checks
/// clean.
fn assert_resolves(request: CompileRequest) {
    match clean_compiler::check(request) {
        Ok(diagnostics) => {
            let errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.level == clean_compiler_types::Level::Error)
                .collect();
            assert!(errors.is_empty(), "expected a clean check, got {errors:?}");
        }
        Err(err) => panic!("expected a clean check, got {err:?}"),
    }
}

#[test]
fn explicit_import_wins_over_folder_ambiguity() {
    // Both libraries are in implicit folder scope and both register
    // `data` — BLOCK001 territory — but the file imports `alpha`
    // explicitly, which §21.2 rule 1 makes the winner.
    let content = "import:\n\talpha\n\ndata UserData:\n\tinteger id primary\n";
    let request = block_request(
        "app/main.cln",
        content,
        &[("app", &["alpha", "beta"])],
        vec![manifest("alpha", &["data"]), manifest("beta", &["data"])],
    );
    assert_resolves(request);
}

#[test]
fn single_folder_library_resolves_implicitly() {
    let content = "data UserData:\n\tinteger id primary\n";
    let request = block_request(
        "app/data/User.cln",
        content,
        &[("app/data", &["alpha"])],
        vec![manifest("alpha", &["data"])],
    );
    assert_resolves(request);
}

#[test]
fn qualified_names_need_exact_registration() {
    // `data.query` looks up the exact string — a `data` registration is
    // not a prefix fallback (§21.2).
    let content = "data.query UserQuery:\n\tinteger id\n";
    let request = block_request(
        "app/main.cln",
        content,
        &[("app", &["alpha"])],
        vec![manifest("alpha", &["data"])],
    );
    let codes = codes_of(request);
    assert!(
        codes.contains(&"BLOCK002".to_string()),
        "expected BLOCK002, got {codes:?}"
    );
}

#[test]
fn core_prefix_is_reserved_at_library_load() {
    let content = "functions:\n\tvoid init()\n\t\treturn\n";
    let request = block_request(
        "app/main.cln",
        content,
        &[],
        vec![manifest("alpha", &["core.data"])],
    );
    let codes = codes_of(request);
    assert!(
        codes.contains(&"BLOCK003".to_string()),
        "expected BLOCK003, got {codes:?}"
    );
}

#[test]
fn folder_matching_is_whole_segment() {
    // `app` must not prefix-match `application/…`.
    let content = "data UserData:\n\tinteger id primary\n";
    let request = block_request(
        "application/main.cln",
        content,
        &[("app", &["alpha"])],
        vec![manifest("alpha", &["data"])],
    );
    let codes = codes_of(request);
    assert!(
        codes.contains(&"BLOCK002".to_string()),
        "expected BLOCK002 (folder scope must not leak), got {codes:?}"
    );
}

#[test]
fn glob_suffix_and_plain_folder_keys_are_equivalent() {
    let content = "data UserData:\n\tinteger id primary\n";
    let request = block_request(
        "app/data/User.cln",
        content,
        &[("app/data/**", &["alpha"])],
        vec![manifest("alpha", &["data"])],
    );
    assert_resolves(request);
}

#[test]
fn library_without_wasm_is_lib004_at_load() {
    let content = "functions:\n\tvoid init()\n\t\treturn\n";
    let mut lib = manifest("alpha", &["data"]);
    lib.compiletime_wasm = None;
    let request = block_request("app/main.cln", content, &[], vec![lib]);
    let codes = codes_of(request);
    assert!(
        codes.contains(&"LIB004".to_string()),
        "expected LIB004 (handles_blocks without compiletime_wasm), got {codes:?}"
    );
}

/// BLOCK003, source leg (21 §21.2): span on the `handles block`
/// declaration, `{library}` = the project's name, primary label
/// `reserved block name` — pinned in full here because the code's DIA-06
/// triple exercises the manifest leg (span on the request document).
#[test]
fn source_handles_block_with_reserved_name_is_block003() {
    let content = "compiletime function expandData(BlockAST ast) returns IR\n\treturn 0\n\nhandles block \"state\" with expandData\n";
    let request = block_request("app/main.cln", content, &[], vec![]);
    let diagnostics = match clean_compiler::check(request) {
        Ok(diagnostics) => diagnostics,
        Err(err) => panic!("expected diagnostics, got {err:?}"),
    };
    let d = diagnostics
        .iter()
        .find(|d| d.code == "BLOCK003")
        .unwrap_or_else(|| panic!("expected BLOCK003, got {diagnostics:?}"));
    assert_eq!(
        d.message,
        "library 'fixture' registers reserved block name `state`"
    );
    assert_eq!(d.primary_label.as_deref(), Some("reserved block name"));
    assert_eq!(d.primary_span.file, "app/main.cln");
    assert_eq!(d.primary_span.start.line, 4);
}

/// BLOCK007/BLOCK008 (BLK-01): shape violations of the bound handler.
#[test]
fn handler_with_wrong_parameter_shape_is_block007() {
    let content = "compiletime function expandData(integer n) returns IR\n\treturn 0\n\nhandles block \"custom\" with expandData\n";
    let request = block_request("app/main.cln", content, &[], vec![]);
    let codes = codes_of(request);
    assert!(
        codes.contains(&"BLOCK007".to_string()),
        "expected BLOCK007, got {codes:?}"
    );
}

#[test]
fn handler_with_optional_ir_return_is_block008() {
    let content = "compiletime function expandData(BlockAST ast) returns IR?\n\treturn 0\n\nhandles block \"custom\" with expandData\n";
    let request = block_request("app/main.cln", content, &[], vec![]);
    let codes = codes_of(request);
    assert!(
        codes.contains(&"BLOCK008".to_string()),
        "expected BLOCK008, got {codes:?}"
    );
}

/// BLOCK009 fires on the name form alone and short-circuits the other
/// registration checks (no cascading SEM019/BLOCK003).
#[test]
fn malformed_block_name_is_block009_alone() {
    let content = "handles block \"has space\" with missingHandler\n";
    let request = block_request("app/main.cln", content, &[], vec![]);
    let codes = codes_of(request);
    assert_eq!(codes, vec!["BLOCK009".to_string()], "got {codes:?}");
}

#[test]
fn source_handles_block_with_missing_handler_is_sem019() {
    let content = "handles block \"custom\" with missingHandler\n";
    let request = block_request("app/main.cln", content, &[], vec![]);
    let codes = codes_of(request);
    assert!(
        codes.contains(&"SEM019".to_string()),
        "expected SEM019, got {codes:?}"
    );
}

#[test]
fn valid_registration_pair_reaches_the_body_frontier() {
    // BLK-01 accepted; the body's TYP-04 checking is a recorded spec
    // blocker (DISCOVERIES-M5) and stays on the Unsupported channel.
    let content = "compiletime function expandData(BlockAST ast) returns IR\n\treturn 0\n\nhandles block \"custom\" with expandData\n";
    let request = block_request("app/main.cln", content, &[], vec![]);
    match clean_compiler::check(request) {
        Err(clean_compiler::driver::CompileError::Unsupported(notes)) => {
            assert!(
                notes
                    .iter()
                    .any(|n| n.construct == "compiletime function bodies"),
                "expected the body frontier note, got {notes:?}"
            );
        }
        other => panic!("expected Unsupported(compiletime function bodies), got {other:?}"),
    }
}

/// MOD-04/05 collision ladder (ratified 2026-08-22): sources-module
/// resolution is tried before the library-manifest leg, so a module and
/// a library sharing a name resolve in the module's favor.
#[test]
fn module_beats_library_on_name_collision() {
    let module = "functions:\n\tpublic:\n\t\tinteger fromModule(integer a)\n\t\t\treturn a\n";
    let main = "import:\n\tdata\n\nfunctions:\n\tvoid init()\n\t\tinteger x = fromModule(1)\n\t\treturn\n";
    let mut request = block_request("app/main.cln", main, &[], vec![manifest("data", &["data"])]);
    request
        .sources
        .push(clean_compiler_types::request::SourceFile {
            path: "app/data.cln".to_string(),
            sha256: common::sha256_hex(module.as_bytes()),
            content: module.to_string(),
        });
    assert_resolves(request);
}

/// MOD-04/05 collision ladder: the built-in rung beats the library leg —
/// a manifest named `math` does not capture `import: math`, which stays
/// on the standard-library frontier.
#[test]
fn builtin_beats_library_on_name_collision() {
    let main = "import:\n\tmath\n\nfunctions:\n\tvoid init()\n\t\treturn\n";
    let request = block_request("app/main.cln", main, &[], vec![manifest("math", &["math"])]);
    match clean_compiler::check(request) {
        Err(clean_compiler::driver::CompileError::Unsupported(notes)) => {
            assert!(
                notes
                    .iter()
                    .any(|n| n.construct == "standard-library imports"),
                "expected the built-in frontier note, got {notes:?}"
            );
        }
        other => panic!("the built-in rung must win: {other:?}"),
    }
}
