//! Compiler `Diagnostic` → LSP `Diagnostic`, exactly per Platform 13 §7.
//!
//! The compiler's positions are 1-based line/character-column pairs (Platform
//! 13 §2); LSP positions are 0-based with UTF-16 code-unit columns (the
//! encoding this server declares at `initialize`). Conversion needs the file
//! content, which the request document carries — nothing is read elsewhere.

use clean_compiler_types::{Diagnostic, Level, Position, Span};
use lsp_types::DiagnosticSeverity;

/// One compiler position to LSP, counted against `content`.
fn position(content: &str, position: &Position) -> lsp_types::Position {
    let line_index = position.line.saturating_sub(1);
    let line_text = content
        .split('\n')
        .nth(line_index as usize)
        .unwrap_or_default();
    let character: u32 = line_text
        .chars()
        .take(position.column.saturating_sub(1) as usize)
        .map(|c| c.len_utf16() as u32)
        .sum();
    lsp_types::Position {
        line: line_index,
        character,
    }
}

/// A compiler span to an LSP range (0-based, UTF-16, end exclusive — the
/// exclusivity conventions already agree).
pub fn range(content: &str, span: &Span) -> lsp_types::Range {
    lsp_types::Range {
        start: position(content, &span.start),
        end: position(content, &span.end),
    }
}

fn severity(level: Level) -> DiagnosticSeverity {
    match level {
        Level::Error => DiagnosticSeverity::ERROR,
        Level::Warning => DiagnosticSeverity::WARNING,
        Level::Info => DiagnosticSeverity::INFORMATION,
        Level::Help => DiagnosticSeverity::HINT,
    }
}

/// Maps one compiler diagnostic per the Platform 13 §7 table. `content` is
/// the text of the diagnostic's primary file; `locate` resolves any file
/// named by a span to its URI and content (for `relatedInformation`).
pub fn diagnostic(
    diagnostic: &Diagnostic,
    content: &str,
    locate: impl Fn(&str) -> Option<(lsp_types::Uri, String)>,
) -> lsp_types::Diagnostic {
    // `message` + `primary_label`, joined with a newline when a label is
    // present; `notes`/`helps` are appended while the editor has no code
    // actions to fetch them from (§7 — this server does not advertise
    // `codeAction` yet, so the appended form is the only delivery).
    let mut message = diagnostic.message.clone();
    if let Some(label) = &diagnostic.primary_label {
        message.push('\n');
        message.push_str(label);
    }
    for note in &diagnostic.notes {
        message.push_str("\nnote: ");
        message.push_str(note);
    }
    for help in &diagnostic.helps {
        message.push_str("\nhelp: ");
        message.push_str(help);
    }

    let related: Vec<lsp_types::DiagnosticRelatedInformation> = diagnostic
        .secondary
        .iter()
        .filter_map(|annotation| {
            let (uri, file_content) = locate(&annotation.span.file)?;
            Some(lsp_types::DiagnosticRelatedInformation {
                location: lsp_types::Location {
                    uri,
                    range: range(&file_content, &annotation.span),
                },
                message: annotation.label.clone(),
            })
        })
        .collect();

    lsp_types::Diagnostic {
        range: range(content, &diagnostic.primary_span),
        severity: Some(severity(diagnostic.level)),
        code: Some(lsp_types::NumberOrString::String(diagnostic.code.clone())),
        code_description: diagnostic
            .doc_url
            .parse()
            .ok()
            .map(|href| lsp_types::CodeDescription { href }),
        source: None,
        message,
        related_information: (!related.is_empty()).then_some(related),
        tags: None,
        // The untouched compiler value (§6.1 schema), so `codeAction` can
        // recover `suggestions[]` without re-running analysis (Platform 04
        // §4.1.1).
        data: Some(serde_json::to_value(diagnostic).expect("diagnostic serializes")),
    }
}
