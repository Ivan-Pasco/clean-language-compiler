# Platform 09. Error Code Registry

Every diagnostic the compiler and runtime can emit carries a unique code. This file is the master index of those codes — organized by range, cross-referenced against the rules they enforce, and versioned so that codes are added deliberately and never silently renamed or reused. For the full rule definitions (conditions, examples, messages), see [10 — Semantic Rules](./10-semantic-rules.md). For the message anatomy, JSON schema, LSP mapping, style guide, and `cln explain` contract every code must obey, see [13 — Diagnostic Format](./13-diagnostic-format.md).

---

## 1. Code Ranges

### ERC-01 — Diagnostic code format

Every diagnostic code MUST match the pattern `PREFIX###` — a fixed 3-letter (or up to 6) prefix followed by three digits. No hyphens inside a code, no library-specific prefixes, no ad-hoc kebab-case names. Prohibited forms (all removed from V2 spec): `MEM-TIER-EXCEEDED`, `BUILD-LIMIT-EXCEEDED`, `FRAME-DATA-E028`, `CONFIG-SCHEMA-*`. Library-emitted diagnostics MUST travel as `LIB010` with a library-supplied sub-label field.

The registered ranges:

| Range | Category | Phase |
|-------|----------|-------|
| SYN001–SYN099 | Syntax | Parsing (lexer and parser) |
| SEM001–SEM099 | Semantic | HIR validation, resolver, type checker |
| SCOPE001–SCOPE099 | Scope | Resolver — name resolution and visibility |
| FUNC001–FUNC099 | Function | HIR validation — function definitions and calls |
| CLASS001–CLASS099 | Class | HIR validation — class definitions and inheritance |
| IDX001–IDX099 | Index | Type checker — collection access |
| STATE001–STATE099 | State | HIR validation and runtime — state management |
| IMPORT001–IMPORT099 | Import | Module resolver — imports and dependencies |
| LIB001–LIB099 | Library | Library loader — library registration and expansion |
| COM001–COM099 | Compilation | Code generation, bridge resolution and linking, host-contract checks (build through instantiation) |
| RUN001–RUN099 | Runtime | WASM execution — runtime assertion failures |
| MEM001–MEM099 | Memory | Runtime — allocator, tier limits, arena lifecycle |
| BLD001–BLD099 | Build | Multi-file compiler — project layout and discovery |
| BLOCK001–BLOCK099 | Block handlers | Block-name resolution and handler registration — see [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md) |
| CFG001–CFG099 | Configuration | Project input files read by the toolchain — manifest schema and constraints, and the text encoding of every file read |
| RQD001–RQD099 | Compilation request | Request-document validation — integrity and schema of the compilation request ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)) |
| CAP001–CAP099 | Capability wiring | Framework — resolving a guest's capability imports to bridge backends at build time ([framework 11 §11.10](../02%20components/framework/11-build-orchestration.md#1110-host-configuration-generation)) |

### 1.1 The 1:1 Rule

### ERC-02 — One code, one rule

Every code in this registry MUST have a rule in [10 — Semantic Rules](./10-semantic-rules.md), with one declared exception: the rule bodies of the `BLOCK` range live in [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md) (see [§3.15](#315-block-handler-codes-block)). Every rule in `10-semantic-rules.md` MUST have a code here. A code without a rule, or a rule without a code, is a spec bug — file it via `report_error` on `component=spec`.

Matrix count as of 2026-08-17 (M4 registry pass): **165 rows registered, 8 withdrawn, 157 emittable** — 151 emittable codes with rules in `10-semantic-rules.md` plus 6 (`BLOCK001`–`BLOCK006`) with rule bodies in 21. The eight withdrawn — `SCOPE005`, `FUNC001`, `CLASS007`, `LIB005`, `LIB007`, `LIB008`, `LIB009`, `IMPORT005` — may never be emitted; each keeps its row in §3 and its withdrawal note in 10 as the permanent record, and no identifier is renumbered or reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)).

### 1.2 Adding a New Code

### ERC-03 — Registration process

Adding a code is a spec change that follows the governance decision process (an [ADR](../01%20governance/decisions/) when it introduces a new class of failure or a new range), not an implementation choice. A new code MUST follow these steps:

1. Pick an unused number in the appropriate range. Ranges close at `099` for now; expanding a range requires developer approval.
2. Add the row to §3 (Full Code Index) here.
3. Add the corresponding rule to `10-semantic-rules.md` with a full entry per [RUL-01](./10-semantic-rules.md) (condition, message template, primary label, examples, suggested fix).
4. Add a snapshot test per [`13-diagnostic-format.md §12`](./13-diagnostic-format.md).
5. If the code represents a new class of failure (not just a new specific case), add it to the report-severity mapping in [`06-error-reporting.md §6.13`](./06-error-reporting.md).

---

## 2. Severity Levels

The severity vocabulary is the one defined in [`13-diagnostic-format.md §3`](./13-diagnostic-format.md): **`error | warning | info | help`**. The index tables in §3 capitalize the levels for readability; `help` is never a top-level severity — it only appears attached to another diagnostic (13 §3).

| Level | Meaning | Compiler behavior |
|-------|---------|-------------------|
| **Error** | Code is invalid and cannot be compiled | Compilation stops after the current phase |
| **Warning** | Code is valid but may be unintentional | Compilation continues; diagnostic is reported |
| **Info** | Code is valid; the diagnostic is purely advisory | Compilation continues |

**Runtime codes.** Rows marked **Runtime** in the Severity column are not a fifth level: the marker records the *phase*, not the level. A Runtime code is raised during WASM execution rather than by the compiler; when it is rendered as a diagnostic it carries `level = error` and maps to LSP severity 1 per [`13-diagnostic-format.md §7`](./13-diagnostic-format.md).

---

## 3. Full Code Index

The rows below are the authoritative binding of code → name → severity. Rule bodies live in the "Defined In" column's target.

### 3.1 Syntax Codes (SYN)

Emitted during parsing. No semantic information is available at this stage — the compiler has only seen tokens.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| SYN001 | `InvalidToken` | Character is not valid in any lexical context | Error | 10-semantic-rules.md §SYN001 |
| SYN002 | `UnexpectedToken` | Token appears where the grammar does not allow it | Error | 10-semantic-rules.md §SYN002 |
| SYN003 | `InvalidIndentation` | Spaces used instead of tabs, or inconsistent nesting level | Error | 10-semantic-rules.md §SYN003 |
| SYN004 | `UnterminatedConstruct` | String literal, comment, or block not closed before end of file | Error | 10-semantic-rules.md §SYN004 |
| SYN005 | `MalformedConstruct` | Partial syntax structure is missing required elements | Error | 10-semantic-rules.md §SYN005 |
| SYN006 | `IndentationError` | Tab/space mixing detected, or indentation level is wrong for the block | Error | 10-semantic-rules.md §SYN006 |
| SYN007 | `SectionOutOfOrder` | Top-level sections appear out of the required order (see [8 — File Structure](../04%20language/08-file-structure.md)) | Error | 10-semantic-rules.md §SYN007 |
| SYN008 | `InvalidPrintBlock` | `print:` block contains no expressions, or a non-expression item | Error | 10-semantic-rules.md §SYN008 |
| SYN009 | `NotATopLevelForm` | A construct that is not one of the permitted top-level sections appears at the top level of a file (distinct from `SYN007`, which is about order) | Error | 10-semantic-rules.md §SYN009 |
| SYN010 | `MissingParentheses` | A method or function call is written without parentheses | Error | 10-semantic-rules.md §SYN010 |
| SYN100 | `MissingSpecPath` | `spec` AI-metadata statement is missing its string-literal path | Error | 10-semantic-rules.md §SYN100 |
| SYN101 | `MissingIntentDescription` | `intent` AI-metadata statement is missing its string-literal description | Error | 10-semantic-rules.md §SYN101 |

### 3.2 Semantic Codes (SEM)

Emitted during HIR validation, name resolution, and type checking.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| SEM001 | `AssignTypeMismatch` | Right-hand side of an assignment does not match the declared type of the left-hand side | Error | 10-semantic-rules.md §SEM001 |
| SEM002 | `UndefinedVariable` | A variable name is referenced but not declared in any enclosing scope | Error | 10-semantic-rules.md §SEM002 |
| SEM003 | `SymbolRedefinition` | Symbol declared more than once in the same scope | Error | 10-semantic-rules.md §SEM003 |
| SEM004 | `InvalidOperationForType` | Operator or operation applied to an unsupported type | Error | 10-semantic-rules.md §SEM004 |
| SEM005 | `AccessViolation` | Private member accessed from outside its allowed scope | Error | 10-semantic-rules.md §SEM005 |
| SEM006 | `InheritanceError` | Class inheritance declaration is invalid | Error | 10-semantic-rules.md §SEM006 |
| SEM007 | `GenericTypeError` | Generic or polymorphic type operation is invalid | Error | 10-semantic-rules.md §SEM007 |
| SEM008 | `InheritanceCycle` | Class inherits from itself directly or indirectly | Error | 10-semantic-rules.md §SEM008 |
| SEM009 | `InvalidTypeSpecification` | Type name does not refer to a valid type | Error | 10-semantic-rules.md §SEM009 |
| SEM010 | `InvalidMatchPattern` | Argument to `string.matches()` is not one of the declared pattern constants ([15 — Standard Library §String Patterns](../04%20language/15-standard-library.md#string-patterns)) | Error | 10-semantic-rules.md §SEM010 |
| SEM011 | `MissingCapabilityMethod` | Class declares `can C` but does not implement a required capability method | Error | 10-semantic-rules.md §SEM011 |
| SEM012 | `UndefinedCapability` | A `can` clause or type reference names an undeclared capability | Error | 10-semantic-rules.md §SEM012 |
| SEM013 | `CapabilityMethodSignatureMismatch` | Class implements a capability method but its signature does not match | Error | 10-semantic-rules.md §SEM013 |
| SEM014 | `CapabilityBodyNotAllowed` | A `can` block declaration includes a method body; capabilities are pure contracts | Error | 10-semantic-rules.md §SEM014 |
| SEM015 | `ReturnTypeMismatch` | The type of a returned expression does not match the function's declared return type | Error | 10-semantic-rules.md §SEM015 |
| SEM016 | `ArgumentTypeMismatch` | An argument value's type does not match the corresponding parameter's declared type | Error | 10-semantic-rules.md §SEM016 |
| SEM017 | `StateInitializerTypeMismatch` | A `state:` initializer value's type does not match the declared state type | Error | 10-semantic-rules.md §SEM017 |
| SEM018 | `ComputedBodyTypeMismatch` | The body of a `computed:` state declaration produces a value that does not match the declared type | Error | 10-semantic-rules.md §SEM018 |
| SEM019 | `UndefinedFunction` | A function name is called but not declared in any imported or top-level `functions:` scope | Error | 10-semantic-rules.md §SEM019 |
| SEM020 | `UndefinedClass` | A class name is used in a `new` or type position but not declared in any imported or top-level scope | Error | 10-semantic-rules.md §SEM020 |
| SEM021 | `UndefinedModule` | A module name in an `import:` list cannot be resolved by the module resolver | Error | 10-semantic-rules.md §SEM021 |
| SEM022 | `UndefinedMethod` | A method call `receiver.method()` references a method not defined on the receiver's type | Error | 10-semantic-rules.md §SEM022 |
| SEM023 | `NonBooleanCondition` | The condition of an `if` or `while` is not a boolean expression | Error | 10-semantic-rules.md §SEM023 |
| SEM024 | `ExpectedValueNotConstant` | The expected value of a test is not evaluable at compile time | Error | 10-semantic-rules.md §SEM024 |
| SEM025 | `ControlFlowOutsideLoop` | `break` or `continue` appears where no `iterate` or `while` encloses it | Error | 10-semantic-rules.md §SEM025 |
| SEM026 | `LiteralOutOfRange` | A numeric literal's applied value does not fit the type it is assigned to | Error | 10-semantic-rules.md §SEM026 |
| SEM027 | `LossyIntegerPromotion` | An implicit `integer` → `number` conversion of a compile-time-evaluable value beyond 2⁵³ provably loses precision ([TYP-06](../04%20language/04-type-system.md#typ-06--type-conversion)) | Warning | 10-semantic-rules.md §SEM027 |
| SEM028 | `UndefinedField` | A field access `receiver.field` references a field not defined on the receiver's type (the field-side counterpart of `SEM022`) | Error | 10-semantic-rules.md §SEM028 |

### 3.3 Scope Codes (SCOPE)

Emitted by the resolver during name resolution.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| SCOPE001 | `UseBeforeDeclaration` | Variable referenced before it is declared | Error | 10-semantic-rules.md §SCOPE001 |
| SCOPE002 | `RedeclarationInScope` | Variable redeclared in the same scope | Error | 10-semantic-rules.md §SCOPE002 |
| SCOPE003 | `MaxScopeDepthExceeded` | Scope nesting depth exceeds implementation limit | Error | 10-semantic-rules.md §SCOPE003 |
| SCOPE004 | `WatchTargetNotState` | Watch block target does not reference a `state:` variable | Error | 10-semantic-rules.md §SCOPE004 |
| SCOPE005 | `ScreenStateAccess` | *(WITHDRAWN 2026-08-07 per [ADR-0030](../01%20governance/decisions/0030-withdraw-screen-from-language.md); ID retained per DOC-13)* | — | 10-semantic-rules.md §SCOPE005 |
| SCOPE006 | `CompiletimeHelperOutsideTest` | A `test.compiletime.*` helper is used outside a `tests:` block | Error | 10-semantic-rules.md §SCOPE006 |

### 3.4 Function Codes (FUNC)

Emitted during HIR validation of function definitions and call sites.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| FUNC001 | `FunctionNotDefined` | *(WITHDRAWN 2026-08-17 — its define-before-use condition contradicted [9 — Functions](../04%20language/09-functions.md); the real case is `SEM019`'s. ID retained per DOC-13)* | — | 10-semantic-rules.md §FUNC001 (withdrawal note) |
| FUNC002 | `ArgumentCountMismatch` | Argument count does not match parameter count | Error | 10-semantic-rules.md §FUNC002 |
| FUNC003 | `CallOnNonFunction` | Parenthesis-invocation of a non-function symbol | Error | 10-semantic-rules.md §FUNC003 |
| FUNC004 | `MissingReturn` | Non-void function has no return on some execution path | Warning | 10-semantic-rules.md §FUNC004 |
| FUNC005 | `EmptyReturnInNonVoid` | `return` with no value in a non-void function | Warning | 10-semantic-rules.md §FUNC005 |
| FUNC006 | `StartBlockHasParameters` | The `start:` entry point block declares parameters | Error | 10-semantic-rules.md §FUNC006 |
| FUNC007 | `StartBlockReturnsValue` | The `start:` entry point returns a value | Warning | 10-semantic-rules.md §FUNC007 |
| FUNC008 | `UnknownNamedArgument` | Named argument label does not match any parameter name | Error | 10-semantic-rules.md §FUNC008 |
| FUNC009 | `DuplicateNamedArgument` | Same named argument label appears more than once in a call | Error | 10-semantic-rules.md §FUNC009 |
| FUNC010 | `PositionalAfterNamed` | Positional argument appears after a named argument in a call | Error | 10-semantic-rules.md §FUNC010 |
| FUNC011 | `NamedArgCoverageError` | Named argument count or coverage does not match parameter count | Error | 10-semantic-rules.md §FUNC011 |
| FUNC012 | `MethodCallOnStandaloneFunction` | Method-call syntax (`receiver.fn()`) used on a non-method symbol | Error | Emitted by resolver |
| FUNC013 | `FunctionOutsideFunctionsBlock` | A function is declared outside a `functions:` block | Error | 10-semantic-rules.md §FUNC013 |
| FUNC014 | `OptionalParameterOrder` | A parameter with a default value precedes a required parameter | Error | 10-semantic-rules.md §FUNC014 |
| FUNC015 | `DuplicateStartBlock` | A file declares more than one `start:` entry block ([FNC-01](../04%20language/09-functions.md#fnc-01--start-is-the-entry-point)) | Error | 10-semantic-rules.md §FUNC015 |

### 3.5 Class Codes (CLASS)

Emitted during HIR validation of class definitions.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| CLASS001 | `ParentClassNotFound` | Parent class named in `is ParentName` is not defined | Error | 10-semantic-rules.md §CLASS001 |
| CLASS002 | `DuplicateField` | Two fields in the same class share a name | Error | 10-semantic-rules.md §CLASS002 |
| CLASS003 | `DuplicateMethod` | Two methods in the same class share a name | Error | 10-semantic-rules.md §CLASS003 |
| CLASS004 | `MissingConstructor` | Class is instantiated but has no constructor | Error | 10-semantic-rules.md §CLASS004 |
| CLASS005 | `AfterBeforeLogic` | `after` statement appears after non-contract logic in a function body | Error | 10-semantic-rules.md §CLASS005 |
| CLASS006 | `AlwaysConditionNotBoolean` | Expression inside an `always:` invariant block is not boolean | Error | 10-semantic-rules.md §CLASS006 |
| CLASS007 | `ContractBlockOutOfPosition` | *(WITHDRAWN 2026-08-17 — every case is parser-owned (`SYN005`) or `CLASS005`'s; no reachable trigger remained. ID retained per DOC-13)* | — | 10-semantic-rules.md §CLASS007 (withdrawal note) |
| CLASS008 | `ResultOutsideAfter` | The `result` identifier is used outside an `after:` expression | Error | 10-semantic-rules.md §CLASS008 |
| CLASS009 | `ContractSideEffect` | A contract expression performs I/O, mutates state, or calls a function that itself carries contracts | Error | 10-semantic-rules.md §CLASS009 |
| CLASS010 | `ConstructorParameterShadowsField` | A constructor parameter has the same name as a field of the class | Error | 10-semantic-rules.md §CLASS010 |
| CLASS011 | `CapabilityInstantiated` | A capability is instantiated; a capability is a contract with no bodies | Error | 10-semantic-rules.md §CLASS011 |
| CLASS012 | `InvalidCompanionAccess` | Companion access on an instance rather than a class name, through an undeclared field, or reaching an instance method | Error | 10-semantic-rules.md §CLASS012 |

### 3.6 Index Access Codes (IDX)

Emitted by the type checker when bracket access is used.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| IDX001 | `ListIndexNotInteger` | List bracket access uses a non-integer index | Error | 10-semantic-rules.md §IDX001 |
| IDX002 | `MatrixIndexNotInteger` | Matrix bracket access uses a non-integer index | Error | 10-semantic-rules.md §IDX002 |
| IDX003 | `PairsKeyTypeMismatch` | Pairs bracket access uses a key whose type is not the declared key type `K` | Error | 10-semantic-rules.md §IDX003 |
| IDX004 | `IndexOnNonIndexable` | Bracket access on a type that is not `list`, `matrix`, `pairs`, or `any` | Error | 10-semantic-rules.md §IDX004 |
| IDX005 | `IndexOnNone` | Bracket access on a value of type `T?` that is provably `none` at the access site | Error | 10-semantic-rules.md §IDX005 |

### 3.7 State Codes (STATE)

Emitted during HIR validation (compile-time) and during WASM execution (runtime).

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| STATE001 | `GuardConditionNotPure` | Guard expression is not a pure boolean or contains side effects | Error | 10-semantic-rules.md §STATE001 |
| STATE002 | `GuardRejection` | State update rejected at runtime because guard evaluated to false | Runtime | 10-semantic-rules.md §STATE002 |
| STATE003 | `CircularStateDependency` | Circular dependency detected between computed state declarations (the type-mismatch case is [`SEM018`](#32-semantic-codes-sem)) | Error | 10-semantic-rules.md §STATE003 |
| STATE004 | `ComputedStateAssignment` | Assignment to a computed state variable (which is read-only) | Error | 10-semantic-rules.md §STATE004 |
| STATE005 | `RulesExpressionNotBoolean` | Expression inside a `state: rules:` block is not boolean | Error | 10-semantic-rules.md §STATE005 |
| STATE006 | `StateRuleViolated` | A `rules:` expression evaluated to `false` when a function that changed state returned | Runtime | 10-semantic-rules.md §STATE006 |

> **Withdrawn name.** `STATE003` was originally named `ComputedReturnTypeMismatch`; that symbolic name is retired (2026-08-01, when the type-mismatch case was ceded to `SEM018`). The code is unchanged and is not reused — only the symbolic name changed ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose): IDs are never renumbered or reused).

### 3.8 Import Codes (IMPORT)

Emitted by the module resolver.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| IMPORT001 | `CircularDependency` | Two or more modules import each other in a cycle — detected by the compiler resolve pass while building the module graph from the compilation request ([14 §14.4.2](./14-compiler-architecture.md)) | Error | 10-semantic-rules.md §IMPORT001 |
| IMPORT002 | `ModuleNotFound` | Imported module does not exist in any search path | Error | 10-semantic-rules.md §IMPORT002 |
| IMPORT003 | `SymbolNotInModule` | Specific symbol imported from a module is not exported by that module | Error | 10-semantic-rules.md §IMPORT003 |
| IMPORT004 | `DuplicateImportItem` | Same item appears more than once in an import list | Error | 10-semantic-rules.md §IMPORT004 |
| IMPORT005 | `ImportCycle` | **Withdrawn** (2026-08-01) — described the same failure as `IMPORT001`; folded into [`IMPORT001`](#38-import-codes-import). IDs are never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)) | — | 10-semantic-rules.md §IMPORT005 (withdrawal note) |

> **Withdrawn code.** `IMPORT005` (`ImportCycle`) duplicated `IMPORT001` — both described a cycle in the module import graph, and the one-violation-one-diagnostic principle allows only one owner. The case is folded into `IMPORT001`, whose rule entry now carries the resolve-pass detection contract. The number `IMPORT005` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose): IDs are never renumbered or reused).

### 3.9 Library Codes (LIB)

Emitted by the library resolver, loader, and compile-time-function driver. Libraries in V2 are Clean source packages (see [Libraries Specification](../02%20components/framework/09-libraries-specification.md)) that may declare typed `host function` bindings against either compiler-generated bridges or native host modules provided as WASM components.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| LIB001 | `LibraryNotFound` | A library named in `clean.toml [dependencies]` or a `[folders]` mapping cannot be resolved from any registry, path, or lockfile source | Error | 10-semantic-rules.md §LIB001 |
| LIB002 | `LibraryVersionConflict` | Two dependencies resolve the same library to incompatible SemVer ranges and no single version satisfies both | Error | 10-semantic-rules.md §LIB002 |
| LIB003 | `LibraryCyclicDependency` | libraries A and B depend on each other, directly or transitively | Error | 10-semantic-rules.md §LIB003 |
| LIB004 | `LibraryManifestInvalid` | `library.toml` is malformed, missing required fields, or declares a manifest schema version the compiler does not support | Error | 10-semantic-rules.md §LIB004 |
| LIB005 | *(withdrawn)* | Withdrawn 2026-08-01 — duplicated `BLOCK001`; the identifier is never reused | — | 10-semantic-rules.md §LIB005 |
| LIB006 | `UnknownBlockHandler` | A `handles "name"` declaration references a block name whose namespace is not owned by this library and is not delegated to it | Error | 10-semantic-rules.md §LIB006 |
| LIB007 | *(withdrawn)* | Withdrawn 2026-08-01 — duplicated `BLOCK006`; the identifier is never reused | — | 10-semantic-rules.md §LIB007 |
| LIB008 | *(withdrawn)* | Withdrawn 2026-08-01 — duplicated `BLOCK004`; the identifier is never reused | — | 10-semantic-rules.md §LIB008 |
| LIB009 | *(withdrawn)* | Withdrawn 2026-08-01 — duplicated `BLOCK005`; the identifier is never reused | — | 10-semantic-rules.md §LIB009 |
| LIB010 | `CompiletimeDiagnostic` | Wrapper code for user-authored diagnostics emitted from inside a `compiletime function` via `error(...)` / `warning(...)` / `info(...)` | Error / Warning / Info | 10-semantic-rules.md §LIB010 |
| LIB011 | `HostFunctionSignatureMismatch` | A `host function` declaration in library source does not match the signature the resolved host module (compiler-generated bridge OR native WASM component) actually exposes | Error | 10-semantic-rules.md §LIB011 |
| LIB012 | `HostFunctionUnbound` | A `host function` declaration sits in a `host interface` whose `requires host worlds` list does not include the current build target, or names an interface no active host provides | Error | 10-semantic-rules.md §LIB012 |
| LIB013 | `HostFunctionSandboxDenied` | The library declared a `host function` whose host module is not permitted by the project's `clean.toml [security]` capabilities | Error | 10-semantic-rules.md §LIB013 |
| LIB014 | `LibraryResourceLimit` | Loading or compiling the library exceeded a resource limit (source-tree file count, per-library heap allocation for compile-time execution, or generated-IR node count) | Error | 10-semantic-rules.md §LIB014 |
| LIB015 | `CapabilityNotImplemented` | A companion type declares `can Persist` (or another library-owned capability) but does not implement one of the capability's required methods, and the library has not supplied a default | Error | 10-semantic-rules.md §LIB015 |
| LIB016 | `CapabilityConflict` | Two libraries in the resolved graph both define a capability with the same fully-qualified name and mutually incompatible method signatures | Error | 10-semantic-rules.md §LIB016 |
| LIB017 | `FolderScopeUnclaimed` | A folder listed in `clean.toml [folders]` maps to a library that is not present in `[dependencies]` | Error | 10-semantic-rules.md §LIB017 |
| LIB018 | `FolderScopeAmbiguous` | Two libraries in scope for the same folder both claim ownership of the same block namespace inside that folder | Error | 10-semantic-rules.md §LIB018 |
| LIB019 | `HostBridgeMisplaced` | A `host interface` / `host function` declaration appears outside the library's `host_bridge.cln` (mandatory location: [LBS §8.2](../02%20components/framework/09-libraries-specification.md#82-file-layout)) | Error | 10-semantic-rules.md §LIB019 |
| LIB020 | `SourceBlockMalformed` | A `source:` block is missing its `spec` or `version` field, or does not appear at the top of the file | Error | 10-semantic-rules.md §LIB020 |

### 3.10 Compilation Codes (COM)

Emitted between code generation and instantiation: WASM emission (COM001–COM008), WIT bridge resolution and link-time verification ([08 — Bridge Versioning](./08-bridge-versioning.md), COM009–COM011), the world import check ([14 §14.4.2 pass 9](./14-compiler-architecture.md)), and the host-contract check moments of [16 — Host Contract Validation](./16-host-contract-validation.md) (COM014–COM017).

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| COM001 | `WasmGenerationError` | Code generator failed to produce valid WASM for a construct | Error | 10-semantic-rules.md §COM001 |
| COM002 | `OptimizationError` | Optimization pass produced invalid code | Error | 10-semantic-rules.md §COM002 |
| COM003 | `MemoryLayoutError` | Memory allocation or layout calculation failed | Error | 10-semantic-rules.md §COM003 |
| COM004 | `ModuleResolutionError` | Multi-file compilation failed to resolve module dependencies | Error | 10-semantic-rules.md §COM004 |
| COM005 | `TargetFeatureUnsupported` | Compilation target does not implement a language feature the source uses | Error | 10-semantic-rules.md §COM005 |
| COM006 | `FunctionNotFoundDuringCompilation` | Function passed semantic analysis but could not be located during code generation | Error | 10-semantic-rules.md §COM006 |
| COM007 | `TargetHostModuleMissing` | Compilation target does not provide a host module the source or a library declared | Error | 10-semantic-rules.md §COM007 |
| COM008 | `TargetSizeBudgetExceeded` | Compiled artifact exceeds the size budget declared for the target in `clean.toml [compile.limits]` (schema home: [`07-build-config.md`](./07-build-config.md)) | Error | 10-semantic-rules.md §COM008 |
| COM009 | `BridgeResolveError` | No single version assignment satisfies every WIT package constraint across the target world and library dependencies ([08 §8.3](./08-bridge-versioning.md)) | Error | 10-semantic-rules.md §COM009 |
| COM010 | `BridgeLinkError` | Link-time verification failed: a guest import, type shape, or resource contract does not match the target world at the resolved version ([08 §8.5](./08-bridge-versioning.md)) | Error | 10-semantic-rules.md §COM010 |
| COM011 | `BridgeRuntimeMismatch` | Host rejected instantiation: a guest import is missing from, or version-mismatched against, the host's `host.wit` ([08 §8.6](./08-bridge-versioning.md), Moment 3) | Runtime | 10-semantic-rules.md §COM011 |
| COM012 | `HostImportNotInWorld` | A `host function` call site's import is not in the target world WIT delivered in the compilation request ([14 §14.4.2 pass 9](./14-compiler-architecture.md)) | Error | 10-semantic-rules.md §COM012 |
| COM013 | `CodegenInternalInvariant` | A self-produced compiler artifact failed self-validation — an internal compiler error (ICE), never a user error ([14](./14-compiler-architecture.md)) | Error | 10-semantic-rules.md §COM013 |
| COM014 | `WorldMismatch` | Moment 1 (`cln build`): the target host's WIT does not provide an interface the project requires ([16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)) | Error | 10-semantic-rules.md §COM014 |
| COM015 | `VersionMismatch` | Moment 2 (`cln check <host>`): a deployed host provides a required package at a version incompatible with the built component ([16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)) | Error | 10-semantic-rules.md §COM015 |
| COM016 | `DeprecatedMemberUse` | Program uses a WIT interface member marked `@deprecated` ([15 §9.2](./15-component-model-architecture.md), [08 §8.7](./08-bridge-versioning.md)) | Warning | 10-semantic-rules.md §COM016 |
| COM017 | `InstantiationFailure` | Moment 3 (`host.load`): the host loader failed to instantiate the component for a reason other than the import/version mismatch owned by COM011 ([16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)) | Runtime | 10-semantic-rules.md §COM017 |

### 3.11 Build Codes (BLD)

Emitted by the multi-file compiler while enforcing build-scoped limits and project layout rules ([07 §7.8](./07-build-config.md#78-compile-time-limits)).

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| BLD001 | `BuildLimitExceeded` | A build-scoped hard cap from `clean.toml [compile.limits]` was exceeded (whole-build timeout, source file size, import depth) | Error | 10-semantic-rules.md §BLD001 |

### 3.12 Runtime Codes (RUN)

Not emitted by the compiler. Raised during WASM execution by the host runtime.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| RUN001 | `MemoryViolation` | WASM execution accessed memory outside allocated bounds | Runtime | 10-semantic-rules.md §RUN001 |
| RUN002 | `StackError` | WASM stack overflow or underflow during execution | Runtime | 10-semantic-rules.md §RUN002 |
| RUN003 | `ArithmeticError` | Integer division/remainder by zero, integer division overflow, or a failed numeric conversion at runtime (`number` arithmetic is IEEE 754 and never raises) | Runtime | 10-semantic-rules.md §RUN003 |
| RUN004 | `ReferenceError` | `none` or invalid reference accessed at runtime (raised by `expr!` on a `none` value) | Runtime | 10-semantic-rules.md §RUN004 |
| RUN005 | `AssertionFailure` | A `before` statement evaluated to false at runtime | Runtime | 10-semantic-rules.md §RUN005 |
| RUN006 | `JsonParseError` | Generic malformed JSON input to `json.textToData`; used when no more specific JSON code applies. `json.tryTextToData` returns `none` in the same conditions | Runtime | 10-semantic-rules.md §RUN006 |
| RUN007 | `JsonInvalidNumber` | JSON number is malformed or out of the `number` range (e.g. `1e999`, leading zeros, `.5`, unsupported `-0` under strict decision) | Runtime | 10-semantic-rules.md §RUN007 |
| RUN008 | `JsonInvalidString` | JSON string has a bad escape, lone surrogate, invalid UTF-8, or is unterminated | Runtime | 10-semantic-rules.md §RUN008 |
| RUN009 | `JsonInvalidStructure` | JSON structural error: unmatched bracket/brace, missing/extra comma, trailing data after the root value, or duplicate keys under strict decision | Runtime | 10-semantic-rules.md §RUN009 |
| RUN010 | `JsonDepthExceeded` | JSON nesting exceeded the documented depth limit (1000 levels) | Runtime | 10-semantic-rules.md §RUN010 |
| RUN011 | `ContractViolation` | An `after:` postcondition or `always` invariant evaluated to false at runtime (including `always:` invariants checked by `Database.save` — the `before` case is [`RUN005`](#312-runtime-codes-run)) | Runtime | 10-semantic-rules.md §RUN011 |
| RUN012 | `TimeBudgetExceeded` | The per-invocation wall-clock (epoch) budget was exhausted and the host trapped the instance ([03 §3.5](./03-memory-model.md#35-host-backing--observable-contract); defaults: [07 `[runtime]`](./07-build-config.md#72-schema--top-level)) | Runtime | 10-semantic-rules.md §RUN012 |
| RUN013 | `IndexOutOfRange` | A collection or string access is outside the valid range at runtime | Error | 10-semantic-rules.md §RUN013 |
| RUN014 | `FileOperationFailed` | A `file.*` operation failed — not found, permission denied, or the write could not complete | Error | 10-semantic-rules.md §RUN014 |
| RUN015 | `HttpRequestFailed` | An `http.*` request did not complete | Error | 10-semantic-rules.md §RUN015 |
| RUN016 | `MatrixShapeMismatch` | A matrix operation was given shapes it does not admit — incompatible dimensions, or a non-square matrix where square is required | Runtime | 10-semantic-rules.md §RUN016 |
| RUN017 | `MatrixSingular` | `inverse()` was called on a matrix whose determinant is zero | Runtime | 10-semantic-rules.md §RUN017 |
| RUN018 | `UnhandledError` | A failure reached the top of the program with no enclosing `onError` | Runtime | 10-semantic-rules.md §RUN018 |
| RUN019 | `ReadOfCancelledTask` | A deferred (`later`) binding was read after its task was cancelled | Runtime | 10-semantic-rules.md §RUN019 |

### 3.13 Core-Library Sub-Labels (LIB010)

A library does not own a code prefix ([§1](#1-code-ranges)); its diagnostics travel as `LIB010` carrying a sub-label. Sub-labels are kebab-case and unique within the emitting library. The Core libraries register these:

| Library | Sub-label | Meaning | Severity |
|---------|-----------|---------|----------|
| `auth` | `session-store-not-shared` | Multi-instance deployment configured with the in-memory session store ([AUTH-02](../02%20components/framework/libraries/01-auth.md)) | Warning |
| `client` | `generated-identifier-collision` | A `load:`/`form:`/`send:` block generates an identifier the component already declares ([CLNT-01](../02%20components/framework/libraries/03-client.md)) | Error |
| `data` | `field-alignment-drift` | A field in `fields:` has no counterpart on the paired entity ([DATA-02](../02%20components/framework/libraries/04-data.md)) | Error |
| `data` | `data-capture-runtime-value` | `<Entity>.data` captured as a value ([DATA-03](../02%20components/framework/libraries/04-data.md)) | Error |
| `locale` | `missing-plural-other` | A pluralised key defines no `_other` form ([LOC-01](../02%20components/framework/libraries/06-locale.md)) | Error |
| `locale` | `missing-translation-key` | A key referenced from source is absent from a loaded locale ([LOC-02](../02%20components/framework/libraries/06-locale.md)) | Warning |
| `server` | `endpoint-no-response` | A handler path completes without returning a Response ([SRV-01](../02%20components/framework/libraries/08-server.md)) | Error |
| `server` | `route-modifier-order` | Route modifiers out of the fixed order ([SRV-02](../02%20components/framework/libraries/08-server.md)) | Error |
| `storage` | `permission-denied`, `disk-full`, `invalid-path`, `parent-not-a-directory`, `io-error` | Non-zero `storage.write` returns ([STOR-03](../02%20components/framework/libraries/09-storage.md)) | Error |

Community and Local libraries register their sub-labels in their own documentation, not here.

### 3.14 Memory Codes (MEM)

Raised by the allocator, tier limits, and arena lifecycle (see [`03-memory-model.md`](./03-memory-model.md) and [`05-memory-policy.md`](./05-memory-policy.md)). MEM001 and MEM003 are raised at runtime by the host; MEM002 is a compile-time warning.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| MEM001 | `TierExceeded` | A `memory.grow` beyond the tier's maximum trapped at the offending call site ([05 §5.3](./05-memory-policy.md#53-enforcement)) | Runtime | 10-semantic-rules.md §MEM001 |
| MEM002 | `ArenaEscape` | A value allocated in a request/frame/task-scoped arena is stored in a persistent structure that outlives the arena reset ([05 §5.4](./05-memory-policy.md#54-reset-policies)) | Warning | 10-semantic-rules.md §MEM002 |
| MEM003 | `ArenaImbalance` | An `arena-pop` without a balanced push, or a pop past a save-point the caller did not receive ([03 MMD-03](./03-memory-model.md#mmd-03--arena-discipline-every-push-balanced-by-exactly-one-pop)) | Runtime | 10-semantic-rules.md §MEM003 |

### 3.15 Block Handler Codes (BLOCK)

Emitted during block-name resolution and handler registration/execution. The rule bodies live in [04 language / 21 — Block Handlers §21.6](../04%20language/21-block-handlers.md), not in `10-semantic-rules.md` (declared exception to [ERC-02](#erc-02--one-code-one-rule)).

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| BLOCK001 | `AmbiguousBlockName` | Two or more libraries in implicit scope register the same block name | Error | [21-block-handlers.md §21.6](../04%20language/21-block-handlers.md) |
| BLOCK002 | `UnknownBlockName` | No library in scope registers this block name | Error | [21-block-handlers.md §21.6](../04%20language/21-block-handlers.md) |
| BLOCK003 | `ReservedBlockName` | A library attempted to register a block name reserved by the language | Error | [21-block-handlers.md §21.6](../04%20language/21-block-handlers.md) |
| BLOCK004 | `HandlerMalformedIR` | A handler returned malformed IR (undefined symbol reference, type mismatch), or crashed and was caught by the compiler | Error | [21-block-handlers.md §21.6](../04%20language/21-block-handlers.md) |
| BLOCK005 | `HandlerBudgetExceeded` | A handler exceeded its compile-time wall-clock or memory budget | Error | [21-block-handlers.md §21.6](../04%20language/21-block-handlers.md) |
| BLOCK006 | `HandlerForbiddenSideEffect` | A block handler attempted a forbidden side effect (I/O, non-determinism) in a `compiletime` context | Error | [21-block-handlers.md §21.6](../04%20language/21-block-handlers.md) |

### 3.16 Configuration Codes (CFG)

Emitted by the components that read project files from disk or from an editor buffer — Clean Framework, Clean Manager and the language server — while validating `clean.toml` / `library.toml` schema and constraints ([07 §7.10](./07-build-config.md#710-validation)) and the text encoding of every file they read ([TXT-02](./17-text-encoding.md#txt-02--the-reader-validates-at-the-moment-it-reads)). The compiler emits none of these: it reads no file from disk ([CMP-01](./14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)).

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| CFG001 | `ManifestSchemaViolation` | Unknown key, wrong value type, or missing required field in `clean.toml` / `library.toml` | Error | 10-semantic-rules.md §CFG001 |
| CFG002 | `ManifestConstraintViolation` | Keys individually valid but jointly inconsistent (cross-key constraint violated, e.g. `memory64` without `build.memory64`) | Error | 10-semantic-rules.md §CFG002 |
| CFG003 | `ManifestWarning` | Manifest is valid but suspicious (deprecated key, custom profile shadowing a built-in) | Warning | 10-semantic-rules.md §CFG003 |
| CFG004 | `LockfileMismatch` | CI build where `clean.toml` and `.cln/lock.toml` disagree ([07 CONF-04](./07-build-config.md#77-dependencies)) | Error | 10-semantic-rules.md §CFG004 |
| CFG005 | `FileEncodingInvalid` | A file read by the toolchain is not well-formed UTF-8 ([TXT-01](./17-text-encoding.md#txt-01--every-ecosystem-text-file-is-utf-8)) | Error | 10-semantic-rules.md §CFG005 |

### 3.17 Compilation Request Codes (RQD)

Emitted by the compiler while validating the compilation request document it receives ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)) — before any source is parsed.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| RQD001 | `RequestIntegrityFailure` | A `sources[].sha256` does not match its decoded content; the whole request is refused | Error | 10-semantic-rules.md §RQD001 |
| RQD002 | `RequestSchemaViolation` | The request document has an unknown key, a missing required field, or a malformed section | Error | 10-semantic-rules.md §RQD002 |

### 3.18 Capability Wiring Codes (CAP)

Emitted by Clean Framework at Moment 1 (`cln build`) while resolving the guest's capability imports to bridge backends ([framework 11 §11.10](../02%20components/framework/11-build-orchestration.md#1110-host-configuration-generation)). These sit beside — never replace — the runtime's own refusal to start with an unsatisfied import ([SRVH-02](../02%20components/hosts/clean-server/01-server.md#srvh-02--absent-configuration-means-the-capability-is-off), [CH-05](../02%20components/hosts/clean-host-core/01-specification.md#ch-05--no-silent-fallbacks)).

Interface-level mismatches are **not** CAP codes: an import the target host does not provide at all is [`COM014`](#310-compilation-codes-com) (Moment 1 world mismatch), and a version-incompatible interface is `COM015` / `COM011`. CAP codes cover the layer above — the host provides the interface, but no backend was resolved to implement it.

| Code | Name | Short Description | Severity | Defined In |
|------|------|-------------------|----------|------------|
| CAP001 | `NoBackendAvailable` | The guest imports a capability for which no backend exists for this target host ([compatibility matrix](../02%20components/hosts/01-compatibility-matrix.md)) | Error | 10-semantic-rules.md §CAP001 |
| CAP002 | `BackendNotInstalled` | The guest imports a capability whose selected (or default) backend is not installed under `~/.cln/bridges/` | Error | 10-semantic-rules.md §CAP002 |
| CAP003 | `BackendUnknown` | `[<capability>] backend` names a backend that does not exist for that capability | Error | 10-semantic-rules.md §CAP003 |

---

## 4. Diagnostic Format

The full contract — CLI rendering, JSON schema, LSP mapping, applicability levels, `cln explain`, and the style guide — lives in [`13-diagnostic-format.md`](./13-diagnostic-format.md).

### ERC-04 — Every diagnostic carries a code

Every diagnostic emitted by the compiler, the framework, the manager, or a host runtime MUST carry a code registered in this file, and MUST satisfy the format contract of [`13-diagnostic-format.md`](./13-diagnostic-format.md). A diagnostic without a code is incomplete and violates concern [C-02 — Error clarity](../01%20governance/05-concerns.md).

---

## 5. Reserved Ranges

The following code numbers are reserved for future use. Do not assign them without updating this registry and [`10-semantic-rules.md`](./10-semantic-rules.md) ([ERC-03](#erc-03--registration-process)).

| Range | Status |
|-------|--------|
| SYN011–SYN099 | Reserved |
| SYN102–SYN199 | Reserved (extension range for AI-metadata syntax codes) |
| SEM029–SEM099 | Reserved |
| SCOPE007–SCOPE099 | Reserved |
| FUNC001 | Withdrawn (2026-08-17) — folded into `SEM019`; never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)) |
| FUNC016–FUNC099 | Reserved |
| CLASS007 | Withdrawn (2026-08-17) — cases parser-owned (`SYN005`) or `CLASS005`'s; never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)) |
| CLASS013–CLASS099 | Reserved |
| IDX006–IDX099 | Reserved |
| STATE007–STATE099 | Reserved |
| IMPORT005 | Withdrawn (2026-08-01) — folded into `IMPORT001`; never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)) |
| IMPORT006–IMPORT099 | Reserved |
| LIB021–LIB099 | Reserved |
| COM018–COM099 | Reserved |
| RUN006–RUN010 | Assigned — JSON parser runtime errors (`json.textToData` / `json.tryTextToData`; see [`../15-standard-library.md`](../04%20language/15-standard-library.md) §JSON Module and [`../11-testing.md`](../04%20language/11-testing.md) §Conformance Testing for Standard-Library Parsers) |
| RUN020–RUN099 | Reserved |
| MEM004–MEM099 | Reserved |
| BLD002–BLD099 | Reserved |
| BLOCK007–BLOCK099 | Reserved |
| CFG006–CFG099 | Reserved |
| RQD003–RQD099 | Reserved |
| CAP004–CAP099 | Reserved |

---

## Changelog

- 2026-08-20 — `RUN003`'s short description scoped to what the rule body now states ([10 §RUN003](./10-semantic-rules.md#run003--arithmetic-error), upgraded from stub the same day via [work/2026-08-20-runtime-error-message-wordings.md](../work/2026-08-20-runtime-error-message-wordings.md)): integer division/remainder by zero, integer division overflow, and failed numeric conversions — with the boundary that `number` arithmetic follows IEEE 754 and never raises. The old one-liner's unqualified "division by zero" read as covering `1.0 / 0.0`. No ID added, removed, or changed ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)).
- 2026-08-18 — §5 Reserved stale row corrected: it read `LIB020–LIB099` while `LIB020` `SourceBlockMalformed` has been registered in §3.9 since 2026-08-01 — the reserved row was never recalculated when the code landed. Now `LIB021–LIB099`. No ID added, removed, or changed ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)). Found by the compiler's Milestone 5 (`clean-language-compiler/docs/DISCOVERIES-M5.md`, item 10).
- 2026-08-17 — M4 registry pass, closing the code gaps the compiler's Milestone 4 hit (`clean-language-compiler/docs/DISCOVERIES-M4.md`; rule bodies in [10](./10-semantic-rules.md) the same day). **Registered per [ERC-03](#erc-03--registration-process)** (new cases within existing classes — no ADR required): [`SEM027`](#32-semantic-codes-sem) `LossyIntegerPromotion` (Warning — the [TYP-06](../04%20language/04-type-system.md#typ-06--type-conversion) warning could not be emitted without inventing a code, a [DIA-01](./13-diagnostic-format.md#dia-01--every-diagnostic-carries-a-registry-code)/ERC-04 breach; item 1); [`SEM028`](#32-semantic-codes-sem) `UndefinedField` (CLS-04 field access had no missing-field code and `SEM022`'s template says "no method named"; item 12); [`FUNC015`](#34-function-codes-func) `DuplicateStartBlock` ([FNC-01](../04%20language/09-functions.md#fnc-01--start-is-the-entry-point)'s "one per file" had no owner and the grammar has no cardinality; item 4). **Withdrawn:** `FUNC001` (define-before-use contradicts chapter 09 — forward references and mutual recursion are legal; folded into `SEM019`; item 18) and `CLASS007` (its cases are parser-owned `SYN005` or `CLASS005`'s; item 11); identifiers never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)). §1.1 matrix recounted both ways: 165 registered / 8 withdrawn / 157 emittable (151 rules in 10 + 6 `BLOCK` in 21). §5 Reserved recalculated (SEM029–, FUNC016–, CLASS013–) and three stale rows corrected — it still read `SCOPE006–`, `FUNC013–` and `CLASS007–` while SCOPE006, FUNC013/014 and CLASS007–CLASS012 were all registered.
- 2026-08-15 — §1.1's matrix count reformulated. "162 registered, 1 withdrawn — 161 active" only closed by counting five withdrawn identifiers (`SCOPE005`, `LIB005`, `LIB007`, `LIB008`, `LIB009` — each marked withdrawn in its own row) as active, with only `IMPORT005` on the withdrawn side. The count now distinguishes rows registered (162) / withdrawn (6) / emittable (156 = 150 rules in [10](./10-semantic-rules.md) + the 6 `BLOCK` bodies in [21](../04%20language/21-block-handlers.md)), verified mechanically both ways. No ID added, removed, or changed ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)). Discovered while registering the codes in the compiler's Milestone 2 (`clean-language-compiler/docs/DISCOVERIES-M2.md`, item 4).

- 2026-08-10 — New range **`CAP001–CAP099`** (Capability wiring) registered per [ERC-03](#erc-03--registration-process), with `CAP001` `NoBackendAvailable`, `CAP002` `BackendNotInstalled`, and `CAP003` `BackendUnknown` (§3.18). Emitted by Clean Framework at Moment 1 when a guest's capability import cannot be resolved to a bridge backend ([FRM-BO-12](../02%20components/framework/11-build-orchestration.md#frm-bo-12--bridges-is-derived-from-the-guests-imports-never-from-configuration-alone), [FRM-BO-15](../02%20components/framework/11-build-orchestration.md#frm-bo-15--unsatisfiable-capabilities-fail-at-moment-1-the-startup-check-is-unchanged)). This is a new class of failure — backend resolution above the interface layer, distinct from the `COM014`/`COM015` world and version checks — so it carries its own range per [ADR-0032](../01%20governance/decisions/0032-capability-wiring-generated-host-toml.md). §5 Reserved recalculated (CAP004–). Rule bodies added to [10 — Semantic Rules](./10-semantic-rules.md).

- 2026-08-02 — [`RUN019`](#312-runtime-codes-run) `ReadOfCancelledTask` registered per [ERC-03](#erc-03--registration-process), backing [ASY-03](../04%20language/18-async.md#asy-03--cancelling-and-failing) under [ADR-0012](../01%20governance/decisions/0012-async-cancellation-and-failure.md).
- 2026-08-02 — [`STATE006`](#37-state-codes-state) `StateRuleViolated` registered per [ERC-03](#erc-03--registration-process), backing [SMG-03](../04%20language/20-state-management.md#smg-03--state-rules) under [ADR-0011](../01%20governance/decisions/0011-state-rules-runtime-semantics.md). It is distinct from `STATE002`: a guard rejecting an update is an expected outcome and the program continues, while a violated whole-state rule is a defect.
- 2026-08-02 — [`RUN018`](#312-runtime-codes-run) `UnhandledError` registered per [ERC-03](#erc-03--registration-process), backing [ERH-05](../04%20language/13-error-handling.md#erh-05--an-unhandled-failure-ends-the-program) under [ADR-0016](../01%20governance/decisions/0016-error-value-or-signal.md). It is a runtime code because a signature does not record that a function can fail, so an unhandled failure is not decidable at compile time.
- 2026-08-02 — [`SEM026`](#32-semantic-codes-sem) `LiteralOutOfRange` registered per [ERC-03](#erc-03--registration-process), under [ADR-0019](../01%20governance/decisions/0019-precision-modifiers.md). This is the diagnostic [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md) deliberately left unregistered when it settled that a range is measured after unary minus applies: the gap belonged to ADR-0019, and withdrawing precision modifiers from the surface language reduced it from a matrix of seven widths to one condition over `integer` and `number`. §5 Reserved recalculated (SEM027–).
- 2026-08-02 — [`RUN016`](#312-runtime-codes-run) `MatrixShapeMismatch` and [`RUN017`](#312-runtime-codes-run) `MatrixSingular` registered per [ERC-03](#erc-03--registration-process), backing the Matrix module under [ADR-0018](../01%20governance/decisions/0018-matrix-operator-overloading.md). Both are runtime codes because `matrix<T>` is dynamically sized, so shape is not carried in the type and cannot be checked at compile time. Element-type errors need no new code — an `inverse()` on a `matrix<integer>`, or an operator between matrices of different element types, is [`SEM004`](#32-semantic-codes-sem) `InvalidOperationForType`. §5 Reserved corrected: it read `RUN013–RUN099` while `RUN013`–`RUN015` were all in use.
- 2026-08-02 — [`SEM025`](#32-semantic-codes-sem) `ControlFlowOutsideLoop` registered per [ERC-03](#erc-03--registration-process), backing [FLW-03](../04%20language/12-control-flow.md#flw-03--break-and-continue) under [ADR-0017](../01%20governance/decisions/0017-break-and-continue.md). One code covers both keywords: the condition and the rule are identical and differ only in the word reported, so two codes would have been one rule with two homes — the ADR had anticipated two. §5 Reserved corrected: it read `SEM023–SEM099` while `SEM023` and `SEM024` were both in use.
- 2026-08-01 — `SCOPE006` `CompiletimeHelperOutsideTest` registered (approved via §1.2), closing the last failure in `04 language/` that was described with no code: [21 §21.9](../04%20language/21-block-handlers.md) restricts the `test.compiletime` namespace to `tests:` blocks and no existing code fitted.
- 2026-08-01 — Sixteen codes registered for `04 language/`, closing gaps where a failure was described with no name ([SDD-04](../01%20governance/03-spec-driven-design.md)): `SYN009`, `SYN010`, `FUNC013`, `FUNC014`, `CLASS007`–`CLASS012`, `SEM023`, `SEM024`, `LIB020`, `RUN013`–`RUN015`. Four codes **withdrawn** as duplicates of the `BLOCK` family, ending three breaches of [ERC-02](#erc-02--one-code-one-rule): `LIB005`→`BLOCK001`, `LIB007`→`BLOCK006`, `LIB008`→`BLOCK004`, `LIB009`→`BLOCK005`; the identifiers are never reused. `IDX001` renamed `ListIndexNotInteger` ("Array" is a rejected term) and `IDX003` renamed `PairsKeyTypeMismatch` — it required a string key, contradicting the generic `K` of the type system, a correction ruled on in the platform pass and never applied. Matrix verified both ways: 150 codes ↔ 144 rule bodies plus the 6 `BLOCK` bodies in [21](../04%20language/21-block-handlers.md).
- 2026-08-02 — [`CFG005`](#316-configuration-codes-cfg) `FileEncodingInvalid` registered per [ERC-03](#erc-03--registration-process), backing [TXT-02](./17-text-encoding.md#txt-02--the-reader-validates-at-the-moment-it-reads) in the new [17 — Text Files](./17-text-encoding.md). The `CFG` range is widened from manifest schema to *project input files read by the toolchain*: the same emitters (Framework, Manager) plus the language server, now covering the encoding of every file they read as well as the schema of the two manifests. It is a new case within CFG's existing class — a toolchain component rejecting a malformed project input file — not a new class or a new range, so no ADR is required by [ERC-03](#erc-03--registration-process). §5 Reserved recalculated (CFG006–). **§1.1's matrix count corrected**: it read "134 registered, 133 active, 127 with rules", stale by 17 codes and inconsistent with the count in [10 — Semantic Rules](./10-semantic-rules.md)'s own changelog. Both files were re-counted mechanically: §3 registers 152 unique codes (`IMPORT005` appears twice — once as its row, once as its withdrawal record), 10 holds 146 rule bodies and 21 holds the 6 `BLOCK` bodies, so the [ERC-02](#erc-02--one-code-one-rule) 1:1 obligation is intact and the only codes absent from 10 are the six declared exceptions.
- 2026-08-01 — Technical-debt closure pass (lote X, approved 2026-08-01): `IMPORT005` **withdrawn** — it duplicated `IMPORT001` (same failure: module import cycle); folded into `IMPORT001`, whose short description now carries the resolve-pass detection ([14 §14.4.2](./14-compiler-architecture.md)); the number is never reused (DOC-13), and §1.1's matrix count now reads "134 registered, 1 withdrawn". Three new codes formally registered per [ERC-03](#erc-03--registration-process): [`CFG004`](#316-configuration-codes-cfg) `LockfileMismatch` (Error — CI build where `clean.toml` and `.cln/lock.toml` disagree, [07 CONF-04](./07-build-config.md#77-dependencies)); [`MEM003`](#314-memory-codes-mem) `ArenaImbalance` (Runtime — unbalanced `arena-pop` or pop past a foreign save-point, [03 MMD-03](./03-memory-model.md#mmd-03--arena-discipline-every-push-balanced-by-exactly-one-pop)); [`RUN012`](#312-runtime-codes-run) `TimeBudgetExceeded` (Runtime — wall-clock/epoch budget exhaustion, [03 §3.5](./03-memory-model.md#35-host-backing--observable-contract) + [07 `[runtime]`](./07-build-config.md#72-schema--top-level)). §5 Reserved recalculated: CFG005–, MEM004–, RUN013–, plus the IMPORT005 withdrawn row.
- 2026-08-01 — Fase 4 (lote 1): formal registration of the codes approved in the diagnostic-code mapping (2026-08-01) — `MEM001` TierExceeded, `MEM002` ArenaEscape; `BLD001` BuildLimitExceeded; `CFG001`–`CFG003` (manifest schema / constraint / warning); `COM009`–`COM017` (bridge resolve/link/runtime, world import check, codegen invariant, Moment 1/2 mismatches, deprecated member use, instantiation failure); new range `RQD001–RQD099` (compilation request document) with `RQD001`/`RQD002`; `IMPORT005` ImportCycle; `LIB019` HostBridgeMisplaced (P10); `RUN011` ContractViolation (P10); `BLOCK001`–`BLOCK005` index rows with newly ratified names (`AmbiguousBlockName`, `UnknownBlockName`, `ReservedBlockName`, `HandlerMalformedIR`, `HandlerBudgetExceeded`; rule bodies remain in 21 §21.6). `STATE003` symbolic name changed `ComputedReturnTypeMismatch` → `CircularStateDependency` (rename ratified; old name withdrawn per DOC-13). §5 Reserved ranges recalculated; §1.1 records the verified matrix count (131 codes = 125 rules in 10 + 6 in 21). Traceability compliance pass: claimed rule prefix `ERC-`; minted `ERC-01` (code format), `ERC-02` (1:1 rule), `ERC-03` (registration process), `ERC-04` (every diagnostic carries a code), each with concern citations; sections marked *Normative.*/*Informative.*
- 2026-08-01 — Fase 3 remediation per the approved conflict log (P16.7, P16.8, resolution 0.4): §2 adopts the severity vocabulary of 13 §3 (`error | warning | info | help`) and defines the Runtime marker as phase-not-level (renders as `level = error`, LSP severity 1 per 13 §7); added `BLOCK006` to the index and range-placeholder sections for MEM (§3.14), BLOCK (§3.15), CFG (§3.16) plus their §5 Reserved rows; renumbered §3.15 → §3.13 (Core-Library Sub-Labels); STATE003 short description narrowed to circular-dependency only (type mismatch ceded to SEM018); §1.2 "spec RFC" → ADR process; §4 "honest-code principle" → cite [C-02](../01%20governance/05-concerns.md); COM008 row `[build.limits]` → `[compile.limits]` (schema home: 07). Concrete new codes from the diagnostic-code mapping (MEM/BLD/CFG/COM/RQD rows, LIB019, RUN011) are deliberately NOT added — their formal registration is Fase 4.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler and runtime maintainers registering or amending diagnostic codes; framework and library authors implementing the diagnostic contract
- **Rule prefix:** `ERC-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [10 — Semantic Rules](./10-semantic-rules.md) (rule bodies), [13 — Diagnostic Format](./13-diagnostic-format.md) (message anatomy, LSP mapping, `cln explain`)
- **Satisfies:** LANG-03, LANG-16, INTEROP-10
