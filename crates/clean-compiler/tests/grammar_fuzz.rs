//! M9 grammar fuzz — programs generated from the law's Lark grammar itself.
//!
//! The vendored grammar (`tests/fixtures/grammar/`, byte-pinned copies of
//! foundation `governance/product/language-principles/*.lark.md`, the
//! companions FS-01 composes into one Lark grammar) seeds a deterministic
//! generator; every generated program goes through `compile()` under the
//! compiler's external contract:
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
use std::path::{Path, PathBuf};

fn grammar_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/grammar")
}

/// Vendored grammar files in filename order — the order FS-01 composes
/// them in.
fn vendored_files() -> Vec<(String, String)> {
    let mut names: Vec<String> = std::fs::read_dir(grammar_dir())
        .expect("vendored grammar directory exists")
        .map(|e| {
            e.expect("readable dir entry")
                .file_name()
                .into_string()
                .expect("utf-8")
        })
        .filter(|n| n.ends_with(".lark.md"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let content = std::fs::read_to_string(grammar_dir().join(&name))
                .expect("vendored grammar file reads");
            (name, content)
        })
        .collect()
}

fn generator() -> generate::Generator {
    generate::Generator::new(lark::Grammar::load(&vendored_files()))
}

/// The vendored copy matches its recorded hashes — refreshing the grammar is
/// a deliberate two-place change (files + SHA256SUMS), same discipline as
/// the vendored host.wit.
#[test]
fn vendored_grammar_matches_recorded_sha256() {
    let manifest = std::fs::read_to_string(grammar_dir().join("SHA256SUMS"))
        .expect("SHA256SUMS manifest exists");
    let mut recorded = 0;
    for line in manifest.lines().filter(|l| !l.trim().is_empty()) {
        let (hash, name) = line.split_once("  ").expect("shasum format");
        let bytes = std::fs::read(grammar_dir().join(name)).expect("listed file exists");
        assert_eq!(
            common::sha256_hex(&bytes),
            hash,
            "{name} no longer matches SHA256SUMS. If this refresh is \
             deliberate, regenerate the manifest in the same commit."
        );
        recorded += 1;
    }
    let on_disk = vendored_files().len();
    assert_eq!(
        recorded, on_disk,
        "SHA256SUMS lists {recorded} files but {on_disk} are vendored"
    );
}

/// When the foundation checkout is present, the vendored copy must be
/// byte-identical to the companions in
/// `governance/product/language-principles/`. Absent checkout: skips locally,
/// but fails when `CLEAN_SPEC_REQUIRED` is set (CI clones the spec checkout
/// and sets it — a conformance test must not self-skip there).
#[test]
fn vendored_grammar_matches_foundation() {
    let foundation = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../clean-language-foundation/governance/product/language-principles");
    if !foundation.is_dir() {
        assert!(
            std::env::var_os("CLEAN_SPEC_REQUIRED").is_none(),
            "CLEAN_SPEC_REQUIRED is set but ../clean-language-foundation is absent — \
             a conformance test must not self-skip when the spec checkout was promised"
        );
        eprintln!("skipping: ../clean-language-foundation not present");
        return;
    }
    for (name, vendored) in vendored_files() {
        let upstream = std::fs::read_to_string(foundation.join(&name))
            .unwrap_or_else(|_| panic!("{name} vanished from foundation language-principles/"));
        assert_eq!(
            vendored, upstream,
            "{name} drifted from foundation — refresh the vendored copy \
             and SHA256SUMS deliberately"
        );
    }
    // New grammar companions must be vendored too, or generation silently
    // under-covers the language.
    for entry in std::fs::read_dir(&foundation).expect("foundation language-principles dir") {
        let name = entry
            .expect("readable dir entry")
            .file_name()
            .into_string()
            .expect("utf-8");
        if name.ends_with(".lark.md") {
            assert!(
                grammar_dir().join(&name).is_file(),
                "foundation added {name}; vendor it and update SHA256SUMS"
            );
        }
    }
}

/// The grammar reads, every name it uses is defined, both start symbols
/// derive finitely, and the sets of duplicates and ungeneratable rules
/// are exactly the known ones — any change here is grammar evolution the
/// fuzzer must absorb deliberately, not silently.
#[test]
fn grammar_loads_and_root_is_generatable() {
    let grammar = lark::Grammar::load(&vendored_files());
    // Lark refuses a name defined twice; DOC-15 calls it a defect. The
    // WatchBlock double definition (DISCOVERIES-M9 §1f) was resolved by
    // foundation's 2026-08-20 erratum and must not come back.
    assert!(
        grammar.duplicates.is_empty(),
        "duplicate definitions reappeared (DOC-15 defect): {:?}",
        grammar.duplicates
    );
    let undefined = grammar.undefined();
    assert!(
        undefined.is_empty(),
        "names used but defined nowhere: {undefined:?}"
    );
    // The indenter's events are declared, not lexed (LEXG-02).
    assert_eq!(grammar.declared, ["_INDENT", "_DEDENT"]);
    assert!(
        grammar.imports.is_empty(),
        "the language grammar imports nothing"
    );
    let generator = generate::Generator::new(grammar);
    generator.assert_root_generatable();
    // Every rule has a finite derivation. Under the EBNF this set was
    // LibraryBlock, BlockArgType, CompileTimeFunctionDeclaration,
    // ExpressionType and IdentifierType: a library block's body was an
    // informal "handler-defined" special, and the BlockArg payload types
    // were schema-tier specials. The Lark grammar spells the body as lines
    // of tokens (BLKG-02) and writes the payload types as comments, not
    // rules, so nothing is left without a derivation.
    let ungeneratable = generator.ungeneratable();
    assert!(
        ungeneratable.is_empty(),
        "ungeneratable-rule set changed — update DISCOVERIES-M9 and this pin: {ungeneratable:?}"
    );
}

/// The snippet root generates too: `statement_sequence` is the start
/// symbol a parser reading the chapters' snippets uses (FS-01). Its
/// sentences are deterministic per seed and not all empty.
#[test]
fn statement_sequence_root_generates() {
    let generator = generator();
    let snippets: Vec<String> = (0..16)
        .map(|seed| generator.sentence("statement_sequence", seed, 200))
        .collect();
    for (seed, text) in snippets.iter().enumerate() {
        assert_eq!(
            *text,
            generator.sentence("statement_sequence", seed as u64, 200),
            "seed {seed}: generation is not deterministic"
        );
    }
    assert!(
        snippets.iter().any(|t| !t.trim().is_empty()),
        "16 seeds of statement_sequence produced no statement"
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
