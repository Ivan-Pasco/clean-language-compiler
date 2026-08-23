# 21 block-handlers — Grammar

Companion grammar file for [21 — Block Handlers](../21-block-handlers.md). Defines the shape of `compiletime function` declarations, `handles block` registrations, and the compile-time-only type constructors (`BlockAST`, `BlockNode`, `BlockArg`, `BlockAttribute`, `BlockLine`, `Token`, `IR`, `Span`, `Diagnostic`). Semantic rules BLK-01..BLK-04 live in the companion chapter — declaration form, block-name resolution, handler diagnostics, and the compile-time execution environment.

**Historical note.** The productions in §1 below already existed in the source chapter (`21-block-handlers.md §21.1`). Before this program, they were the *only* grammar productions in the entire `04 language/` folder. Extracting them here satisfies DOC-15's companion-file requirement and unblocks the chapter's Stage 3 split.

The Schema-tier definitions (BlockAST fields, BlockNode variants, IR builder API) live in [`04 language/schema/block-ast.md`](../schema/block-ast.md) per DOC-18; this Grammar file references them by name.

---

## 1. `compiletime function` declaration and `handles block` registration

These are the productions the original `21-block-handlers.md §21.1` shipped inline. Extracted here per DOC-15.

```ebnf
(* BLK-01: a block handler is a `compiletime function` paired with
   a `handles block` registration.  Both live in library source. *)

CompileTimeFunctionDeclaration
                = "compiletime", "function", Identifier,
                  "(", ParameterList, ")",
                  "returns", TypeName,
                  NEWLINE, INDENT, CompileTimeFunctionBody, DEDENT ;

(* Parameter constraint (BLK-01): exactly one parameter of type
   BlockAST.  ParameterList grammar admits multiple parameters;
   the checker enforces the arity-and-type restriction. *)

(* Return-type constraint (BLK-01): must be IR.  The `returns IR`
   clause is not optional.  Grammar enforces the `returns TypeName`
   syntax; the checker verifies TypeName is `IR`. *)

CompileTimeFunctionBody = [ DescriptionClause ], StatementSequence ;

HandlesBlockDeclaration = "handles", "block", StringLiteral,
                          "with", Identifier, NEWLINE ;

(* BLK-01: the block name in the string literal must be a valid
   qualified identifier (`name` or `name.name.name` — no spaces,
   no punctuation other than `.`).  Checker enforces this
   restriction on the string literal's content. *)

(* BLK-01: a `handles block` declaration must reference a
   `compiletime function` defined in the same library.  Checker
   rule, not grammar. *)
```

## 1a. `LibraryBlock` — the block header (BLK-02)

The normative production for a library-registered block's header, ratified from the compiler's M3 parser. Its single home is this file per DOC-15; [`grammar/08-file-structure.ebnf.md §3`](./08-file-structure.ebnf.md) references it (same pattern as the 2026-08-20 `WatchBlock` deduplication).

```ebnf
(* BLK-02: the block name is a qualified identifier — the same
   string a library registers via `handles block`. *)

BlockName       = Identifier, { ".", Identifier } ;

(* Header arguments come in two surfaces that MAY be combined: an
   optional parenthesized, comma-separated list, followed by zero
   or more bare arguments juxtaposed by whitespace (no commas),
   terminated by the header's closing ":".  Both surfaces append
   to BlockAST.arguments in source order — `name(a) b:` is legal
   and yields [a, b]. *)

LibraryBlock    = BlockName,
                  [ "(", [ BlockHeaderArg, { ",", BlockHeaderArg } ], ")" ],
                  { BlockHeaderArg },
                  ":", NEWLINE, INDENT, ? handler-defined body ?, DEDENT ;

BlockHeaderArg  = Identifier, "=", Expression   (* BlockArg::Keyword *)
                | Expression ;                   (* BlockArg::Positional *)

(* LEX-04 disambiguation, stated per the header brief's acceptance
   check: a contextual keyword is specialised only when ":"
   immediately follows it, so a header carrying arguments is
   always a LibraryBlock.  A line is recognised as a block header
   only when ":" is its final token and an indented body follows —
   a library block with an empty body is not source syntax. *)
```

## 1b. `match` statement — variant discrimination (21 §21.3.3)

Decided 2026-08-22. `match` and `case` are hard keywords (LEX-04).

```ebnf
(* 21 §21.3.3: the variant-discrimination statement, scoped in this
   version to the compile-time sum types (BlockNode, BlockArg,
   Token).  Grammar admits it wherever a statement is admitted; the
   operand rule (SEM004 on a non-sum type) confines it in practice
   to compiletime bodies.  Exact coverage of the operand's variants
   is the checker's SEM031, not grammar. *)

MatchStatement  = "match", Expression, NEWLINE, INDENT,
                  CaseArm, { CaseArm },
                  DEDENT ;

CaseArm         = "case", VariantName, [ Identifier ], NEWLINE,
                  INDENT, StatementSequence, DEDENT ;

VariantName     = ? a variant name of the operand's compile-time
                    sum type — BlockNodeType (§2), BlockArgType
                    (§2), or a Token variant name; the payload the
                    optional Identifier binds is field-level in
                    ../schema/block-ast.md ? ;
```

## 2. `BlockAST`, `BlockNode`, `BlockArg` (compile-time value types)

Grammar-side sum-type definitions per the original `21 §21.3`. Field-level Schema lives in [`schema/block-ast.md`](../schema/block-ast.md).

```ebnf
(* BLK-01 §21.3: BlockNode is a sum type over the two kinds of
   child a BlockAST body may contain.  Exposed here as an
   algebraic type; the concrete field-level schema is in
   schema/block-ast.md.  The former `Statement` variant is
   deliberately absent — see the schema and the 2026-08-22
   changelog entry. *)

BlockNodeType   = "BlockAST"            (* a nested block *)
                | "BlockLine" ;         (* a structured DSL line *)

BlockArgType    = "Positional", ExpressionType
                | "Keyword", IdentifierType, ExpressionType ;

ExpressionType  = ? the Expression payload of a BlockArg — a
                    schema-tier compile-time type, field-level
                    definition in ../schema/block-ast.md §BlockArg ? ;

IdentifierType  = ? the Identifier payload of a BlockArg — a
                    schema-tier compile-time type, field-level
                    definition in ../schema/block-ast.md §BlockArg ? ;

(* ExpressionType / IdentifierType are special sequences, not
   language non-terminals: BlockArg payloads exist only inside the
   compile-time environment, so — like LibraryBlock's
   handler-defined body in 08-file-structure.ebnf.md — they are
   deliberately outside the language grammar and BlockArgType is
   not generatable as source syntax. *)

(* These are TYPE-level constructors — they exist during
   compilation, not in a Clean program's runtime.  The variant
   names are schema-tier discriminators (schema/block-ast.md);
   on the compile-time wire they surface as the node's "kind"
   discriminator field.  In Clean source they are discriminated
   by the MatchStatement of §1b (decided 2026-08-22): `is`
   remains the identity operator of 06-expressions (EXP-01
   level 8), and assigning a BlockNode to a variant-typed name
   is still not a defined coercion — `case` binding is the one
   downcast surface. *)
```

## 3. `error`, `warning`, `info` — diagnostic emission (BLK-03)

```ebnf
(* BLK-03: handler diagnostics are emitted via three top-level
   functions available inside compiletime bodies.  Grammatically
   they are ordinary function calls (Call in
   06-expressions.ebnf.md) whose callees are these three names,
   which are hard-context names inside a compiletime body. *)

DiagnosticEmission = ( "error" | "warning" | "info" ),
                     "(", StringLiteral, ",", StringLiteral, ",", Expression, ")" ;
                     (* Args: code (sub-label kebab-case),
                              message (human-readable prose),
                              span (real source span from input). *)

(* Note: `error` here is DISTINCT from the `error` keyword used
   for runtime failure signalling (ERH-01).  Inside a
   compiletime function body, `error(code, message, span)` — with
   three arguments — is the diagnostic emitter.  Outside,
   `error(message)` — with one string argument — is the runtime
   signal.  Grammar distinguishes them by argument count.
   ⚠ Whether the parser needs additional context to disambiguate
   is under-specified.  Encoded here as arity-based dispatch;
   a compiletime body's error() takes 3 args, a runtime body's
   error() takes 1.  Needs review. *)
```

## 4. `test.compiletime` namespace (BLK-04 §21.9)

Grammar-wise, `test.compiletime.parseBlock(...)`, `test.compiletime.collectDiagnostics(...)`, etc. are ordinary member-access + call expressions on the `test.compiletime` namespace. No new production required — subsumed by [06-expressions.ebnf.md](./06-expressions.ebnf.md)'s `PostfixExpression`.

The chapter's rule ([21 §21.9](../21-block-handlers.md#219-testing-block-handlers)) restricts these helpers to appearing inside `tests:` blocks only — a scope rule (SCOPE006), not grammar.

## 5. Reserved namespace: `ir` builder API

```ebnf
(* BLK-01 §21.4: `IR` is opaque; handlers compose it only through
   the `ir` builder-namespace functions.  These are ordinary calls
   in expression grammar — ir.class(...), ir.function(...), etc.
   The Schema-tier catalogue of every builder in the surface is
   in schema/block-ast.md.
   No new grammar production needed — subsumed by MemberAccess +
   Call in 06-expressions.ebnf.md. *)
```

---

## Changelog

- 2026-08-22 (escalation resolved) — New **§1b**: `MatchStatement`/`CaseArm` productions (21 §21.3.3); the §2 trailing comment updated — the variant names are now discriminated in source by `match`, and `case` binding is the one downcast surface. `match`/`case` reserved in LEX-04.
- 2026-08-22 — Three rulings from the compiler-milestone decision briefs. (a) New §1a: the `LibraryBlock` header production ratified from the compiler's M3 parser (`work/2026-08-17-library-block-header-grammar.md`, `DISCOVERIES-M3.md` item 4) — qualified `BlockName`, optional parenthesized list **plus** bare arguments (the two surfaces combine, which none of the brief's candidate shapes described), keyword form legal in both surfaces, LEX-04 disambiguation stated; `08-file-structure.ebnf.md §3` now references this production instead of defining its own. (b) §2's `BlockNodeType` reduced to two variants (`work/2026-08-17-block-ast-statement-classification.md`, `DISCOVERIES-M3.md` item 2 — the parse-time `Statement` variant was never producible and the wire ABI never carried it). (c) The ⚠ (a) marker of §2 resolved normatively as "no discrimination construct exists in this version" (`work/2026-08-18-block-pattern-match-syntax.md`, `DISCOVERIES-M5.md` item 16): the false "lives in 06-expressions.ebnf.md" claim removed; the construct choice itself remains an open decision. The ⚠ (b) of §3 (arity-based `error()` dispatch) is untouched.
- 2026-08-20 — Erratum from the compiler's Milestone 9 (`clean-language-compiler/docs/DISCOVERIES-M9.md` §1, item 1g): `BlockArgType` referenced `ExpressionType` and `IdentifierType`, which no grammar file defined. Both are now defined as ISO 14977 special sequences pointing at their field-level home ([`schema/block-ast.md` §BlockArg](../schema/block-ast.md)) — they are schema-tier compile-time payloads, not source syntax, so `BlockArgType` (and through it `CompileTimeFunctionDeclaration`'s pattern-match surface) is deliberately ungeneratable as language grammar, same status as `LibraryBlock`'s handler-defined body.
- 2026-08-07 — File minted. `CompileTimeFunctionDeclaration` and `HandlesBlockDeclaration` productions extracted from [21-block-handlers.md §21.1](../21-block-handlers.md#211-declaring-a-block-handler) (Accepted 2026-08-01) — the only pre-existing grammar productions in `04 language/`, per the survey that motivated the Docs Readiness Program. `BlockNodeType`, `BlockArgType`, `DiagnosticEmission` productions derived from BLK-01..BLK-03 in the source chapter. Two `⚠` markers: (a) whether Clean has first-class pattern-matching syntax for `BlockNode` variants; (b) how the parser disambiguates `error(...)` between the diagnostic emitter (3 args, compiletime scope) and the runtime signal (1 arg, ERH-01).

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Library authors writing block handlers; compiler implementers of the compile-time execution environment
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [21-block-handlers.md](../21-block-handlers.md) (BLK-01..BLK-04)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [04-type-system.ebnf.md](./04-type-system.ebnf.md), [06-expressions.ebnf.md](./06-expressions.ebnf.md), [07-statements.ebnf.md](./07-statements.ebnf.md) (StatementSequence), [09-functions.ebnf.md](./09-functions.ebnf.md) (ParameterList, DescriptionClause), [13-error-handling.ebnf.md](./13-error-handling.ebnf.md) (ErrorStatement — the runtime `error` form), forward reference to [`../schema/block-ast.md`](../schema/block-ast.md) for BlockAST field-level schema
