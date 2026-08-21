# Platform 11. Standard Library — Validator Module

The `validator` module provides declarative, field-level input validation for the places where a program meets the outside world — form submissions, API request bodies, configuration files, user input. It defines one DSL for declaring field constraints in a readable indented style (`validator.create:`), one result type (`ValidationResult`) that forces callers to handle the failure case, and one input normalization rule. It is the single source of truth for the `validator` namespace: no exceptions are thrown, no alternate result shape exists, and no general regex surface is offered — named pattern checks defer to `string.matches`, and this module's `match:` rules use their own bounded glob micro-syntax.

---

## 1. Overview


The `validator` module provides declarative, field-level input validation with a Clean Language native DSL (domain-specific language — here, the `validator.create:` block form). It is designed for validating structured data at system boundaries: form submissions, API request bodies, configuration files, and user input.

The design follows Clean Language's one-way-to-do-things principle: one syntax for defining rules, one type for results, one way to inspect failures. No exceptions are thrown, and exactly one input normalization exists ([VAL-03](#val-03--single-input-normalization)).

**What it is:**
- A rules engine for structured data validation
- A DSL for declaring field constraints in a readable, indented style
- A result type that forces the caller to handle the error case

**What it is not:**
- A regular expression engine — no general regex surface exists in Clean. Named pattern checks on a single string use the standard library's `string.matches(pattern)` ([15 — Standard Library §String Patterns](../04%20language/15-standard-library.md#string-patterns)); the glob strings of `match:` rules ([§2.3](#23-patterns)) are this module's own micro-syntax
- A schema language for type generation — types are defined in `class:` blocks
- An ORM validator — use `data` model constraints for database-level validation

---

## 2. Concepts


### 2.1 ValidationResult

Every validation operation returns a `ValidationResult`. It is an opaque value with two states: success and failure. The caller must branch on `result.ok` before accessing the value.

```clean
ValidationResult result = validator.run(rules, input)
if result.ok
	process(result.value)
else
	print(result.firstError)
```

A `ValidationResult` is never `none`. `validator.run` always returns a result — it never halts on invalid input ([VAL-01](#val-01--validation-never-halts)).

### 2.2 ValidationRules

A `ValidationRules` object is an opaque container built by `validator.create:`. It holds a list of field rules. Once built, it is immutable and safe to reuse across multiple calls to `validator.run`.

### 2.3 Patterns

A pattern constraint in a `match:` rule takes one of two forms: a **named pattern constant** or a **glob string**. In both forms the pattern matches the entire field value — the match must cover the full string, not just a substring.

**Named pattern constants.** The named pattern constants (`emailPattern`, `urlPattern`, …) are declared by [15 — Standard Library §String Patterns](../04%20language/15-standard-library.md#string-patterns) — the single home of the pattern vocabulary ([ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md), Accepted 2026-08-01). The `match:` rule consumes those constants; this document does not restate the list.

**Glob strings — the validator's own micro-syntax.** Custom patterns are plain strings interpreted as glob-style patterns. This glob form is the validator's own micro-syntax, distinct from the standard-library pattern-constant vocabulary ([ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md) companion decision): it exists only inside `match:` rules and is not part of `string.matches(pattern)`. Glob syntax:
- `*` matches any sequence of characters
- `?` matches exactly one character
- All other characters match literally

```clean
field: "code"
	match: "INV-???-*"
```

The pattern `"INV-???-*"` matches `"INV-ABC-2026"` but not `"INV-AB-2026"` (too short) or `"abc"`.

---

## 3. DSL Block Syntax


Rules are declared using a `validator.create:` block. This is the canonical way to define validators — function-call style is available but not preferred.

```clean
ValidationRules rules = validator.create:
	field: "name"
		required: true
		type: string
		minLength: 1
		maxLength: 100
	field: "email"
		required: true
		type: string
		match: emailPattern
		maxLength: 254
	field: "age"
		required: false
		type: integer
		range: 0, 150
	field: "website"
		required: false
		type: string
		match: urlPattern
```

Each `field:` block applies rules to one named field of the input. Rules within a field block are ANDed — all must pass for the field to be valid. Fields not declared in the rules are passed through without validation.

---

## 4. Function Reference


### 4.1 Creation

#### `validator.create() → ValidationRules`

Creates an empty rules container. Used as the return value of a `validator.create:` block; also callable directly.

**Layer:** 1 (WASM native — see [`01-execution-layers.md`](./01-execution-layers.md))
**Error semantics:** [VAL-02](#val-02--construction-never-fails) — never fails, always returns a valid rules object.

```clean
ValidationRules rules = validator.create:
	field: "username"
		required: true
```

---

#### `validator.createWithName(name: string) → ValidationRules`

Creates a named rules container. The name is included in error messages to identify which validator failed.

**Layer:** 1 (WASM native)

```clean
ValidationRules rules = validator.createWithName("UserRegistration"):
	field: "email"
		required: true
		match: emailPattern
```

---

### 4.2 Execution

#### `validator.run(rules: ValidationRules, input: pairs) → ValidationResult`

Runs all rules in `rules` against `input` and returns a `ValidationResult`. If all fields pass, `result.ok` is `true` and `result.value` holds `input`. If any field fails, `result.ok` is `false` and `result.errors` holds the list of error messages.

**Layer:** 1 (WASM native)
**Error semantics:** [VAL-01](#val-01--validation-never-halts) — never halts, always returns a `ValidationResult`.

`validator.validate` is a synonym — identical behaviour.

```clean
ValidationResult result = validator.run(rules, formData)
if result.ok
	saveUser(result.value)
else
	iterate err in result.errors
		print(err)
```

---

#### `validator.runField(rules: ValidationRules, fieldName: string, value: string) → ValidationResult`

Validates a single field by name against the matching rule in `rules`. Useful for real-time field validation.

**Layer:** 1 (WASM native)

```clean
ValidationResult emailResult = validator.runField(rules, "email", inputEmail)
if emailResult.ok
	showValid()
else
	showError(emailResult.firstError)
```

---

### 4.3 Result Inspection

#### `validator.isOk(result: ValidationResult) → boolean`
Returns `true` if validation passed.

#### `validator.isError(result: ValidationResult) → boolean`
Returns `true` if validation failed.

#### `validator.getValue(result: ValidationResult) → pairs`
Returns the validated input value. Only meaningful when `result.ok` is `true`. Returns `none` if validation failed.

#### `validator.getErrors(result: ValidationResult) → list<string>`
Returns the list of error messages. Only meaningful when `result.ok` is `false`. Returns an empty list if validation passed.

#### `validator.getFirstError(result: ValidationResult) → string`
Returns the first error message, or an empty string if validation passed.

**Preferred access:** Use property syntax on the result:

| Property | Equivalent function |
|----------|---------------------|
| `result.ok` | `validator.isOk(result)` |
| `result.value` | `validator.getValue(result)` |
| `result.errors` | `validator.getErrors(result)` |
| `result.firstError` | `validator.getFirstError(result)` |

---

### 4.4 Result Construction

These functions are used when writing custom validators with `validator.custom`.

#### `validator.ok(value: pairs) → ValidationResult`
Creates a successful result carrying `value`.

#### `validator.error(errors: list<string>) → ValidationResult`
Creates a failed result with the given error list.

---

### 4.5 Rule Builders

These are used inside `validator.create:` blocks. They are also callable as functions for dynamic rule construction.

#### `validator.field(rules: ValidationRules, name: string) → ValidationRules`
Adds a new field rule to `rules` and sets it as the current field for subsequent constraint calls. Returns `rules` for chaining.

#### `validator.required(rules: ValidationRules, isRequired: boolean) → ValidationRules`
Marks the current field as required (`true`) or optional (`false`). Default is optional.

#### `validator.optional(rules: ValidationRules) → ValidationRules`
Marks the current field as optional. Equivalent to `validator.required(rules, false)`.

#### `validator.type(rules: ValidationRules, typeName: string) → ValidationRules`
Constrains the field value to a Clean Language type: `"string"`, `"integer"`, `"number"`, `"boolean"`. Type checking happens before other constraints — a field that fails a type constraint skips remaining constraints.

#### `validator.match(rules: ValidationRules, pattern: string) → ValidationRules`
Constrains the field to match a named pattern or glob pattern string. See [§2.3](#23-patterns).

#### `validator.range(rules: ValidationRules, min: integer, max: integer) → ValidationRules`
Constrains a numeric field to fall within `[min, max]` inclusive. Applied after type checking.

#### `validator.minLength(rules: ValidationRules, min: integer) → ValidationRules`
Constrains a string field to have at least `min` characters.

#### `validator.maxLength(rules: ValidationRules, max: integer) → ValidationRules`
Constrains a string field to have at most `max` characters.

#### `validator.message(rules: ValidationRules, msg: string) → ValidationRules`
Sets a custom error message for the most recently declared constraint. Overrides the default error message generated for that constraint.

```clean
field: "email"
	required: true
		message: "Email is required"
	match: emailPattern
		message: "Must be a valid email address"
```

#### `validator.custom(rules: ValidationRules, fn: function) → ValidationRules`
Adds a custom validation function for the current field. The function receives the field value as a string and returns a `ValidationResult`.

```clean
functions:
	ValidationResult isStrongPassword(string value)
		if value.length() < 8
			return validator.error(["Password must be at least 8 characters"])
		return validator.ok(value)
```

```clean
field: "password"
	required: true
	custom: isStrongPassword
```

---

## 5. Error Semantics


### VAL-01 — Validation never halts


`validator.run` and `validator.runField` MUST always return a `ValidationResult` and MUST NOT halt on invalid input. The result MUST always have a defined state (`ok` or error); a `ValidationResult` is never `none`.

### VAL-02 — Construction never fails


`validator.create`, `validator.createWithName`, `validator.ok`, and `validator.error` MUST always return a valid object (`ValidationRules` / `ValidationResult`) and MUST NOT fail. They produce values only — they MUST NOT perform I/O.

### VAL-03 — Single input normalization


Exactly one input normalization exists: a `none` or missing input value MUST be treated as the empty string for string constraints and as absent for `required:` checks. No other implicit coercion is performed.

### VAL-04 — Total result accessors


Accessing `result.value` when `result.ok` is `false` MUST return `none`. Accessing `result.firstError` when `result.ok` is `true` MUST return the empty string. Neither access halts.

### 5.1 Result strings are data, not diagnostics

The strings a `ValidationResult` carries (`errors`, `firstError`, and the default messages of [§6](#6-default-error-messages)) are **return values** of the module — data produced for the calling program — not diagnostics (ratified 2026-08-01). They do not enter the diagnostic pipeline of [`13-diagnostic-format.md`](./13-diagnostic-format.md) and carry no code from the [error code registry](./09-error-codes.md). A registry code would only attach to a *misuse of the module itself* surfaced by the compiler or runtime; no validator-specific misuse diagnostic is currently specified (none defined yet).

---

## 6. Default Error Messages


When no `message:` is set, the validator generates a message in this form (these strings are `ValidationResult` data, not diagnostics — [§5.1](#51-result-strings-are-data-not-diagnostics)). The `match: <constant>` rows are keyed to the pattern constants the validator consumes from [15 — Standard Library §String Patterns](../04%20language/15-standard-library.md#string-patterns) ([§2.3](#23-patterns)); the `match: <glob>` row covers the validator's own glob micro-syntax:

| Constraint | Default message |
|------------|-----------------|
| `required: true` | `"<field> is required"` |
| `type: string` | `"<field> must be a string"` |
| `type: integer` | `"<field> must be a whole number"` |
| `type: number` | `"<field> must be a number"` |
| `type: boolean` | `"<field> must be true or false"` |
| `match: emailPattern` | `"<field> must be a valid email address"` |
| `match: urlPattern` | `"<field> must be a valid URL"` |
| `match: phonePattern` | `"<field> must be a valid phone number"` |
| `match: uuidPattern` | `"<field> must be a valid UUID"` |
| `match: integerPattern` | `"<field> must be a whole number"` |
| `match: numberPattern` | `"<field> must be a number"` |
| `match: alphanumericPattern` | `"<field> must contain only letters and digits"` |
| `match: slugPattern` | `"<field> must contain only lowercase letters, digits, and hyphens"` |
| `match: datePattern` | `"<field> must be a date in YYYY-MM-DD format"` |
| `match: timePattern` | `"<field> must be a time in HH:MM or HH:MM:SS format"` |
| `match: ipv4Pattern` | `"<field> must be a valid IPv4 address"` |
| `match: hexColorPattern` | `"<field> must be a hex color in #RGB or #RRGGBB format"` |
| `match: alphaPattern` | `"<field> must contain only letters"` |
| `match: numericPattern` | `"<field> must contain only digits"` |
| `match: <glob>` | `"<field> does not match the required format"` |
| `range: min, max` | `"<field> must be between <min> and <max>"` |
| `minLength: n` | `"<field> must be at least <n> characters"` |
| `maxLength: n` | `"<field> must be at most <n> characters"` |

---

## 7. Internal Representation


The in-memory layout of `ValidationRules` and `ValidationResult` is an implementation detail: the internal representation is unspecified ([SDD-02](../01%20governance/03-spec-driven-design.md)). Both types are opaque to Clean code; the only observable surface is the function reference in §4.

---

## 8. Complete Examples


### 8.1 Form Validation

```clean
start:
	ValidationRules rules = validator.create:
		field: "username"
			required: true
			type: string
			minLength: 3
			maxLength: 30
			match: alphanumericPattern
				message: "Username can only contain letters and digits"
		field: "email"
			required: true
			type: string
			match: emailPattern
		field: "age"
			required: true
			type: integer
			range: 13, 120
				message: "You must be between 13 and 120 years old"
		field: "website"
			required: false
			type: string
			match: urlPattern

	pairs<string, any> formData = {}
	formData["username"] = "alice99"
	formData["email"] = "alice@example.com"
	formData["age"] = "28"

	ValidationResult result = validator.run(rules, formData)
	if result.ok
		print("Registration successful")
	else
		print("Please fix the following:")
		iterate err in result.errors
			print("  - " + err)
```

### 8.2 Reusable Validators

```clean
functions:
	ValidationRules makeUserRules()
		return validator.create:
			field: "email"
				required: true
				match: emailPattern
			field: "password"
				required: true
				minLength: 8

	ValidationRules makeAddressRules()
		return validator.create:
			field: "street"
				required: true
				maxLength: 200
			field: "city"
				required: true
				maxLength: 100
			field: "postcode"
				required: true
				match: alphanumericPattern
				maxLength: 10

start:
	ValidationRules userRules = makeUserRules()
	ValidationRules addressRules = makeAddressRules()

	ValidationResult userResult = validator.run(userRules, userData)
	ValidationResult addrResult = validator.run(addressRules, addressData)
	if userResult.ok and addrResult.ok
		createAccount(userResult.value, addrResult.value)
```

### 8.3 Custom Validator

```clean
functions:
	ValidationResult validatePassword(string value)
		if value.length() < 8
			return validator.error(["Password must be at least 8 characters"])
		if value.contains(" ")
			return validator.error(["Password must not contain spaces"])
		return validator.ok(value)

start:
	ValidationRules rules = validator.create:
		field: "password"
			required: true
			custom: validatePassword

	ValidationResult result = validator.run(rules, input)
	if result.ok
		setPassword(result.value)
	else
		print(result.firstError)
```

### 8.4 Real-Time Field Validation

```clean
start:
	ValidationRules rules = validator.create:
		field: "email"
			required: true
			match: emailPattern

	ValidationResult emailCheck = validator.runField(rules, "email", userInput)
	if emailCheck.ok
		showGreenIndicator()
	else
		showError(emailCheck.firstError)
```

---

## 9. WASM Import Signatures


The validator module runs at **Layer 1 (WASM native)** — all functions execute within the WASM module itself using direct memory operations. There are no host bridge imports for the validator namespace.

---

## 10. Proposed Enhancements


Proposed enhancements are not spec content; they live outside the spec in [`work/2026-08-01-validator-backlog.md`](../work/2026-08-01-validator-backlog.md) until individually specified or discarded.

---

## Changelog

- 2026-08-01 — [ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md) applied (Accepted 2026-08-01): §2.3's defining catalog of the twelve `…Pattern` names replaced by a citation of the vocabulary's new single home, [15 — Standard Library §String Patterns](../04%20language/15-standard-library.md#string-patterns), and the "(pattern vocabulary: pending — ADR-0009, Draft)" cross-note removed; the glob strings of `match:` rules declared the validator's own micro-syntax, distinct from the pattern-constant vocabulary (ADR-0009 companion decision); §1's phantom pointer to a built-in `string.match` replaced by a reference to the stdlib's `string.matches(pattern)`; §6's `match:` default messages now keyed to the cited constants.
- 2026-08-01 — Fase 4 (lote 1), traceability compliance pass: claimed rule prefix `VAL-`; minted `VAL-01` (validation never halts), `VAL-02` (construction never fails), `VAL-03` (single input normalization), `VAL-04` (total result accessors) in §5, each with concern citations; §§2–6 marked *Normative.*, §§1, 7–10 marked *Informative.*; §4's error-semantics lines and §1/§2.1 now cite the VAL rules instead of restating them (DOC-14). Ratification applied: the strings inside a `ValidationResult` are return **values** (data), not diagnostics — the two "(diagnostic code: pending — Fase 4)" markers removed and replaced by the informative note §5.1; no validator-specific misuse diagnostic exists yet (none defined — none invented, EXE-09). This closes the diagnostic-code debt recorded in the Fase 3 entry below.
- 2026-08-01 — Fase 3 remediation per the approved conflict log (P14): §10 (Proposed Enhancements P1–P10, ~31% of the file) extracted to `work/2026-08-01-validator-backlog.md` (roadmap is task-brief material, DOC-07/DOC-12); §7 memory layout and Type IDs removed — internal representation is unspecified (SDD-02); all examples rewritten to valid Clean (parenthesized calls, explicit declaration types, `iterate`, camelCase — the previous no-parentheses dialect is gone); the four missing default messages added in §6 (`numberPattern`, `timePattern`, `ipv4Pattern`, `hexColorPattern`); coercion contradiction harmonized — the "No magic, no implicit coercion" slogan replaced by the actual rule (`none`/missing input is treated as empty string / absent, §1 and §5); validation-failure statements marked "(diagnostic code: pending — Fase 4)" — no codes coined (09 §1.2); §2.3 cross-noted to the pending unified pattern vocabulary ([ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md), Draft). Explicit debt: this document remains Accepted-by-refactoring while its diagnostic codes are pending (SDD-04; user decision 2026-07-31).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Standard-library implementors and application authors validating structured input
- **Rule prefix:** `VAL-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [15 — Standard Library](../04%20language/15-standard-library.md), [13 — Diagnostic Format](./13-diagnostic-format.md), [ADR-0009 — String Pattern Vocabulary](../01%20governance/decisions/0009-string-pattern-vocabulary.md)
