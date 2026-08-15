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
    /// written anywhere else).
    #[arg(long)]
    out: PathBuf,
}

fn main() -> ExitCode {
    let args = Args::parse();

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
        return finish_rejected(&args.out, intake.into_diagnostics());
    }
    let request = request.expect("intake produced no request yet raised no error");

    match compile(request) {
        Ok(artifact) => {
            if let Err(err) = write_artifacts(&args.out, &artifact) {
                eprintln!("clean-compiler: cannot write outputs: {err}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(CompileError::Rejected(diagnostics)) => finish_rejected(&args.out, diagnostics),
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
