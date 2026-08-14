//! Source positions and spans, as serialized in `diagnostics.json`
//! (Platform 13 §2: 1-based lines and columns, columns counted in
//! characters; `end` is exclusive).

use serde::{Deserialize, Serialize};

/// A 1-based line/column position, counted in characters (Platform 13 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// A source region. `file` is the project-relative, forward-slashed path from
/// the request document; `end` is exclusive and `end.line >= start.line`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(file: impl Into<String>, start: Position, end: Position) -> Self {
        Self {
            file: file.into(),
            start,
            end,
        }
    }

    /// Span for diagnostics about the request document itself (RQD codes),
    /// which have no `.cln` source to point at. The pseudo-file name is part
    /// of the compiler's observable output, so it is defined once, here.
    pub fn request_document() -> Self {
        Self {
            file: "<request>".to_string(),
            start: Position { line: 1, column: 1 },
            end: Position { line: 1, column: 1 },
        }
    }
}
