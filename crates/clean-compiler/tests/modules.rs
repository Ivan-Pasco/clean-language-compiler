//! M4 module-graph checks (chapter 17): visibility across modules
//! (MOD-01/MOD-02), request-only resolution (MOD-03), the IMPORT family,
//! and the local adoptions recorded in docs/DISCOVERIES-M4.md (classes
//! exported by default, local names shadow imports).

use clean_compiler::{compile, CompileError};

mod common;

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

fn diagnostics(sources: &[(&str, &str)]) -> Vec<clean_compiler_types::Diagnostic> {
    match compile(request_for(sources)) {
        Err(CompileError::Rejected(diagnostics)) => diagnostics,
        other => panic!("expected rejection, got {other:?}"),
    }
}

fn typechecks(sources: &[(&str, &str)]) {
    match compile(request_for(sources)) {
        Ok(_) | Err(CompileError::Incomplete { .. }) | Err(CompileError::Unsupported(_)) => {}
        Err(CompileError::Rejected(diagnostics)) => {
            panic!("program was rejected: {diagnostics:#?}")
        }
    }
}

#[test]
fn imported_public_function_is_visible() {
    typechecks(&[
        (
            "utils.cln",
            "functions:\n\tpublic:\n\t\tinteger add(integer a, integer b)\n\t\t\treturn a + b\n",
        ),
        (
            "main.cln",
            "import:\n\tutils\n\nfunctions:\n\tvoid init()\n\t\tinteger s = add(2, 3)\n\t\treturn\n",
        ),
    ]);
}

#[test]
fn private_function_is_invisible_across_modules() {
    let diagnostics = diagnostics(&[
        (
            "utils.cln",
            "functions:\n\tinteger hidden()\n\t\treturn 42\n",
        ),
        (
            "main.cln",
            "import:\n\tutils\n\nfunctions:\n\tvoid init()\n\t\tinteger x = hidden()\n\t\treturn\n",
        ),
    ]);
    // MOD-02: not exported ⇒ not in scope; the use site reports SEM019.
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "SEM019" && d.primary_span.file == "main.cln"),
        "expected SEM019 for the private function, got {diagnostics:#?}"
    );
}

#[test]
fn unimported_module_is_not_in_scope() {
    let diagnostics = diagnostics(&[
        (
            "utils.cln",
            "functions:\n\tpublic:\n\t\tinteger add(integer a, integer b)\n\t\t\treturn a + b\n",
        ),
        (
            "main.cln",
            "functions:\n\tvoid init()\n\t\tinteger s = add(2, 3)\n\t\treturn\n",
        ),
    ]);
    assert!(
        diagnostics.iter().any(|d| d.code == "SEM019"),
        "no import, no visibility: {diagnostics:#?}"
    );
}

#[test]
fn symbol_import_binds_under_alias() {
    typechecks(&[
        (
            "utils.cln",
            "functions:\n\tpublic:\n\t\tinteger add(integer a, integer b)\n\t\t\treturn a + b\n",
        ),
        (
            "main.cln",
            "import:\n\tutils.add as plus\n\nfunctions:\n\tvoid init()\n\t\tinteger s = plus(2, 3)\n\t\treturn\n",
        ),
    ]);
}

#[test]
fn local_declaration_shadows_import() {
    // Adoption (DISCOVERIES-M4): a locally declared name wins over an
    // imported one — the local `add` takes one argument and the call
    // resolves against it.
    typechecks(&[
        (
            "utils.cln",
            "functions:\n\tpublic:\n\t\tinteger add(integer a, integer b)\n\t\t\treturn a + b\n",
        ),
        (
            "main.cln",
            "import:\n\tutils\n\nfunctions:\n\tinteger add(integer a)\n\t\treturn a\n\n\tvoid init()\n\t\tinteger s = add(2)\n\t\treturn\n",
        ),
    ]);
}

#[test]
fn every_cycle_reports_once_per_build() {
    // MOD-03: one cycle → exactly one IMPORT001; resolution continues
    // within the pass so one build reports every cycle it finds. (The
    // pipeline still stops after pass [4] on errors, Platform 09 §2.)
    let diagnostics = diagnostics(&[
        (
            "a.cln",
            "import:\n\tb\n\nfunctions:\n\tpublic:\n\t\tinteger fromA()\n\t\t\treturn 1\n",
        ),
        (
            "b.cln",
            "import:\n\ta\n\nfunctions:\n\tvoid init()\n\t\treturn\n",
        ),
        (
            "c.cln",
            "import:\n\td\n\nfunctions:\n\tinteger fromC()\n\t\treturn 3\n",
        ),
        (
            "d.cln",
            "import:\n\tc\n\nfunctions:\n\tinteger fromD()\n\t\treturn 4\n",
        ),
    ]);
    let cycles: Vec<&clean_compiler_types::Diagnostic> = diagnostics
        .iter()
        .filter(|d| d.code == "IMPORT001")
        .collect();
    assert_eq!(
        cycles.len(),
        2,
        "two independent cycles, two diagnostics: {diagnostics:#?}"
    );
    assert!(cycles
        .iter()
        .any(|d| d.message == "import cycle detected: a → b → a"));
    assert!(cycles
        .iter()
        .any(|d| d.message == "import cycle detected: c → d → c"));
}

#[test]
fn same_private_name_in_two_modules_is_not_a_redefinition() {
    // SEM003 is module-scoped now: private helpers may share a name.
    typechecks(&[
        (
            "a.cln",
            "functions:\n\tinteger helper()\n\t\treturn 1\n\n\tpublic:\n\t\tinteger fromA()\n\t\t\treturn helper()\n",
        ),
        (
            "b.cln",
            "functions:\n\tinteger helper()\n\t\treturn 2\n\n\tvoid init()\n\t\tinteger x = helper()\n\t\treturn\n",
        ),
    ]);
}

#[test]
fn file_path_import_resolves_relative_to_importer() {
    typechecks(&[
        (
            "app/data/models.cln",
            "functions:\n\tpublic:\n\t\tinteger double(integer x)\n\t\t\treturn x * 2\n",
        ),
        (
            "app/main.cln",
            "import \"data/models.cln\"\n\nfunctions:\n\tvoid init()\n\t\tinteger d = double(21)\n\t\treturn\n",
        ),
    ]);
}

/// MOD-06 fix-and-pin (ratified 2026-08-22): visibility does not chain
/// through a middle module, in either `sources[]` order. Phase 4's scope
/// extension used to read the target module's already-extended scope, so
/// `a → b → c` leaked c's exports into a exactly when the `b → c` edge
/// happened to be processed first — an order-dependent surface.
#[test]
fn import_visibility_does_not_chain_through_a_middle_module() {
    let c = (
        "c.cln",
        "functions:\n\tpublic:\n\t\tinteger fromC()\n\t\t\treturn 3\n",
    );
    let b = (
        "b.cln",
        "import:\n\tc\n\nfunctions:\n\tpublic:\n\t\tinteger fromB()\n\t\t\treturn fromC()\n",
    );
    let a = (
        "a.cln",
        "import:\n\tb\n\nfunctions:\n\tvoid init()\n\t\tinteger x = fromC()\n\t\treturn\n",
    );
    // [c, b, a] processes the b→c edge first (the leaking order);
    // [a, b, c] processes a→b first (never leaked). Both must reject.
    for order in [[c, b, a], [a, b, c]] {
        let diagnostics = diagnostics(&order);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == "SEM019" && d.primary_span.file == "a.cln"),
            "what b imported must not reach a (order {:?}): {diagnostics:#?}",
            order.map(|(p, _)| p)
        );
    }
}

/// MOD-06 (ratified 2026-08-22): when two imports would bind the same
/// name, the first import in source order wins, **silently** — the
/// shadowing warning is a recorded possible future, not current surface.
#[test]
fn first_import_wins_silently_on_name_collision() {
    let one = (
        "one.cln",
        "functions:\n\tpublic:\n\t\tinteger pick(integer a)\n\t\t\treturn a\n",
    );
    let two = (
        "two.cln",
        "functions:\n\tpublic:\n\t\tinteger pick(integer a, integer b)\n\t\t\treturn a + b\n",
    );
    // `one` first: the unary signature is bound and the check is silent.
    let main_one_first = (
        "main.cln",
        "import:\n\tone\n\ttwo\n\nfunctions:\n\tvoid init()\n\t\tinteger s = pick(5)\n\t\treturn\n",
    );
    match clean_compiler::check(request_for(&[one, two, main_one_first])) {
        Ok(diagnostics) => assert!(
            diagnostics.is_empty(),
            "the collision is silent: {diagnostics:#?}"
        ),
        Err(err) => panic!("expected a silent clean check, got {err:?}"),
    }
    // `two` first: the same call now misses the binary arity — order
    // decides which import won.
    let main_two_first = (
        "main.cln",
        "import:\n\ttwo\n\tone\n\nfunctions:\n\tvoid init()\n\t\tinteger s = pick(5)\n\t\treturn\n",
    );
    let rejected = diagnostics(&[one, two, main_two_first]);
    assert!(
        rejected.iter().any(|d| d.code == "FUNC002"),
        "the first import's signature is the bound one: {rejected:#?}"
    );
}

/// MOD-04 ladder (ratified 2026-08-22), legs 1–3: the path relative to
/// the importer beats the request-root path, which beats a unique
/// `…/name.cln` suffix match.
#[test]
fn module_resolution_prefers_relative_then_root_then_unique_suffix() {
    let near = (
        "app/utils.cln",
        "functions:\n\tpublic:\n\t\tinteger near(integer a)\n\t\t\treturn a\n",
    );
    let far = (
        "utils.cln",
        "functions:\n\tpublic:\n\t\tinteger far(integer a)\n\t\t\treturn a\n",
    );
    let deep = (
        "lib/nested/utils.cln",
        "functions:\n\tpublic:\n\t\tinteger deep(integer a)\n\t\t\treturn a\n",
    );
    // Leg 1: relative wins even with a root candidate present.
    typechecks(&[
        (
            "app/main.cln",
            "import:\n\tutils\n\nfunctions:\n\tvoid init()\n\t\tinteger s = near(1)\n\t\treturn\n",
        ),
        near,
        far,
    ]);
    // Leg 2: the request-root path when no relative match exists.
    typechecks(&[
        (
            "app/main.cln",
            "import:\n\tutils\n\nfunctions:\n\tvoid init()\n\t\tinteger s = far(1)\n\t\treturn\n",
        ),
        far,
    ]);
    // Leg 3: a unique suffix match anywhere in the set.
    typechecks(&[
        (
            "app/main.cln",
            "import:\n\tutils\n\nfunctions:\n\tvoid init()\n\t\tinteger s = deep(1)\n\t\treturn\n",
        ),
        deep,
    ]);
}

/// MOD-04 (ratified 2026-08-22): an ambiguous suffix match resolves to
/// nothing — IMPORT002, never an arbitrary pick.
#[test]
fn ambiguous_suffix_match_resolves_to_nothing() {
    let rejected = diagnostics(&[
        (
            "app/main.cln",
            "import:\n\tutils\n\nfunctions:\n\tvoid init()\n\t\treturn\n",
        ),
        (
            "x/utils.cln",
            "functions:\n\tpublic:\n\t\tinteger fx(integer a)\n\t\t\treturn a\n",
        ),
        (
            "y/utils.cln",
            "functions:\n\tpublic:\n\t\tinteger fy(integer a)\n\t\t\treturn a\n",
        ),
    ]);
    assert!(
        rejected.iter().any(|d| d.code == "IMPORT002"),
        "ambiguity must not pick a winner: {rejected:#?}"
    );
}
