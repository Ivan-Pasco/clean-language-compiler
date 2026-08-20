# 18 async — Grammar

Companion grammar file for [18 — Asynchronous Programming](../18-async.md). Defines the syntactic surface of asynchronous execution: the `start` expression (background execution — distinct from the `start:` entry block), the `later` deferred binding, the `background` prefix on function calls, and the `background` modifier on function declarations. Semantic rules ASY-01..ASY-03 live in the companion chapter.

Per ASY-01, the `start` keyword has two distinct meanings in the language: `start:` marks the entry-point block (defined in [09-functions.ebnf.md](./09-functions.ebnf.md) as `StartBlock`), and `start <expression>` runs a function in the background. This file defines only the latter.

---

## 1. `start` expression (background)

```ebnf
(* ASY-01 §Start Expression: `start` prefixes a function call to
   begin it in the background.  It appears in exactly TWO
   positions per ASY-01 boundary rule:
     1. RHS of a `later T name = start f()` binding
     2. Following `background` in `background f()`
   A `start` in any other position is SYN002. *)

StartExpression = "start", CallExpression ;

CallExpression  = PrimaryExpression, { PostfixOp }, Call ;
                  (* A PostfixExpression whose LAST postfix
                     operation is a Call — i.e. any expression
                     whose top-level operation is a function or
                     method call: `f()`, `obj.method()`,
                     `a.b.c(x)[0].run()`.  PrimaryExpression,
                     PostfixOp and Call are defined in
                     06-expressions.ebnf.md. *)
```

## 2. `later` deferred binding

```ebnf
(* ASY-01: `later T name = start f()` declares a deferred binding
   of type T.  T is any TypeExpression.  The RHS MUST be a
   StartExpression per the boundary rule cited above.
   ⚠ ASY-01 permits multiple forms of RHS?  The chapter's
   §Semantics Summary says "reading a `later` binding blocks the
   current task until the underlying computation finishes."  Only
   the `start f()` RHS is shown in examples.  Encoded here as
   RHS = StartExpression only; if other RHS forms are legal, this
   production narrows.  Needs review. *)

LaterBinding    = "later", TypeExpression, Identifier, "=", StartExpression ;

(* LaterBinding is a distinct statement kind, NOT a modifier on
   VariableDeclaration.  Reasons:
     - Syntactically different (has the `later` prefix keyword)
     - Semantically different (creates a deferred binding, not an
       immediate value)
     - Different lowering (compiles to poll / ready / cancel at the
       WIT boundary)
   07-statements.ebnf.md's Statement production adds LaterBinding
   as its own alternative.  RHS is restricted to StartExpression
   per ASY-01 — every chapter example uses `start f()` on the RHS,
   and ASY-01 constrains `start` to exactly two positions. *)
```

## 3. `background` prefix expression

```ebnf
(* ASY-01: `background f()` runs f() and keeps no binding.
   Statement, not an expression.  Can be followed by an
   OnErrorBlock or OnErrorSuffix per ASY-03 for handling failures. *)

BackgroundStatement = "background", CallExpression, [ OnErrorTail ] ;

OnErrorTail     = "onError", ( Expression | ":", NEWLINE, INDENT, StatementSequence, DEDENT ) ;
                  (* Reuses OnErrorSuffix / OnErrorBlock shapes
                     from 13-error-handling.ebnf.md — same
                     handler-attachment pattern. *)
```

## 4. `background` function modifier

```ebnf
(* ASY-02: `void syncCache() background` marks a function
   declaration so every call to it runs in the background
   automatically.  The `background` modifier is postfix-only —
   it comes AFTER the parameter list, BEFORE the body's NEWLINE.
   Every chapter example uses this form; allowing multiple
   placements would give library authors two ways to write the
   same thing, which contradicts LDR-08 "one way to do things". *)

(* This is a modifier on FunctionDeclaration (in
   09-functions.ebnf.md).  Grammatically, the modifier extends
   that production:

     FunctionDeclaration = ReturnType, Identifier,
                           "(", [ ParameterList ], ")",
                           [ "background" ],
                           NEWLINE, INDENT, FunctionBody, DEDENT ;

   Rather than restate FunctionDeclaration here, this file
   declares the modifier as a suffix keyword that 09-functions
   should incorporate. *)

BackgroundModifier = "background" ;
                     (* Appears once, immediately after the
                        function signature's closing ")", before
                        the NEWLINE that opens the body. *)
```

## 5. Cancellation

```ebnf
(* ASY-03: `name.cancel()` requests a deferred binding be
   cancelled.  No new production — `.cancel()` is an ordinary
   Call postfix on the identifier bound by a LaterBinding.
   Semantics (RUN019 if read after cancel) are ASY-03's rules,
   not grammar. *)
```

---

## Changelog

- 2026-08-20 — Two errata from the compiler's Milestone 9 (`clean-language-compiler/docs/DISCOVERIES-M9.md` §1, items 1a and 1e). (a) `OnErrorTail`'s block alternative concatenated `":" NEWLINE INDENT StatementSequence DEDENT` by juxtaposition, without the `,` the [grammar README](./README.md#notation) requires for ISO/IEC 14977 concatenation — commas added; the intended parse is unchanged. (b) `CallExpression` was referenced by `StartExpression` and `BackgroundStatement` but defined only in comment prose — now a real production, `PrimaryExpression, { PostfixOp }, Call` (a postfix chain whose last operation is a `Call`), matching the prose "any expression whose top-level operation is a call". Note this is deliberately wider than a bare `Identifier`-callee call: method calls (`obj.fetch()`) are legal `start` / `background` targets.
- 2026-08-07 (afternoon, third pass) — Resolved the third `⚠` marker: `background` modifier stays postfix-only after the parameter list. Allowing multiple placements would give library authors two ways to write the same thing, contradicting LDR-08 "one way to do things". No production change.
- 2026-08-07 (afternoon) — Resolved the first two `⚠` markers: (a) `LaterBinding` RHS is restricted to `StartExpression` — every chapter example uses `start f()`, and allowing plain expressions would raise questions about when the value becomes ready that the chapter doesn't answer; (b) `LaterBinding` stays as a distinct Statement kind (not a modifier on `VariableDeclaration`) — it is syntactically, semantically, and lowering-wise different from an immediate declaration.
- 2026-08-07 — File minted. Productions derived from ASY-01..ASY-03 in [18-async.md](../18-async.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; anyone reasoning about async syntax and failure attachment
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [18-async.md](../18-async.md) (ASY-01..ASY-03)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [04-type-system.ebnf.md](./04-type-system.ebnf.md), [06-expressions.ebnf.md](./06-expressions.ebnf.md), [07-statements.ebnf.md](./07-statements.ebnf.md), [09-functions.ebnf.md](./09-functions.ebnf.md) (FunctionDeclaration), [13-error-handling.ebnf.md](./13-error-handling.ebnf.md) (OnErrorSuffix, OnErrorBlock)
