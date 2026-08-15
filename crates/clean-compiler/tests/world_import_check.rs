//! Milestone 1 step 7 checks: pass [9] emits COM012 at the offending call
//! site and aborts before codegen (CMP-03) — for functions the world lacks
//! and for signature mismatches; conforming programs pass untouched.

use clean_compiler::{compile, CompileError};
use clean_compiler_types::codes;

mod common;

fn compile_sources(
    sources: &[(&str, &str)],
) -> Result<clean_compiler::CompileArtifact, CompileError> {
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
    compile(request)
}

#[test]
fn com012_names_function_and_world_and_aborts() {
    // The exact fixture named by the brief's acceptance check 3.
    let fixture = include_str!("../../../tests/cln/component/import-not-in-world.cln");
    let result = compile_sources(&[("tests/cln/component/import-not-in-world.cln", fixture)]);
    let Err(CompileError::Rejected(diagnostics)) = result else {
        panic!("expected COM012 rejection, got {result:?}");
    };
    let d = &diagnostics[0];
    assert_eq!(d.code, codes::COM012);
    assert_eq!(
        d.message,
        "your program uses 'clean:host/dom#set-inner-html' but you compiled for 'server', which does not provide it"
    );
    // The span is the call site, inside init().
    assert!(
        d.primary_span.start.line > 10,
        "span was {:?}",
        d.primary_span
    );
}

#[test]
fn com012_on_signature_mismatch_with_existing_function() {
    // set-status exists in the server world, but with (status: u16) —
    // declaring it with a string parameter is a signature miss.
    let host_bridge = "\
host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function setStatus(status: string)
\t\tdescription \"Wrong signature on purpose.\"
";
    let main = "\
functions:
\tvoid init()
\t\tsetStatus(\"200\")
";
    let result = compile_sources(&[("app/host_bridge.cln", host_bridge), ("app/main.cln", main)]);
    let Err(CompileError::Rejected(diagnostics)) = result else {
        panic!("expected COM012 rejection, got {result:?}");
    };
    assert_eq!(diagnostics[0].code, codes::COM012);
    assert!(diagnostics[0]
        .message
        .contains("clean:host/response#set-status"));
}

#[test]
fn declared_but_uncalled_missing_function_does_not_fire() {
    // COM012 is a call-site check: an unused declaration outside the world
    // compiles (the emitted component simply never imports it — step 8
    // emits only what is called).
    let host_bridge = "\
host interface dom version \"0.1.0\":
\trequires host worlds [\"browser\"]

\thost function setInnerHTML(selector: string, html: string)
\t\tdescription \"Not provided by the server world, and never called.\"
";
    let main = "\
functions:
\tvoid init()
\t\treturn
";
    match compile_sources(&[("app/host_bridge.cln", host_bridge), ("app/main.cln", main)]) {
        Ok(_) | Err(CompileError::Incomplete { .. }) => {}
        other => panic!("uncalled declaration must not reject: {other:?}"),
    }
}

#[test]
fn conforming_program_passes_the_world_check() {
    let host_bridge = "\
host interface response version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function setStatus(status: integer:u16)
\t\tdescription \"Set the status code.\"
";
    let main = "\
functions:
\tvoid handle(integer handlerId)
\t\tsetStatus(200)
";
    match compile_sources(&[("app/host_bridge.cln", host_bridge), ("app/main.cln", main)]) {
        Ok(_) | Err(CompileError::Incomplete { .. }) => {}
        other => panic!("conforming program must pass: {other:?}"),
    }
}
