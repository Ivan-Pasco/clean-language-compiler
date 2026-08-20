//! `number.toString()` (15 §Conversions): the shortest round-trip
//! rendering, computed exactly in the guest. Oracle: Rust's own shortest
//! digit generation (`{:e}`), re-rendered under the pinned notation rules
//! (plain decimal for -4 ≤ E ≤ 21, scientific otherwise, integral values
//! with `.0` — DISCOVERIES-M8).

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{codegen, hir, mir, parser, resolver, typecheck};

mod common;

const BRIDGE: &str = "\
host interface probe version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function nextNumber() returns number
\t\tdescription \"The next corpus value.\"

\thost function emitText(message: string)
\t\tdescription \"Record one string.\"
";

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

struct Host {
    values: Vec<f64>,
    next: usize,
    log: Vec<String>,
}

/// Runs `v.toString()` in the guest for every corpus value, in order.
fn tostring_all(values: &[f64]) -> Vec<String> {
    let main = format!(
        "functions:\n\tvoid init()\n\t\titerate i in 1 to {}\n\t\t\tnumber v = nextNumber()\n\t\t\temitText(v.toString())\n",
        values.len()
    );
    let wasm = compile(&[("app/host_bridge.cln", BRIDGE), ("app/main.cln", &main)]);
    wasmparser::validate(&wasm).expect("core module validates");

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).expect("module loads");
    let mut linker: wasmtime::Linker<Host> = wasmtime::Linker::new(&engine);
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "next-number",
            |mut caller: wasmtime::Caller<'_, Host>| -> f64 {
                let host = caller.data_mut();
                let v = host.values[host.next];
                host.next += 1;
                v
            },
        )
        .expect("links next-number");
    linker
        .func_wrap(
            "clean:host/probe@0.1.0",
            "emit-text",
            |mut caller: wasmtime::Caller<'_, Host>, ptr: i32, len: i32| {
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("memory export");
                let mut buf = vec![0u8; len as usize];
                memory
                    .read(&mut caller, ptr as usize, &mut buf)
                    .expect("read string");
                caller
                    .data_mut()
                    .log
                    .push(String::from_utf8(buf).expect("utf-8"));
            },
        )
        .expect("links emit-text");

    let mut store = wasmtime::Store::new(
        &engine,
        Host {
            values: values.to_vec(),
            next: 0,
            log: Vec::new(),
        },
    );
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiates");
    instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .expect("init export")
        .call(&mut store, ())
        .expect("init runs");
    store.into_data().log
}

/// The oracle: Rust's shortest digits (`{:e}` is shortest round-trip),
/// re-rendered under the pinned notation rules.
fn expected(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    let sign = if v.is_sign_negative() { "-" } else { "" };
    if v.is_infinite() {
        return format!("{sign}Infinity");
    }
    if v == 0.0 {
        return format!("{sign}0.0");
    }
    // "d[.ddd]e±E" → digits + our E (value = 0.digits × 10^E).
    let text = format!("{:e}", v.abs());
    let (mantissa, exp) = text.split_once('e').expect("LowerExp has an exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let e: i32 = exp.parse::<i32>().unwrap() + 1;
    let k = digits.len() as i32;

    if (-4..=21).contains(&e) {
        if e >= k {
            let zeros = "0".repeat((e - k) as usize);
            format!("{sign}{digits}{zeros}.0")
        } else if e >= 1 {
            format!("{sign}{}.{}", &digits[..e as usize], &digits[e as usize..])
        } else {
            let zeros = "0".repeat((-e) as usize);
            format!("{sign}0.{zeros}{digits}")
        }
    } else {
        let head = &digits[..1];
        let tail = &digits[1..];
        let dot = if tail.is_empty() {
            String::new()
        } else {
            format!(".{tail}")
        };
        format!("{sign}{head}{dot}e{}", e - 1)
    }
}

#[allow(clippy::excessive_precision)] // exact-boundary corpus values are spelled in full
fn corpus() -> Vec<f64> {
    let mut values = vec![
        0.1,
        42.0,
        -3.25,
        6.02e23,
        1.0,
        -1.0,
        0.5,
        2.675,
        0.3,
        1.0 / 3.0,
        1e21,
        1e22,
        1e-4,
        1e-5,
        1e300,
        1e-300,
        5e-324,                  // smallest subnormal
        2.2250738585072014e-308, // smallest normal
        f64::MAX,
        f64::MIN_POSITIVE,
        -0.0,
        0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        9.999999999999999e22,
        123456789.123,
        1.5,
        2.0_f64.powi(60),
        2.0_f64.powi(-60),
        1234567890123456.7,
        0.30000000000000004, // 0.1 + 0.2
        8.98846567431158e307,
        4.9406564584124654e-320, // subnormal with structure
    ];
    // Deterministic pseudo-random bit patterns (xorshift), finite only.
    let mut x: u64 = 0x9E3779B97F4A7C15;
    while values.len() < 250 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let v = f64::from_bits(x);
        if v.is_finite() {
            values.push(v);
        }
    }
    values
}

#[test]
fn to_string_is_the_shortest_round_trip_rendering() {
    let values = corpus();
    let rendered = tostring_all(&values);
    assert_eq!(rendered.len(), values.len());
    let mut failures = Vec::new();
    for (v, got) in values.iter().zip(&rendered) {
        let want = expected(*v);
        if *got != want {
            failures.push(format!(
                "{v:?} (bits {:016x}): got `{got}`, want `{want}`",
                v.to_bits()
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} renderings diverge from the oracle:\n{}",
        failures.len(),
        values.len(),
        failures.join("\n")
    );
}

/// The spec's own examples (15 §Conversions), byte for byte.
#[test]
fn spec_examples_render_exactly() {
    let rendered = tostring_all(&[0.1, 42.0, -3.25, 6.02e23]);
    assert_eq!(rendered, ["0.1", "42.0", "-3.25", "6.02e23"]);
}
