# 20. State Management

State in Clean is memory that outlives a single function call. Declare it in a `state:` block at the top level (app-wide); mutate it with ordinary assignment; observe it with a `watch:` block; derive read-only values from it with `computed:`; guard individual writes with a `guard` clause; and enforce cross-variable invariants with a `rules:` sub-block. This chapter is the home of all of these — the parallel with contracts (guard is to `before` what rule is to `always`), the diagnostics, and how state interacts with background tasks.

> **Note:** an earlier revision of this chapter defined screen-scoped state via a language-level `screen <Name>:` construct. That construct was withdrawn per [ADR-0030](../01%20governance/decisions/0030-withdraw-screen-from-language.md); `screen` is not a language keyword. UI-local scoping, if needed, is a concern for the application to organise, not for the language to define.

State is a first-class concept in Clean Language. It provides persistent memory that outlives function calls, with built-in observability and sequential update guarantees.

### Core Principles

1. **Persistent Memory**: State stores values beyond function execution. Variables are temporary; state is remembered.
2. **Mutable**: State values are updated using normal assignment.
3. **Observable**: The runtime detects all state changes and can react to them.
4. **Explicit Scope**: State is declared at the top level of a file (app-wide).
5. **Sequential Updates**: State mutations are processed in order, preventing race conditions.
6. **Background Compatible**: Background operations update state on completion, and the sequential-update guarantee still holds.
7. **In-Memory**: State lives in memory for the lifetime of its scope. It is not automatically persisted to disk.
8. **First-Class**: State is recognized by the compiler and enforced by the runtime.

### SMG-01 — State declaration

Use the `state:` block at the top level of a file to define state variables. State declared here persists for the application's lifetime and is accessible from any function in the module.

```clean
state:
	integer count = 0
	string username = ""
	boolean isLoggedIn = false
```

**Rules:**
- `state:` is a top-level section per [FIL-01](./08-file-structure.md).
- Initial values are required for all state variables.
- Names within a `state:` block must be unique.


### State Access

**Access state directly by name.** State variable names are unique within a `state:` block, so no prefix is ever needed.

```clean
state:
	integer count = 0
	string username = ""

functions:
	void showStatus()
		integer current = count
		string name = username
		print("User: " + name + ", Count: " + current.toString())
```

### State Mutation

Mutate state with standard assignment.

```clean
state:
	integer count = 0

functions:
	void increment()
		count = count + 1

	void reset()
		count = 0
```

### SMG-02 — Guard clauses

A state variable declaration may carry a **guard clause**, written as an indented line directly beneath the declaration:

```
guard <expr> else "<message>"
```

```clean
state:
	integer count = 0
		guard value >= 0 else "Count cannot be negative"
```

**Rules:**

- The guard expression must be a **pure boolean expression**. It may reference `value` (the proposed new value) and any currently-in-scope identifiers, but must not contain side-effecting operations such as function calls that perform I/O or mutate state. A guard that is not a pure boolean expression is a compile-time error ([STATE001, Platform 10 §8](../03%20platform/10-semantic-rules.md)).
- The `else "<message>"` clause is **mandatory** — every guard names the message reported when it rejects an update.
- **Runtime semantics:** on assignment to a guarded state variable, the guard expression is evaluated with `value` bound to the proposed new value. If it evaluates to `false`, the state update is rejected, the state variable retains its previous value, and the message from the `else` clause is reported as `"State update rejected: {guard_message}"` ([STATE002, Platform 10 §8](../03%20platform/10-semantic-rules.md) — a runtime rule, not a compile-time error).

```clean
state:
	integer count = 0
		guard value >= 0 else "Count cannot be negative"

functions:
	void decrement()
		count = count - 1    // runtime rejection (STATE002) if count is already 0
```

**A declaration may carry more than one guard.** They are evaluated in written order and stop at the first one that fails: that guard's message is the one reported, and no later guard runs. Each guard names its own message, so the first failure is the one the developer needs; evaluating the rest would produce several messages for a single rejected update with no defined way to report them.

### SMG-03 — State rules

A `state:` block may contain a **`rules:` sub-block** listing expressions over the state in scope:

```clean
state:
	integer count = 0

	rules:
		count >= 0
```

**Rules:**

- Every expression listed under `rules:` must be a **boolean expression**. A non-boolean expression (e.g., integer arithmetic with no comparison) is a compile-time error, reported as `"State rule expression must be a boolean expression, got {type}"` ([STATE005, Platform 10 §8](../03%20platform/10-semantic-rules.md)):

```clean
state:
	integer count = 0

	rules:
		count + 1    // error (STATE005): not a boolean expression
```

- **Runtime semantics:** every rule in the block is evaluated when a function that assigned to any state variable in that block **returns**. A rule that evaluates to `false` raises [`STATE006`](../03%20platform/09-error-codes.md#37-state-codes-state), naming the rule's source text. The state is not rolled back.

**A guard and a rule are not two spellings of the same thing.** They are the state-block counterpart of the split [10 — Contracts](./10-contracts.md) already draws between `before` and `always`:

| | Guard | Rule |
|---|---|---|
| Attached to | one declaration | the `state:` block |
| Sees | `value`, the *proposed* update | every state variable in the block |
| Runs | on each assignment, before it lands | when a function that changed state returns |
| On `false` | rejects the update, state unchanged, [`STATE002`](../03%20platform/09-error-codes.md#37-state-codes-state) | raises [`STATE006`](../03%20platform/09-error-codes.md#37-state-codes-state), state unchanged from what the function left |

A guard vetoes one change and the program carries on — a rejected update is an expected outcome. A violated rule means the state as a whole reached a combination the program declared impossible, which is a defect in the program rather than an input to handle. Catching `STATE006` in ordinary code is the same mistake as catching a contract violation ([10 §5](./10-contracts.md)).

**Why rules run at function exit rather than on each assignment.** A rule spans several variables, so reaching a valid combination often takes more than one assignment: with `rules: end > start`, setting `start` before `end` passes through a state the rule forbids. Evaluating after every assignment would make that update impossible to write. The enclosing function's return is a boundary that already exists in the language, needs no transaction construct, and is where `after` and `always` contracts are already checked.

### SMG-04 — Observing state with `watch`

Use `watch:` to react when state changes. The block runs automatically after the state is updated.

```clean
state:
	integer count = 0

watch count:
	print("Count changed to: " + count.toString())

functions:
	void increment()
		count = count + 1    // Triggers the watch block
```

**Watching multiple state variables:**

```clean
state:
	string firstName = ""
	string lastName = ""

watch (firstName, lastName):
	print("Name changed")
```

### SMG-05 — Computed state

**Computed state is a read-only derived value that is automatically re-evaluated when its dependencies change.** Declare computed values inside a `computed:` block within `state:`.

```clean
state:
	string firstName = ""
	string lastName = ""

	computed:
		string fullName
			return firstName + " " + lastName

functions:
	void setName(string first, string last)
		firstName = first
		lastName = last

start:
	setName("Alice", "Smith")
	print(fullName)    // Prints: Alice Smith
```

**Rules:**
- Computed state is **read-only**; assigning to it is [`STATE004`](../03%20platform/09-error-codes.md#37-state-codes-state).
- **Dependency tracking is performed by static analysis at compile time.** The compiler inspects which state variables appear in the computed block's body and registers them as dependencies.
- External function calls inside a computed body are treated as opaque — their internal dependencies cannot be tracked. If a computed value depends on an external function, it is conservatively re-evaluated on every state change.
- **Circular dependencies between computed state variables are a compile error** ([`STATE003`](../03%20platform/09-error-codes.md#37-state-codes-state)). For example, if `fullName` references `displayName` and `displayName` references `fullName`, compilation fails. `STATE003` covers the circular case only.
- The return type of the computed body must match the declared type of the computed variable. A mismatch is [`SEM018`](../03%20platform/09-error-codes.md#32-semantic-codes-sem), not `STATE003` — the two are reciprocal and the boundary is fixed in [Platform 10](../03%20platform/10-semantic-rules.md).

### State Reset

Reset state to its initial value using the `reset` keyword.

```clean
state:
	integer count = 0
	string username = ""

functions:
	void clearCount()
		reset count          // count returns to 0

	void clearAll()
		reset state          // all state returns to initial values
```

### State with Background Tasks

**Background functions can update state when they complete. Updates remain sequential, preventing race conditions.**

Errors in background tasks do not propagate to the caller — always attach an `onError` handler when updating state in a background function, so cleanup happens even on failure:

```clean
state:
	string username = ""
	boolean isLoggedIn = false
	boolean loading = false

functions:
	void fetchUser(integer id) background
		loading = true
		any user = start api.getUser(id) onError:
			loading = false
			error("Failed to fetch user")
		username = user.name
		isLoggedIn = true
		loading = false
```

### Complete Example

```clean
// App-level state
state:
	string user = ""
	string theme = "light"

	computed:
		string greeting
			return "Hello, " + user

// React to user changes
watch user:
	print(greeting)

// Core functions
functions:
	void setUser(string name)
		user = name

	void setTheme(string newTheme)
		theme = newTheme

	void loadProfile() background
		any profile = start fetchProfile()
		user = profile.name

// Entry point
start:
	setUser("Alice")       // Triggers watch, prints "Hello, Alice"
```

### Summary

| Syntax | Purpose |
|--------|---------|
| `state:` (top-level) | App-scoped state |
| `fieldName` | Access state by name |
| `fieldName = value` | Mutate state |
| `guard <expr> else "<msg>"` | Reject invalid updates to a state variable (STATE001/STATE002) |
| `rules:` | Boolean rule expressions over state in scope (STATE005) |
| `watch fieldName:` | React to state changes |
| `watch (a, b):` | React to multiple state changes |
| `computed:` | Define derived state |
| `reset fieldName` | Reset to initial value |
| `reset state` | Reset all state in scope |

### State vs Variables

| Aspect | Variables | State |
|--------|-----------|-------|
| Lifetime | Function scope | Application lifetime |
| Observability | Not observable | Observable via `watch:` |
| Persistence | Lost after function returns | Persists until reset or scope ends |
| Declaration | `integer x = 0` | Inside `state:` block |

---

## Changelog

- 2026-08-07 — Screen-scoped state and the language-level `screen <Name>:` construct WITHDRAWN per [ADR-0030](../01%20governance/decisions/0030-withdraw-screen-from-language.md). SMG-01 simplified to top-level `state:` only; §"State in Screens" removed entirely; the Complete Example trimmed to remove its `screen Home:` section; summary tables updated. `screen` is not a language keyword; the ui library does not register it as a block either. UI-local scoping is a concern for the application to organise, not for the language to define. No changes to SMG-02..SMG-05 (guard, rules, computed, watch — all remain app-scoped state features).
- 2026-08-02 — [ADR-0011](../01%20governance/decisions/0011-state-rules-runtime-semantics.md) closed. `rules:` gains the runtime semantics it never had — a `rules:` block type-checked and then did nothing observable. Every rule is evaluated when a function that assigned to the block's state returns, and a false rule raises the new [`STATE006`](../03%20platform/09-error-codes.md#37-state-codes-state). Guards and rules are kept as distinct mechanisms on the `before`/`always` model of [10 — Contracts](./10-contracts.md): a guard sees the proposed `value` and vetoes one update, a rule sees the whole block and asserts a combination. Multiple guards per declaration are permitted, evaluated in written order, stopping at the first failure.

- 2026-08-01 — Fase 5 (zero-debt pass): assignment to computed state cites [`STATE004`](../03%20platform/09-error-codes.md#37-state-codes-state).
- 2026-08-01 — Fase 3/4 (L17): the computed-state codes corrected — `STATE003` covers the **circular** case only, and a type mismatch is [`SEM018`](../03%20platform/09-error-codes.md#32-semantic-codes-sem); the chapter asserted the reverse and cited `SEM001` (an unrelated code) for circularity. `STATE003`'s old name had been retired on 2026-08-01 when that boundary was ratified, and this chapter never followed. Rules `SMG-01`..`SMG-05` minted. The unspecified runtime semantics of `rules:` and of multiple guard clauses are recorded in [ADR-0011](../01%20governance/decisions/0011-state-rules-runtime-semantics.md).
- 2026-08-01 — Promoted `guard <expr> else "<msg>"` and the `state: rules:` sub-block into this document as their home (conflict-log P16.11): both constructs previously existed only in the rule bodies of [Platform 10 §8 (STATE001–STATE005)](../03%20platform/10-semantic-rules.md), which remains the home of their diagnostics. Semantics not specified by those rule bodies are marked "(unspecified)" rather than invented. Added Status header and Changelog section.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users declaring persistent state; anyone reasoning about guards, rules, watches, or computed derivations
- **Rule prefix:** `SMG-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Contracts](./10-contracts.md) (parallel `before`/`always` model), [Asynchronous Programming](./18-async.md) (state updates from background tasks), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md), [Platform 10 — Semantic Rules](../03%20platform/10-semantic-rules.md)
