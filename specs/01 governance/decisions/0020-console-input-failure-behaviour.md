# ADR-0020 — Failure behaviour of the console input functions

`input.integer()`, `input.number()`, `input.yesNo()` and `input()` have return types with no room for failure, and the spec described them as "safe defaults" that retry on bad input — a design that hangs indefinitely on a closed stream (CI, pipeline) since a retry loop has no value to return. This ADR makes those functions fallible: `input()` returns `string?` and the parsers return the optional variants of their types, yielding `none` at end of input or on unparseable text, and the library performs no retry.

---

## Context

`input.integer(prompt)`, `input.number(prompt)` and `input.yesNo(prompt)` read a line and parse it. Each has a return type that cannot carry a failure: `integer`, `number`, `boolean`. The specification describes their behaviour on bad input as *"safe defaults: invalid input automatically retries with helpful messages"*.

Retrying is a reasonable design for an interactive prompt, but as specified it is not implementable, because three questions have no answer:

1. **End of input.** Standard input is not always a terminal. When a Clean program runs in a pipeline, in CI, or with input redirected from a file, the stream ends. A retry loop on a closed stream never terminates, and there is no value to return. Does the program trap? Does it raise through the error path? Does `input.integer` become fallible?
2. **Retry bound.** "Retries" is unbounded as written. An unbounded loop against a stream that keeps yielding unparseable data is a hang, not a safe default.
3. **The message.** *"Helpful messages"* names no text and no diagnostic code, so nothing about the retry is checkable ([SDD-05](../03-spec-driven-design.md), [SDD-04](../03-spec-driven-design.md)).

The same three questions apply to `input()` itself, whose `string` return type also has no room for "nothing left to read".

## Options considered

**A — Fallible input.** The parsing forms return `integer?`, `number?`, `boolean?` and `input()` returns `string?`; end of input yields `none` and the caller decides. Honest, composes with the optional type and the `default` operator, and removes the retry loop from the library entirely. Cost: every prompt site gains a `default` or a check, which is heavier for the tutorial case that motivated these functions.

**B — Retry when interactive, fail otherwise.** Keep the retry loop when standard input is a terminal; on a non-interactive stream, read once and raise through the error path on a parse failure or end of input. Matches what a user at a prompt expects while staying usable in a pipeline. Cost: behaviour depends on how the program was invoked, which makes it harder to test and means the same code path behaves differently in CI.

**C — Bounded retry, then raise.** Retry a fixed number of times (three, say), then raise. Terminates in every case, one behaviour everywhere. Cost: the bound is arbitrary, and it still raises from a function whose type says it cannot.

Options B and C both depend on [ADR-0016](0016-error-value-or-signal.md): "raise" has no defined meaning until `error(...)` is settled.

## Decision

**Option A — fallible input.** `input()` returns `string?`; the three parsing forms return `integer?`, `number?` and `boolean?`. Each yields `none` at end of input or on text it cannot parse. The library performs no retry.

**Option B was rejected on testability.** Retrying when standard input is a terminal and failing otherwise means the same source line behaves differently on a developer's machine than in CI. That is precisely the divergence [C-04](../05-concerns.md) exists to prevent, and it cannot be covered by a conformance test that does not itself know how it was invoked.

**Option C was rejected as arbitrary.** A bound of three has nothing behind it, and the ADR's second objection to it — that it raises from a function whose type cannot carry a failure — dissolved when [ADR-0016](0016-error-value-or-signal.md) settled that a raise is a signal and never appears in a signature. What remained was an unmotivated constant.

**The retry loop was not a design to preserve.** A stdlib function whose return type cannot express failure, looping on a stream that may be closed, is a hang with no value to return. Moving the loop into the program does not remove the possibility — a program may still loop forever on a closed stream — but it puts the loop in code its author wrote and can see, rather than inside a library call that looks total.

This became available only after [ADR-0015](0015-optional-type-first-class.md): an optional in return position was not settled surface until then.

## Consequences

The three questions the ADR listed are answered without a diagnostic code, and none is registered. End of input is `none`. There is no retry bound because there is no retry. There is no message text to specify, so nothing is left unconformance-testable.

The tutorial cost the ADR anticipated is real and visible in the chapter's own example: every prompt site now carries a `default`. It is the price of a signature that does not lie.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 15 — Standard Library §Console Module](../../04%20language/15-standard-library.md)
