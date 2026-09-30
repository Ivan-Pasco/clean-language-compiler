# 18 async — Grammar

**Title:** 18 async — Grammar

**Id:** `ASYG-`

**Purpose:** Companion grammar file for 18 — Asynchronous Programming, per DOC-15. It defines the syntactic surface of asynchronous execution and nothing of its meaning: the `start` expression that begins a call in the background — distinct from the `start:` entry block, which `09-functions.lark.md` defines as `start_block` — and the two positions it may occupy; the `later` deferred binding as a statement kind of its own; the `background` statement with its optional `onError` tail; the `background` modifier on a function declaration and its single placement; and the note that cancellation is an ordinary `.cancel()` call needing no production. Whether a construct is well-formed is this file's question; what it does at runtime — blocking reads, failure handling, read-after-cancel — is ASY-01..ASY-03's, in the companion chapter. It is written for parser implementers and for readers of chapter 18 checking a form against the grammar; it rests on the companion chapter for the semantics it encodes and on the expression, function and error-handling grammars for the productions it reuses.

---

### ASYG-01 — 1. `start` expression (background)

**Id:** ASYG-01

**Kind:** rule

**Name:** 1. `start` expression (background)

**Question:** In which production shape is the `start` background call written, and how narrowly is its operand constrained — any expression, or a postfix expression whose last operation is a call?

**Parent:** LANG-15

**Norm:**

```lark
// ASY-01 §Start Expression: `start` prefixes a function call to begin
// it in the background.  It appears in exactly TWO positions per ASY-01
// boundary rule:
//     1. Right-hand side of a `later T name = start f()` binding
//     2. Following `background` in `background f()`
// A `start` in any other position is SYN002.

start_expression: _START call_expression

// A postfix_expression whose LAST postfix operation is a call — i.e.
// any expression whose top-level operation is a function or method
// call: `f()`, `obj.method()`, `a.b.c(x)[0].run()`.  postfix_expression
// and argument_list are defined in 06-expressions.lark.md.

call_expression: postfix_expression "(" argument_list? ")"
```

**Why:** `start` is admitted in exactly two positions, so the production names its operand precisely — a postfix expression whose last operation is a call — instead of any expression, and the reader is reminded that the entry-point `start:` is another construct entirely.

---

### ASYG-02 — 2. `later` deferred binding

**Id:** ASYG-02

**Kind:** rule

**Name:** 2. `later` deferred binding

**Question:** Is a `later` deferred binding its own statement kind or a modifier on a variable declaration, and which right-hand sides does its production admit?

**Parent:** LANG-15

**Norm:**

```lark
// ASY-01: `later T name = start f()` declares a deferred binding of
// type T.  T is any type_expression.  The right-hand side MUST be a
// start_expression per the boundary rule cited above: ASY-01 constrains
// `start` to exactly two positions, and every chapter example uses
// `start f()` on the right-hand side, so no other right-hand side is
// admitted.

later_binding: _LATER type_expression IDENTIFIER "=" start_expression

// later_binding is a distinct statement kind, NOT a modifier on
// variable_declaration.  Reasons:
//   - Syntactically different (has the `later` prefix keyword)
//   - Semantically different (creates a deferred binding, not an
//     immediate value)
//   - Different lowering (compiles to poll / ready / cancel at the WIT
//     boundary)
// 07-statements.lark.md's statement alternation adds later_binding as
// its own alternative.
```

**Why:** a deferred binding differs from a variable declaration in syntax, meaning and lowering, so it is its own statement kind, and its right-hand side is narrowed to `start f()` because that is every example the chapter shows, and ASY-01's two positions for `start` leave no other.

---

### ASYG-03 — 3. `background` prefix expression

**Id:** ASYG-03

**Kind:** rule

**Name:** 3. `background` prefix expression

**Question:** Is `background f()` a statement or an expression, and what optional `onError` tail may its production carry?

**Parent:** LANG-15

**Norm:**

```lark
// ASY-01: `background f()` runs f() and keeps no binding.  Statement,
// not an expression.  Can be followed by an onError tail per ASY-03 for
// handling failures, in either shape of 13-error-handling.lark.md: the
// suffix (`onError fallback`) or the block (`onError:` and a body) —
// the same handler-attachment pattern.
//
// A background statement ends its own line (STMG-01): with its
// _NEWLINE when it has no tail or a suffix tail, with the _DEDENT of its
// handler body when the tail is a block.

background_statement: _BACKGROUND call_expression _NEWLINE
                    | _BACKGROUND call_expression on_error_tail

on_error_tail: _ONERROR expression _NEWLINE
             | _ONERROR ":" block
```

**Why:** `background f()` keeps no binding, so it is a statement rather than an expression, and the optional handler tail reuses the two `onError` shapes of 13-error-handling.lark.md so a failure attaches to a background call the way it attaches anywhere else.

---

### ASYG-04 — 4. `background` function modifier

**Id:** ASYG-04

**Kind:** rule

**Name:** 4. `background` function modifier

**Question:** In how many positions of a function declaration may the `background` modifier stand, and is that placement restated here or declared as a suffix keyword for the function grammar to incorporate?

**Parent:** LANG-15

**Norm:**

```lark
// ASY-02: `void syncCache() background` marks a function declaration
// so every call to it runs in the background automatically.  The
// `background` modifier is postfix-only — it comes AFTER the parameter
// list, BEFORE the body's _NEWLINE.  Every chapter example uses this
// form; allowing multiple placements would give library authors two
// ways to write the same thing, which contradicts LDR-08 "one way to do
// things".
//
// This is a modifier on function_declaration (in 09-functions.lark.md),
// which incorporates it in that single position:
//     function_declaration: type_expression IDENTIFIER
//                           "(" parameter_list? ")" background_modifier?
//                           _NEWLINE _INDENT function_body _DEDENT
// Rather than restate function_declaration here, this file declares
// the modifier as a suffix keyword that 09-functions incorporates.

background_modifier: _BACKGROUND
```

**Why:** one placement for the modifier — after the parameter list, before the body — keeps one way to write it, as LDR-08 asks, and stating it as a suffix keyword for 09-functions.lark.md to incorporate avoids restating `FunctionDeclaration` in two files that could drift apart.

---

### ASYG-05 — 5. Cancellation

**Id:** ASYG-05

**Kind:** rule

**Name:** 5. Cancellation

**Question:** Does cancelling a deferred binding need a production of its own, or is `name.cancel()` already covered as an ordinary call postfix on the bound identifier?

**Parent:** LANG-15

**Norm:**

```lark
// ASY-03: `name.cancel()` requests a deferred binding be cancelled.  No
// new rule — `.cancel()` is an ordinary call postfix on the identifier
// bound by a later_binding.  Semantics (RUN019 if read after cancel)
// are ASY-03's rules, not grammar.
```

**Why:** `.cancel()` is an ordinary call postfix on a bound name, so the grammar gains nothing from a production of its own; the section exists to say so and to leave the read-after-cancel semantics (RUN019) with ASY-03.

---
