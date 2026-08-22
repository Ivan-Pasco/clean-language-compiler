# ADR-0016 — Is `error(...)` a value or a signal?

Clean raises failures by calling `error("...")` and handles them with `onError`, but no chapter said what `error(...)` actually is: the testing chapter compared call results against it (only possible if it is a value) while the error-handling chapter treated it as a signal that interrupts an expression. This ADR settles it as a signal — `error(...)` never yields a value — while typing what a handler binds as a real built-in `Error` record with `message` and `code` fields, so the testing chapter's failure comparisons stay expressible without giving every arithmetic operator a nullable return.

---

## Context

Clean raises a failure by calling `error("message")` and handles one with `onError`, in either the suffix form (`value = riskyCall() onError 0`) or the block form. [13 — Error Handling](../../04%20language/13-error-handling.md) defines the mechanism and the `error` binding available inside a handler.

What no chapter states is what `error("…")` *is*. Two chapters use it in ways only one of the readings supports:

- [11 — Testing](../../04%20language/11-testing.md) writes `safeDivide(10, 0) = error("Division by zero")` — comparing the result of a call against an error, which requires the error to be a **value**: constructible, storable, comparable, and returnable from a function whose declared type is not an error type.
- [13 — Error Handling](../../04%20language/13-error-handling.md) and [21 — Block Handlers](../../04%20language/21-block-handlers.md) treat it as a **signal**: it interrupts the expression, and `onError` is what resumes it. Under this reading the test comparison above has no meaning, because the call never returns anything to compare.

The two readings are not stylistic. They produce different function signatures (does a fallible function's type mention the error?), different exhaustiveness obligations, and different answers to whether an unhandled `error` is a compile-time defect or a runtime trap. The distinction also decides how the language maps onto the `result` types the WIT boundary already uses.

The chapter is 32 lines and cites no diagnostic code, so neither reading is currently checkable.

## Options considered

**A — Signal only.** `error(...)` interrupts; `onError` resumes with a value; errors never appear in a type. Smallest surface, and it matches how the language reads today. Cost: the testing chapter's comparison form is withdrawn, and there is no way to hold an error to inspect it later.

**B — Value only.** A fallible call yields either its value or an error value; `onError` is sugar for matching on it. Errors become inspectable and testable, and the mapping onto `result<T, E>` at the bridge is direct. Cost: every fallible signature changes, and the language acquires an error type that [04 — Type System](../../04%20language/04-type-system.md) does not have.

**C — Signal, with a reified error object inside the handler only.** `error(...)` interrupts, but the `error` binding is a real value with fields (message, code, span) that can be stored and compared once caught. Keeps signatures unchanged while making the testing form expressible if it is rewritten to catch first. Cost: the testing chapter's current spelling still has to change.

## Decision

**Option C — a signal, with the handler binding reified.** `error(...)` raises a failure and never produces a value; what a handler binds is a real value of a built-in type.

**Option B was not available.** The ADR presents value-only as a viable trade, but [ERH-03](../../04%20language/13-error-handling.md#erh-03--the-failures-the-language-itself-raises) — already Accepted — makes the runtime raise failures for division by zero, for `!` on `none`, and for a violated contract, all catchable by `onError`. Under value-only the type of `a / b` would have to admit an error, and with it every arithmetic expression in the language. Adopting B was never a question of preference; it would have required retyping every operator.

**Between A and C, C was close to forced.** [ERH-02](../../04%20language/13-error-handling.md#erh-02--handling-an-error-with-onerror) already binds the failure to `error` and shows `print(error)`. A says errors never appear in a type, which leaves that binding without one — the gap itself. C is A with the binding typed; it adds no surface, it names something the specification was already using.

**The shape of the binding.** `Error` is a record of two `string` fields: `message`, and `code` carrying the registered diagnostic code for runtime-raised failures and the empty string for a program's own `error(...)`. It is capitalised, so [LEX-08](../../04%20language/03-lexical-structure.md#lex-08--every-name-is-case-sensitive) keeps it distinct from the `error` keyword without reserving a new word or disturbing the keyword partition. There is no constructor: an `Error` exists only after a failure.

Both fields are `string` and neither is optional, which is a constraint rather than a preference — an absent code would naturally be `string?`, and whether `T?` may appear in a field position is [ADR-0015](0015-optional-type-first-class.md)'s still-open question. The empty string is the stated absence until that lands.

**Unhandled failures are a runtime outcome**, [`RUN018`](../../03%20platform/09-error-codes.md#312-runtime-codes-run), not a compile-time defect. Under this decision no signature records that a function can fail, so whether a call fails is not statically decidable; a compile-time rule would have to reject correct programs or accept incorrect ones.

**The bridge is unaffected.** The libraries specification already lowers the failure path to `result<T, error>` on the WIT side. Signal in the language, `result` at the boundary, is consistent with what was already written.

## Consequences

The testing chapter recovers its failure case. It had carried `safeDivide(10, 0) = error("Division by zero")` — the comparison that presumed a value — and this ADR is why it was withdrawn. It is now asserted through the handler, `(safeDivide(10, 0) onError error.message) == "Division by zero"`, which needs no test-only syntax: `onError` already binds `error` inside its fallback expression.

The three items the ADR listed as depending on this are answered. The type of the `error` binding is `Error`. The interaction with `!` needed no new rule: `!` raises `RUN004`, which is a failure like any other and reaches a handler as an `Error` carrying that code. An unhandled error carries `RUN018`.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 13 — Error Handling](../../04%20language/13-error-handling.md) (`ERH-01`, new `ERH-04`, `ERH-05`) · [04 language / 11 — Testing](../../04%20language/11-testing.md) · [04 language / 04 — Type System](../../04%20language/04-type-system.md) (`TYP-01`) · [03 platform / 09 — Error Codes](../../03%20platform/09-error-codes.md) (`RUN018`) · [03 platform / 10 — Semantic Rules](../../03%20platform/10-semantic-rules.md) (`RUN018`)
