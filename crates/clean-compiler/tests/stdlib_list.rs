//! Milestone 6 stage 5c — the chapter-15 list module, verified under
//! wasmtime. `add`/`insert` stay blocked (§3.4.1 inline elements + growth
//! relocation break aliasing — DISCOVERIES-M6); everything else lowers.

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

fn ints(log: &[Logged]) -> Vec<i64> {
    log.iter()
        .map(|l| match l {
            Logged::Int(v) => *v,
            other => panic!("expected Int, got {other:?}"),
        })
        .collect()
}

#[test]
fn length_and_emptiness() {
    let out = run("\
\t\tlist<integer> xs = [1, 2, 3]
\t\temitInt(xs.length())
\t\temitBool(xs.isEmpty())
\t\temitBool(xs.isNotEmpty())
");
    assert_eq!(out, [Logged::Int(3), Logged::Bool(0), Logged::Bool(1)]);
}

#[test]
fn get_and_set_in_place() {
    let out = run("\
\t\tlist<integer> xs = [10, 20, 30]
\t\txs.set(1, 99)
\t\temitInt(xs.get(0))
\t\temitInt(xs.get(1))
\t\temitInt(xs[1])
");
    assert_eq!(ints(&out), [10, 99, 99]);
}

#[test]
fn first_last_and_removals() {
    let out = run("\
\t\tlist<integer> xs = [5, 6, 7, 8]
\t\temitInt(xs.first())
\t\temitInt(xs.last())
\t\temitInt(xs.removeLast())
\t\temitInt(xs.length())
\t\temitInt(xs.remove(0))
\t\temitInt(xs.first())
\t\temitInt(xs.length())
");
    assert_eq!(ints(&out), [5, 8, 8, 3, 5, 6, 2]);
}

#[test]
fn behavior_line_and_pile() {
    let out = run("\
\t\tlist<integer>.line queue = [1, 2, 3]
\t\temitInt(queue.peek())
\t\temitInt(queue.remove())
\t\temitInt(queue.peek())
\t\tlist<integer>.pile stack = [1, 2, 3]
\t\temitInt(stack.peek())
\t\temitInt(stack.remove())
\t\temitInt(stack.peek())
");
    assert_eq!(ints(&out), [1, 1, 2, 3, 3, 2]);
}

#[test]
fn searching_by_element_type() {
    let out = run("\
\t\tlist<integer> xs = [7, 8, 7]
\t\temitBool(xs.contains(8))
\t\temitBool(xs.contains(9))
\t\temitInt(xs.indexOf(7))
\t\temitInt(xs.lastIndexOf(7))
\t\temitInt(xs.indexOf(9))
\t\tlist<string> names = [\"ana\", \"luz\", \"ana\"]
\t\temitBool(names.contains(\"luz\"))
\t\temitInt(names.indexOf(\"ana\"))
\t\temitInt(names.lastIndexOf(\"ana\"))
\t\tlist<number> ps = [0.5, 1.5]
\t\temitInt(ps.indexOf(1.5))
");
    assert_eq!(
        out,
        [
            Logged::Bool(1),
            Logged::Bool(0),
            Logged::Int(0),
            Logged::Int(2),
            Logged::Int(-1),
            Logged::Bool(1),
            Logged::Int(0),
            Logged::Int(2),
            Logged::Int(1),
        ]
    );
}

#[test]
fn slice_reverse_sort_are_fresh_lists() {
    let out = run("\
\t\tlist<integer> xs = [3, 1, 4, 1, 5]
\t\titerate v in xs.slice(1, 4)
\t\t\temitInt(v)
\t\titerate v in xs.reverse()
\t\t\temitInt(v)
\t\titerate v in xs.sort()
\t\t\temitInt(v)
\t\temitInt(xs.first())
\t\titerate v in xs.slice(3, 2)
\t\t\temitInt(v)
\t\titerate s in [\"pear\", \"fig\", \"kiwi\"].sort()
\t\t\temitText(s)
");
    assert_eq!(
        out,
        [
            Logged::Int(1),
            Logged::Int(4),
            Logged::Int(1),
            Logged::Int(5),
            Logged::Int(1),
            Logged::Int(4),
            Logged::Int(1),
            Logged::Int(3),
            Logged::Int(1),
            Logged::Int(1),
            Logged::Int(3),
            Logged::Int(4),
            Logged::Int(5),
            // xs unchanged by any of the above (fresh lists).
            Logged::Int(3),
            // slice(3, 2) is empty.
            Logged::Text("fig".into()),
            Logged::Text("kiwi".into()),
            Logged::Text("pear".into()),
        ]
    );
}

#[test]
fn sort_numbers() {
    let out = run("\
\t\titerate v in [2.5, 0.5, 1.5].sort()
\t\t\temitNum(v)
");
    assert_eq!(out, [Logged::Num(0.5), Logged::Num(1.5), Logged::Num(2.5)]);
}

#[test]
fn namespace_functions() {
    let out = run("\
\t\titerate v in list.concat([1, 2], [3])
\t\t\temitInt(v)
\t\titerate v in list.range(1, 4)
\t\t\temitInt(v)
\t\titerate v in list.range(5, 1)
\t\t\temitInt(v)
\t\titerate v in list.fill(3, 7)
\t\t\temitInt(v)
\t\temitText(list.join([\"a\", \"b\", \"c\"], \", \"))
\t\temitText(list.join([\"solo\"], \"-\"))
");
    assert_eq!(
        out,
        [
            Logged::Int(1),
            Logged::Int(2),
            Logged::Int(3),
            Logged::Int(1),
            Logged::Int(2),
            Logged::Int(3),
            Logged::Int(4),
            Logged::Int(5),
            Logged::Int(4),
            Logged::Int(3),
            Logged::Int(2),
            Logged::Int(1),
            Logged::Int(7),
            Logged::Int(7),
            Logged::Int(7),
            Logged::Text("a, b, c".into()),
            Logged::Text("solo".into()),
        ]
    );
}

#[test]
fn empty_collection_access_traps() {
    for body in [
        "\t\tlist<integer> xs = []\n\t\temitInt(xs.first())\n",
        "\t\tlist<integer> xs = []\n\t\temitInt(xs.removeLast())\n",
    ] {
        let main = format!("functions:\n\tvoid init()\n{body}");
        let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
        let (mut store, instance) = instantiate(&wasm);
        let err = instance
            .get_typed_func::<(), ()>(&mut store, "init")
            .expect("init export")
            .call(&mut store, ())
            .expect_err("empty-collection access must trap");
        assert!(err.downcast_ref::<wasmtime::Trap>().is_some());
    }
}

#[test]
fn remove_without_behavior_is_sem004() {
    let main = "functions:\n\tvoid init()\n\t\tlist<integer> xs = [1]\n\t\temitInt(xs.remove())\n"
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
        diagnostics.iter().any(|d| d.code == "SEM004"),
        "expected SEM004 for remove() without a behavior, got: {diagnostics:#?}"
    );
}
