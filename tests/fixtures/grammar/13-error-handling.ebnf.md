# 13 error-handling — Grammar

Companion grammar file for [13 — Error Handling](../13-error-handling.md). Defines the shape of the failure path: raising with `error(...)`, catching with `onError` in both suffix and block forms, and the `Error` value bound in a handler. Semantic rules ERH-01..ERH-05 live in the companion chapter (raising, catching, runtime failures, the `Error` value, unhandled failures).

The `onError` operator sits at level 13 (loosest) of the [precedence ladder in 06-expressions.ebnf.md](./06-expressions.ebnf.md). Its concrete grammar — suffix vs. block form — lives here because it is chapter 13's home. `06-expressions.ebnf.md` references `OnErrorExpression` at the top of the ladder; this file supplies the full form.

---

## 1. Raising an error

```ebnf
(* ERH-01: error(message) — signal a failure with a human-readable
   message.  Grammatically a Call whose callee is the hard keyword
   `error` and whose single argument is a string expression.
   Because `error` is a hard keyword (LEX-04), it is not an
   Identifier — the parser matches "error" "(" ... ")" as its own
   production and does NOT resolve `error` as a name.
   ERH-01 also says error(...) MUST NOT appear in value position
   (SEM004).  Grammar admits it anywhere a Statement is admitted;
   the checker enforces the value-position restriction. *)

ErrorStatement  = "error", "(", Expression, ")" ;

(* ErrorStatement participates in Statement alternation.
   07-statements.ebnf.md's Statement production adds this form. *)
```

## 2. Handling an error — the two `onError` forms

```ebnf
(* ERH-02: onError has a suffix and a block form.  Both bind the
   failure to the identifier `error` (an Error value per ERH-04)
   in scope within the fallback expression / block body. *)

(* The precedence-ladder form (from 06-expressions.ebnf.md):
     OnErrorExpression = DefaultExpression, { "onError", DefaultExpression } ;
   That definition covers only the SUFFIX form.  The BLOCK form is
   defined here because it terminates a Statement, not an
   Expression. *)

(* Suffix form — participates in the expression grammar, no new
   production needed here; documented for completeness. *)

OnErrorSuffix   = Expression, "onError", Expression ;
                  (* Where the LEFT Expression is the potentially-
                     failing expression and the RIGHT is the fallback.
                     Actual production is folded into the precedence
                     ladder in 06-expressions.ebnf.md. *)

(* Block form — a Statement, not an Expression.  Used when the
   handler needs more than one line.  Grammar:
     <expression> onError:
         <handler body>
   The handler body is an indented sequence of statements; the
   `error` identifier is bound inside. *)

OnErrorBlock    = Expression, "onError", ":", NEWLINE,
                  INDENT, StatementSequence, DEDENT ;
                  (* OnErrorBlock appears in two positions:
                     1. As a Statement — a bare `expr onError: body`
                        line whose value is discarded.
                     2. As an Expression-terminating form on the RHS
                        of an Assignment or VariableDeclaration —
                        `x = expr onError: body`.  The block
                        terminates at DEDENT, and the whole thing
                        is the value assigned.
                     The chapter shows position (2) as canonical
                     usage (`string content = file.read(path)
                     onError: ...`), so both positions are admitted. *)
```

## 3. The `Error` value

```ebnf
(* ERH-04: inside a handler, `error` is a value of the built-in
   type Error, a record with .message (string) and .code (string?).
   Grammatically Error is a Type name (ClassType in 04-type-system.
   ebnf.md), reachable in type position; its fields are accessed
   via ordinary MemberAccess.  No new production needed here — the
   value-side grammar is subsumed by expressions.

   `error` (lowercase) is used in two roles distinguished by the
   parser based on the following token:
     - `error(`  → the raise keyword; the entire form is an
                   ErrorStatement or DiagnosticEmission
     - `error.`  → the bound identifier's member access
     - `error`   → the bound identifier (in operand position, alone)
   The parser dispatches on the following token; the grammar admits
   both roles without ambiguity because the raise form REQUIRES `(`
   to follow immediately.  Grammar-only disambiguation would need
   context-sensitive productions, which is heavier. *)
```

---

## Changelog

- 2026-08-07 (afternoon) — Resolved both `⚠` markers: (a) `OnErrorBlock` stays as both a Statement AND an Expression-terminating form on the RHS of an assignment — the chapter's canonical example (`string content = file.read(path) onError: ...`) uses the assignment-RHS form, so rejecting it would invalidate a documented pattern; (b) `error` keyword vs. bound identifier disambiguation stays as a parser rule based on the following token (`error(` → raise, `error.` → binding member access, `error` → binding identifier). No production change.
- 2026-08-07 — File minted. Productions derived from ERH-01..ERH-05 in [13-error-handling.md](../13-error-handling.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; anyone writing failure-handling code
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [13-error-handling.md](../13-error-handling.md) (ERH-01..ERH-05)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [06-expressions.ebnf.md](./06-expressions.ebnf.md), [07-statements.ebnf.md](./07-statements.ebnf.md)
