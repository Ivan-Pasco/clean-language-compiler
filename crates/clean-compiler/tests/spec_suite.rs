//! Spec conformance suite: one fixture directory per EBNF grammar
//! chapter under `tests/spec/<chapter>/`, each `.cln` run through the
//! front-end — lex, parse, resolve, typecheck (M4) — to an insta snapshot
//! under `tests/snapshots/spec/`. A surprising diff is a design question —
//! never accept blindly.
//!
//! Fixtures see an empty target world, so host-interface declarations
//! project against nothing; chapter fixtures are language-level. The
//! snapshot pins the AST shape, every diagnostic, and the pre-v1
//! unsupported notes (the frontier of what later passes cannot lower yet).
//!
//! The harness self-discovers fixtures so adding a `.cln` file is enough;
//! discovery is sorted for determinism (CMP-02 discipline applies to test
//! output too).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use clean_compiler::diag::DiagnosticSink;
use clean_compiler::lexer::lex;
use clean_compiler::parser::parse;
use clean_compiler::resolver::{resolve, ParsedFile};
use clean_compiler_types::request::TargetWorld;

const EMPTY_WORLD_WIT: &str = "package spec:harness;\n\nworld conformance {\n}\n";

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/spec")
}

fn snapshot_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/snapshots/spec")
}

/// Renders the front-end run of one fixture: AST debug tree, every
/// diagnostic (code + message), and every unsupported note, so snapshots
/// pin the shape, the findings, and the frontier.
fn render(path: &str, source: &str) -> String {
    let mut sink = DiagnosticSink::new();
    let stream = lex(path, source, &mut sink);
    let ast = parse(&stream, &mut sink);
    let mut out = String::new();
    writeln!(out, "AST:").unwrap();
    writeln!(out, "{:#?}", ast.items).unwrap();

    let target = TargetWorld {
        host: "spec-harness".to_string(),
        version: "0.0.0".to_string(),
        world: "conformance".to_string(),
        sha256: String::new(),
        wit: EMPTY_WORLD_WIT.to_string(),
    };
    let world = clean_compiler::codegen::world::parse(&target, &mut sink)
        .expect("the embedded empty world parses");
    let resolved = resolve(vec![ParsedFile { ast, stream }], &[], &mut sink);
    clean_compiler::typecheck::check(&resolved, &world, &mut sink);

    let unsupported: Vec<String> = sink
        .unsupported()
        .iter()
        .map(|u| {
            format!(
                "  {} @ {}:{}:{}",
                u.construct, u.span.file, u.span.start.line, u.span.start.column
            )
        })
        .collect();
    let diagnostics = sink.into_diagnostics();
    writeln!(out, "DIAGNOSTICS: {}", diagnostics.len()).unwrap();
    for d in &diagnostics {
        writeln!(
            out,
            "  {} {}:{}:{} {}",
            d.code,
            d.primary_span.file,
            d.primary_span.start.line,
            d.primary_span.start.column,
            d.message
        )
        .unwrap();
    }
    writeln!(out, "UNSUPPORTED: {}", unsupported.len()).unwrap();
    for line in &unsupported {
        writeln!(out, "{line}").unwrap();
    }
    out
}

#[test]
fn spec_fixtures_match_snapshots() {
    let root = spec_root();
    assert!(root.is_dir(), "tests/spec/ missing at {}", root.display());
    let mut chapters: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("read tests/spec")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_dir())
        .collect();
    chapters.sort();
    assert!(
        !chapters.is_empty(),
        "tests/spec/ has no chapter directories"
    );

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(snapshot_dir());
    settings.set_prepend_module_to_snapshot(false);
    let _guard = settings.bind_to_scope();

    let mut total = 0usize;
    for chapter in chapters {
        let chapter_name = chapter.file_name().unwrap().to_string_lossy().to_string();
        let mut fixtures: Vec<PathBuf> = std::fs::read_dir(&chapter)
            .expect("read chapter dir")
            .map(|e| e.expect("dir entry").path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "cln"))
            .collect();
        fixtures.sort();
        for fixture in fixtures {
            let name = fixture.file_stem().unwrap().to_string_lossy().to_string();
            let source = std::fs::read_to_string(&fixture).expect("read fixture");
            let logical_path = format!("spec/{chapter_name}/{name}.cln");
            let rendered = render(&logical_path, &source);
            insta::assert_snapshot!(format!("{chapter_name}__{name}"), rendered);
            total += 1;
        }
    }
    assert!(total > 0, "no .cln fixtures under tests/spec/");
}
