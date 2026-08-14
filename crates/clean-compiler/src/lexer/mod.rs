//! Pass [2] — Lex (Platform 14 §14.4.2). Hand-written lexer (ADR-0006):
//! per-file `TokenStream` with byte-accurate spans; comment spans preserved
//! for the LSP. Syntax authority: `04 language/grammar/03-lexical-structure.ebnf.md`
//! (DOC-15) — tab-structured indentation (LEX-01), CRLF normalisation
//! (LEX-07), exact-case keywords (LEX-08), nesting block comments (LEX-09).

mod scan;
mod token;

pub use scan::lex;
pub use token::{Kw, Token, TokenKind, TokenStream};
