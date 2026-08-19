//! L5 conformance — the vendored JSONTestSuite corpus against the
//! chapter-15 JSON parser (quality playbook §1.9; 05 execution/testing/
//! 06-stdlib-conformance-testing.md).
//!
//! - `y_*` MUST parse, `n_*` MUST reject; `i_*` may do either but the
//!   verdict is pinned in `i_verdicts.txt` and CI fails on drift
//!   (post-ADR-0010 the accept/reject boundary itself lives in the
//!   RUN007/RUN009/RUN010 rule conditions).
//! - The guest surface takes Clean strings, which are well-formed UTF-8
//!   by construction (TXT-01) — corpus files that are not valid UTF-8
//!   are outside the input domain and are skipped, listed explicitly.
//! - The mandatory golden test: `tryTextToData` parity with the `n_*`
//!   set, plus trap parity of `textToData` over every `n_*` case.

use std::path::PathBuf;

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::{codegen, hir, mir, parser, resolver, typecheck};

mod common;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/json/JSONTestSuite/test_parsing")
}

fn compile_probe() -> Vec<u8> {
    // One guest, executed once per corpus case: reads the case text from
    // the host, answers through `verdict` (1 = parsed, 0 = rejected).
    let bridge = "\
host interface probe version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function caseText() returns string
\t\tdescription \"The JSON text under test.\"

\thost function verdict(isNone: boolean)
\t\tdescription \"Whether tryTextToData produced none.\"
";
    let main = "\
functions:
\tvoid init()
\t\tstring text = caseText()
\t\tverdict(json.tryTextToData(text) is none)
";
    let strict = "\
functions:
\tvoid init()
\t\tstring text = caseText()
\t\tany d = json.textToData(text)
\t\tverdict(d is not none)
";
    let _ = strict;
    let request = {
        let mut request = common::minimal_valid_request();
        request.sources = [("app/host_bridge.cln", bridge), ("app/main.cln", main)]
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
    let resolved = resolver::resolve(files, &[], &mut sink);
    let typed = typecheck::check(&resolved, &validated.world, &mut sink);
    assert!(!sink.has_errors(), "{:#?}", sink.into_diagnostics());
    let hir = hir::lower(typed);
    let mir = mir::lower(
        &hir,
        &resolved,
        &validated.world,
        &validated.world.package_version(),
        clean_compiler::layout::tier("standard").expect("standard tier exists"),
        &mut sink,
    );
    assert!(sink.unsupported().is_empty(), "{:#?}", sink.unsupported());
    codegen::core::emit_core(&mir).expect("static data fits below the heap start")
}

/// Compile the strict (`textToData`) probe for trap-parity checks.
fn compile_strict_probe() -> Vec<u8> {
    let bridge = "\
host interface probe version \"0.1.0\":
\trequires host worlds [\"server\"]

\thost function caseText() returns string
\t\tdescription \"The JSON text under test.\"

\thost function verdict(isNone: boolean)
\t\tdescription \"Reached only when parsing did not trap.\"
";
    let main = "\
functions:
\tvoid init()
\t\tstring text = caseText()
\t\tany d = json.textToData(text)
\t\tverdict(d is none)
";
    let request = {
        let mut request = common::minimal_valid_request();
        request.sources = [("app/host_bridge.cln", bridge), ("app/main.cln", main)]
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
    let resolved = resolver::resolve(files, &[], &mut sink);
    let typed = typecheck::check(&resolved, &validated.world, &mut sink);
    assert!(!sink.has_errors(), "{:#?}", sink.into_diagnostics());
    let hir = hir::lower(typed);
    let mir = mir::lower(
        &hir,
        &resolved,
        &validated.world,
        &validated.world.package_version(),
        clean_compiler::layout::tier("standard").expect("standard tier exists"),
        &mut sink,
    );
    assert!(sink.unsupported().is_empty(), "{:#?}", sink.unsupported());
    codegen::core::emit_core(&mir).expect("static data fits below the heap start")
}

struct Rig {
    engine: wasmtime::Engine,
    module: wasmtime::Module,
}

#[derive(Default)]
struct CaseState {
    text: String,
    parsed: Option<bool>,
}

impl Rig {
    fn new(wasm: &[u8]) -> Rig {
        let engine = wasmtime::Engine::default();
        let module = wasmtime::Module::new(&engine, wasm).expect("module loads");
        Rig { engine, module }
    }

    /// Runs one case in a fresh instance; Ok(parsed) or Err on trap.
    fn run(&self, text: &str) -> Result<bool, wasmtime::Error> {
        let mut linker: wasmtime::Linker<CaseState> = wasmtime::Linker::new(&self.engine);
        linker
            .func_wrap(
                "clean:host/probe@0.1.0",
                "case-text",
                |mut caller: wasmtime::Caller<'_, CaseState>, retptr: i32| {
                    let payload = caller.data().text.clone().into_bytes();
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .expect("memory export");
                    let realloc = caller
                        .get_export("cabi_realloc")
                        .and_then(|e| e.into_func())
                        .expect("realloc export")
                        .typed::<(i32, i32, i32, i32), i32>(&caller)
                        .expect("realloc type");
                    let dst = realloc
                        .call(&mut caller, (0, 0, 1, payload.len() as i32))
                        .expect("realloc runs");
                    memory
                        .write(&mut caller, dst as usize, &payload)
                        .expect("write payload");
                    let head = [
                        (dst as u32).to_le_bytes(),
                        (payload.len() as u32).to_le_bytes(),
                    ]
                    .concat();
                    memory
                        .write(&mut caller, retptr as usize, &head)
                        .expect("write retptr");
                },
            )
            .expect("links");
        linker
            .func_wrap(
                "clean:host/probe@0.1.0",
                "verdict",
                |mut caller: wasmtime::Caller<'_, CaseState>, is_none: i32| {
                    // The guest reports `is none`; parsed is its negation
                    // (the strict probe reports true, which also lands
                    // here as "not none").
                    caller.data_mut().parsed = Some(is_none == 0);
                },
            )
            .expect("links");
        let mut store = wasmtime::Store::new(
            &self.engine,
            CaseState {
                text: text.to_string(),
                parsed: None,
            },
        );
        let instance = linker
            .instantiate(&mut store, &self.module)
            .expect("instantiates");
        instance
            .get_typed_func::<(), ()>(&mut store, "init")
            .expect("init export")
            .call(&mut store, ())?;
        Ok(store.data().parsed.expect("verdict reported"))
    }
}

fn corpus_files() -> Vec<(String, Vec<u8>)> {
    let dir = corpus_dir();
    assert!(
        dir.is_dir(),
        "vendored corpus missing at {} — see tests/conformance/json/SOURCE.md",
        dir.display()
    );
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(&dir)
        .expect("corpus dir reads")
        .map(|e| e.expect("dir entry"))
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .map(|e| {
            (
                e.file_name().to_string_lossy().to_string(),
                std::fs::read(e.path()).expect("case reads"),
            )
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        files.len() >= 300,
        "corpus looks truncated: {}",
        files.len()
    );
    files
}

/// RUN009 deliberately rejects duplicate object keys, so two upstream
/// `y_*` files are expected rejections here (the boundary is the rule
/// condition, not the corpus — SOURCE.md).
const Y_EXCEPTIONS: [&str; 2] = [
    "y_object_duplicated_key.json",
    "y_object_duplicated_key_and_value.json",
];

#[test]
fn jsontestsuite_verdicts_hold() {
    let rig = Rig::new(&compile_strict_probe());
    let verdicts_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance/json/i_verdicts.txt");
    // UPDATE_I_VERDICTS=1 regenerates the pin deliberately (the
    // UPDATE_DIAG_FIXTURES convention); a verdict change is a design
    // question, never an accident.
    if std::env::var("UPDATE_I_VERDICTS").is_ok() {
        let mut lines =
            vec!["# Pinned verdicts for JSONTestSuite i_* cases (see SOURCE.md).".to_string()];
        for (name, bytes) in corpus_files() {
            if !name.starts_with("i_") {
                continue;
            }
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            let parsed = rig.run(&text).is_ok();
            lines.push(format!(
                "{name} {}",
                if parsed { "accept" } else { "reject" }
            ));
        }
        std::fs::write(&verdicts_path, lines.join("\n") + "\n").expect("verdicts write");
    }
    let pinned: std::collections::BTreeMap<String, bool> = std::fs::read_to_string(&verdicts_path)
        .expect("i_verdicts.txt (pinned implementation-defined verdicts) reads")
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (name, v) = l.rsplit_once(' ').expect("`name verdict` lines");
            (name.to_string(), v == "accept")
        })
        .collect();

    let mut skipped_non_utf8 = Vec::new();
    let mut failures = Vec::new();
    let mut i_seen = std::collections::BTreeMap::new();
    for (name, bytes) in corpus_files() {
        let Ok(text) = String::from_utf8(bytes) else {
            // Outside the input domain: a Clean string cannot hold it
            // (TXT-01). Named, not silent (no-silent-caps).
            skipped_non_utf8.push(name);
            continue;
        };
        // Accept = the strict entry did not trap (a `null` document
        // parses to none, which is still an accept).
        let parsed = rig.run(&text).is_ok();
        if name.starts_with("y_") && !parsed && !Y_EXCEPTIONS.contains(&name.as_str()) {
            failures.push(format!("{name}: y_ case rejected"));
        } else if Y_EXCEPTIONS.contains(&name.as_str()) && parsed {
            failures.push(format!(
                "{name}: RUN009 requires rejecting duplicate keys, but it parsed"
            ));
        } else if name.starts_with("n_") && parsed {
            failures.push(format!("{name}: n_ case silently accepted"));
        } else if name.starts_with("i_") {
            i_seen.insert(name.clone(), parsed);
            match pinned.get(&name) {
                Some(&expected) if expected != parsed => failures.push(format!(
                    "{name}: i_ verdict drifted (pinned {}, got {})",
                    if expected { "accept" } else { "reject" },
                    if parsed { "accept" } else { "reject" },
                )),
                Some(_) => {}
                None => failures.push(format!("{name}: i_ case has no pinned verdict")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} corpus failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // The skip list is part of the contract: pinned exactly (25 of the
    // 318 files are not valid UTF-8 — outside the Clean string input
    // domain, TXT-01); it may only shrink.
    assert_eq!(
        skipped_non_utf8.len(),
        25,
        "non-UTF-8 skip list changed: {skipped_non_utf8:?}"
    );
    eprintln!(
        "corpus: {} cases, {} i_ pinned, {} non-UTF-8 skipped: {:?}",
        corpus_files().len(),
        i_seen.len(),
        skipped_non_utf8.len(),
        skipped_non_utf8
    );
}

#[test]
fn text_to_data_traps_exactly_where_try_returns_none() {
    // The mandatory golden test (06-stdlib-conformance-testing.md §5):
    // the strict entry traps in exactly the try entry's none conditions.
    let try_rig = Rig::new(&compile_probe());
    let strict_rig = Rig::new(&compile_strict_probe());
    let mut mismatches = Vec::new();
    for (name, bytes) in corpus_files() {
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let try_parsed = try_rig.run(&text).unwrap_or(false);
        let strict_outcome = strict_rig.run(&text);
        match (try_parsed, strict_outcome) {
            (true, Ok(true)) => {}
            (false, Err(_)) => {}
            // A document that parses to JSON null: try yields none while
            // the strict entry succeeds with a none value — chapter 15's
            // own wording makes the two indistinguishable (DISCOVERIES-M6).
            (false, Ok(false)) => {}
            (t, s) => mismatches.push(format!(
                "{name}: try={} strict={}",
                t,
                match s {
                    Ok(v) => format!("ok({v})"),
                    Err(_) => "trap".to_string(),
                }
            )),
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} parity mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
