//! Milestone 6 stage 6 — first-class `bytes` (§14.14.2 with chapter-15
//! naming): operators, indexing, length, slice, fromText/toText with full
//! RFC 3629 validation. `b"..."` literals do not exist in the EBNF
//! (DISCOVERIES-M6), so bytes values originate from `bytes.fromText` and
//! host returns.

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

/// `raw()` returns host-provided bytes — including invalid UTF-8 — so
/// `toText`'s validator sees data no Clean string could carry.
const PROBE: &str = "\
host interface probe version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function emitInt(value: integer)
\t\tdescription \"Record one integer.\"

\thost function emitBool(value: boolean)
\t\tdescription \"Record one boolean.\"

\thost function emitText(value: string)
\t\tdescription \"Record one string.\"

\thost function emitData(value: bytes)
\t\tdescription \"Record raw bytes.\"

\thost function raw(which: integer) returns bytes
\t\tdescription \"Host-chosen byte payloads by index.\"
";

#[derive(Debug, PartialEq)]
enum Logged {
    Int(i64),
    Bool(i32),
    Text(String),
    Data(Vec<u8>),
}

fn raw_payload(which: i64) -> Vec<u8> {
    match which {
        0 => b"caf\xC3\xA9".to_vec(),      // valid UTF-8 "café"
        1 => vec![0xFF, 0x41],             // invalid lead
        2 => vec![0xC3],                   // truncated sequence
        3 => vec![0xED, 0xA0, 0x80],       // surrogate D800
        4 => vec![0xC0, 0xAF],             // overlong '/'
        5 => vec![0xF4, 0x90, 0x80, 0x80], // beyond U+10FFFF
        _ => Vec::new(),
    }
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
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-data",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, ptr: i32, len: i32| {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("memory export");
                let mut buf = vec![0u8; len as usize];
                memory.read(&caller, ptr as usize, &mut buf).expect("read");
                caller.data_mut().push(Logged::Data(buf));
            },
        )
        .expect("links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "raw",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, which: i64, retptr: i32| {
                let payload = raw_payload(which);
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("memory export");
                let realloc = caller
                    .get_export("cabi_realloc")
                    .and_then(|e| e.into_func())
                    .expect("cabi_realloc export")
                    .typed::<(i32, i32, i32, i32), i32>(&caller)
                    .expect("realloc type");
                let dst = realloc
                    .call(&mut caller, (0, 0, 1, payload.len() as i32))
                    .expect("realloc runs");
                memory
                    .write(&mut caller, dst as usize, &payload)
                    .expect("write payload");
                let head = [
                    (dst as u32).to_le_bytes(),
                    (payload.len() as u32).to_le_bytes(),
                ]
                .concat();
                memory
                    .write(&mut caller, retptr as usize, &head)
                    .expect("write retptr");
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

#[test]
fn from_text_length_index_and_operators() {
    let out = run("\
\t\tbytes b = bytes.fromText(\"café\")
\t\temitInt(b.length)
\t\temitInt(b[0])
\t\temitInt(b[3])
\t\temitBool(b == bytes.fromText(\"café\"))
\t\temitBool(b == bytes.fromText(\"cafe\"))
\t\temitBool(b != bytes.fromText(\"cafe\"))
\t\temitData(bytes.fromText(\"ab\") + bytes.fromText(\"cd\"))
");
    assert_eq!(
        out,
        [
            // "café" is 5 bytes (é = C3 A9); b[3] is the C3 lead byte.
            Logged::Int(5),
            Logged::Int(0x63),
            Logged::Int(0xC3),
            Logged::Bool(1),
            Logged::Bool(0),
            Logged::Bool(1),
            Logged::Data(b"abcd".to_vec()),
        ]
    );
}

#[test]
fn index_out_of_range_traps() {
    let main =
        "functions:\n\tvoid init()\n\t\tbytes b = bytes.fromText(\"ab\")\n\t\temitInt(b[2])\n"
            .to_string();
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    let (mut store, instance) = instantiate(&wasm);
    let err = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect_err("b[2] on 2 bytes must trap");
    assert!(err.downcast_ref::<wasmtime::Trap>().is_some());
}

#[test]
fn slice_clamps_bytes() {
    let out = run("\
\t\tbytes b = bytes.fromText(\"hello\")
\t\temitData(b.slice(1, 3))
\t\temitData(b.slice(3, 99))
\t\temitData(b.slice(3, 2))
");
    assert_eq!(
        out,
        [
            Logged::Data(b"el".to_vec()),
            Logged::Data(b"lo".to_vec()),
            Logged::Data(Vec::new()),
        ]
    );
}

#[test]
fn to_text_validates_utf8() {
    let out = run("\
\t\temitText(bytes.toText(raw(0)) default \"invalid\")
\t\temitText(bytes.toText(raw(1)) default \"invalid\")
\t\temitText(bytes.toText(raw(2)) default \"invalid\")
\t\temitText(bytes.toText(raw(3)) default \"invalid\")
\t\temitText(bytes.toText(raw(4)) default \"invalid\")
\t\temitText(bytes.toText(raw(5)) default \"invalid\")
\t\temitText(bytes.toText(raw(9)) default \"invalid\")
");
    assert_eq!(
        out,
        [
            Logged::Text("café".into()),
            Logged::Text("invalid".into()),
            Logged::Text("invalid".into()),
            Logged::Text("invalid".into()),
            Logged::Text("invalid".into()),
            Logged::Text("invalid".into()),
            Logged::Text("".into()),
        ]
    );
}

#[test]
fn round_trip_through_the_host() {
    let out = run("\
\t\tbytes b = raw(0)
\t\temitInt(b.length)
\t\temitData(b.slice(0, 3) + b.slice(3, 5))
");
    assert_eq!(out, [Logged::Int(5), Logged::Data(b"caf\xC3\xA9".to_vec())]);
}

/// LEX-06 `BytesLiteral` (bbdf483): the string shape with `\xNN`, no
/// `\u`, no interpolation.
#[test]
fn bytes_literals_lex_and_compare() {
    let out = run("\
\t\tbytes b = b\"caf\\xC3\\xA9\"
\t\temitInt(b.length)
\t\temitBool(b == bytes.fromText(\"café\"))
\t\temitInt(b\"\\x00\\xFF\"[1])
\t\temitData(b\"a{b}\" + b\"\\t\")
\t\temitInt(b\"\".length)
");
    assert_eq!(
        out,
        [
            Logged::Int(5),
            Logged::Bool(1),
            Logged::Int(0xFF),
            Logged::Data(b"a{b}\t".to_vec()),
            Logged::Int(0),
        ]
    );
}

#[test]
fn bytes_literal_rejects_unicode_escape() {
    let main = "functions:\n\tvoid init()\n\t\tbytes b = b\"\\u00e9\"\n\t\temitInt(b.length)\n"
        .to_string();
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
    for s in &validated.request.sources {
        clean_compiler::lexer::lex(&s.path, &s.content, &mut sink);
    }
    let diagnostics = sink.into_diagnostics();
    assert!(
        diagnostics.iter().any(|d| d.code == "SYN005"),
        "expected SYN005 for \\u in a bytes literal, got: {diagnostics:#?}"
    );
}
