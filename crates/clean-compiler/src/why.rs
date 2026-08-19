//! Diagnostic re-projection (Platform 14 §14.14.1, the operation behind
//! `cln why`): given the `diagnostics.json` of the most recent build and a
//! source location, re-present the diagnostics at that location with
//! expanded provenance — which pass rejected it, what the compiler knew,
//! which alternatives it suggested.
//!
//! This is a re-presentation, not a re-compilation: the input is the
//! diagnostics the caller hands over (CMP-01 — the compiler goes looking
//! for nothing), and every fact in the report comes from those diagnostics
//! plus the static code registry. The pass attribution is registry-derived
//! because the Platform 13 `Diagnostic` value carries no pass field
//! (DISCOVERIES-M8 §1); a code emittable from more than one pass lists
//! every candidate rather than guessing.

use serde::{Deserialize, Serialize};

use clean_compiler_types::{Annotation, Diagnostic, Level, Span, Suggestion};

/// A location inside one of the request's sources, as the caller names it
/// (`<file>:<line>[:<column>]`). Lines and columns are 1-based, counted in
/// characters (Platform 13 §2); `file` is the project-relative path exactly
/// as it appears in the request document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhyQuery {
    pub file: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

/// One pipeline pass (Platform 14 §14.4), by number and name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PassRef {
    pub number: u8,
    pub name: &'static str,
}

/// One diagnostic re-projected at the queried location. Every field except
/// `passes` and `rendered` is carried over verbatim from the diagnostic —
/// the report invents nothing and loses nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhyEntry {
    pub code: String,
    pub level: Level,
    pub message: String,
    pub primary_span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_label: Option<String>,
    /// The pass(es) able to emit this code. Usually one; empty only for
    /// codes with no pipeline home (`COM013` can break anywhere).
    pub passes: Vec<PassRef>,
    pub secondary: Vec<Annotation>,
    pub notes: Vec<String>,
    pub helps: Vec<String>,
    pub suggestions: Vec<Suggestion>,
    pub doc_url: String,
    /// The provenance chain as CLI text. The tree shape is illustrative per
    /// §14.14.1; the diagnostic text inside it follows Platform 13.
    pub rendered: String,
}

/// The re-projection result: every diagnostic whose primary span covers the
/// queried location, in emission order (Platform 13 §10.3). Empty `entries`
/// means the last build reported nothing at that location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhyReport {
    pub query: WhyQuery,
    pub entries: Vec<WhyEntry>,
}

/// The re-projection operation. Pure over its inputs: same diagnostics and
/// same query give the byte-identical report (CMP-02 applies to every
/// operation surface, not only builds).
pub fn why(diagnostics: &[Diagnostic], query: &WhyQuery) -> WhyReport {
    let entries = diagnostics
        .iter()
        .filter(|d| span_contains(&d.primary_span, &query.file, query.line, query.column))
        .map(reproject)
        .collect();
    WhyReport {
        query: query.clone(),
        entries,
    }
}

fn reproject(diagnostic: &Diagnostic) -> WhyEntry {
    let passes = passes_for(&diagnostic.code);
    let rendered = render_tree(diagnostic, &passes);
    WhyEntry {
        code: diagnostic.code.clone(),
        level: diagnostic.level,
        message: diagnostic.message.clone(),
        primary_span: diagnostic.primary_span.clone(),
        primary_label: diagnostic.primary_label.clone(),
        passes,
        secondary: diagnostic.secondary.clone(),
        notes: diagnostic.notes.clone(),
        helps: diagnostic.helps.clone(),
        suggestions: diagnostic.suggestions.clone(),
        doc_url: diagnostic.doc_url.clone(),
        rendered,
    }
}

/// True when `line[:column]` falls inside the span. `end` is exclusive at
/// the column level; a zero-width span (request-document diagnostics) is
/// treated as covering its single position.
fn span_contains(span: &Span, file: &str, line: u32, column: Option<u32>) -> bool {
    if span.file != file {
        return false;
    }
    // A span ending at column 1 of a later line contains nothing on that
    // last line, so the final covered line is the one before it.
    let last_line = if span.end.line > span.start.line && span.end.column == 1 {
        span.end.line - 1
    } else {
        span.end.line
    };
    if line < span.start.line || line > last_line {
        return false;
    }
    let Some(column) = column else { return true };
    let zero_width = span.start == span.end;
    if line == span.start.line && column < span.start.column {
        return false;
    }
    if !zero_width && line == span.end.line && column >= span.end.column {
        return false;
    }
    if zero_width && column != span.start.column {
        return false;
    }
    true
}

const PASSES: [PassRef; 10] = [
    PassRef {
        number: 1,
        name: "Request Validation",
    },
    PassRef {
        number: 2,
        name: "Lex",
    },
    PassRef {
        number: 3,
        name: "Parse",
    },
    PassRef {
        number: 4,
        name: "Resolve",
    },
    PassRef {
        number: 5,
        name: "Type Check",
    },
    PassRef {
        number: 6,
        name: "Block Handler Expansion",
    },
    PassRef {
        number: 7,
        name: "HIR Lowering",
    },
    PassRef {
        number: 8,
        name: "MIR Lowering + Optimize",
    },
    PassRef {
        number: 9,
        name: "World Import Check",
    },
    PassRef {
        number: 10,
        name: "Codegen + Assembly",
    },
];

fn pass(number: u8) -> PassRef {
    PASSES[number as usize - 1]
}

/// Registry-derived pass attribution: which §14.4 pass(es) can emit `code`.
/// Kept total over the compiler-emittable registry by
/// `why_operation::pass_map_covers_every_emittable_code`; the per-code
/// overrides mirror the actual emission sites, the prefix defaults mirror
/// the Platform 09 §3 section preambles.
pub fn passes_for(code: &str) -> Vec<PassRef> {
    let numbers: &[u8] = match code {
        // Lexer-only and lexer-or-parser syntax codes.
        "SYN001" | "SYN003" | "SYN004" | "SYN006" => &[2],
        "SYN002" | "SYN005" => &[2, 3],
        // Missing parentheses is detected on the typed call surface.
        "SYN010" => &[5],
        // Symbol redefinition surfaces in both the resolver and the checker;
        // SEM019 also fires when a block expansion re-checks its output.
        "SEM003" => &[4, 5],
        "SEM019" => &[5, 6],
        "FUNC013" => &[4],
        // Request-level compilation refusals raised by the driver before or
        // during validation.
        "COM005" => &[1],
        "COM002" | "COM003" => &[8],
        "COM007" | "COM012" => &[9],
        // The internal-invariant code has no pipeline home: any pass can
        // break an invariant (CMP-04).
        "COM013" => &[],
        _ => match code.trim_end_matches(|c: char| c.is_ascii_digit()) {
            "RQD" | "BLD" | "CFG" => &[1],
            "SYN" => &[3],
            "IMPORT" => &[4],
            "SEM" | "SCOPE" | "FUNC" | "CLASS" | "IDX" | "STATE" | "MEM" => &[5],
            "BLOCK" | "LIB" => &[6],
            "COM" => &[10],
            _ => &[],
        },
    };
    numbers.iter().map(|&n| pass(n)).collect()
}

/// The provenance tree (shape illustrative per §14.14.1): headline from the
/// diagnostic, then one branch per fact the diagnostic already carries.
fn render_tree(diagnostic: &Diagnostic, passes: &[PassRef]) -> String {
    let span = &diagnostic.primary_span;
    let mut head = format!(
        "{level}[{code}] at {file}:{line}:{column} — {message}",
        level = level_word(diagnostic.level),
        code = diagnostic.code,
        file = span.file,
        line = span.start.line,
        column = span.start.column,
        message = diagnostic.message,
    );
    if let Some(label) = &diagnostic.primary_label {
        head.push_str(&format!(" ({label})"));
    }

    let mut branches = Vec::new();
    match passes {
        [] => branches.push("raised by: an internal invariant check — any pass".to_string()),
        list => branches.push(format!(
            "raised by: {}",
            list.iter()
                .map(|p| format!("pass [{}] {}", p.number, p.name))
                .collect::<Vec<_>>()
                .join(" or ")
        )),
    }
    for note in &diagnostic.notes {
        branches.push(format!("note: {note}"));
    }
    for annotation in &diagnostic.secondary {
        branches.push(format!(
            "related: {} ({}:{}:{})",
            annotation.label,
            annotation.span.file,
            annotation.span.start.line,
            annotation.span.start.column,
        ));
    }
    for help in &diagnostic.helps {
        branches.push(format!("help: {help}"));
    }
    for suggestion in &diagnostic.suggestions {
        branches.push(format!("suggestion: {}", suggestion.message));
    }
    branches.push(format!("see {}", diagnostic.doc_url));

    let mut out = head;
    let last = branches.len() - 1;
    for (i, branch) in branches.iter().enumerate() {
        out.push('\n');
        out.push_str(if i == last { "  └─ " } else { "  ├─ " });
        out.push_str(branch);
    }
    out
}

fn level_word(level: Level) -> &'static str {
    match level {
        Level::Error => "error",
        Level::Warning => "warning",
        Level::Info => "info",
        Level::Help => "help",
    }
}

/// Parses the diagnostics NDJSON the build wrote (Platform 13 §6): one
/// `Diagnostic` object per line, order preserved. `Err` names the first
/// malformed line — a corrupt file is the caller's defect, reported as
/// transport failure, never as a user diagnostic.
pub fn diagnostics_from_ndjson(text: &str) -> Result<Vec<Diagnostic>, String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line)
                .map_err(|e| format!("line {}: not a Diagnostic object: {e}", index + 1))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clean_compiler_types::Position;

    fn span(file: &str, start: (u32, u32), end: (u32, u32)) -> Span {
        Span::new(
            file,
            Position {
                line: start.0,
                column: start.1,
            },
            Position {
                line: end.0,
                column: end.1,
            },
        )
    }

    #[test]
    fn containment_is_line_inclusive_and_column_end_exclusive() {
        let s = span("app/main.cln", (3, 5), (3, 9));
        assert!(span_contains(&s, "app/main.cln", 3, None));
        assert!(span_contains(&s, "app/main.cln", 3, Some(5)));
        assert!(span_contains(&s, "app/main.cln", 3, Some(8)));
        assert!(!span_contains(&s, "app/main.cln", 3, Some(9)));
        assert!(!span_contains(&s, "app/main.cln", 3, Some(4)));
        assert!(!span_contains(&s, "app/main.cln", 2, None));
        assert!(!span_contains(&s, "other.cln", 3, Some(5)));
    }

    #[test]
    fn multiline_span_ending_at_column_1_excludes_that_line() {
        let s = span("a.cln", (2, 1), (4, 1));
        assert!(span_contains(&s, "a.cln", 3, None));
        assert!(!span_contains(&s, "a.cln", 4, None));
    }

    #[test]
    fn zero_width_span_covers_its_single_position() {
        let s = Span::request_document();
        assert!(span_contains(&s, "<request>", 1, None));
        assert!(span_contains(&s, "<request>", 1, Some(1)));
        assert!(!span_contains(&s, "<request>", 1, Some(2)));
    }

    #[test]
    fn rendered_tree_carries_every_fact_of_the_diagnostic() {
        let diagnostic = Diagnostic {
            level: Level::Error,
            code: "COM012".to_string(),
            message: "host import `demo` is not in the target world".to_string(),
            primary_span: span("app/main.cln", (42, 9), (42, 30)),
            primary_label: Some("rejected call".to_string()),
            secondary: vec![Annotation {
                span: span("app/util.cln", (7, 1), (7, 12)),
                label: "declared here".to_string(),
            }],
            notes: vec!["the wasm32-server world imports no such function".to_string()],
            helps: vec!["change target to wasm32-server".to_string()],
            suggestions: vec![],
            doc_url: Diagnostic::doc_url_for("COM012"),
            rendered: String::new(),
        };
        let tree = render_tree(&diagnostic, &passes_for("COM012"));
        let expected = "\
error[COM012] at app/main.cln:42:9 — host import `demo` is not in the target world (rejected call)
  ├─ raised by: pass [9] World Import Check
  ├─ note: the wasm32-server world imports no such function
  ├─ related: declared here (app/util.cln:7:1)
  ├─ help: change target to wasm32-server
  └─ see https://errors.cleanlanguage.dev/E/COM012";
        assert_eq!(tree, expected);
    }
}
