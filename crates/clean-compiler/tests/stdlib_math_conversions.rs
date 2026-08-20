//! Milestone 6 stage 5a — the chapter-15 `math` module (wasm-native
//! subset + constants) and 15 §Conversions, verified under wasmtime.

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
        &validated.world,
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

\thost function emitNum(value: number)
\t\tdescription \"Record one number.\"

\thost function emitBool(value: boolean)
\t\tdescription \"Record one boolean.\"

\thost function emitText(value: string)
\t\tdescription \"Record one string.\"
";

#[derive(Debug, PartialEq)]
enum Logged {
    Int(i64),
    Num(f64),
    Bool(i32),
    Text(String),
}

fn instantiate(wasm: &[u8]) -> (wasmtime::Store<Vec<Logged>>, wasmtime::Instance) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Vec<Logged>> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-int",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, value: i64| {
                caller.data_mut().push(Logged::Int(value));
            },
        )
        .expect("links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-num",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, value: f64| {
                caller.data_mut().push(Logged::Num(value));
            },
        )
        .expect("links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-bool",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, value: i32| {
                caller.data_mut().push(Logged::Bool(value));
            },
        )
        .expect("links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-text",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, ptr: i32, len: i32| {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("memory export");
                let mut buf = vec![0u8; len as usize];
                memory.read(&caller, ptr as usize, &mut buf).expect("read");
                caller
                    .data_mut()
                    .push(Logged::Text(String::from_utf8(buf).expect("UTF-8")));
            },
        )
        .expect("links");
    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    (store, instance)
}

fn run(body: &str) -> Vec<Logged> {
    let main = format!("functions:\n\tvoid init()\n{body}");
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    wasmparser::validate(&wasm).expect("core module validates");
    let (mut store, instance) = instantiate(&wasm);
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect("init runs");
    store.into_data()
}

fn run_expecting_trap(body: &str) {
    let main = format!("functions:\n\tvoid init()\n{body}");
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    let (mut store, instance) = instantiate(&wasm);
    let err = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect_err("this program must trap");
    assert!(
        err.downcast_ref::<wasmtime::Trap>().is_some(),
        "expected a trap: {err:?}"
    );
}

#[test]
fn math_native_subset() {
    let out = run("\
\t\temitNum(math.sqrt(9.0))
\t\temitNum(math.absNumber(-2.5))
\t\temitInt(math.absInteger(-7))
\t\temitNum(math.max(1.5, 2.5))
\t\temitNum(math.min(1.5, 2.5))
\t\temitNum(math.floor(2.7))
\t\temitNum(math.ceil(2.2))
\t\temitNum(math.trunc(-2.7))
\t\temitNum(math.round(2.5))
\t\temitNum(math.sign(-3.5))
\t\temitNum(math.sign(0.0))
\t\temitNum(math.sign(9.9))
");
    assert_eq!(
        out,
        [
            Logged::Num(3.0),
            Logged::Num(2.5),
            Logged::Int(7),
            Logged::Num(2.5),
            Logged::Num(1.5),
            Logged::Num(2.0),
            Logged::Num(3.0),
            Logged::Num(-2.0),
            // wasm f64.nearest: ties to even (DISCOVERIES-M6 flags the
            // rounding-mode question).
            Logged::Num(2.0),
            Logged::Num(-1.0),
            Logged::Num(0.0),
            Logged::Num(1.0),
        ]
    );
}

#[test]
fn math_constants_without_parentheses() {
    let out = run("\
\t\temitNum(math.pi)
\t\temitNum(math.e)
\t\temitNum(math.tau)
");
    assert_eq!(
        out,
        [
            Logged::Num(std::f64::consts::PI),
            Logged::Num(std::f64::consts::E),
            Logged::Num(std::f64::consts::TAU),
        ]
    );
}

#[test]
fn unknown_math_name_is_sem019() {
    // 15 §Math: basic arithmetic is operators only — math.add MUST NOT
    // exist.
    let main = "functions:\n\tvoid init()\n\t\temitNum(math.add(1.0, 2.0))\n".to_string();
    let request = {
        let mut request = common::minimal_valid_request();
        request.sources = vec![
            clean_compiler_types::request::SourceFile {
                path: "app/host_bridge.cln".into(),
                sha256: common::sha256_hex(PROBE.as_bytes()),
                content: PROBE.into(),
            },
            clean_compiler_types::request::SourceFile {
                path: "app/main.cln".into(),
                sha256: common::sha256_hex(main.as_bytes()),
                content: main.clone(),
            },
        ];
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
    let _ = typecheck::check(&resolved, &validated.world, &mut sink);
    let diagnostics = sink.into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "SEM019" && d.message.contains("`add`")),
        "expected SEM019 for math.add, got: {diagnostics:#?}"
    );
}

#[test]
fn conversions_between_scalars() {
    let out = run("\
\t\temitInt(2.9.toInteger())
\t\temitInt((-2.9).toInteger())
\t\temitNum(3.toNumber())
\t\temitBool(0.toBoolean())
\t\temitBool(7.toBoolean())
\t\temitBool(0.0.toBoolean())
\t\temitBool(0.5.toBoolean())
");
    assert_eq!(
        out,
        [
            Logged::Int(2),
            Logged::Int(-2),
            Logged::Num(3.0),
            Logged::Bool(0),
            Logged::Bool(1),
            Logged::Bool(0),
            Logged::Bool(1),
        ]
    );
}

/// bbdf483: the 15 §Conversions table is exhaustive — an unlisted
/// (source, conversion) pair is SEM022, never accepted.
#[test]
fn unlisted_conversion_pairs_are_sem022() {
    for body in [
        "\t\temitBool(\"yes\".toBoolean())\n",
        "\t\temitInt(true.toInteger())\n",
        "\t\temitNum(false.toNumber())\n",
    ] {
        let main = format!("functions:\n\tvoid init()\n{body}");
        let request = {
            let mut request = common::minimal_valid_request();
            request.sources = vec![
                clean_compiler_types::request::SourceFile {
                    path: "app/host_bridge.cln".into(),
                    sha256: common::sha256_hex(PROBE.as_bytes()),
                    content: PROBE.into(),
                },
                clean_compiler_types::request::SourceFile {
                    path: "app/main.cln".into(),
                    sha256: common::sha256_hex(main.as_bytes()),
                    content: main.clone(),
                },
            ];
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
        let _ = typecheck::check(&resolved, &validated.world, &mut sink);
        let diagnostics = sink.into_diagnostics();
        assert!(
            diagnostics.iter().any(|d| d.code == "SEM022"),
            "expected SEM022 for {body:?}, got: {diagnostics:#?}"
        );
    }
}

#[test]
fn to_string_renders_literals() {
    let out = run("\
\t\temitText(0.toString())
\t\temitText(42.toString())
\t\temitText((-42).toString())
\t\temitText(9223372036854775807.toString())
\t\temitText(true.toString())
\t\temitText(false.toString())
\t\temitText(\"as-is\".toString())
");
    assert_eq!(
        out,
        [
            Logged::Text("0".into()),
            Logged::Text("42".into()),
            Logged::Text("-42".into()),
            Logged::Text("9223372036854775807".into()),
            Logged::Text("true".into()),
            Logged::Text("false".into()),
            Logged::Text("as-is".into()),
        ]
    );
}

#[test]
fn string_parses_round_trip() {
    let out = run("\
\t\temitInt(\"123\".toInteger())
\t\temitInt(\"-9223372036854775808\".toInteger())
\t\temitInt(\"+7\".toInteger())
\t\temitNum(\"2.5\".toNumber())
\t\temitNum(\"-0.25\".toNumber())
\t\temitNum(\"40\".toNumber())
");
    assert_eq!(
        out,
        [
            Logged::Int(123),
            Logged::Int(i64::MIN),
            Logged::Int(7),
            Logged::Num(2.5),
            Logged::Num(-0.25),
            Logged::Num(40.0),
        ]
    );
}

#[test]
fn trap_on_trailing_garbage() {
    run_expecting_trap("\t\temitInt(\"12x\".toInteger())\n");
}

#[test]
fn trap_on_empty_string() {
    run_expecting_trap("\t\temitInt(\"\".toInteger())\n");
}

#[test]
fn trap_on_bare_sign() {
    run_expecting_trap("\t\temitInt(\"-\".toInteger())\n");
}

#[test]
fn trap_on_integer_overflow() {
    // One past i64::MAX overflows.
    run_expecting_trap("\t\temitInt(\"9223372036854775808\".toInteger())\n");
}

#[test]
fn trap_on_trailing_dot() {
    run_expecting_trap("\t\temitNum(\"2.\".toNumber())\n");
}

#[test]
fn trap_on_double_dot() {
    run_expecting_trap("\t\temitNum(\"1.2.3\".toNumber())\n");
}

/// bbdf483: toNumber is correctly rounded (roundTiesToEven). The classic
/// hazards of naive accumulation: 0.1, long fractions, and mantissas
/// past 2^53.
#[test]
fn to_number_is_correctly_rounded() {
    let out = run("\
\t\temitNum(\"0.1\".toNumber())
\t\temitNum(\"0.3\".toNumber())
\t\temitNum(\"123.456\".toNumber())
\t\temitNum(\"0.000001\".toNumber())
\t\temitNum(\"9007199254740993\".toNumber())
\t\temitNum(\"1234567890.12345\".toNumber())
");
    assert_eq!(
        out,
        [
            Logged::Num(0.1),
            Logged::Num(0.3),
            Logged::Num(123.456),
            Logged::Num(0.000001),
            Logged::Num(9007199254740993i64 as f64),
            Logged::Num(1234567890.12345),
        ]
    );
}

/// The literal grammar's exponent form (LEX §NumberLiteral): toNumber
/// accepts `digits e digits`, the dotted form, a signed exponent, and the
/// leading-dot form; overflow saturates to infinity, underflow to zero.
#[test]
fn string_to_number_parses_exponents() {
    let log = run("\t\temitNum(\"6.02e23\".toNumber())
\t\temitNum(\"6e23\".toNumber())
\t\temitNum(\"1.5E2\".toNumber())
\t\temitNum(\"2e-3\".toNumber())
\t\temitNum(\"2e+3\".toNumber())
\t\temitNum(\".5e2\".toNumber())
\t\temitNum(0.0 - \"1.25e1\".toNumber())
\t\temitNum(\"1e999\".toNumber())
\t\temitNum(\"1e-999\".toNumber())
");
    assert_eq!(
        log,
        [
            Logged::Num(6.02e23),
            Logged::Num(6e23),
            Logged::Num(150.0),
            Logged::Num(0.002),
            Logged::Num(2000.0),
            Logged::Num(50.0),
            Logged::Num(-12.5),
            Logged::Num(f64::INFINITY),
            Logged::Num(0.0),
        ]
    );
}

/// Malformed exponents raise RUN003 — bare `e`, digitless tails, and the
/// LEX-06 "digit required after the dot" rule combined with an exponent.
#[test]
fn malformed_exponents_raise_run003() {
    for bad in ["1e", "1e+", "1e-", "3.e2", "1e2x", "e5"] {
        run_expecting_trap(&format!("\t\temitNum(\"{bad}\".toNumber())\n"));
    }
}
