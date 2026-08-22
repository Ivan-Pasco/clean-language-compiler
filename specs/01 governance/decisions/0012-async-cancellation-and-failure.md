# ADR-0012 — Asynchronous execution: cancellation, failure propagation, and the type of `start`

The 85-line async chapter defined three keywords — `start`, `later`, `background` — and left four load-bearing things unstated: cancellation, who schedules, what `start f()` yields without `later`, and a "silent failure by default" contract with an unreachable escape hatch. This ADR gives async an explicit surface: a `later` binding carries `cancel()`, reading a cancelled binding raises `RUN019`, silent failure is withdrawn, `start` is restricted to two positions, and `future<T>` stays a WIT lowering rather than a language type.

---

## Context

[18 — Asynchronous Programming](../../04%20language/18-async.md) is 85 lines and defines three keywords: `start` (begin work), `later` (bind its future), `background` (fire and forget). Four things it does not define are load-bearing:

1. **Cancellation.** [02 — Host Bridge §2.8](../../03%20platform/02-host-bridge.md) states normatively that the compiler translates `start`, `later` and `background` into `poll` / `ready` / **`cancel`** calls at the WIT boundary, and that this exists specifically to keep the guest in control of scheduling and cancellation. Chapter 18 offers no cancellation surface at all — there is nothing in the language that lowers to `cancel`.
2. **Who schedules.** Chapter 18 says background tasks run on *"the runtime's scheduler"*; §2.8 says the guest keeps control of scheduling. The word "thread" in chapter 18 appears nowhere else in the platform model, which is explicit that instance count, not thread count, is the concurrency unit.
3. **The type of `start f()` without `later`.** The chapter fixes `later T name = start f()` as binding a future of type `T`, and then writes `any user = start api.getUser(id)` two examples later. What `start` yields without `later`, and what reading a future whose task failed produces, are both unstated.
4. **Silent failure as a contract.** The chapter specifies that a failing background task *"fails silently unless an `onError` handler is attached to the expression that started it"* — while its own `background logAction("login")` form binds no expression, so no handler can be attached. A specified silent failure with an unreachable escape hatch is not a design; and it sits against [C-02](../05-concerns.md), which requires every error to tell the user what to do next.

The chapter also never mentions `stream<T>` or `future<T>`, the WIT types §2.8 says these keywords map onto.

## Options considered

**A — Futures are values with an explicit surface.** `start f()` yields `future<T>`; `later` is the binding form; `future` carries `cancel()`, and reading a failed future raises through the normal error path. Cancellation and failure both become observable, and the mapping to `poll`/`ready`/`cancel` is direct. Cost: `future<T>` must enter the type system, and the interaction with `onError` must be specified.

**B — Structured concurrency.** Tasks are bound to a lexical scope; leaving the scope cancels outstanding work and surfaces failures. No orphan tasks, no silent failure. Cost: `background` as a fire-and-forget statement no longer fits, and the chapter is rewritten around a construct it does not currently have.

**C — Keep fire-and-forget, add an explicit handle.** `background` stays silent by design but returns a handle that can be cancelled and inspected; the escape hatch becomes reachable. Cost: the smallest change, but leaves "silent by default" as the behaviour, which is what [C-02](../05-concerns.md) objects to.

## Decision

**Option A — an explicit surface for cancellation and failure**, written as [ASY-03](../../04%20language/18-async.md#asy-03--cancelling-and-failing), with one adjustment: the surface names no future type.

**Cancellation.** A deferred binding carries `cancel()`, and reading a cancelled binding raises [`RUN019`](../../03%20platform/09-error-codes.md#312-runtime-codes-run). This is what [Platform 02 §2.8](../../03%20platform/02-host-bridge.md) had been requiring on its own: it states normatively that these keywords lower to `poll` / `ready` / `cancel` so the guest keeps scheduling control, while the language offered no construct that lowered to `cancel`.

**`future<T>` stays out of the surface.** The ADR's option A puts it in the type system. It does not need to be there: `later T name` already declares the binding and its result type, and adding `future<T>` as a writable type would give the language two ways to spell one thing ([LDR-08](../../04%20language/02-language-design-rules.md#ldr-08--one-way-to-do-things)). `future<T>` and `stream<T>` remain WIT types the compiler lowers onto, which is all §2.8 ever claimed for them.

**Silent failure is withdrawn.** The chapter specified that a background failure is silent unless `onError` is attached to the starting expression, while its own `background logAction("login")` form binds no expression — a specified behaviour with an unreachable escape hatch, against [C-02](../05-concerns.md). Now a `later` binding's failure surfaces at its read, a `background` failure at the `background` expression where an `onError` can attach, and an uncaught failure ends the program with `RUN018` like any other. Nothing about this needed new machinery: [ADR-0016](0016-error-value-or-signal.md) had already settled what an unhandled failure does.

**`start` is restricted to two positions** — the right-hand side of a `later` binding, or after `background`. Anywhere else it is `SYN002`. This retires the chapter's `any user = start api.getUser(id)`, which contradicted its own rule two examples earlier and had no defined moment at which the value would be read.

**Option B — structured concurrency** — was rejected as disproportionate: it removes `background`, a reserved keyword with two documented forms, and rewrites the chapter around a construct the language does not have. **Option C** keeps silent-by-default, which is the thing [C-02](../05-concerns.md) objects to.

## Consequences

All four gaps the ADR listed are closed, and the scheduling contradiction with it: chapter 18's "runtime's scheduler" is replaced by §2.8's guest-controlled model, and the thread vocabulary is gone — the concurrency unit is the component instance.

A `background` task still cannot be cancelled. That is inherent to a form that keeps no binding, and it is now the only thing `background` gives up rather than one of three unstated behaviours.

---

## Metadata

- **Status:** Accepted (2026-08-02)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [04 language / 18 — Asynchronous Programming](../../04%20language/18-async.md) (new `ASY-03`) · [03 platform / 09 — Error Codes](../../03%20platform/09-error-codes.md) (`RUN019`) · [03 platform / 10 — Semantic Rules](../../03%20platform/10-semantic-rules.md) (`RUN019`)
