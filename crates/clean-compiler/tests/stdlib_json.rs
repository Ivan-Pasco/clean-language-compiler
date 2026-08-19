//! Milestone 6 stage 9 — the chapter-15 JSON module over the ADR-0005
//! `any` box: pure guest parsing/serialization, chapter-15 access
//! semantics, and the RUN006–RUN010 accept/reject boundary, verified
//! under wasmtime.

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

fn try_run(body: &str) -> Result<Vec<Logged>, wasmtime::Error> {
    let main = format!("functions:\n\tvoid init()\n{body}");
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    wasmparser::validate(&wasm).expect("core module validates");
    let (mut store, instance) = instantiate(&wasm);
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())?;
    Ok(store.into_data())
}

fn run(body: &str) -> Vec<Logged> {
    try_run(body).expect("program runs")
}

/// Renders raw text as a Clean string literal: quotes and backslashes
/// escape, and `{`/`}` escape because Clean strings interpolate.
fn clean_str(raw: &str) -> String {
    let mut out = String::from("\"");
    for c in raw.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[test]
fn parse_scalars_and_unbox() {
    let body = format!(
        "\t\tinteger n = json.textToData({})\n\
         \t\temitInt(n)\n\
         \t\tnumber x = json.textToData({})\n\
         \t\temitNum(x)\n\
         \t\tboolean b = json.textToData({})\n\
         \t\temitBool(b)\n\
         \t\tstring s = json.textToData({})\n\
         \t\temitText(s)\n\
         \t\temitBool(json.textToData({}) is none)\n\
         \t\temitNum(json.textToData({}))\n",
        clean_str("42"),
        clean_str("2.5"),
        clean_str("true"),
        clean_str(r#""hola""#),
        clean_str("null"),
        clean_str("-0"),
    );
    assert_eq!(
        run(&body),
        [
            Logged::Int(42),
            Logged::Num(2.5),
            Logged::Bool(1),
            Logged::Text("hola".into()),
            Logged::Bool(1),
            Logged::Num(-0.0),
        ]
    );
}

#[test]
fn object_and_array_access() {
    let doc = clean_str(r#"{"name": "ana", "age": 33, "tags": ["a", "b"]}"#);
    let body = format!(
        "\t\tany data = json.textToData({doc})\n\
         \t\tstring name = data.name\n\
         \t\temitText(name)\n\
         \t\tinteger age = data[\"age\"]\n\
         \t\temitInt(age)\n\
         \t\tstring first = data.tags[0]\n\
         \t\temitText(first)\n\
         \t\tstring second = data[\"tags\"][1]\n\
         \t\temitText(second)\n\
         \t\temitBool(data.missing is none)\n\
         \t\temitBool(data.tags[9] is none)\n"
    );
    assert_eq!(
        run(&body),
        [
            Logged::Text("ana".into()),
            Logged::Int(33),
            Logged::Text("a".into()),
            Logged::Text("b".into()),
            Logged::Bool(1),
            Logged::Bool(1),
        ]
    );
}

#[test]
fn string_escapes_decode() {
    let doc = clean_str(r#""a\n\t\\ é 🎉""#);
    let body = format!("\t\tstring s = json.textToData({doc})\n\t\temitText(s)\n");
    assert_eq!(run(&body), [Logged::Text("a\n\t\\ é 🎉".into())]);
}

#[test]
fn try_parse_returns_none_where_parse_traps() {
    for bad in [
        r#"{"a": 1,}"#,
        "[1, 2",
        "01",
        ".5",
        "5.",
        "1e",
        "1e999",
        r#"{"a": 1, "a": 2}"#,
        r#""unterminated"#,
        "tru",
        "",
        "[1] extra",
    ] {
        let lit = clean_str(bad);
        // tryTextToData → none, no trap.
        let out = run(&format!(
            "\t\temitBool(json.tryTextToData({lit}) is none)\n"
        ));
        assert_eq!(out, [Logged::Bool(1)], "tryTextToData({bad})");
        // textToData → trap (RUN006–RUN010 family).
        let err = try_run(&format!("\t\tany d = json.textToData({lit})\n"))
            .expect_err(&format!("textToData({bad}) must trap"));
        assert!(err.downcast_ref::<wasmtime::Trap>().is_some());
    }
}

#[test]
fn depth_limit_is_exactly_1000() {
    let ok = format!("{}1{}", "[".repeat(999), "]".repeat(999));
    let lit = clean_str(&ok);
    assert_eq!(
        run(&format!(
            "\t\temitBool(json.tryTextToData({lit}) is none)\n"
        )),
        [Logged::Bool(0)],
        "999 levels parse"
    );
    let too_deep = format!("{}1{}", "[".repeat(1001), "]".repeat(1001));
    let lit = clean_str(&too_deep);
    assert_eq!(
        run(&format!(
            "\t\temitBool(json.tryTextToData({lit}) is none)\n"
        )),
        [Logged::Bool(1)],
        "1001 levels reject"
    );
}

#[test]
fn serialize_round_trips_source_text() {
    let doc = clean_str(r#"{"a": [1, 2.5, -0.75e2, true, null], "s": "x"}"#);
    let body = format!(
        "\t\tany data = json.textToData({doc})\n\
         \t\temitText(json.dataToText(data))\n"
    );
    assert_eq!(
        run(&body),
        [Logged::Text(
            r#"{"a":[1,2.5,-0.75e2,true,null],"s":"x"}"#.into()
        )],
        "numbers re-emit their source text verbatim (ADR 0005)"
    );
}

#[test]
fn serialize_constructed_values() {
    let body = format!(
        "\t\temitText(json.dataToText({}))\n\
         \t\temitText(json.dataToText(42))\n\
         \t\temitText(json.dataToText(true))\n",
        clean_str("hola"),
    );
    assert_eq!(
        run(&body),
        [
            Logged::Text("\"hola\"".into()),
            Logged::Text("42".into()),
            Logged::Text("true".into()),
        ]
    );
}

#[test]
fn parse_serialize_parse_is_stable() {
    // The round-trip property (quality playbook §1.9 layer 2), spot-form:
    // parse ∘ serialize ∘ parse equals the first parse, observed through
    // re-serialization.
    let doc = clean_str(r#"{"k": [1.5e-3, "v", false]}"#);
    let body = format!(
        "\t\tany one = json.textToData({doc})\n\
         \t\tstring s1 = json.dataToText(one)\n\
         \t\tany two = json.textToData(s1)\n\
         \t\tstring s2 = json.dataToText(two)\n\
         \t\temitBool(s1 == s2)\n\
         \t\temitText(s2)\n"
    );
    assert_eq!(
        run(&body),
        [
            Logged::Bool(1),
            Logged::Text(r#"{"k":[1.5e-3,"v",false]}"#.into())
        ]
    );
}
