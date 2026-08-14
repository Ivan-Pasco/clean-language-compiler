//! The `Diagnostic` value, exactly as Platform 13 defines it (§2, §6).
//!
//! Serialization notes (Platform 13 §6): `diagnostics.json` is NDJSON — one
//! object per line, `snake_case` fields, `rendered` carrying the exact CLI
//! text. JSON output preserves emission order (§10.3). Consumers MUST ignore
//! unknown fields (DIA-04), so deserialization here is not `deny_unknown_fields`.

use serde::{Deserialize, Serialize};

use crate::span::Span;

/// Diagnostic severity level (Platform 13 §2). `Runtime`-severity codes from
/// the registry render as `error` (Platform 09 §3): there is no fifth level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warning,
    Info,
    Help,
}

/// A secondary span with its label (label ≤ 80 chars, Platform 13 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotation {
    pub span: Span,
    pub label: String,
}

/// Whether a suggestion can be applied mechanically (Platform 13 §5, DIA-03).
/// `Unspecified` is treated as `MaybeIncorrect` by consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Applicability {
    MachineApplicable,
    HasPlaceholders,
    MaybeIncorrect,
    Unspecified,
}

/// One concrete text replacement inside a suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    pub span: Span,
    pub replacement: String,
}

/// A suggested fix (Platform 13 §5). Placeholders use `__NAME__` delimiters;
/// equal names form one tab-stop group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggestion {
    pub message: String,
    pub replacements: Vec<Replacement>,
    pub applicability: Applicability,
}

/// A single compiler finding (Platform 13 §2). Every diagnostic carries a
/// registered code (DIA-01) and the exact rendered CLI text (§6.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: Level,
    /// Registered code, e.g. `SEM001` (Platform 09).
    pub code: String,
    /// One-line headline, ≤ 100 chars, no trailing punctuation (DIA-02).
    pub message: String,
    pub primary_span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_label: Option<String>,
    pub secondary: Vec<Annotation>,
    pub notes: Vec<String>,
    pub helps: Vec<String>,
    pub suggestions: Vec<Suggestion>,
    /// `https://errors.cleanlanguage.dev/E/<CODE>`.
    pub doc_url: String,
    /// The exact CLI text the compiler would print for this diagnostic
    /// (Platform 13 §6.1 — required on every JSON diagnostic).
    pub rendered: String,
}

impl Diagnostic {
    /// Deduplication key per Platform 13 §10.2.
    pub fn dedup_key(&self) -> (String, Span, String) {
        (
            self.code.clone(),
            self.primary_span.clone(),
            self.message.clone(),
        )
    }

    pub fn doc_url_for(code: &str) -> String {
        format!("https://errors.cleanlanguage.dev/E/{code}")
    }
}
