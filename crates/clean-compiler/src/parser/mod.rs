//! Pass [3] — Parse (Platform 14 §14.4.2). Hand-written recursive descent
//! with per-line error recovery (ADR-0006); every AST node has a real span.
//! Grammar authority: the Lark companions
//! `foundation/governance/product/language-principles/*.lark.md` (FS-01) plus the
//! LBS-02 host-bridge rules (DOC-15).

pub mod ast;
mod parse;

pub use parse::{parse, parse_with_limit};
