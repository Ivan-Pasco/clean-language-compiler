//! The `DiagnosticSink` every pass appends to (Platform 14 §14.4.1: passes
//! may append, never read — no pass branches on prior diagnostics) and the
//! CLI renderer that fills `Diagnostic::rendered` (Platform 13 §4.2).
//!
//! Rendering happens twice: `build` fills a source-less rendering at
//! emission time so a `Diagnostic` is never incomplete, and the driver
//! re-renders every drained diagnostic through [`finalize`] with the
//! request's sources in hand, producing the full rustc-style block with
//! quoted source lines and caret runs.

use clean_compiler_types::{Diagnostic, Level};
use indexmap::IndexMap;

/// A language construct the current build recognises but cannot compile yet.
/// Pre-v1 only: each entry names a milestone gap, not a user error and not a
/// registered diagnostic — the driver turns them into
/// `CompileError::Unsupported` so they can never be mistaken for either.
#[derive(Debug, Clone)]
pub struct Unsupported {
    pub construct: &'static str,
    pub span: clean_compiler_types::Span,
}

/// Append-only diagnostic accumulator. The driver drains it once, after the
/// last pass that ran; passes receive `&mut DiagnosticSink` and only push.
#[derive(Debug, Default)]
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
    unsupported: Vec<Unsupported>,
}

impl DiagnosticSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// True if any accumulated diagnostic is an error — the signal the driver
    /// uses to stop the pipeline after the current pass completes.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == Level::Error)
    }

    /// Emission-ordered drain (Platform 13 §10.3: JSON preserves emission
    /// order; the CLI renderer re-sorts its own copy by file/line/column).
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Records a construct outside the current milestone surface. Idempotent
    /// per `(construct, span)`: pass [6] re-resolves and re-checks the
    /// expanded program, so the same note can legitimately be reached twice.
    pub fn note_unsupported(&mut self, construct: &'static str, span: clean_compiler_types::Span) {
        if self
            .unsupported
            .iter()
            .any(|u| u.construct == construct && u.span == span)
        {
            return;
        }
        self.unsupported.push(Unsupported { construct, span });
    }

    pub fn unsupported(&self) -> &[Unsupported] {
        &self.unsupported
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Source text keyed by the project-relative path spans carry, so the
/// renderer can quote the offending lines (Platform 13 §4.2).
#[derive(Debug, Default)]
pub struct SourceCache {
    files: IndexMap<String, String>,
}

impl SourceCache {
    /// No sources — request-level diagnostics (`RQD`) render without a
    /// caret block, which is their documented shape.
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_sources(sources: &[clean_compiler_types::request::SourceFile]) -> Self {
        let mut files = IndexMap::new();
        for source in sources {
            files.insert(source.path.clone(), source.content.clone());
        }
        Self { files }
    }

    pub fn insert(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    fn line(&self, path: &str, line: u32) -> Option<&str> {
        let content = self.files.get(path)?;
        content
            .lines()
            .nth(line.checked_sub(1)? as usize)
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
    }
}

/// Drain-time pipeline: deduplicate by `(code, primary_span, message)`
/// (Platform 13 §10.2, first occurrence wins so emission order is kept for
/// the NDJSON stream) and fill `rendered` with the full CLI block.
pub fn finalize(diagnostics: Vec<Diagnostic>, sources: &SourceCache) -> Vec<Diagnostic> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(diagnostics.len());
    for mut diagnostic in diagnostics {
        let key = format!(
            "{}\u{0}{}\u{0}{}\u{0}{}\u{0}{}\u{0}{}\u{0}{}",
            diagnostic.code,
            diagnostic.primary_span.file,
            diagnostic.primary_span.start.line,
            diagnostic.primary_span.start.column,
            diagnostic.primary_span.end.line,
            diagnostic.primary_span.end.column,
            diagnostic.message,
        );
        if !seen.insert(key) {
            continue;
        }
        diagnostic.rendered = render_cli(&diagnostic, sources);
        out.push(diagnostic);
    }
    out
}

/// Assembles a diagnostic with its `doc_url` and a source-less `rendered`
/// filled in, so the Platform 13 invariants (registered code, rendered
/// present) hold by construction; [`finalize`] re-renders with sources.
pub fn build(
    level: Level,
    code: &str,
    message: String,
    primary_span: clean_compiler_types::Span,
    primary_label: Option<String>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic {
        level,
        code: code.to_string(),
        message,
        primary_span,
        primary_label,
        secondary: Vec::new(),
        notes: Vec::new(),
        helps: Vec::new(),
        suggestions: Vec::new(),
        doc_url: Diagnostic::doc_url_for(code),
        rendered: String::new(),
    };
    diagnostic.rendered = render_cli(&diagnostic, &SourceCache::empty());
    diagnostic
}

/// One underline on one quoted source line.
struct Mark<'a> {
    /// 1-based column range, end exclusive, clipped to the line.
    start: usize,
    end: usize,
    primary: bool,
    label: Option<&'a str>,
}

/// Registers `span`'s start line for quoting; false when the file (or the
/// line) is not in the cache, which suppresses the caret block.
fn push_mark<'a>(
    blocks: &mut IndexMap<String, IndexMap<u32, Vec<Mark<'a>>>>,
    span: &clean_compiler_types::Span,
    primary: bool,
    label: Option<&'a str>,
    sources: &SourceCache,
) -> bool {
    let Some(line) = sources.line(&span.file, span.start.line) else {
        return false;
    };
    let line_len = line.chars().count();
    let start = (span.start.column as usize).clamp(1, line_len + 1);
    let end_col = if span.end.line == span.start.line {
        span.end.column as usize
    } else {
        line_len + 1
    };
    let end = end_col.clamp(start + 1, line_len + 2);
    blocks
        .entry(span.file.clone())
        .or_default()
        .entry(span.start.line)
        .or_default()
        .push(Mark {
            start,
            end,
            primary,
            label,
        });
    true
}

/// Renders the CLI text for a diagnostic — the exact §4.2 block:
///
/// ```text
/// <level>[<CODE>]: <message>
///   --> <file>:<line>:<column>
///    |
/// LN | <source line>
///    | <caret run>  <label>
///    |
///    = note: …  /  = help: …  /  = suggestion: …  /  = docs: …
/// ```
///
/// ASCII box drawing only; `^` underlines the primary span, `-` the
/// secondary spans. When two labelled spans share a line, the right-most
/// label sits on the annotation line and the others hang below it via `|`
/// continuations (the §4.2 example). Diagnostics whose file is not in the
/// cache — request-level codes, whole-build diagnostics — omit the caret
/// block, per the "not applicable" renderings of Platform 10.
pub fn render_cli(d: &Diagnostic, sources: &SourceCache) -> String {
    let level = match d.level {
        Level::Error => "error",
        Level::Warning => "warning",
        Level::Info => "info",
        Level::Help => "help",
    };

    // Group the quoted lines: primary file first, then each other file the
    // secondary annotations touch, in first-appearance order.
    let mut blocks: IndexMap<String, IndexMap<u32, Vec<Mark>>> = IndexMap::new();
    let primary_quoted = push_mark(
        &mut blocks,
        &d.primary_span,
        true,
        d.primary_label.as_deref(),
        sources,
    );
    for annotation in &d.secondary {
        push_mark(
            &mut blocks,
            &annotation.span,
            false,
            Some(annotation.label.as_str()),
            sources,
        );
    }

    // Gutter width: digits of the widest quoted line number; the primary
    // span's line when nothing is quoted.
    let widest_line = blocks
        .values()
        .flat_map(|lines| lines.keys().copied())
        .max()
        .unwrap_or(d.primary_span.start.line);
    let width = widest_line.max(1).to_string().len();
    let pad = " ".repeat(width);
    let bar = format!("{pad} |");
    let gutter = format!("{pad} | ");

    let mut out = format!("{level}[{}]: {}\n", d.code, d.message);
    out.push_str(&format!(
        "{pad}--> {}:{}:{}\n",
        d.primary_span.file, d.primary_span.start.line, d.primary_span.start.column
    ));

    for (file, mut lines) in blocks {
        if file != d.primary_span.file {
            // A secondary annotation in another file opens its own block.
            let first = lines.keys().min().copied().unwrap_or(1);
            out.push_str(&format!("{pad}--> {file}:{first}\n"));
        }
        out.push_str(&bar);
        out.push('\n');
        lines.sort_keys();
        for (line_no, mut marks) in lines {
            let source_line = sources.line(&file, line_no).unwrap_or_default();
            let chars: Vec<char> = source_line.chars().collect();
            // Mirror tabs from the source into the padding so caret runs
            // stay visually aligned; every other padding char is a space.
            let pad_like = |from: usize, to: usize, out: &mut String| {
                for col in from..to {
                    match chars.get(col - 1) {
                        Some('\t') => out.push('\t'),
                        _ => out.push(' '),
                    }
                }
            };

            out.push_str(&format!("{line_no:>width$} | {source_line}\n"));

            marks.sort_by_key(|m| m.start);
            let mut annotation = String::new();
            let mut col = 1usize;
            for mark in &marks {
                if mark.start > col {
                    pad_like(col, mark.start, &mut annotation);
                    col = mark.start;
                }
                let run = mark.end.saturating_sub(col).max(1);
                annotation.push_str(&if mark.primary { "^" } else { "-" }.repeat(run));
                col += run;
            }
            // The right-most mark's label sits inline; earlier labels hang
            // below through a `|` continuation.
            let mut hanging: Vec<&Mark> = Vec::new();
            if let Some((last, rest)) = marks.split_last() {
                if let Some(label) = last.label {
                    annotation.push(' ');
                    annotation.push_str(label);
                }
                hanging.extend(rest.iter().filter(|m| m.label.is_some()));
            }
            out.push_str(&gutter);
            out.push_str(annotation.trim_end());
            out.push('\n');

            if !hanging.is_empty() {
                let mut bars = String::new();
                let mut col = 1usize;
                for mark in &hanging {
                    pad_like(col, mark.start, &mut bars);
                    bars.push('|');
                    col = mark.start + 1;
                }
                out.push_str(&gutter);
                out.push_str(bars.trim_end());
                out.push('\n');
                for (i, mark) in hanging.iter().enumerate().rev() {
                    let mut line = String::new();
                    let mut col = 1usize;
                    for earlier in &hanging[..i] {
                        pad_like(col, earlier.start, &mut line);
                        line.push('|');
                        col = earlier.start + 1;
                    }
                    pad_like(col, mark.start, &mut line);
                    line.push_str(mark.label.unwrap_or_default());
                    out.push_str(&gutter);
                    out.push_str(line.trim_end());
                    out.push('\n');
                }
            }
        }
        out.push_str(&bar);
        out.push('\n');
    }

    // Without a caret block the primary label still surfaces, as an `=` line
    // (the M1 shape for request-level diagnostics).
    if !primary_quoted {
        if let Some(label) = &d.primary_label {
            out.push_str(&format!("{pad} = {label}\n"));
        }
    }
    for note in &d.notes {
        out.push_str(&format!("{pad} = note: {note}\n"));
    }
    for help in &d.helps {
        out.push_str(&format!("{pad} = help: {help}\n"));
    }
    for suggestion in &d.suggestions {
        // §4.2 renders "<replacement snippet>": the first replacement's new
        // text, falling back to the suggestion's message when it has none.
        let snippet = suggestion
            .replacements
            .first()
            .map(|r| r.replacement.as_str())
            .unwrap_or(suggestion.message.as_str());
        out.push_str(&format!("{pad} = suggestion: {snippet}\n"));
    }
    out.push_str(&format!("{pad} = docs: {}\n", d.doc_url));
    out
}
