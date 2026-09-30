# 21 block-handlers — Grammar

**Title:** 21 block-handlers — Grammar

**Id:** `BLKG-`

**Purpose:** Companion grammar file for 21 — Block Handlers. Defines the shape of a `compiletime function` declaration and its `handles block` registration, the `library_block` header a registered block is written with — its qualified name and its two argument surfaces — for which this file is the single home, the `match`/`case` statement that discriminates the compile-time sum types, the grammar-side sum types `BlockNodeType` and `BlockArgType`, and the call shape of the `error`, `warning` and `info` diagnostic emitters, including how the three-argument `error` is told apart from the one-argument runtime signal; it also records that the `test.compileTime` helpers and the `ir` builder API need no production of their own, being ordinary member access and call. Semantic rules BLK-01..BLK-04 — declaration form, block-name resolution, handler diagnostics and the compile-time execution environment — live in the companion chapter, and the field-level definitions of `BlockAST`, its nodes and the `IR` builders live in `../block-ast.schema.md` per DOC-18; this file references them by name and repeats none. It is written for compiler implementors building the parser and for library authors who need the exact form a handler takes; it rests on 21 — Block Handlers, on `06-expressions.lark.md` for the call and member-access productions it reuses, and on 03 — Lexical Structure (LEX-04) for the keywords it introduces.

---

### BLKG-01 — 1. `compiletime function` declaration and `handles block` registration

**Id:** BLKG-01

**Kind:** rule

**Name:** 1. `compiletime function` declaration and `handles block` registration

**Question:** In what written form is a `compiletime function` declared and bound to a block name by `handles block`, and which of its constraints — one `BlockAST` parameter, `returns IR`, a qualified block name, a same-library handler — does the grammar itself carry rather than the checker?

**Parent:** LANG-19

**Norm:**

These are the productions the original `21-block-handlers.md §21.1` shipped inline. Extracted here per DOC-15.

```lark
// BLK-01: a block handler is a `compiletime function` paired with a
// `handles block` registration.  Both live in library source.

compile_time_function_declaration: _COMPILETIME _FUNCTION IDENTIFIER "(" parameter_list ")" _RETURNS return_type _NEWLINE _INDENT compile_time_function_body _DEDENT

// Parameter constraint (BLK-01): exactly one parameter of type
// BlockAST.  parameter_list admits multiple parameters; the checker
// enforces the arity-and-type restriction.
//
// Return-type constraint (BLK-01): must be IR.  The `returns IR` clause
// is not optional.  The grammar enforces the `returns` type syntax —
// any return_type (04-type-system.lark.md); the checker verifies the
// type is `IR`.

compile_time_function_body: description_clause? statement*

handles_block_declaration: _HANDLES _BLOCK string_literal _WITH IDENTIFIER _NEWLINE

// BLK-01: the block name in the string literal must be a valid
// qualified identifier (`name` or `name.name.name` — no spaces, no
// punctuation other than `.`).  Checker enforces this restriction on
// the string literal's content.
//
// BLK-01: a `handles block` declaration must reference a `compiletime
// function` defined in the same library.  Checker rule, not grammar.
```

**Why:** these productions shipped inline in the chapter and are extracted here so the grammar has one home; the constraints that ride with them — one `BlockAST` parameter, `returns IR`, a qualified block name, a same-library handler — are checker rules, so the grammar admits the general shape.

---

### BLKG-02 — 1a. `LibraryBlock` — the block header (BLK-02)

**Id:** BLKG-02

**Kind:** rule

**Name:** 1a. `LibraryBlock` — the block header (BLK-02)

**Question:** What written form does the header of a library-registered block take — its qualified block name, its parenthesized and its bare argument surfaces, their order into `BlockAST.arguments`, and what makes a line a block header at all?

**Parent:** PRD-08

**Norm:**

The normative production for a library-registered block's header, ratified from the compiler's M3 parser. Its single home is this file per DOC-15; `08-file-structure.lark.md §3` references it (same pattern as the 2026-08-20 `watch_block` deduplication).

```lark
// BLK-02: the block name is a qualified identifier — the same string a
// library registers via `handles block`.

block_name: IDENTIFIER ("." IDENTIFIER)*

// Header arguments come in two surfaces that MAY be combined: an
// optional parenthesized, comma-separated list, followed by zero or
// more bare arguments juxtaposed by whitespace (no commas), terminated
// by the header's closing ":".  Both surfaces append to
// BlockAST.arguments in source order — `name(a) b:` is legal and yields
// [a, b].

library_block: block_name block_argument_list? block_bare_argument* ":" _NEWLINE _INDENT block_body _DEDENT

block_argument_list: "(" (block_header_arg ("," block_header_arg)*)? ")"

// The body is read by the core grammar alone, into the generic child
// nodes of BlockAST.body (block-ast.schema.md): lines of text, a line
// opening a nested body of lines when the next one is indented one tab
// deeper (LEX-01).  A line whose last character is ":" and which opens
// a nested body is a nested BlockAST; every other line is a BlockLine,
// which the handler tokenises itself.  The parser never reads a body
// line as a Clean statement and never tokenises it: the body is
// handler-defined — HTML, a query, a scene — and the library's own
// grammar companion states the syntax its handler accepts.

block_body: block_line+

// A block may also stand as the value of a typed declaration or of a
// `return`: the
// initializer — a library call such as `validator.create` — is followed
// by ":" and an indented body, and the handler of that name builds the
// value from the body (VAL-08).  Which initializers may take a body is
// the checker's; the grammar reads the shape.
block_valued_declaration: type_expression IDENTIFIER "=" expression ":" _NEWLINE _INDENT block_body _DEDENT
block_valued_return: _RETURN expression ":" _NEWLINE _INDENT block_body _DEDENT
block_line: BLOCK_TEXT _NEWLINE (_INDENT block_body _DEDENT)?
BLOCK_TEXT.10: /[^\n]+/

block_header_arg: IDENTIFIER "=" expression  -> keyword_argument     // BlockArg::Keyword
                | expression                 -> positional_argument  // BlockArg::Positional

// A bare argument is juxtaposed to the next one with no separator, so
// it cannot be an arbitrary expression: `name a -b:` would read as one
// argument or two, and `name a [1]:` as an index or a list.  A bare
// argument is therefore a literal or a qualified name, bare or after
// `key =`; any other expression is written in the parenthesized list.

block_bare_argument: IDENTIFIER "=" block_bare_value  -> keyword_argument
                   | block_bare_value                 -> positional_argument

block_bare_value: literal
                | IDENTIFIER ("." IDENTIFIER)*

// LEX-04 disambiguation, stated per the header brief's acceptance
// check: a contextual keyword is specialised only when ":" immediately
// follows it, so a header carrying arguments is always a
// library_block.  A line is recognised as a block header only when ":"
// is its final token and an indented body follows — a library block
// with an empty body is not source syntax.  The core section headers
// (`functions:`, `tests:`, `start:` ...) and `watch` keep their
// meaning at the top level: a library cannot register a block under
// those names.
```

**Why:** the header has two argument surfaces that may combine, so the production spells out both and their order into `BlockAST.arguments`, and it has a single home here because 08-file-structure.lark.md references it rather than repeating it — the pattern the `WatchBlock` deduplication set.

---

### BLKG-03 — 1b. `match` statement — variant discrimination (21 §21.3.3)

**Id:** BLKG-03

**Kind:** rule

**Name:** 1b. `match` statement — variant discrimination (21 §21.3.3)

**Question:** What written form does the `match`/`case` statement that discriminates the compile-time sum types take, and are the operand's type, the statement's placement and the exhaustiveness of its arms fixed by the grammar or left to the checker?

**Parent:** LANG-19

**Norm:**

Decided 2026-08-22. `match` and `case` are hard keywords (LEX-04).

```lark
// 21 §21.3.3: the variant-discrimination statement, scoped in this
// version to the compile-time sum types (BlockNode, BlockArg, Token).
// The grammar admits it wherever a statement is admitted; the operand
// rule (SEM004 on a non-sum type) confines it in practice to
// compiletime bodies.  Exact coverage of the operand's variants is the
// checker's SEM031, not grammar.

match_statement: _MATCH expression _NEWLINE _INDENT case_arm+ _DEDENT

case_arm: _CASE variant_name IDENTIFIER? block

// A variant name of the operand's compile-time sum type —
// BlockNodeType (BLKG-04), BlockArgType (BLKG-04), or a Token variant
// name; the payload the optional IDENTIFIER binds is field-level in
// ../block-ast.schema.md.  Every variant name is an identifier; which
// names the operand's type has is the checker's.

variant_name: IDENTIFIER
```

**Why:** the compile-time sum types need one discrimination surface, and `case` binding is that surface; the grammar admits `match` wherever a statement stands and leaves confinement to compiletime bodies (SEM004) and coverage of the variants (SEM031) to the checker.

---

### BLKG-04 — 2. `BlockAST`, `BlockNode`, `BlockArg` (compile-time value types)

**Id:** BLKG-04

**Kind:** rule

**Name:** 2. `BlockAST`, `BlockNode`, `BlockArg` (compile-time value types)

**Question:** Which variants do the grammar-side sum types `BlockNodeType` and `BlockArgType` carry, and are their payloads language non-terminals a program could write as source syntax or special sequences of the compile-time environment?

**Parent:** LANG-19

**Norm:**

Grammar-side sum-type definitions per the original `21 §21.3`. Field-level Schema lives in `block-ast.schema.md`.

```lark
// BLK-01 §21.3: BlockNode is a sum type over the two kinds of child a
// BlockAST body may contain.  Exposed here as an algebraic type; the
// concrete field-level schema is in block-ast.schema.md.  The former
// `Statement` variant is deliberately absent — see the schema (removal
// ratified 2026-08-22; history in git).
//
//     BlockNodeType:  "BlockAST"     (a nested block)
//                   | "BlockLine"    (a structured DSL line)
//
//     BlockArgType:   "Positional" ExpressionType
//                   | "Keyword" IdentifierType ExpressionType
//
// ExpressionType is the expression payload of a BlockArg and
// IdentifierType its identifier payload — schema-tier compile-time
// types, field-level definition in ../block-ast.schema.md §BlockArg.
//
// These are written as comments, not as rules: BlockArg payloads exist
// only inside the compile-time environment, so — like the body a
// library block's handler interprets (BLKG-02) — they are deliberately
// outside the language grammar, and neither type is generatable as
// source syntax.  A rule no source text can reach would only be an
// unused rule of the grammar.
//
// These are TYPE-level constructors — they exist during compilation,
// not in a Clean program's runtime.  The variant names are schema-tier
// discriminators (block-ast.schema.md); on the compile-time wire they
// surface as the node's "kind" discriminator field.  In Clean source
// they are discriminated by the match_statement of BLKG-03 (decided
// 2026-08-22): `is` remains the identity operator of 06-expressions
// (EXP-01 level 8), and assigning a BlockNode to a variant-typed name
// is still not a defined coercion — `case` binding is the one downcast
// surface.
```

**Why:** the variant names are schema-tier discriminators that exist only during compilation, so the payloads are special sequences rather than language non-terminals and `BlockArgType` is deliberately not generatable as source syntax; the field-level schema stays in block-ast.schema.md.

---

### BLKG-05 — 3. `error`, `warning`, `info` — diagnostic emission (BLK-03)

**Id:** BLKG-05

**Kind:** rule

**Name:** 3. `error`, `warning`, `info` — diagnostic emission (BLK-03)

**Question:** What call shape do the `error`, `warning` and `info` handler diagnostics take, and what tells the three-argument compile-time `error` apart from the one-argument runtime failure signal?

**Parent:** LANG-19

**Norm:**

```lark
// BLK-03: handler diagnostics are emitted via three top-level functions
// available inside compiletime bodies.  Grammatically they are calls
// whose callees are these three names, which are hard-context names
// inside a compiletime body.  Args: code (sub-label kebab-case),
// message (human-readable prose), span (real source span from input).

diagnostic_emission: _ERROR "(" diagnostic_arguments ")"    -> error_emission
                   | _WARNING "(" diagnostic_arguments ")"  -> warning_emission
                   | _INFO "(" diagnostic_arguments ")"     -> info_emission

diagnostic_arguments: string_literal "," string_literal "," expression

// `warning` and `info` are not reserved words: each is the emitter
// only where "(" follows it, and an identifier everywhere else.
_WARNING.1: /warning(?=[ \t]*\()/
_INFO.1: /info(?=[ \t]*\()/

// Note: `error` here is DISTINCT from the `error` keyword used for
// runtime failure signalling (ERH-01).  Inside a compiletime function
// body, `error(code, message, span)` — with three arguments, the first
// two string literals — is the diagnostic emitter.  Outside,
// `error(message)` — with one argument — is the runtime signal.  The
// grammar distinguishes them by their arguments: after `error (` a
// string literal followed by "," can only begin diagnostic_arguments,
// and anything else can only be the one expression of the
// error_statement (13-error-handling.lark.md), so the parser needs no
// context beyond the tokens.  That the three-argument form stands only
// in a compiletime body is the checker's.
```

**Why:** the three emitters are ordinary calls, so the production exists to fix their argument shape — code, message, span — and to tell the three-argument `error` apart from the one-argument runtime signal by arity, which the tokens alone decide: a string literal followed by "," after `error (` can only begin the three arguments.

---

### BLKG-06 — 4. `test.compileTime` namespace (BLK-04 §21.9)

**Id:** BLKG-06

**Kind:** rule

**Name:** 4. `test.compileTime` namespace (BLK-04 §21.9)

**Question:** Do the `test.compileTime` helpers need a production of their own, and is their confinement to `tests:` blocks a matter of grammar or of scope rules?

**Parent:** LANG-17

**Norm:**

Grammar-wise, `test.compileTime.parseBlock(...)`, `test.compileTime.collectDiagnostics(...)`, etc. are ordinary member-access + call expressions on the `test.compileTime` namespace. No new production required — subsumed by 06-expressions.lark.md's `postfix_expression`.

The chapter's rule (21 §21.9) restricts these helpers to appearing inside `tests:` blocks only — a scope rule (SCOPE006), not grammar.

**Why:** the helpers are member-access-and-call expressions the expression grammar already covers, so the section records that no production exists and that their confinement to `tests:` blocks is a scope rule (SCOPE006), not grammar.

---

### BLKG-07 — 5. Reserved namespace: `ir` builder API

**Id:** BLKG-07

**Kind:** rule

**Name:** 5. Reserved namespace: `ir` builder API

**Question:** Do the `ir` builder-namespace calls a handler composes `IR` with need productions of their own, and where is the catalogue of those builders kept?

**Parent:** LANG-19

**Norm:**

```lark
// BLK-01 §21.4: `IR` is opaque; handlers compose it only through the
// `ir` builder-namespace functions.  These are ordinary calls in the
// expression grammar — ir.field(...), ir.withSpan(...), etc.  The
// Schema-tier catalogue of every builder in the surface is in
// block-ast.schema.md.  No new grammar rule needed — subsumed by
// member_access and call in 06-expressions.lark.md.
```

**Why:** `IR` is opaque and composed only through the `ir` builder calls, which are ordinary member access and call, so the section reserves the namespace and points to the schema catalogue instead of inventing productions the expression grammar makes redundant.

---
