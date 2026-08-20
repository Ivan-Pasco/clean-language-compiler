//! Error lowering, stage A (13 §ERH-01/02/04/05): `error(...)` raises
//! through the error channel, `onError` catches any failing expression —
//! not only fallible host calls — the handler binds `error` with
//! `message`/`code`, failures propagate out of callees, and an uncaught
//! failure ends execution at the entry (the RUN018 shape). Verified under
//! wasmtime on the emitted core module.

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{codegen, hir, mir, parser, resolver, typecheck};

mod common;

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
    codegen::core::emit_core(&mir).expect("core emission succeeds")
}

/// Probe + websocket surfaces: `emitInt`/`emitText` record observations;
/// `accept` is the fallible host call (result<u64, socket-error> in the
/// world).
const BRIDGE: &str = "\
host interface websocket version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function accept() returns integer:u64
\t\tdescription \"Accept the pending upgrade.\"

host interface probe version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function emitInt(value: integer)
\t\tdescription \"Record one integer.\"

\thost function emitText(message: string)
\t\tdescription \"Record one string.\"
";

struct Host {
    accept_fails: bool,
    log: Vec<String>,
}

fn read_string(caller: &mut wasmtime::Caller<'_, Host>, ptr: i32, len: i32) -> String {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .expect("memory export");
    let mut buf = vec![0u8; len as usize];
    memory
        .read(&mut *caller, ptr as usize, &mut buf)
        .expect("read string");
    String::from_utf8(buf).expect("utf-8 payload")
}

fn run(body: &str, accept_fails: bool) -> Result<Vec<String>, wasmtime::Error> {
    let main = format!("functions:\n{body}");
    let wasm = compile(&[("app/host_bridge.cln", BRIDGE), ("app/main.cln", &main)]);
    wasmparser::validate(&wasm).expect("core module validates");

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Host> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/websocket@0.1.0",
            "accept",
            |mut caller: wasmtime::Caller<'_, Host>, retptr: i32| {
                let fail = caller.data().accept_fails;
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("memory export");
                let mut area = [0u8; 16];
                if fail {
                    area[0] = 1;
                } else {
                    area[8..16].copy_from_slice(&77u64.to_le_bytes());
                }
                memory
                    .write(&mut caller, retptr as usize, &area)
                    .expect("write result");
                caller.data_mut().log.push(format!("accept(fail={fail})"));
            },
        )
        .expect("links accept");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-int",
            |mut caller: wasmtime::Caller<'_, Host>, value: i64| {
                caller.data_mut().log.push(format!("int({value})"));
            },
        )
        .expect("links emit-int");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-text",
            |mut caller: wasmtime::Caller<'_, Host>, ptr: i32, len: i32| {
                let text = read_string(&mut caller, ptr, len);
                caller.data_mut().log.push(format!("text({text})"));
            },
        )
        .expect("links emit-text");

    let mut store = wasmtime::Store::new(
        &engine,
        Host {
            accept_fails,
            log: Vec::new(),
        },
    );
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())?;
    Ok(store.into_data().log)
}

/// ERH-01/02: a program's own `error(...)` in a callee is caught by the
/// caller's suffix `onError`, which supplies the fallback.
#[test]
fn raised_error_is_caught_by_on_error() {
    let log = run(
        "\tinteger boom()\n\t\terror(\"exploded\")\n\t\treturn 1\n\
         \tvoid init()\n\t\tinteger x = boom() onError 7\n\t\temitInt(x)\n",
        false,
    )
    .expect("caught error does not trap");
    assert_eq!(log, ["int(7)"]);
}

/// The success path is untouched: no failure, no fallback.
#[test]
fn success_path_ignores_the_handler() {
    let log = run(
        "\tinteger fine()\n\t\treturn 41\n\
         \tvoid init()\n\t\tinteger x = fine() onError 7\n\t\temitInt(x + 1)\n",
        false,
    )
    .expect("success path runs");
    assert_eq!(log, ["int(42)"]);
}

/// ERH-05: an uncaught failure ends execution at the entry — the RUN018
/// shape — and the statements after the raise never run.
#[test]
fn uncaught_error_ends_execution() {
    let err = run(
        "\tinteger boom()\n\t\terror(\"exploded\")\n\t\treturn 1\n\
         \tvoid init()\n\t\tinteger x = boom()\n\t\temitInt(x)\n",
        false,
    )
    .expect_err("uncaught failure must end execution");
    assert!(
        err.downcast_ref::<wasmtime::Trap>().is_some(),
        "expected a trap: {err:?}"
    );
}

/// ERH-04: the suffix handler binds `error`, whose `message` is the
/// raise's argument and whose `code` is `none` for a program's own
/// failure (surfaced through `default`).
#[test]
fn error_binding_carries_message_and_none_code() {
    let log = run(
        "\tstring boom()\n\t\terror(\"exploded\")\n\t\treturn \"never\"\n\
         \tvoid init()\n\t\tstring message = boom() onError error.message\n\t\temitText(message)\n\t\tstring code = boom() onError error.code default \"<none>\"\n\t\temitText(code)\n",
        false,
    )
    .expect("suffix handlers bind the failure");
    assert_eq!(log, ["text(exploded)", "text(<none>)"]);
}

/// A raise inside a fallback propagates outward to the next handler
/// (ERH-02: the binding scope is the handler, the failure path continues).
#[test]
fn raise_inside_fallback_propagates_outward() {
    let log = run(
        "\tinteger boomA()\n\t\terror(\"a\")\n\t\treturn 1\n\
         \tinteger boomB()\n\t\terror(\"b\")\n\t\treturn 2\n\
         \tvoid init()\n\t\tinteger x = (boomA() onError boomB()) onError 9\n\t\temitInt(x)\n",
        false,
    )
    .expect("outer handler catches");
    assert_eq!(log, ["int(9)"]);
}

/// LBS §8.3: a bare fallible host call raises through the ordinary
/// chapter-13 path — a caller's `onError` catches it like any failure.
#[test]
fn bare_host_failure_raises_catchably() {
    let log = run(
        "\tinteger wrap()\n\t\tinteger sock = accept()\n\t\treturn sock\n\
         \tvoid init()\n\t\tinteger x = wrap() onError 5\n\t\temitInt(x)\n",
        true,
    )
    .expect("host failure caught by the caller");
    assert_eq!(log, ["accept(fail=true)", "int(5)"]);
}

/// The host's error payload never surfaces (LBS §8.3): the binding a
/// handler sees carries the generic message and no code.
#[test]
fn host_failure_binds_the_generic_message() {
    let log = run(
        "\tinteger report(string m)\n\t\temitText(m)\n\t\treturn 0\n\
         \tvoid init()\n\t\tinteger sock = accept() onError report(error.message)\n\t\temitInt(sock)\n",
        true,
    )
    .expect("handler binds the generic message");
    assert_eq!(
        log,
        [
            "accept(fail=true)",
            "text(host function `accept` failed)",
            "int(0)"
        ]
    );
}

/// An uncaught failure never reaches the statements after the raising
/// call — and the entry trap happens after the raise unwound.
#[test]
fn statements_after_an_uncaught_raise_do_not_run() {
    let err = run(
        "\tvoid init()\n\t\tinteger sock = accept()\n\t\temitInt(sock)\n",
        true,
    )
    .expect_err("bare host failure is uncaught here");
    assert!(err.downcast_ref::<wasmtime::Trap>().is_some());
}

/// ERH-03: arithmetic failure raises RUN003, catchable — division and
/// remainder by zero, with the code surfaced through the binding.
#[test]
fn division_by_zero_raises_run003() {
    let log = run(
        "\tinteger zero()\n\t\treturn 0\n\
         \tvoid init()\n\t\tinteger x = 10 / zero() onError 3\n\t\temitInt(x)\n\t\tinteger y = 10 % zero() onError 4\n\t\temitInt(y)\n",
        false,
    )
    .expect("caught arithmetic failure");
    assert_eq!(log, ["int(3)", "int(4)"]);
}

#[test]
fn division_by_zero_carries_the_run003_code() {
    let log = run(
        "\tinteger zero()\n\t\treturn 0\n\
         \tstring describe()\n\t\tinteger x = 10 / zero()\n\t\treturn \"ok\"\n\
         \tvoid init()\n\t\tstring c = describe() onError error.code default \"<none>\"\n\t\temitText(c)\n",
        false,
    )
    .expect("code surfaces through the binding");
    assert_eq!(log, ["text(RUN003)"]);
}

/// Uncaught division by zero still ends execution (ERH-05).
#[test]
fn uncaught_division_by_zero_ends_execution() {
    let err = run(
        "\tinteger zero()\n\t\treturn 0\n\
         \tvoid init()\n\t\temitInt(10 / zero())\n",
        false,
    )
    .expect_err("uncaught arithmetic failure ends execution");
    assert!(err.downcast_ref::<wasmtime::Trap>().is_some());
}

/// i64.div_s would also trap on MIN / -1: that overflow raises RUN003 too.
#[test]
fn division_overflow_raises_run003() {
    let log = run(
        "\tinteger minusOne()\n\t\treturn 0 - 1\n\
         \tvoid init()\n\t\tinteger min = 0 - 9223372036854775807 - 1\n\t\tinteger x = min / minusOne() onError 6\n\t\temitInt(x)\n",
        false,
    )
    .expect("overflow caught");
    assert_eq!(log, ["int(6)"]);
}

/// ERH-03: number→integer conversion outside the domain raises RUN003 —
/// NaN and out-of-range, both catchable.
#[test]
fn out_of_domain_truncation_raises_run003() {
    let log = run(
        "\tnumber nan()\n\t\treturn 0.0 / 0.0\n\
         \tnumber huge()\n\t\treturn 1.0e300\n\
         \tvoid init()\n\t\tinteger a = nan().toInteger() onError 1\n\t\temitInt(a)\n\t\tinteger b = huge().toInteger() onError 2\n\t\temitInt(b)\n",
        false,
    )
    .expect("domain failures caught");
    assert_eq!(log, ["int(1)", "int(2)"]);
}

/// RUN013 raises catchably on list index out of range, with the Platform
/// 10 template filled at raise time.
#[test]
fn list_index_out_of_range_raises_run013() {
    let log = run(
        "\tinteger pick(list<integer> xs, integer i)\n\t\treturn xs.get(i)\n\
         \tstring describe(list<integer> xs, integer i)\n\t\tinteger v = xs.get(i)\n\t\treturn \"ok\"\n\
         \tvoid init()\n\t\tlist<integer> xs = [10, 20, 30]\n\t\tinteger a = pick(xs, 5) onError 0 - 1\n\t\temitInt(a)\n\t\tstring m = describe(xs, 5) onError error.message\n\t\temitText(m)\n\t\tstring c = describe(xs, 5) onError error.code default \"<none>\"\n\t\temitText(c)\n",
        false,
    )
    .expect("index failure caught");
    assert_eq!(
        log,
        [
            "int(-1)",
            "text(Index 5 is out of range for a list of length 3)",
            "text(RUN013)"
        ]
    );
}

/// RUN013 raises catchably on string code-point index out of range.
#[test]
fn string_index_out_of_range_raises_run013() {
    let log = run(
        "\tstring at(string s, integer i)\n\t\treturn s.charAt(i)\n\
         \tvoid init()\n\t\tstring c = at(\"abc\", 9) onError error.message\n\t\temitText(c)\n",
        false,
    )
    .expect("string index failure caught");
    assert_eq!(
        log,
        ["text(Index 9 is out of range for a string of length 3)"]
    );
}

/// The in-range paths still work after the guards.
#[test]
fn in_range_access_is_unchanged() {
    let log = run(
        "\tvoid init()\n\t\tlist<integer> xs = [10, 20, 30]\n\t\temitInt(xs.get(2))\n\t\temitText(\"abc\".charAt(1))\n",
        false,
    )
    .expect("in-range paths run");
    assert_eq!(log, ["int(30)", "text(b)"]);
}

/// RUN003 raises catchably from the string→number parsers in the runtime.
#[test]
fn string_parse_failure_raises_run003() {
    let log = run(
        "\tinteger parse(string s)\n\t\treturn s.toInteger()\n\
         \tstring describe(string s)\n\t\tinteger v = parse(s)\n\t\treturn \"ok\"\n\
         \tvoid init()\n\t\tinteger x = parse(\"12x\") onError 0 - 2\n\t\temitInt(x)\n\t\tstring c = describe(\"nope\") onError error.code default \"<none>\"\n\t\temitText(c)\n\t\temitInt(parse(\"64\") onError 0 - 3)\n",
        false,
    )
    .expect("parse failure caught");
    assert_eq!(log, ["int(-2)", "text(RUN003)", "int(64)"]);
}

/// 10 §RUN003 (2026-08-20): remainder by zero raises with the SAME string
/// as division — one message for both operations.
#[test]
fn remainder_by_zero_shares_the_division_message() {
    let log = run(
        "\tinteger zero()\n\t\treturn 0\n\
         \tstring describe()\n\t\tinteger x = 10 % zero()\n\t\treturn \"ok\"\n\
         \tvoid init()\n\t\tstring m = describe() onError error.message\n\t\temitText(m)\n",
        false,
    )
    .expect("message surfaces through the binding");
    assert_eq!(log, ["text(division by zero)"]);
}

/// 10 §RUN003 (2026-08-20): integer.MIN % -1 is DEFINED as 0 — wasm
/// rem_s semantics, no raise (only MIN / -1 overflows).
#[test]
fn min_remainder_by_minus_one_is_zero_not_a_raise() {
    let log = run(
        "\tinteger minusOne()\n\t\treturn 0 - 1\n\
         \tvoid init()\n\t\tinteger min = 0 - 9223372036854775807 - 1\n\t\temitInt(min % minusOne())\n",
        false,
    )
    .expect("defined remainder, nothing raised");
    assert_eq!(log, ["int(0)"]);
}

/// 15 §Math (2026-08-20): number `/` is IEEE f64.div and never raises —
/// ±Infinity then falls in 10 §RUN003's "number is out of the integer
/// range" arm when truncated.
#[test]
fn infinite_number_truncation_is_out_of_range() {
    let log = run(
        "\tnumber inf()\n\t\treturn 1.0 / 0.0\n\
         \tstring describe(number n)\n\t\tinteger x = n.toInteger()\n\t\treturn \"ok\"\n\
         \tvoid init()\n\t\tstring a = describe(inf()) onError error.message\n\t\temitText(a)\n\t\tstring b = describe(0.0 - inf()) onError error.message\n\t\temitText(b)\n",
        false,
    )
    .expect("non-finite truncation caught");
    assert_eq!(
        log,
        [
            "text(number is out of the integer range)",
            "text(number is out of the integer range)"
        ]
    );
}

/// 15 §Conversions (2026-08-20): the non-finite spellings are OUTPUT
/// only — string.toNumber("NaN"/"Infinity"/"-Infinity") raises RUN003
/// as an invalid number literal; toString∘toNumber round-trips finite
/// values only.
#[test]
fn non_finite_spellings_are_not_tonumber_input() {
    let log = run(
        "\tstring describe(string s)\n\t\tnumber n = s.toNumber()\n\t\treturn \"ok\"\n\
         \tvoid init()\n\t\tstring a = describe(\"NaN\") onError error.message\n\t\temitText(a)\n\t\tstring b = describe(\"Infinity\") onError error.message\n\t\temitText(b)\n\t\tstring c = describe(\"-Infinity\") onError error.message\n\t\temitText(c)\n",
        false,
    )
    .expect("non-finite spellings rejected");
    assert_eq!(
        log,
        [
            "text(the string is not a valid number literal)",
            "text(the string is not a valid number literal)",
            "text(the string is not a valid number literal)"
        ]
    );
}
