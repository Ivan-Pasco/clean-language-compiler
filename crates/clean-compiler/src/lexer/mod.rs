//! Pass [2] — Lex (Platform 14 §14.4.2). Hand-written lexer (ADR-0006):
//! per-file `TokenStream` with byte-accurate spans; comment spans preserved
//! for the LSP. Syntax authority: the Lark companion
//! `governance/product/language-principles/03-lexical-structure.lark.md`
//! (DOC-15, FS-01) — tab-structured indentation (LEX-01), CRLF normalisation
//! (LEX-07), exact-case keywords (LEX-08), nesting block comments (LEX-09).

mod scan;
mod token;

pub use scan::lex;
pub use token::{is_reserved_word, plain_text, Kw, StrPart, Token, TokenKind, TokenStream};
