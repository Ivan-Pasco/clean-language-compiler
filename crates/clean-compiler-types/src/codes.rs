//! Error-code constants the compiler emits, mirroring the registry in
//! Platform 09 (`PREFIX###`, three digits). Message templates live with the
//! emitting pass and are copied verbatim from Platform 10 — never redacted.
//!
//! Milestone 1 registers only the codes its passes can emit; the full
//! 161-code table lands in M2 with the 1:1 code↔rule↔snapshot CI gate.

/// Registry severity (Platform 09 §3). `Runtime` marks the *phase*, not a
/// fifth diagnostic level — it renders as `error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Runtime,
}

/// RQD001 — `RequestIntegrityFailure`: a `sources[].sha256` does not match
/// the decoded content (Platform 14 §14.1.1, CMP-01).
pub const RQD001: &str = "RQD001";

/// RQD002 — `RequestSchemaViolation`: unknown key, missing required field,
/// or malformed value in the request document (Platform 10 §16).
pub const RQD002: &str = "RQD002";

/// COM012 — `HostImportNotInWorld`: a `host function` call site is absent
/// from the target world (Platform 14 pass [9], CMP-03).
pub const COM012: &str = "COM012";

/// COM013 — `InternalInvariant`: the compiler broke its own invariant; always
/// presented as a compiler bug, never a user error (CMP-04).
pub const COM013: &str = "COM013";
