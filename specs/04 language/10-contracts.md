# 10. Contracts

Contracts are how a Clean function or class states its assumptions in plain English and has the compiler enforce them at runtime. There are three: `before` (must hold on entry), `after` (must hold on return), and `always` (must hold across the object's lifetime). This chapter defines the syntax of each, the diagnostic they raise on violation, and which are strippable at build time — a `before` is always on, because it guards the caller's side of the contract; `after` and `always` can be stripped for release builds.

Clean Language supports **design-by-contract** with three plain-English conditions: `before`, `after`, and `always`. Contracts document the assumptions a function or class makes and enforce them at runtime. If a contract is violated, execution stops with a contract violation error.

| Keyword | Role | Classical name | Checked | Strippable |
|---------|------|----------------|---------|------------|
| `before` | Precondition — must hold on entry | precondition | on function entry, before any statement | **No** — always on |
| `after`  | Postcondition — must hold on exit | postcondition | on every function return | Yes — via `--strip-checks` |
| `always` | Invariant — must hold across the object's lifetime | class invariant | after construction, and around every public method call (see §3) | Yes — via `--strip-checks` |

See §6 for the rationale behind the split and the exact behavior of `--strip-checks`.

---

## 1. `before` — Preconditions

Use `before:` to declare conditions that must be true for a function to execute. If any condition is false when the function is called, execution stops with a contract violation error.

**Syntax:** `before` is always a block. Each line inside the block is one boolean expression, checked in order.

```clean
before:
	<boolean_expression>
	<boolean_expression>
	...
```

**Example:**
```clean
functions:
	integer divide(integer a, integer b)
		before:
			b != 0
		return a / b

	void setAge(integer age)
		before:
			age >= 0
			age <= 150
		// implementation
```

### CTR-01 — `before` preconditions

- Can only appear inside functions or class methods
- Must appear at the top of the function body, before any other statement
- Every line inside the block is a boolean expression, checked in the order written — a non-boolean line is [`SEM023`](../03%20platform/09-error-codes.md#32-semantic-codes-sem)
- **Always checked at runtime and cannot be disabled** (see §6)

**Error on violation:** [`RUN005`](../03%20platform/09-error-codes.md#312-runtime-codes-run) `AssertionFailure`.
```
Contract violation: before failed at divide:2
  Expression: b != 0
```

---

## 2. `after` — Postconditions

Use `after:` to declare conditions that must be true when a function returns. If any condition is false at the point of return, execution stops with a contract violation error.

**Syntax:** `after` is always a block. Each line inside the block is one boolean expression, checked in order.

```clean
after:
	<boolean_expression>
	<boolean_expression>
	...
```

**Example:**
```clean
functions:
	integer absoluteValue(integer n)
		after:
			result >= 0
		if n < 0
			return -n
		return n

	list<integer> sorted(list<integer> input)
		after:
			result.length() == input.length()
		// implementation
```

### CTR-02 — `after` postconditions

- Can only appear inside functions or class methods
- Must appear immediately after any `before:` block, before other statements
- The special identifier `result` refers to the function's return value inside an `after:` expression
- Every line inside the block is a boolean expression, checked in the order written — a non-boolean line is [`SEM023`](../03%20platform/09-error-codes.md#32-semantic-codes-sem)
- Checked on every return path (including implicit returns at end of body)
- **Strippable via `--strip-checks`** (see §6)

**Error on violation:** [`RUN011`](../03%20platform/09-error-codes.md#312-runtime-codes-run) `ContractViolation`.
```
Contract violation: after failed at absoluteValue:2
  Expression: result >= 0
```

---

## 3. `always` — Invariants

Use `always` to declare conditions that must hold across the entire lifetime of a class instance. Invariants are checked before and after every public method call on the instance, guaranteeing that no external interaction can leave the object in a broken state.

**Syntax:** `always:` is a block inside a class body, holding one boolean expression per line. It takes the same form as `before:` and `after:`, and sits after the fields, alongside the other contract blocks.

```clean
class ClassName
	// fields
	always:
		<boolean_expression>
		<boolean_expression>
	// constructor, methods
```

**Example:**
```clean
class BankAccount
	number balance
	string accountNumber

	always:
		balance >= 0
		accountNumber.length() == 10

	constructor(string accountNo, number initialDeposit)
		accountNumber = accountNo
		balance = initialDeposit

	void deposit(number amount)
		before:
			amount > 0
		balance = balance + amount

	void withdraw(number amount)
		before:
			amount > 0
			amount <= balance
		balance = balance - amount
```

### CTR-03 — `always` invariants

- Can only appear inside a `class` body, as an `always:` block
- Appears after the field declarations, so every field it references is already in scope for the reader
- A class has at most one `always:` block; it may hold any number of expressions, checked in the order written
- Every expression must be boolean — otherwise [`CLASS006`](../03%20platform/09-error-codes.md#35-class-codes-class)
- Checked at three points:
  - After the constructor completes (the object is born valid)
  - Before a public method body runs (the object is valid on entry)
  - After a public method body returns (the method left it valid)
- A fourth check point comes from the data library: [`Database.save`](../02%20components/framework/libraries/04-data.md) evaluates an entity's invariants **before** persisting, and a failure raises the same diagnostic without persisting anything
- Private/internal helpers do NOT trigger invariant checks (they run inside a public call that already did)
- **Strippable via `--strip-checks`** (see §6)

**Error on violation:** a failed invariant raises [`RUN011`](../03%20platform/09-error-codes.md#312-runtime-codes-run) `ContractViolation`, the same code an `after` failure raises. The message follows the template registered in [Platform 10](../03%20platform/10-semantic-rules.md):

```
Contract violation: always failed at BankAccount.withdraw
  Expression: balance >= 0
```

---

## 4. Contract Ordering Inside a Function

When a function uses both `before` and `after`, they must appear in this order at the top of the function body:

```clean
functions:
	integer clamp(integer value, integer low, integer high)
		before:
			low <= high         // preconditions first
		after:
			result >= low       // then postconditions
			result <= high
		if value < low
			return low
		if value > high
			return high
		return value
```

---

## 5. Design Notes

- Contracts are **not** replacements for input validation on untrusted data. Use `before` for internal assumptions between components; use explicit `if` / `error` for user-facing validation where a graceful message is expected.
- The `result` identifier is only in scope inside `after` expressions; using it elsewhere is [`CLASS008`](../03%20platform/09-error-codes.md#35-class-codes-class).
- Contracts must not have side effects: a contract expression that performs I/O, mutates state, or calls a function that itself carries contracts is [`CLASS009`](../03%20platform/09-error-codes.md#35-class-codes-class). Contracts describe truths about the program; they do not change it.

---

## 6. Runtime Cost and `--strip-checks`

Contracts execute at runtime, so they are not free, and the cost grows when a contract inspects a large data structure or sits on a hot path. The three keywords fall into two categories so the tradeoff is explicit.

### 6.1 `before` is always on

`before` guards the **caller's** contract with the function. A `before` that only fires in debug is a documentation lie: production code would silently accept inputs the function declares invalid, and the failure would surface later as memory corruption, wrong output, or a security hole. `before` is therefore always emitted and always checked.

The cost is bounded by construction: a `before` runs once per call and evaluates a boolean over the parameters.

### 6.2 `after` and `always` are strippable
`after` and `always` verify the **implementor's** work. They protect the developer during construction and testing; once the code has shipped and has been exercised, they add cost without adding safety the `before` clauses of downstream callers do not already provide.

Setting `strip_checks` causes the compiler to emit **no code** for `after` and `always`: their expressions are not evaluated, and no branch, trap, or diagnostic is generated. `before` is unaffected.

Stripping is requested in one of two places, and this chapter is the home of neither — it is the home of *what stripping does*:

- `[build] strip_checks = true` in `clean.toml` — schema owned by [Platform 07 — Build Configuration](../03%20platform/07-build-config.md).
- `cln build --strip-checks` — command surface owned by [Clean Manager](../02%20components/manager/00-manager.md).

**Stripping is a compile-time choice made per build, not per contract.** A stripped build strips every `after` and `always` in the compilation unit — there is no per-function opt-in and no per-contract override. This keeps builds predictable: a binary is either "checked" or "release-optimized," never a mix.

**Diagnostics vs runtime silence.** Stripping only affects code generation. All compile-time diagnostics — [`CLASS005`](../03%20platform/09-error-codes.md#35-class-codes-class) (`after` must precede logic), [`SEM023`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) (a `before:`/`after:` line must be boolean), [`CLASS006`](../03%20platform/09-error-codes.md#35-class-codes-class) (`always:` must be boolean), [`SEM001`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) on a mistyped contract expression — still fire in stripped builds. The compiler still parses, type-checks, and enforces the shape of every contract; it just does not emit the runtime guard.

**Default is off.** `strip_checks` defaults to `false`. Contracts are on by default; you opt into stripping when you build for production.

### 6.3 Cheap by construction

Even unstripped, a contract's cost stays bounded if two rules hold:

1. **Keep contracts O(1).** A contract that walks a collection is legal, but it turns a constant-cost guard into one proportional to the data. Where an invariant is over a whole structure, maintaining it incrementally in computed state ([20 — State Management](./20-state-management.md)) costs less than re-deriving it on every call.
2. **No contract call across the boundary.** A contract on function `f` may not call a function `g` that itself carries contracts. This keeps one call from expanding into a cascade of nested checks, and it is a compile-time rule with no opt-out.


## Changelog

- 2026-08-17 — The non-boolean case of a `before:`/`after:` line gets its owner: [`SEM023`](../03%20platform/09-error-codes.md#32-semantic-codes-sem), whose Platform 10 condition now names contract lines alongside `if`/`while` conditions ([CTR-01](#ctr-01--before-preconditions)/[CTR-02](#ctr-02--after-postconditions) make every contract line one boolean expression, so it *is* a condition and no new code is needed); `always:` keeps [`CLASS006`](../03%20platform/09-error-codes.md#35-class-codes-class). The chapter registered a code only for `always:`, leaving the other two blocks' boolean requirement uncoded — a [DIA-01](../03%20platform/13-diagnostic-format.md#dia-01--every-diagnostic-carries-a-registry-code) breach. §6's stripped-build diagnostics list now includes SEM023. Ratifies the compiler's Milestone 4 adoption (`clean-language-compiler/docs/DISCOVERIES-M4.md`, item 10).
- 2026-08-01 — Fase 5 (zero-debt pass): the side-effect prohibition cites [`CLASS009`](../03%20platform/09-error-codes.md#35-class-codes-class) and the `result` scope rule cites [`CLASS008`](../03%20platform/09-error-codes.md#35-class-codes-class) — both registered by this pass; §6.2 marked *Normative*; the unmeasurable "contracts should be cheap" restated as a bounded-cost condition.
- 2026-08-01 — Fase 3/4 (L7, L8, L20): **`always` is an `always:` block**, placed after the fields alongside `before:`/`after:` — the chapter defined a colon-less statement form placed *before* the fields while the registry, the data library and its own §6 all used the block form. The three runtime failures now cite their codes: [`RUN005`](../03%20platform/09-error-codes.md#312-runtime-codes-run) for `before`, [`RUN011`](../03%20platform/09-error-codes.md#312-runtime-codes-run) for `after` and `always` — codes minted on 2026-08-01 expressly to close this chapter's [SDD-04](../01%20governance/03-spec-driven-design.md) hole, whose remediation had reached the platform tree and never arrived here. A fourth evaluation point added: `Database.save` checks invariants before persisting. `strip_checks` given its homes — the schema key in [Platform 07](../03%20platform/07-build-config.md), the flag in [Manager](../02%20components/manager/00-manager.md) — and the rival `[compile] contract-safety` table retired; neither existed in any schema. WASM instruction counts removed (SDD-02). Rules `CTR-01`..`CTR-03` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users adding preconditions, postconditions, and invariants; anyone deciding when to enable `--strip-checks`
- **Rule prefix:** `CTR-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Platform 07 — Build Configuration](../03%20platform/07-build-config.md) (`strip_checks` key), [Clean Manager](../02%20components/manager/00-manager.md) (`cln build --strip-checks`), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md), [Platform 10 — Semantic Rules](../03%20platform/10-semantic-rules.md)
