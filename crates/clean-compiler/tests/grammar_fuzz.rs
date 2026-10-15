//! M9 grammar fuzz — programs generated from the Lark grammar of Clean.
//!
//! The grammar (`grammar/*.md`, derived from foundation's examples and held
//! to them by `tools/grammar/check.py`; see `grammar/README.md`) seeds a
//! deterministic generator; every generated program goes through
//! `compile()` under the compiler's external contract:
//!
//! - no panic ever escapes (an escaped panic is an ICE — CMP-04 says
//!   internal failures are COM013, and no input may produce one);
//! - `COM013` never appears in diagnostics (no input is allowed to break a
//!   codegen invariant — `tests/cln/diagnostics/unimplemented.txt` pins that
//!   the code has no reproducing input);
//! - every emitted diagnostic carries a registered code (DIA-01);
//! - anything that compiles emits a component `wasmparser` validates;
//! - a byte-identical request recompiles byte-identically (CMP-02).
//!
//! Failures print the seed and the generated program; the run reproduces
//! from the seed alone. `GRAMMAR_FUZZ_SEED` / `GRAMMAR_FUZZ_COUNT` widen the
//! sweep in the nightly job without touching the checked-in defaults.

mod common;
#[path = "grammar_fuzz/generate.rs"]
mod generate;
#[path = "grammar_fuzz/lark.rs"]
mod lark;

use clean_compiler::{compile, CompileError};
use clean_compiler_types::codes;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn generator() -> generate::Generator {
    generate::Generator::new(lark::Grammar::load(&lark::chapter_files()))
}

/// Every chapter in `grammar/` is in the composition order, and every
/// chapter of the order is on disk: a chapter left out would silently
/// under-cover the language.
#[test]
fn composition_lists_every_chapter() {
    let mut on_disk: Vec<String> = std::fs::read_dir(lark::grammar_dir())
        .expect("grammar/ directory exists")
        .map(|e| {
            e.expect("readable dir entry")
                .file_name()
                .into_string()
                .expect("utf-8")
        })
        .filter(|n| n.ends_with(".md") && n != "README.md")
        .map(|n| n.trim_end_matches(".md").to_string())
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = lark::COMPOSITION.iter().map(|c| c.to_string()).collect();
    listed.sort();
    assert_eq!(on_disk, listed, "grammar/ chapters and COMPOSITION differ");
}

/// The grammar reads, every name it uses is defined, both start symbols
/// derive finitely, and the sets of duplicates and ungeneratable rules
/// are exactly the known ones — any change here is grammar evolution the
/// fuzzer must absorb deliberately, not silently.
#[test]
fn grammar_loads_and_root_is_generatable() {
    let grammar = lark::Grammar::load(&lark::chapter_files());
    // Lark refuses a name defined twice.
    assert!(
        grammar.duplicates.is_empty(),
        "duplicate definitions: {:?}",
        grammar.duplicates
    );
    let undefined = grammar.undefined();
    assert!(
        undefined.is_empty(),
        "names used but defined nowhere: {undefined:?}"
    );
    // The indenter's events are declared, not lexed (grammar lexical.md).
    assert_eq!(grammar.declared, ["_INDENT", "_DEDENT"]);
    assert!(
        grammar.imports.is_empty(),
        "the language grammar imports nothing"
    );
    let generator = generate::Generator::new(grammar);
    generator.assert_root_generatable();
    // Every rule has a finite derivation.
    let ungeneratable = generator.ungeneratable();
    assert!(
        ungeneratable.is_empty(),
        "rules with no finite derivation: {ungeneratable:?}"
    );
}

/// The fragment root generates too: `fragment` is the start symbol of the
/// specification's examples that are not whole files. Its sentences are
/// deterministic per seed and not all empty.
#[test]
fn fragment_root_generates() {
    let generator = generator();
    let snippets: Vec<String> = (0..16)
        .map(|seed| generator.sentence("fragment", seed, 200))
        .collect();
    for (seed, text) in snippets.iter().enumerate() {
        assert_eq!(
            *text,
            generator.sentence("fragment", seed as u64, 200),
            "seed {seed}: generation is not deterministic"
        );
    }
    assert!(
        snippets.iter().any(|t| !t.trim().is_empty()),
        "16 seeds of fragment produced no statement"
    );
}

struct FuzzOutcome {
    compiled: u32,
    rejected: u32,
    unsupported: u32,
}

fn fuzz_one(source: &str, seed: u64) -> &'static str {
    let mut request = common::minimal_valid_request();
    request.sources[0].content = source.to_string();
    request.sources[0].sha256 = common::sha256_hex(source.as_bytes());

    let result = catch_unwind(AssertUnwindSafe(|| compile(request)));
    let result = match result {
        Ok(result) => result,
        Err(panic) => {
            let msg = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("<non-string panic payload>");
            panic!(
                "ICE: panic escaped compile() (CMP-04 violation)\n\
                 seed: {seed}\npanic: {msg}\nprogram:\n{source}"
            );
        }
    };

    let diagnostics = match &result {
        Ok(artifact) => {
            let mut validator = wasmparser::Validator::new();
            if let Err(err) = validator.validate_all(&artifact.wasm) {
                panic!(
                    "emitted component fails wasmparser validation\n\
                     seed: {seed}\nerror: {err}\nprogram:\n{source}"
                );
            }
            artifact.diagnostics.clone()
        }
        Err(CompileError::Rejected(diags)) => diags.clone(),
        Err(CompileError::Unsupported(_)) => Vec::new(),
        Err(CompileError::Incomplete { completed }) => panic!(
            "compile() returned the retired pre-v1 Incomplete channel \
             (completed: {completed})\nseed: {seed}\nprogram:\n{source}"
        ),
    };

    for diag in &diagnostics {
        assert_ne!(
            diag.code, "COM013",
            "generated input broke a codegen invariant (COM013)\n\
             seed: {seed}\nmessage: {}\nprogram:\n{source}",
            diag.message
        );
        assert!(
            codes::lookup(&diag.code).is_some(),
            "unregistered diagnostic code {:?} (DIA-01)\nseed: {seed}\nprogram:\n{source}",
            diag.code
        );
    }

    match result {
        Ok(_) => "compiled",
        Err(CompileError::Rejected(_)) => "rejected",
        Err(CompileError::Unsupported(_)) => "unsupported",
        Err(CompileError::Incomplete { .. }) => unreachable!("panicked above"),
    }
}

/// The fuzz sweep. Defaults are deterministic and CI-sized; the nightly job
/// widens via `GRAMMAR_FUZZ_SEED` (base) and `GRAMMAR_FUZZ_COUNT`.
#[test]
fn generated_programs_never_ice() {
    let base: u64 = std::env::var("GRAMMAR_FUZZ_SEED")
        .map(|v| v.parse().expect("GRAMMAR_FUZZ_SEED is a u64"))
        .unwrap_or(0);
    let count: u64 = std::env::var("GRAMMAR_FUZZ_COUNT")
        .map(|v| v.parse().expect("GRAMMAR_FUZZ_COUNT is a u64"))
        .unwrap_or(256);

    let generator = generator();
    generator.assert_root_generatable();

    let mut outcome = FuzzOutcome {
        compiled: 0,
        rejected: 0,
        unsupported: 0,
    };
    for seed in base..base + count {
        let source = generator.program(seed, 400);
        // Debug aid: dump generated programs without compiling them.
        if std::env::var("GRAMMAR_FUZZ_GEN_ONLY").is_ok() {
            println!("--- seed {seed} ({} bytes)\n{source}", source.len());
            continue;
        }
        match fuzz_one(&source, seed) {
            "compiled" => outcome.compiled += 1,
            "rejected" => outcome.rejected += 1,
            _ => outcome.unsupported += 1,
        }
    }
    println!(
        "grammar fuzz: {count} programs from seed {base} — \
         {} compiled, {} rejected, {} unsupported",
        outcome.compiled, outcome.rejected, outcome.unsupported
    );
}

/// CMP-02 under fuzz: a sample of generated requests recompiles
/// byte-identically (component bytes and serialized manifest).
#[test]
fn generated_programs_compile_deterministically() {
    let generator = generator();
    for seed in (0..64).step_by(8) {
        let source = generator.program(seed, 400);
        let mut request = common::minimal_valid_request();
        request.sources[0].content = source.clone();
        request.sources[0].sha256 = common::sha256_hex(source.as_bytes());

        let first = compile(request.clone());
        let second = compile(request);
        match (first, second) {
            (Ok(a), Ok(b)) => {
                assert_eq!(a.wasm, b.wasm, "seed {seed}: component bytes diverged");
                let ma = serde_json::to_string(&a.manifest).expect("manifest serializes");
                let mb = serde_json::to_string(&b.manifest).expect("manifest serializes");
                assert_eq!(ma, mb, "seed {seed}: manifests diverged");
            }
            (Err(CompileError::Rejected(a)), Err(CompileError::Rejected(b))) => {
                let da = serde_json::to_string(&a).expect("diagnostics serialize");
                let db = serde_json::to_string(&b).expect("diagnostics serialize");
                assert_eq!(da, db, "seed {seed}: diagnostics diverged");
            }
            (a, b) => assert_eq!(
                variant_name(&a),
                variant_name(&b),
                "seed {seed}: outcome variant diverged between identical requests"
            ),
        }
    }
}

fn variant_name(result: &Result<clean_compiler::CompileArtifact, CompileError>) -> &'static str {
    match result {
        Ok(_) => "ok",
        Err(CompileError::Rejected(_)) => "rejected",
        Err(CompileError::Unsupported(_)) => "unsupported",
        Err(CompileError::Incomplete { .. }) => "incomplete",
    }
}
