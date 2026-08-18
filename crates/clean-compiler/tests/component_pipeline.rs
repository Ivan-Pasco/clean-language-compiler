//! Milestone 1 step 8 checks (brief acceptance checks 1, 2, 2b, 5): the
//! full acceptance-guest program compiles through `compile()` to a
//! component whose imports are interface-qualified, which conforms to the
//! published server world (`wasm-tools component targets`), and whose
//! three artifacts are byte-deterministic.

use clean_compiler::compile;

mod common;

const HOST_BRIDGE: &str = "\
import:
\tclasses

host interface routing version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function register(m: method, path: string, handlerId: integer:u32, opts: options)
\t\tdescription \"Register one route.\"

host interface request version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function getParam(name: string) returns string?
\t\tdescription \"One path parameter by name.\"

\thost function getBody() returns bytes
\t\tdescription \"The request body.\"

host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function setStatus(status: integer:u16)
\t\tdescription \"Set the status code.\"

\thost function addHeader(name: string, value: string)
\t\tdescription \"Append a response header.\"

\thost function setBody(body: bytes)
\t\tdescription \"Set the response body.\"

host interface log version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function emit(l: level, message: string, fields: list<Field>)
\t\tdescription \"Emit a structured record.\"
";

const CLASSES: &str = "\
class Options
\tboolean csrf

class Field
\tstring key
\tstring value
";

/// The 9a routes of the acceptance guest (SSE/WS/counter are 9b, M6).
const MAIN: &str = "\
import:
\tclasses

functions:
\tvoid init()
\t\tregister(\"get\", \"/\", 0, Options(true))
\t\tregister(\"get\", \"/users/:id\", 1, Options(true))
\t\tregister(\"post\", \"/echo\", 4, Options(true))
\t\tregister(\"post\", \"/hook\", 7, Options(false))
\t\tregister(\"get\", \"/log\", 6, Options(true))

\tvoid handle(integer handlerId)
\t\tif handlerId == 0
\t\t\tsetStatus(200)
\t\t\taddHeader(\"content-type\", \"text/plain; charset=utf-8\")
\t\t\tsetBody(\"hello world\")
\t\telse if handlerId == 1
\t\t\tstring id = getParam(\"id\") default \"no id\"
\t\t\tsetStatus(200)
\t\t\tsetBody(id)
\t\telse if handlerId == 4
\t\t\tbytes body = getBody()
\t\t\tsetStatus(200)
\t\t\tsetBody(body)
\t\telse if handlerId == 7
\t\t\tsetStatus(200)
\t\t\tsetBody(\"hook received\")
\t\telse if handlerId == 6
\t\t\temit(\"info\", \"hello from the guest\", [Field(\"route\", \"log-demo\")])
\t\t\tsetStatus(200)
\t\t\tsetBody(\"logged\")
\t\telse
\t\t\tsetStatus(404)
";

fn acceptance_request() -> clean_compiler_types::CompileRequest {
    let mut request = common::minimal_valid_request();
    request.sources = [
        ("app/host_bridge.cln", HOST_BRIDGE),
        ("app/classes.cln", CLASSES),
        ("app/main.cln", MAIN),
    ]
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
}

#[test]
fn acceptance_guest_compiles_to_a_conforming_component() {
    let artifact = compile(acceptance_request()).expect("acceptance guest compiles");

    // Check 1: component header.
    assert_eq!(
        &artifact.wasm[..8],
        &[0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00]
    );
    // Validates as a component.
    wasmparser::Validator::new()
        .validate_all(&artifact.wasm)
        .expect("component validates");

    // Checks 2 and 2b need the wasm-tools CLI; CI installs it, and local
    // runs use the one the brief verified on this machine.
    let wasm_tools = which_wasm_tools();
    let Some(wasm_tools) = wasm_tools else {
        eprintln!("wasm-tools not found on PATH; skipping CLI cross-checks");
        return;
    };
    let dir = std::env::temp_dir().join(format!("clean-compiler-accept-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let wasm_path = dir.join("app.wasm");
    std::fs::write(&wasm_path, &artifact.wasm).unwrap();

    // Check 2: imports are interface-qualified.
    let wit = std::process::Command::new(&wasm_tools)
        .args(["component", "wit"])
        .arg(&wasm_path)
        .output()
        .expect("wasm-tools runs");
    assert!(
        wit.status.success(),
        "{}",
        String::from_utf8_lossy(&wit.stderr)
    );
    let wit_text = String::from_utf8_lossy(&wit.stdout).to_string();
    assert!(
        wit_text.contains("clean:host/"),
        "component wit must show clean:host/ imports:\n{wit_text}"
    );

    // Check 2b: conformance, stronger than grep. Discovery for the brief:
    // `component targets host.wit -w server` cannot hold for a *guest* —
    // `server` is the world the HOST implements (it exports the interfaces
    // and imports init/handle). The guest targets the mirror world, exactly
    // as clean-server's own fake-guest does (`clean:guest/app` with
    // host.wit as a dep); the host-side gate is its Moment 3 check.
    let wit_dir = dir.join("wit");
    std::fs::create_dir_all(wit_dir.join("deps")).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/wit/host.wit"),
        wit_dir.join("deps/host.wit"),
    )
    .unwrap();
    let host_wit_text =
        std::fs::read_to_string(wit_dir.join("deps/host.wit")).expect("host wit readable");
    let request = acceptance_request();
    let mut sink = clean_compiler::diag::DiagnosticSink::new();
    let validated = clean_compiler::request::validate(request, &mut sink).expect("validates");
    let files = validated
        .request
        .sources
        .iter()
        .map(|s| {
            let stream = clean_compiler::lexer::lex(&s.path, &s.content, &mut sink);
            let ast = clean_compiler::parser::parse(&stream, &mut sink);
            clean_compiler::resolver::ParsedFile { ast, stream }
        })
        .collect();
    let resolved = clean_compiler::resolver::resolve(files, &[], &mut sink);
    let typed = clean_compiler::typecheck::check(&resolved, &validated.world, &mut sink);
    let hir = clean_compiler::hir::lower(typed);
    let mir = clean_compiler::mir::lower(
        &hir,
        &resolved,
        &validated.world.package_version(),
        clean_compiler::layout::tier("standard").expect("standard tier exists"),
        &mut sink,
    );
    let guest_world = clean_compiler::codegen::component::synthesize_guest_world(
        &mir,
        &validated.world.package_version(),
    );
    assert!(host_wit_text.contains("world server"));
    std::fs::write(wit_dir.join("guest.wit"), &guest_world).unwrap();

    let targets = std::process::Command::new(&wasm_tools)
        .args(["component", "targets", "-w", "clean:guest/app"])
        .arg(&wit_dir)
        .arg(&wasm_path)
        .output()
        .expect("wasm-tools runs");
    assert!(
        targets.status.success(),
        "component must target the guest world:\n{}\nguest world was:\n{guest_world}",
        String::from_utf8_lossy(&targets.stderr)
    );
}

#[test]
fn all_three_artifacts_are_byte_deterministic() {
    // Brief acceptance check 5 / CMP-02: same request in, byte-identical
    // component, manifest, and diagnostics out.
    let a = compile(acceptance_request()).expect("compiles");
    let b = compile(acceptance_request()).expect("compiles");
    assert_eq!(a.wasm, b.wasm, "component.wasm must be byte-identical");
    assert_eq!(
        serde_json::to_string(&a.manifest).unwrap(),
        serde_json::to_string(&b.manifest).unwrap(),
        "build-manifest.json must be byte-identical"
    );
    assert_eq!(
        serde_json::to_string(&a.diagnostics).unwrap(),
        serde_json::to_string(&b.diagnostics).unwrap(),
        "diagnostics.json must be byte-identical"
    );
}

#[test]
fn artifacts_stay_byte_deterministic_with_block_handlers() {
    // The M5 leg of CMP-02: handler expansion is inside the determinism
    // boundary — same request with a `library_manifests` handler ⇒
    // byte-identical component, manifest, and diagnostics.
    let handler_request = || {
        let envelope = r#"{"ir":{"kind":"function","name":"answer","params":[],"return":{"kind":"integer"},"body":{"kind":"return","expression":{"kind":"literal_integer","value":42}}},"diagnostics":[]}"#;
        let content = "data UserData:\n\tinteger id primary\n\nfunctions:\n\tvoid init()\n\t\tinteger x = answer()\n\t\treturn\n";
        let mut request = common::minimal_valid_request();
        request.sources[0].content = content.to_string();
        request.sources[0].sha256 = common::sha256_hex(content.as_bytes());
        request.dependencies.insert(
            "alpha".to_string(),
            clean_compiler_types::request::Dependency {
                version: "1.0.0".to_string(),
                resolved_from: "registry".to_string(),
            },
        );
        request
            .folders
            .insert("app".to_string(), vec!["alpha".to_string()]);
        request.library_manifests = vec![common::handler_manifest("alpha", &["data"], envelope)];
        request
    };
    let a = compile(handler_request()).expect("expanded program compiles");
    let b = compile(handler_request()).expect("expanded program compiles");
    assert_eq!(a.wasm, b.wasm, "component.wasm must be byte-identical");
    assert_eq!(
        serde_json::to_string(&a.manifest).unwrap(),
        serde_json::to_string(&b.manifest).unwrap(),
        "build-manifest.json must be byte-identical"
    );
    assert_eq!(
        serde_json::to_string(&a.diagnostics).unwrap(),
        serde_json::to_string(&b.diagnostics).unwrap(),
        "diagnostics.json must be byte-identical"
    );
}

fn which_wasm_tools() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|p| p.join("wasm-tools"))
        .find(|p| p.is_file())
}

/// Regression (M4, §14.7): assigning a plain value to an optional local
/// takes the TYP-03 wrap — MIR emits the discriminant then the payload, so
/// the operand stack balances against the local's flattened width. The M1
/// checker accepted `string? x = "hi"` without materialising the wrap,
/// which would have emitted a 2-value payload into a 3-slot local.
#[test]
fn optional_local_from_plain_value_compiles_and_validates() {
    let content = "functions:\n\tvoid init()\n\t\tstring? x = \"hi\"\n\t\treturn\n";
    let mut request = common::minimal_valid_request();
    request.sources[0].content = content.to_string();
    request.sources[0].sha256 = common::sha256_hex(content.as_bytes());
    let artifact = compile(request).expect("optional local compiles");
    assert!(artifact.diagnostics.is_empty());
    let mut validator = wasmparser::Validator::new();
    validator
        .validate_all(&artifact.wasm)
        .expect("component with an optional local passes validation");
}
