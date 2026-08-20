//! M9 measurement harness for the §14.9 performance targets.
//!
//! The targets are informative (§14.9: "they exist so we notice
//! regressions, not so we ship a benchmark suite"); M9's job is to
//! MEASURE them reproducibly and record the numbers in
//! docs/DISCOVERIES-M9.md — a missed target is documented there, never
//! "optimized" blind. The harness therefore always exits 0.
//!
//! Run with the compiler compiled for real use:
//!
//! ```text
//! cargo run --release -p clean-compiler --example perf_budget
//! ```
//!
//! Methodology (recorded alongside the numbers):
//! - Synthetic projects are generated deterministically in-process: small
//!   (~0.9k LOC, 4 modules) and medium (~16k LOC, 9 modules) per the §14.9
//!   size classes; no library manifests (no released libraries exist yet).
//! - "Cold" cases re-exec this binary per run — each measurement is a
//!   fresh process timing exactly one operation; 5 runs, median reported.
//! - The watch-rebuild case is warm by definition (§14.14.3 keeps the
//!   process alive): compile once, change one module, time the re-run.
//! - `timings` in the build manifest stay zero (CMP-02); all clocks here
//!   live outside the compiler.

use clean_compiler::{check, compile, why};
use clean_compiler_types::request::CompileRequest;
use sha2::{Digest, Sha256};
use std::time::Instant;

const HOST_WIT: &str = include_str!("../../../tests/fixtures/wit/host.wit");

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// One synthetic module: `functions:` / `public:` with numeric bodies that
/// reach codegen (locals, branches, iterate loops, cross-calls).
fn module_source(module: usize, functions: usize) -> String {
    let mut src = String::from("functions:\n\tpublic:\n");
    for f in 0..functions {
        src.push_str(&format!("\t\tnumber mod{module}f{f}(number x)\n"));
        src.push_str(&format!("\t\t\tnumber acc = x + {f}.0\n"));
        src.push_str("\t\t\tif acc > 10.0\n");
        src.push_str("\t\t\t\tacc = acc - 1.0\n");
        src.push_str("\t\t\titerate k in 1 to 3\n");
        src.push_str("\t\t\t\tacc = acc + 1.0\n");
        if f > 0 {
            src.push_str(&format!("\t\t\tacc = acc + mod{module}f{}(0.0)\n", f - 1));
        }
        src.push_str("\t\t\treturn acc\n");
    }
    src
}

/// The §14.9 size classes. `broken` injects one undefined call so the
/// compile rejects — the `why` case needs a real diagnostics set.
fn project(modules: usize, functions_per_module: usize, broken: bool) -> Vec<(String, String)> {
    let mut sources = Vec::new();
    let mut main = String::from("import:\n");
    for m in 0..modules {
        let mut source = module_source(m, functions_per_module);
        if broken {
            // One undefined call per module: the why case measures over a
            // diagnostics set proportional to the project, not a single row.
            source.push_str(&format!(
                "\t\tnumber mod{m}bad(number x)\n\t\t\treturn x + noSuchFunction{m}(x)\n"
            ));
        }
        sources.push((format!("mod{m}.cln"), source));
        main.push_str(&format!("\tmod{m}\n"));
    }
    main.push_str("\nfunctions:\n\tvoid init()\n\t\tnumber acc = 0.0\n");
    for m in 0..modules {
        main.push_str(&format!("\t\tacc = acc + mod{m}f0(acc)\n"));
    }
    if broken {
        main.push_str("\t\tacc = acc + noSuchFunction(acc)\n");
    }
    main.push_str("\t\treturn\n");
    sources.push(("main.cln".to_string(), main));
    sources
}

fn request_for(sources: &[(String, String)]) -> CompileRequest {
    serde_json::from_value(serde_json::json!({
        "spec_version": "1",
        "project": { "name": "perf", "version": "0.0.1" },
        "build": { "target": "wasm32-server", "optimization": "debug" },
        "target_world": {
            "host": "clean-server",
            "version": "0.7.0",
            "world": "server",
            "sha256": sha256_hex(HOST_WIT.as_bytes()),
            "wit": HOST_WIT,
        },
        "sources": sources.iter().map(|(path, content)| serde_json::json!({
            "path": path,
            "sha256": sha256_hex(content.as_bytes()),
            "content": content,
        })).collect::<Vec<_>>(),
    }))
    .expect("request schema")
}

fn request(size: &str, optimization: &str, broken: bool) -> CompileRequest {
    let sources = match size {
        "small" => project(4, 30, broken),
        "medium" => project(9, 250, broken),
        other => panic!("unknown size {other}"),
    };
    let mut request = request_for(&sources);
    request.build.optimization = optimization.to_string();
    request
}

fn loc(sources: &[(String, String)]) -> usize {
    sources.iter().map(|(_, c)| c.lines().count()).sum()
}

/// Child mode: time exactly one operation in a fresh process, print ms.
fn run_cold_case(case: &str) {
    match case {
        "compile-small-debug"
        | "compile-small-release"
        | "compile-medium-debug"
        | "compile-medium-release" => {
            let mut it = case.split('-');
            it.next();
            let size = it.next().expect("size");
            let optimization = it.next().expect("profile");
            let request = request(size, optimization, false);
            let start = Instant::now();
            let artifact = compile(request).expect("synthetic project compiles");
            let elapsed = start.elapsed();
            assert!(!artifact.wasm.is_empty());
            println!("{}", elapsed.as_secs_f64() * 1000.0);
        }
        "check-medium" => {
            let request = request("medium", "debug", false);
            let start = Instant::now();
            let diagnostics = check(request).expect("synthetic project checks");
            let elapsed = start.elapsed();
            assert!(diagnostics.is_empty());
            println!("{}", elapsed.as_secs_f64() * 1000.0);
        }
        "why-medium" => {
            // §14.14.1: re-projection over diagnostics.json, cold. The
            // timed span covers NDJSON parse + report assembly, i.e. the
            // whole operation once the file bytes are in hand.
            let request = request("medium", "debug", true);
            let diagnostics = match compile(request) {
                Err(clean_compiler::CompileError::Rejected(diags)) => diags,
                other => panic!("expected rejection, got {other:?}"),
            };
            let ndjson: String = diagnostics
                .iter()
                .map(|d| serde_json::to_string(d).expect("diagnostic serializes") + "\n")
                .collect();
            let query = why::WhyQuery {
                file: "main.cln".to_string(),
                line: 1,
                column: None,
            };
            let start = Instant::now();
            let parsed = why::diagnostics_from_ndjson(&ndjson).expect("ndjson parses");
            let report = why::why(&parsed, &query);
            let elapsed = start.elapsed();
            assert_eq!(report.query.file, "main.cln");
            println!("{}", elapsed.as_secs_f64() * 1000.0);
        }
        other => panic!("unknown cold case {other}"),
    }
}

/// Warm watch-rebuild (§14.14.3): compile, touch one module, recompile.
fn run_watch_rebuild() -> f64 {
    let mut sources = project(9, 250, false);
    compile(request_for(&sources)).expect("warm-up compiles");
    // The single-file change: one more function in one module.
    sources[0].1 = module_source(0, 251);
    let request = request_for(&sources);
    let start = Instant::now();
    compile(request).expect("rebuild compiles");
    start.elapsed().as_secs_f64() * 1000.0
}

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    samples[samples.len() / 2]
}

/// Parent mode: each cold case re-execs this binary 5 times.
fn measure_cold(case: &str) -> f64 {
    let exe = std::env::current_exe().expect("own path");
    let samples: Vec<f64> = (0..5)
        .map(|_| {
            let output = std::process::Command::new(&exe)
                .arg(case)
                .output()
                .expect("child runs");
            assert!(
                output.status.success(),
                "cold case {case} failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout)
                .expect("utf-8")
                .trim()
                .parse()
                .expect("child prints ms")
        })
        .collect();
    median(samples)
}

fn main() {
    if let Some(case) = std::env::args().nth(1) {
        run_cold_case(&case);
        return;
    }

    println!(
        "perf_budget — §14.9 informative targets, medians of 5 cold runs (warm: 1 run)\n\
         project sizes: small {} LOC / 4 modules, medium {} LOC / 9 modules, 0 libraries\n",
        loc(&project(4, 30, false)),
        loc(&project(9, 250, false)),
    );

    let cases: [(&str, f64, &str); 6] = [
        ("compile-small-debug", 500.0, "§14.9 small cold debug"),
        ("compile-small-release", 1500.0, "§14.9 small cold release"),
        ("compile-medium-debug", 3000.0, "§14.9 medium cold debug"),
        (
            "compile-medium-release",
            10000.0,
            "§14.9 medium cold release",
        ),
        ("check-medium", 300.0, "§14.14.4 check cold"),
        ("why-medium", 100.0, "§14.14.1 why cold"),
    ];

    let mut rows = Vec::new();
    for (case, target_ms, cite) in cases {
        let measured = measure_cold(case);
        rows.push((case.to_string(), target_ms, measured, cite));
    }
    let watch = run_watch_rebuild();
    rows.push((
        "watch-rebuild-medium".to_string(),
        500.0,
        watch,
        "§14.14.3 warm rebuild",
    ));

    let (case_h, target_h, measured_h) = ("case", "target", "measured");
    println!("{case_h:<24} {target_h:>10} {measured_h:>12}  verdict");
    for (case, target, measured, cite) in &rows {
        let verdict = if measured <= target {
            "within"
        } else {
            "MISSED"
        };
        println!("{case:<24} {target:>8.0}ms {measured:>10.1}ms  {verdict} ({cite})");
    }
    println!("\nInformative targets: record these numbers in docs/DISCOVERIES-M9.md.");
}
