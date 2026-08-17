//! The Clean compiler: one request document in, one artifact set out
//! (Platform 14). `compile` is the canonical entry point (§14.2.1); the
//! process adapter and every future transport are wrappers over it.
//!
//! Pipeline modules map 1:1 to the passes in §14.4:
//!
//! | Pass | Module | Output |
//! |------|--------|--------|
//! | [1] Request Validation | [`request`] | `ValidatedRequest` |
//! | [2] Lex | [`lexer`] | per-file `TokenStream` |
//! | [3] Parse | [`parser`] | per-file AST |
//! | [4] Resolve | [`resolver`] | `ResolvedAST` |
//! | [5] Type Check | [`typecheck`] | `TypedAST` |
//! | [6] Block Handler Expansion | [`blocks`] | `TypedAST'` |
//! | [7] HIR Lowering | [`hir`] | HIR |
//! | [8] MIR Lowering + Optimize | [`mir`] | MIR |
//! | [9] World Import Check | [`codegen`] | verified against target world |
//! | [10] Codegen + Assembly | [`codegen`] | `component.wasm` |
//!
//! Pass contracts (§14.4.1): one input type, one output type, no I/O,
//! append-only `DiagnosticSink`, deterministic iteration (`BTreeMap` /
//! `IndexMap` or explicit sort — never hash-order in emitting paths).

pub mod blocks;
pub mod codegen;
pub mod diag;
pub mod driver;
pub mod hir;
pub mod lexer;
pub mod mir;
pub mod parser;
pub mod request;
pub mod resolver;
pub mod source;
pub mod typecheck;

pub use clean_compiler_types as types;
pub use driver::{check, compile, emit_hir, CompileArtifact, CompileError};
