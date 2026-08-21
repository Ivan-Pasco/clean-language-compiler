# 18. Asynchronous Programming

Clean's async surface is three keywords: `start` runs an expression in the background, `later` declares a deferred binding whose read blocks until the value is ready, and `background` runs a task without keeping a binding at all. A deferred binding can be cancelled, a failure surfaces at the read site (for `later`) or immediately (for `background`), and marking a function `background` makes every call to it run in the background. This chapter defines the surface and how it lowers to the WIT `poll` / `ready` / `cancel` boundary.

Clean uses `start`, `later`, and `background` for simple background execution:
- `start` begins a task in the background
- `later` declares that the result will be available in the future
- The value blocks only when accessed
- Use `background` to run a task without keeping the result
- Mark a function as `background` to always run it in the background

### ASY-01 — The two uses of `start`

The `start` keyword has two distinct meanings in Clean Language:

| Context | Syntax | Purpose |
|---------|--------|---------|
| Entry point | `start:` | Block that marks where your program begins (top-level only) |
| Background expression | `start functionCall()` | Starts a function running in the background |

These are completely separate features that happen to share a keyword. The compiler tells them apart by context.

### Start Expression

Use `start` before a function call to run it in the background:

```clean
start:
	// Using 'start' for background — different from the entry block!
	later data = start fetchData("url")
	print("Working...")
	print(data)          // blocks here only

	background logAction("login")    // runs and ignores result
```

- `later T name = start f()` — begins `f()` and binds `name` as a **deferred binding** of type `T`. Execution continues with the next statement.
- Reading `name` blocks until the task completes and yields its `T`.
- `background f()` runs `f()` and keeps no binding.

**`start` appears in exactly those two places.** It is the right-hand side of a `later` binding, or it follows `background`. A `start` anywhere else — `any user = start api.getUser(id)` — is [`SYN002`](../03%20platform/09-error-codes.md#31-syntax-codes-syn): there is nothing to bind the task to and no defined moment at which its value would be read.

### ASY-03 — Cancelling and failing

**A deferred binding can be cancelled.** `name.cancel()` requests that the task stop; the binding may not be read afterwards, and reading a cancelled binding is [`RUN019`](../03%20platform/09-error-codes.md#312-runtime-codes-run). Cancelling a task that has already completed does nothing and is not an error.

```clean
later string page = start fetch(url)
if userNavigatedAway()
	page.cancel()
```

Cancellation is what [Platform 02 §2.8](../03%20platform/02-host-bridge.md) requires: the compiler lowers `start`, `later` and `background` to `poll` / `ready` / `cancel` at the WIT boundary precisely so the guest keeps control of scheduling. Without a surface that lowers to `cancel`, that boundary call had no source construct.

**A failure has one destination, and it is never silence.**

| Form | Where the failure surfaces |
|------|---------------------------|
| `later T x = start f()` | At the **read** of `x`, catchable by an `onError` there like any other failure |
| `background f()` | Immediately, at the `background` expression — catchable by an `onError` attached to it |

```clean
background logAction("login") onError:
	print("logging failed: " + error.message)
```

A failure that no handler catches ends the program with [`RUN018`](../03%20platform/09-error-codes.md#312-runtime-codes-run), exactly as an unhandled failure anywhere else does ([ERH-05](./13-error-handling.md#erh-05--an-unhandled-failure-ends-the-program)). There is no silent failure: a background task whose failure vanished would leave a program that reports nothing and did not do what it says ([C-02](../01%20governance/05-concerns.md)).

**The surface names no future type.** `future<T>` and `stream<T>` are WIT types the compiler lowers onto ([Platform 02 §2.8](../03%20platform/02-host-bridge.md)); neither is a type a Clean program writes. A deferred binding is declared with `later` and its type is the type of the value it will hold.

### ASY-02 — Background functions

**Mark a function as `background` so every call to it runs in the background automatically.**

```clean
functions:
	void syncCache() background
		sendUpdateToServer()
		clearLocalTemp()

start:
	syncCache()    // runs in background automatically
```

### Background Task Error Handling

Failures follow [ASY-03](#asy-03--cancelling-and-failing): at the read site for a `later` binding, at the expression itself for `background`, and never silently.

The `loading` pattern — setting a flag before an async operation and clearing it after — should always be wrapped in `onError` to ensure cleanup happens even on failure:

```clean
functions:
	void fetchUser(integer id) background
		loading = true
		any user = start api.getUser(id) onError:
			loading = false
			error("Failed to fetch user")
		username = user.name
		loading = false
```

Without the `onError` block, if `api.getUser(id)` fails, `loading` would stay `true` and the UI would be stuck in a loading state.

### Semantics Summary

- A `later` binding is a future — a placeholder for a value that will exist later.
- Accessing a `later` binding blocks the current task until the underlying computation finishes.
- The guest keeps control of scheduling: the compiler lowers these keywords to `poll` / `ready` / `cancel` calls the guest makes, not to a host scheduler it hands work to ([Platform 02 §2.8](../03%20platform/02-host-bridge.md)). The concurrency unit is the component instance, not a thread.
- Failures surface at the read of a `later` binding and at the `background` expression itself — never silently ([ASY-03](#asy-03--cancelling-and-failing)).
- Marking a function `background` is equivalent to prefixing every call to it with `background`.

## Changelog

- 2026-08-02 — [ADR-0012](../01%20governance/decisions/0012-async-cancellation-and-failure.md) closed; `ASY-03` minted. Cancellation gains a surface — `name.cancel()` on a deferred binding — which [Platform 02 §2.8](../03%20platform/02-host-bridge.md) had required at the WIT boundary while the language offered nothing that lowered to it. Silent failure is withdrawn: a `later` binding's failure surfaces at its read, a `background` failure at the expression itself where an `onError` can reach it, and an uncaught one ends the program with [`RUN018`](../03%20platform/09-error-codes.md#312-runtime-codes-run) — the chapter had specified silence with an escape hatch its own `background` form could not reach. `start` is restricted to the two positions that bind it, which retires the contradictory `any user = start api.getUser(id)`. The "runtime's scheduler" wording is replaced by §2.8's guest-controlled model.
- 2026-08-01 — Fase 5 (zero-debt pass): "the current thread continues" restated without "thread", which names nothing in the platform model.
- 2026-08-01 — Fase 4: rules `ASY-01`, `ASY-02` minted. Cancellation, failure propagation and the type of `start` without `later` recorded as open in [ADR-0012](../01%20governance/decisions/0012-async-cancellation-and-failure.md) — the chapter offers no cancellation surface although [Platform 02 §2.8](../03%20platform/02-host-bridge.md) requires `cancel` at the WIT boundary, and it specifies silent failure with an escape hatch its own `background` form cannot reach.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users running work in the background; anyone reasoning about cancellation or failure of a `later` binding
- **Rule prefix:** `ASY-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Error Handling](./13-error-handling.md), [Platform 02 — Host Bridge §2.8](../03%20platform/02-host-bridge.md), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md)
