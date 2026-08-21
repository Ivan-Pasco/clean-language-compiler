# 15. Standard Library

The standard library is the set of modules every Clean program can use without importing anything: `console` for `print` and `input`, `math` for advanced arithmetic, `string` and `list` for text and collection operations, `file` and `http` for I/O the host grants, `json` for parsing and serialization, `time` and `bytes` for building the corresponding types, `matrix` for the built-in matrix arithmetic, and `validator` for declarative field checks. This chapter is the single home of every module's surface — the methods, the namespace functions, the diagnostics they raise, and the scope note on when each is available.

Clean Language provides built-in utility classes for common operations. All standard library classes follow the compiler instructions:

- All methods are in `functions:` blocks
- Method calls require parentheses
- No `Utils` suffix in class names
- Use `any` for generic operations

### Console Module

The console surface: writing to standard output and reading from standard input. `print` and `input` are language keywords ([3 — Lexical Structure](./03-lexical-structure.md)), and the `print:` block form is language grammar ([7 — Statements](./07-statements.md)); this section is the home of what they *do*.

**Output.**

| Call | Returns | Description |
|------|---------|-------------|
| `print(value)` | void | Write `value` followed by a newline |
| `print(value, newline: false)` | void | Write `value` with no trailing newline |

`newline` defaults to `true`. There is exactly one call form — parentheses always, per [Rule 2](./02-language-design-rules.md). There is no bare `print "text"` form and no `+` suffix; `+` is the addition operator.

Any type may be printed. A value that is not already a `string` is rendered with its [`toString()`](#conversions), including a class's own `toString()` where it defines one:

```clean
integer age = 25
print(age)                      // 25
print("Age: " + age.toString()) // Age: 25 — explicit when building a larger string

class Person
	string name
	integer age
	functions:
		string toString()
			return name + " (" + age.toString() + " years old)"

Person user = Person("Alice", 30)
print(user)                     // Alice (30 years old)
```

A class with no `toString()` of its own prints its class name.

**Input.**

| Call | Returns | Description |
|------|---------|-------------|
| `input(prompt)` | `string?` | Write `prompt`, then read one line |
| `input()` | `string?` | Read one line with no prompt |
| `input.integer(prompt)` | `integer?` | Read a line and parse it as an integer |
| `input.number(prompt)` | `number?` | Read a line and parse it as a number |
| `input.yesNo(prompt)` | `boolean?` | Read a line and parse it as a yes/no answer |

**Every input call is fallible, and says so in its type.** Each returns `none` when the line cannot be produced or cannot be parsed: end of input, or text the parsing form does not accept. There is no re-prompt loop inside the library.

```clean
start:
	string userName = input("Enter your name: ") default "world"
	print("Hello, " + userName + "!")

	integer num1 = input.integer("First number: ") default 0
	integer num2 = input.integer("Second number: ") default 0
	print("Sum: " + (num1 + num2).toString())

	if input.yesNo("Would you like coffee? ") default false
		print("Great! Coffee coming right up.")
	else
		print("No problem, maybe next time.")
```

`input.yesNo` accepts `yes`/`no`, `y`/`n`, `true`/`false`, `1`/`0`, compared case-insensitively. The accepted set is fixed and ASCII — it is not localized; an application that must prompt in another language reads with `input()` and interprets the answer itself, using the [locale library](../02%20components/framework/libraries/06-locale.md).

**Re-prompting is the program's job, not the library's.** A program that wants to keep asking writes the loop, where it is visible:

```clean
integer? age = input.integer("Age: ")
while age is none
	print("Please enter a whole number.")
	age = input.integer("Age: ")
```

The library had been specified to re-prompt internally, which could not be implemented: standard input is not always a terminal, and an internal retry loop against a closed stream never terminates and has no value to return. Bounding the retries would have required an arbitrary constant, and varying the behaviour by whether the stream is a terminal would make the same program behave differently in CI than on a developer's machine ([C-04](../01%20governance/05-concerns.md)). Returning an optional terminates in every case, behaves identically everywhere, and needs no diagnostic code — the failure is a value the caller already has to handle ([ADR-0020](../01%20governance/decisions/0020-console-input-failure-behaviour.md)).

A loop that never ends is still possible, as above on a closed stream — but it is written in the program, where its author can see it.

### Conversions

Four conversion methods are available on every value whose type supports the target. They are methods, not namespace functions ([16 — Method-Style Syntax](./16-method-style-syntax.md)); the rule that governs when a conversion is implicit is in [4 — Type System §Type Conversion](./04-type-system.md).

| Call | Returns | Behaviour |
|------|---------|-----------|
| `value.toString()` | string | Available on every type. `integer` and `number` render in their literal form; `boolean` renders as `true` / `false` |
| `value.toInteger()` | integer | From `number`: truncates toward zero. From `string`: parses a decimal integer literal; a string that is not one raises [`RUN003`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| `value.toNumber()` | number | From `integer`: widens (see the precision note in [4 — Type System](./04-type-system.md)). From `string`: parses a decimal literal; a string that is not one raises [`RUN003`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| `value.toBoolean()` | boolean | From `integer` or `number`: zero is `false`, every other value is `true` |

**The table is exhaustive.** A source-type/target pair the table does not list is not a conversion: `text.toBoolean()` on a `string`, for example, does not exist, and calling it is [`SEM022`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) like any other undefined method. A program that must read a boolean out of text states its own accepted spellings and compares.

**`number.toString()` renders the shortest round-trip form.** The result is the decimal string with the fewest significant digits that parses back (per the rule below) to the exact same `number`, written in the literal grammar of [3 — Lexical Structure](./03-lexical-structure.md). "Fewest digits that round-trip" fixes the digits; the presentation edges are fixed by the four rules below, so two conforming implementations cannot disagree on a single output byte. `integer.toString()` is plain decimal with no padding or separators.

- **Plain vs. scientific.** Write the shortest digit sequence in normalized scientific form `d₁.d₂…dₖ × 10^E` with `d₁ ≠ 0`. The rendering is plain decimal exactly when **-4 ≤ E ≤ 21**, scientific otherwise. Worked boundary examples: `0.0001` (E = −4) is plain; the next magnitude down is scientific, rendering `1e-5` (E = −5); `10²¹` (E = 21) is plain — `1` followed by 21 zeros and `.0` — and `10²²` (E = 22) is scientific, rendering `1e22`. (The convention matters: read the digits as `0.d₁d₂…` instead and every E shifts by one, silently moving the boundary. The worked examples pin it.)
- **Scientific spelling.** The mantissa is the shortest digits with the point after the first, and carries no fractional part when it is a single digit (`1e22`, not `1.0e22`); the exponent is a lowercase `e` followed by the decimal value of E — `-` when negative, no `+`, no leading zeros (`6.02e23`, `1e-5`). Both output forms stay inside the NumberLiteral grammar, which admits an exponent with no dot.
- **Integral values carry `.0` in plain decimal.** An integral `number` renders with a trailing `.0` — `42.0`, never `42` — keeping the output in the dotted NumberLiteral form and visibly a `number`. Zero renders `0.0`.
- **Signed zero.** Negative zero renders `-0.0`. IEEE 754 distinguishes the two zeros, and the shortest-round-trip rule's "exact same number" clause requires the sign: `0.0` parses back to the positive zero.
- **Non-finite values.** NaN renders `NaN`; the infinities render `Infinity` and `-Infinity` — exactly these spellings, case-sensitive. They are **outputs only**: none is a NumberLiteral, and `string.toNumber()` does not accept them (below).

**`string.toNumber()` parses correctly rounded.** The result is the `number` nearest the exact decimal value of the input (ties to even, IEEE 754 `roundTiesToEven`) — never an accumulation of per-digit floating-point operations, which drifts on long inputs. This is the same contract every mainstream `strtod` honors, and it is what makes `toString ∘ toNumber` the identity the shortest-round-trip rule above assumes.

**Non-finite spellings are not input.** `string.toNumber()` accepts only decimal-literal spellings: `"NaN"`, `"Infinity"` and `"-Infinity"` are parse failures like any other non-literal string, raising [`RUN003`](../03%20platform/09-error-codes.md#312-runtime-codes-run) with `"the string is not a valid number literal"` ([10 §RUN003](../03%20platform/10-semantic-rules.md#run003--arithmetic-error)). The `toString ∘ toNumber` identity is therefore scoped to **finite** values; a program that must carry non-finite numbers through text states its own spellings and checks them. Output-only keeps one grammar on the input side and matches the stance [`RUN007`](../03%20platform/10-semantic-rules.md#run007--json-invalid-number) already takes at the JSON boundary, where a non-finite reading of numeric text is refused rather than admitted.

### Math Module

The math module follows Clean Language's "one way to do things" principle. Basic arithmetic operations use operators, while advanced mathematical functions use methods.

**Basic Arithmetic - Use Operators:**
- Addition: `a + b` (not `math.add(a, b)`)
- Subtraction: `a - b` (not `math.subtract(a, b)`)
- Multiplication: `a * b` (not `math.multiply(a, b)`)
- Division: `a / b` (not `math.divide(a, b)`)
- Exponentiation: `a ^ b` (not `math.pow(a, b)`)

**Advanced Mathematics - Use Functions:**

**Available functions:**

| Function | Returns | Description |
|----------|---------|-------------|
| `math.sqrt(x)` | number | Square root |
| `math.absInteger(x: integer)` | integer | Absolute value of an integer |
| `math.absNumber(x: number)` | number | Absolute value of a number |
| `math.max(a, b)` | number | Larger of two values |
| `math.min(a, b)` | number | Smaller of two values |
| `math.floor(x)` | number | Round down to nearest integer |
| `math.ceil(x)` | number | Round up to nearest integer |
| `math.round(x)` | number | Round to nearest integer |
| `math.trunc(x)` | number | Remove decimal part |
| `math.sign(x)` | number | Returns -1, 0, or 1 |
| `math.sin(x)` | number | Sine (radians) |
| `math.cos(x)` | number | Cosine (radians) |
| `math.tan(x)` | number | Tangent (radians) |
| `math.asin(x)` | number | Arc sine |
| `math.acos(x)` | number | Arc cosine |
| `math.atan(x)` | number | Arc tangent |
| `math.atan2(y, x)` | number | Two-argument arc tangent |
| `math.ln(x)` | number | Natural logarithm (base e) |
| `math.log10(x)` | number | Base-10 logarithm |
| `math.log2(x)` | number | Base-2 logarithm |
| `math.exp(x)` | number | e raised to the power of x |
| `math.exp2(x)` | number | 2 raised to the power of x |
| `math.sinh(x)` | number | Hyperbolic sine |
| `math.cosh(x)` | number | Hyperbolic cosine |
| `math.tanh(x)` | number | Hyperbolic tangent |
| `math.pi` | number | π ≈ 3.14159 (constant, no parens) |
| `math.e` | number | Euler's number ≈ 2.71828 (constant, no parens) |
| `math.tau` | number | τ = 2π ≈ 6.28318 (constant, no parens) |

**Edge-case contract.** The functions with a WASM instruction behind them mean exactly what the instruction means, so the same call returns the same bits on every host:

- `math.round(x)` rounds to the nearest integer with **ties to even** — the `f64.nearest` instruction: `math.round(2.5)` is `2.0`, `math.round(3.5)` is `4.0`. This differs from the ties-away-from-zero `round` of several mainstream languages; ties-to-even is the IEEE 754 default and the only mode WASM provides as an instruction, and a Clean program that wants away-from-zero writes it explicitly (`math.floor(x + 0.5)` for non-negative `x`).
- Domain errors produce **NaN**, never a raised failure: `math.sqrt(-1.0)`, `math.ln(-1.0)`, `math.asin(2.0)` are NaN per IEEE 754. A program checks the domain before the call or the result after it.
- `number` division (`/`) is the `f64.div` instruction and **never raises**: `1.0 / 0.0` is `Infinity`, `-1.0 / 0.0` is `-Infinity`, `0.0 / 0.0` is NaN, per IEEE 754 — the same stance as the domain-error rule above. Only `integer` arithmetic raises [`RUN003`](../03%20platform/09-error-codes.md#312-runtime-codes-run): division or remainder by zero, and the overflowing `integer` minimum divided by `-1` ([10 §RUN003](../03%20platform/10-semantic-rules.md#run003--arithmetic-error) owns the raise-site list and message wordings).
- `math.max` / `math.min` follow the `f64.max` / `f64.min` instructions: NaN propagates (either argument NaN → result NaN), and `-0.0` orders below `+0.0`.

**Examples:**

```clean
start:
	// Basic calculations — use operators for basic math, math.* for advanced operations
	number result = 5.0 + 3.0               // Use + operator, not math.add()
	number maximum = math.max(10.5, 7.2)    // Use math functions for advanced operations

	// Geometry - calculate circle area
	number radius = 5.0
	number area = math.pi * (radius ^ 2.0)   // Use operators for basic arithmetic

	// Trigonometry - find triangle sides
	number angle = math.pi / 4.0             // Use / operator, not math.divide()
	number opposite = 10.0 * math.sin(angle) // Use * operator, not math.multiply()
	number adjacent = 10.0 * math.cos(angle)

	// Rounding numbers for display
	number price = 19.99567
	number rounded = math.round(price)  // 20.0
	number floored = math.floor(price)  // 19.0

	// Logarithmic calculations
	number growth = math.exp(0.05)      // e^0.05 for 5% growth
	number halfLife = math.log2(100.0)  // How many times to halve 100 to get 1

	// Distance calculations using Pythagorean theorem
	number dx = 3.0
	number dy = 4.0
	number distance = math.sqrt((dx ^ 2.0) + (dy ^ 2.0))  // Use + operator, not math.add()

	// Absolute values for different types
	number numberAbs = math.absNumber(-5.7)     // 5.7
	integer intAbs = math.absInteger(-42)       // 42
```

### String Module

Text manipulation. Method-style calls act on a string value; the two namespace functions build a new string from several inputs.

**Method-style calls (act on a string value):**

| Call | Returns | Description |
|------|---------|-------------|
| `text.length()` | integer | Number of characters |
| `text.toUpperCase()` | string | All letters uppercase |
| `text.toLowerCase()` | string | All letters lowercase |
| `text.trim()` | string | Remove leading/trailing whitespace |
| `text.trimStart()` | string | Remove leading whitespace only |
| `text.trimEnd()` | string | Remove trailing whitespace only |
| `text.contains(search)` | boolean | Returns true if search string is found |
| `text.indexOf(search)` | integer | First position of search, or -1 |
| `text.lastIndexOf(search)` | integer | Last position of search, or -1 |
| `text.startsWith(prefix)` | boolean | True if text begins with prefix |
| `text.endsWith(suffix)` | boolean | True if text ends with suffix |
| `text.replace(old, new)` | string | Replace all occurrences of old with new |
| `text.split(delimiter)` | list\<string\> | Split into a list on the delimiter |
| `text.charAt(index)` | string | Character at position (0-based) |
| `text.charCodeAt(index)` | integer | Numeric code of character at position |
| `text.isEmpty()` | boolean | True if length is zero |
| `text.isBlank()` | boolean | True if empty or only whitespace |
| `text.padStart(length, pad)` | string | Pad beginning to reach length |
| `text.padEnd(length, pad)` | string | Pad end to reach length |
| `text.substring(start, end)` | string | Extract substring from start to end |
| `text.matches(pattern)` | boolean | True if the entire string matches a named pattern constant — see [String Patterns](#string-patterns) |

**Every index and length counts Unicode code points.** A "character" and a "position" throughout this module — `length()`, `charAt`, `charCodeAt`, `indexOf`, `lastIndexOf`, `substring`, `padStart`/`padEnd` — is one Unicode scalar value, not a byte of the UTF-8 layout ([Platform 03 §3.3](../03%20platform/03-memory-model.md#33-string-representation)) and not a UTF-16 unit: `"héllo".length()` is 5 on every host, whatever the payload's byte count. `charCodeAt(i)` returns the code point's scalar value (`"€".charCodeAt(0)` is 8364), and `charAt(i)` returns a one-code-point string. Code points are the only unit that is coherent over the UTF-8 layout without exposing it; byte positions would make the same program's answers depend on the text's encoding length, and UTF-16 units belong to a representation Clean never uses.

**Edge-case contract**, so the same call means the same thing everywhere:

- `substring(start, end)` **clamps**: each index is clamped to `[0, length]`, and `end < start` (after clamping) yields `""`. It never raises for a range.
- `charAt(i)` / `charCodeAt(i)` out of `[0, length)` raise [`RUN013`](../03%20platform/09-error-codes.md#312-runtime-codes-run) — a position names one code point, and there is none to name.
- Whitespace for `trim`/`trimStart`/`trimEnd`/`isBlank` is the ASCII set `{space, \t, \n, \r}`. Unicode whitespace classes vary by Unicode version; a fixed set keeps the result version-independent.
- `replace(old, new)` with `old == ""` returns the receiver unchanged; `padStart`/`padEnd` with an empty `pad` return the receiver unchanged (the alternative in each case is a loop that cannot terminate or an arbitrary insertion rule).
- `split("")` returns a one-element list holding the receiver — the delimiter was not found. Clean does not adopt the JavaScript per-character explosion; per-character work is `iterate ch in text`.

| Call | Returns | Description |
|------|---------|-------------|
| `string.concat(a, b)` | string | Concatenate two strings |

**Examples:**

```clean
start:
	// Basic text processing
	string userInput = "  Hello World!  "
	string cleaned = userInput.trim()              // "Hello World!"
	integer length = cleaned.length()              // 12

	// Case normalization for comparisons
	string email1 = "USER@EXAMPLE.COM"
	string email2 = "user@example.com"
	boolean same = email1.toLowerCase() == email2.toLowerCase()  // true

	// Text searching and validation
	string filename = "document.pdf"
	boolean isPdf = filename.endsWith(".pdf")      // true
	integer dotPos = filename.lastIndexOf(".")     // 8

	// URL processing
	string url = "https://api.example.com/users"
	boolean isHttps = url.startsWith("https://")  // true
	boolean hasApi = url.contains("api")          // true

	// Text parsing and reconstruction
	string csvLine = "John,Doe,25,Engineer"
	list<string> fields = csvLine.split(",")       // ["John", "Doe", "25", "Engineer"]
	string fullName = list.join([fields[0], fields[1]], " ")  // "John Doe"

	// Text replacement and cleaning
	string messyText = "Hello    World"
	string cleanedText = messyText.replace("    ", " ")    // "Hello World"

	// Formatting and padding
	string number = "42"
	string padded = number.padStart(5, "0")        // "00042"

	// Character-level operations
	string word = "Hello"
	string firstChar = word.charAt(0)              // "H"
	integer charCode = word.charCodeAt(0)          // 72 (ASCII for 'H')

	// Input validation
	string userField = "   "
	boolean isValid = !userField.isBlank()         // false

	// Pattern matching against the declared vocabulary
	boolean isEmail = email2.matches(emailPattern) // true
```

#### String Patterns
The string module declares the **pattern vocabulary**: fourteen named pattern constants. This section is the vocabulary's single home, established by [ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md) (Accepted 2026-08-01).

| Constant | Matches |
|----------|---------|
| `emailPattern` | An email address per RFC 5321 |
| `urlPattern` | An absolute URL with scheme and host |
| `phonePattern` | An international phone number, optional `+` country code |
| `uuidPattern` | A UUID, versions 1–5 |
| `integerPattern` | A whole number — optional leading sign, then digits only |
| `numberPattern` | A number — optional leading sign, digits, optional decimal point |
| `alphanumericPattern` | Letters and digits only |
| `slugPattern` | Lowercase letters, digits, and hyphens only |
| `datePattern` | An ISO 8601 calendar date (`YYYY-MM-DD`) |
| `timePattern` | An ISO 8601 time (`HH:MM` or `HH:MM:SS`, 24-hour) |
| `ipv4Pattern` | A dotted-quad IPv4 address, each octet 0–255 |
| `hexColorPattern` | A hex color, `#RGB` or `#RRGGBB` |
| `alphaPattern` | Letters only — no digits, spaces, or punctuation |
| `numericPattern` | Digits only — no sign, no decimal point |

**`string.matches(pattern)`** — `text.matches(pattern)` returns `boolean`: `true` when the **entire** string matches the named pattern (full match, not substring).

- The argument to `string.matches()` MUST be one of the fourteen constants declared above — an ordinary identifier check performed at compile time. Passing a bare string (`"email"`), a variable, or any other expression violates [SEM010](../03%20platform/10-semantic-rules.md#sem010--invalid-match-pattern).
- The vocabulary is closed: extending it is an ordinary spec change to this section ([ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md)). There are no pattern packs and no user-defined pattern constants.
- The validator's `match:` rule consumes these same constants ([Platform 11 §2.3](../03%20platform/11-stdlib-validator.md#23-patterns)). The glob strings accepted there (`"INV-???-*"`) are the validator's own micro-syntax, not part of this vocabulary.

```clean
start:
	string input = "alice@example.com"
	boolean ok = input.matches(emailPattern)       // true
	boolean isSlug = input.matches(slugPattern)    // false
```

### `secret` operations

`secret` is a language-level taint-tracked type for values that must never appear in diagnostic messages, logs, HTTP responses, or debug representations without explicit declassification. The taint rules and production semantics live in [`04-type-system.md § The secret type`](./04-type-system.md); the design rationale lives in [ADR-0023](../01%20governance/decisions/0023-secret-handling-strategy.md). This section documents the operations that can be performed on a `secret` value — see also [SEC-07](../01%20governance/08-security-principles.md).

#### Operations

| Call | Returns | Description |
|------|---------|-------------|
| `s.reveal()` | `string` | Declassify the secret value; returns the underlying string. Every call site is a review target under SEC-07. This is the only way to obtain the raw value. |
| `s.is_empty()` | `boolean` | Returns `true` if the underlying value has zero length. Safe to call without declassifying — no content is leaked. |
| `s == other` | `boolean` | Constant-time byte comparison of two `secret` values. Never use `string` equality on secrets; the compiler MUST reject comparisons between `secret` and `string` without an intervening `.reveal()`. |

#### Default representations

All of the following emit the literal string `"[REDACTED]"` without any framework opt-in:

- `toString()` — string representation for interpolation or concatenation with a plain string result
- Debug repr — any diagnostic, print, or log surface
- JSON serialisation — when a `secret` value is serialised into a JSON structure

These overrides apply mechanically. Adding a new serialisation surface to the framework does not require a separate redaction rule; it inherits the behaviour from the type.

#### Worked examples

**JWT signing — the secret flows through; `.reveal()` is called once at the signer boundary**

```clean
// env.get returns secret because "JWT_SECRET" ends in _SECRET.
secret jwtSecret = env.get("JWT_SECRET")

functions:
    string issueToken(UserId userId, string email, string role)
        // auth.jwt.sign accepts secret directly — no reveal needed here.
        return auth.jwt.sign({
            sub: userId,
            email: email,
            role: role,
            exp: time.now().plusMinutes(60)
        }, jwtSecret)

        // If a third-party signer requires raw bytes, .reveal() is called
        // at exactly that boundary and nowhere else:
        //   string rawSecret = jwtSecret.reveal()
        //   return thirdParty.sign(payload, rawSecret)
```

**Database password — passes as `secret` all the way to the connection factory**

```clean
// env.get returns secret because "DB_PASSWORD" ends in _PASSWORD.
secret dbPassword = env.get("DB_PASSWORD")

functions:
    Connection openDb()
        // The connection factory accepts secret directly.
        return db.connect({
            host: env.get("DB_HOST"),    // string — name does not match pattern
            user: env.get("DB_USER"),    // string
            password: dbPassword         // secret — typed correctly
        })
```

**API key — passes through a helper; taint preserved across the call boundary**

```clean
secret apiKey = env.get("PAYMENTS_API_KEY")

functions:
    Response chargeCard(PaymentDetails details)
        // chargeCard receives apiKey typed as secret.
        // Inside chargeCard, the parameter is secret.
        // .reveal() is called only when the HTTP client needs the raw header value.
        return http.post("https://payments.example.com/charge",
            headers: { "Authorization": "Bearer " + apiKey.reveal() },
            body: details.toJson()
        )
```

### List Module

Collection operations. Method-style calls act on a list value; the namespace functions build a new list, or a string, from other values.

**Method-style calls (act on a list value):**

| Call | Returns | Description |
|------|---------|-------------|
| `items.length()` | integer | Number of elements |
| `items.get(index)` | element type | Element at index (0-based) |
| `items.set(index, value)` | void | Set element at index |
| `items.add(item)` | void | Add item to the end |
| `items.remove(index)` | element type | Remove and return element at index |
| `items.removeLast()` | element type | Remove and return the last element |
| `items.insert(index, item)` | void | Insert item at position |
| `items.contains(item)` | boolean | True if item is in the list |
| `items.indexOf(item)` | integer | First position of item, or -1 |
| `items.lastIndexOf(item)` | integer | Last position of item, or -1 |
| `items.slice(start, end)` | list | New list with elements from start to end |
| `items.reverse()` | list | New list in reverse order |
| `items.sort()` | list | New list sorted ascending |
| `items.isEmpty()` | boolean | True if length is zero |
| `items.isNotEmpty()` | boolean | True if length is non-zero |
| `items.first()` | element type | First element |
| `items.last()` | element type | Last element |
| `items.remove()` | element type | Remove and return the next element **as the list's behavior determines** — the front of a `.line`, the top of a `.pile`. Only available on a list declared with a behavior; see [4 — Type System §List Behaviors](./04-type-system.md) |
| `items.peek()` | element type | Return that same next element **without** removing it. Same availability as `remove()` |

> **No higher-order list methods.** Clean has no lambdas or arrow functions, so `map`, `filter`, `reduce`, `forEach` have no place to put their function argument. Use an `iterate` loop instead — one keyword, one block, no callback. Higher-order forms may return in a future release once first-class function values are added; do not introduce them earlier.

**Edge-case contract:**

- `slice(start, end)` clamps exactly like [`string.substring`](#string-module): indices clamped to `[0, length]`, `end < start` yields `[]`. `get`/`set`/`remove(index)`/`insert` out of range raise [`RUN013`](../03%20platform/09-error-codes.md#312-runtime-codes-run) — an index names one element; a range names a span, which may be empty.
- `sort()` is defined on `list<integer>`, `list<number>`, and `list<string>` only — ascending numeric order, or byte-wise lexicographic order for strings ([TYP-07](./04-type-system.md#typ-07--string-equality-is-byte-exact-nothing-normalizes)'s comparison). No ordering is defined for class-typed or other elements, so `sort()` on such a list is [`SEM004`](../03%20platform/09-error-codes.md#32-semantic-codes-sem).
- `contains`/`indexOf`/`lastIndexOf` compare by value for `integer`/`number`/`boolean`/`string` elements (string comparison per TYP-07). Equality is not defined for class-typed elements, so searching such a list is [`SEM004`](../03%20platform/09-error-codes.md#32-semantic-codes-sem); iterate and compare the field that identifies the object.

**Namespace-style calls (utility functions on the `list` module):**

| Call | Returns | Description |
|------|---------|-------------|
| `list.concat(a, b)` | list | Combine two lists into one |
| `list.range(start, end)` | list\<integer\> | List of integers from start to end |
| `list.fill(size, value)` | list | New list of size filled with value |
| `list.join(items, sep)` | string | Join list elements into a string with separator |

**Examples:**

```clean
start:
	// Basic list operations
	list<integer> numbers = [1, 2, 3]
	integer count = numbers.length()              // 3
	integer first = numbers.get(0)                // 1
	numbers.set(1, 99)                            // [1, 99, 3]

	// Building and modifying lists
	list<string> fruits = ["apple", "banana"]
	fruits.add("orange")                          // ["apple", "banana", "orange"]
	string removed = fruits.remove(2)             // "orange", fruits becomes ["apple", "banana"]

	// Searching through data
	list<integer> scores = [85, 92, 78, 96, 88]
	boolean hasHighScore = scores.contains(96)    // true
	integer position = scores.indexOf(92)         // 1

	// Data processing and transformation — iterate loop, no callbacks
	list<integer> data = [1, 2, 3, 4, 5]

	list<integer> doubled = []
	iterate n in data:
		doubled.add(n * 2)                        // [2, 4, 6, 8, 10]

	list<integer> evens = []
	iterate n in data:
		if n % 2 == 0:
			evens.add(n)                          // [2, 4]

	integer sum = 0
	iterate n in data:
		sum = sum + n                             // 15

	// List manipulation
	list<string> names1 = ["Alice", "Bob"]
	list<string> names2 = ["Charlie", "Diana"]
	list<string> allNames = list.concat(names1, names2)    // ["Alice", "Bob", "Charlie", "Diana"]
	list<string> reversed = allNames.reverse()             // ["Diana", "Charlie", "Bob", "Alice"]

	// Working with sections of lists
	list<integer> bigList = [10, 20, 30, 40, 50]
	list<integer> middle = bigList.slice(1, 4)             // [20, 30, 40]

	// Text processing with lists
	list<string> words = ["hello", "world", "from", "Clean"]
	string sentence = list.join(words, " ")                // "hello world from Clean"

	// Creating lists programmatically
	list<string> greetings = list.fill(3, "Hello")         // ["Hello", "Hello", "Hello"]
	list<integer> countdown = list.range(5, 1)             // [5, 4, 3, 2, 1]

	// Validation and utility
	boolean empty = [].isEmpty()                           // true
	string firstWord = words.first()                       // "hello"
	string lastWord = words.last()                         // "Clean"
```

### File Module

The file module gives Clean programs direct, unrestricted filesystem access wherever the host grants `wasi:filesystem`. Use it for CLI apps, scripts, build tools, and anything else that needs to read or write arbitrary paths the host has authorized.

| Call | Returns | Description |
|------|---------|-------------|
| `file.read(path)` | string | Read entire file as a string |
| `file.lines(path)` | list\<string\> | Read file and return each line as a list element |
| `file.write(path, content)` | void | Write content to file (creates if missing, replaces if existing) |
| `file.append(path, content)` | void | Append content to end of file (creates if missing) |
| `file.exists(path)` | boolean | True if a file exists at path |
| `file.delete(path)` | void | Delete a file (does nothing if not found) |

> **When to use `file.*` vs the `storage` library:** `file.*` is unsandboxed — it can reach any path the host's `wasi:filesystem` preopens allow. Server-side web applications that persist user-supplied bytes (uploaded files, generated artifacts, cached blobs) should use the [`storage`](../02%20components/framework/libraries/09-storage.md) library instead: it pins every write to a single host-configured root and rejects `..` at the boundary, so a compromised handler cannot escape the storage directory. Rule of thumb: **`file.*` for tools you control end-to-end; `storage.*` when a path or filename could be influenced by user input.**

**Examples:**

```clean
start:
	// Read a configuration file
	string config = file.read("settings.txt")

	// Process a log file line by line (read content, then split by newline)
	string logContent = file.read("app.log")
	list<string> logLines = logContent.split("\n")

	// Save user data
	file.write("user_data.txt", "John Doe, 25, Engineer")

	// Add to a log file
	file.append("activity.log", "User logged in at 2:30 PM")

	// Check if a file exists before reading
	if file.exists("backup.txt")
		string backup = file.read("backup.txt")

	// Clean up temporary files
	file.delete("temp_data.txt")
```

### Http Module

The http module makes **server-side, outbound** web requests simple and intuitive. Use it whenever a server component needs to fetch or send data to another service.

| Call | Returns | Description |
|------|---------|-------------|
| `http.get(url)` | string | Send a GET request; return response body |
| `http.post(url, body)` | string | Send a POST request with body; return response body |
| `http.put(url, body)` | string | Send a PUT request with body; return response body |
| `http.patch(url, body)` | string | Send a PATCH request with body; return response body |
| `http.delete(url)` | string | Send a DELETE request; return response body |

> **Scope — `http.*` is only available in the `server` world of `clean:host`.** It maps to `wasi:http/handler@0.3.0`, which browser components do not expose. For browser-side HTTP (fetch from client-side WASM to your own server), use the [`client`](../02%20components/framework/libraries/03-client.md) library's `api.*` calls instead. Building a CLI or desktop app? `http.*` is available there too, provided the host grants `wasi:http`.

**Examples:**

```clean
start:
	// Fetch user data from an API
	string users = http.get("https://api.example.com/users")

	// Create a new user
	string newUser = "{\"name\": \"Alice\", \"email\": \"alice@example.com\"}"
	string response = http.post("https://api.example.com/users", newUser)

	// Update user information
	string updatedUser = "{\"name\": \"Alice Smith\", \"email\": \"alice.smith@example.com\"}"
	http.put("https://api.example.com/users/123", updatedUser)

	// Partially update user (just the email)
	string emailUpdate = "{\"email\": \"newemail@example.com\"}"
	http.patch("https://api.example.com/users/123", emailUpdate)

	// Remove a user
	http.delete("https://api.example.com/users/123")

	// Fetch weather data
	string weather = http.get("https://api.weather.com/current?city=London")
```

### JSON Module

The json module provides functions for parsing JSON text into Clean Language data structures and serializing data back to JSON text. This is essential for working with web APIs, configuration files, and data exchange.

**Implementation.** The json module is implemented in pure Clean (no host bridge). Every host — server, browser, CLI, wasmtime runner — executes the same compiled parser bytecode. This guarantees that the `RUN006`–`RUN010` accept/reject boundary and the pinned `i_*` decisions (see [`./11-testing.md`](./11-testing.md) §Conformance Testing for Standard-Library Parsers) apply uniformly across hosts. Routing JSON through host-native parsers (V8, `serde_json`, etc.) would require certifying each of them against the same conformance corpus on every release; a single stdlib parser certifies once.

```clean
// Core operations
json.textToData(text), json.dataToText(data)
json.tryTextToData(text), json.prettyDataToText(data)
```

#### Parsing JSON

| Call | Returns | Description |
|------|---------|-------------|
| `json.textToData(text)` | any | Parse a JSON string into a Clean value. Raises a runtime error on invalid JSON — see the code list below. |
| `json.tryTextToData(text)` | any | Parse a JSON string; returns `none` on invalid JSON instead of raising. `none` is returned in exactly the conditions where `json.textToData` would raise. |

JSON types map to Clean types:
- JSON object → `pairs<string, any>`
- JSON array → `list<any>`
- JSON string → `string`
- JSON number → `number`
- JSON boolean → `boolean`
- JSON null → `none`

Both functions support nested structures up to 1000 levels of combined array/object nesting. Beyond that limit, `json.textToData` raises `RUN010 JsonDepthExceeded` and `json.tryTextToData` returns `none`.

**`tryTextToData` conflates the document `null` with failure — deliberately.** The valid document `null` maps to `none` (the type-mapping table below), and a parse failure also returns `none`, so the two are indistinguishable from `tryTextToData`'s result alone. This is accepted, not an oversight: the `try*` form exists for callers who treat "no usable value" as one case. A caller that must distinguish them uses `json.textToData` with `onError` — there the document `null` returns `none` and only a failure takes the `onError` arm:

```clean
boolean failed = false
any value = json.textToData(text) onError:
	failed = true
	return none
// failed == false and value == none  ⇔  the document was the valid text `null`
```

**Runtime errors raised by `json.textToData`.** Full definitions live in [`platform/09-error-codes.md`](../03%20platform/09-error-codes.md) and [`platform/10-semantic-rules.md`](../03%20platform/10-semantic-rules.md); the list here is a quick reference.

| Code | When raised |
|------|-------------|
| `RUN006 JsonParseError` | Generic malformed input — used when no more specific code below applies. |
| `RUN007 JsonInvalidNumber` | Number literal malformed or out of `number` range (`1e999`, `.5`, leading zeros, etc.). |
| `RUN008 JsonInvalidString` | String has a bad `\` escape, lone surrogate, invalid UTF-8, or is unterminated. |
| `RUN009 JsonInvalidStructure` | Unmatched bracket/brace, missing/extra comma, trailing data after root, duplicate object key. |
| `RUN010 JsonDepthExceeded` | Nesting exceeded 1000 levels. |

Where the JSON grammar leaves a case open — JSONTestSuite's `i_*` category — the resolution is the condition of the rule that rejects it, in [Platform 10](../03%20platform/10-semantic-rules.md): `RUN007` lists every rejected number form and accepts `-0`, `RUN009` rejects duplicate object keys, `RUN010` fixes nesting at 1000 levels. There is no separate decisions document; a diagnostic's accept/reject boundary is what its rule states ([ADR-0010](../01%20governance/decisions/0010-implementation-defined-parser-decisions.md)).

#### Accessing JSON Data

The `any` type returned by `json.textToData()` supports both dot notation and bracket notation for accessing nested data. **Dot notation is preferred** for its readability; use bracket notation only when dot notation cannot be used (dynamic keys, computed indices).

```clean
// Preferred: Dot notation for field access
any data = json.textToData(jsonString)
any fieldValue = data.fieldName         // Access field using dot notation

// Bracket notation for dynamic keys or indices
any arrayData = json.textToData(arrayJson)
any element = arrayData[0]              // Array element by integer index
any dynamicKey = data[keyVariable]      // Dynamic key from variable

// Chained access for nested structures
any nested = data.user.profile.name     // Preferred: dot notation chain
any item = data.items[0].id             // Mixed: dot notation with array index
```

**Dot Notation on `any` Type:**

On a value of type `any`, dot notation and bracket notation are equivalent: `data.name` and `data["name"]` denote the same access and yield the same value. Dot notation is available whenever the key is a valid identifier; bracket notation is what to use when it is not.

```clean
// These are equivalent:
any name = data.name                    // Preferred - more readable
any name = data["name"]                 // Bracket notation - verbose

// Nested access:
any city = data.user.address.city       // Preferred
any city = data["user"]["address"]["city"]  // Bracket notation
```

**Access Notation Guidelines:**
- **Dot notation** (`data.field`): Preferred for known field names, returns `any`
- **String keys** (`data["key"]`): Use for dynamic/computed keys, returns `any`
- **Integer indices** (`data[0]`): Use for array element access, returns `any`
- **Mixed access**: Combine dot and bracket notation as needed (`data.items[0].name`)
- **Missing fields**: Returns `none` when field doesn't exist or index is out of bounds

```clean
start:
	string jsonText = '{"name": "Alice", "scores": [85, 92, 78], "profile": {"city": "NYC"}}'
	any data = json.textToData(jsonText)

	// Preferred: Dot notation for object fields
	any name = data.name              // Returns "Alice"
	any city = data.profile.city      // Returns "NYC"
	any missing = data.unknown        // Returns none

	// Bracket notation for array access
	any scores = data.scores
	any first = scores[0]             // Returns 85
	any outOfBounds = scores[100]     // Returns none

	// Mixed notation for complex structures
	any firstScore = data.scores[0]   // Returns 85

	// Use default operator for fallback values
	string userName = data.name default "Guest"
	integer score = data.scores[0] default 0
```

#### Serializing to JSON

| Call | Returns | Description |
|------|---------|-------------|
| `json.dataToText(data)` | string | Compact JSON string from a Clean value |
| `json.prettyDataToText(data)` | string | Formatted, human-readable JSON string |

#### Usage Examples

```clean
start:
	// Parse JSON from an API response
	string apiResponse = http.get("https://api.example.com/user/123")
	any userData = json.textToData(apiResponse)

	// Access parsed data (userData is a pairs<string, any>)
	string name = userData["name"] default "Unknown"
	integer age = userData["age"] default 0

	// Safe parsing with tryTextToData
	string maybeJson = getUserInput()
	any parsed = json.tryTextToData(maybeJson)
	if parsed == none
		print("Invalid JSON provided")
	else
		print("Successfully parsed JSON")

	// Parse nested objects with escape sequences
	string nestedJson = "{\"user\": {\"name\": \"Alice\", \"age\": 25, \"address\": {\"city\": \"NYC\"}}}"
	any data = json.textToData(nestedJson)

	// Access nested fields using dot notation
	string userName = data.user.name        // Returns "Alice"
	integer userAge = data.user.age         // Returns 25
	string city = data.user.address.city    // Returns "NYC"

	// Parse nested arrays
	string matrixJson = "{\"matrix\": [[1, 2, 3], [4, 5, 6], [7, 8, 9]]}"
	any matrixData = json.textToData(matrixJson)
	any firstRow = matrixData.matrix[0]     // Returns [1, 2, 3]
	any element = matrixData.matrix[0][1]   // Returns 2

	// Parse arrays of objects
	string usersJson = "[{\"id\": 1, \"name\": \"Alice\"}, {\"id\": 2, \"name\": \"Bob\"}]"
	any users = json.textToData(usersJson)
	any firstUser = users[0]                // Returns {"id": 1, "name": "Alice"}
	string firstName = users[0].name        // Returns "Alice"
	integer secondId = users[1].id          // Returns 2

	// Complex nested structure
	string complexJson = "{\"users\": [{\"id\": 1, \"tags\": [\"admin\", \"staff\"]}, {\"id\": 2, \"tags\": [\"user\"]}]}"
	any complex = json.textToData(complexJson)
	any adminTags = complex.users[0].tags   // Returns ["admin", "staff"]
	string firstTag = complex.users[0].tags[0]  // Returns "admin"

	// Create data and serialize to JSON
	pairs<string, any> user = {}
	user["name"] = "Bob"
	user["email"] = "bob@example.com"
	user["active"] = true

	string jsonString = json.dataToText(user)
	// Result: {"name":"Bob","email":"bob@example.com","active":true}

	// Pretty-print for debugging or config files
	string prettyJson = json.prettyDataToText(user)
	// Result:
	// {
	//   "name": "Bob",
	//   "email": "bob@example.com",
	//   "active": true
	// }

	// Working with JSON arrays
	list<any> items = [1, 2, 3, "four", true, none]
	string arrayJson = json.dataToText(items)
	// Result: [1,2,3,"four",true,null]

	// Nested structures
	pairs<string, any> config = {}
	config["database"] = {}
	config["database"]["host"] = "localhost"
	config["database"]["port"] = 5432
	config["features"] = ["auth", "logging", "caching"]

	file.write("config.json", json.prettyDataToText(config))
```

#### JSON Type Mapping

| JSON Type | Clean Type | Example |
|-----------|------------|---------|
| object | `pairs<string, any>` | `{"key": "value"}` → `pairs` |
| array | `list<any>` | `[1, 2, 3]` → `list<any>` |
| string | `string` | `"hello"` → `"hello"` |
| number | `number` | `3.14` → `3.14` |
| boolean | `boolean` | `true` → `true` |
| null | `none` | `null` → `none` |

**Note on Nested Structures**: The JSON parser fully supports nested objects and arrays. When parsing nested structures, inner objects and arrays are recursively parsed and stored as `any` values within the parent structure.

```clean
// Nested object example
string json = "{\"user\": {\"name\": \"Alice\", \"age\": 25}}"
any data = json.textToData(json)
// data type: pairs<string, any>
// data.user type: any (contains a pairs<string, any>)
// data.user.name type: any (contains a string)

// Nested array example
string arrayJson = "{\"matrix\": [[1, 2], [3, 4]]}"
any matrix = json.textToData(arrayJson)
// matrix type: pairs<string, any>
// matrix.matrix type: any (contains a list<any>)
// matrix.matrix[0] type: any (contains a list<any>)
// matrix.matrix[0][0] type: any (contains a number)
```

#### Nested Structure Support

The JSON parser uses a **recursive architecture** to support arbitrary nesting depth:

**Capabilities:**
- **Deep nesting**: structures nest up to the 1000-level limit stated above; beyond it the parser reports [`RUN010`](../03%20platform/09-error-codes.md#312-runtime-codes-run) rather than exhausting the stack
- **Mixed structures**: Combine objects and arrays at any depth
- **Complex data**: Handle real-world JSON from APIs and config files

**Examples of supported structures:**

```clean
// Deep object nesting (5 levels)
string deepJson = "{\"a\": {\"b\": {\"c\": {\"d\": {\"e\": 42}}}}}"
any deep = json.textToData(deepJson)
integer value = deep.a.b.c.d.e  // Returns 42

// Deep array nesting
string arrayJson = "[[[[[1]]]]]"
any arrays = json.textToData(arrayJson)
integer val = arrays[0][0][0][0][0]  // Returns 1

// Real-world API response
string apiResponse = "{\"data\": {\"users\": [{\"id\": 1, \"profile\": {\"name\": \"Alice\", \"tags\": [\"admin\", \"user\"]}}]}}"
any response = json.textToData(apiResponse)
string name = response.data.users[0].profile.name  // Returns "Alice"
string tag = response.data.users[0].profile.tags[0]  // Returns "admin"
```

**Portability:** the parser is written in Clean and compiled with the program, so its accept/reject boundary is the same on every host. The 1000-level depth limit is part of that boundary — it is a property of the parser, not of the host's stack, and a document that parses on one host parses on all of them.

#### Error Handling

```clean
start:
	// textToData throws on invalid JSON
	string badJson = "{ invalid json }"
	any data = json.textToData(badJson) onError none

	// Or use tryTextToData for none-based error handling
	any safeData = json.tryTextToData(badJson)
	if safeData == none
		print("JSON parsing failed")

	// Use default operator for fallback values
	any result = json.tryTextToData(maybeJson) default {}
```

### Matrix Module

`matrix<T>` is a built-in type ([4 — Type System](./04-type-system.md)), so its arithmetic is written with operators and the language defines what they mean ([6 — Expressions §Operators on built-in types](./06-expressions.md#operators-on-built-in-types)). This section is the home of the whole surface.

**Operators.** Defined between two `matrix<T>` values of the same element type:

| Written | Means | Requires |
|---------|-------|----------|
| `A * B` | Matrix multiplication | columns of `A` == rows of `B` |
| `A + B` | Element-wise addition | identical shapes |
| `A - B` | Element-wise subtraction | identical shapes |

**Functions.**

| Function | Returns | Description |
|----------|---------|-------------|
| `A.transpose()` | `matrix<T>` | Rows become columns. Defined for any shape and any element type. |
| `A.determinant()` | `number` | The determinant. Requires a square `matrix<number>`. |
| `A.inverse()` | `matrix<number>` | The multiplicative inverse. Requires a square, non-singular `matrix<number>`. |

**`determinant()` and `inverse()` are defined on `matrix<number>` only.** An inverse generally has fractional entries, so a single rule covering `matrix<integer>` would have to either lose information or return a different element type than it received. Calling either on a matrix of any other element type is [`SEM004`](../03%20platform/09-error-codes.md#32-semantic-codes-sem); convert the matrix first.

**Failure modes.** `matrix<T>` is dynamically sized, so shape is not part of the type and no shape error can be caught before the values exist:

| Situation | Diagnostic |
|-----------|-----------|
| `A * B` where columns of `A` ≠ rows of `B` | [`RUN016`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| `A + B` or `A - B` on differing shapes | [`RUN016`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| `determinant()` or `inverse()` on a non-square matrix | [`RUN016`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| `inverse()` where the determinant is zero | [`RUN017`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| `determinant()` or `inverse()` on a non-`number` element type | [`SEM004`](../03%20platform/09-error-codes.md#32-semantic-codes-sem) — compile time |

```clean
matrix<number> a = [[1.0, 2.0], [3.0, 4.0]]
matrix<number> b = [[0.0, 1.0], [1.0, 0.0]]

matrix<number> product = a * b
matrix<number> sum = a + b
number d = a.determinant()
matrix<number> inv = a.inverse()
```

**Every matrix operation is guest computation.** None of it crosses the host bridge. The reasoning is the one that keeps format parsers out of the bridge ([Platform 02 §2.2.1](../03%20platform/02-host-bridge.md#221-portable-l2-in-every-world)): routing pure computation through the host would make the result depend on which host is running, and for floating-point arithmetic that means two hosts could return different numbers for the same matrix.

---

### Time Module

The `time` namespace is the only way to obtain a `datetime` value inside a Clean program.

| Function | Returns | Description |
|----------|---------|-------------|
| `time.now()` | `datetime` | The current wall-clock instant, with UTC offset |
| `time.parse(text)` | `datetime?` | Parse an RFC 3339 timestamp (`"2026-01-15T10:30:00Z"`); `none` if `text` is not one |

Reading a clock is a host capability, not pure computation. It is available in **every** world, because the portable bridge catalog already carries `wasi:clocks/wall-clock` ([Platform 02 §2.2.1](../03%20platform/02-host-bridge.md#221-portable-l2-in-every-world)) — so unlike `file` and `http`, `time` needs no scope note and a component using it links anywhere.

`time.now()` MUST NOT be called from a `compiletime` function. A compile-time result that varies with when it was compiled would break build reproducibility, and the block-handler sandbox stubs the import to an error ([21 §21.7](./21-block-handlers.md#217-compile-time-execution-environment)).

### Bytes Module

The `bytes` namespace builds and reads the `bytes` type ([4 — Type System](./04-type-system.md)). Most `bytes` values arrive from a host function; these two exist so a program can also produce and inspect one.

| Function | Returns | Description |
|----------|---------|-------------|
| `bytes.fromText(text)` | `bytes` | The UTF-8 encoding of `text` |
| `bytes.toText(data)` | `string?` | `data` decoded as UTF-8; `none` if it is not well-formed UTF-8 |

**Operations on a `bytes` value** (the compiler-side contract is [Platform 14 §14.14.2](../03%20platform/14-compiler-architecture.md#14142-first-class-bytes-type)):

| Call | Returns | Description |
|------|---------|-------------|
| `data.length()` | integer | Number of bytes |
| `data[i]` | integer | The byte at position `i` (0–255); out of range raises [`RUN013`](../03%20platform/09-error-codes.md#312-runtime-codes-run) |
| `data.slice(start, end)` | bytes | New value from `start` to `end`; indices clamp exactly like [`string.substring`](#string-module) |
| `a + b` | bytes | Concatenation |
| `a == b` | boolean | Byte-exact comparison |

Indexes here count **bytes** — `bytes` is the type whose unit is the byte, which is precisely what distinguishes it from `string`, whose indexes count code points. A `bytes` literal is written `b"…"` — the UTF-8 bytes of the text, with the string escapes plus `\xNN` for arbitrary bytes ([3 — Lexical Structure §LEX-06](./03-lexical-structure.md), grammar: [`03-lexical-structure.ebnf.md` §7](./grammar/03-lexical-structure.ebnf.md)).

There is no constructor taking a list of numbers. The natural spelling would have been a list of unsigned 8-bit integers, and no such type exists in the surface language ([ADR-0019](../01%20governance/decisions/0019-precision-modifiers.md)); `bytes` is itself the byte-buffer type, so a list of bytes would be a second way to say the same thing.

`bytes.toText` returns an optional because the guarantee that Clean strings are UTF-8 holds inside the language ([TXT-01](../03%20platform/17-text-encoding.md#txt-01--every-ecosystem-text-file-is-utf-8), [Platform 03 §3.3](../03%20platform/03-memory-model.md)) and cannot hold for bytes that came from outside it.

### Validator Module

The `validator` namespace provides declarative, field-level input validation: a rules DSL for field constraints and a `ValidationResult` type that forces the caller to handle the failure case. Its specification is owned by [Platform 11 — Standard Library: Validator Module](../03%20platform/11-stdlib-validator.md), the single source of truth for the namespace; this entry registers it in the standard-library catalog.

---

### STD-01 — The standard-library catalog

These are the modules of the standard library. Every one is available without an import ([17 — Modules and Imports](./17-modules-and-imports.md)); this list is their single home.

| Module | Surface |
|--------|---------|
| `console` | `print`, `print:`, `input` — standard output and input |
| `math` | Advanced mathematics; basic arithmetic uses operators |
| `string` | Text manipulation, plus the named [string patterns](#string-patterns) |
| `list` | Collection operations |
| `file` | Filesystem access, unsandboxed |
| `http` | Outgoing HTTP, in the worlds that grant it |
| `json` | `textToData` / `dataToText` and their tolerant variants |
| `time` | `now`, `parse` — the constructors of `datetime` |
| `bytes` | `fromText`, `toText` — building and reading the `bytes` type |
| `matrix` | Operators and functions of the built-in `matrix<T>` type |
| `validator` | Declarative field validation (owned by [Platform 11](../03%20platform/11-stdlib-validator.md)) |

`time` and `bytes` were cited across the ecosystem with no specified surface anywhere; both are now modules of this catalog ([ADR-0021](../01%20governance/decisions/0021-time-and-bytes-namespaces.md)).

---

## Changelog

- 2026-08-20 — **§Conversions**: `number.toString()`'s four presentation edges closed, from the compiler's Milestone 8 post-work (`clean-language-compiler/docs/DISCOVERIES-M8.md` §10, via [work/2026-08-20-number-tostring-notation-thresholds.md](../work/2026-08-20-number-tostring-notation-thresholds.md)) — the four edges closed as the brief spelled them: plain decimal exactly for −4 ≤ E ≤ 21 in the normalized-scientific convention (`0.0001` plain / `1e-5` scientific / `10²¹` plain / `10²²` scientific as pinning examples; the brief's own `0.d₁d₂…` statement of the convention was the off-by-one it warned against), integral values render a trailing `.0` (`42.0` promoted from illustration to contract), non-finite spellings `NaN` / `Infinity` / `-Infinity`, and negative zero renders `-0.0`. The scientific spelling itself (single-digit mantissa carries no `.0`, lowercase `e`, no `+`, no leading zeros) stated so the boundary examples are byte-exact. The unpinned sub-question decided **outputs-only**: `string.toNumber()` rejects the non-finite spellings as `RUN003` parse failures, keeping the literal grammar the sole input domain and matching `RUN007`'s refusal of non-finite readings at the JSON boundary; the `toString ∘ toNumber` identity is stated as scoped to finite values. *(Correction, same day: edges (b)–(d) were ratifications of the compiler's in-force adoption; edge (a) was not — landing the decision (compiler commit `fc0385a`) revealed the in-force renderer and oracle applied the threshold in the `0.d₁d₂…` convention, the off-by-one this entry's parenthetical flags, i.e. −5 ≤ E ≤ 20 normalized: `1e-5` rendered plain and `10²¹` scientific, the reverse of this rule. The compiler migrated both, shifting the boundary one position; the migration note in the brief's closure records the exact flipped renderings. The normative text above is unchanged.)*
- 2026-08-20 — **§Math**: the number-division boundary stated, from the compiler's Milestone 8 post-work (`clean-language-compiler/docs/DISCOVERIES-M8.md` §9, via [work/2026-08-20-runtime-error-message-wordings.md](../work/2026-08-20-runtime-error-message-wordings.md)): `number` `/` is `f64.div` and never raises (`1.0 / 0.0` → `Infinity`, `0.0 / 0.0` → NaN, per IEEE 754); only `integer` arithmetic raises [`RUN003`](../03%20platform/10-semantic-rules.md#run003--arithmetic-error), whose upgraded entry owns the raise-site list and wordings. No chapter had said this; RUN003's old stub ("division by zero … at runtime") read as covering `1.0 / 0.0`.
- 2026-08-19 — Six errata from the compiler's Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, items 6d, 6e, 6f, 6g periphery, 6h/6i, 6n), each ratifying a fixture-pinned compiler adoption or closing a contract the module tables left open. **§Conversions**: the table declared exhaustive (`string.toBoolean()` does not exist — `SEM022`); `number.toString()` contracted to shortest-round-trip decimal in the literal grammar; `string.toNumber()` contracted to correctly-rounded parsing (IEEE 754 `roundTiesToEven`), never per-digit accumulation. **§Math**: `math.round` is ties-to-even (`f64.nearest`); domain errors are NaN, never a raised failure; `max`/`min` follow `f64.max`/`f64.min` (NaN propagates, `-0.0 < +0.0`). **§String**: indexes and lengths count Unicode code points (the chapter never named a unit; bytes and UTF-16 units both rejected with reasons), plus the edge-case contract — `substring` clamps, `charAt`/`charCodeAt` out of range are `RUN013`, trim whitespace is ASCII `{space, \t, \n, \r}`, `replace("")`/empty-pad return the receiver, `split("")` yields one element. **§List**: `slice` clamps like `substring`; `sort()` and value search (`contains`/`indexOf`/`lastIndexOf`) restricted to element types with defined order/equality — class-typed elements are `SEM004`. **§JSON**: `tryTextToData`'s conflation of the document `null` with failure stated as deliberate, with `textToData onError` as the distinguishing form. **§Bytes**: the value operations (`length()`, `data[i]` with `RUN013`, clamping `slice`, `+`, `==`) registered here as the surface home, aligned with the corrected [Platform 14 §14.14.2](../03%20platform/14-compiler-architecture.md#14142-first-class-bytes-type) (whose `to_bytes`/`to_string(encoding)` rows are withdrawn in this module's favour); byte-unit indexing stated; the `b"…"` literal cross-referenced to its new grammar production.
- 2026-08-05 — §HTTP Module scope note: WASI interface name updated from `wasi:http/outgoing-handler` (WASI 0.2) to `wasi:http/handler@0.3.0` (WASI 0.3) — Preview 3 sweep debt from [ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md); no behavioral change to `http.*`.
- 2026-08-02 — The JSON module's implementation-defined note points at the rule conditions in [Platform 10](../03%20platform/10-semantic-rules.md) instead of a decisions document that never existed ([ADR-0010](../01%20governance/decisions/0010-implementation-defined-parser-decisions.md)). `RUN009`'s row states that a duplicate object key is rejected, rather than deferring to a "strict decision" recorded nowhere.

- 2026-08-02 — Two ADRs closed here. [ADR-0020](../01%20governance/decisions/0020-console-input-failure-behaviour.md): every `input` form is **fallible** and returns an optional, `none` on end of input or unparseable text; the internal re-prompt loop is withdrawn as unimplementable — it cannot terminate on a closed stream — and re-prompting moves into the program, where a loop that never ends is at least visible. [ADR-0021](../01%20governance/decisions/0021-time-and-bytes-namespaces.md): new **§Time Module** and **§Bytes Module**, registered in [STD-01](#std-01--the-standard-library-catalog). `datetime` had been a core type with no way to construct a value. `time` needs no world scope note, because the portable bridge already carries `wasi:clocks/wall-clock`. `bytes` gains `fromText`/`toText` and no list-of-numbers constructor, the narrow integer type it would have required having been withdrawn by [ADR-0019](../01%20governance/decisions/0019-precision-modifiers.md).

- 2026-08-02 — **§Matrix Module** added and registered in [STD-01](#std-01--the-standard-library-catalog), closing part of [ADR-0018](../01%20governance/decisions/0018-matrix-operator-overloading.md). `matrix<T>`'s surface had no home anywhere: [6 — Expressions](./06-expressions.md) named `transpose`, `inverse` and `determinant` in an example and no chapter defined them, their error paths, or their element-type constraints. `determinant()` and `inverse()` are fixed to `matrix<number>`, their shape failures are the new [`RUN016`](../03%20platform/09-error-codes.md#312-runtime-codes-run)/[`RUN017`](../03%20platform/09-error-codes.md#312-runtime-codes-run), and the whole surface is stated to be guest computation that never crosses the bridge — for the reason that already keeps format parsers off it.

- 2026-08-01 — Fase 5 (zero-debt pass): the dot-notation desugaring described as an equivalence rather than as a compiler action (SDD-02); the module blurbs rewritten as descriptions instead of capability claims (SDD-05).
- 2026-08-01 — Fase 3/4 (L16, L19, L24): new **§Console Module** (promoted from [7 — Statements](./07-statements.md), where 160 lines of library surface lived inside a syntax chapter) and **§Conversions** (`toString`/`toInteger`/`toNumber`/`toBoolean`, cited from three chapters and defined in none). The catalog section added as the single home of the module list — [17](./17-modules-and-imports.md) and the libraries specification each carried a divergent copy. JSON nesting depth settled at **1000**, the figure [`RUN010`](../03%20platform/09-error-codes.md#312-runtime-codes-run) backs; the file also claimed "unlimited" and "~50, limited by WebAssembly stack depth", the last of which made the accept boundary host-dependent. `string.join` retired in favour of `list.join` (the chapter registered both while [16](./16-method-style-syntax.md) forbade aliases). The parser's three `__json_parse_*` internals removed (SDD-02, and underscore names are a rejected form). `remove()`/`peek()` registered as the behavior-dependent operations [4](./04-type-system.md) relies on. Rule `STD-01` minted.
- 2026-08-01 — String module: declared the pattern vocabulary per [ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md) (Accepted 2026-08-01) — fourteen named pattern constants (the twelve of Platform 11 §2.3 plus `alphaPattern` and `numericPattern`, per the ADR's companion decision) and the `string.matches(pattern)` surface, in the new *Normative* [String Patterns](#string-patterns) subsection; this chapter is now the vocabulary's single home. `text.matches(pattern)` added to the method-style table.
- 2026-08-01 — Added the `validator` namespace to the standard-library catalog, citing [Platform 11](../03%20platform/11-stdlib-validator.md) as the home of the module (conflict-log P14). Added Status header and Changelog section.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users looking up any built-in module (`console`, `math`, `string`, `list`, `file`, `http`, `json`, `time`, `bytes`, `matrix`, `validator`)
- **Rule prefix:** `STD-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Type System](./04-type-system.md), [Method-Style Syntax](./16-method-style-syntax.md), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md), [Platform 11 — Validator Module](../03%20platform/11-stdlib-validator.md)
