//! Milestone 6 stage 4 — `list<T>` runtime machinery over the MMD §3.4.1
//! layout: construction, index access with bounds trap, list iterate, and
//! the internal→Canonical-ABI boundary serialization, verified under
//! wasmtime against the emitted binary (KNOWLEDGE §9).

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::layout::{HEAP_START, LIST_ELEMS_OFFSET};
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

\thost function emitText(value: string)
\t\tdescription \"Record one string.\"

\thost function emitInts(values: list<integer>)
\t\tdescription \"Record an integer list.\"

\thost function emitTexts(values: list<string>)
\t\tdescription \"Record a string list.\"
";

#[derive(Debug, PartialEq)]
enum Logged {
    Int(i64),
    Text(String),
    Ints(Vec<i64>),
    Texts(Vec<String>),
}

fn read_bytes(caller: &mut wasmtime::Caller<'_, Vec<Logged>>, ptr: i32, len: usize) -> Vec<u8> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .expect("guest exports memory");
    let mut buf = vec![0u8; len];
    memory
        .read(&caller, ptr as usize, &mut buf)
        .expect("read in bounds");
    buf
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
        .expect("emit-int links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-text",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, ptr: i32, len: i32| {
                let bytes = read_bytes(&mut caller, ptr, len as usize);
                caller
                    .data_mut()
                    .push(Logged::Text(String::from_utf8(bytes).expect("UTF-8")));
            },
        )
        .expect("emit-text links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-ints",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, ptr: i32, count: i32| {
                // Canonical ABI list<s64>: contiguous 8-byte elements.
                let bytes = read_bytes(&mut caller, ptr, count as usize * 8);
                let values = bytes
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|c| i64::from_le_bytes(*c))
                    .collect();
                caller.data_mut().push(Logged::Ints(values));
            },
        )
        .expect("emit-ints links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-texts",
            |mut caller: wasmtime::Caller<'_, Vec<Logged>>, ptr: i32, count: i32| {
                // Canonical ABI list<string>: (ptr, len) pairs.
                let heads = read_bytes(&mut caller, ptr, count as usize * 8);
                let mut values = Vec::new();
                for pair in heads.as_chunks::<8>().0 {
                    let sptr = i32::from_le_bytes(pair[0..4].try_into().unwrap());
                    let slen = i32::from_le_bytes(pair[4..8].try_into().unwrap());
                    let bytes = read_bytes(&mut caller, sptr, slen as usize);
                    values.push(String::from_utf8(bytes).expect("UTF-8"));
                }
                caller.data_mut().push(Logged::Texts(values));
            },
        )
        .expect("emit-texts links");
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
fn constant_lists_cross_the_boundary() {
    let out = run("\
\t\temitInts([10, 20, 30])
\t\temitTexts([\"a\", \"bc\", \"\"])
");
    assert_eq!(
        out,
        [
            Logged::Ints(vec![10, 20, 30]),
            Logged::Texts(vec!["a".into(), "bc".into(), "".into()]),
        ]
    );
}

#[test]
fn runtime_built_lists_cross_the_boundary() {
    // Elements computed at runtime force the heap construction path.
    let out = run("\
\t\tinteger n = 4
\t\temitInts([n, n * 2, n * n])
\t\tstring s = \"a\" + \"b\"
\t\temitTexts([s, s + \"c\"])
");
    assert_eq!(
        out,
        [
            Logged::Ints(vec![4, 8, 16]),
            Logged::Texts(vec!["ab".into(), "abc".into()]),
        ]
    );
}

#[test]
fn index_access_reads_elements() {
    let out = run("\
\t\tlist<integer> xs = [7, 11, 13]
\t\temitInt(xs[0])
\t\temitInt(xs[2])
\t\tlist<string> names = [\"ana\", \"luz\"]
\t\temitText(names[1])
");
    assert_eq!(
        out,
        [Logged::Int(7), Logged::Int(13), Logged::Text("luz".into())]
    );
}

#[test]
fn out_of_range_index_traps() {
    let main = "\
functions:
\tvoid init()
\t\tlist<integer> xs = [1, 2]
\t\temitInt(xs[2])
";
    let main = main.to_string();
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    let (mut store, instance) = instantiate(&wasm);
    let err = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect_err("index 2 into a 2-element list must trap");
    assert_eq!(
        err.downcast_ref::<wasmtime::Trap>(),
        Some(&wasmtime::Trap::UnreachableCodeReached),
        "trap kind: {err:?}"
    );
}

#[test]
fn negative_index_traps() {
    let main = "\
functions:
\tvoid init()
\t\tlist<integer> xs = [1, 2]
\t\tinteger i = 0 - 1
\t\temitInt(xs[i])
";
    let main = main.to_string();
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    let (mut store, instance) = instantiate(&wasm);
    let err = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect_err("negative index must trap");
    assert_eq!(
        err.downcast_ref::<wasmtime::Trap>(),
        Some(&wasmtime::Trap::UnreachableCodeReached),
        "trap kind: {err:?}"
    );
}

#[test]
fn iterate_over_lists_binds_each_element() {
    let out = run("\
\t\titerate x in [3, 1, 4]
\t\t\temitInt(x)
\t\tlist<string> greetings = [\"hola\", \"adios\"]
\t\titerate g in greetings
\t\t\temitText(g)
");
    assert_eq!(
        out,
        [
            Logged::Int(3),
            Logged::Int(1),
            Logged::Int(4),
            Logged::Text("hola".into()),
            Logged::Text("adios".into()),
        ]
    );
}

#[test]
fn iterate_over_lists_honors_break_and_continue() {
    let out = run("\
\t\titerate x in [1, 2, 3, 4, 5]
\t\t\tif x == 2
\t\t\t\tcontinue
\t\t\tif x == 4
\t\t\t\tbreak
\t\t\temitInt(x)
");
    assert_eq!(out, [Logged::Int(1), Logged::Int(3)]);
}

#[test]
fn static_list_object_matches_the_mmd_layout() {
    let main = "\
functions:
\tvoid init()
\t\tlist<integer> xs = [5, 6]
\t\temitInt(xs[0])
";
    let main = main.to_string();
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    let (mut store, instance) = instantiate(&wasm);
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect("init runs");
    // Locate the static object: scan the data region below HEAP_START for
    // the header {len=2, cap=2} followed by 5,6 as i64 LE at +16.
    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");
    let mut region = vec![0u8; HEAP_START as usize];
    memory
        .read(&store, 0, &mut region)
        .expect("read data region");
    let needle_elems: Vec<u8> = [5i64, 6i64].iter().flat_map(|v| v.to_le_bytes()).collect();
    let found = region
        .windows(LIST_ELEMS_OFFSET as usize + needle_elems.len())
        .enumerate()
        .any(|(at, w)| {
            let len = u32::from_le_bytes(w[0..4].try_into().unwrap());
            let cap = u32::from_le_bytes(w[4..8].try_into().unwrap());
            len == 2
                && cap == 2
                && w[LIST_ELEMS_OFFSET as usize..] == needle_elems[..]
                && at % 8 == 0
        });
    assert!(
        found,
        "no 8-aligned §3.4.1 header {{len=2, cap=2}} with elements [5, 6] in static data"
    );
}
