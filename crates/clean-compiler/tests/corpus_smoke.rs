//! Smoke harness over the harvested corpus (tests/corpus/): every `.cln`
//! from the retired compiler runs through the M3 front-end (lex + parse)
//! and is classified parse-clean / expected-diagnostic / out-of-surface.
//! The classification is pinned as a snapshot so grammar gains and
//! regressions both surface as reviewable diffs. See tests/corpus/README.md
//! — fixtures are data, never a source of unspecified behaviour.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::lexer::lex;
use clean_compiler::parser::parse;

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus")
}

fn snapshot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshots/corpus")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "cln") {
            out.push(path);
        }
    }
}

#[test]
fn corpus_parses_without_panicking_and_classification_is_pinned() {
    let root = corpus_root();
    let mut fixtures = Vec::new();
    collect(&root, &mut fixtures);
    assert!(
        fixtures.len() >= 689,
        "corpus shrank: {} fixtures found",
        fixtures.len()
    );

    let mut lines = Vec::new();
    let mut clean = 0usize;
    let mut expected = 0usize;
    let mut out_of_surface = 0usize;
    for path in &fixtures {
        let rel = path
            .strip_prefix(&root)
            .expect("under corpus root")
            .to_string_lossy()
            .replace('\\', "/");
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let mut sink = DiagnosticSink::new();
        let stream = lex(&rel, &source, &mut sink);
        let _ast = parse(&stream, &mut sink);
        let diagnostics = sink.into_diagnostics();
        let class = if diagnostics.is_empty() {
            clean += 1;
            "parse-clean"
        } else if rel.split('/').any(|part| part == "fail") {
            expected += 1;
            "expected-diagnostic"
        } else {
            out_of_surface += 1;
            "out-of-surface"
        };
        lines.push(format!("{class:<19} {rel}"));
    }

    let mut rendered = String::new();
    writeln!(
        rendered,
        "total {} | parse-clean {clean} | expected-diagnostic {expected} | out-of-surface {out_of_surface}",
        fixtures.len()
    )
    .unwrap();
    for line in &lines {
        rendered.push_str(line);
        rendered.push('\n');
    }

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_dir());
    settings.set_prepend_module_to_snapshot(false);
    let _guard = settings.bind_to_scope();
    insta::assert_snapshot!("classification", rendered);
}
