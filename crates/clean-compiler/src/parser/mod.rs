//! Pass [3] — Parse (Platform 14 §14.4.2). Hand-written recursive descent
//! with per-line error recovery (ADR-0006); every AST node has a real span.
//! Grammar authority: `foundation/04 language/grammar/*.ebnf.md` plus the
//! LBS-02 host-bridge productions (DOC-15).

pub mod ast;
mod parse;

pub use parse::parse;
