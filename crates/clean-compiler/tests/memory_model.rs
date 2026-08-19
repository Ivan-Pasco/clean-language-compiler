//! Milestone 6 stage 1 — the MMD-01/02/03/04 memory model, verified against
//! the emitted binary under wasmtime (KNOWLEDGE §9: the binary, never the
//! generator, is the proof).

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::layout::{EMPTY_STRING_ADDR, HEAP_START};
use clean_compiler::{codegen, hir, mir, parser, resolver, typecheck};

mod common;

fn compile_with_tier(sources: &[(&str, &str)], tier: &str) -> Vec<u8> {
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
        clean_compiler::layout::tier(tier).expect("known tier"),
        &mut sink,
    );
    assert!(
        sink.unsupported().is_empty(),
        "unexpected unsupported constructs: {:#?}",
        sink.unsupported()
    );
    codegen::core::emit_core(&mir).expect("static data fits below the heap start")
}

fn compile(sources: &[(&str, &str)]) -> Vec<u8> {
    compile_with_tier(sources, "standard")
}

/// One recorded host call: `(rendered value, payload ptr)` — the pointer is
/// part of several layout assertions.
type CallLog = Vec<(String, i32)>;

fn read_string(caller: &mut wasmtime::Caller<'_, CallLog>, ptr: i32, len: i32) -> String {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .expect("guest exports memory");
    let mut buf = vec![0u8; len as usize];
    memory
        .read(&caller, ptr as usize, &mut buf)
        .expect("string payload is in bounds");
    String::from_utf8(buf).expect("payload is UTF-8")
}

/// The probe world: one sink per shape under test.
const PROBE: &str = "\
host interface probe version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function emitBool(value: boolean)
\t\tdescription \"Record one boolean.\"

\thost function emitText(value: string)
\t\tdescription \"Record one string.\"
";

fn instantiate(wasm: &[u8]) -> (wasmtime::Store<CallLog>, wasmtime::Instance) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm).expect("module loads");
    let mut linker: wasmtime::Linker<CallLog> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-bool",
            |mut caller: wasmtime::Caller<'_, CallLog>, value: i32| {
                caller.data_mut().push((value.to_string(), 0));
            },
        )
        .expect("emit-bool links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-text",
            |mut caller: wasmtime::Caller<'_, CallLog>, ptr: i32, len: i32| {
                let text = read_string(&mut caller, ptr, len);
                caller.data_mut().push((text, ptr));
            },
        )
        .expect("emit-text links");
    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    (store, instance)
}

fn run_init(store: &mut wasmtime::Store<CallLog>, instance: &wasmtime::Instance) {
    instance
        .get_typed_func::<(), ()>(&mut *store, "init")
        .expect("init export")
        .call(store, ())
        .expect("init runs");
}

fn global_i32(
    store: &mut wasmtime::Store<CallLog>,
    instance: &wasmtime::Instance,
    name: &str,
) -> i32 {
    match instance
        .get_global(&mut *store, name)
        .unwrap_or_else(|| panic!("global '{name}' is exported"))
        .get(store)
    {
        wasmtime::Val::I32(v) => v,
        other => panic!("global '{name}' is not i32: {other:?}"),
    }
}

#[test]
fn mmd01_layout_in_emitted_binary() {
    let main = "\
functions:
\tvoid init()
\t\temitText(\"hi\")
\t\temitText(\"\")
";
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", main)]);
    wasmparser::validate(&wasm).expect("core module validates");
    let (mut store, instance) = instantiate(&wasm);

    // MMD-01 guest-visible globals.
    assert_eq!(
        global_i32(&mut store, &instance, "__heap_start"),
        HEAP_START as i32
    );
    assert_eq!(
        global_i32(&mut store, &instance, "__heap_ptr"),
        HEAP_START as i32
    );

    // TIER-01 standard: 2 MiB initial, 32 MiB maximum.
    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");
    assert_eq!(memory.size(&store), 32);
    assert_eq!(memory.ty(&store).maximum(), Some(512));

    // The empty-string constant occupies the first 4 bytes of the data
    // section; the first interned literal follows it.
    let mut head = [0u8; 4];
    memory
        .read(&store, EMPTY_STRING_ADDR as usize, &mut head)
        .expect("data section is mapped");
    assert_eq!(head, [0, 0, 0, 0], "empty-string constant is zero length");

    run_init(&mut store, &instance);
    let log = store.data().clone();
    assert_eq!(log[0].0, "hi");
    // "hi" is the first string interned after the seeded empty constant:
    // object at 1028, payload at 1032 (MMD-04: address = length field).
    assert_eq!(log[0].1, EMPTY_STRING_ADDR as i32 + 8);
    assert_eq!(log[1].0, "");
    // Every empty string shares EMPTY_STRING_ADDR; its payload is +4.
    assert_eq!(log[1].1, EMPTY_STRING_ADDR as i32 + 4);
}

#[test]
fn string_equality_truth_table() {
    // Both polarities across equal / unequal / prefix / empty, plus a
    // content-equal pair with distinct pointers (concat result vs literal)
    // so the fast path cannot answer alone (KNOWLEDGE §2).
    let main = "\
functions:
\tvoid init()
\t\temitBool(\"abc\" == \"abc\")
\t\temitBool(\"abc\" == \"abd\")
\t\temitBool(\"abc\" == \"ab\")
\t\temitBool(\"\" == \"\")
\t\temitBool(\"abc\" != \"abc\")
\t\temitBool(\"abc\" != \"abd\")
\t\temitBool(\"abc\" != \"ab\")
\t\temitBool(\"\" != \"\")
\t\temitBool((\"ab\" + \"c\") == \"abc\")
\t\temitBool((\"ab\" + \"c\") != \"abc\")
";
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", main)]);
    let (mut store, instance) = instantiate(&wasm);
    run_init(&mut store, &instance);
    let values: Vec<&str> = store.data().iter().map(|(v, _)| v.as_str()).collect();
    assert_eq!(
        values,
        ["1", "0", "0", "1", "0", "1", "1", "0", "1", "0"],
        "eq/neq truth table (order: ==×4, !=×4, cross-pointer ==/!=)"
    );
}

#[test]
fn string_concat_allocates_and_composes() {
    let main = "\
functions:
\tvoid init()
\t\tstring greeting = \"Hello, \" + \"World\"
\t\temitText(greeting + \"!\")
\t\temitText(\"\" + \"\")
";
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", main)]);
    let (mut store, instance) = instantiate(&wasm);
    run_init(&mut store, &instance);
    let log = store.data().clone();
    assert_eq!(log[0].0, "Hello, World!");
    // Concat results live on the heap, behind HEAP_START.
    assert!(
        log[0].1 >= HEAP_START as i32,
        "concat result at {} is not heap-allocated",
        log[0].1
    );
    // "" + "" returns the shared constant, never an allocation (MMD-01).
    assert_eq!(log[1].0, "");
    assert_eq!(log[1].1, EMPTY_STRING_ADDR as i32 + 4);
}

/// Doubles a 4096-byte literal `n` times without loop support: each `set`
/// re-binds to `s + s`.
fn doubling_program(doublings: usize) -> String {
    let seed = "x".repeat(4096);
    let mut program = format!("functions:\n\tvoid init()\n\t\tstring s = \"{seed}\"\n");
    for _ in 0..doublings {
        program.push_str("\t\ts = s + s\n");
    }
    program.push_str("\t\temitText(s)\n");
    program
}

#[test]
fn allocator_grows_past_the_initial_commitment() {
    // 4096 × 2¹⁰ = 4 MiB — past the standard tier's 2 MiB initial memory,
    // well inside its 32 MiB maximum (TIER-02 growth).
    let main = doubling_program(10);
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    let (mut store, instance) = instantiate(&wasm);
    run_init(&mut store, &instance);
    assert_eq!(store.data()[0].0.len(), 4096 << 10);
    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");
    assert!(
        memory.size(&store) > 32,
        "memory did not grow past the initial 32 pages"
    );
    assert!(
        memory.size(&store) <= 512,
        "memory grew past the tier maximum"
    );
}

#[test]
fn allocator_traps_at_the_tier_ceiling() {
    // Enough doublings that cumulative bump allocation must pass 32 MiB;
    // MMD-02: the failed growth traps at the call site, no failure value.
    let main = doubling_program(14);
    let wasm = compile(&[("app/host_bridge.cln", PROBE), ("app/main.cln", &main)]);
    let (mut store, instance) = instantiate(&wasm);
    let err = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect_err("allocation past the tier limit must trap");
    assert_eq!(
        err.downcast_ref::<wasmtime::Trap>(),
        Some(&wasmtime::Trap::UnreachableCodeReached),
        "trap kind: {err:?}"
    );
}

#[test]
fn handle_scope_resets_the_heap_pointer() {
    // TIER-04 per-request reset: the `handle` shim restores `__heap_ptr`,
    // so repeated requests do not accumulate heap.
    let sources = [
        ("app/host_bridge.cln", PROBE),
        (
            "app/main.cln",
            "\
functions:
\tvoid handle(integer handlerId)
\t\temitText(\"a\" + \"b\")
",
        ),
    ];
    let wasm = compile(&sources);
    let (mut store, instance) = instantiate(&wasm);
    let handle = instance
        .get_typed_func::<i32, ()>(&mut store, "handle")
        .expect("handle export");

    let before = global_i32(&mut store, &instance, "__heap_ptr");
    handle.call(&mut store, 1).expect("first request");
    let after_first = global_i32(&mut store, &instance, "__heap_ptr");
    handle.call(&mut store, 2).expect("second request");
    let after_second = global_i32(&mut store, &instance, "__heap_ptr");

    assert_eq!(before, after_first, "heap pointer reset after request 1");
    assert_eq!(before, after_second, "heap pointer reset after request 2");
    assert_eq!(
        store
            .data()
            .iter()
            .map(|(v, _)| v.as_str())
            .collect::<Vec<_>>(),
        ["ab", "ab"]
    );
}

#[test]
fn minimal_tier_starts_below_heap_start_and_grows_across_it() {
    // TIER-01 minimal: 8 initial pages (512 KiB) sit below HEAP_START; the
    // first allocation must grow across the gap rather than trap.
    let main = "\
functions:
\tvoid init()
\t\temitText(\"grow\" + \" across\")
";
    let wasm = compile_with_tier(
        &[("app/host_bridge.cln", PROBE), ("app/main.cln", main)],
        "minimal",
    );
    let (mut store, instance) = instantiate(&wasm);
    let memory = instance
        .get_memory(&mut store, "memory")
        .expect("memory export");
    assert_eq!(memory.size(&store), 8);
    assert_eq!(memory.ty(&store).maximum(), Some(128));
    run_init(&mut store, &instance);
    assert_eq!(store.data()[0].0, "grow across");
    assert!(memory.size(&store) >= 17, "grew past HEAP_START");
}
