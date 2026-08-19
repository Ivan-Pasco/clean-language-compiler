//! Process adapter (Platform 14 §14.2.2): a thin transport over
//! `clean_compiler::compile`. It reads the request document, invokes the
//! library, and writes the outputs — no compilation logic of its own. If a
//! bug appears only here and not through the library API, the bug is in this
//! adapter.
//!
//! This binary is dispatched by Clean Framework / Clean Manager; it is not a
//! user-facing command (CCMP-04 — every developer-visible verb belongs to
//! `cln`).

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{compile, CompileError};

#[derive(Parser)]
#[command(name = "clean-compiler", version, about)]
struct Args {
    /// Path to the request document JSON; `-` reads stdin.
    #[arg(long, default_value = "-")]
    request: String,
    /// Directory the artifact set is written into (CMP-05: nothing is
    /// written anywhere else). Required by every operation except `--why`,
    /// which writes nothing.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Diagnostics-only build (Platform 14 §14.14.4, the operation behind
    /// `cln check`): run passes 1–9 and write `diagnostics.json` only —
    /// never `component.wasm`. Exit 0 when no diagnostic is an error.
    #[arg(long)]
    check: bool,
    /// Intermediate-representation emission (M4, the AI-review surface).
    /// The only value is `hir-json`: run passes 1–7 and write `hir.json`
    /// plus `diagnostics.json` — never `component.wasm`.
    #[arg(long)]
    emit: Option<String>,
    /// Diagnostic re-projection (Platform 14 §14.14.1, the operation behind
    /// `cln why`): `<file>:<line>[:<column>]`. Re-presents the diagnostics
    /// at that location from `--diagnostics` — no compilation, no request
    /// document. Prints the report JSON to stdout.
    #[arg(long, requires = "diagnostics", conflicts_with_all = ["check", "emit"])]
    why: Option<String>,
    /// The `diagnostics.json` (NDJSON) of the most recent build, as the
    /// caller recorded it. Only read by `--why` (CMP-01: the compiler
    /// discovers nothing — the caller says exactly which file).
    #[arg(long)]
    diagnostics: Option<PathBuf>,
    /// Build reproduction (Platform 14 §14.14.6, the operation behind
    /// `cln repro build`): path to the `build-manifest.json` to reproduce.
    /// Inputs are resolved from the request document (`--request`), every
    /// hash re-verified; the reproduced `component.wasm` is written to
    /// `--out` only when it hashes to the manifest's record.
    #[arg(long, conflicts_with_all = ["check", "emit", "why"])]
    repro_build: Option<PathBuf>,
    /// The originally shipped `component.wasm`, if the caller still has it:
    /// enables first-divergent-byte reporting on mismatch (the manifest
    /// records the artifact only by hash).
    #[arg(long, requires = "repro_build")]
    original: Option<PathBuf>,
    /// Request replay (Platform 14 §14.14.6, the replay half of `cln
    /// repro`): path to the request-trace JSON. Re-runs the captured
    /// request against `--component` with every host import served from
    /// the trace; prints the replay report to stdout. No request document,
    /// no output directory.
    #[arg(long, requires = "component", conflicts_with_all = ["check", "emit", "why", "repro_build"])]
    replay: Option<PathBuf>,
    /// The `component.wasm` the trace was captured against (verified by
    /// hash). Only read by `--replay`.
    #[arg(long)]
    component: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if let Some(location) = &args.why {
        return run_why(
            location,
            args.diagnostics.as_deref().expect("clap requires"),
        );
    }

    if let Some(trace_path) = &args.replay {
        return run_replay(
            trace_path,
            args.component.as_deref().expect("clap requires"),
        );
    }

    let Some(out) = args.out.clone() else {
        eprintln!("clean-compiler: --out is required for this operation");
        return ExitCode::from(2);
    };

    let json = match read_request(&args.request) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("clean-compiler: cannot read request: {err}");
            return ExitCode::from(2);
        }
    };

    let mut intake = DiagnosticSink::new();
    let request = clean_compiler::request::from_json(&json, &mut intake);
    if intake.has_errors() {
        // Intake failures precede any source, so they render unquoted.
        let diagnostics = clean_compiler::diag::finalize(
            intake.into_diagnostics(),
            &clean_compiler::diag::SourceCache::empty(),
        );
        return finish_rejected(&out, diagnostics);
    }
    let request = request.expect("intake produced no request yet raised no error");

    if let Some(manifest_path) = &args.repro_build {
        return run_repro(manifest_path, args.original.as_deref(), request, &out);
    }

    if let Some(emit) = &args.emit {
        if emit != "hir-json" {
            eprintln!("clean-compiler: unknown --emit target `{emit}` (expected `hir-json`)");
            return ExitCode::from(2);
        }
        return match clean_compiler::emit_hir(request) {
            Ok((json, diagnostics)) => {
                if let Err(err) = write_diagnostics(&out, &diagnostics) {
                    eprintln!("clean-compiler: cannot write diagnostics: {err}");
                    return ExitCode::from(2);
                }
                let failed = diagnostics
                    .iter()
                    .any(|d| d.level == clean_compiler::types::Level::Error);
                if failed {
                    return ExitCode::FAILURE;
                }
                if let Err(err) = std::fs::write(out.join("hir.json"), json) {
                    eprintln!("clean-compiler: cannot write hir.json: {err}");
                    return ExitCode::from(2);
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("clean-compiler: {err}");
                ExitCode::from(3)
            }
        };
    }

    if args.check {
        return match clean_compiler::check(request) {
            Ok(diagnostics) => {
                if let Err(err) = write_diagnostics(&out, &diagnostics) {
                    eprintln!("clean-compiler: cannot write diagnostics: {err}");
                    return ExitCode::from(2);
                }
                let failed = diagnostics
                    .iter()
                    .any(|d| d.level == clean_compiler::types::Level::Error);
                if failed {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            // Pre-v1 channels (Unsupported/Incomplete): same non-rejection
            // exit as the build path uses.
            Err(err) => {
                eprintln!("clean-compiler: {err}");
                ExitCode::from(3)
            }
        };
    }

    match compile(request) {
        Ok(artifact) => {
            if let Err(err) = write_artifacts(&out, &artifact) {
                eprintln!("clean-compiler: cannot write outputs: {err}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(CompileError::Rejected(diagnostics)) => finish_rejected(&out, diagnostics),
        Err(err @ CompileError::Unsupported(_)) => {
            // Pre-v1 state: valid program, construct outside the milestone
            // surface. Same non-rejection exit as an incomplete pipeline.
            eprintln!("clean-compiler: {err}");
            ExitCode::from(3)
        }
        Err(err @ CompileError::Incomplete { .. }) => {
            // Pre-v1 state: the request was valid but the pipeline cannot yet
            // produce a component. Distinct exit code so callers never
            // mistake it for a rejection.
            eprintln!("clean-compiler: {err}");
            ExitCode::from(3)
        }
    }
}

/// The `--repro-build` operation (Platform 14 §14.14.6): reproduce a build
/// from its manifest, serving inputs from the request document on stdin /
/// `--request`. Success writes the artifact set (the component is
/// byte-identical to the shipped one); a divergence or corruption writes a
/// COM013 `diagnostics.json` and exits 1 with no component (CMP-05); a
/// missing input or incomplete manifest is transport failure, exit 2.
fn run_repro(
    manifest_path: &std::path::Path,
    original_path: Option<&std::path::Path>,
    request: clean_compiler::types::CompileRequest,
    out: &PathBuf,
) -> ExitCode {
    let manifest: clean_compiler::types::BuildManifest =
        match std::fs::read_to_string(manifest_path)
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str(&text).map_err(|e| e.to_string()))
        {
            Ok(manifest) => manifest,
            Err(err) => {
                eprintln!(
                    "clean-compiler: cannot read build manifest {}: {err}",
                    manifest_path.display()
                );
                return ExitCode::from(2);
            }
        };
    let original = match original_path {
        Some(path) => match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(err) => {
                eprintln!("clean-compiler: cannot read {}: {err}", path.display());
                return ExitCode::from(2);
            }
        },
        None => None,
    };

    let resolver = clean_compiler::RequestDocumentResolver::new(&request);
    match clean_compiler::repro_build(&manifest, original.as_deref(), &resolver) {
        Ok(artifact) => {
            if let Err(err) = write_artifacts(out, &artifact) {
                eprintln!("clean-compiler: cannot write outputs: {err}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(failure) => {
            if let Some(diagnostic) = clean_compiler::repro::divergence_diagnostic(&failure) {
                let diagnostics = clean_compiler::diag::finalize(
                    vec![diagnostic],
                    &clean_compiler::diag::SourceCache::empty(),
                );
                return finish_rejected(out, diagnostics);
            }
            match failure {
                clean_compiler::ReproFailure::Compile(CompileError::Rejected(diagnostics)) => {
                    finish_rejected(out, diagnostics)
                }
                clean_compiler::ReproFailure::Compile(err) => {
                    eprintln!("clean-compiler: {err}");
                    ExitCode::from(3)
                }
                err => {
                    eprintln!("clean-compiler: {err}");
                    ExitCode::from(2)
                }
            }
        }
    }
}

/// The `--why` operation (Platform 14 §14.14.1): parse the location, read
/// the caller-named diagnostics NDJSON, re-project, print the report JSON.
/// Exit 0 even when no diagnostic covers the location — an empty `entries`
/// list is the answer, not a failure; exit 2 is transport failure only.
/// The `--replay` operation (Platform 14 §14.14.6): parse and validate the
/// trace, re-run it against the component, print the report JSON. Exit 0
/// on a byte-for-byte match, 1 on divergence or mismatch (a Clean runtime
/// bug or a trace corruption — never "close enough"), 2 on transport
/// failure.
fn run_replay(trace_path: &std::path::Path, component_path: &std::path::Path) -> ExitCode {
    let trace = match std::fs::read_to_string(trace_path)
        .map_err(|e| e.to_string())
        .and_then(|text| clean_compiler::replay::trace_from_json(&text))
    {
        Ok(trace) => trace,
        Err(err) => {
            eprintln!(
                "clean-compiler: cannot read trace {}: {err}",
                trace_path.display()
            );
            return ExitCode::from(2);
        }
    };
    let component = match std::fs::read(component_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!(
                "clean-compiler: cannot read {}: {err}",
                component_path.display()
            );
            return ExitCode::from(2);
        }
    };
    match clean_compiler::replay::replay(&trace, &component) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report serializes")
            );
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("clean-compiler: {failure}");
            ExitCode::FAILURE
        }
    }
}

fn run_why(location: &str, diagnostics_path: &std::path::Path) -> ExitCode {
    let query = match parse_location(location) {
        Ok(query) => query,
        Err(err) => {
            eprintln!("clean-compiler: invalid --why location `{location}`: {err}");
            return ExitCode::from(2);
        }
    };
    let text = match std::fs::read_to_string(diagnostics_path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!(
                "clean-compiler: cannot read {}: {err}",
                diagnostics_path.display()
            );
            return ExitCode::from(2);
        }
    };
    let diagnostics = match clean_compiler::why::diagnostics_from_ndjson(&text) {
        Ok(diagnostics) => diagnostics,
        Err(err) => {
            eprintln!(
                "clean-compiler: {} is not diagnostics NDJSON: {err}",
                diagnostics_path.display()
            );
            return ExitCode::from(2);
        }
    };
    let report = clean_compiler::why(&diagnostics, &query);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report serializes")
    );
    ExitCode::SUCCESS
}

/// `<file>:<line>[:<column>]`. The file part may itself contain `:` (never
/// in practice — request paths are forward-slashed and relative — but the
/// parse is right-anchored so it costs nothing to be correct).
fn parse_location(location: &str) -> Result<clean_compiler::WhyQuery, String> {
    let mut parts = location.rsplitn(3, ':');
    let last = parts.next().ok_or("empty location")?;
    let middle = parts.next().ok_or("expected <file>:<line>[:<column>]")?;
    let (file, line, column) = match parts.next() {
        Some(file) => (
            file,
            middle.parse::<u32>().map_err(|_| "line is not a number")?,
            Some(last.parse::<u32>().map_err(|_| "column is not a number")?),
        ),
        None => (
            middle,
            last.parse::<u32>().map_err(|_| "line is not a number")?,
            None,
        ),
    };
    if file.is_empty() {
        return Err("file part is empty".to_string());
    }
    if line == 0 || column == Some(0) {
        return Err("lines and columns are 1-based".to_string());
    }
    Ok(clean_compiler::WhyQuery {
        file: file.to_string(),
        line,
        column,
    })
}

fn read_request(source: &str) -> std::io::Result<String> {
    if source == "-" {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        std::fs::read_to_string(source)
    }
}

/// Failure output contract (Platform 14 §14.1.2): `diagnostics.json` is
/// written, exit code is 1, and no partial `component.wasm` exists.
fn finish_rejected(out: &PathBuf, diagnostics: Vec<clean_compiler::types::Diagnostic>) -> ExitCode {
    if let Err(err) = write_diagnostics(out, &diagnostics) {
        eprintln!("clean-compiler: cannot write diagnostics: {err}");
        return ExitCode::from(2);
    }
    ExitCode::FAILURE
}

/// NDJSON — one diagnostic object per line (Platform 13 §6), in emission
/// order (§10.3).
fn write_diagnostics(
    out: &PathBuf,
    diagnostics: &[clean_compiler::types::Diagnostic],
) -> std::io::Result<()> {
    std::fs::create_dir_all(out)?;
    let mut lines = String::new();
    for diagnostic in diagnostics {
        lines.push_str(&serde_json::to_string(diagnostic).expect("diagnostic serializes"));
        lines.push('\n');
    }
    std::fs::write(out.join("diagnostics.json"), lines)
}

fn write_artifacts(
    out: &PathBuf,
    artifact: &clean_compiler::CompileArtifact,
) -> std::io::Result<()> {
    std::fs::create_dir_all(out)?;
    std::fs::write(out.join("component.wasm"), &artifact.wasm)?;
    std::fs::write(
        out.join("build-manifest.json"),
        serde_json::to_string_pretty(&artifact.manifest).expect("manifest serializes"),
    )?;
    write_diagnostics(out, &artifact.diagnostics)?;
    if let Some(source_map) = &artifact.source_map {
        std::fs::write(
            out.join("source-map.json"),
            serde_json::to_string(source_map).expect("source map serializes"),
        )?;
    }
    Ok(())
}
