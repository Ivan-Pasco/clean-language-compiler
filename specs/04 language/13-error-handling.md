# 13. Error Handling

A Clean expression can fail: division by zero, a `!` on a `none`, a violated contract, or an explicit `error("...")`. `error(...)` is how a program signals a failure — it is a signal, not a value, so a function's declared return type never mentions failure — and `onError` is how a caller catches one. This chapter is the home of the whole failure path: raising, catching (in both suffix and block forms), the runtime failures the language itself raises, the `Error` value the handler binds, and what happens when a failure reaches the top of the program with nothing catching it.

A Clean expression can fail. `error(...)` is how a program signals a failure, and `onError` is how a caller deals with one. This chapter is the home of both.

### ERH-01 — Raising an error

`error(message)` raises a failure with a human-readable message. It interrupts the expression it appears in: the statements after it in the same body do not run.

```clean
functions:
	integer divide(integer a, integer b)
		if b == 0
			error("Cannot divide by zero")
		return a / b
```

`error` is a hard keyword ([3 — Lexical Structure](./03-lexical-structure.md)). It takes one `string` argument.

**`error(...)` is a signal, not a value.** It raises a failure; it does not produce something the surrounding expression can use. It MUST NOT appear in value position — `x = error("boom")` is [`SEM004`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) — and a function's declared return type never mentions failure. A function that can fail has the same signature as one that cannot ([ADR-0016](../01%20governance/decisions/0016-error-value-or-signal.md)).

The alternative was for a fallible call to yield either its value or an error value. It is unavailable: the runtime raises failures for division by zero, for `!` on `none`, and for a violated contract ([ERH-03](#erh-03--the-failures-the-language-itself-raises)), so under that reading the type of `a / b` would have to admit an error, and with it every arithmetic expression in the language.

### ERH-02 — Handling an error with `onError`

`onError` supplies what happens when the expression to its left fails. It has two forms.

**Suffix form** — a single fallback expression on the same line:

```clean
integer value = riskyCall() onError 0
string name = lookupName(id) onError "unknown"
```

**Block form** — an `onError:` block, for a handler that needs more than one line:

```clean
string content = file.read(path) onError:
	print("Could not read " + path)
	print(error)
	return ""
```

Both forms bind the failure to `error`, an identifier in scope only inside the handler: in the suffix form that scope is the fallback expression, in the block form it is the block body.

**Precedence.** `onError` binds loosest of all operators — looser than assignment — so `x = f() onError 0` parses as `x = (f() onError 0)` and the fallback covers the whole right-hand side. It is the last level of the table in [6 — Expressions](./06-expressions.md).

**Relation to the none-handling operators.** `onError` and `default` answer different questions and do not substitute for each other: `default` supplies a value when an expression yields `none` — a successful result that happens to be absent — while `onError` supplies one when an expression *fails*. The `!` operator turns absence into a failure: `value!` raises [`RUN004`](../03%20platform/09-error-codes.md#312-runtime-codes-run) when `value` is `none`, and that failure is catchable by an enclosing `onError` like any other. All three operators are specified in [6 — Expressions](./06-expressions.md); this chapter owns only the failure path.

### ERH-03 — The failures the language itself raises

A program's own `error(...)` is not the only source of failure. The runtime raises these too, and `onError` catches them the same way:

| Situation | Code |
|-----------|------|
| Integer arithmetic failure — `integer` division or remainder by zero, `integer` division overflow — or a failed numeric conversion. `number` arithmetic never raises: it follows IEEE 754, so `1.0 / 0.0` is `Infinity` and `math.*` domain errors are NaN ([15 §Math Module](./15-standard-library.md#math-module)) | [`RUN003`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| `!` applied to a value that is `none` | [`RUN004`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| A `before` contract violated | [`RUN005`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| An `after` or `always` contract violated | [`RUN011`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| A JSON parse failure | [`RUN006`–`RUN010`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |

Contract violations are catchable, but catching one in ordinary code is a mistake: a violated contract means the program's own assumptions are wrong, not that an external operation failed. See [10 — Contracts §5](./10-contracts.md).

### Errors in Asynchronous Code

A failure inside a `background` task does not propagate to whatever started it. What it does instead — and whether such a task can be cancelled at all — is unspecified; see [18 — Asynchronous Programming](./18-async.md) and [ADR-0012](../01%20governance/decisions/0012-async-cancellation-and-failure.md).

### Errors in Compile-Time Code

Inside a `compiletime` function, `onError` catches failures raised by the Clean code the handler calls. It does **not** catch or convert the diagnostics a handler emits — those are structured output, not failures. See [21 §21.10](./21-block-handlers.md).

### ERH-04 — The `error` binding is an `Error` value

Inside a handler, `error` is a value of the built-in type **`Error`**, a record with two fields:

| Field | Holds |
|-------|-------|
| `error.message` | The human-readable message. For a program's own `error("…")`, the argument it was given. |
| `error.code` | `string?` — the registered diagnostic code of the failure (`RUN003`, `RUN004` and the rest of [ERH-03](#erh-03--the-failures-the-language-itself-raises)), and `none` when the failure came from a program's own `error(...)`. |

An `Error` is an ordinary value once caught: it can be stored in a variable declared `Error`, compared, and passed to a function. What it cannot do is exist before a failure — there is no `Error` constructor, and `error(...)` raises rather than builds.

```clean
string content = file.read(path) onError:
	print(error.message)
	print(error.code default "raised by the program")
	return ""
```

`Error` is capitalised, so it is a distinct name from the `error` keyword and from the `error` binding ([LEX-08](./03-lexical-structure.md#lex-08--every-name-is-case-sensitive)). It is not a reserved word.

A program's own `error(...)` carries no code, and that absence is `none` rather than an empty string — a distinction [ADR-0015](../01%20governance/decisions/0015-optional-type-first-class.md) made available by settling `T?` in field position.

**A failing host function.** When the error arm of a fallible host-function call is taken, the binding carries `message = "host function {name} failed"` — `{name}` being the function's Clean-source declaration name — and `code = none`. The host's error payload never surfaces to the program, so this synthesized message is the only thing a Clean program can observe about a host failure. The wording and the boundary rule are owned by the [libraries specification §8.3](../02%20components/framework/09-libraries-specification.md#83-syntax) ("Fallibility at the boundary").

### ERH-05 — An unhandled failure ends the program

A failure that reaches the top of the program with no enclosing `onError` MUST end execution with [`RUN018`](../03%20platform/09-error-codes.md#312-runtime-codes-run), reporting the `Error`'s message and code.

This is a runtime outcome and MUST NOT be reported at compile time. Under [ERH-01](#erh-01--raising-an-error) nothing in a function's type records that it can fail, so no static analysis can decide whether a given call is reachable-and-failing; a compile-time rule would have to either reject correct programs or accept incorrect ones.

## Changelog

- 2026-08-20 — Two fixes from the compiler's Milestone 8 post-work (`clean-language-compiler/docs/DISCOVERIES-M8.md` §9, via [work/2026-08-20-runtime-error-message-wordings.md](../work/2026-08-20-runtime-error-message-wordings.md)). **ERH-03**: the RUN003 row scoped to integer arithmetic and failed conversions — its old "division by zero, domain error" contradicted 15's math contract (domain errors are NaN, never raised) and read as covering `number` division, which is IEEE 754 and never raises (full raise-site list and byte-exact message templates: [10 §RUN003](../03%20platform/10-semantic-rules.md#run003--arithmetic-error)). **ERH-04**: gains the host-failure binding contract — `message = "host function {name} failed"`, `code = none` — ratifying the compiler's pinned adoption; the exact wording is homed in the libraries specification §8.3, cross-linked.
- 2026-08-02 — `ERH-04`'s `code` field becomes `string?`, holding `none` for a program-raised failure instead of the empty string. It was written as a `string` placeholder because `T?` in field position was undecided; [ADR-0015](../01%20governance/decisions/0015-optional-type-first-class.md) settled it, and an absent code is now stated as absence rather than encoded as an empty value.
- 2026-08-02 — [ADR-0016](../01%20governance/decisions/0016-error-value-or-signal.md) closed: `error(...)` is a **signal**, and the open banner on ERH-01 is replaced by that statement plus the reason the value reading was unavailable — [ERH-03](#erh-03--the-failures-the-language-itself-raises)'s runtime failures would have forced an error into the type of `a / b`, and so into every arithmetic expression. `ERH-04` gives the handler binding the type it had been used without: the built-in `Error`, with `message` and `code`. `ERH-05` makes an unhandled failure the new [`RUN018`](../03%20platform/09-error-codes.md#312-runtime-codes-run), a runtime outcome rather than a compile-time one, since no signature records that a function can fail.
- 2026-08-01 — Conflict-log remediation (L18): the chapter is now the home of the whole failure path rather than of the suffix form alone. Added the `onError:` block form — used by [18 — Async](./18-async.md) and [20 — State Management](./20-state-management.md) and previously defined nowhere — the precedence of `onError`, its relation to `default` and `!`, the scope of the `error` binding, and the table of runtime failures the language itself raises; the chapter previously cited no diagnostic code at all ([SDD-04](../01%20governance/03-spec-driven-design.md)). Whether `error(...)` yields a value or only a signal is recorded as open in [ADR-0016](../01%20governance/decisions/0016-error-value-or-signal.md). Corrected the `divide` example, whose parameters were declared in an `input` block that no chapter defines.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users signaling and catching failures; anyone reasoning about the `Error` value bound inside a handler
- **Rule prefix:** `ERH-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Expressions](./06-expressions.md) (`default`, `!`, and `onError` precedence), [Contracts](./10-contracts.md), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md)
