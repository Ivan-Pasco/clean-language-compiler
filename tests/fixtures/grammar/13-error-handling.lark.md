# 13 error-handling — Grammar

**Title:** 13 error-handling — Grammar

**Id:** `ERHG-`

**Purpose:** This is the companion grammar of 13 — Error Handling: the productions for the failure path, with the semantic rules left in that chapter. It defines raising with `error(...)` — a statement of its own, since `error` is a hard keyword and not a name — catching with `onError` in its suffix form, which the expression grammar already places at the loosest level of the precedence ladder, and in its block form, which terminates a statement and so is defined here, and the `Error` value a handler binds, including how the parser tells the raise keyword from the bound identifier by the token that follows. It is written for implementers of the parser and for anyone checking a raise or a handler's written shape against the law; it rests on 13 — Error Handling and on the expression grammar.

---

### ERHG-01 — 1. Raising an error

**Id:** ERHG-01

**Kind:** rule

**Name:** 1. Raising an error

**Question:** What production form does raising a failure take — `error` matched as a hard keyword in a statement production of its own, or a Call whose callee `error` the parser resolves as an identifier — and does the grammar itself bar that form from value position?

**Parent:** LANG-16

**Norm:**

```lark
// ERH-01: error(message) — signal a failure with a human-readable
// message.  A call whose callee is the hard keyword `error` and whose
// single argument is a string expression.  Because `error` is a hard
// keyword (LEX-04), it is not an IDENTIFIER — the parser matches
// `error` "(" ... ")" as its own rule and does NOT resolve `error` as a
// name.  The rule is a statement: it takes part in the statement
// alternation of 07-statements.lark.md and in no expression, so
// error(...) in value position is not admitted by the grammar.

error_statement: _ERROR "(" expression ")"
```

**Why:** `error` is a hard keyword, not an Identifier, so the raise form has to be its own production rather than a Call the parser would resolve by name; the ban on value position (SEM004) is the checker's, so the grammar admits the statement wherever a statement is admitted.

---

### ERHG-02 — 2. Handling an error — the two `onError` forms

**Id:** ERHG-02

**Kind:** rule

**Name:** 2. Handling an error — the two `onError` forms

**Question:** What production gives the block form of `onError` its shape, and in which positions — a bare statement line and the right-hand side of a declaration or assignment — is that block admitted?

**Parent:** LANG-16

**Norm:**

```lark
// ERH-02: onError has a suffix and a block form.  Both bind the
// failure to the identifier `error` (an Error value per ERH-04) in
// scope within the fallback expression / block body.
//
// The suffix form is the precedence-ladder rule of
// 06-expressions.lark.md:
//     on_error_expression: default_expression
//                        | on_error_expression _ONERROR default_expression
// — the potentially-failing expression on the left of `onError`, the
// fallback on the right.  It needs no rule here.
//
// Block form — a statement, not an expression.  Used when the handler
// needs more than one line:
//     <expression> onError:
//         <handler body>
// The handler body is an indented sequence of statements; the `error`
// identifier is bound inside.  The expression before `onError` is an
// on_error_expression — the whole of an expression, since expression
// is on_error_expression — so that the parser decides between the
// suffix and the block form on the token after `onError`.

on_error_block: on_error_expression _ONERROR ":" block

// on_error_block appears in two positions:
//   1. As a statement — a bare `expr onError: body` line whose value
//      is discarded.
//   2. As an expression-terminating form on the right-hand side of an
//      assignment or a variable declaration — `x = expr onError: body`.
//      The block terminates at _DEDENT, and the whole thing is the
//      value assigned.
// The chapter shows position (2) as canonical usage (`string content =
// file.read(path) onError: ...`), so both positions are admitted.  All
// three are compound statements (07-statements.lark.md).

handled_declaration: type_expression IDENTIFIER "=" on_error_block

handled_assignment: assignment_target "=" on_error_block
```

**Why:** the suffix form already lives in the precedence ladder of 06-expressions.lark.md, but the block form terminates a statement, not an expression, so its production stands here with both positions named — bare line and right-hand side of a binding — because the chapter shows the second as canonical.

---

### ERHG-03 — 3. The `Error` value

**Id:** ERHG-03

**Kind:** rule

**Name:** 3. The `Error` value

**Question:** What tells the parser whether a lowercase `error` token is the raise keyword or the handler-bound identifier, and does the `Error` value's type name and field access need productions of their own in this grammar?

**Parent:** LANG-16

**Norm:**

```lark
// ERH-04: inside a handler, `error` is a value of the built-in type
// Error, a record with .message (string) and .code (string?).
// Grammatically Error is a type name (class_type in
// 04-type-system.lark.md), reachable in type position; its fields are
// accessed via ordinary member_access.
//
// `error` (lowercase) is used in two roles distinguished by the token
// that follows it:
//   - `error(`  -> the raise keyword; the entire form is an
//                  error_statement or a diagnostic_emission
//   - `error.`  -> the bound identifier's member access
//   - `error`   -> the bound identifier (in operand position, alone)
// The raise form REQUIRES `(` to follow, so the lexer tells the two
// roles apart by that token alone: _ERROR (03-lexical-structure) is
// `error` before "(", and ERROR_VALUE is `error` anywhere else.  The
// bound identifier is an operand of the expression grammar.

ERROR_VALUE.4: /error(?![A-Za-z0-9_])(?![ \t]*\()/

%extend primary_expression: ERROR_VALUE -> error_value
```

**Why:** the type and its fields are already reachable through type names and member access, so what this section settles is the two roles of lowercase `error` — raise keyword and bound identifier — told apart by the next token, because a grammar-only split would need context-sensitive productions.

---
