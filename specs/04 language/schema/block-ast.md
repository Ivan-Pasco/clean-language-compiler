# BlockAST and IR — Canonical schema

Canonical schema for the compile-time value types passed to and returned by block handlers: `BlockAST`, `BlockNode`, `BlockArg`, `BlockAttribute`, `BlockLine`, `Token`, and the `IR` builder API surface. Per [DOC-18](../../01%20governance/00-documentation-principles.md#doc-18--structured-data-artifacts-have-a-single-canonical-schema-file), this file is the single source of truth for these type definitions; [`21-block-handlers.md §21.3, §21.4`](../21-block-handlers.md#213-the-blockast-type) — the previous home — cites fields from this file rather than restating them.

These types exist ONLY during compilation. They cannot be declared, constructed, or stored by ordinary runtime code ([TYP-04](../04-type-system.md#typ-04--compile-time-types)). The grammar shape of the types themselves (as sum-type variants named in Clean source) is in [`grammar/21-block-handlers.ebnf.md`](../grammar/21-block-handlers.ebnf.md); this file gives the field-level structure a handler pattern-matches against.

---

## `BlockAST` — The parsed block a handler receives

A struct passed to every `compiletime function` as its single parameter.

| Field | Type | Purpose |
|---|---|---|
| `name` | `string` | The qualified identifier (`"data"`, `"data.query"`, `"html"`). |
| `arguments` | `list<BlockArg>` | Positional and keyword arguments passed in parentheses. Empty list if none. |
| `body` | `list<BlockNode>` | Child nodes of the block: statements, nested blocks, or structured lines. |
| `attributes` | `list<BlockAttribute>` | Modifier annotations. Empty list if none. |
| `span` | `Span` | Source location — file path, line, column, byte range — anchored to the block header (the `identifier:` token). |

---

## `BlockNode` — Sum type over body children

A `BlockNode` is one of three shapes:

| Variant | Constructor payload | Meaning |
|---|---|---|
| `Statement` | a normal Clean statement, already typed | Ordinary Clean code appearing inside the block body. |
| `BlockAST` | a nested `BlockAST` | A block nested inside this block; the handler recurses. |
| `BlockLine` | a structured line (see below) | A DSL-specific line the handler tokenises itself. |

Handlers pattern-match on the variant to decide how to interpret each body element.

---

## `BlockArg` — Positional and keyword arguments

A `BlockArg` is one of two shapes:

| Variant | Constructor payload | Meaning |
|---|---|---|
| `Positional` | `Expression` (already parsed and typed) | An argument passed without a name. |
| `Keyword` | `Identifier`, `Expression` | An argument passed as `name = value`. |

Expressions inside arguments are already parsed and typed. The handler receives them as typed IR fragments, not raw text.

---

## `BlockAttribute` — Block modifier annotations

Attributes decorate a block. Written as a keyword-prefixed line at the top of the block body — no sigils, matching Clean's convention for `private`, `constant`, `compiletime`, and `host`.

| Field | Type | Purpose |
|---|---|---|
| `name` | `string` | The attribute name (`"deprecated"`, `"cache"`, etc.). |
| `arguments` | `list<Expression>` | Optional arguments. |
| `span` | `Span` | Source location of the attribute. |

**Example (source-side):**

```clean
data UserData:
    deprecated "Use ExtendedUserData instead"
    fields:
        integer id primary
```

Produces `attributes` on the outer `BlockAST` containing one entry: `{name: "deprecated", arguments: ["Use ExtendedUserData instead"]}`.

---

## `BlockLine` — Structured lines inside a DSL block

Inside a DSL block, some lines are not full Clean statements — they are structured entries the DSL interprets. Each such line is exposed as a `BlockLine`.

| Field | Type | Purpose |
|---|---|---|
| `tokens` | `list<Token>` | The line's tokens after lexing. |
| `span` | `Span` | Source location. |

The DSL handler decides how to interpret tokens. The compiler does not attempt to parse a `BlockLine` as an expression or statement.

**Example (source-side, inside `fields:`):**

```
integer id primary
string email required unique
string passwordHash as "password_hash"
```

Each line becomes one `BlockLine` with a `tokens` list ready for the DSL handler.

---

## `Token` — Sum type over lexed tokens

A `Token` is one of the following variants:

| Variant | Fields | Meaning |
|---|---|---|
| `Identifier` | `text: string` | An unquoted name. |
| `Keyword` | `text: string` | A reserved word (`primary`, `unique`, `required`, `as`, etc. — the DSL decides which names are meaningful to it). |
| `String` | `text: string` | A `"..."` literal. |
| `Integer` | `value: integer` | A parsed integer literal. |
| `Number` | `value: number` | A parsed number literal. |
| `Symbol` | `text: string` | A punctuation token (`.`, `,`, `->`, etc.). |

The distinction between `Identifier` and `Keyword` is made by the tokenizer against the language's global keyword set ([03 — Lexical Structure](../03-lexical-structure.md)). Names that are language keywords come through as `Keyword`; everything else comes through as `Identifier`. Handlers that want their own DSL keywords (like `primary`) treat matching `Identifier` tokens as keywords by convention.

---

## `Span` — Source location

| Field | Type | Purpose |
|---|---|---|
| `file` | `string` | Source file path. |
| `line` | `integer` | 1-based line number. |
| `column` | `integer` | 1-based column number. |
| `byteRange` | `pairs<integer, integer>` | Start and end byte offsets in the source file. |

---

## `IR` — The typed return value

`IR` is a typed fragment of the compiler's intermediate representation. It is **opaque** to the handler: there is no field access and no method call on an `IR` value. Handlers compose IR only through the `ir` builder namespace functions listed below.

Every builder function is a pure factory: it produces a new `IR` value without side effects.

### The `ir` builder namespace (surface)

```clean
namespace ir
    // ─── Declarations ────────────────────────────────────
    IR class(string name, list<Field> fields, list<Method> methods)
    IR function(string name, list<Parameter> params, TypeRef return_, IR body)
    IR field(string name, TypeRef type)
    IR method(string name, list<Parameter> params, TypeRef return_, IR body)

    // ─── Statements ──────────────────────────────────────
    IR return_(IR expression)
    IR assign(string name, IR expression)
    IR if_(IR condition, IR then_, IR else_)
    IR block(list<IR> statements)

    // ─── Expressions ─────────────────────────────────────
    IR call(string qualified_name, list<IR> arguments)
    IR literal_integer(integer value)
    IR literal_string(string value)
    IR variable(string name)
    IR field_access(IR receiver, string field)

    // ─── Composition ─────────────────────────────────────
    IR concat(list<IR> fragments)          // splice multiple IRs sequentially
    IR empty()                             // no-op, valid IR fragment
    IR withSpan(IR node, Span span)        // re-anchor a node's source span
```

### IR construction rules

- Returning `ir.empty()` is valid and means "this block contributes nothing to the program" — useful for pure-annotation blocks.
- The compiler validates that the returned IR is well-typed against the surrounding compilation unit. IR that references an undefined symbol produces a diagnostic at the corresponding user span, not at "generated code line N."
- Every IR node inherits a span. By default, that span points at the block header the handler is processing. Handlers should override the span via `ir.withSpan(node, span)` when a subtree corresponds to a specific sub-region of the user's source, so diagnostics later in compilation point at the correct line.

---

## Supporting types referenced by `ir` builders

The following are ordinary Clean types available inside a `compiletime` context:

- `Field` — a record `{name: string, type: TypeRef}`.
- `Parameter` — a record `{name: string, type: TypeRef, default: option<IR>}`.
- `Method` — a record `{name: string, params: list<Parameter>, return_: TypeRef, body: IR}`.
- `TypeRef` — a namespace of type-name constructors: `TypeRef.integer()`, `TypeRef.string()`, `TypeRef.list(T)`, `TypeRef.class(name)`, `TypeRef.optional(T)`, etc.

The concrete shape of `TypeRef` is beyond the surface of this schema — it is a black-box handle the `ir` builders consume.

---

## `Diagnostic` — Handler-emitted diagnostic

Emitted via `error(...)`, `warning(...)`, `info(...)` (see [BLK-03](../21-block-handlers.md#blk-03--handler-diagnostics)). Handlers do not construct `Diagnostic` values directly; the framework builds them from the emission call.

| Field | Type | Purpose |
|---|---|---|
| `severity` | `variant { error, warning, info }` | Severity level. |
| `code` | `string` | Kebab-case sub-label supplied by the library (e.g. `"field-missing-type"`). Not a diagnostic code — surfaces as `LIB010` with this sub-label. |
| `message` | `string` | Human-readable prose. |
| `span` | `Span` | Source location. Must be a real span from the input `BlockAST` or one of its children. |

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Library authors writing block handlers; compiler implementers of the compile-time execution environment; anyone reading or writing `compiletime function` bodies
- **Notation:** Prose + Clean type-declaration syntax
- **Part of:** [04 language](../README.md)
- **Source of truth for:** BlockAST, BlockNode, BlockArg, BlockAttribute, BlockLine, Token, Span, IR builder API, Diagnostic (compile-time value types)
- **Rules referencing this schema:** [21 — Block Handlers](../21-block-handlers.md) (BLK-01, BLK-03), [04 — Type System](../04-type-system.md) (TYP-04)
- **References:** [DOC-18](../../01%20governance/00-documentation-principles.md#doc-18--structured-data-artifacts-have-a-single-canonical-schema-file), [grammar/21-block-handlers.ebnf.md](../grammar/21-block-handlers.ebnf.md)
