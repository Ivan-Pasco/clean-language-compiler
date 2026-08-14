//! Stable value types shared by the compiler, its binary adapter, and
//! external tools (LSP, framework, CI): spans, diagnostics, error codes,
//! the compilation request document, and the build manifest.
//!
//! This crate holds no I/O and no wasm dependencies (ADR-0006): it is the
//! cheap, stable surface. Behaviour lives in `clean-compiler`.

pub mod codes;
pub mod diagnostic;
pub mod manifest;
pub mod request;
pub mod source_map;
pub mod span;

pub use codes::Severity;
pub use diagnostic::{Annotation, Applicability, Diagnostic, Level, Replacement, Suggestion};
pub use manifest::BuildManifest;
pub use request::CompileRequest;
pub use source_map::SourceMap;
pub use span::{Position, Span};
