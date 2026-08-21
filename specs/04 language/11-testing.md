# 11. Testing

Clean has a built-in testing framework: put a `tests:` block in any source file, list your assertions with `==`, and run them with `cln test`. Tests can be single-line named, single-line anonymous, or multi-statement blocks introduced by `assert`. This chapter defines those three forms, what `cln test` does with them, and where the conformance regime for standard-library parsers (which is a quality regime, not a language feature) actually lives.

Clean Language includes a built-in testing framework with a simple and readable syntax. Tests can be embedded directly in your source code using the `tests:` block.

### Test Block Syntax

Tests are defined within a `tests:` block and can be either named or anonymous:

```clean
tests:
	// Named tests with descriptions
	"adds numbers": add(2, 3) == 5
	"squares a number": square(4) == 16
	"detects empty string": "".isEmpty() == true
	
	// Anonymous tests (no description)
	"hi".toUpperCase() == "HI"
	math.absInteger(-42) == 42
	[1, 2, 3].length() == 3
```

### TST-01 — Test forms

1. **Named Tests**: `"description": expression == expected`
   - The description is a string literal used as the label in test output
   - The colon (`:`) separates the description from the test expression
   - The comparison uses `==`, the equality operator ([6 — Expressions](./06-expressions.md)). `=` is assignment and is not valid here

2. **Anonymous Tests**: `expression == expected`
   - No description — the expression itself serves as documentation
   - Simpler form for obvious cases

3. **Block Tests**: for a test that needs more than one line before it can assert, the description is followed by an indented body and the assertion is written with `assert`:

   ```clean
   tests:
   	"expandDataBlock emits a class with the expected fields"
   		BlockAST input = test.compiletime.parseBlock(sourceText)
   		IR output = expandDataBlock(input)
   		assert test.compiletime.classFieldNames(output, "UserData") == ["id", "email"]
   ```

   The description carries **no colon** in this form — the colon is what marks the single-line form. A block test may contain any number of statements and any number of `assert` lines; the test passes when every `assert` holds. `assert` takes one boolean expression and is a hard keyword ([3 — Lexical Structure](./03-lexical-structure.md)).

4. **Test Expressions**: any valid Clean expression
   - Function calls: `add(2, 3)`
   - Method calls: `"".isEmpty()`
   - Complex expressions: `(x + y) * 2`
   - Object creation and method chaining: `Point(3, 4).distanceFromOrigin()`

5. **Expected Values**: the right side of `==` is the expected result
   - Must be a compile-time evaluable expression or literal
   - Its type must match the test expression's type

What the checker enforces about a test's shape is that the assertion expression is **boolean** — a non-boolean assertion is [`SEM023`](../03%20platform/09-error-codes.md#32-semantic-codes-sem). The comparison forms above are the canonical spellings of that boolean, not a separately coded rule: no diagnostic exists (or is needed) for "top-level operator is not a comparison", because a boolean expression that is not a comparison — `"is empty": list.isEmpty()` — is a legitimate test.

### TST-02 — `cln test` runs the test blocks

`cln test` runs a project's test blocks. It is the only command that does so — the user never invokes a component binary directly ([C-03](../01%20governance/05-concerns.md)), and the command surface is owned by [Clean Manager](../02%20components/manager/00-manager.md).

```bash
cln test
```

`tests:` blocks are excluded from a normal `cln build`; they are compiled only for a test run.

### Test Output Format

The test runner provides clear, readable output:

```
Running tests for myprogram.cln...

✅ adds numbers: add(2, 3) == 5 (PASS)
✅ squares a number: square(4) == 16 (PASS)
❌ detects empty string: "".isEmpty() == true (FAIL: expected true, got false)
✅ "hi".toUpperCase() == "HI" (PASS)

Test Results: 3 passed, 1 failed, 4 total
```

### Advanced Testing Features

#### Testing Functions with Error Handling

```clean
functions:
	integer safeDivide(integer a, integer b)
		if b == 0
			error("Division by zero")
		return a / b

tests:
	"normal division": safeDivide(10, 2) == 5
	"division by zero": (safeDivide(10, 0) onError error.message) == "Division by zero"
```

The failure case is tested through the handler, not by comparing against an error. `error(...)` is a signal and never a value ([ERH-01](./13-error-handling.md#erh-01--raising-an-error)), so there is nothing to compare a call against; what a handler binds is an `Error` whose fields can be asserted on ([ERH-04](./13-error-handling.md#erh-04--the-error-binding-is-an-error-value)). `onError` binds `error` inside the fallback expression, so the assertion needs no test-only syntax.

#### Testing Object Methods

```clean
class Calculator
	integer value
	
	constructor(integer initialValue)
		value = initialValue
	
	functions:
		integer add(integer x)
			value = value + x
			return value

tests:
	"calculator addition": Calculator(10).add(5) == 15
```

#### Testing List and String Operations

```clean
tests:
	"list operations": [1, 2, 3].length() == 3
	"list contains": [1, 2, 3].contains(2) == true
	"string operations": "hello".toUpperCase() == "HELLO"
	"string indexing": "world".indexOf("r") == 2
```

### Best Practices

1. **Descriptive Test Names**: Use clear, descriptive names for complex tests
   ```clean
   tests:
   	"calculates compound interest correctly": calculateCompoundInterest(1000, 0.05, 2) == 1102.5
   ```

2. **Test Edge Cases**: Include tests for boundary conditions
   ```clean
   tests:
   	"handles empty list": [].length() == 0
   	"handles single character": "a".toUpperCase() == "A"
   	"handles zero input": factorial(0) == 1
   ```

3. **Group Related Tests**: Organize tests logically within the `tests:` block
   ```clean
   tests:
   	// Basic arithmetic
   	"addition": add(2, 3) == 5
   	"subtraction": subtract(5, 2) == 3
   	
   	// String operations  
   	"uppercase conversion": "hello".toUpperCase() == "HELLO"
   	"lowercase conversion": "WORLD".toLowerCase() == "world"
   ```

4. **Cover the boundaries, not only the centre**: the interesting cases are the empty input, the maximum, and the one-past-the-end
   ```clean
   tests:
   	"valid input": processInput("valid") == "processed: valid"
   	"trims surrounding space": processInput("  ok  ") == "processed: ok"
   ```

### Conformance Testing for Standard-Library Parsers

A `tests:` block covers user code and hand-written standard-library cases. It is not enough on its own for a module that parses an external, standardised format — `json` today, and any format module added later. Those need to be checked against the whole surface of the format, not only the cases their author thought of.

That is a **quality regime**, not a language feature: it prescribes CI configuration, a vendored third-party corpus, and test layout inside a component repository. It therefore lives at a different rung of the ladder ([DOC-07](../01%20governance/00-documentation-principles.md)) and in a different repository ([EXE-04](../01%20governance/04-execution-model.md)) from this chapter. Its home is the [Quality Playbook](../01%20governance/02-quality-playbook.md).

What belongs to the language, and is specified here and in [15 — Standard Library](./15-standard-library.md), is which diagnostic a parser raises for each class of malformed input — `RUN006`–`RUN010` for `json` ([15 §JSON Module](./15-standard-library.md)).

Where the format leaves a case open, the resolution lives in the condition of the rule that rejects it — `RUN007`, `RUN009` and `RUN010` in [Platform 10](../03%20platform/10-semantic-rules.md). A conformance corpus asserts against those conditions; there is no separate decisions document to keep in step with them ([ADR-0010](../01%20governance/decisions/0010-implementation-defined-parser-decisions.md)).

## Changelog

- 2026-08-17 — TST-01 clarified: what the checker enforces is that the assertion expression is boolean ([`SEM023`](../03%20platform/09-error-codes.md#32-semantic-codes-sem)); the comparison forms are the canonical spellings, not a separately coded shape rule — no code exists for "top-level operator is not a comparison" and none is registered, since a non-comparison boolean (`list.isEmpty()`) is a legitimate test ([DIA-01](../03%20platform/13-diagnostic-format.md#dia-01--every-diagnostic-carries-a-registry-code) forbids an uncoded requirement). Ratifies the compiler's Milestone 4 adoption (`clean-language-compiler/docs/DISCOVERIES-M4.md`, item 19).
- 2026-08-02 — The **implementation-defined decisions document** is retired as a concept, closing [ADR-0010](../01%20governance/decisions/0010-implementation-defined-parser-decisions.md). This chapter had named the path `foundation/spec/stdlib/json/implementation-defined.md`, which did not exist and neither did the directory above it, while three registered diagnostics delegated their accept/reject boundary to it. The boundary is now stated where it belongs, in each rule's condition.
- 2026-08-02 — The failure case of `safeDivide` is a worked test again, closing the gap [ADR-0016](../01%20governance/decisions/0016-error-value-or-signal.md) left. The chapter had carried `safeDivide(10, 0) = error("Division by zero")`, which presumed an error was a value; with `error(...)` settled as a signal, the failure is asserted through the handler binding instead, using only rules [13](./13-error-handling.md) already defines.
- 2026-08-01 — Fase 5 (zero-debt pass): Best Practices marked *Informative*.
- 2026-08-01 — Fase 3/4 (L12, L23): assertions use **`==`**, not `=` — the chapter compared with the assignment operator. A **block form** added with `assert`, for tests needing statements before they can assert ([21 §21.9](./21-block-handlers.md) used it and no chapter defined it). `cleanc --test` replaced by `cln test`: `cleanc` appeared twice in the entire repository and contradicted [C-03](../01%20governance/05-concerns.md). The four-layer conformance regime moved to its rung — [Quality Playbook §1.9](../01%20governance/02-quality-playbook.md) — since it prescribed CI configuration and test paths in another repository ([DOC-07](../01%20governance/00-documentation-principles.md), [EXE-04](../01%20governance/04-execution-model.md)); the pinned decisions document it depends on does not exist and is now [ADR-0010](../01%20governance/decisions/0010-implementation-defined-parser-decisions.md). Rules `TST-01`, `TST-02` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users writing `tests:` blocks; anyone using `cln test`
- **Rule prefix:** `TST-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Error Handling](./13-error-handling.md) (asserting failure cases), [Standard Library — JSON Module](./15-standard-library.md) (parser diagnostics), [Clean Manager](../02%20components/manager/00-manager.md) (`cln test`), [Quality Playbook](../01%20governance/02-quality-playbook.md) (conformance regime)
