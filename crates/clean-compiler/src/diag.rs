//! The `DiagnosticSink` every pass appends to (Platform 14 §14.4.1: passes
//! may append, never read — no pass branches on prior diagnostics) and the
//! CLI renderer that fills `Diagnostic::rendered` (Platform 13 §4.2).

use clean_compiler_types::{Diagnostic, Level};

/// Append-only diagnostic accumulator. The driver drains it once, after the
/// last pass that ran; passes receive `&mut DiagnosticSink` and only push.
#[derive(Debug, Default)]
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
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

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Renders the CLI text for a diagnostic (Platform 13 §4.2). The full
/// source-quoting renderer lands in M2 with the DIA-06 byte-exact fixtures;
/// this covers the header, location, notes, helps, and docs lines, which is
/// the complete shape for request-level (`RQD`) diagnostics that have no
/// source line to quote.
pub fn render_cli(d: &Diagnostic) -> String {
    let level = match d.level {
        Level::Error => "error",
        Level::Warning => "warning",
        Level::Info => "info",
        Level::Help => "help",
    };
    let mut out = format!("{level}[{}]: {}\n", d.code, d.message);
    out.push_str(&format!(
        "  --> {}:{}:{}\n",
        d.primary_span.file, d.primary_span.start.line, d.primary_span.start.column
    ));
    if let Some(label) = &d.primary_label {
        out.push_str(&format!("   = {label}\n"));
    }
    for note in &d.notes {
        out.push_str(&format!("   = note: {note}\n"));
    }
    for help in &d.helps {
        out.push_str(&format!("   = help: {help}\n"));
    }
    out.push_str(&format!("   = docs: {}\n", d.doc_url));
    out
}
