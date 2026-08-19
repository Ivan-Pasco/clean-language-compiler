//! The complete diagnostic-code registry, mirroring Platform 09 §3 row for
//! row. Every diagnostic the toolchain can emit carries one of these codes
//! (DIA-01); the message templates are copied **verbatim** from Platform 10
//! (never redacted), and the six withdrawn identifiers are retained so they
//! can never be reused (DOC-13).
//!
//! Counts per Platform 09 §1.1 (M4 registry pass, 2026-08-17): 165 rows
//! registered, 8 withdrawn (`SCOPE005`, `FUNC001`, `CLASS007`, `LIB005`,
//! `LIB007`, `LIB008`, `LIB009`, `IMPORT005` — never emitted, never
//! reused), 157 emittable.
//!
//! The 1:1 obligation (ERC-02 / RUL-02) is enforced by
//! `tests/registry_spec.rs` against the local `clean-language-foundation`
//! checkout when present, and the DIA-06 fixture harness in
//! `clean-compiler/tests/` keys off [`CodeInfo::is_compiler_emittable`].

/// Severity as registered in Platform 09 §3. `Runtime` marks the *phase*,
/// not a fifth diagnostic level — a Runtime code renders as `error`
/// (Platform 09 §2). `PerDiagnostic` is `LIB010` only: the wrapper travels
/// at the level of the wrapped library-authored diagnostic
/// (Error / Warning / Info).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Runtime,
    PerDiagnostic,
}

/// Which component raises the code (Platform 09 §3 section preambles).
/// Only [`Emitter::Compiler`] codes can ever appear in this compiler's
/// output, and only they participate in the DIA-06 fixture harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emitter {
    /// The compiler pipeline, passes 1–10 (includes the block-handler
    /// sandbox of pass 6 and the library loader).
    Compiler,
    /// Clean Framework at build orchestration time (Moment 1/2 checks,
    /// capability wiring).
    Framework,
    /// A host runtime during WASM execution or instantiation.
    Host,
    /// The components that read project files from disk — Clean Framework,
    /// Clean Manager, the language server. Never the compiler (CMP-01).
    Toolchain,
}

/// Registration status (DOC-13: IDs are never renumbered or reused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Active,
    /// Retired identifier. It MUST never be emitted and never reused; the
    /// row is retained as the permanent record.
    Withdrawn,
}

/// One row of the Platform 09 §3 code index.
#[derive(Debug, Clone, Copy)]
pub struct CodeInfo {
    /// The `PREFIX###` code (ERC-01).
    pub code: &'static str,
    /// The registry symbolic name (e.g. `AssignTypeMismatch`). Empty only
    /// for the four withdrawn `LIB` rows whose 09 index shows no name.
    pub name: &'static str,
    pub status: Status,
    /// `None` iff withdrawn.
    pub severity: Option<Severity>,
    /// `None` iff withdrawn.
    pub emitter: Option<Emitter>,
    /// The literal message template from Platform 10 (or the `format` line
    /// for `LIB010`), copied verbatim including `{placeholder}` / `<slot>`
    /// spelling. `None` where the rule body defines no template yet — a
    /// stub, which RUL-03 declares a spec defect, not ours to invent.
    pub template: Option<&'static str>,
}

impl CodeInfo {
    /// True for codes this compiler may emit: active rows whose emitter is
    /// the compiler pipeline. The DIA-06 harness requires each of these to
    /// have either a fixture triple or an `unimplemented.txt` entry.
    pub fn is_compiler_emittable(&self) -> bool {
        self.status == Status::Active && self.emitter == Some(Emitter::Compiler)
    }
}

/// Looks a code up by its `PREFIX###` string.
pub fn lookup(code: &str) -> Option<&'static CodeInfo> {
    REGISTRY.iter().find(|info| info.code == code)
}

macro_rules! code_consts {
    ($($code:ident),* $(,)?) => {
        $(pub const $code: &str = stringify!($code);)*
    };
}

// One `pub const` per live (non-withdrawn) code, so passes reference codes
// as `codes::SEM001` and a typo is a compile error. Withdrawn identifiers
// deliberately get no constant: nothing can emit them.
#[rustfmt::skip]
code_consts!(
    SYN001, SYN002, SYN003, SYN004, SYN005, SYN006, SYN007, SYN008, SYN009,
    SYN010, SYN100, SYN101,
    SEM001, SEM002, SEM003, SEM004, SEM005, SEM006, SEM007, SEM008, SEM009,
    SEM010, SEM011, SEM012, SEM013, SEM014, SEM015, SEM016, SEM017, SEM018,
    SEM019, SEM020, SEM021, SEM022, SEM023, SEM024, SEM025, SEM026, SEM027,
    SEM028,
    SCOPE001, SCOPE002, SCOPE003, SCOPE004, SCOPE006,
    FUNC001, FUNC002, FUNC003, FUNC004, FUNC005, FUNC006, FUNC007, FUNC008,
    FUNC009, FUNC010, FUNC011, FUNC012, FUNC013, FUNC014, FUNC015,
    CLASS001, CLASS002, CLASS003, CLASS004, CLASS005, CLASS006, CLASS007,
    CLASS008, CLASS009, CLASS010, CLASS011, CLASS012,
    IDX001, IDX002, IDX003, IDX004, IDX005,
    STATE001, STATE002, STATE003, STATE004, STATE005, STATE006,
    IMPORT001, IMPORT002, IMPORT003, IMPORT004,
    LIB001, LIB002, LIB003, LIB004, LIB006, LIB010, LIB011, LIB012, LIB013,
    LIB014, LIB015, LIB016, LIB017, LIB018, LIB019, LIB020,
    COM001, COM002, COM003, COM004, COM005, COM006, COM007, COM008, COM009,
    COM010, COM011, COM012, COM013, COM014, COM015, COM016, COM017,
    BLD001,
    RUN001, RUN002, RUN003, RUN004, RUN005, RUN006, RUN007, RUN008, RUN009,
    RUN010, RUN011, RUN012, RUN013, RUN014, RUN015, RUN016, RUN017, RUN018,
    RUN019,
    MEM001, MEM002, MEM003,
    BLOCK001, BLOCK002, BLOCK003, BLOCK004, BLOCK005, BLOCK006,
    CFG001, CFG002, CFG003, CFG004, CFG005,
    RQD001, RQD002,
    CAP001, CAP002, CAP003,
);

/// Active row without a message template (the rule body is a stub or defines
/// the condition by reference — RUL-03).
const fn c(
    code: &'static str,
    name: &'static str,
    severity: Severity,
    emitter: Emitter,
) -> CodeInfo {
    CodeInfo {
        code,
        name,
        status: Status::Active,
        severity: Some(severity),
        emitter: Some(emitter),
        template: None,
    }
}

/// Active row with its literal Platform 10 message template.
const fn t(
    code: &'static str,
    name: &'static str,
    severity: Severity,
    emitter: Emitter,
    template: &'static str,
) -> CodeInfo {
    CodeInfo {
        code,
        name,
        status: Status::Active,
        severity: Some(severity),
        emitter: Some(emitter),
        template: Some(template),
    }
}

/// Withdrawn row — the identifier is retained and never reused (DOC-13).
const fn w(code: &'static str, name: &'static str) -> CodeInfo {
    CodeInfo {
        code,
        name,
        status: Status::Withdrawn,
        severity: None,
        emitter: None,
        template: None,
    }
}

use Emitter::{Compiler, Framework, Host, Toolchain};
use Severity::{Error, PerDiagnostic, Runtime, Warning};

/// The full Platform 09 §3 index — 165 rows, in registry order.
#[rustfmt::skip]
pub const REGISTRY: &[CodeInfo] = &[
    // §3.1 Syntax (SYN) — parsing; only tokens are known.
    c(SYN001, "InvalidToken", Error, Compiler),
    c(SYN002, "UnexpectedToken", Error, Compiler),
    c(SYN003, "InvalidIndentation", Error, Compiler),
    c(SYN004, "UnterminatedConstruct", Error, Compiler),
    c(SYN005, "MalformedConstruct", Error, Compiler),
    c(SYN006, "IndentationError", Error, Compiler),
    c(SYN007, "SectionOutOfOrder", Error, Compiler),
    t(SYN008, "InvalidPrintBlock", Error, Compiler,
      "print: block must contain at least one expression"),
    t(SYN009, "NotATopLevelForm", Error, Compiler,
      "'{construct}' cannot appear at the top level of a file"),
    t(SYN010, "MissingParentheses", Error, Compiler,
      "Call to '{name}' is missing parentheses"),
    t(SYN100, "MissingSpecPath", Error, Compiler,
      "Expected string literal after 'spec'"),
    t(SYN101, "MissingIntentDescription", Error, Compiler,
      "Expected string literal after 'intent'"),

    // §3.2 Semantic (SEM) — HIR validation, resolver, type checker.
    t(SEM001, "AssignTypeMismatch", Error, Compiler,
      "type mismatch in assignment"),
    t(SEM002, "UndefinedVariable", Error, Compiler,
      "I cannot find a variable named `<name>` in scope"),
    c(SEM003, "SymbolRedefinition", Error, Compiler),
    t(SEM004, "InvalidOperationForType", Error, Compiler,
      "operator `<op>` is not defined for type `<T>`"),
    t(SEM005, "AccessViolation", Error, Compiler,
      "'{name}' is private and cannot be accessed from outside '{scope}'"),
    c(SEM006, "InheritanceError", Error, Compiler),
    c(SEM007, "GenericTypeError", Error, Compiler),
    c(SEM008, "InheritanceCycle", Error, Compiler),
    t(SEM009, "InvalidTypeSpecification", Error, Compiler,
      "`<T>?` is already optional: absence does not stack"),
    c(SEM010, "InvalidMatchPattern", Error, Compiler),
    c(SEM011, "MissingCapabilityMethod", Error, Compiler),
    c(SEM012, "UndefinedCapability", Error, Compiler),
    c(SEM013, "CapabilityMethodSignatureMismatch", Error, Compiler),
    c(SEM014, "CapabilityBodyNotAllowed", Error, Compiler),
    t(SEM015, "ReturnTypeMismatch", Error, Compiler,
      "return type mismatch in `<fn>`"),
    t(SEM016, "ArgumentTypeMismatch", Error, Compiler,
      "argument `<n>` of `<fn>` has the wrong type"),
    t(SEM017, "StateInitializerTypeMismatch", Error, Compiler,
      "state initializer for `<name>` has the wrong type"),
    t(SEM018, "ComputedBodyTypeMismatch", Error, Compiler,
      "computed state `<name>` returns the wrong type"),
    t(SEM019, "UndefinedFunction", Error, Compiler,
      "I cannot find a function named `<name>`"),
    t(SEM020, "UndefinedClass", Error, Compiler,
      "I cannot find a class named `<name>`"),
    t(SEM021, "UndefinedModule", Error, Compiler,
      "I cannot resolve the module `<name>`"),
    t(SEM022, "UndefinedMethod", Error, Compiler,
      "type `<T>` has no method named `<method>`"),
    t(SEM023, "NonBooleanCondition", Error, Compiler,
      "Condition must be a boolean expression, found {type}"),
    t(SEM024, "ExpectedValueNotConstant", Error, Compiler,
      "Expected value must be evaluable at compile time"),
    t(SEM025, "ControlFlowOutsideLoop", Error, Compiler,
      "'{keyword}' is not inside a loop"),
    t(SEM026, "LiteralOutOfRange", Error, Compiler,
      "literal {value} does not fit {type} (range {min} to {max})"),
    t(SEM027, "LossyIntegerPromotion", Warning, Compiler,
      "integer value {value} exceeds 2^53 and loses precision as a number"),
    t(SEM028, "UndefinedField", Error, Compiler,
      "type `<T>` has no field named `<field>`"),

    // §3.3 Scope (SCOPE) — resolver.
    c(SCOPE001, "UseBeforeDeclaration", Error, Compiler),
    c(SCOPE002, "RedeclarationInScope", Error, Compiler),
    c(SCOPE003, "MaxScopeDepthExceeded", Error, Compiler),
    c(SCOPE004, "WatchTargetNotState", Error, Compiler),
    w("SCOPE005", "ScreenStateAccess"),
    t(SCOPE006, "CompiletimeHelperOutsideTest", Error, Compiler,
      "'test.compiletime.{name}' is only available inside a 'tests:' block"),

    // §3.4 Function (FUNC) — HIR validation of definitions and calls.
    w(FUNC001, "FunctionNotDefined"),
    t(FUNC002, "ArgumentCountMismatch", Error, Compiler,
      "function '{name}' expects between {required} and {total} arguments, found {count}"),
    c(FUNC003, "CallOnNonFunction", Error, Compiler),
    c(FUNC004, "MissingReturn", Warning, Compiler),
    c(FUNC005, "EmptyReturnInNonVoid", Warning, Compiler),
    c(FUNC006, "StartBlockHasParameters", Error, Compiler),
    c(FUNC007, "StartBlockReturnsValue", Warning, Compiler),
    c(FUNC008, "UnknownNamedArgument", Error, Compiler),
    c(FUNC009, "DuplicateNamedArgument", Error, Compiler),
    c(FUNC010, "PositionalAfterNamed", Error, Compiler),
    c(FUNC011, "NamedArgCoverageError", Error, Compiler),
    c(FUNC012, "MethodCallOnStandaloneFunction", Error, Compiler),
    t(FUNC013, "FunctionOutsideFunctionsBlock", Error, Compiler,
      "Function '{name}' must be declared inside a 'functions:' block"),
    t(FUNC014, "OptionalParameterOrder", Error, Compiler,
      "Parameter '{name}' has no default and follows '{previous}', which has one"),
    t(FUNC015, "DuplicateStartBlock", Error, Compiler,
      "file declares more than one 'start:' block"),

    // §3.5 Class (CLASS) — HIR validation of class definitions.
    c(CLASS001, "ParentClassNotFound", Error, Compiler),
    c(CLASS002, "DuplicateField", Error, Compiler),
    c(CLASS003, "DuplicateMethod", Error, Compiler),
    c(CLASS004, "MissingConstructor", Error, Compiler),
    t(CLASS005, "AfterBeforeLogic", Error, Compiler,
      "'after:' must follow 'before:' at the top of the function body"),
    t(CLASS006, "AlwaysConditionNotBoolean", Error, Compiler,
      "expression inside 'always:' must be a boolean expression, found {type}"),
    w(CLASS007, "ContractBlockOutOfPosition"),
    t(CLASS008, "ResultOutsideAfter", Error, Compiler,
      "'result' is only in scope inside an 'after:' expression"),
    t(CLASS009, "ContractSideEffect", Error, Compiler,
      "Contract expression must be pure: '{operation}' is not allowed here"),
    t(CLASS010, "ConstructorParameterShadowsField", Error, Compiler,
      "Constructor parameter '{name}' has the same name as a field"),
    t(CLASS011, "CapabilityInstantiated", Error, Compiler,
      "'{name}' is a capability and cannot be instantiated"),
    t(CLASS012, "InvalidCompanionAccess", Error, Compiler,
      "'{receiver}.{member}' is not a valid companion access: {reason}"),

    // §3.6 Index access (IDX) — type checker.
    t(IDX001, "ListIndexNotInteger", Error, Compiler,
      "list index must be `integer`, found `<T>`"),
    t(IDX002, "MatrixIndexNotInteger", Error, Compiler,
      "matrix index must be `integer`, found `<T>`"),
    t(IDX003, "PairsKeyTypeMismatch", Error, Compiler,
      "`pairs<K, V>` is indexed with `<K>`, found `<T>`"),
    t(IDX004, "IndexOnNonIndexable", Error, Compiler,
      "type `<T>` does not support bracket access"),
    t(IDX005, "IndexOnNone", Error, Compiler,
      "cannot index `<name>` because it may be `none`"),

    // §3.7 State (STATE) — HIR validation (compile time) and host (runtime).
    t(STATE001, "GuardConditionNotPure", Error, Compiler,
      "Guard condition must be a pure boolean expression"),
    t(STATE002, "GuardRejection", Runtime, Host,
      "State update rejected: {guard_message}"),
    t(STATE003, "CircularStateDependency", Error, Compiler,
      "Circular dependency in computed state: '{name}' depends on itself"),
    t(STATE004, "ComputedStateAssignment", Error, Compiler,
      "Cannot assign to computed state variable '{name}': it is read-only"),
    t(STATE005, "RulesExpressionNotBoolean", Error, Compiler,
      "State rule expression must be a boolean expression, got {type}"),
    t(STATE006, "StateRuleViolated", Runtime, Host,
      "state rule violated: {rule_text}"),

    // §3.8 Import (IMPORT) — module resolver.
    t(IMPORT001, "CircularDependency", Error, Compiler,
      "import cycle detected: {A → B → ... → A}"),
    c(IMPORT002, "ModuleNotFound", Error, Compiler),
    c(IMPORT003, "SymbolNotInModule", Error, Compiler),
    c(IMPORT004, "DuplicateImportItem", Error, Compiler),
    w("IMPORT005", "ImportCycle"),

    // §3.9 Library (LIB) — library resolver, loader, compile-time driver.
    t(LIB001, "LibraryNotFound", Error, Compiler,
      "Library '{name}' not found in any configured source"),
    t(LIB002, "LibraryVersionConflict", Error, Compiler,
      "Library '{name}' has conflicting version requirements: {list_of_ranges}"),
    t(LIB003, "LibraryCyclicDependency", Error, Compiler,
      "Cyclic library dependency detected: {A → B → ... → A}"),
    t(LIB004, "LibraryManifestInvalid", Error, Compiler,
      "Library '{path}' has invalid manifest: {reason}"),
    w("LIB005", ""),
    t(LIB006, "UnknownBlockHandler", Error, Compiler,
      "Library '{lib}' declares a handler for '{block}' but does not own that namespace"),
    w("LIB007", ""),
    w("LIB008", ""),
    w("LIB009", ""),
    // LIB010's "template" is the wrapper format line of Platform 10 §10.3.
    t(LIB010, "CompiletimeDiagnostic", PerDiagnostic, Compiler,
      "{file}:{line}:{col}: {severity} [LIB010 via {library}::{function}] {message}"),
    t(LIB011, "HostFunctionSignatureMismatch", Error, Compiler,
      "Host function '{name}' from module '{host}' expected signature {expected}, library declares {actual}"),
    t(LIB012, "HostFunctionUnbound", Error, Compiler,
      "Host interface '{host}' required by library '{lib}' is not available for target '{target}'"),
    t(LIB013, "HostFunctionSandboxDenied", Error, Compiler,
      "Library '{lib}' requires host interface '{host}' which is not in [security].allowedHostModules"),
    t(LIB014, "LibraryResourceLimit", Error, Compiler,
      "Library '{lib}' exceeded {limit_name}: {actual} > {max}"),
    t(LIB015, "CapabilityNotImplemented", Error, Compiler,
      "Type '{type}' claims capability '{cap}' but does not implement required method '{method}'"),
    t(LIB016, "CapabilityConflict", Error, Compiler,
      "Capability '{name}' is defined by both '{libA}' and '{libB}' with incompatible signatures"),
    t(LIB017, "FolderScopeUnclaimed", Error, Compiler,
      "Folder scope '{folder}' maps to library '{lib}' which is not a declared dependency"),
    t(LIB018, "FolderScopeAmbiguous", Error, Compiler,
      "Namespace '{ns}' in folder '{folder}' is claimed by both '{libA}' and '{libB}'"),
    t(LIB019, "HostBridgeMisplaced", Error, Compiler,
      "Host declaration '{name}' must live in host_bridge.cln (found in '{file}')"),
    t(LIB020, "SourceBlockMalformed", Error, Compiler,
      "'source:' block is malformed: {reason}"),

    // §3.10 Compilation (COM) — codegen through instantiation. COM014/015
    // are Clean Framework's Moment 1/2 checks; COM011/017 are the host's
    // Moment 3 (Platform 16 §16.4).
    c(COM001, "WasmGenerationError", Error, Compiler),
    c(COM002, "OptimizationError", Error, Compiler),
    t(COM003, "MemoryLayoutError", Error, Compiler,
      "static data (<emitted> bytes) exceeds the data region (<available> bytes below HEAP_START)"),
    c(COM004, "ModuleResolutionError", Error, Compiler),
    t(COM005, "TargetFeatureUnsupported", Error, Compiler,
      "target `<target>` does not support `<feature>`"),
    c(COM006, "FunctionNotFoundDuringCompilation", Error, Compiler),
    t(COM007, "TargetHostModuleMissing", Error, Compiler,
      "target `<target>` does not provide host module `<module>`"),
    t(COM008, "TargetSizeBudgetExceeded", Error, Compiler,
      "output size <bytes> exceeds target budget <max> for `<target>`"),
    t(COM009, "BridgeResolveError", Error, Compiler,
      "no version of '{package}' satisfies all constraints: {constraint_list}"),
    t(COM010, "BridgeLinkError", Error, Compiler,
      "link check failed for '{interface}': {mismatch}"),
    t(COM011, "BridgeRuntimeMismatch", Runtime, Host,
      "cannot instantiate component: requires {package}@{expected}, host provides {package}@{actual}"),
    t(COM012, "HostImportNotInWorld", Error, Compiler,
      "your program uses '{interface}' but you compiled for '{world}', which does not provide it"),
    t(COM013, "CodegenInternalInvariant", Error, Compiler,
      "internal compiler invariant violated: {invariant} — this is a compiler bug, please report it"),
    t(COM014, "WorldMismatch", Error, Framework,
      "target host '{host}' does not provide required interface '{interface}'"),
    t(COM015, "VersionMismatch", Error, Framework,
      "host '{host}' provides '{package}@{actual}', component requires '{package}@{expected}'"),
    t(COM016, "DeprecatedMemberUse", Warning, Compiler,
      "'{member}' is deprecated: {deprecation_message}"),
    t(COM017, "InstantiationFailure", Runtime, Host,
      "cannot instantiate component '{component}': {cause}"),

    // §3.11 Build (BLD) — multi-file compiler, build-scoped limits.
    t(BLD001, "BuildLimitExceeded", Error, Compiler,
      "build limit '{limit}' exceeded: {actual} > {max}"),

    // §3.12 Runtime (RUN) — raised during WASM execution by the host.
    // RUN013–RUN015 carry severity Error in the 09 index; the rest Runtime.
    c(RUN001, "MemoryViolation", Runtime, Host),
    c(RUN002, "StackError", Runtime, Host),
    c(RUN003, "ArithmeticError", Runtime, Host),
    c(RUN004, "ReferenceError", Runtime, Host),
    c(RUN005, "AssertionFailure", Runtime, Host),
    c(RUN006, "JsonParseError", Runtime, Host),
    t(RUN007, "JsonInvalidNumber", Runtime, Host,
      "invalid JSON number at offset {offset}: {reason}"),
    c(RUN008, "JsonInvalidString", Runtime, Host),
    t(RUN009, "JsonInvalidStructure", Runtime, Host,
      "invalid JSON structure at offset {offset}: {reason}"),
    t(RUN010, "JsonDepthExceeded", Runtime, Host,
      "JSON nesting exceeded {limit} levels"),
    t(RUN011, "ContractViolation", Runtime, Host,
      "Contract violation: {after|always} failed at {location}\n  Expression: {expression}"),
    t(RUN012, "TimeBudgetExceeded", Runtime, Host,
      "time budget exceeded: invocation ran {elapsed} ms, budget is {budget} ms"),
    t(RUN013, "IndexOutOfRange", Error, Host,
      "Index {index} is out of range for a {kind} of length {length}"),
    t(RUN014, "FileOperationFailed", Error, Host,
      "File operation failed on '{path}': {reason}"),
    t(RUN015, "HttpRequestFailed", Error, Host,
      "HTTP request to '{url}' failed: {reason}"),
    // RUN016 also defines a single-operand variant: "matrix operation
    // '{op}' requires a square matrix, got {shape}".
    t(RUN016, "MatrixShapeMismatch", Runtime, Host,
      "matrix operation '{op}' does not admit shapes {left} and {right}"),
    t(RUN017, "MatrixSingular", Runtime, Host,
      "matrix is singular: determinant is zero, so it has no inverse"),
    // RUN018 appends " ({code})" when the failure carries a runtime code.
    t(RUN018, "UnhandledError", Runtime, Host,
      "unhandled failure: {message}"),
    t(RUN019, "ReadOfCancelledTask", Runtime, Host,
      "'{name}' was cancelled and has no value to read"),

    // §3.14 Memory (MEM) — allocator, tier limits, arena lifecycle.
    t(MEM001, "TierExceeded", Runtime, Host,
      "Memory grow to {n} bytes exceeded tier limit {m} bytes"),
    t(MEM002, "ArenaEscape", Warning, Compiler,
      "value allocated in the {reset_policy} arena is stored in '{name}', which outlives the arena reset"),
    t(MEM003, "ArenaImbalance", Runtime, Host,
      "arena imbalance: {pop without matching push | pop past a save-point not owned by the caller} at {location}"),

    // §3.15 Block handlers (BLOCK) — rule bodies live in 04 language / 21
    // §21.6 (the declared ERC-02 exception); no templates are defined there.
    c(BLOCK001, "AmbiguousBlockName", Error, Compiler),
    c(BLOCK002, "UnknownBlockName", Error, Compiler),
    c(BLOCK003, "ReservedBlockName", Error, Compiler),
    c(BLOCK004, "HandlerMalformedIR", Error, Compiler),
    c(BLOCK005, "HandlerBudgetExceeded", Error, Compiler),
    c(BLOCK006, "HandlerForbiddenSideEffect", Error, Compiler),

    // §3.16 Configuration (CFG) — Framework/Manager/LSP read project files;
    // the compiler emits none of these (CMP-01).
    t(CFG001, "ManifestSchemaViolation", Error, Toolchain,
      "invalid {file}: {reason} at '{key_path}'"),
    t(CFG002, "ManifestConstraintViolation", Error, Toolchain,
      "configuration constraint violated: {constraint} (keys: {key_list})"),
    t(CFG003, "ManifestWarning", Warning, Toolchain,
      "{finding} — {consequence}"),
    t(CFG004, "LockfileMismatch", Error, Toolchain,
      "lockfile out of date: '{name}' declared {constraint} but locked at {locked_version}"),
    t(CFG005, "FileEncodingInvalid", Error, Toolchain,
      "file is not valid UTF-8: {path} (first invalid byte at offset {offset})"),

    // §3.17 Compilation request (RQD) — request-document validation, before
    // any source is parsed.
    t(RQD001, "RequestIntegrityFailure", Error, Compiler,
      "request integrity failure: '{path}' declares sha256 {declared}, content hashes to {actual}"),
    t(RQD002, "RequestSchemaViolation", Error, Compiler,
      "invalid compilation request: {reason} at '{json_path}'"),

    // §3.18 Capability wiring (CAP) — Clean Framework at Moment 1.
    t(CAP001, "NoBackendAvailable", Error, Framework,
      "capability '{capability}' has no backend available on host '{host}'"),
    t(CAP002, "BackendNotInstalled", Error, Framework,
      "capability '{capability}' is imported but no backend is installed"),
    t(CAP003, "BackendUnknown", Error, Framework,
      "unknown backend '{backend}' for capability '{capability}'"),
];
