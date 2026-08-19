//! Milestone 6 stage 7 — fallible host imports (framework 09 §8: the
//! declaration carries the ok type, the world carries `result<T, E>`,
//! `onError` is the error channel) and world-qualified import modules,
//! verified under wasmtime against the real vendored host.wit signatures.

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{codegen, hir, mir, parser, resolver, typecheck};

mod common;

fn compile_request(request: clean_compiler_types::CompileRequest) -> Vec<u8> {
    let mut sink = DiagnosticSink::new();
    let validated = match clean_compiler::request::validate(request, &mut sink) {
        Some(v) => v,
        None => panic!("request validates: {:#?}", sink.into_diagnostics()),
    };
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

fn compile(sources: &[(&str, &str)]) -> Vec<u8> {
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
    compile_request(request)
}

/// The websocket surface, declared per 09 §8: ok types only, no result<>.
const WS_BRIDGE: &str = "\
host interface websocket version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function accept() returns integer:u64
\t\tdescription \"Accept the pending upgrade.\"

\thost function sendText(socket: integer:u64, message: string)
\t\tdescription \"Queue a text message.\"

host interface probe version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function emitInt(value: integer)
\t\tdescription \"Record one integer.\"
";

/// Host state: whether fallible calls fail, plus the observed calls.
struct Host {
    fail: bool,
    log: Vec<String>,
}

fn instantiate(wasm: &[u8], fail: bool) -> (wasmtime::Store<Host>, wasmtime::Instance) {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Host> = wasmtime::Linker::new(&engine);
    // accept: () -> result<u64, socket-error>. Core shape: (retptr) -> ().
    // Canonical layout: disc u8 @0; ok u64 @8 | err enum @8.
    linker
        .func_wrap(
            "clean:host/websocket@0.1.0",
            "accept",
            |mut caller: wasmtime::Caller<'_, Host>, retptr: i32| {
                let fail = caller.data().fail;
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("memory export");
                let mut area = [0u8; 16];
                if fail {
                    area[0] = 1; // err: not-an-upgrade (case 0)
                } else {
                    area[8..16].copy_from_slice(&77u64.to_le_bytes());
                }
                memory
                    .write(&mut caller, retptr as usize, &area)
                    .expect("write result");
                caller.data_mut().log.push(format!("accept(fail={fail})"));
            },
        )
        .expect("links");
    // send-text: (u64, string) -> result<_, socket-error>. Core shape:
    // (i64, ptr, len, retptr) -> (). Layout: disc u8 @0; err @1.
    linker
        .func_wrap(
            "clean:host/websocket@0.1.0",
            "send-text",
            |mut caller: wasmtime::Caller<'_, Host>,
             socket: i64,
             ptr: i32,
             len: i32,
             retptr: i32| {
                let fail = caller.data().fail;
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("memory export");
                let mut buf = vec![0u8; len as usize];
                memory.read(&caller, ptr as usize, &mut buf).expect("read");
                let message = String::from_utf8(buf).expect("UTF-8");
                let disc: &[u8] = if fail { &[1, 1] } else { &[0, 0] };
                memory
                    .write(&mut caller, retptr as usize, disc)
                    .expect("write result");
                caller
                    .data_mut()
                    .log
                    .push(format!("send-text({socket}, \"{message}\", fail={fail})"));
            },
        )
        .expect("links");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-int",
            |mut caller: wasmtime::Caller<'_, Host>, value: i64| {
                caller.data_mut().log.push(format!("emit({value})"));
            },
        )
        .expect("links");
    let mut store = wasmtime::Store::new(
        &engine,
        Host {
            fail,
            log: Vec::new(),
        },
    );
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    (store, instance)
}

fn run(body: &str, fail: bool) -> Result<Vec<String>, wasmtime::Error> {
    let main = format!("functions:\n\tvoid init()\n{body}");
    let wasm = compile(&[("app/host_bridge.cln", WS_BRIDGE), ("app/main.cln", &main)]);
    wasmparser::validate(&wasm).expect("core module validates");
    let (mut store, instance) = instantiate(&wasm, fail);
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())?;
    Ok(store.into_data().log)
}

#[test]
fn on_error_takes_the_ok_arm() {
    let log = run(
        "\t\tinteger sock = accept() onError 0\n\t\temitInt(sock)\n",
        false,
    )
    .expect("ok arm runs");
    assert_eq!(log, ["accept(fail=false)", "emit(77)"]);
}

#[test]
fn on_error_takes_the_fallback_arm() {
    let log = run(
        "\t\tinteger sock = accept() onError 0\n\t\temitInt(sock)\n",
        true,
    )
    .expect("fallback arm runs");
    assert_eq!(log, ["accept(fail=true)", "emit(0)"]);
}

#[test]
fn bare_fallible_call_succeeds_on_ok() {
    let log = run(
        "\t\tinteger sock = accept()\n\t\temitInt(sock)\n\t\tsendText(sock, \"hi\")\n",
        false,
    )
    .expect("ok path runs");
    assert_eq!(
        log,
        [
            "accept(fail=false)",
            "emit(77)",
            "send-text(77, \"hi\", fail=false)"
        ]
    );
}

#[test]
fn bare_fallible_call_traps_on_error() {
    // RUN018's unhandled-error shape until error lowering: the error arm
    // of an unhandled fallible call traps.
    let err = run("\t\tinteger sock = accept()\n\t\temitInt(sock)\n", true)
        .expect_err("error arm must trap");
    assert!(
        err.downcast_ref::<wasmtime::Trap>().is_some(),
        "expected a trap: {err:?}"
    );
}

#[test]
fn void_fallible_call_traps_on_error() {
    let err = run("\t\tsendText(9, \"boom\")\n", true).expect_err("error arm must trap");
    assert!(err.downcast_ref::<wasmtime::Trap>().is_some());
}

/// A composed-bridge interface lives in its own package (the /counter
/// shape): the emitted import module must be world-qualified, not
/// clean:host.
#[test]
fn imports_resolve_to_the_world_package() {
    let bridge_wit = r#"package clean:test@0.1.0;

package clean:fake-bridge@0.1.0 {
    interface store {
        bump: func() -> u32;
    }
}

world testworld {
    export clean:fake-bridge/store@0.1.0;

    import init: func();
    import handle: func(handler-id: u32);
}
"#;
    let bridge_decl = "\
host interface store version \"0.1.0\":
\trequires host worlds [\"testworld\"]

\thost function bump() returns integer:u32
\t\tdescription \"Increment and return the counter.\"
";
    let main = "\
functions:
\tvoid init()
\t\tinteger n = bump()
";
    let mut request = common::minimal_valid_request();
    request.target_world.wit = bridge_wit.to_string();
    request.target_world.sha256 = common::sha256_hex(bridge_wit.as_bytes());
    request.target_world.world = "testworld".to_string();
    request.sources = [("app/host_bridge.cln", bridge_decl), ("app/main.cln", main)]
        .iter()
        .map(
            |(path, content)| clean_compiler_types::request::SourceFile {
                path: path.to_string(),
                sha256: common::sha256_hex(content.as_bytes()),
                content: content.to_string(),
            },
        )
        .collect();
    let wasm = compile_request(request);

    // The import must link under the fake-bridge package, not clean:host.
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let import = module.imports().next().expect("one import");
    assert_eq!(import.module(), "clean:fake-bridge/store@0.1.0");
    assert_eq!(import.name(), "bump");
}

/// 37cda47: the error payload is discarded, never read — an err side
/// WITH a payload (here a string) lowers, and its 4-byte alignment
/// positions a u64 ok payload at offset 8 all the same.
#[test]
fn payload_carrying_errors_lower_and_discard() {
    let wit = r#"package clean:test@0.1.0;

package clean:payload@0.1.0 {
    interface risky {
        fetch: func(which: u32) -> result<u64, string>;
    }
    interface probe {
        emit-int: func(value: u64);
    }
}

world testworld {
    export clean:payload/risky@0.1.0;
    export clean:payload/probe@0.1.0;

    import init: func();
    import handle: func(handler-id: u32);
}
"#;
    let bridge_decl = "\
host interface risky version \"0.1.0\":
\trequires host worlds [\"testworld\"]

\thost function fetch(which: integer:u32) returns integer:u64
\t\tdescription \"Fails with a stringly error on demand.\"

host interface probe version \"0.1.0\":
\trequires host worlds [\"testworld\"]

\thost function emitInt(value: integer:u64)
\t\tdescription \"Record one integer.\"
";
    let main = "\
functions:
\tvoid init()
\t\temitInt(fetch(0) onError 111)
\t\temitInt(fetch(1) onError 111)
";
    let mut request = common::minimal_valid_request();
    request.target_world.wit = wit.to_string();
    request.target_world.sha256 = common::sha256_hex(wit.as_bytes());
    request.target_world.world = "testworld".to_string();
    request.sources = [("app/host_bridge.cln", bridge_decl), ("app/main.cln", main)]
        .iter()
        .map(
            |(path, content)| clean_compiler_types::request::SourceFile {
                path: path.to_string(),
                sha256: common::sha256_hex(content.as_bytes()),
                content: content.to_string(),
            },
        )
        .collect();
    let wasm = compile_request(request);

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Vec<i64>> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:payload/risky@0.1.0",
            "fetch",
            |mut caller: wasmtime::Caller<'_, Vec<i64>>, which: i32, retptr: i32| {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("memory export");
                if which == 0 {
                    // ok arm: disc 0 @0, u64 payload @8.
                    let mut buf = [0u8; 16];
                    buf[8..16].copy_from_slice(&777u64.to_le_bytes());
                    memory.write(&mut caller, retptr as usize, &buf).unwrap();
                } else {
                    // err arm: disc 1, then a (ptr, len) string payload the
                    // guest must ignore entirely.
                    let mut buf = [0u8; 16];
                    buf[0] = 1;
                    memory.write(&mut caller, retptr as usize, &buf).unwrap();
                }
            },
        )
        .expect("links");
    linker
        .func_wrap(
            "clean:payload/probe@0.1.0",
            "emit-int",
            |mut caller: wasmtime::Caller<'_, Vec<i64>>, value: i64| {
                caller.data_mut().push(value);
            },
        )
        .expect("links");
    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect("init runs");
    assert_eq!(store.data().as_slice(), [777, 111]);
}
