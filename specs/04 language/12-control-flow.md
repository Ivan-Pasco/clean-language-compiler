# 12. Control Flow

Clean has three control-flow constructs: `if` / `else if` / `else` for conditionals, `iterate` for both list iteration and range loops, and `while` for open-ended looping. `break` and `continue` are statements that bind to the innermost enclosing loop within the same body. This chapter defines each of these; there is no `for` (it is reserved for a future release), no `switch`, and no labelled form of `break` or `continue`.

### FLW-01 — Conditional statements

```clean
// Basic if statement
if condition
	// statements

// If-else
if condition
	statements
else
	statements

// If-else if chain
if condition1
	statements
else if condition2
	statements
else
	statements
```

### FLW-02 — Loops

#### The `iterate` Loop

```clean
// Iterate over list elements
iterate item in list
	print(item)

// Iterate over string characters
iterate char in "hello"
	print(char)
```

#### Range-based Loops

```clean
iterate name in source [step n]
	// body

// Examples:
iterate i in 1 to 10
	print(i)

iterate k in 10 to 1 step -2
	print(k)                 // 10, 8, 6, 4, 2

iterate ch in "Clean"
	print(ch)

iterate row in matrix
	iterate value in row
		print(value)

iterate idx in 0 to 100 step 5
	print(idx)               // 0, 5, 10, …, 100
```

**Range semantics** *(Normative)*:

- **Both endpoints are inclusive when reached.** `from to to` visits `from`, then `from + step`, and so on while the value has not passed `to`; `to` itself is visited exactly when some `from + k·step` equals it (`1 to 10` ends at 10; `0 to 100 step 3` ends at 99).
- **The default step is directional.** With no `step` clause, the step is `1` when `from ≤ to` and `-1` when `from > to` — `iterate i in 5 to 1` visits 5, 4, 3, 2, 1, mirroring [`list.range(5, 1)`](./15-standard-library.md#list-module). A range never silently runs zero iterations because of its direction; `from == to` visits the one value once.
- **`from`, `to`, and `step` are evaluated exactly once each**, in that order, before the first iteration. Reassigning a variable they mentioned inside the body does not change the loop's bounds or stride.
- **`step 0`** is under decision — see the open brief [`work/2026-08-17-iterate-step-non-range.md`](../work/2026-08-17-iterate-step-non-range.md); until it closes, a program MUST NOT rely on any behaviour for it.

#### While Loop
The `while` loop executes a block of code repeatedly as long as a condition remains true. This is useful when you don't know in advance how many iterations are needed.

**Syntax:**
```clean
while condition
	// body - executed while condition is true
```

**Examples:**

```clean
// Basic counter loop
integer count = 0
while count < 5
	print(count.toString())
	count = count + 1
// Prints: 0, 1, 2, 3, 4

// Loop with boolean condition
boolean running = true
integer iterations = 0
while running
	iterations = iterations + 1
	if iterations >= 3
		running = false
// Stops after 3 iterations

// Nested while loops
integer outer = 0
while outer < 3
	integer inner = 0
	while inner < 2
		print("outer: " + outer.toString() + ", inner: " + inner.toString())
		inner = inner + 1
	outer = outer + 1

// While loop with if statement inside
integer i = 0
while i < 10
	integer remainder = i % 2
	if remainder == 0
		print("Even: " + i.toString())
	else
		print("Odd: " + i.toString())
	i = i + 1
```

**Rules:**
- The condition must evaluate to a boolean value
- The body is indented one level deeper than the `while` keyword
- A variable assigned in the loop body keeps its value into the next iteration; the loop introduces no new binding per pass
- Infinite loops occur if the condition never becomes false (ensure loop variables are updated)

**Important Notes:**
- Early exit and iteration skipping are [FLW-03](#flw-03--break-and-continue)
- The while loop is useful for input validation, processing until a condition is met, or when the number of iterations is unknown

### FLW-03 — `break` and `continue`

Both are **statements**. Each stands alone on its own line, takes no operand, carries no condition and no label, and produces no value.

- `break` ends the innermost enclosing loop at once; execution resumes after that loop.
- `continue` ends the current iteration of the innermost enclosing loop. The loop then proceeds exactly as it would have had the body finished normally — in an `iterate`, the next item is bound and any `step` applies unchanged, so a skipped iteration consumes its item like any other.

```clean
iterate item in items
	if item.isEmpty()
		continue          // next item, step applied as usual
	if item == "stop"
		break             // leaves this iterate
	print(item)
```

**The innermost loop, within the same body.** Both bind to the nearest enclosing `iterate` or `while` in the body they appear in. A function body, a contract block (`before`, `after`, `always`) and a `compiletime function` body each begin a new body, and neither statement crosses one: a `break` inside a function called from a loop ends nothing in the caller. Either statement with no enclosing loop in its own body is [`SEM025`](../03%20platform/09-error-codes.md#32-semantic-codes-sem).

**There is no labelled form.** To leave a loop from inside a nested one, extract the inner loop into a function and `return` from it. Labels would introduce a naming form and a scope that exist nowhere else in the language, for a case that already has an answer ([LDR-08](./02-language-design-rules.md#ldr-08--one-way-to-do-things)).

**Why the keywords exist at all:** without them, a loop that must stop early is written with a flag variable tested in its own condition — the idiom `break` exists to remove. Extracting a function is the alternative only when the body is extractable, and a body that updates several local variables is not.

**Check:** a `continue` inside a nested `iterate` advances the inner loop and leaves the outer one running; a `break` in a function called from a loop reports `SEM025`.

## Changelog

- 2026-08-19 — Erratum from the compiler's Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, item 7): FLW-02's range loops gain the normative sentences the worked examples only implied — inclusive endpoints when reached, directional default step (`-1` when `from > to`, mirroring `list.range(5, 1)`), and exactly-once evaluation of `from`/`to`/`step` before the first iteration. Ratifies the compiler's adoptions (a) and (b) verbatim. `step 0` on a range is *not* decided here: it joins the still-open [`work/2026-08-17-iterate-step-non-range.md`](../work/2026-08-17-iterate-step-non-range.md) brief, since its natural semantics never terminate and no code exists to reject it.
- 2026-08-02 — `FLW-03` minted, closing [ADR-0017](../01%20governance/decisions/0017-break-and-continue.md). `break` and `continue` were reserved hard keywords with a single descriptive sentence in the whole language tree and no grammar, binding rule or diagnostic. They are statements binding to the innermost enclosing loop within their own body, with no labelled form; `continue` advances an `iterate` exactly as a normally-finished body does, `step` included; either without an enclosing loop is the new [`SEM025`](../03%20platform/09-error-codes.md#32-semantic-codes-sem). The §While note that carried the original sentence now cites the rule instead of restating it.
- 2026-08-01 — Fase 5 (zero-debt pass): the `while` section marked *Normative*; "variables are properly updated" restated as the observable binding rule; "(for-each)" dropped from the `iterate` heading — the glossary term is `iterate`.
- 2026-08-01 — Fase 4: rules `FLW-01`, `FLW-02` minted. `break` and `continue` — one sentence in the whole tree, with no grammar, scope or diagnostic — recorded as open in [ADR-0017](../01%20governance/decisions/0017-break-and-continue.md).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users writing conditionals and loops
- **Rule prefix:** `FLW-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Expressions](./06-expressions.md) (boolean operators), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md) (`SEM025`)
