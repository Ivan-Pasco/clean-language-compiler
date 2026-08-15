//! Milestone 1 step 5d checks: a scalar-only program lowers through
//! HIR/MIR to a valid core module whose imports are interface-qualified,
//! whose entry points export, and which actually runs under wasmtime with
//! a recording host stub.

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{codegen, hir, mir, parser, resolver, typecheck};

mod common;

const HOST_BRIDGE: &str = "\
host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function setStatus(status: integer:u16)
\t\tdescription \"Set the status code.\"
";

const MAIN: &str = "\
functions:
\tvoid init()
\t\treturn
\tvoid handle(integer handlerId)
\t\tif handlerId == 0 or handlerId == 6
\t\t\tsetStatus(200)
\t\telse if handlerId == 7
\t\t\tsetStatus(201)
\t\telse
\t\t\tsetStatus(404)
";

/// Runs the pipeline passes [2]..[8] plus core emission directly (the
/// driver withholds artifacts until the component wrap exists).
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

#[test]
fn scalar_program_emits_valid_core_module_with_qualified_imports() {
    let wasm = compile_to_core(&[("app/host_bridge.cln", HOST_BRIDGE), ("app/main.cln", MAIN)]);

    wasmparser::validate(&wasm).expect("core module validates");

    let mut import_names = Vec::new();
    let mut export_names = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&wasm) {
        match payload.expect("parses") {
            wasmparser::Payload::ImportSection(imports) => {
                for group in imports {
                    for entry in group.expect("import group parses") {
                        let (_, import) = entry.expect("import entry parses");
                        import_names.push(format!("{}::{}", import.module, import.name));
                    }
                }
            }
            wasmparser::Payload::ExportSection(exports) => {
                for export in exports {
                    export_names.push(export.expect("export parses").name.to_string());
                }
            }
            _ => {}
        }
    }
    assert_eq!(
        import_names,
        ["clean:host/response@0.1.0::set-status"],
        "imports carry the interface-qualified name, never a flat namespace"
    );
    assert!(export_names.contains(&"init".to_string()));
    assert!(export_names.contains(&"handle".to_string()));
    assert!(export_names.contains(&"memory".to_string()));
}

#[test]
fn dispatch_runs_under_wasmtime_and_reaches_the_host_stub() {
    let wasm = compile_to_core(&[("app/host_bridge.cln", HOST_BRIDGE), ("app/main.cln", MAIN)]);

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Vec<i32>> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/response@0.1.0",
            "set-status",
            |mut caller: wasmtime::Caller<'_, Vec<i32>>, status: i32| {
                caller.data_mut().push(status);
            },
        )
        .expect("stub links");

    let mut store = wasmtime::Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    let handle = instance
        .get_typed_func::<i64, ()>(&mut store, "handle")
        .expect("handle export");

    for (handler_id, expected_status) in [(0, 200), (6, 200), (7, 201), (3, 404)] {
        handle.call(&mut store, handler_id).expect("handle runs");
        assert_eq!(
            store.data().last().copied(),
            Some(expected_status),
            "handler {handler_id} must set status {expected_status}"
        );
    }
}

#[test]
fn core_emission_is_deterministic() {
    let sources = [("app/host_bridge.cln", HOST_BRIDGE), ("app/main.cln", MAIN)];
    assert_eq!(
        compile_to_core(&sources),
        compile_to_core(&sources),
        "same request in, byte-identical core module out (CMP-02)"
    );
}
