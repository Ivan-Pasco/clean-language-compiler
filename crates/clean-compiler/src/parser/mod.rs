//! Pass [3] — Parse (Platform 14 §14.4.2). Hand-written recursive descent
//! with per-production error recovery (ADR-0006); every AST node carries a
//! real source span. Grammar source of truth: the EBNF files under
//! `foundation/04 language/grammar/` (DOC-15). Lands in Milestone 1 step 5.
