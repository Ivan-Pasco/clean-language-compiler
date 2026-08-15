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

/// SYN001 — `InvalidToken`: character not valid in any lexical context.
pub const SYN001: &str = "SYN001";
/// SYN002 — `UnexpectedToken`: a token where the grammar does not allow it.
pub const SYN002: &str = "SYN002";
/// SYN003 — `InvalidIndentation`: spaces used for indentation, or nesting
/// inconsistent with the enclosing block.
pub const SYN003: &str = "SYN003";
/// SYN004 — `UnterminatedConstruct`: string, comment, or block left open.
pub const SYN004: &str = "SYN004";
/// SYN005 — `MalformedConstruct`: partially correct syntax structure.
pub const SYN005: &str = "SYN005";
/// SYN006 — `IndentationError`: tab/space mixing or wrong level.
pub const SYN006: &str = "SYN006";
/// SYN007 — `SectionOutOfOrder`: top-level sections out of FIL-01 order.
pub const SYN007: &str = "SYN007";
/// SYN008 — `InvalidPrintBlock`: `print:` with no expressions.
pub const SYN008: &str = "SYN008";
/// SYN009 — `NotATopLevelForm`: construct not permitted at the top level.
pub const SYN009: &str = "SYN009";
/// SYN010 — `MissingParentheses`: call written without parentheses.
pub const SYN010: &str = "SYN010";

/// SEM001 — `AssignTypeMismatch`.
pub const SEM001: &str = "SEM001";
/// SEM002 — `UndefinedVariable`.
pub const SEM002: &str = "SEM002";
/// SEM003 — `SymbolRedefinition`.
pub const SEM003: &str = "SEM003";
/// SEM004 — `InvalidOperationForType`.
pub const SEM004: &str = "SEM004";
/// SEM015 — `ReturnTypeMismatch`.
pub const SEM015: &str = "SEM015";
/// SEM016 — `ArgumentTypeMismatch`.
pub const SEM016: &str = "SEM016";
/// SEM019 — `UndefinedFunction`.
pub const SEM019: &str = "SEM019";
/// SEM020 — `UndefinedClass`.
pub const SEM020: &str = "SEM020";
/// SEM023 — `NonBooleanCondition`.
pub const SEM023: &str = "SEM023";
/// SEM026 — `LiteralOutOfRange`.
pub const SEM026: &str = "SEM026";
/// SCOPE002 — `VariableCannotBeRedeclaredInSameScope` (resolver-phase form).
pub const SCOPE002: &str = "SCOPE002";
/// FUNC002 — `ArgumentCountMustMatchParameterCount`.
pub const FUNC002: &str = "FUNC002";
/// FUNC003 — `CannotCallNonFunctionType`.
pub const FUNC003: &str = "FUNC003";
/// FUNC005 — `EmptyReturnInNonVoidFunction` (Warning).
pub const FUNC005: &str = "FUNC005";

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
