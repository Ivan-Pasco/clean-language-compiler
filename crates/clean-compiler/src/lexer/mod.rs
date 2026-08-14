//! Pass [2] — Lex (Platform 14 §14.4.2). Hand-written lexer (ADR-0006):
//! per-file `TokenStream` with byte-accurate spans; comment tokens preserved
//! for the LSP. Syntax derives from `foundation/04 language/grammar/`
//! (DOC-15). Lands in Milestone 1 step 5.
