//! The Clean language server: an LSP transport over the compiler pipeline.
//!
//! Platform 04 owns the protocol contract (LSP-01: the server is the single
//! source of truth for language intelligence; extensions are thin clients).
//! Platform 13 §7 owns the `Diagnostic` → LSP mapping. This crate owns the
//! implementation and its packaging (component spec §9); it shares the
//! compiler's lexer, parser, and type checker by calling `clean_compiler`
//! directly (CCMP-25) — it registers no diagnostic codes and carries no
//! language knowledge of its own.
//!
//! `run` drives a [`lsp_server::Connection`], so the same loop serves stdio
//! in production and an in-memory pair in tests.

mod analysis;
mod convert;
mod server;
mod session;

pub use server::{run, ServerError};
