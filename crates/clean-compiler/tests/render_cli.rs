//! Byte-exact renderer checks against the worked examples of Platform 13
//! (§4.2 and §5.4). The spec's own example blocks are the fixtures: if the
//! renderer drifts from them by one byte, these fail.

use clean_compiler::diag::{finalize, render_cli, SourceCache};
use clean_compiler_types::{Annotation, Diagnostic, Level, Position, Span};

fn span(file: &str, line: u32, start_col: u32, end_col: u32) -> Span {
    Span::new(
        file,
        Position {
            line,
            column: start_col,
        },
        Position {
            line,
            column: end_col,
        },
    )
}

fn diagnostic(code: &str, message: &str, primary: Span) -> Diagnostic {
    Diagnostic {
        level: Level::Error,
        code: code.to_string(),
        message: message.to_string(),
        primary_span: primary,
        primary_label: None,
        secondary: Vec::new(),
        notes: Vec::new(),
        helps: Vec::new(),
        suggestions: Vec::new(),
        doc_url: Diagnostic::doc_url_for(code),
        rendered: String::new(),
    }
}

/// The §4.2 example: primary and secondary spans on the same line, the
/// secondary label inline, the primary label hanging below via `|`.
#[test]
fn renders_the_sem001_example_of_platform_13_4_2() {
    let mut sources = SourceCache::empty();
    // Line 42 of the example file; lines 1..41 are irrelevant padding.
    let mut content = "\n".repeat(41);
    content.push_str("    integer total = subtotal + shipping\n");
    sources.insert("app/orders/checkout.cln", content);

    let mut d = diagnostic(
        "SEM001",
        "type mismatch in assignment",
        span("app/orders/checkout.cln", 42, 5, 12),
    );
    d.primary_label = Some("`total` is declared with type `integer`".to_string());
    d.secondary.push(Annotation {
        span: span("app/orders/checkout.cln", 42, 21, 40),
        label: "this expression has type `number`".to_string(),
    });
    d.helps.push(
        "either declare `total` as `number`, or convert with `subtotal.toInteger()`".to_string(),
    );

    let expected = "\
error[SEM001]: type mismatch in assignment
  --> app/orders/checkout.cln:42:5
   |
42 |     integer total = subtotal + shipping
   |     ^^^^^^^         ------------------- this expression has type `number`
   |     |
   |     `total` is declared with type `integer`
   |
   = help: either declare `total` as `number`, or convert with `subtotal.toInteger()`
   = docs: https://errors.cleanlanguage.dev/E/SEM001
";
    assert_eq!(render_cli(&d, &sources), expected);
}

/// The §5.4 example's diagnostic block: a single labelled primary span, the
/// label inline on the caret line.
#[test]
fn renders_the_sem002_example_of_platform_13_5_4() {
    let mut sources = SourceCache::empty();
    let mut content = "\n".repeat(17);
    content.push_str("    integer n = lenght(users)\n");
    sources.insert("app/reports/summary.cln", content);

    // The spec block's `-->` line says column 12 while its caret run and
    // its suggestion spans both say 17..23 ("lenght"); the caret positions
    // are taken as authoritative (recorded in docs/DISCOVERIES-M2.md).
    let mut d = diagnostic(
        "SEM002",
        "I cannot find a variable named `lenght` in scope",
        span("app/reports/summary.cln", 18, 17, 23),
    );
    d.primary_label = Some("no variable with this name exists here".to_string());
    d.helps
        .push("closest known names are `length`, `lengthOf`".to_string());

    let expected = "\
error[SEM002]: I cannot find a variable named `lenght` in scope
  --> app/reports/summary.cln:18:17
   |
18 |     integer n = lenght(users)
   |                 ^^^^^^ no variable with this name exists here
   |
   = help: closest known names are `length`, `lengthOf`
   = docs: https://errors.cleanlanguage.dev/E/SEM002
";
    assert_eq!(render_cli(&d, &sources), expected);
}

/// Tabs in the quoted line are mirrored into the annotation padding so the
/// caret run stays visually under the marked characters (Clean indents with
/// tabs — LEX rules).
#[test]
fn caret_padding_mirrors_tabs() {
    let mut sources = SourceCache::empty();
    sources.insert("app/main.cln", "functions:\n\tinteger add()\n");
    let d = diagnostic("SEM019", "example", span("app/main.cln", 2, 2, 9));
    let rendered = render_cli(&d, &sources);
    assert!(
        rendered.contains(" | \t^^^^^^^\n"),
        "annotation line must pad with the source's tab:\n{rendered}"
    );
}

/// A file the cache does not hold — request-level diagnostics — renders
/// without a caret block, keeping the `= label` line of the M1 shape.
#[test]
fn unquoted_diagnostics_omit_the_caret_block() {
    let mut d = diagnostic(
        "RQD002",
        "invalid compilation request: unknown top-level key at '$.target'",
        Span::request_document(),
    );
    d.primary_label = Some("unknown top-level key".to_string());
    let expected = "\
error[RQD002]: invalid compilation request: unknown top-level key at '$.target'
 --> <request>:1:1
  = unknown top-level key
  = docs: https://errors.cleanlanguage.dev/E/RQD002
";
    assert_eq!(render_cli(&d, &SourceCache::empty()), expected);
}

/// Platform 13 §10.2: diagnostics deduplicate by (code, primary span,
/// message), first emission wins.
#[test]
fn finalize_deduplicates_by_code_span_and_message() {
    let a = diagnostic("SEM002", "dup", span("app/main.cln", 1, 1, 2));
    let b = diagnostic("SEM002", "dup", span("app/main.cln", 1, 1, 2));
    let c = diagnostic("SEM002", "other", span("app/main.cln", 1, 1, 2));
    let out = finalize(vec![a, b, c], &SourceCache::empty());
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].message, "dup");
    assert_eq!(out[1].message, "other");
}
