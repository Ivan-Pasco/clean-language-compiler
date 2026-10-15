//! Parser conformance against the grammar corpus (`grammar/corpus.json`).
//!
//! The corpus holds every ```clean example of foundation that the grammar
//! checks, with the result it expects: `passes` (the parser accepts it) or
//! `fails:SYN…` (the parser rejects it). `tools/grammar/extract.py` builds
//! it; `tools/grammar/check.py` holds the Lark grammar to it. This test
//! holds the real lexer and parser to the same cases.
//!
//! The parser reads whole files, so a case is wrapped by its entry:
//! `file` and `outer_block` as they are; `fragment` and `expression` as the
//! body of a `start:` section; `identifier` as the name of an `integer`
//! declaration in a `start:` section. A case is accepted when lexing and
//! parsing emit no diagnostic; a rejection is listed with its first one
//! (the line counts the wrapper's `start:` line, when there is one).
//!
//! Every case where the parser disagrees with the corpus is listed in a
//! pinned snapshot, so each construct the parser learns (or loses) is a
//! reviewable diff. The goal is an empty list.

use std::fmt::Write as _;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::lexer::lex;
use clean_compiler::parser::parse;

fn corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/corpus.json")
}

fn snapshot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshots/grammar")
}

fn indent(code: &str) -> String {
    let mut out = String::from("start:\n");
    for line in code.lines() {
        if !line.trim().is_empty() {
            out.push('\t');
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn wrap(entry: &str, code: &str) -> String {
    match entry {
        "file" | "outer_block" => format!("{code}\n"),
        "fragment" | "expression" => indent(code),
        "identifier" => {
            let name = code.split("//").next().unwrap_or("").trim();
            format!("start:\n\tinteger {name} = 0\n")
        }
        other => panic!("unknown entry {other}"),
    }
}

/// `Ok(None)` accepted; `Ok(Some(first diagnostic))` rejected; `Err(())`
/// the parser panicked.
fn run(source: &str) -> Result<Option<String>, ()> {
    catch_unwind(AssertUnwindSafe(|| {
        let mut sink = DiagnosticSink::new();
        let stream = lex("app/main.cln", source, &mut sink);
        let _ = parse(&stream, &mut sink);
        sink.into_diagnostics().first().map(|d| {
            format!(
                "{} {}:{} {}",
                d.code, d.primary_span.start.line, d.primary_span.start.column, d.message
            )
        })
    }))
    .map_err(|_| ())
}

#[test]
fn parser_agrees_with_the_grammar_corpus() {
    let text = std::fs::read_to_string(corpus_path()).expect("grammar/corpus.json reads");
    let corpus: serde_json::Value = serde_json::from_str(&text).expect("corpus is JSON");
    let cases = corpus["cases"].as_array().expect("cases array");

    let mut checked = 0usize;
    let mut disagreements = Vec::new();
    for case in cases {
        let expected = case["expected"].as_str().expect("expected");
        if expected == "illustrative" {
            continue;
        }
        checked += 1;
        let entry = case["entry"].as_str().expect("entry");
        let code = case["code"].as_str().expect("code");
        let file = case["file"].as_str().expect("file");
        let line = case["line"].as_u64().expect("line");
        let want = expected == "passes";
        let verdict = match run(&wrap(entry, code)) {
            Err(()) => Some("panics".to_string()),
            Ok(None) if want => None,
            Ok(Some(_)) if !want => None,
            Ok(None) => Some("accepts, should reject".to_string()),
            Ok(Some(first)) => Some(format!("rejects, should accept — {first}")),
        };
        if let Some(verdict) = verdict {
            disagreements.push(format!("{file}:{line} [{entry}] {expected}: {verdict}"));
        }
    }

    let mut rendered = String::new();
    writeln!(
        rendered,
        "checked {checked} | agree {} | disagree {}",
        checked - disagreements.len(),
        disagreements.len()
    )
    .unwrap();
    for d in &disagreements {
        rendered.push_str(d);
        rendered.push('\n');
    }

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_dir());
    settings.set_prepend_module_to_snapshot(false);
    let _guard = settings.bind_to_scope();
    insta::assert_snapshot!("parser_disagreements", rendered);
}
