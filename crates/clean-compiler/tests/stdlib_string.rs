//! Milestone 6 stage 5b — the chapter-15 string module, verified under
//! wasmtime. Indexes and lengths count code points (local adoption,
//! DISCOVERIES-M6), exercised with multi-byte UTF-8 throughout.

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

\thost function emitBool(value: boolean)
\t\tdescription \"Record one boolean.\"

\thost function emitText(value: string)
\t\tdescription \"Record one string.\"
";

#[derive(Debug, PartialEq)]
enum Logged {
    Int(i64),
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

fn bools(log: &[Logged]) -> Vec<i32> {
    log.iter()
        .map(|l| match l {
            Logged::Bool(v) => *v,
            other => panic!("expected Bool, got {other:?}"),
        })
        .collect()
}

fn texts(log: &[Logged]) -> Vec<String> {
    log.iter()
        .map(|l| match l {
            Logged::Text(v) => v.clone(),
            other => panic!("expected Text, got {other:?}"),
        })
        .collect()
}

#[test]
fn length_counts_code_points() {
    let out = run("\
\t\temitInt(\"hello\".length())
\t\temitInt(\"\".length())
\t\temitInt(\"héllo\".length())
\t\temitInt(\"día 🎉\".length())
");
    assert_eq!(ints(&out), [5, 0, 5, 5]);
}

#[test]
fn emptiness_and_blankness() {
    let out = run("\
\t\temitBool(\"\".isEmpty())
\t\temitBool(\" \".isEmpty())
\t\temitBool(\"\".isBlank())
\t\temitBool(\" \\t\\n\\r \".isBlank())
\t\temitBool(\" x \".isBlank())
");
    assert_eq!(bools(&out), [1, 0, 1, 1, 0]);
}

#[test]
fn searching() {
    let out = run("\
\t\temitBool(\"hello world\".contains(\"o w\"))
\t\temitBool(\"hello\".contains(\"z\"))
\t\temitBool(\"hello\".startsWith(\"he\"))
\t\temitBool(\"hello\".startsWith(\"hello!\"))
\t\temitBool(\"hello\".endsWith(\"lo\"))
\t\temitBool(\"hello\".endsWith(\"hel\"))
\t\temitInt(\"banana\".indexOf(\"na\"))
\t\temitInt(\"banana\".lastIndexOf(\"na\"))
\t\temitInt(\"banana\".indexOf(\"zz\"))
\t\temitInt(\"début fin\".indexOf(\"fin\"))
");
    assert_eq!(
        bools(&out[0..6]),
        [1, 0, 1, 0, 1, 0],
        "contains/startsWith/endsWith"
    );
    // "début fin": f is the 7th code point (index 6) even though é is two
    // bytes — indexes count code points.
    assert_eq!(ints(&out[6..]), [2, 4, -1, 6]);
}

#[test]
fn char_access_by_code_point() {
    let out = run("\
\t\temitText(\"héllo\".charAt(1))
\t\temitText(\"día 🎉\".charAt(4))
\t\temitInt(\"A\".charCodeAt(0))
\t\temitInt(\"héllo\".charCodeAt(1))
\t\temitInt(\"día 🎉\".charCodeAt(4))
");
    assert_eq!(
        texts(&out[0..2]),
        ["é".to_string(), "🎉".to_string()],
        "charAt returns whole code points"
    );
    assert_eq!(ints(&out[2..]), [65, 0xE9, 0x1F389]);
}

#[test]
fn char_access_out_of_range_traps() {
    let main = "functions:\n\tvoid init()\n\t\temitText(\"ab\".charAt(2))\n".to_string();
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    let (mut store, instance) = instantiate(&wasm);
    let err = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect_err("charAt(2) on a 2-code-point string must trap");
    assert!(err.downcast_ref::<wasmtime::Trap>().is_some());
}

#[test]
fn substring_clamps_and_slices_code_points() {
    let out = run("\
\t\temitText(\"hello\".substring(1, 3))
\t\temitText(\"héllo\".substring(1, 3))
\t\temitText(\"hello\".substring(3, 99))
\t\temitText(\"hello\".substring(3, 2))
\t\temitText(\"hello\".substring(0 - 5, 2))
");
    assert_eq!(texts(&out), ["el", "él", "lo", "", "he"]);
}

#[test]
fn trimming() {
    let out = run("\
\t\temitText(\"  pad  \".trim())
\t\temitText(\"  pad  \".trimStart())
\t\temitText(\"  pad  \".trimEnd())
\t\temitText(\"\\t\\n x \\r\".trim())
\t\temitText(\"   \".trim())
");
    assert_eq!(texts(&out), ["pad", "pad  ", "  pad", "x", ""]);
}

#[test]
fn padding_counts_code_points() {
    let out = run("\
\t\temitText(\"5\".padStart(3, \"0\"))
\t\temitText(\"5\".padEnd(3, \"ab\"))
\t\temitText(\"abcd\".padStart(2, \"x\"))
\t\temitText(\"x\".padStart(4, \"é\"))
\t\temitText(\"x\".padStart(4, \"\"))
");
    assert_eq!(texts(&out), ["005", "5ab", "abcd", "éééx", "x"]);
}

#[test]
fn replace_all_occurrences() {
    let out = run("\
\t\temitText(\"banana\".replace(\"na\", \"NA\"))
\t\temitText(\"aaa\".replace(\"aa\", \"b\"))
\t\temitText(\"hello\".replace(\"zz\", \"x\"))
\t\temitText(\"hello\".replace(\"\", \"x\"))
\t\temitText(\"día\".replace(\"í\", \"i\"))
");
    assert_eq!(texts(&out), ["baNANA", "ba", "hello", "hello", "dia"]);
}

#[test]
fn split_produces_lists() {
    let out = run("\
\t\titerate part in \"a,b,,c\".split(\",\")
\t\t\temitText(part)
\t\tinteger n = 0
\t\titerate piece in \"a,b\".split(\",\")
\t\t\tn = n + 1
\t\temitInt(n)
\t\titerate solo in \"solo\".split(\"\")
\t\t\temitText(solo)
");
    assert_eq!(
        out,
        [
            Logged::Text("a".into()),
            Logged::Text("b".into()),
            Logged::Text("".into()),
            Logged::Text("c".into()),
            Logged::Int(2),
            Logged::Text("solo".into()),
        ]
    );
}

#[test]
fn namespace_concat() {
    let out = run("\
\t\temitText(string.concat(\"foo\", \"bar\"))
\t\temitText(string.concat(\"\", \"\"))
");
    assert_eq!(texts(&out), ["foobar", ""]);
}

#[test]
fn methods_chain() {
    let out = run("\
\t\temitText(\"  Hello, World  \".trim().replace(\"World\", \"Clean\").substring(0, 5))
\t\temitInt(\"abc\".substring(1, 3).length())
");
    assert_eq!(out, [Logged::Text("Hello".into()), Logged::Int(2)]);
}
