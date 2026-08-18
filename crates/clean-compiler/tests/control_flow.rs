//! Milestone 6 stage 2 — FLW-02/FLW-03 loop lowering, verified under
//! wasmtime. Includes the two regression shapes the retired compiler
//! silently miscompiled (KNOWLEDGE §13): a statement after a nested
//! if/else inside a loop, and `else: break` inside `while`.

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{codegen, hir, mir, parser, resolver, typecheck};

mod common;

fn compile(sources: &[(&str, &str)]) -> Vec<u8> {
    let request = {
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
    };
    let mut sink = DiagnosticSink::new();
    let validated =
        clean_compiler::request::validate(request, &mut sink).expect("request validates");
    let files = validated
        .request
        .sources
        .iter()
        .map(|s| {
            let stream = clean_compiler::lexer::lex(&s.path, &s.content, &mut sink);
            let ast = parser::parse(&stream, &mut sink);
            resolver::ParsedFile { ast, stream }
        })
        .collect();
    let resolved = resolver::resolve(files, &[], &mut sink);
    let typed = typecheck::check(&resolved, &validated.world, &mut sink);
    assert!(
        !sink.has_errors(),
        "unexpected diagnostics: {:#?}",
        sink.into_diagnostics()
    );
    let hir = hir::lower(typed);
    let mir = mir::lower(
        &hir,
        &resolved,
        &validated.world.package_version(),
        clean_compiler::layout::tier("standard").expect("standard tier exists"),
        &mut sink,
    );
    assert!(
        sink.unsupported().is_empty(),
        "unexpected unsupported constructs: {:#?}",
        sink.unsupported()
    );
    codegen::core::emit_core(&mir).expect("static data fits below the heap start")
}

const PROBE: &str = "\
host interface probe version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function emitInt(value: integer)
\t\tdescription \"Record one integer.\"
";

/// Compiles `body` (the indented statements of `void init()`), runs it,
/// and returns every integer the guest emitted.
fn run(body: &str) -> Vec<i64> {
    let main = format!("functions:\n\tvoid init()\n{body}");
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    wasmparser::validate(&wasm).expect("core module validates");

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Vec<i64>> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-int",
            |mut caller: wasmtime::Caller<'_, Vec<i64>>, value: i64| {
                caller.data_mut().push(value);
            },
        )
        .expect("emit-int links");
    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect("init runs");
    store.into_data()
}

#[test]
fn while_loop_counts_and_terminates() {
    let out = run("\
\t\tinteger i = 1
\t\twhile i <= 4
\t\t\temitInt(i)
\t\t\ti = i + 1
");
    assert_eq!(out, [1, 2, 3, 4]);
}

/// KNOWLEDGE §13.1 — the retired compiler dropped the statement after a
/// nested if/else, so the loop advance never ran and the loop hung. The
/// advance after the two sibling if/else nests must execute every pass.
#[test]
fn statement_after_nested_if_else_inside_while_still_runs() {
    let out = run("\
\t\tinteger i = 0
\t\twhile i < 3
\t\t\tif i == 0
\t\t\t\temitInt(100)
\t\t\telse
\t\t\t\temitInt(200)
\t\t\tif i == 2
\t\t\t\temitInt(300)
\t\t\telse
\t\t\t\temitInt(400)
\t\t\ti = i + 1
");
    assert_eq!(out, [100, 400, 200, 400, 200, 300]);
}

/// KNOWLEDGE §13.2 — `else: break` was mistaken for "no else clause" and
/// the loop ran forever.
#[test]
fn else_break_terminates_the_loop() {
    let out = run("\
\t\tinteger i = 0
\t\twhile true
\t\t\tif i < 3
\t\t\t\temitInt(i)
\t\t\telse
\t\t\t\tbreak
\t\t\ti = i + 1
");
    assert_eq!(out, [0, 1, 2]);
}

#[test]
fn iterate_ascending_is_inclusive() {
    let out = run("\
\t\titerate i in 1 to 5
\t\t\temitInt(i)
");
    assert_eq!(out, [1, 2, 3, 4, 5]);
}

/// Local adoption pinned here (DISCOVERIES-M6): a range with `from > to`
/// and no explicit step descends, mirroring `list.range(5, 1)`.
#[test]
fn iterate_descending_without_step_mirrors_list_range() {
    let out = run("\
\t\titerate i in 5 to 1
\t\t\temitInt(i)
");
    assert_eq!(out, [5, 4, 3, 2, 1]);
}

/// The chapter-12 worked example: `iterate k in 10 to 1 step -2`.
#[test]
fn iterate_negative_step_matches_the_spec_example() {
    let out = run("\
\t\titerate k in 10 to 1 step -2
\t\t\temitInt(k)
");
    assert_eq!(out, [10, 8, 6, 4, 2]);
}

#[test]
fn iterate_step_reaches_the_inclusive_endpoint() {
    let out = run("\
\t\titerate idx in 0 to 100 step 5
\t\t\temitInt(idx)
");
    let expected: Vec<i64> = (0..=100).step_by(5).map(|v| v as i64).collect();
    assert_eq!(out, expected);
}

#[test]
fn iterate_step_against_the_direction_runs_zero_iterations() {
    let out = run("\
\t\titerate i in 1 to 10 step -1
\t\t\temitInt(i)
\t\temitInt(99)
");
    assert_eq!(out, [99]);
}

/// FLW-03: a skipped iteration consumes its item — `continue` still
/// applies the step.
#[test]
fn continue_applies_the_step() {
    let out = run("\
\t\titerate i in 1 to 5
\t\t\tif i == 3
\t\t\t\tcontinue
\t\t\temitInt(i)
");
    assert_eq!(out, [1, 2, 4, 5]);
}

/// FLW-03: both statements bind to the innermost enclosing loop.
#[test]
fn break_and_continue_bind_to_the_innermost_loop() {
    let out = run("\
\t\titerate i in 1 to 3
\t\t\titerate j in 1 to 3
\t\t\t\tif j == 2
\t\t\t\t\tbreak
\t\t\t\temitInt(i * 10 + j)
\t\t\temitInt(i)
");
    assert_eq!(out, [11, 1, 21, 2, 31, 3]);
}

#[test]
fn continue_in_while_retests_the_condition() {
    let out = run("\
\t\tinteger i = 0
\t\twhile i < 5
\t\t\ti = i + 1
\t\t\tif i == 2
\t\t\t\tcontinue
\t\t\temitInt(i)
");
    assert_eq!(out, [1, 3, 4, 5]);
}
