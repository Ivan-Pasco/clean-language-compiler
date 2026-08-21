# 21. Block Handlers

> **Companion artifacts (Docs Readiness Program, Stage 3 B1):**
> - Grammar: [`grammar/21-block-handlers.ebnf.md`](./grammar/21-block-handlers.ebnf.md) — `CompileTimeFunctionDeclaration`, `HandlesBlockDeclaration`, `BlockNodeType`, `BlockArgType`, `DiagnosticEmission` (DOC-15 companion).
> - Schema: [`schema/block-ast.md`](./schema/block-ast.md) — `BlockAST`, `BlockNode`, `BlockArg`, `BlockAttribute`, `BlockLine`, `Token`, `Span`, `IR` builder API (DOC-18 companion).
> - Worked examples: [`../05 execution/language/patterns/block-handlers-worked-examples.md`](../05%20execution/language/patterns/block-handlers-worked-examples.md) — runnable illustrations of BLK-01..04 (informative, per DOC-11 tier 5).
>
> The four platform documents that anchor into this chapter's `21.x` heading numbering (see the 2026-08-01 changelog note) keep working — this chapter's structure is unchanged. The companions extract the grammar and schema for tooling consumption; this chapter remains the authoritative home of the BLK- rules and the accompanying prose.

A block handler is a `compiletime function` in a library that turns a generic `identifier:` block — `data:`, `endpoints:`, `component:`, `html:`, `canvas:` — into typed IR the compiler splices into the program. This chapter is the authoritative contract: how handlers are declared, how block names resolve when multiple libraries could handle one, the shape of the `BlockAST` a handler receives and the `IR` it returns, how it emits diagnostics, and what the compile-time execution environment guarantees (deterministic, sandboxed, bounded).

A **block handler** is a compile-time function that turns a generic `identifier:` block, written by the user, into typed IR that the compiler splices into the program. Block handlers are how libraries add new DSL syntax (`data:`, `endpoints:`, `component:`, `canvas:`, `html:`) without changing the compiler itself.

This section defines the concrete contract: the grammar for declaring a handler, the shape of the AST passed in, the shape of the IR returned, how the compiler resolves which library owns a block name, how handlers emit diagnostics, and the execution environment guarantees.

For the motivation, see [Libraries Specification §3](../02%20components/framework/09-libraries-specification.md#3-language-additions). This section is the authoritative contract; the illustrative examples in Libraries Specification §3.2 are subordinate to what appears here.

---

## 21.1 Declaring a Block Handler

### BLK-01 — Declaring a block handler

A block handler is a `compiletime function` paired with a `handles block` registration. Both live in library source (not compiler source).

**Syntax:**

```
CompiletimeFunction = "compiletime" "function" Identifier "(" ParamList ")" [ "returns" TypeName ] NEWLINE INDENT FunctionBody DEDENT

HandlesBlock        = "handles" "block" StringLiteral "with" Identifier NEWLINE
```

**Example:**

```clean
compiletime function expandDataBlock(BlockAST ast) returns IR
	description "Interprets `data <Name>:` blocks and emits the companion class"
	// walk ast.body, emit typed IR
	...

handles block "data" with expandDataBlock
```

**Rules:**

- `compiletime` is a keyword prefix on `function`, mirroring `constant` and `host`.
- The function's parameter list must have exactly one parameter of type `BlockAST` (see §21.3).
- The function's return type must be `IR` (see §21.4). The `returns IR` clause is not optional.
- A `handles block` declaration must reference a `compiletime function` defined in the same library.
- A library may declare zero or more `handles block` registrations. Each registration binds one block name to one handler.
- The block name in the string literal must be a valid qualified identifier (`name` or `name.name.name` — no spaces, no punctuation other than `.`).

---

## 21.2 Block Name Resolution

### BLK-02 — Block name resolution order

When the compiler parses `data UserData:` in a user's source file, it must resolve which library's handler runs. Resolution follows these rules, in order:

1. **Explicit library import wins.** If the file contains `import data.experimental` and that library registers `handles block "data" with ...`, the explicit-import handler runs.
2. **Implicit folder scope, single library.** If the file's path matches exactly one entry in the project's `clean.toml [folders]` that brings a handler-carrying library into scope, that library's handler runs.
3. **Implicit folder scope, multiple libraries.** If two or more implicitly-scoped libraries register the same block name, compilation fails with `BLOCK001` (see §21.6). The user must either narrow the folder mapping in `clean.toml` or add an explicit `import` to disambiguate.
4. **No handler registered.** If no library in scope registers the block name, compilation fails with `BLOCK002`.

**Qualified names.** A block written as `data.query UserQuery:` looks up the exact string `"data.query"` — not `"data"` with `.query` as a modifier. Libraries must register each qualified name they intend to handle; there is no prefix-match fallback.

**Name reservation.** A library may not claim any name reserved by the language, nor any name beginning with `core.`. The reserved set is the four keyword tables of [3 — Lexical Structure §Keywords](./03-lexical-structure.md), which is its single home — this section does not keep a second list. Attempting to register a reserved name fails at library load with `BLOCK003`.

---

## 21.3 The `BlockAST` Type

`BlockAST` is a typed node the compiler passes to the handler. It is a struct with these fields:

| Field | Type | Meaning |
|-------|------|---------|
| `name` | `string` | The qualified identifier (`"data"`, `"data.query"`, `"html"`). |
| `arguments` | `list<BlockArg>` | Positional and keyword arguments passed in parentheses. Empty list if none. |
| `body` | `list<BlockNode>` | Child nodes of the block: statements, nested blocks, or structured lines. |
| `attributes` | `list<BlockAttribute>` | Modifier annotations (see §21.3.1). Empty list if none. |
| `span` | `Span` | Source location — file path, line, column, byte range — anchored to the block header (the `identifier:` token). |

**`BlockNode` is a sum type:**

```
BlockNode = Statement            // a normal Clean statement, already typed
          | BlockAST             // a nested block (recurse)
          | BlockLine            // a structured line inside a DSL block (see §21.3.2)
```

**`BlockArg`:**

```
BlockArg = Positional Expression
         | Keyword Identifier Expression
```

Expressions inside arguments are already parsed and typed. The handler receives them as typed IR fragments, not raw text.

### 21.3.1 Block Attributes

Attributes decorate a block. They are written as a keyword-prefixed line at the top of the block body — no sigils, matching Clean's convention for `private`, `constant`, `compiletime`, and `host`:

```clean
data UserData:
	deprecated "Use ExtendedUserData instead"
	fields:
		integer id primary
```

`attributes` on the outer `BlockAST` for `data:` contains one entry: `{name: "deprecated", arguments: ["Use ExtendedUserData instead"]}`.

`BlockAttribute`:

| Field | Type | Meaning |
|-------|------|---------|
| `name` | `string` | The attribute name. |
| `arguments` | `list<Expression>` | Optional arguments. |
| `span` | `Span` | Source location of the attribute. |

### 21.3.2 Structured Lines (`BlockLine`)

Inside a DSL block, some lines are not full Clean statements — they are structured entries the DSL interprets. Example, inside `fields:`:

```
integer id primary
string email required unique
string passwordHash as "password_hash"
```

Each such line is exposed as a `BlockLine`:

| Field | Type | Meaning |
|-------|------|---------|
| `tokens` | `list<Token>` | The line's tokens after lexing. |
| `span` | `Span` | Source location. |

The DSL handler decides how to interpret tokens. The compiler does not attempt to parse a `BlockLine` as an expression or statement.

Tokens carry the information a DSL needs:

| Token variant | Fields | Meaning |
|---------------|--------|---------|
| `Identifier` | `text: string` | An unquoted name. |
| `Keyword` | `text: string` | A reserved word (`primary`, `unique`, `required`, `as`, etc. — the DSL decides which names are meaningful). |
| `String` | `text: string` | A `"..."` literal. |
| `Integer` | `value: integer` | A parsed integer literal. |
| `Number` | `value: number` | A parsed number literal. |
| `Symbol` | `text: string` | A punctuation token (`.`, `,`, `->`, etc.). |

The distinction between `Identifier` and `Keyword` is made by the tokenizer against the language's global keyword set (see [`03-lexical-structure.md`](./03-lexical-structure.md)). Names that are language keywords come through as `Keyword`; everything else comes through as `Identifier`. Handlers that want their own DSL keywords (like `primary`) treat matching `Identifier` tokens as keywords by convention.

---

## 21.4 The `IR` Return Type

`IR` is a typed fragment of the compiler's intermediate representation, built through a constructor API the handler calls. Handlers do not construct raw AST trees; they build IR through typed builder functions the compiler exposes.

**The IR builder API (surface):**

```clean
namespace ir
	// Declarations
	IR class(string name, list<Field> fields, list<Method> methods)
	IR function(string name, list<Parameter> params, TypeRef return_, IR body)
	IR field(string name, TypeRef type)
	IR method(string name, list<Parameter> params, TypeRef return_, IR body)

	// Statements
	IR return_(IR expression)
	IR assign(string name, IR expression)
	IR if_(IR condition, IR then_, IR else_)
	IR block(list<IR> statements)

	// Expressions
	IR call(string qualified_name, list<IR> arguments)
	IR literal_integer(integer value)
	IR literal_string(string value)
	IR variable(string name)
	IR field_access(IR receiver, string field)

	// Composition
	IR concat(list<IR> fragments)  // splice multiple IRs sequentially
	IR empty()                     // no-op, valid IR fragment
	IR withSpan(IR node, Span span) // re-anchor a node's source span (§21.5)
```

**Rules:**

- `IR` is opaque to the handler — there is no field access and no method call on an `IR` value. Handlers compose IR only through the `ir` builder functions, including `ir.withSpan(node, span)` (§21.5).
- Every builder function is a pure factory: it produces a new `IR` value without side effects.
- Returning `ir.empty()` is valid and means "this block contributes nothing to the program" (useful for pure-annotation blocks).
- The compiler is responsible for validating that the returned IR is well-typed against the surrounding compilation unit. IR that references an undefined symbol produces a diagnostic at the corresponding user span, not at "generated code line N."

The IR builder surface is the one defined in §21.4 above; there is no other.

---

## 21.5 Span Preservation

Every IR node the handler builds inherits a span. By default, that span points at the block header the handler is processing. Handlers should override the span when a subtree corresponds to a specific sub-region of the user's source, so that diagnostics later in compilation point at the correct line.

```clean
compiletime function expandDataBlock(BlockAST ast) returns IR
	// Emit a field. Span this field to the source line that declared it.
	BlockLine line = ast.body[0]
	return ir.withSpan(ir.field("id", TypeRef.integer()), line.span)
```

**Rule:** Diagnostics from later compilation stages (semantic analysis, codegen) always point at the span carried by the offending IR node. There is no generated-source view.

---

## 21.6 Diagnostics from Compile-Time Functions

### BLK-03 — Handler diagnostics

Handlers emit diagnostics via three top-level functions available inside `compiletime` bodies:

```clean
error(string code, string message, Span span)
warning(string code, string message, Span span)
info(string code, string message, Span span)
```

**Rules:**

- `code` is a library-supplied **sub-label** (kebab-case, e.g. `"field-missing-type"`). It is not a diagnostic code — those are `PREFIX###` and belong to the registry ([Glossary](../01%20governance/06-glossary.md)). Library diagnostics surface to the user as [`LIB010`](../03%20platform/09-error-codes.md) with this sub-label attached — libraries do not own code prefixes (Platform 09 §1). Sub-labels registered in the library's `library.toml [mcp.diagnostics]` section are discoverable via the MCP; unregistered sub-labels work but produce a warning at library load.
- `message` is human-readable prose. Interpolation with the user's identifiers is encouraged.
- `span` must be a real source span from the input `BlockAST` or one of its children. Passing a synthetic span produces a compilation error in the library itself.
- `error` halts compilation for the current file after the handler returns; the compiler still finishes analyzing the rest of the project so multiple errors surface in one pass.
- `warning` and `info` do not halt compilation.

**Reserved error codes** (emitted by the compiler, not by handlers). Names are the registered symbolic names of [Platform 09 §3.15](../03%20platform/09-error-codes.md#315-block-handler-codes-block):

| Code | Name | Meaning |
|------|------|---------|
| `BLOCK001` | `AmbiguousBlockName` | Ambiguous block name — two libraries in implicit scope register the same block name. |
| `BLOCK002` | `UnknownBlockName` | Unknown block name — no library in scope registers this block name. |
| `BLOCK003` | `ReservedBlockName` | Reserved block name — a library attempted to register a name reserved by the language. |
| `BLOCK004` | `HandlerMalformedIR` | Handler returned malformed IR (referenced undefined symbol, type mismatch, etc.). |
| `BLOCK005` | `HandlerBudgetExceeded` | Handler exceeded its compile-time budget (see §21.7). |
| `BLOCK006` | `HandlerForbiddenSideEffect` | Handler attempted a forbidden side effect (I/O, non-determinism). |

---

## 21.7 Compile-Time Execution Environment

### BLK-04 — The compile-time execution environment

Compile-time functions run inside the compiler process, in a restricted execution environment.

**Guarantees:**

- **Deterministic.** Given the same `BlockAST` input and the same set of in-scope libraries, a handler produces the same IR. Determinism is what makes caching sound; where handler artifacts are cached, and by which component, is decided in [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md).
- **Sandboxed.** Handlers cannot access the filesystem, network, environment variables, or system time. `random()`, `file.read()`, `http.get()`, and equivalent standard-library calls are unavailable at compile time. Calling any of them from a `compiletime` context is `BLOCK006`. There is no exception for reading files.
- **Bounded, per invocation.** Each handler invocation has a wall-clock budget (default: 5 seconds) and a memory budget (default: 128 MB). Exceeding either produces `BLOCK005` and aborts the handler. The `IR` an invocation returns may nest builder nodes at most **128 levels** deep; deeper nesting is malformed IR (`BLOCK004`, "nesting exceeds the depth limit"), not a budget breach. No legitimate §21.4 composition approaches this depth — the cap exists so a defective handler cannot overflow the compiler's own stack within the node budget, which would be an internal compiler error ([CMP-04](../03%20platform/14-compiler-architecture.md#cmp-04--internal-failures-are-com013-never-a-user-error)).
- **Bounded, per library.** Beyond the per-invocation budgets, a library as a whole is capped: 512 MiB of compile-time heap across all of its handler invocations, and 500 000 IR nodes emitted by any single invocation. Exceeding one of these is [`LIB014`](../03%20platform/09-error-codes.md#39-library-codes-lib) — a limit on the library, not on one block, which is why it is not a `BLOCK` code.

  All of these defaults are configurable per project under `[compile.limits]` in `clean.toml`; the schema is owned by [Platform 07 — Build Configuration](../03%20platform/07-build-config.md).
- **Isolated.** A handler's crash or panic is caught by the compiler and reported as `BLOCK004` at the user's block span; it does not crash the compiler.

**What compile-time code CAN do:**

- Call other pure Clean functions from the same library or its dependencies.
- Read `BlockAST` fields freely.
- Emit diagnostics.
- Build IR through the `ir` builder API.
- Access constants declared with `constant` in the same library.

**What compile-time code CANNOT do:**

- Call `host function`s. Host functions run at runtime; a `compiletime` context has no host.
- Call other `compiletime function`s in other libraries directly (no cross-library compile-time API). If one library needs another's help at compile time, it composes through the IR that other's handler emits, not through direct calls.
- Mutate global state. There is no global state at compile time; handlers are pure functions.

**Implementation note (non-normative):** The reference compiler runs `compiletime` functions through the same WASM runtime it targets, compiled from Clean source with all host imports stubbed to `BLOCK006`. This gives correctness parity between the compile-time and runtime interpretations of a library's own code. The execution model — handlers distributed as Clean source, compiled to WASM and cached by the framework, and executed by the compiler in its sandboxed wasmtime pass — is decided in [ADR-0004 — Block handler execution model](../01%20governance/decisions/0004-block-handler-execution-model.md).

---

## 21.8 Host Function Typechecking

Host functions a library declares (see [Libraries Specification §8](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)) are typechecked by the compiler against uses in the library and against the host's registered implementations.

**Compile-time checks:**

- Every call to a `host function` in library source is typechecked against the function's declared signature.
- Return types are propagated normally.

**Link-time checks:**

- Before emitting the final WASM module, the compiler collects every `host function` declaration reachable from the entry point. For each, it verifies that the host declared by the library's `host interface` provides an implementation with a matching signature ([Libraries Specification §8](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations) owns that grammar; see also [Platform 02 — Host Bridge](../03%20platform/02-host-bridge.md)).
- Missing implementations produce a link error naming the library, the function, and the host that should provide it.
- Signature drift between declaration and registry produces a link error with both signatures shown side by side.

**No runtime check.** By the time the WASM module runs, all host function bindings have been validated. There is no runtime type check on host function calls.

---

## 21.9 Testing Block Handlers

Library authors write tests for `compiletime` functions the same way they write tests for runtime functions — with the `tests:` block (see [`11-testing.md`](./11-testing.md)). The compiler exposes helpers under the `test.compiletime` namespace for constructing `BlockAST` inputs and inspecting `IR` outputs.

```clean
tests:
	"expandDataBlock emits a class with expected fields"
		BlockAST input = test.compiletime.parseBlock("""
		data UserData:
			fields:
				integer id primary
				string email required
		""")
		IR output = expandDataBlock(input)
		assert test.compiletime.classFieldNames(output, "UserData") == ["id", "email"]

	"expandDataBlock rejects field without a type"
		BlockAST input = test.compiletime.parseBlock("""
		data UserData:
			fields:
				id primary
		""")
		list<Diagnostic> diags = test.compiletime.collectDiagnostics(expandDataBlock(input))
		assert diags.length() == 1
		assert diags[0].code == "field-missing-type"
```

**Rules:**

- `test.compiletime` helpers run only inside `tests:` blocks. Using one outside a test is [`SCOPE006`](../03%20platform/09-error-codes.md#33-scope-codes-scope).
- `parseBlock` accepts a raw string containing exactly one top-level block and returns its `BlockAST`.
- `collectDiagnostics` catches diagnostics emitted by the handler under test, returning them as data instead of halting the test.
- `classFieldNames`, `functionNames`, `hasMethod`, and similar inspectors let tests assert on the IR's shape without depending on internal IR structure.

---

## 21.10 Interaction with Other Language Features

- **[Imports](./17-modules-and-imports.md).** `import` statements bring library exports into scope for runtime code. Block handlers are not exported values and are not imported directly; they are activated by `handles block` registrations picked up during library resolution.
- **[Contracts](./10-contracts.md).** `compiletime` functions may declare `before` and `after` contracts on their parameters and return value. `always` invariants do not apply — compile-time code has no persistent state.
- **[State](./20-state-management.md).** `state:` blocks are runtime constructs. A block handler that generates a `state:` block emits it as IR; the handler itself does not have or use state.
- **[Classes](./14-classes-and-objects.md).** Handlers may emit `class` declarations, including capability conformance (`class UserData can Persist` on a companion type). The emitted class is subject to the same semantic rules as a class written directly in source.
- **[Async](./18-async.md).** `compiletime` functions cannot use `start`, `later`, or `background`. Compile time is synchronous.
- **[Error handling](./13-error-handling.md).** `onError` inside a `compiletime` function catches runtime-style errors from Clean code the handler calls; it does not catch or convert `error(...)` diagnostic emissions, which are structured output not exceptions.

---

## 21.11 Boundary Rules
1. **No handler composition.** Two handlers in one library cannot cooperate on the same block. The outer handler recurses into its own body and interprets nested blocks itself.
2. **No cross-library IR extension.** A library cannot define new IR node types for other libraries to emit. The IR builder API is closed.
3. **A handler is a pure function of its block.** Its result depends on nothing but the `BlockAST` it receives and the libraries in scope, which is what allows an unchanged block to be skipped on a rebuild. Whether and where results are cached is a toolchain decision, not a language rule — see [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md).

---

## Changelog

- 2026-08-18 — §21.7's per-invocation budget bullet gains an IR nesting-depth cap (128 levels; exceeding it is `BLOCK004` "nesting exceeds the depth limit", not `BLOCK005`), from the compiler's Milestone 5 (`clean-language-compiler/docs/DISCOVERIES-M5.md`, item 13): within the 500 000-node budget a handler could nest `concat`/`withSpan` deep enough to overflow a recursive lowerer's stack — an internal compiler error ([CMP-04](../03%20platform/14-compiler-architecture.md#cmp-04--internal-failures-are-com013-never-a-user-error)) long before the node budget fires. Ratifies the compiler's M5 adoption verbatim; no legitimate §21.4 composition approaches the cap.
- 2026-08-17 — Erratum from compiler Milestone 3 (`clean-language-compiler/docs/DISCOVERIES-M3.md`, item 1): the `expandDataBlock` example signature in §21.1 and §21.5 was `(block BlockAST)` — unparseable under the Accepted specs on two counts: `Parameter` is type-first per [`grammar/09-functions.ebnf.md`](./grammar/09-functions.ebnf.md) (authoritative per DOC-15, referenced by `21-block-handlers.ebnf.md` §1), and `block` is a LEX-04 hard keyword, so using it as an identifier is SYN002. Both examples now read `(BlockAST ast)`, with the body references (`block.body` → `ast.body`) updated to match. No grammar or rule change — the examples were wrong, not the productions. (The discovery cites §21.6 for the second site; the actual second occurrence is §21.5.)
- 2026-08-01 — Fase 5 (zero-debt pass): the `test.compiletime` restriction now cites [`SCOPE006`](../03%20platform/09-error-codes.md#33-scope-codes-scope); §21.11 marked *Normative*; the §21.9 example no longer asserts `"field-redeclaration-drift"`, a sub-label registered for a different failure and never declared by any manifest in the repository.
- 2026-08-01 — Fase 3/4 (L14): the four `LIB` codes that duplicated `BLOCK001`/`BLOCK004`/`BLOCK005`/`BLOCK006` withdrawn from the registry, ending three breaches of [ERC-02](../03%20platform/09-error-codes.md#erc-02--one-code-one-rule); `LIB018`'s boundary with `BLOCK001` stated; the per-library limits of `LIB014` brought into §21.7, where they were missing. The last surviving `host function … from "…"` in the repository replaced by the [LBS-02](../02%20components/framework/09-libraries-specification.md) grammar. §21.2's private list of reserved names replaced by a citation of [3 — Lexical Structure](./03-lexical-structure.md), which is its home — the two lists had already diverged. The `IR` opacity contradiction resolved: `withSpan` is a builder function, not a method on an opaque value, and is now registered in the §21.4 surface. §21.11's second cache rule replaced by a citation of [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md), which already decided caching. Six stale chapter cross-references corrected (they used a superseded numbering), the LBS §4/§9 citations repointed to §3/§8, and a self-link removed. Rules `BLK-01`..`BLK-04` minted **without touching the `21.x` heading numbering**, which four platform documents anchor into. `readSpecFile` noted as not carried over: it is defined in no chapter.
- 2026-08-01 — Fase 4 (lote 1): `BLOCK001`–`BLOCK005` are now formally registered in [Platform 09 §3.15](../03%20platform/09-error-codes.md#315-block-handler-codes-block) alongside `BLOCK006`; the §21.6 reserved-codes table gains a Name column with the ratified symbolic names (`AmbiguousBlockName`, `UnknownBlockName`, `ReservedBlockName`, `HandlerMalformedIR`, `HandlerBudgetExceeded`, `HandlerForbiddenSideEffect`) so chapter and registry coincide. Rule bodies remain here (declared exception to the 1:1 rule, [ERC-02](../03%20platform/09-error-codes.md#erc-02--one-code-one-rule)).
- 2026-08-01 — §21.7 implementation note now cites [ADR-0004 — Block handler execution model](../01%20governance/decisions/0004-block-handler-execution-model.md), where the execution model (source-distributed handlers, framework-compiled and cached, compiler-executed in the sandboxed wasmtime pass) was decided. Added Status header and Changelog section. `BLOCK006` (§21.6) confirmed defined here — [Platform 09](../03%20platform/09-error-codes.md) indexes it. Follow-up same day: handler diagnostic `code` redefined as a `LIB010` sub-label (kebab-case) per Platform 09 §1 — the previous "uppercase, hyphenated, library-prefixed" convention (`FRAME-DATA-E*`) was a nominally prohibited form; the §21.9 test example updated accordingly.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Library authors adding new DSL block syntax; compiler maintainers implementing the compile-time execution environment
- **Rule prefix:** `BLK-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Libraries Specification](../02%20components/framework/09-libraries-specification.md), [ADR-0004 — Block Handler Execution Model](../01%20governance/decisions/0004-block-handler-execution-model.md), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md) (`BLOCK` and `LIB` ranges), [Platform 07 — Build Configuration](../03%20platform/07-build-config.md) (`[compile.limits]`)
