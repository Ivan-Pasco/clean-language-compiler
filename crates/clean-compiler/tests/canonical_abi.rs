//! Milestone 1 step 6 checks (brief acceptance check 4): every boundary
//! type round-trips through a compiled call under wasmtime, in the brief's
//! dependency order — scalars, then string, then record/option/enum. The
//! host stub records what it receives; strings are read back from the
//! guest's exported memory.

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{codegen, hir, mir, parser, resolver, typecheck};

mod common;

fn compile_to_core(sources: &[(&str, &str)]) -> Vec<u8> {
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
    let resolved = resolver::resolve(files, &mut sink);
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
        &mut sink,
    );
    assert!(
        sink.unsupported().is_empty(),
        "unexpected unsupported constructs: {:#?}",
        sink.unsupported()
    );
    codegen::core::emit_core(&mir)
}

/// One recorded host call: function name plus rendered arguments.
type CallLog = Vec<String>;

fn read_string(caller: &mut wasmtime::Caller<'_, CallLog>, ptr: i32, len: i32) -> String {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .expect("guest exports memory");
    let mut buf = vec![0u8; len as usize];
    memory
        .read(caller, ptr as usize, &mut buf)
        .expect("string bytes are in bounds");
    String::from_utf8(buf).expect("guest strings are UTF-8")
}

#[test]
fn roundtrip_register_enum_string_u32_record() {
    // The full `register` shape from host.wit:
    //   register: func(m: method, path: string, handler-id: u32, opts: options)
    // Flattened: (i32 enum, i32 ptr, i32 len, i32 u32, i32 bool).
    let host_bridge = "\
host interface routing version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function register(m: method, path: string, handlerId: integer:u32, opts: options)
\t\tdescription \"Register one route.\"
";
    let options_class = "class Options\n\tboolean csrf\n";
    let main = "\
functions:
\tvoid init()
\t\tregister(\"get\", \"/\", 0, Options(true))
\t\tregister(\"post\", \"/hook\", 7, Options(false))
";
    let wasm = compile_to_core(&[
        ("app/host_bridge.cln", host_bridge),
        ("app/options.cln", options_class),
        ("app/main.cln", main),
    ]);
    wasmparser::validate(&wasm).expect("core module validates");

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<CallLog> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/routing@0.1.0",
            "register",
            |mut caller: wasmtime::Caller<'_, CallLog>,
             method: i32,
             path_ptr: i32,
             path_len: i32,
             handler_id: i32,
             csrf: i32| {
                let path = read_string(&mut caller, path_ptr, path_len);
                caller.data_mut().push(format!(
                    "register({method}, \"{path}\", {handler_id}, {csrf})"
                ));
            },
        )
        .expect("stub links");

    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    let init = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export");
    init.call(&mut store, ()).expect("init runs");

    assert_eq!(
        store.data().as_slice(),
        [
            // method enum: get = case 0, post = case 2 (WIT order).
            "register(0, \"/\", 0, 1)".to_string(),
            "register(2, \"/hook\", 7, 0)".to_string(),
        ]
    );
}

#[test]
fn roundtrip_string_variables_and_reuse() {
    // String values held in locals and interned data deduplication.
    let host_bridge = "\
host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function addHeader(name: string, value: string)
\t\tdescription \"Append a response header.\"
";
    let main = "\
functions:
\tvoid init()
\t\tstring contentType = \"content-type\"
\t\taddHeader(contentType, \"text/plain\")
\t\taddHeader(contentType, \"text/plain\")
";
    let wasm = compile_to_core(&[("app/host_bridge.cln", host_bridge), ("app/main.cln", main)]);

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<CallLog> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/response@0.1.0",
            "add-header",
            |mut caller: wasmtime::Caller<'_, CallLog>,
             n_ptr: i32,
             n_len: i32,
             v_ptr: i32,
             v_len: i32| {
                let name = read_string(&mut caller, n_ptr, n_len);
                let value = read_string(&mut caller, v_ptr, v_len);
                caller.data_mut().push(format!("{name}: {value}"));
            },
        )
        .expect("stub links");

    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect("init runs");

    assert_eq!(
        store.data().as_slice(),
        [
            "content-type: text/plain".to_string(),
            "content-type: text/plain".to_string()
        ]
    );
}

#[test]
fn roundtrip_string_literal_where_bytes_expected() {
    // ADR-0002 boundary identity: a string value satisfies a `bytes`
    // parameter — both are (ptr, len) over UTF-8.
    let host_bridge = "\
host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function setBody(body: bytes)
\t\tdescription \"Set the response body.\"
";
    let main = "\
functions:
\tvoid init()
\t\tsetBody(\"hello world\")
";
    let wasm = compile_to_core(&[("app/host_bridge.cln", host_bridge), ("app/main.cln", main)]);

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<CallLog> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/response@0.1.0",
            "set-body",
            |mut caller: wasmtime::Caller<'_, CallLog>, ptr: i32, len: i32| {
                let body = read_string(&mut caller, ptr, len);
                caller.data_mut().push(body);
            },
        )
        .expect("stub links");

    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect("init runs");

    assert_eq!(store.data().as_slice(), ["hello world".to_string()]);
}

#[test]
fn roundtrip_option_string_return_with_default() {
    // /users/:id shape: get-param returns option<string> via retptr; the
    // guest unwraps with `default` and forwards to set-body.
    let host_bridge = "\
host interface request version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function getParam(name: string) returns string?
\t\tdescription \"One path parameter by name.\"

host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function setBody(body: bytes)
\t\tdescription \"Set the response body.\"
";
    let main = "\
functions:
\tvoid handle(integer handlerId)
\t\tstring id = getParam(\"id\") default \"no id\"
\t\tsetBody(id)
";
    let wasm = compile_to_core(&[("app/host_bridge.cln", host_bridge), ("app/main.cln", main)]);
    wasmparser::validate(&wasm).expect("core module validates");

    // Host state: (param value to return, call log).
    struct Host {
        param: Option<&'static str>,
        log: CallLog,
    }
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Host> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/request@0.1.0",
            "get-param",
            |mut caller: wasmtime::Caller<'_, Host>, _n_ptr: i32, _n_len: i32, retptr: i32| {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("guest exports memory");
                let realloc = caller
                    .get_export("cabi_realloc")
                    .and_then(|e| e.into_func())
                    .expect("guest exports cabi_realloc")
                    .typed::<(i32, i32, i32, i32), i32>(&caller)
                    .expect("realloc signature");
                match caller.data().param {
                    Some(value) => {
                        // Canonical lift into the guest: allocate, copy,
                        // write [disc=1, ptr, len] into the return area.
                        let ptr = realloc
                            .call(&mut caller, (0, 0, 1, value.len() as i32))
                            .expect("realloc runs");
                        memory
                            .write(&mut caller, ptr as usize, value.as_bytes())
                            .expect("write param value");
                        let mut area = [0u8; 12];
                        area[0] = 1;
                        area[4..8].copy_from_slice(&ptr.to_le_bytes());
                        area[8..12].copy_from_slice(&(value.len() as i32).to_le_bytes());
                        memory
                            .write(&mut caller, retptr as usize, &area)
                            .expect("write return area");
                    }
                    None => {
                        memory
                            .write(&mut caller, retptr as usize, &[0u8; 12])
                            .expect("write none");
                    }
                }
            },
        )
        .expect("get-param stub links");
    linker
        .func_wrap(
            "clean:host/response@0.1.0",
            "set-body",
            |mut caller: wasmtime::Caller<'_, Host>, ptr: i32, len: i32| {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("guest exports memory");
                let mut buf = vec![0u8; len as usize];
                memory
                    .read(&caller, ptr as usize, &mut buf)
                    .expect("read body");
                let body = String::from_utf8(buf).expect("utf8");
                caller.data_mut().log.push(body);
            },
        )
        .expect("set-body stub links");

    // Case 1: the parameter exists.
    let mut store = wasmtime::Store::new(
        &engine,
        Host {
            param: Some("42"),
            log: Vec::new(),
        },
    );
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    let handle = instance
        .get_typed_func::<i64, ()>(&mut store, "handle")
        .expect("handle export");
    handle.call(&mut store, 1).expect("handle runs");
    assert_eq!(store.data().log.as_slice(), ["42".to_string()]);

    // Case 2: no parameter — the `default` fallback flows to set-body.
    let mut store = wasmtime::Store::new(
        &engine,
        Host {
            param: None,
            log: Vec::new(),
        },
    );
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    let handle = instance
        .get_typed_func::<i64, ()>(&mut store, "handle")
        .expect("handle export");
    handle.call(&mut store, 1).expect("handle runs");
    assert_eq!(store.data().log.as_slice(), ["no id".to_string()]);
}

#[test]
fn roundtrip_bytes_return_echoed_back() {
    // /echo shape: get-body returns list<u8> via retptr; the guest passes
    // it straight to set-body.
    let host_bridge = "\
host interface request version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function getBody() returns bytes
\t\tdescription \"The request body.\"

host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function setBody(body: bytes)
\t\tdescription \"Set the response body.\"
";
    let main = "\
functions:
\tvoid handle(integer handlerId)
\t\tbytes body = getBody()
\t\tsetBody(body)
";
    let wasm = compile_to_core(&[("app/host_bridge.cln", host_bridge), ("app/main.cln", main)]);

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<CallLog> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/request@0.1.0",
            "get-body",
            |mut caller: wasmtime::Caller<'_, CallLog>, retptr: i32| {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("guest exports memory");
                let realloc = caller
                    .get_export("cabi_realloc")
                    .and_then(|e| e.into_func())
                    .expect("guest exports cabi_realloc")
                    .typed::<(i32, i32, i32, i32), i32>(&caller)
                    .expect("realloc signature");
                let payload = b"echo me";
                let ptr = realloc
                    .call(&mut caller, (0, 0, 1, payload.len() as i32))
                    .expect("realloc runs");
                memory
                    .write(&mut caller, ptr as usize, payload)
                    .expect("write body");
                let mut area = [0u8; 8];
                area[0..4].copy_from_slice(&ptr.to_le_bytes());
                area[4..8].copy_from_slice(&(payload.len() as i32).to_le_bytes());
                memory
                    .write(&mut caller, retptr as usize, &area)
                    .expect("write return area");
            },
        )
        .expect("get-body stub links");
    linker
        .func_wrap(
            "clean:host/response@0.1.0",
            "set-body",
            |mut caller: wasmtime::Caller<'_, CallLog>, ptr: i32, len: i32| {
                let body = read_string(&mut caller, ptr, len);
                caller.data_mut().push(body);
            },
        )
        .expect("set-body stub links");

    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    instance
        .get_typed_func::<i64, ()>(&mut store, "handle")
        .expect("handle export")
        .call(&mut store, 4)
        .expect("handle runs");
    assert_eq!(store.data().as_slice(), ["echo me".to_string()]);
}

#[test]
fn roundtrip_constant_list_of_records() {
    // /log shape: emit(level, message, fields) with a constant list of
    // field records, serialized to static data in canonical layout.
    let host_bridge = "\
host interface log version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function emit(l: level, message: string, fields: list<Field>)
\t\tdescription \"Emit a structured record.\"
";
    let field_class = "class Field\n\tstring key\n\tstring value\n";
    let main = "\
functions:
\tvoid handle(integer handlerId)
\t\temit(\"info\", \"hello from the guest\", [Field(\"route\", \"log-demo\")])
";
    let wasm = compile_to_core(&[
        ("app/host_bridge.cln", host_bridge),
        ("app/field.cln", field_class),
        ("app/main.cln", main),
    ]);
    wasmparser::validate(&wasm).expect("core module validates");

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<CallLog> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/log@0.1.0",
            "emit",
            |mut caller: wasmtime::Caller<'_, CallLog>,
             level: i32,
             msg_ptr: i32,
             msg_len: i32,
             fields_ptr: i32,
             fields_len: i32| {
                let message = read_string(&mut caller, msg_ptr, msg_len);
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("guest exports memory");
                // Canonical list<record{string,string}>: 16-byte stride.
                let mut fields = Vec::new();
                for i in 0..fields_len {
                    let mut element = [0u8; 16];
                    memory
                        .read(&caller, (fields_ptr + i * 16) as usize, &mut element)
                        .expect("element in bounds");
                    let k_ptr = i32::from_le_bytes(element[0..4].try_into().unwrap());
                    let k_len = i32::from_le_bytes(element[4..8].try_into().unwrap());
                    let v_ptr = i32::from_le_bytes(element[8..12].try_into().unwrap());
                    let v_len = i32::from_le_bytes(element[12..16].try_into().unwrap());
                    let key = read_string(&mut caller, k_ptr, k_len);
                    let value = read_string(&mut caller, v_ptr, v_len);
                    fields.push(format!("{key}={value}"));
                }
                caller
                    .data_mut()
                    .push(format!("emit[{level}] {message} ({})", fields.join(",")));
            },
        )
        .expect("emit stub links");

    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    instance
        .get_typed_func::<i64, ()>(&mut store, "handle")
        .expect("handle export")
        .call(&mut store, 6)
        .expect("handle runs");
    assert_eq!(
        store.data().as_slice(),
        // level enum: trace,debug,info,warn,error → info = 2.
        ["emit[2] hello from the guest (route=log-demo)".to_string()]
    );
}
