# 04. Type System

Clean has a small, static type system: two numeric widths (`integer`, `number`), text (`string`), a byte buffer (`bytes`), a timestamp (`datetime`), a boolean, `void`, and a few generic containers (`list<T>`, `matrix<T>`, `pairs<K,V>`). Every variable, parameter, field, and return value has a type known at compile time — there is no runtime type check on ordinary code — and the `any` escape hatch exists for the few cases where a value's type genuinely cannot be known until it arrives (parsed JSON, a library return). This chapter defines every built-in type, the optional form `T?`, the behaviors a list can carry (`.line`, `.pile`, `.unique`), and the rules for conversions and string equality.

### TYP-01 — The core types and their ranges

| Type&nbsp;(keyword) | Description | Range / form | Literal Examples |
|---------------------|-------------|--------------|------------------|
| `boolean`  | Logical value (`true` / `false`) | — | `true`, `false` |
| `integer`  | Whole numbers, signed, 64-bit | `-9,223,372,036,854,775,808` … `9,223,372,036,854,775,807` | `42` |
| `number`   | Decimal numbers, 64-bit | IEEE-754 binary64 | `3.14`, `6.02e23` |
| `string`   | UTF-8 text, dynamically sized | — | `"Hello"` |
| `bytes`    | Raw byte buffer, dynamically sized | — | *(no literal — obtained from a bridge return)* |
| `datetime` | Timestamp with UTC offset | — | *(no literal — obtained from the standard library)* |
| `void`     | No value / empty return type | — | *(function return only)* |

**A range is measured after the sign is applied.** A numeric literal carries no sign of its own ([LEX-06](./03-lexical-structure.md#lex-06--literal-forms)): `-17` is unary minus applied to `17`. Whether a value fits its type is therefore decided on the result of that operation, not on the bare literal.

This is what makes the minimum above writable. A signed 64-bit range is asymmetric — the negatives get one more value than the non-negatives, because zero occupies a slot on the positive side — so the magnitude of the smallest `integer`, `9223372036854775808`, is one larger than the largest `integer` and does not fit on its own. Checking the literal alone would reject `-9223372036854775808`, a value this table declares valid. Checking the applied result accepts it.

A literal whose applied value does not fit its declared type is [`SEM026`](../03%20platform/09-error-codes.md#32-semantic-codes-sem).

Both numeric types are 64-bit. `integer` carries the full signed 64-bit range, so ordinary counting, identifiers and money-in-minor-units never silently wrap; `number` is IEEE-754 binary64. This is the width the host bridge already transports (`integer` maps to `s64` — see [Libraries Specification §8](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)), so a value that is representable in Clean is representable at the boundary.

One further built-in type is not listed above because it cannot be constructed: **`Error`**, the value a handler binds when an expression fails. It is specified in [13 — Error Handling](./13-error-handling.md#erh-04--the-error-binding-is-an-error-value), the home of the failure path.

The *in-memory representation* of each type is not part of this chapter — it is specified once in [Platform 03 — Memory Model](../03%20platform/03-memory-model.md), and the mapping across the host boundary once in [Libraries Specification §8](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations).

### Numeric widths
The surface language has exactly two numeric types: `integer`, signed 64-bit, and `number`, IEEE-754 binary64. **There is no width or signedness modifier on either.** `integer:8`, `integer:32u`, `number:32` and the rest are not Clean types, and a program that writes one is [`SEM009`](../03%20platform/09-error-codes.md#32-semantic-codes-sem).

Narrower widths exist in one place: a `host function` declaration, where a parameter or return type names the width the WIT interface on the other side actually uses ([Libraries Specification §8.3](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)). They are a property of the boundary, not of the language, and the compiler checks the value's range as it crosses.

A byte buffer is the `bytes` type, not a list of narrow integers.

**Why the language does not carry them:** a width modifier in the surface language obliges the specification to answer four further questions — the memory layout of a narrow numeric and therefore of `list<integer:8>`, the behaviour of a narrowing assignment at the boundary, the conversion lattice between widths, and a diagnostic for each. That is a sub-type-system, and the two places that ever used a modifier were both boundary declarations, neither of them Clean code a developer writes ([ADR-0019](../01%20governance/decisions/0019-precision-modifiers.md)).

### TYP-02 — Composite and generic types

| Type syntax | What it is | Example |
|-------------|------------|---------|
| `list<T>`  | Homogeneous resizable list (T is the element type) | `list<integer>`, `list<string>` |
| `matrix<T>` | 2-D list of lists (T is the element type) | `matrix<number>`, `[[1.0, 2.0], [3.0, 4.0]]` |
| `pairs<K,V>`  | Key-value associative container — Clean's map type; there is no separate `map<K,V>` | `pairs<string, integer>` |
| `any`         | Compile-time generic: compiler skips type checking for this value | Used when the type genuinely cannot be known at compile time (library returns, JSON, external data) |

`pairs<K,V>` takes any key type; `K` is a free type parameter, not fixed to `string`.

Lists are zero-indexed: `items[0]` is the first element. The method surface of `list<T>` — every operation available on a list value — is specified once in [15 — Standard Library §List Module](./15-standard-library.md), not here.

### TYP-03 — Optional types and `none`

An **optional type** is written `T?` and holds either a value of type `T` or the absence of one. The absence is the literal `none`.

```clean
string? nickname = none            // declared optional, currently absent
string? found = users.get("ada")   // an operation that may not produce a value
```

Rules:

- `T?` is the only way to express "may be absent". A plain `T` never holds `none`.
- A value of type `T` is assignable to a variable of type `T?`; the reverse is not. Assigning a `T?` to a `T` is a type error, because the absence has nowhere to go.
- `none` has no type of its own. It is assignable to any optional type and to nothing else, so `integer x = none` is a type error.
- Comparison against `none` (`value is none`) is how absence is tested.

Two operators consume optionals, and both are specified in [6 — Expressions](./06-expressions.md), which is their home: `default` supplies a fallback (`name default "anonymous"`), and `!` asserts non-absence and fails at runtime with [`RUN004`](../03%20platform/09-error-codes.md#312-runtime-codes-run) if the value is `none`. Indexing into a value that is provably `none` is [`IDX005`](../03%20platform/09-error-codes.md#36-index-access-codes-idx).

Across the host boundary, `T?` maps to `option<T>` ([Libraries Specification §8](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)).

**`T?` is a type, and composes like one.** It may appear anywhere a type may: a variable declaration, a parameter, a return type, a field, and a type argument. `list<string?>` is a list of optional strings; `list<string>?` is an optional list. They are different types and the distinction is preserved across the host boundary, where both map onto `option<T>` ([Libraries Specification §8.3](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)).

**Absence does not stack.** Writing `T??` is [`SEM009`](../03%20platform/09-error-codes.md#32-semantic-codes-sem). Where instantiating a generic *would* produce an optional of an optional — a lookup returning `V?` over a `pairs<string, integer?>` — the result is `integer?`, not `integer??`: the two levels collapse into one. A value is absent or it is not, and the language has one way to say so.

The collapse is a deliberate loss. It makes "no such key" and "the key holds an absence" the same answer, which a nesting type would distinguish. The cost of keeping them apart is that every reader of every optional has to ask how deep it goes, and every `default` and `!` has to be specified against nesting — paid on every optional in the language to serve a case that a program can express with its own type when it needs to.

`T?` is a distinct type from `T`, not a modifier on it. A capability method declaring `string` is not satisfied by one declaring `string?` ([CLS-03](./14-classes-and-objects.md#cls-03--capabilities-are-contracts-without-bodies) is nominal).

### TYP-04 — Compile-time types

The following types exist only during compilation and are passed to and returned from `compiletime` functions. They cannot be declared, constructed, or stored by ordinary runtime code. Their fields and contracts are specified in [21 — Block Handlers](./21-block-handlers.md), which is their home; they are listed here so the type system is complete.

| Type | What it is | Defined in |
|------|-----------|------------|
| `BlockAST` | The parsed block a handler receives | [21 §21.3](./21-block-handlers.md#213-the-blockast-type) |
| `BlockNode`, `BlockArg`, `BlockAttribute`, `BlockLine`, `Token` | The constituent parts of a `BlockAST` | [21 §21.3](./21-block-handlers.md#213-the-blockast-type) |
| `IR` | The typed fragment a handler returns; opaque to the handler | [21 §21.4](./21-block-handlers.md#214-the-ir-return-type) |
| `Span` | A source location — file, line, column, byte range | [21 §21.3](./21-block-handlers.md#213-the-blockast-type) |
| `Diagnostic` | A diagnostic a handler emits | [21 §21.6](./21-block-handlers.md#216-diagnostics-from-compile-time-functions) |

### TYP-05 — List behaviors

**A behavior is part of the list's type, fixed at declaration.** `list<string>` and `list<string>.line` are different types, and a value of one is not assignable to the other. The behavior is not a runtime property and cannot be changed after declaration: it is what tells the compiler which element `remove()` acts on, so a list that could change behavior would be a list whose `remove()` has no defined meaning.

**Behaviors occupy two independent axes.**

| Axis | Suffixes | What it fixes |
|------|----------|---------------|
| **Removal discipline** | `.line`, `.pile` | Which end `add`, `remove` and `peek` act on. The two are **mutually exclusive** — a list has one end it removes from, or none. |
| **Membership** | `.unique` | Whether a duplicate addition is stored. Independent of any removal discipline. |

One suffix from each axis may be combined, in either order. The legal forms are therefore:

```
.line          .pile          .unique
.line.unique   .pile.unique
```

`.line.pile` and `.line.unique.pile` name two removal disciplines at once and are [`SEM009`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) — they are not a type. A list cannot remove from the front and from the top.

#### Behavior Type Syntax

The type is declared inline in the variable declaration using dot notation:

```clean
list<integer>.line numbers = []          // FIFO queue of integers
list<string>.unique visitors = []        // Set of strings (no duplicates)
list<string>.line.unique taskQueue = []  // FIFO queue with uniqueness
```

The behavior suffixes are:
- `.line` — FIFO queue behavior
- `.pile` — LIFO stack behavior
- `.unique` — set behavior (no duplicates)
- `.line.unique` — FIFO queue with uniqueness
- `.pile.unique` — LIFO stack with uniqueness

#### Supported Properties

**`.line` — Queue Behavior (FIFO)**

**First-In-First-Out behavior: elements are added to the back and removed from the front.**

```clean
functions:
	void processTaskQueue()
		list<string>.line tasks = []

		// Add tasks (to back)
		tasks.add("Task 1")
		tasks.add("Task 2")
		tasks.add("Task 3")

		// Process tasks (from front)
		iterate i in 1 to 3
			string currentTask = tasks.remove()  // Gets "Task 1", then "Task 2", etc.
			print("Processing: " + currentTask)
```

**Modified Operations**:
- `add(item)` → Adds to the **back** of the list
- `remove()` → Removes from the **front** of the list
- `peek()` → Views the **front** element without removing
- Every other list operation ([15 §List Module](./15-standard-library.md)) is unaffected by the behavior

**`.pile` — Stack Behavior (LIFO)**

**Last-In-First-Out behavior: elements are added and removed from the same end (top).**

```clean
functions:
	void undoSystem()
		list<string>.pile actions = []

		// Perform actions (add to top)
		actions.add("Create file")
		actions.add("Edit text")
		actions.add("Save file")

		// Undo actions (remove from top)
		iterate i in 1 to 3
			string lastAction = actions.remove()  // Gets "Save file", then "Edit text", etc.
			print("Undoing: " + lastAction)
```

**Modified Operations**:
- `add(item)` → Adds to the **top** of the list
- `remove()` → Removes from the **top** of the list
- `peek()` → Views the **top** element without removing
- Every other list operation ([15 §List Module](./15-standard-library.md)) is unaffected by the behavior

**`.unique` — Set Behavior (Uniqueness Constraint)**

**Only unique elements are stored; duplicate additions are silently ignored.**

```clean
functions:
	void trackUniqueVisitors()
		list<string>.unique visitors = []

		// Add visitors (duplicates ignored)
		visitors.add("Alice")    // Added
		visitors.add("Bob")      // Added
		visitors.add("Alice")    // Ignored (duplicate)
		visitors.add("Charlie")  // Added

		print("Unique visitors: " + visitors.length().toString())  // Prints: 3

		if visitors.contains("Alice")
			print("Alice has visited")
```

**Modified Operations**:
- `add(item)` → Adds only if `item` is not already present; adding an item the list already holds leaves the list unchanged and is not an error
- `contains(item)` → Membership test
- `.unique` constrains membership only. It does not by itself define a removal end, so `remove()` and `peek()` are available only when the declaration also carries `.line` or `.pile`
- Every other list operation ([15 §List Module](./15-standard-library.md)) is unaffected by the behavior

#### Behavior Combinations

**Behaviors combine by chaining dot suffixes on the type declaration.**

```clean
// Unique queue — FIFO with no duplicates
list<string>.line.unique uniqueQueue = []

// Unique stack — LIFO with no duplicates
list<integer>.pile.unique uniqueStack = []
```

The two removal disciplines do not combine: `list<T>.line.pile` and `list<T>.line.unique.pile` name both a front and a top to remove from, and are [`SEM009`](../03%20platform/09-error-codes.md#32-semantic-codes-sem).

#### Method Surface

Every list, whatever its behavior, exposes one method surface, specified once in [15 — Standard Library §List Module](./15-standard-library.md). This chapter does not restate it.

What the behavior changes is *which element* the position-dependent operations act on:

| Operation | `.line` (FIFO) | `.pile` (LIFO) | `.unique` |
|-----------|----------------|----------------|-----------|
| `add(item)` | appends to the back | pushes onto the top | appends; an item already present is not added again |
| `remove()`, `peek()` | act on the front | act on the top | not available — `.unique` defines membership, not a removal end |

**Behavior declaration:** the behavior is written as part of the type at variable declaration time — `list<T>.line`, `list<T>.pile`, `list<T>.unique`. Because it is static, nothing about a behavior is stored in the value: the compiler already knows which end to act on and emits the operation directly, so [Platform 03 — Memory Model](../03%20platform/03-memory-model.md)'s layout for `list<T>` needs no field for it.

#### Complete Example

```clean
start:
	// Test line behavior (FIFO queue)
	list<integer>.line lineList = []
	lineList.add(1)
	lineList.add(2)
	lineList.add(3)

	integer first = lineList.remove()   // Returns 1 (first in, first out)
	integer second = lineList.remove()  // Returns 2

	// Pile behavior (LIFO stack)
	list<integer>.pile pileList = []
	pileList.add(10)
	pileList.add(20)
	pileList.add(30)

	integer top = pileList.remove()     // Returns 30 (last in, first out)

	// Unique behavior (set)
	list<integer>.unique uniqueList = []
	uniqueList.add(100)
	uniqueList.add(200)
	uniqueList.add(100)  // Ignored (duplicate)

	boolean hasHundred = uniqueList.contains(100)  // Returns true
	integer listSize = uniqueList.length()            // Returns 2 (no duplicates)

	print("List demonstrates flexible behavior via type declaration")
```

### Type Annotations and Variable Declaration

Variables use **type-first** syntax:

```clean
// Basic variable declarations
integer count = 0
number temperature = 23.5
boolean isActive = true
string name = "Alice"

// Uninitialized variables
integer sum
string message
```

### TYP-06 — Type conversion

**Implicit conversion.** `integer` → `number` is the only implicit conversion. It is not lossless: `number` is binary64, which has 53 bits of significand, so an `integer` whose magnitude exceeds 2⁵³ loses precision. The conversion is permitted and carries the compile-time warning [`SEM027`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) when the converted value is compile-time evaluable and its magnitude provably exceeds that bound; a value the compiler cannot evaluate never warns.

Every other conversion is explicit.

**Explicit conversions (all require parentheses):**
```clean
value.toInteger()   // convert to integer
value.toNumber()    // convert to number
value.toString()    // convert to string
value.toBoolean()   // convert to boolean
```

These four are methods on the value, per [16 — Method-Style Syntax](./16-method-style-syntax.md); their exact per-type behaviour is catalogued in [15 — Standard Library §Conversions](./15-standard-library.md).

**Examples:**
```clean
integer num = 42
number numFloat = num.toNumber()      // ✅ Works: converts 42 to 42.0
integer piInt = 3.14.toInteger()      // ✅ Works: converts 3.14 to 3 (truncated)
boolean flag = 0.toBoolean()          // ✅ Works: converts 0 to false
boolean nonZero = 5.toBoolean()       // ✅ Works: converts 5 to true
```

### TYP-07 — String equality is byte-exact; nothing normalizes

Two `string` values are equal iff their UTF-8 payloads are identical byte for byte. **No normalization is performed anywhere** — not on a literal at compile time, not on a comparison at runtime, not at the host boundary. What the source file contains is what reaches memory, unchanged.

The consequence is observable and MUST be documented wherever it can surprise: Unicode can express the same visible text with different byte sequences, so two strings a reader sees as identical may compare unequal.

```clean
// Both display as "café". The first spells é as one character,
// the second as "e" followed by a combining accent.
string a = "caf\u0000E9"
string b = "cafe\u000301"
a == b        // false — different bytes, therefore different strings
a.length()    // also differs: b carries one character more
```

The alternatives were rejected deliberately. Normalizing literals at compile time would mean the compiler silently rewrites a value the developer wrote, so a string leaves the source file different from how it entered it. Making `==` normalize would put that cost on every string comparison in the language and on every crossing of the host bridge, to serve a case that appears rarely. Byte-exact equality is also what the representation already implies ([Platform 03 — Memory Model](../03%20platform/03-memory-model.md)); the other two options would have required changing it.

A program that genuinely needs canonical comparison performs it explicitly. The bridge reserves heavyweight text operations, normalization among them, for exactly this ([Platform 02 — Host Bridge](../03%20platform/02-host-bridge.md)).

**Check:** a literal's bytes in the compiled artifact are identical to its bytes in the source file, and `==` on two strings yields `true` only when their payloads match byte for byte.

## Changelog

- 2026-08-17 — TYP-06's lossy-promotion warning gets its code: [`SEM027`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) `LossyIntegerPromotion`, registered per [ERC-03](../03%20platform/09-error-codes.md#erc-03--registration-process). As written the warning could not be emitted at all — [DIA-01](../03%20platform/13-diagnostic-format.md#dia-01--every-diagnostic-carries-a-registry-code) forbids a diagnostic without a registered code and Platform 09 had none — and its trigger ("may exceed") named no decidable check. The condition is now decidable: the warning fires when the converted value is compile-time evaluable and provably exceeds 2⁵³, never on unevaluable values. Found by the compiler's Milestone 4 (`clean-language-compiler/docs/DISCOVERIES-M4.md`, item 1).
- 2026-08-02 — The residual ADR-0013 open banner in §Behavior Combinations replaced by the decided rule. It still said `.line.pile` was unspecified and that the chapter asserted both readings of whether a behavior is part of the type; both were settled earlier the same day and the banner had been left behind.
- 2026-08-02 — [ADR-0013](../01%20governance/decisions/0013-composed-list-behaviors.md) closed in TYP-05, on the **orthogonal-axes** reading: `.line` and `.pile` are removal disciplines and mutually exclusive, `.unique` is a membership constraint independent of both. `.line.pile` and `.line.unique.pile` are withdrawn from the canonical list and are [`SEM009`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) — they had been documented as "FIFO + LIFO combined" and "all three behaviors", which cannot be given a meaning since `remove()` would have two answers. The chapter's opening claim that a behavior changes a list "without changing its type" is replaced by its opposite: the behavior **is** part of the type and is fixed at declaration, which is the only reading under which `remove()` is checkable. Because it is static, no memory-model field is needed for it.
- 2026-08-02 — [ADR-0015](../01%20governance/decisions/0015-optional-type-first-class.md) closed in TYP-03: `T?` is a first-class type usable in every type position, and absence does not nest — `T??` written in source is [`SEM009`](../03%20platform/09-error-codes.md#32-semantic-codes-sem), and an instantiation that would produce one collapses to `T?`. `list<string?>` and `list<string>?` are distinct, which the WIT boundary already required since it maps both onto `option<T>`. `T?` is a distinct type rather than a modifier, so capability conformance treats them as different signatures.
- 2026-08-02 — TYP-01 points at `Error` as a built-in type specified in [13 — Error Handling](./13-error-handling.md#erh-04--the-error-binding-is-an-error-value), per [ADR-0016](../01%20governance/decisions/0016-error-value-or-signal.md). It is named here and defined there rather than added to the core table: it has no literal and no constructor, and the failure path is chapter 13's.
- 2026-08-02 — §Precision Control withdrawn and replaced by **§Numeric widths**, closing [ADR-0019](../01%20governance/decisions/0019-precision-modifiers.md). Roughly seventy-five lines described a sub-type-system of seven widths that no Clean code in the repository used — its only two consumers were the Clean-to-WIT mapping table and one `host function` parameter, both boundary declarations. Narrow widths now live there explicitly and nowhere else; a byte buffer is the `bytes` type. The stale claim that `integer:32` equals the standard integer goes with the section, as does the open banner. TYP-01's deferred out-of-range diagnostic is now the registered [`SEM026`](../03%20platform/09-error-codes.md#32-semantic-codes-sem).
- 2026-08-02 — TYP-01 now states that a type's range is measured **after** unary minus is applied, not on the bare literal, closing the type-system half of question 11 of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md). Without it the `integer` minimum this table declares would be unwritable, since a signed range is asymmetric and the minimum's magnitude exceeds the maximum. `-17` removed from the `integer` literal examples: it is two tokens, not a literal ([LEX-06](./03-lexical-structure.md#lex-06--literal-forms)). The missing out-of-range diagnostic is marked as [ADR-0019](../01%20governance/decisions/0019-precision-modifiers.md)'s open item rather than invented here.
- 2026-08-02 — `TYP-07` minted, closing question 3 (normalization) of [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md). String equality is byte-exact and nothing normalizes at any stage. The fact had no home: [Platform 03](../03%20platform/03-memory-model.md) said comparison uses "(length, payload)" without saying whether the payload comparison normalizes, and no chapter stated the NFC/NFD consequence a developer actually meets. Representation stays in Platform 03, which now cites this rule for the observable semantics.
- 2026-08-01 — Fase 3/4 (L5, L6, L16): **`integer` is 64-bit** — the language said 32-bit while the host bridge transported `s64`. New §Optional Types and `none`: `T?` had four dependents across three trees and no home. New §Compile-Time Types registering the types [21](./21-block-handlers.md) passes to handlers. The duplicated `list` method table removed — [15 §List Module](./15-standard-library.md) is the single home — and `size()`/`peek()` reconciled with the names it actually defines. `time.*` and `bytes.*` removed as constructors: they are defined nowhere ([ADR-0021](../01%20governance/decisions/0021-time-and-bytes-namespaces.md)). Memory-layout and complexity claims removed (SDD-02: the home is [Platform 03](../03%20platform/03-memory-model.md)). Composed list behaviors and precision modifiers marked open ([ADR-0013](../01%20governance/decisions/0013-composed-list-behaviors.md), [ADR-0019](../01%20governance/decisions/0019-precision-modifiers.md)). Rules `TYP-01`..`TYP-06` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users learning types, optionals, and list behaviors; compiler and library authors implementing type-checking
- **Rule prefix:** `TYP-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Lexical Structure](./03-lexical-structure.md) (numeric literal forms), [Standard Library](./15-standard-library.md) (per-type method surfaces), [Platform 03 — Memory Model](../03%20platform/03-memory-model.md), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md)
