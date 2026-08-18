//! Milestone 6 stage 3 — `number` (IEEE-754 binary64) end to end:
//! literals, arithmetic, comparisons, TYP-06 widening, and the f64 host
//! boundary, verified under wasmtime.

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

\thost function emitNum(value: number)
\t\tdescription \"Record one number.\"

\thost function emitBool(value: boolean)
\t\tdescription \"Record one boolean.\"

\thost function seed() returns number
\t\tdescription \"A number produced by the host.\"
";

enum Logged {
    Num(f64),
    Bool(i32),
}

fn run(body: &str) -> Vec<Logged> {
    let main = format!("functions:\n\tvoid init()\n{body}");
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    wasmparser::validate(&wasm).expect("core module validates");

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Vec<Logged>> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-num",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, value: f64| {
                caller.data_mut().push(Logged::Num(value));
            },
        )
        .expect("emit-num links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-bool",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, value: i32| {
                caller.data_mut().push(Logged::Bool(value));
            },
        )
        .expect("emit-bool links");
    linker
        .func_wrap("clean:host/probe@0.1.0", "seed", || 6.25f64)
        .expect("seed links");
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

fn nums(log: &[Logged]) -> Vec<f64> {
    log.iter()
        .map(|l| match l {
            Logged::Num(v) => *v,
            Logged::Bool(_) => panic!("expected number"),
        })
        .collect()
}

fn bools(log: &[Logged]) -> Vec<i32> {
    log.iter()
        .map(|l| match l {
            Logged::Bool(v) => *v,
            Logged::Num(_) => panic!("expected boolean"),
        })
        .collect()
}

#[test]
fn arithmetic_in_the_f64_domain() {
    let out = run("\
\t\temitNum(1.5 + 2.25)
\t\temitNum(10.0 - 0.5)
\t\temitNum(2.5 * 4.0)
\t\temitNum(7.5 / 2.0)
\t\temitNum(-1.5)
");
    assert_eq!(nums(&out), [3.75, 9.5, 10.0, 3.75, -1.5]);
}

#[test]
fn float_remainder_is_truncated_fmod() {
    let out = run("\
\t\temitNum(7.5 % 2.0)
\t\temitNum(-7.5 % 2.0)
\t\temitNum(1.25 % 0.5)
");
    assert_eq!(nums(&out), [1.5, -1.5, 0.25]);
}

#[test]
fn comparisons_produce_booleans() {
    let out = run("\
\t\temitBool(1.5 < 2.0)
\t\temitBool(2.0 <= 2.0)
\t\temitBool(1.5 > 2.0)
\t\temitBool(2.0 >= 2.5)
\t\temitBool(0.5 == 0.5)
\t\temitBool(0.5 != 0.5)
");
    assert_eq!(bools(&out), [1, 1, 0, 0, 1, 0]);
}

#[test]
fn integer_widens_into_the_float_domain() {
    let out = run("\
\t\tnumber x = 3
\t\tinteger n = 2
\t\temitNum(x + 0.5)
\t\temitNum(n + 0.25)
");
    assert_eq!(nums(&out), [3.5, 2.25]);
}

#[test]
fn numbers_cross_the_boundary_both_ways() {
    let out = run("\
\t\tnumber s = seed()
\t\temitNum(s / 2.0)
");
    assert_eq!(nums(&out), [3.125]);
}

#[test]
fn number_locals_flow_through_loops() {
    let out = run("\
\t\tnumber acc = 0.0
\t\titerate i in 1 to 4
\t\t\tacc = acc + 0.25
\t\temitNum(acc)
");
    assert_eq!(nums(&out), [1.0]);
}
