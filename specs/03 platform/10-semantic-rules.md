# Platform 10. Semantic Rules

This file is the rule catalogue: for each numbered diagnostic code the compiler and runtime can emit, it defines the normative condition, the message template, the primary label, a passing example, a failing example, and the suggested fix. It is the single source of truth for what each rule requires — pair it with [09 — Error Codes](./09-error-codes.md) (the code registry) and [13 — Diagnostic Format](./13-diagnostic-format.md) (the wire format and rendering). Every rule here has a code of the form `PREFIX###` (e.g. `SYN001`, `SEM014`, `LIB011`); the `RUL-` prefix identifies only the *meta-rules* of this catalog, not the diagnostic rules themselves.

### RUL-01 — Mandatory entry format


Every rule in this catalog MUST use the same structure:

- **Rule code and name** as the heading.
- **Condition** — the normative statement (MUST / SHOULD / MAY per RFC 2119).
- **Message template** — the exact string the compiler emits, with `{placeholder}` slots.
- **Primary label** — the phrase attached to the error's primary span.
- **Example (passes)** — minimal code that satisfies the rule.
- **Example (fails)** — minimal code that violates the rule.
- **Suggested fix** — the code action the compiler offers, per [13 §5](./13-diagnostic-format.md).

### RUL-02 — 1:1 with the code registry


Every rule in this file MUST have a matching code row in [`09-error-codes.md §3`](./09-error-codes.md), and every registered code MUST have its rule here — the reciprocal obligation of [ERC-02](./09-error-codes.md#erc-02--one-code-one-rule), which also declares the single exception (the `BLOCK` range's rule bodies live in [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md)).

### RUL-03 — A stub is a spec bug


Rules that predate [RUL-01](#rul-01--mandatory-entry-format) (one-line stubs) are being upgraded incrementally. A stub MUST be treated as a spec defect — file it via `report_error` on `component=spec`; new rules MUST NOT be added as stubs.

---

## 1. Error Code Ranges

Code ranges are owned by [`09-error-codes.md §1`](./09-error-codes.md); this file does not restate the range table. The rules in this file cover:

- SYN — §2 (SYN001–SYN010 plus SYN100/SYN101)
- SEM — §3 (SEM001–SEM028)
- SCOPE — §4, FUNC — §5 (FUNC001–FUNC015; FUNC001 withdrawn, folded into SEM019), CLASS — §6 (CLASS001–CLASS012; CLASS007 withdrawn — see its entry), IDX — §7, STATE — §8
- IMPORT — §9 (IMPORT001–IMPORT004; IMPORT005 withdrawn, folded into IMPORT001)
- LIB — §10 (LIB001–LIB020; LIB005, LIB007, LIB008 and LIB009 withdrawn — see their entries)
- COM — §11 (COM001–COM017)
- BLD — §12 (BLD001)
- RUN — §13 (RUN001–RUN019)
- MEM — §14 (MEM001–MEM003)
- CFG — §15 (CFG001–CFG005)
- RQD — §16 (RQD001–RQD002)
- CAP — §17 (CAP001–CAP003)

The BLOCK range's rule bodies live in [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md), not here ([ERC-02](./09-error-codes.md#erc-02--one-code-one-rule)).

Sections §2–§16 below are the rule catalog and are *Normative*.

---

## 2. Syntax Rules (SYN)


### SYN001 — Invalid Token
The lexer encountered a character that is not valid in any context.

### SYN002 — Unexpected Token
A token appears where the grammar does not allow it.

### SYN003 — Invalid Indentation
Indentation uses spaces instead of tabs, or the nesting level is inconsistent with the enclosing block.

### SYN004 — Unterminated Construct
A string literal, comment, or block was opened but not closed before end of file.

### SYN005 — Malformed Construct
A syntax structure is partially correct but missing required elements.

### SYN006 — Indentation Error
Tab/space mixing detected, or indentation does not match the expected level for the current block.

### SYN007 — Section Out of Order

Top-level sections must appear in the order that [08 — File Structure](../04%20language/08-file-structure.md) defines. That chapter is the home of the order, of which forms are top-level at all, and of where a library's framework blocks sit; this entry does not restate it.

**Example (fails — SYN007):**
```clean
start:                              // error: 'start:' must appear last
	print(add(2, 3))

functions:
	integer add(integer a, integer b)
		return a + b
```

### SYN008 — Invalid Print Block

A `print:` block contains no expressions, or contains non-expression statements.

**Condition:** A `print:` block is parsed but the indented body contains no expressions, or one of the items in the body is not a valid expression.

**Example (fails — SYN008):**
```clean
print:
	// no expressions here
```

**Message:** `"print: block must contain at least one expression"`

---

### SYN009 — Not A Top-Level Form

A construct appears at the top level of a `.cln` file that is not one of the permitted top-level sections. Distinct from [SYN007](#syn007--section-out-of-order), which fires when a *permitted* section appears in the wrong position.

**Condition:** the parser reaches a top-level construct whose head is not one of the forms listed in [08 — File Structure](../04%20language/08-file-structure.md), and not a framework block contributed by a library in scope.

**Message:** `"'{construct}' cannot appear at the top level of a file"`

**Primary label:** `not a top-level form`

**Example (fails — SYN009):**
```clean
integer x = 5
print("hello")
```

**Example (passes):**
```clean
start:
	integer x = 5
	print("hello")
```

**Suggested fix:** move the statement inside `start:`, a function, or another block that admits statements.

### SYN010 — Missing Parentheses

A method or function call is written without parentheses.

**Condition:** an identifier in call position — after a `.` on a value, or as a standalone name resolving to a callable — is not followed by an argument list.

**Message:** `"Call to '{name}' is missing parentheses"`

**Primary label:** `add ()`

**Example (fails — SYN010):**
```clean
string text = value.toString
```

**Example (passes):**
```clean
string text = value.toString()
```

**Suggested fix:** append `()`. Every call carries parentheses ([02 — Language Design Rules](../04%20language/02-language-design-rules.md) Rule 2); the `math` constants are the only parenthesis-free members.

### SYN100 — Missing Spec Path

The `spec` AI-metadata statement was parsed but the required string-literal path is missing or is not a string literal.

The `spec` statement links a function (or other declaration) to its specification document. The argument must be a string literal naming the relative or absolute path of the spec; identifiers, numbers, and unquoted paths are not accepted.

**Condition:** The `spec` keyword token is consumed, and the next non-whitespace token is not a string literal.

**Example (fails — SYN100):**
```clean
functions:
	void greet()
		spec       // ← missing string literal path
		print("hi")
```

**Example (passes):**
```clean
functions:
	void greet()
		spec "documentation/greet.md"
		print("hi")
```

**Message:** `"Expected string literal after 'spec'"`

---

### SYN101 — Missing Intent Description

The `intent` AI-metadata statement was parsed but the required string-literal description is missing or is not a string literal.

The `intent` statement records a natural-language description of a function's purpose so AI tooling can reason about callers without re-reading the body. The argument must be a string literal; identifiers, numbers, and unquoted descriptions are not accepted.

**Condition:** The `intent` keyword token is consumed, and the next non-whitespace token is not a string literal.

**Example (fails — SYN101):**
```clean
functions:
	void greet()
		intent     // ← missing string literal description
		print("hi")
```

**Example (passes):**
```clean
functions:
	void greet()
		intent "Print a friendly greeting to stdout."
		print("hi")
```

**Message:** `"Expected string literal after 'intent'"`

---

## 3. Semantic Rules (SEM)


### SEM001 — Assign Type Mismatch
The right-hand side of an assignment produces a value whose type is not assignable to the left-hand side's declared type.

**Headline template:** `"type mismatch in assignment"`
**Primary label:** `` "`<name>` is declared with type `<declared>`" `` — placed at the variable name.
**Secondary label:** `` "this expression has type `<actual>`" `` — placed at the RHS.
**Suggestion:** if the two types are related by a total conversion (e.g. `integer` → `number`), emit a `MachineApplicable` suggestion replacing the declared type; otherwise emit a `MaybeIncorrect` suggestion replacing the RHS with `<rhs>.to<Declared>()` when such a conversion exists.

**Example (passes):**
```clean
integer x = 42
string name = "Alice"
```

**Example (fails — SEM001):**
```clean
integer x = "hello"     // string is not assignable to integer
```

### SEM002 — Undefined Variable
A variable name appears in an expression or on the left-hand side of an assignment but no `state:`, parameter, or local declaration for it is in scope. This is distinct from [`SCOPE001`](#scope001--variable-must-be-declared-before-use), which fires when a declaration for the name *does* exist in the scope but lexically after the use; SEM002 fires when no declaration exists at all.

**Headline template:** `` "I cannot find a variable named `<name>` in scope" ``
**Primary label:** `"no variable with this name exists here"`
**Help:** if the name is within edit distance 2 of any in-scope name, list the top three closest matches.
**Suggestion:** one `MaybeIncorrect` suggestion per close match, replacing the reference. Never `MachineApplicable` — the compiler cannot verify intent from a name alone.

**Example (fails — SEM002):**
```clean
start:
	print(x)    // no variable named `x` is in scope
```

### SEM003 — Symbol Redefinition
A symbol (variable, function, class) is declared more than once in the same scope.

**Applies to:**
- Variable declared twice in same scope
- Function defined twice at top level
- Class defined twice
- Duplicate import items

This is distinct from two neighbouring rules: [`SCOPE002`](#scope002--variable-cannot-be-redeclared-in-same-scope) is the resolver-phase form of the variable case (the resolver emits SCOPE002 when it catches the redeclaration during name resolution; SEM003 covers redefinitions surfaced during HIR validation), and [`IMPORT004`](#import004--duplicate-import-item) is the module-resolver form of the duplicate-import case. One violation produces exactly one diagnostic — the phase that detects it first owns it.

**Example (fails — SEM003):**
```clean
functions:
	integer add(integer a, integer b)
		return a + b
	integer add(integer x, integer y)    // redefinition
		return x + y
```

### SEM004 — Invalid Operation for Type
An operator or operation is applied to a type that does not support it.

**Condition:** any of — an operator is applied to an operand type for which [6 — Expressions §Operators on built-in types](../04%20language/06-expressions.md#operators-on-built-in-types) defines no meaning; the source of an `iterate` is not an iterable type (list, matrix, string, or range); an operand of string interpolation or of a `print:` line has no textual form.

**Message templates:**
- `` "operator `<op>` is not defined for type `<T>`" `` — the operator case (one wording for all operators; there is no separate "arithmetic is not defined" variant).
- `` "cannot iterate over a value of type `<T>`" `` — non-iterable `iterate` source.
- `` "value of type `<T>` has no text form" `` — interpolation / `print:` operand.

**Primary label:** `not defined for this type`

**Example (fails — SEM004):**
```clean
string result = "hello" - "world"    // operator `-` is not defined for type `string`
```

**Example (passes):**
```clean
string result = "hello" + "world"    // + on two strings is concatenation
```

**Suggested fix:** use an operation the operand types define ([6 §Operators on built-in types](../04%20language/06-expressions.md#operators-on-built-in-types)), or convert the operand explicitly first.

### SEM005 — Access Violation

A private member is accessed from outside its allowed scope.

**Visibility model:** Members are **private by default**. The **only** visibility construct is a `public:` sub-section. There is no `private:` keyword. Any declaration NOT inside a `public:` block is private.

**Applies to:**
- A module function NOT inside a `public:` sub-section of `functions:` is called from an importing module
- A class field NOT inside the class's `public:` field sub-section is read or written outside the class
- A class method NOT inside a `public:` sub-section of the class's `functions:` block is called outside the class
- A state variable NOT inside a `public:` sub-section of `state:` is read or written from an importing module

**Example (fails — SEM005, module function):**
```clean
// file: mymodule.cln
functions:
	public:
		integer publicFunc()
			return helper()

	// Private helper — no `public:` prefix, so private by default
	integer helper()
		return 42

// file: main.cln
import:
	mymodule

start:
	integer x = helper()    // error: helper is private to mymodule
```

**Example (fails — SEM005, class field):**
```clean
class BankAccount
	// Private field — no `public:` prefix
	number balance = 0

	public:
		string owner

start:
	BankAccount acc = BankAccount("Alice", 100.0)
	number b = acc.balance    // error: balance is private to BankAccount
```

**Example (fails — SEM005, class method):**
```clean
class BankAccount
	functions:
		// Private helper — no `public:` prefix
		boolean isValid(number amount)
			return amount > 0

		public:
			void deposit(number amount)
				if isValid(amount)
					balance = balance + amount

start:
	BankAccount acc = BankAccount("Alice", 100.0)
	boolean ok = acc.isValid(50.0)    // error: isValid is private to BankAccount
```

**Message:** `"'{name}' is private and cannot be accessed from outside '{scope}'"`

### SEM006 — Inheritance Error
A class inheritance declaration is invalid.

**Applies to:**
- Parent class does not exist
- Parent is not a class type (e.g., trying to inherit from a primitive)

**Example (fails — SEM006):**
```clean
class Child is NonExistentParent    // parent not defined
	integer x
```

### SEM007 — Generic Type Error
A generic or polymorphic type operation is invalid.

**Applies to:**
- Incompatible type arguments
- Tuple size mismatch
- Union type incompatibility

### SEM008 — Inheritance Cycle
A class inherits from itself directly or indirectly, creating a cycle.

**Example (fails — SEM008):**
```clean
class A is B
class B is A    // circular
```

### SEM009 — Invalid Type Specification
A type expression does not denote a valid type.

**Condition:** any of — a type name does not refer to a declared type; `?` is applied to a type that is already optional (absence does not stack); an integer width outside the host-signature set is used in a Clean declaration; a [TYP-05](../04%20language/04-type-system.md) behavior chain is invalid (a second removal discipline, or a repeated `.unique`).

**Message templates:**
- `` "`<name>` is not a defined type" `` — unknown type name.
- `` "`<T>?` is already optional: absence does not stack" `` — repeated `?`.
- `` "integer width `<name>` exists only in host signatures" `` — width outside host signatures.
- `` "invalid behavior chain: <reason>" `` — invalid TYP-05 chain.

**Primary label:** `not a valid type`

**Example (fails — SEM009):**
```clean
FooBar x = 42        // `FooBar` is not a defined type
string?? label       // `string?` is already optional: absence does not stack
```

**Example (passes):**
```clean
string? label = none
```

**Suggested fix:** name a declared type; write `?` once; keep host-only integer widths in `host function` signatures; state each list behavior at most once.

---

### SEM010 — Invalid Match Pattern
The argument passed to `.matches()` is not one of the pattern constants declared by the standard library.

**Condition:** The argument to `string.matches()` MUST be one of the named pattern constants declared in [15 — Standard Library §String Patterns](../04%20language/15-standard-library.md#string-patterns) (`emailPattern`, `urlPattern`, …, `alphaPattern`, `numericPattern`) — an ordinary identifier check performed at compile time; no runtime lookup. The former bare string names (`"email"`, `"url"`, …) and the pattern packs that extended them (`import: validate.patterns.financial`) are **retired** ([ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md), Accepted 2026-08-01). This rule does not define the constant list — the standard library chapter is its single home.

**Example (passes):**
```clean
boolean ok = someString.matches(emailPattern)
```

**Example (fails — SEM010):**
```clean
boolean ok = someString.matches("email")   // bare names are retired — use emailPattern
```

**Also fails — SEM010:** any expression that is not a declared pattern constant:
```clean
string pat = "email"
boolean ok = someString.matches(pat)   // a variable is not a pattern constant
```

**Fix:** Use one of the pattern constants declared in [15 — Standard Library §String Patterns](../04%20language/15-standard-library.md#string-patterns). Extending the vocabulary is a spec change to that section, not an import.

---

### SEM011 — Missing Capability Method

A class declares `can C` but does not implement one of C's required (non-default) methods.

**Example (fails — SEM011):**
```clean
can Draw:
	draw()

class Circle can Draw     // ERROR: missing method draw()
	public:
		number radius
```

**Fix:** Add the missing method to the class's `functions: public:` block, or remove the capability from the `can` clause.

---

### SEM012 — Undefined Capability

A `can` clause on a class, or a type reference in a parameter/return position, names a capability that has not been declared.

**Example (fails — SEM012):**
```clean
class Circle can Fly    // ERROR: capability Fly not defined
```

**Fix:** Declare the capability with a `can Fly:` block, or correct the spelling.

---

### SEM013 — Capability Method Signature Mismatch

A class implements a capability method, but its parameter types or return type do not match the capability's signature.

**Example (fails — SEM013):**
```clean
can Save:
	save(string path)

class Circle can Save
	functions:
		public:
			save(integer path)    // ERROR: expected string path, got integer
				print(path)
```

**Fix:** Change the class's method signature to match the capability exactly.

---

### SEM014 — Capability Body Not Allowed

A `can` block declaration includes a method body. Capabilities in Clean Language are pure contracts — signatures only, never bodies (see [14 — Classes and Objects §Capabilities](../04%20language/14-classes-and-objects.md)).

**Example (fails — SEM014):**
```clean
can Describe:
	describe() -> string
	tag() -> string
		return "default"             // ERROR: capability methods cannot have bodies
```

**Fix:** Remove the method body from the capability. If a shared default is needed, put the implementation on each conforming class, or expose a top-level function the classes can call.

---

### SEM015 — Return Type Mismatch
The type of a returned expression does not match the function's declared return type.

**Headline template:** `` "return type mismatch in `<fn>`" ``
**Primary label:** `` "this expression has type `<actual>`" `` — placed at the returned expression.
**Secondary label:** `` "function declares return type `<declared>`" `` — placed at the return type in the signature.
**Suggestion:** if a total conversion exists (`integer` → `number`), emit a `MachineApplicable` replacement that wraps the returned expression; otherwise `MaybeIncorrect`.

**Example (fails — SEM015):**
```clean
functions:
	integer double(integer n)
		return n * 1.0    // returns `number`, declared `integer`
```

---

### SEM016 — Argument Type Mismatch
An argument value's type does not match the corresponding parameter's declared type.

**Headline template:** `` "argument `<n>` of `<fn>` has the wrong type" ``
**Primary label:** `` "this argument has type `<actual>`" `` — placed at the argument expression.
**Secondary label:** `` "parameter `<name>` is declared with type `<declared>`" `` — placed at the parameter in the signature.

**Example (fails — SEM016):**
```clean
functions:
	integer add(integer a, integer b)
		return a + b

start:
	integer x = add(1, "two")    // argument 2 has type `string`, expected `integer`
```

---

### SEM017 — State Initializer Type Mismatch
A `state:` initializer produces a value whose type does not match the declared state type.

**Headline template:** `` "state initializer for `<name>` has the wrong type" ``
**Primary label:** `` "this initializer has type `<actual>`" `` — placed at the initializer.
**Secondary label:** `` "`<name>` is declared with type `<declared>`" `` — placed at the state declaration.

**Example (fails — SEM017):**
```clean
state:
	integer count = "zero"    // initializer is `string`, declared `integer`
```

---

### SEM018 — Computed Body Type Mismatch
The body of a `computed:` state declaration produces a value that does not match the declared type. SEM018 owns the type-mismatch case; a circular dependency between computed state declarations is [`STATE003`](#state003--circular-state-dependency), never SEM018.

**Headline template:** `` "computed state `<name>` returns the wrong type" ``
**Primary label:** `` "this body evaluates to `<actual>`" `` — placed at the computed body.
**Secondary label:** `` "`<name>` is declared with type `<declared>`" `` — placed at the computed declaration.

**Example (fails — SEM018):**
```clean
state:
	computed:
		integer total
			return "not an integer"    // body is `string`, declared `integer`
```

---

### SEM019 — Undefined Function
A function name is called with parentheses but no top-level or imported `functions:` declaration for it is in scope. This is distinct from [`FUNC001`](#func001--function-must-be-defined-before-use), which fires when a definition for the name *does* exist but the call precedes it; SEM019 fires when no definition exists at all.

**Headline template:** `` "I cannot find a function named `<name>`" ``
**Primary label:** `"no function with this name is in scope"`
**Help:** if the name is within edit distance 2 of any in-scope function, list the top three closest matches. If a matching name is available in a module that is not imported, suggest the import.
**Suggestion:** `MaybeIncorrect` per close match; `MachineApplicable` for a missing `import:` line when the name is uniquely available in one importable module.

**Example (fails — SEM019):**
```clean
start:
	integer n = lenght(users)    // did you mean `length`?
```

---

### SEM020 — Undefined Class
A class name is used in a `new` expression or type annotation but no class of that name is declared.

**Headline template:** `` "I cannot find a class named `<name>`" ``
**Primary label:** `"no class with this name is in scope"`
**Suggestion:** same shape as SEM019 — closest-match `MaybeIncorrect` suggestions plus a `MachineApplicable` import fix when the class exists in exactly one importable module.

**Example (fails — SEM020):**
```clean
start:
	User u = new Users()    // did you mean `User`?
```

---

### SEM021 — Undefined Module
A module name in an `import:` list cannot be resolved by the module resolver.

**Headline template:** `` "I cannot resolve the module `<name>`" ``
**Primary label:** `"no source or library provides this module"`
**Help:** if the module could be provided by a library that is not in `clean.toml [dependencies]`, suggest adding it. This overlaps with [`LIB001`](#lib001--library-not-found) — SEM021 fires when the resolver failed on a name; LIB001 fires when a declared dependency was not installable.
**Suggestion:** `MachineApplicable` — add the missing entry to `clean.toml [dependencies]` when the name maps uniquely to a known library.

**Example (fails — SEM021):**
```clean
import:
	orders.checkout    // no such module found
```

---

### SEM022 — Undefined Method
A method call `receiver.method()` references a method that does not exist on the receiver's type. This is distinct from [`FUNC012`](#func012--method-call-on-standalone-function), which fires when dot-notation is applied to a symbol that is not a method at all, and from [`SEM028`](#sem028--undefined-field), which owns the missing-*field* case — SEM022 covers methods only.

**Headline template:** `` "type `<T>` has no method named `<method>`" ``
**Primary label:** `"no method with this name is defined on the receiver"`
**Help:** list the closest-named methods on the receiver's type; if the name matches a method on a related type reachable via a conversion, suggest the conversion.
**Suggestion:** `MaybeIncorrect` per close match on the same type; `MachineApplicable` never — the correct fix depends on user intent.

**Example (fails — SEM022):**
```clean
start:
	string s = "hello"
	integer n = s.lenght()    // type `string` has no method `lenght` — did you mean `length`?
```

---

### SEM023 — Non-Boolean Condition

The condition of an `if` or `while`, or a contract line in `before:`/`after:`, is not boolean.

**Condition:** the static type of the condition expression is not `boolean`. Applies to the condition of an `if` or `while`, and to each line of a `before:` or `after:` contract block — [CTR-01/CTR-02](../04%20language/10-contracts.md) make every contract line one boolean expression, so a contract line *is* a condition and this code owns its non-boolean case. (A non-boolean line inside `always:` is [`CLASS006`](#class006--always-expressions-must-be-boolean)'s.) There is no truthiness conversion: `0`, `""` and an empty list are not conditions.

**Message:** `"Condition must be a boolean expression, found {type}"`

**Primary label:** `expected boolean`

**Example (fails — SEM023):**
```clean
if items.length()
	print("not empty")
```

**Example (passes):**
```clean
if items.length() > 0
	print("not empty")
```

**Suggested fix:** compare explicitly, or call a predicate such as `isEmpty()`.

### SEM024 — Expected Value Not Constant

The expected value of a test is not evaluable at compile time.

**Condition:** the right-hand side of a test assertion is not a literal or a compile-time-evaluable expression ([11 — Testing](../04%20language/11-testing.md)). **Compile-time evaluable** reads conservatively: a literal, or a composition of literals under the built-in operators; any expression containing a name reference or a call is not.

**Message:** `"Expected value must be evaluable at compile time"`

**Primary label:** `not a compile-time value`

**Example (fails — SEM024):**
```clean
tests:
	"reads config": readSetting("port") == input("expected port: ")
```

**Example (passes):**
```clean
tests:
	"reads config": readSetting("port") == 8080
```

**Suggested fix:** write the expected value as a literal, so the test states what it asserts.

### SEM025 — Control Flow Outside Loop

**Condition:** `break` and `continue` are statements of the innermost enclosing loop ([FLW-03](../04%20language/12-control-flow.md#flw-03--break-and-continue)). Either one appearing where no `iterate` or `while` encloses it, within the same body, MUST be rejected with `SEM025`. A function body, a contract block, and a `compiletime function` body each begin a new body: a loop outside one of them does not enclose statements inside it.

**Message template:** `"'{keyword}' is not inside a loop"`
**Primary label:** `"no enclosing 'iterate' or 'while'"`

**Example (passes):**
```clean
iterate item in items
	if item.isEmpty()
		continue
	print(item)
```

**Example (fails — SEM025):** a `break` in a function called from a loop body, where the loop is in the caller —
```clean
functions:
	void handle(string item)
		break            // SEM025: 'break' is not inside a loop
```

**Suggested fix:** none machine-applicable — the statement must move inside a loop, or the early exit must be expressed as a `return`.

### SEM026 — Literal Out Of Range

**Condition:** A numeric literal MUST fit the type it is assigned to, and the check MUST be applied to the value **after** any unary minus is folded, not to the bare literal ([TYP-01](../04%20language/04-type-system.md#typ-01--the-core-types-and-their-ranges), [LEX-06](../04%20language/03-lexical-structure.md#lex-06--literal-forms)). A value outside the declared type's range is `SEM026`. Because a signed range is asymmetric, checking the bare literal would reject the smallest `integer`, whose magnitude exceeds the largest.

**Message template:** `"literal {value} does not fit {type} (range {min} to {max})"`
**Primary label:** `"out of range for {type}"`

**Example (passes):** `integer floor = -9223372036854775808` — the applied value is the smallest `integer`, and it fits, even though its magnitude alone does not.

**Example (fails — SEM026):** `integer n = 9223372036854775808` — `literal 9223372036854775808 does not fit integer (range -9223372036854775808 to 9223372036854775807)`.

**Suggested fix:** none machine-applicable — a value outside the range cannot be represented, and `number` is the only wider numeric type, with a loss of exactness above 2⁵³.

### SEM027 — Lossy Integer Promotion

An implicit `integer` → `number` conversion provably loses precision. This is the warning [TYP-06](../04%20language/04-type-system.md#typ-06--type-conversion) requires: `number` is binary64 with 53 bits of significand, so an `integer` whose magnitude exceeds 2⁵³ does not convert exactly.

**Condition:** at a site where TYP-06's implicit conversion applies, the converted value MUST be checked when it is compile-time evaluable (a literal, or a composition of literals — the same conservative reading as [SEM024](#sem024--expected-value-not-constant)); if its magnitude exceeds 2⁵³, the compiler emits `SEM027`. A value not evaluable at compile time never triggers the warning — the condition is decidable, not a may-analysis.

**Severity:** Warning — the program compiles; the converted value is the nearest representable `number`.

**Message template:** `"integer value {value} exceeds 2^53 and loses precision as a number"`

**Primary label:** `lossy conversion to number`

**Example (fails — SEM027):**
```clean
number weight = 9007199254740993    // one past 2^53: stored as 9007199254740992.0
```

**Example (passes):**
```clean
number weight = 9007199254740993.toNumber()    // explicit: the loss is stated intent (TYP-06)
```

**Suggested fix:** `MaybeIncorrect` — convert explicitly with `.toNumber()` to state the loss is intended, or keep the value in an `integer`.

### SEM028 — Undefined Field

A field access `receiver.field` references a field that does not exist on the receiver's type. This is the field-side counterpart of [`SEM022`](#sem022--undefined-method) (methods) and is distinct from [`CLASS012`](#class012--invalid-companion-access), which owns companion access through a class name.

**Condition:** the member named in a field-access expression is not a field of the receiver's static type (including inherited public fields, [CLS-02](../04%20language/14-classes-and-objects.md#cls-02--inheritance)).

**Message template:** `` "type `<T>` has no field named `<field>`" ``

**Primary label:** `no field with this name is defined on the receiver`

**Help:** list the closest-named fields on the receiver's type; if a *method* of that name exists, suggest calling it with parentheses.

**Example (fails — SEM028):**
```clean
Point p = Point(3, 4)
integer z = p.z          // type `Point` has no field named `z`
```

**Example (passes):**
```clean
Point p = Point(3, 4)
integer x = p.x
```

**Suggested fix:** `MaybeIncorrect` per close match; `MachineApplicable` never — the correct fix depends on user intent.

## 4. Scope Rules


### SCOPE001 — Variable Must Be Declared Before Use
Variables must be declared with an explicit type before being referenced in expressions or statements.

### SCOPE002 — Variable Cannot Be Redeclared in Same Scope
A variable name that already exists in the current scope cannot be redeclared.

### SCOPE003 — Maximum Scope Depth Exceeded
The scope nesting depth has exceeded the implementation limit. This indicates deeply nested code that should be refactored.

### SCOPE004 — Watch Target Must Reference State Variable
Watch block target identifiers must reference variables declared in a `state:` block.

### SCOPE005 — Screen State Access (WITHDRAWN)

**Status:** Withdrawn 2026-08-07 per [ADR-0030](../01%20governance/decisions/0030-withdraw-screen-from-language.md). The language-level `screen <Name>:` construct was withdrawn; screen-local state is no longer a language-defined scope. The code ID `SCOPE005` is retained as withdrawn per DOC-13 (IDs are never renumbered or reused). If a UI library registers a `screen:` block handler and defines its own scope discipline, it emits diagnostics through `LIB010` with a library-supplied sub-label, not through this code.

---

### SCOPE006 — Compiletime Helper Outside Test

A `test.compiletime.*` helper is used where it is not reachable.

**Condition:** a name in the `test.compiletime` namespace is resolved from a scope that is not the body of a `tests:` block. The namespace exists only there; it is the surface a library uses to exercise its own block handlers ([04 language / 21 §21.9](../04%20language/21-block-handlers.md)).

**Message:** `"'test.compiletime.{name}' is only available inside a 'tests:' block"`

**Primary label:** `not available here`

**Example (fails — SCOPE006):**
```clean
compiletime function expandDataBlock(block BlockAST) returns IR
	BlockAST parsed = test.compiletime.parseBlock(source)
	return ir.empty()
```

**Example (passes):**
```clean
tests:
	"expandDataBlock emits a class"
		BlockAST input = test.compiletime.parseBlock(sourceText)
		assert test.compiletime.classFieldNames(expandDataBlock(input), "UserData") == ["id"]
```

**Suggested fix:** move the call into a `tests:` block. A handler under test is invoked from the test, not the other way round.

## 5. Function Rules


### FUNC001 — Function Must Be Defined Before Use (withdrawn)

> **Withdrawn** (2026-08-17, M4 registry pass). The stub required functions to be "defined before being called" — a source-order restriction [9 — Functions](../04%20language/09-functions.md) never imposes, and one that would forbid forward references and mutual recursion, both legal. The only real failure is a call to a function defined nowhere in scope, and that case already has its owner: [`SEM019`](#sem019--undefined-function) `UndefinedFunction`. Folded into `SEM019` per the one-violation-one-diagnostic principle; the number `FUNC001` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)). Found by the compiler's Milestone 4 (`clean-language-compiler/docs/DISCOVERIES-M4.md`, item 18).

### FUNC002 — Argument Count Must Match Parameter Count
The number of arguments in a call must be admissible for the signature: at least the number of required parameters, at most the total. A parameter with a default value may be omitted ([FNC-04](../04%20language/09-functions.md#fnc-04--default-parameter-values) rule 4), so the admissible count is the range `required..=total`, collapsing to a single value when the signature has no defaults.

**Message template:** `"function '{name}' expects between {required} and {total} arguments, found {count}"` — or `"function '{name}' expects {n} arguments, found {count}"` when the signature has no defaults.

**Example (fails — FUNC002):**
```clean
functions:
	integer add(integer a, integer b)
		return a + b

start:
	integer result = add(1, 2, 3)    // expects 2 args, got 3
```

### FUNC003 — Cannot Call Non-Function Type
Only function-typed symbols can be invoked with parentheses.

### FUNC004 — Missing Return in Non-Void Function
A function with a non-void return type must have a return statement on all execution paths. (Warning)

### FUNC005 — Empty Return in Non-Void Function
A return statement in a function with a non-void return type must provide a value. (Warning)

### FUNC006 — Start Block Cannot Have Parameters
The `start:` entry point block does not accept parameters.

### FUNC007 — Start Block Should Return Void
The `start:` entry point should not return a value. (Warning)

### FUNC008 — Named Argument Label Must Match Parameter
The identifier in a named argument must exactly match a parameter name of the called function or constructor.

**Example (fails — FUNC008):**
```clean
functions:
	integer add(integer a, integer b)
		return a + b

start:
	integer result = add(x: 1, b: 2)    // "x" does not match any parameter name
```

### FUNC009 — No Duplicate Named Arguments
Each parameter name may appear at most once in a call's argument list.

**Example (fails — FUNC009):**
```clean
integer result = add(a: 1, a: 2)    // "a" appears twice
```

### FUNC010 — Positional Arguments Must Precede Named Arguments
In a mixed call, all positional arguments must come before any named arguments.

**Example (fails — FUNC010):**
```clean
integer result = add(a: 1, 2)    // named before positional
```

**Valid mixed call:**
```clean
integer result = add(1, b: 2)    // positional first, then named
```

### FUNC011 — All Parameters Must Be Covered
Every **required** parameter must be satisfied by exactly one positional or named argument; a parameter with a default value by at most one ([FNC-04](../04%20language/09-functions.md#fnc-04--default-parameter-values) rule 4 — an uncovered defaulted parameter takes its default). No parameter may be provided both positionally and by name.

**Example (fails — FUNC011):**
```clean
integer result = add(1, a: 1, b: 2)    // "a" covered twice (positionally and by name)
```

### FUNC012 — Method Call On Standalone Function
Dot-notation method call was applied to a symbol that is not a method.

---

### FUNC013 — Function Outside Functions Block

A function is declared outside a `functions:` block.

**Condition:** a function declaration's enclosing scope is a file or a class body rather than a `functions:` block.

**Message:** `"Function '{name}' must be declared inside a 'functions:' block"`

**Primary label:** `not inside functions:`

**Example (fails — FUNC013):**
```clean
integer add(integer a, integer b)
	return a + b
```

**Example (passes):**
```clean
functions:
	integer add(integer a, integer b)
		return a + b
```

**Suggested fix:** wrap the declaration in a `functions:` block ([09 — Functions](../04%20language/09-functions.md)).

### FUNC014 — Optional Parameter Order

A parameter with a default value precedes a parameter without one.

**Condition:** in a parameter list, a parameter carrying a default is followed by one that does not.

**Message:** `"Parameter '{name}' has no default and follows '{previous}', which has one"`

**Primary label:** `required parameter after an optional one`

**Example (fails — FUNC014):**
```clean
functions:
	void greet(string name = "there", string title)
		print(title + " " + name)
```

**Example (passes):**
```clean
functions:
	void greet(string title, string name = "there")
		print(title + " " + name)
```

**Suggested fix:** move every parameter with a default to the end of the list.

### FUNC015 — Duplicate Start Block

A file declares more than one `start:` block.

**Condition:** a file MUST contain at most one `start:` block ([FNC-01](../04%20language/09-functions.md#fnc-01--start-is-the-entry-point)). The grammar carries no cardinality on `Item::Start`, so the HIR validator enforces the rule and reports every block after the first.

**Message template:** `"file declares more than one 'start:' block"`

**Primary label:** `second 'start:' block` — placed on each block after the first, with a secondary label on the first.

**Example (fails — FUNC015):**
```clean
start:
	print("first")

start:
	print("second")    // file declares more than one 'start:' block
```

**Example (passes):**
```clean
start:
	print("first")
	print("second")
```

**Suggested fix:** merge the bodies into the single `start:` block, or move non-entry code into `functions:`.

## 6. Class Rules


### CLASS001 — Parent Class Must Exist
When a class uses `is ParentName` for inheritance, the parent class must be defined.

### CLASS002 — Duplicate Field in Class
Field names within a class must be unique.

### CLASS003 — Duplicate Method in Class
Method names within a class must be unique.

### CLASS004 — Constructor Must Exist for Instantiation
A class that is instantiated must have a constructor (explicit or implicit).

### CLASS005 — `after` Must Appear After `before` and Before Other Statements

`after` statements must appear at the top of a function body, after any `before` statements, and before any other logic. See [`10-contracts.md`](../04%20language/10-contracts.md) for the full contract syntax.

**Condition:** an `after:` block precedes a `before:` block, or appears after the first non-contract statement of the function body. This is the only contract-position case the checker owns: the other misplacements (a contract block after statements, `always:` outside a class body) cannot be built by the grammar and are rejected by the parser as [`SYN005`](#syn005--malformed-construct) — which is why [`CLASS007`](#class007--contract-block-out-of-position-withdrawn) was withdrawn into this rule.

**Message template:** `"'after:' must follow 'before:' at the top of the function body"`

**Primary label:** `'after:' out of position`

**Example (fails — CLASS005):**
```clean
functions:
	integer divide(integer a, integer b)
		before:
			b != 0
		integer result = a / b
		after:
			result * b == a      // error: 'after' must appear before non-contract logic
		return result
```

### CLASS006 — `always:` Expressions Must Be Boolean

Every expression inside an `always:` invariant block must evaluate to `boolean`. See [`10-contracts.md`](../04%20language/10-contracts.md). A non-boolean line in a `before:`/`after:` block is [`SEM023`](#sem023--non-boolean-condition)'s — this code owns the `always:` case only.

**Message template:** `"expression inside 'always:' must be a boolean expression, found {type}"`

**Primary label:** `expected boolean`

**Note:** In V2, `before` preconditions are always checked at runtime and cannot be disabled — they guard the caller's contract. `after` postconditions and `always` invariants may be stripped by the compile-time flag `--strip-checks` (see [`../10-contracts.md`](../04%20language/10-contracts.md) §Runtime Cost); when stripped, their expressions are not emitted at all. `before` is unaffected by the flag.

---

### CLASS007 — Contract Block Out Of Position (withdrawn)

> **Withdrawn** (2026-08-17, M4 registry pass). Its condition list decomposed into cases that all have other owners, leaving no reachable trigger: a contract block after body statements and an `always:` outside a class body are constructions the grammar cannot build — the parser rejects them as [`SYN005`](#syn005--malformed-construct) (M3, snapshot-pinned) — and the remaining case, `after:` out of position relative to `before:`, was already the strict subset owned by [`CLASS005`](#class005--after-must-appear-after-before-and-before-other-statements). One violation, one owner ([ERC-02](./09-error-codes.md#erc-02--one-code-one-rule)); the number `CLASS007` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)). Found by the compiler's Milestone 4 (`clean-language-compiler/docs/DISCOVERIES-M4.md`, item 11).

### CLASS008 — Result Outside After

The `result` identifier is used outside an `after:` expression.

**Condition:** `result` is resolved in a scope that is not the body of an `after:` block.

**Message:** `"'result' is only in scope inside an 'after:' expression"`

**Primary label:** `'result' is not in scope here`

**Example (fails — CLASS008):**
```clean
functions:
	integer double(integer n)
		return result * 2
```

**Example (passes):**
```clean
functions:
	integer double(integer n)
		after:
			result == n * 2
		return n * 2
```

**Suggested fix:** use the value directly; `result` names the return value only while checking it.

### CLASS009 — Contract Side Effect

A contract expression is not pure.

**Condition:** an expression inside `before:`, `after:` or `always:` performs I/O, assigns to state or a field, or calls a function that itself carries contracts ([10 — Contracts §6.3](../04%20language/10-contracts.md)).

**Message:** `"Contract expression must be pure: '{operation}' is not allowed here"`

**Primary label:** `contract expressions cannot have effects`

**Example (fails — CLASS009):**
```clean
functions:
	integer readSize(string path)
		before:
			file.exists(path)
		return 0
```

**Example (passes):**
```clean
functions:
	integer readSize(string path)
		before:
			path.length() > 0
		return 0
```

**Suggested fix:** state the assumption over values already in scope, or move the check into the body with an explicit `if` and `error`.

### CLASS010 — Constructor Parameter Shadows Field

A constructor parameter has the same name as a field of its class.

**Condition:** a constructor parameter list contains a name that also names a field of the enclosing class.

**Message:** `"Constructor parameter '{name}' has the same name as a field"`

**Primary label:** `rename this parameter`

**Example (fails — CLASS010):**
```clean
class Point
	integer x
	integer y

	constructor(integer x, integer y)
		// which x?
```

**Example (passes):**
```clean
class Point
	integer x
	integer y

	constructor(integer initialX, integer initialY)
		x = initialX
		y = initialY
```

**Suggested fix:** rename the parameter, so an assignment in the constructor body has exactly one reading ([14 — Classes and Objects](../04%20language/14-classes-and-objects.md)).

### CLASS011 — Capability Instantiated

A capability is used as if it were a class.

**Condition:** a capability name appears in constructor position.

**Message:** `"'{name}' is a capability and cannot be instantiated"`

**Primary label:** `capabilities have no bodies`

**Example (fails — CLASS011):**
```clean
Drawable d = Drawable()
```

**Example (passes):**
```clean
Drawable d = Circle(5)
```

**Suggested fix:** instantiate a class that claims the capability with `can`.

### CLASS012 — Invalid Companion Access

A companion access does not resolve.

**Condition:** any of — the receiver of `Name.field` is an instance rather than a class name; the named field is not declared on that class; the member reached is an instance method rather than a companion member ([14 §Companion Access](../04%20language/14-classes-and-objects.md#cls-05--companion-access)).

**Message:** `"'{receiver}.{member}' is not a valid companion access: {reason}"`

**Primary label:** `invalid companion access`

**Example (fails — CLASS012):**
```clean
User user = User("ada")
user.data.findBy("name", "ada")
```

**Example (passes):**
```clean
User.data.findBy("name", "ada")
```

**Suggested fix:** reach the companion through the class name, not through an instance.

## 7. Index Access Rules


### IDX001 — List Index Must Be Integer
`list<T>` bracket access requires an `integer` index. Lists are zero-indexed ([04 language / 04 — Type System](../04%20language/04-type-system.md)).

**Message template:** `` "list index must be `integer`, found `<T>`" ``
**Primary label:** `expected an integer index`
**Example (fails — IDX001):** `names["first"]` on a `list<string>`. **Example (passes):** `names[0]`.
**Suggested fix:** index with an `integer`; for keyed lookup, use `pairs<K, V>`.

**Withdrawn name.** `IDX001` was originally named `ArrayIndexNotInteger`; that symbolic name is retired (2026-08-01) — `Array` is not a Clean term ([Glossary](../01%20governance/06-glossary.md)). The code is unchanged.

### IDX002 — Matrix Index Must Be Integer
Matrix bracket access requires an integer index.

**Message template:** `` "matrix index must be `integer`, found `<T>`" ``
**Primary label:** `expected an integer index`
**Example (fails — IDX002):** `grid[1.5]`. **Example (passes):** `grid[1]`.
**Suggested fix:** index with an `integer` (use `.toInteger()` only when truncation is the intent).

### IDX003 — Pairs Key Type Mismatch
`pairs<K, V>` bracket access requires a key of the declared key type `K`. `K` is a free type parameter, not fixed to `string` ([04 language / 04 — Type System](../04%20language/04-type-system.md)) — `pairs<integer, string>` is indexed with an `integer`.

**Message template:** `` "`pairs<K, V>` is indexed with `<K>`, found `<T>`" `` — with the declared types substituted, e.g. `` `pairs<integer, string>` is indexed with `integer`, found `string` ``.
**Primary label:** `wrong key type for this pairs`
**Example (fails — IDX003):** `ages["41"]` on a `pairs<integer, string>`. **Example (passes):** `ages[41]`.
**Suggested fix:** pass a key of the declared type `K`.

**Withdrawn name.** `IDX003` was originally named `PairsKeyNotString` and required a string key; that requirement contradicted the generic key of the type system and is retired (2026-08-01). The code is unchanged.

### IDX004 — Index On Non-Indexable Type
Bracket access `expr[key]` was applied to a value whose type does not support indexing at all. Only `list<T>`, `matrix<T>`, `pairs`, and `any` support bracket access.

**Headline template:** `` "type `<T>` does not support bracket access" ``
**Primary label:** `` "cannot index a value of type `<T>`" ``
**Help:** if the type has a named accessor method (e.g. a `string` has `.charAt(n)`), suggest it as an alternative.
**Suggestion:** `MaybeIncorrect` — replace `expr[key]` with the named accessor when one exists.

**Example (fails — IDX004):**
```clean
start:
	string s = "hello"
	string c = s[0]    // `string` does not support bracket access; use `s.charAt(0)`
```

### IDX005 — Index On None
Bracket access was applied to a value whose static type is nullable (`T?`) and the value is `none` at the access site. This is a compile-time diagnostic when the compiler can prove the receiver is `none` on all paths reaching the access; it is [`RUN004`](#run004--reference-error) at runtime when the proof fails.

**Headline template:** `` "cannot index `<name>` because it may be `none`" ``
**Primary label:** `` "receiver has type `<T>?` and may be `none` here" ``
**Suggestion:** `MachineApplicable` — insert an `if <name> is not none` guard around the access, OR replace `<name>[<key>]` with `<name>?.at(<key>)` when the safe-access surface exists on the type.

**Example (fails — IDX005):**
```clean
functions:
	void first(list<integer>? items)
		integer n = items[0]    // items may be `none`; guard first
```

---

## 8. State Rules


### STATE001 — Guard Condition Must Be Pure Boolean

A guard expression on a state variable is not a pure boolean expression, or contains side effects.

**Syntax home:** [20 — State Management](../04%20language/20-state-management.md) (the `guard <expr> else` clause).

**Condition:** The expression after `guard` must be a pure boolean expression. It may reference `value` (the proposed new value) and any currently-in-scope identifiers, but must not contain side-effecting operations such as function calls that perform I/O or mutate state.

**Example (fails — STATE001):**
```clean
state:
	integer count = 0
		guard print("checking") else "no side effects allowed"
```

**Message:** `"Guard condition must be a pure boolean expression"`

**Runtime behavior:** If the guard condition evaluates to `false` at runtime, the state update is rejected and the state variable retains its previous value. The error message from the `else` clause is reported (STATE002).

### STATE002 — Guard Rejection (Runtime)

A state update is rejected at runtime because a guard condition evaluated to false.

**Syntax home:** [20 — State Management](../04%20language/20-state-management.md) (the `guard <expr> else` clause).

**Condition:** This is a runtime rule, not a compile-time error. It is raised when an assignment to a guarded state variable evaluates the guard expression and receives `false`.

**Example (triggers — STATE002 at runtime):**
```clean
state:
	integer count = 0
		guard value >= 0 else "Count cannot be negative"

functions:
	void decrement()
		count = count - 1    // runtime rejection if count is already 0
```

**Message:** `"State update rejected: {guard_message}"`

### STATE003 — Circular State Dependency

Computed state declarations depend on each other in a cycle (directly or transitively). (Registry name: `CircularStateDependency` — the former name `ComputedReturnTypeMismatch` is withdrawn; see [09 §3.7](./09-error-codes.md#37-state-codes-state).)

**Syntax home:** [20 — State Management](../04%20language/20-state-management.md) (the `computed:` sub-block).

**Boundary (reciprocal with SEM018):** STATE003 covers *only* the circular-dependency case. A computed body whose value does not match the declared type is [`SEM018`](#sem018--computed-body-type-mismatch), never STATE003.

**Example (fails — STATE003, circular dependency):**
```clean
state:
	computed:
		string a
			return b    // a depends on b
		string b
			return a    // b depends on a — circular
```

**Message:** `"Circular dependency in computed state: '{name}' depends on itself"`

### STATE004 — Computed State Assignment

Code attempts to assign a value directly to a computed state variable.

**Syntax home:** [20 — State Management](../04%20language/20-state-management.md) (the `computed:` sub-block).

**Condition:** Computed state is read-only. Any assignment statement whose left-hand side is a computed state variable is rejected at compile time.

**Example (fails — STATE004):**
```clean
state:
	string firstName = ""
	string lastName = ""
	computed:
		string fullName
			return firstName + " " + lastName

functions:
	void badAssign()
		fullName = "Alice Smith"    // error: fullName is computed, cannot assign
```

**Message:** `"Cannot assign to computed state variable '{name}': it is read-only"`

### STATE005 — Rules Expression Must Be Boolean

An expression inside a `state: rules:` block does not evaluate to a boolean.

**Syntax home:** [20 — State Management](../04%20language/20-state-management.md) (the `state: rules:` block).

**Condition:** Every expression listed under `rules:` must be a boolean expression. Non-boolean expressions (e.g., integer arithmetic with no comparison) are a compile-time error.

**Example (fails — STATE005):**
```clean
state:
	integer count = 0

	rules:
		count + 1    // error: not a boolean expression
```

**Message:** `"State rule expression must be a boolean expression, got {type}"`

---

### STATE006 — State Rule Violated

**Condition:** Every expression in a `rules:` block MUST be evaluated when a function that assigned to any state variable of the enclosing `state:` block returns ([SMG-03](../04%20language/20-state-management.md#smg-03--state-rules)). A rule evaluating to `false` MUST raise `STATE006`, naming the rule's source text. The state MUST NOT be rolled back — the function has already run. Rules are not evaluated after each individual assignment: a rule spanning several variables would otherwise forbid the intermediate states a multi-step update passes through.

**Message template:** `"state rule violated: {rule_text}"`
**Primary label:** `"this rule does not hold when {function} returns"`

**Example (passes):** with `rules: end > start`, a function that sets `start` then `end` leaves a state satisfying the rule at the moment it returns, even though the intermediate assignment did not.

**Example (fails — STATE006):** with `rules: count >= 0`, a function that assigns `count = -1` and returns — `state rule violated: count >= 0`.

**Suggested fix:** none machine-applicable — a violated rule means the program reached a combination it declared impossible; the defect is in the code that produced it, not at the point of report.

## 9. Import Rules


### IMPORT001 — Circular Dependency

Two or more modules import each other in a cycle. Cycles are detected by the compiler's resolve pass while it builds the module graph from the compilation request document ([14 §14.4.2 pass 4](./14-compiler-architecture.md#1442-detailed-pass-responsibilities)).

**Condition:** While following `import` statements across the `sources[]` set of the compilation request, the resolver MUST detect any cycle in the module graph (direct or transitive) and MUST report it as `IMPORT001`, naming the full cycle path. Per the one-violation-one-diagnostic principle (see the SEM003 boundary note), a single cycle produces exactly one diagnostic. Resolution continues for the rest of the graph so multiple errors surface in one pass.

**Message template:** `"import cycle detected: {A → B → ... → A}"`
**Primary label:** `"this import closes the cycle"` — placed at the `import:` entry that completes the cycle.

**Example (fails — IMPORT001):**
```clean
// file: a.cln
import:
	b

// file: b.cln
import:
	a        // error: import cycle detected: a → b → a
```

**Suggested fix:** none machine-applicable — breaking a cycle requires moving shared declarations into a third module both can import.

### IMPORT002 — Module Not Found
The imported module does not exist in any search path.

### IMPORT003 — Symbol Not Found in Module
The specific symbol imported from a module is not exported by that module.

### IMPORT004 — Duplicate Import Item
The same item appears more than once in an import list.

### IMPORT005 — Import Cycle (withdrawn)

> **Withdrawn** (2026-08-01, technical-debt closure pass). `IMPORT005` described the same failure class as [`IMPORT001`](#import001--circular-dependency) — a cycle in the module import graph — and the one-violation-one-diagnostic principle allows only one owner. The case is **folded into `IMPORT001`**, whose entry above now carries the full contract (including the resolve-pass detection of [14 §14.4.2 pass 4](./14-compiler-architecture.md#1442-detailed-pass-responsibilities)). The number `IMPORT005` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose): IDs are never renumbered or reused).

---

## 10. Library Rules (LIB)


Libraries in V2 are Clean source packages that extend the compiler through three mechanisms defined in [Libraries Specification](../02%20components/framework/09-libraries-specification.md): generic `identifier:` blocks, `compiletime function` handlers that turn those blocks into typed IR, and typed `host function` declarations that bind to host-provided implementations. A library may bind its `host function` declarations to either (a) compiler-generated bridges that already exist in the runtime, or (b) native host modules provided as WASM components. The rules below cover both loading models.

### 10.1 Resolution

#### LIB001 — Library Not Found
A library named in `clean.toml [dependencies]` or in a `[folders]` mapping cannot be resolved from any registry, path, or lockfile source.

**Condition:** the resolver has exhausted every configured source (public registry, local path override, lockfile pin) without finding the named library.

**Message:** `"Library '{name}' not found in any configured source"`

#### LIB002 — Library Version Conflict
Two dependencies resolve the same library to incompatible SemVer ranges and no single version satisfies both.

**Example:** app depends on `data ^2.0`, `auth` transitively depends on `data ^1.5`. No 1.x version satisfies `^2.0`, and no 2.x version satisfies `^1.5`.

**Fix:** upgrade the transitive dependency, or pin `data` in the root `clean.toml [dependencies]` to a version both accept.

**Message:** `"Library '{name}' has conflicting version requirements: {list_of_ranges}"`

#### LIB003 — Library Cyclic Dependency
libraries A and B depend on each other, directly or transitively.

**Condition:** the resolver detected a cycle in the resolved dependency graph after applying version selection.

**Message:** `"Cyclic library dependency detected: {A → B → ... → A}"`

#### LIB004 — Library Manifest Invalid
`library.toml` is malformed, missing required fields, or declares a manifest schema version the compiler does not support.

**Condition triggers this rule:**
- TOML parse failure.
- Missing any of: `name`, `version`, `schema_version`.
- `schema_version` is greater than the highest schema the compiler recognizes.

**Message:** `"Library '{path}' has invalid manifest: {reason}"`

**At the compiler boundary**, the manifest under validation is the lowered `library_manifests[]` entry of the compilation request ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)) — the compiler never reads `library.toml` and holds no filesystem path for it ([CMP-01](./14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)). There, `{path}` is filled with the entry's `name`, and the TOML-level conditions above apply to the toolchain component that reads `library.toml` and lowers it (Clean Framework). The template is unchanged either way.

### 10.2 Blocks and Handlers

#### LIB005 — Block Handler Conflict *(withdrawn)*

**Withdrawn 2026-08-01.** This rule duplicated [`BLOCK001`](./09-error-codes.md#315-block-handler-codes-block): both described block-name conflict between libraries in scope, which is one failure with two codes and a breach of [ERC-02](./09-error-codes.md#erc-02--one-code-one-rule). `BLOCK001` is retained — it is the code the framework documents cite, and its rule body lives with the construct it describes in [04 language / 21 — Block Handlers §21.6](../04%20language/21-block-handlers.md#216-diagnostics-from-compile-time-functions). The identifier `LIB005` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)).

#### LIB006 — Unknown Block Handler
A `handles "name"` declaration references a block name whose namespace is not owned by this library and is not delegated to it.

**Condition:** the library's manifest declares no `namespace` matching the block, and no explicit delegation from another library grants it.

**Message:** `"Library '{lib}' declares a handler for '{block}' but does not own that namespace"`

### 10.3 Compile-time Execution

#### LIB007 — Compiletime Side Effect *(withdrawn)*

**Withdrawn 2026-08-01.** This rule duplicated [`BLOCK006`](./09-error-codes.md#315-block-handler-codes-block): both described forbidden side effect in a compile-time context, which is one failure with two codes and a breach of [ERC-02](./09-error-codes.md#erc-02--one-code-one-rule). `BLOCK006` is retained — it is the code the framework documents cite, and its rule body lives with the construct it describes in [04 language / 21 — Block Handlers §21.6](../04%20language/21-block-handlers.md#216-diagnostics-from-compile-time-functions). The identifier `LIB007` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)).

The withdrawn body carried one clause with no home elsewhere: an exception permitting filesystem access "through the compiler-provided `readSpecFile` primitive". `readSpecFile` is defined in no chapter of the specification, so it is not carried over to [`BLOCK006`](./09-error-codes.md#315-block-handler-codes-block); compile-time code has no filesystem access at all. Reinstating such a primitive would require specifying it first ([SDD-09](../01%20governance/03-spec-driven-design.md)).

#### LIB008 — Compiletime Type Error *(withdrawn)*

**Withdrawn 2026-08-01.** This rule duplicated [`BLOCK004`](./09-error-codes.md#315-block-handler-codes-block): both described a handler returning malformed or ill-typed IR, which is one failure with two codes and a breach of [ERC-02](./09-error-codes.md#erc-02--one-code-one-rule). `BLOCK004` is retained — it is the code the framework documents cite, and its rule body lives with the construct it describes in [04 language / 21 — Block Handlers §21.6](../04%20language/21-block-handlers.md#216-diagnostics-from-compile-time-functions). The identifier `LIB008` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)).

#### LIB009 — Compiletime Budget Exceeded *(withdrawn)*

**Withdrawn 2026-08-01.** This rule duplicated [`BLOCK005`](./09-error-codes.md#315-block-handler-codes-block): both described a handler exceeding its wall-clock or memory budget, which is one failure with two codes and a breach of [ERC-02](./09-error-codes.md#erc-02--one-code-one-rule). `BLOCK005` is retained — it is the code the framework documents cite, and its rule body lives with the construct it describes in [04 language / 21 — Block Handlers §21.6](../04%20language/21-block-handlers.md#216-diagnostics-from-compile-time-functions). The identifier `LIB009` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)).

#### LIB010 — Compiletime Diagnostic
A `compiletime function` emitted a diagnostic via `error(...)`, `warning(...)`, or `info(...)`. This is not a compiler-internal error — the code is used to wrap library-authored diagnostics so tooling can attribute them to the emitting library.

**Format of a LIB010 diagnostic:**
```
{file}:{line}:{col}: {severity} [LIB010 via {library}::{function}] {message}
```

Library-authored diagnostics always carry a source span from the *user's* code (the block that triggered the compile-time function), not from the library.

### 10.4 Host Function Bindings

#### LIB011 — Host Function Signature Mismatch
A `host function` declaration in library source does not match the signature the resolved host module actually exposes.

**Applies to both loading models:**
- **Compiler-generated bridge**: the compiler has an intrinsic bridge function for the named host interface (inside the built-in `db` module) with a different signature.
- **Native WASM component**: the WIT interface exported by the component declares a different parameter or return type.

**Condition:** signature comparison happens at link time, before any user code is compiled. Types are checked structurally, not by name.

**Message:** `"Host function '{name}' from module '{host}' expected signature {expected}, library declares {actual}"`

#### LIB012 — Host Function Unbound
A `host function` declaration sits in a `host interface` whose `requires host worlds` list does not include the current build target's world, or names an interface no active host provides. Declaration grammar: [Libraries Specification §8.3 (LBS-02)](../02%20components/framework/09-libraries-specification.md).

**Example (fails — LIB012):** a library's `host_bridge.cln` declares:

```clean
host interface fs version "0.1.0":
	requires host worlds ["server", "cli"]

	host function readFile(path: string) returns bytes
		description "Read a file from the host filesystem."
```

The project targets `wasm32-browser`; the `browser` world is not in the interface's `requires host worlds` list, so the declaration cannot bind.

**Fix:** add the target's world to the interface's `requires host worlds` list (only if the host genuinely provides it), or include the library only for targets that support it.

**Message:** `"Host interface '{host}' required by library '{lib}' is not available for target '{target}'"`

#### LIB013 — Host Function Sandbox Denied
The library declared a `host function` in a `host interface` that is not permitted by the project's `clean.toml [security]` capabilities (schema home: [`07-build-config.md`](./07-build-config.md)).

**Example:**
```toml
[security]
allowedHostModules = ["db", "http.client"]
```

A library's `host_bridge.cln` declares:

```clean
host interface fs version "0.1.0":
	requires host worlds ["server"]

	host function writeFile(path: string, data: bytes) returns integer
		description "Write bytes to a file on the host filesystem. Returns 0 on success."
```

Even though the runtime provides `fs`, the project has not granted that capability.

**Message:** `"Library '{lib}' requires host interface '{host}' which is not in [security].allowedHostModules"`

#### LIB019 — Host Bridge Misplaced

A `host interface` / `host function` declaration appears in a library source file other than `host_bridge.cln`.

**Condition:** Every host declaration in a library MUST live in `host_bridge.cln` at the library root — the single, mandatory location defined by [LBS §8.2](../02%20components/framework/09-libraries-specification.md#82-file-layout). A `host interface` or `host function` declaration found in any other file of the library MUST be rejected with `LIB019` at the declaration site.

**Message template:** `"Host declaration '{name}' must live in host_bridge.cln (found in '{file}')"`
**Primary label:** `"host declarations are only allowed in host_bridge.cln"` — placed at the `host interface` / `host function` keyword.

**Example (passes):** the declaration sits in `mylib/host_bridge.cln`:
```clean
// mylib/host_bridge.cln
host interface fs version "0.1.0":
	requires host worlds ["server"]

	host function readFile(path: string) returns bytes
		description "Read a file from the host filesystem."
```

**Example (fails — LIB019):** the same declaration sits in `mylib/src/util.cln`:
```clean
// mylib/src/util.cln
host interface fs version "0.1.0":    // error: host declarations belong in host_bridge.cln
	...
```

**Suggested fix:** `MachineApplicable` — move the declaration block to `host_bridge.cln` (creating the file at the library root if absent).

#### LIB020 — Source Block Malformed

A `source:` block is incomplete or misplaced.

**Condition:** a `source:` block is missing its `spec` field, is missing its `version` field, or does not appear at the position [08 — File Structure](../04%20language/08-file-structure.md) assigns it.

**Message:** `"'source:' block is malformed: {reason}"`

**Primary label:** `incomplete source: block`

**Example (fails — LIB020):**
```clean
source:
	spec: "specs/auth.spec.cln"
```

**Example (passes):**
```clean
source:
	spec: "specs/auth.spec.cln"
	version: "2.1.0"
```

**Suggested fix:** supply both fields ([19 — AI Integration](../04%20language/19-ai-integration.md)).

### 10.5 Resource Limits

#### LIB014 — Library Resource Limit
Loading or compiling the library exceeded a resource limit.

**Limits (defaults; configurable in `clean.toml [compile.limits]` — schema home: [`07-build-config.md`](./07-build-config.md)):**
- Source-tree file count: 1000 files per library.
- Compile-time heap: 512 MiB total across all `compiletime function` invocations for one library.
- Generated-IR node count: 500 000 nodes emitted by a single `compiletime function` invocation.

**Message:** `"Library '{lib}' exceeded {limit_name}: {actual} > {max}"`

### 10.6 Capabilities

Capabilities are declared in library source and claimed by user classes with `can CapabilityName`. See [Classes and Objects](../04%20language/14-classes-and-objects.md) for the `can` syntax and [Libraries Specification §4.3](../02%20components/framework/09-libraries-specification.md) for the companion-type pattern.

#### LIB015 — Capability Not Implemented
A companion type declares `can Persist` (or any other library-owned capability) but does not implement one of the capability's required methods. All methods declared in a capability are required — capabilities are pure contracts with no default bodies (see [14 — Classes and Objects §Capabilities](../04%20language/14-classes-and-objects.md) and [SEM014](#sem014--capability-body-not-allowed)).

**Example (fails — LIB015):**
```clean
can Persist:
	save() returns integer
	load(integer id) returns any

class User can Persist    // ERROR: missing method save() and load()
	public:
		string email
```

**Message:** `"Type '{type}' claims capability '{cap}' but does not implement required method '{method}'"`

Note: general capability rules (SEM011–SEM014) apply to *language-native* capabilities. LIB015 is specifically for library-owned capabilities where the capability's namespace is owned by a library manifest — the diagnostic is attributed to the library so IDEs can offer capability-specific quick fixes.

#### LIB016 — Capability Conflict
Two libraries in the resolved graph both define a capability with the same fully-qualified name and mutually incompatible method signatures.

**Message:** `"Capability '{name}' is defined by both '{libA}' and '{libB}' with incompatible signatures"`

### 10.7 Folder Scope

#### LIB017 — Folder Scope Unclaimed
A folder listed in `clean.toml [folders]` maps to a library that is not present in `[dependencies]`.

**Example (fails — LIB017):**
```toml
[folders]
"app/data" = "data"        # OK: data is a dependency
"app/ui"   = "clean.ui.v2"       # ERROR: clean.ui.v2 not in [dependencies]

[dependencies]
"data" = "^2.0"
```

**Message:** `"Folder scope '{folder}' maps to library '{lib}' which is not a declared dependency"`

#### LIB018 — Folder Scope Ambiguous

**Boundary with [`BLOCK001`](./09-error-codes.md#315-block-handler-codes-block).** `BLOCK001` fires when two libraries in scope register the same *block name*. `LIB018` is the wider case: two libraries claim the same *namespace* for a folder, so the ambiguity exists whether or not a specific name has yet collided. A single colliding name is `BLOCK001`; an ambiguous namespace mapping is `LIB018`.
Two libraries in scope for the same folder both claim ownership of the same block namespace inside that folder.

**Condition:** the folder's `clean.toml` scope permits both `libA` and `libB` (a folder may list multiple libraries), and both libraries' manifests declare ownership of the same block namespace.

**Message:** `"Namespace '{ns}' in folder '{folder}' is claimed by both '{libA}' and '{libB}'"`

---

## 11. Compilation Rules (COM)


COM001–COM008 cover WASM code generation. COM009–COM017 cover the WIT bridge pipeline: version resolution and link-time verification ([08 — Bridge Versioning](./08-bridge-versioning.md)), the world import check ([14 §14.4.2 pass 9](./14-compiler-architecture.md#1442-detailed-pass-responsibilities)), and the three host-contract check moments of [16 — Host Contract Validation](./16-host-contract-validation.md#164-the-three-check-moments). Not every COM rule is enforced by the compiler: COM014/COM015 are emitted by Clean Framework, COM011/COM017 by the host at instantiation.

### COM001 — WASM Generation Error
The code generator failed to produce valid WASM for a construct.

### COM002 — Optimization Error
An optimization pass produced invalid code.

### COM003 — Memory Layout Error
A memory-layout calculation failed while emitting the component. The registered condition: the program's static data — string constants and every other compiler-emitted datum — does not fit the fixed data region `[DATA_SECTION_START, HEAP_START)` of [MMD-01](./03-memory-model.md#mmd-01--layout-and-guest-visible-constants). `HEAP_START` is 1 MiB and never moves ([03 §3.1](./03-memory-model.md#31-linear-memory-layout)), so a program with ≈1 MiB or more of emitted static data has no conforming layout and MUST be rejected here. This is a user-program condition, never a `COM013` internal invariant.

**Headline template:** `` "static data (<emitted> bytes) exceeds the data region (<available> bytes below HEAP_START)" ``
**Primary label:** `` "largest contributor: this literal (<bytes> bytes)" `` — attached to the largest single compiler-attributable datum; when no source span can be attributed, the diagnostic is program-level with no primary span.
**Help:** move large literal payloads out of the source — load them at runtime (`file.read`, a host function) or split the program. `HEAP_START` is fixed by MMD-01 and does not grow with data.
**Suggestion:** none (the fix is restructuring data, not an edit the compiler can propose).

**Example (fails — COM003):**
```clean
constant string BLOB = "…"    // a generated literal of 1.5 MiB
start:
	print(BLOB.length().toString())
```

### COM004 — Module Resolution Error
Multi-file compilation failed to resolve module dependencies.

### COM005 — Target Feature Unsupported
The compilation target does not implement a language feature the source uses. This is a compile-time diagnostic emitted by the target backend during lowering, not a link-time or runtime failure.

**Headline template:** `` "target `<target>` does not support `<feature>`" ``
**Primary label:** `` "this construct requires `<feature>`" ``
**Help:** name the targets that DO support the feature and, when applicable, a source-level rewrite that avoids the feature.
**Suggestion:** `MaybeIncorrect` when a source-level rewrite exists; otherwise no suggestion (the fix is choosing a different target in `clean.toml [build]`).

**Example (fails — COM005):**
```clean
// clean.toml sets target = "wasm32-browser"
start:
	file.write("/tmp/x", "hi")    // target `wasm32-browser` does not support file I/O
```

### COM006 — Function Not Found During Compilation
A function that passed semantic analysis could not be located during code generation.

### COM007 — Target Host Module Missing
The compilation target does not provide a host interface that a library declares in its `host_bridge.cln` (`host interface` block — [LBS-02](../02%20components/framework/09-libraries-specification.md)). Distinct from [`LIB012 HostFunctionUnbound`](#lib012--host-function-unbound), which fires at library-link time; COM007 fires when the compiler is finalizing target-specific bindings for the user's program.

**Headline template:** `` "target `<target>` does not provide host module `<module>`" ``
**Primary label:** `` "no host module named `<module>` on this target" ``
**Help:** if another declared target provides the module, name it; if a library provides an equivalent, name the library.
**Suggestion:** `MachineApplicable` — switch `clean.toml [build] target` to a target that provides the module, when exactly one such target is available.

### COM008 — Target Size Budget Exceeded
The compiled artifact exceeds the size budget declared for the target in `clean.toml [compile.limits]` (schema home: [`07-build-config.md`](./07-build-config.md)). Applies mostly to `wasm32-browser` and embedded targets where startup cost matters.

**Headline template:** `` "output size <bytes> exceeds target budget <max> for `<target>`" ``
**Primary label:** *not applicable* — this is a whole-program diagnostic with no single span. The renderer omits the caret block and lists the top five largest functions in the `note:` section.
**Help:** list the largest functions and suggest running `cln analyze --size` for a full breakdown.
**Suggestion:** none — size fixes require author judgment.

### COM009 — Bridge Resolve Error

No single version assignment satisfies every WIT package constraint in the build.

**Condition:** When solving package versions ([08 §8.3](./08-bridge-versioning.md#83-compiler-resolution)) — one version per WIT package satisfying the target world's constraints and every library's constraints — the compiler MUST emit `COM009` if no valid assignment exists. The diagnostic MUST list every conflicting constraint, naming packages and constraints as version ranges, never as SAT-solver internals.

**Message template:** `"no version of '{package}' satisfies all constraints: {constraint_list}"`
**Primary label:** *not applicable* — this is a whole-build diagnostic; the renderer lists each constraint with the dependency that introduced it in the `note:` section.

**Example (fails — COM009):** the target world requires `"clean:bridge" = "^0.1.0"` while a library's `library.toml` pins `"clean:bridge" = "0.2.0"`. No single version satisfies both ranges.

**Suggested fix:** none machine-applicable — name the dependency whose constraint must be relaxed or upgraded.

### COM010 — Bridge Link Error

Link-time verification of the guest component against the target world failed.

**Condition:** Before emitting the final `.wasm` component, the compiler MUST verify ([08 §8.5](./08-bridge-versioning.md#85-link-time-verification)): (1) every WIT interface the guest imports is listed in the target world at the resolved version; (2) every WIT type used across the boundary has the same shape on both sides; (3) every resource matches on lifetime and method set. Any failure MUST produce `COM010` with side-by-side WIT excerpts showing the mismatch, in machine-parseable output (so the language server can render a code action).

**Message template:** `"link check failed for '{interface}': {mismatch}"`
**Primary label:** `"this use links against '{interface}'"` — placed at the source construct whose lowering imports the mismatched interface.

**Example (fails — COM010):** the guest imports `clean:bridge/db@0.1.0` whose `query` returns `list<row>`, but the resolved world declares `query` returning `result<list<row>, error>` — a shape mismatch reported with both excerpts.

**Suggested fix:** none machine-applicable — the fix is aligning library and world versions (usually via `cln update` or a constraint change).

### COM011 — Bridge Runtime Mismatch

*(Runtime — raised by the host, not the compiler.)*

**Condition:** At instantiation, the host MUST read the guest component's declared imports and match each against its own `host.wit` ([08 §8.6](./08-bridge-versioning.md#86-runtime-version-check), the Moment 3 version check of [16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)). If any import is missing or carries a different version than the host provides, the host MUST reject instantiation and report `COM011` through the [error reporting pipeline](./06-error-reporting.md), including the guest's expected WIT and the host's actual WIT.

**Message template:** `"cannot instantiate component: requires {package}@{expected}, host provides {package}@{actual}"`

**Example (triggers — COM011 at runtime):** a component built against `clean:bridge/db@0.1.0` is deployed to a host upgraded to `clean:bridge/db@0.2.0` (upgrade drift). The loader rejects it before any guest code runs.

**Suggested fix (rendered as `hint:`):** rebuild the component against the upgraded host, or downgrade the host to a release providing the required version. Instantiation failures with a cause other than import/version mismatch are [`COM017`](#com017--instantiation-failure).

### COM012 — Host Import Not In World

A `host function` call site imports a function that is not in the target world's WIT.

**Condition:** The compiler MUST walk every `host function` call site in MIR and verify its signature exists in the target world WIT *as delivered in the compilation request* (for library-declared imports, `library_manifests[].wit`) — [14 §14.4.2 pass 9](./14-compiler-architecture.md#1442-detailed-pass-responsibilities). Any call site whose imported function is not in the world MUST produce `COM012` and abort before codegen. The compiler MUST NOT fetch a host's WIT to perform this check (scope split: [16 §16.10](./16-host-contract-validation.md#1610-component-responsibilities)).

**Message template:** `"your program uses '{interface}' but you compiled for '{world}', which does not provide it"`
**Primary label:** `"this call requires '{interface}'"` — placed at the call site.

**Example (fails — COM012):**
```clean
// clean.toml sets target = "wasm32-browser"; the browser world has no clean:host/routing
start:
	print(request.path())    // error: uses clean:host/routing, not in world `browser`
```

**Suggested fix:** `MaybeIncorrect` — switch the build target to a world providing the interface, or remove the construct that requires it.

### COM013 — Codegen Internal Invariant

A self-produced compiler artifact failed the compiler's own validation — an internal compiler error (ICE).

**Condition:** Every artifact the compiler produces (component binary, build manifest, source map) MUST be validated before it leaves the compiler; a validation failure MUST be reported as `COM013` and treated as a compiler bug, never a user error ([CMP-04, 14 §14.6](./14-compiler-architecture.md#146-diagnostics-and-error-handling)). A byte-divergence detected during build replay (`cln repro build`, [14 §14.14.6](./14-compiler-architecture.md#14146-build-reproduction-and-request-replay-behind-cln-repro-build)) is likewise `COM013` (or manifest corruption).

**Message template:** `"internal compiler invariant violated: {invariant} — this is a compiler bug, please report it"`
**Primary label:** *not applicable* — no user span; the diagnostic carries the compiler version and the failing pass in `note:`.

**Suggested fix:** none — the offered action is filing the report (`report_error` on `component=compiler`).

### COM014 — World Mismatch

*(Emitted by Clean Framework at Moment 1, not by the compiler.)*

**Condition:** During `cln build`, before invoking the compiler, Clean Framework MUST compare the project's required imports against the target host's fetched `host.wit` (Moment 1 — [16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)). If the target host does not provide a required interface, the build MUST fail with `COM014`, naming the missing interface and the source constructs that need it. The compiler is never invoked.

**Message template:** `"target host '{host}' does not provide required interface '{interface}'"`
**Primary label:** *not applicable* — rendered with a `note:` listing the blocks/files that require the interface and a `hint:` naming hosts that do provide it.

**Example (fails — COM014):** a project with `endpoints:` blocks (requiring `clean:host/routing@0.1.0`) built with `host = "browser"` — the browser `host.wit` has no `routing` interface ([16 §16.8 case B](./16-host-contract-validation.md)).

**Suggested fix:** switch the target to a host providing the interface, add such a target alongside, or remove the requiring construct.

### COM015 — Version Mismatch

*(Emitted by Clean Framework at Moment 2, not by the compiler.)*

**Condition:** `cln check <host>` MUST compare the built component's imports against the WIT published by a concrete (possibly live) host deployment (Moment 2 — [16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)). A required package that the host provides only at a semver-incompatible version — or not at all — MUST be reported as `COM015` with both versions shown.

**Message template:** `"host '{host}' provides '{package}@{actual}', component requires '{package}@{expected}'"`

**Example (fails — COM015):** `cln check https://prod.example.com` reports that prod ships `clean:bridge/db@0.2.0` while the component was built against `@0.1.0` ([16 §16.8 case C](./16-host-contract-validation.md) — caught before deploy instead of failing at Moment 3).

**Suggested fix:** rebuild against the deployed host's versions, or upgrade/downgrade the deployment.

### COM016 — Deprecated Member Use

*(Warning.)* The program uses a WIT interface member marked `@deprecated`.

**Condition:** The compiler MUST emit a `COM016` warning at every call or use site of an interface member marked `@deprecated` in the resolved WIT ([15 §9.2](./15-component-model-architecture.md#92-deprecation-of-interface-members), [08 §8.7](./08-bridge-versioning.md#87-deprecation-protocol)), carrying the deprecation message and its replacement pointer. Non-deprecated members MUST NOT trigger it.

**Message template:** `"'{member}' is deprecated: {deprecation_message}"`
**Primary label:** `"deprecated member used here"` — placed at the call site.
**Help:** name the replacement member declared in the `@deprecated` annotation.

**Suggested fix:** `MaybeIncorrect` — replace the call with the declared replacement when its signature is compatible.

### COM017 — Instantiation Failure

*(Runtime — raised by the host loader at Moment 3.)*

**Condition:** When the host's Moment 3 check ([16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)) fails to instantiate a component for any reason *other than* the import/version mismatch owned by [`COM011`](#com011--bridge-runtime-mismatch) — e.g. a world-required export missing from the component, a malformed or tampered component binary, or a canonical-ABI validation failure — the host MUST reject the load and report `COM017` as a structured error naming the component and the cause.

**Message template:** `"cannot instantiate component '{component}': {cause}"`

**Suggested fix (rendered as `hint:`):** rebuild the component (`cln build`) and redeploy; if the failure persists on a freshly built component, file it against the host.

---

## 12. Build Rules (BLD)


### BLD001 — Build Limit Exceeded

A build-scoped hard cap from `clean.toml [compile.limits]` was exceeded.

**Condition:** The build-scoped limits of `[compile.limits]` ([07 §7.8](./07-build-config.md#78-compile-time-limits)) — `total-timeout-min`, `max-file-size-mb`, `max-import-depth`, `max-nesting-depth` — are hard caps, not soft warnings. Exceeding any of them MUST abort the build with `BLD001`, naming the limit and the observed value. The per-handler and per-library budgets in the same table are enforced as [`BLOCK005`](../04%20language/21-block-handlers.md#216-diagnostics-from-compile-time-functions) and [`LIB014`](#lib014--library-resource-limit), not as BLD001.

**What counts toward `max-nesting-depth`.** The limit bounds the *structural depth* of a source file: the length of the longest chain of nested countable constructs in its parse tree, all families counted into one depth — a construct's depth is one more than that of the innermost enclosing countable construct of *any* family. The countable constructs are:

- **expression nesting** — every expression node: a parenthesized group, a unary or binary operator application, a call or index argument, a collection-literal element, a string-interpolation splice. An operator chain nests one level per application under its associativity, so a 300-term sum exceeds the default — deliberately: every recursive pass walks that chain by recursion, and an expressions-only or brackets-only count would leave the abort reproducible through the uncounted form;
- **statement/block nesting** — one level per INDENT level;
- **type-expression nesting** — one level per generic argument layer (each `list<…>` of a `list<list<…>>`).

**Enforcement point.** The limit MUST be enforced no later than the end of parsing, so every later pass may assume structural depth ≤ `max-nesting-depth`. This is what turns the cap into a guarantee for implementations — a recursive-descent pipeline is entitled to recurse to the configured depth and no further input shape can drive it past it. `{actual}` is `max + 1`: the first depth that exceeds the limit, where a left-to-right parse stops counting — not the input's full depth, which the parser need never discover.

**Message template:** `"build limit '{limit}' exceeded: {actual} > {max}"`
**Primary label:** `"this file/import exceeds '{limit}'"` when the violation is a file size or import depth; `"nesting here exceeds 'max-nesting-depth'"` for the nesting limit, anchored at the earliest token at which a left-to-right parse establishes that a construct's depth exceeds the limit; *not applicable* for the whole-build timeout, which is rendered without a caret block.

**Example (fails — BLD001):** with the default `max-file-size-mb = 4`, a generated 6 MB `app/data/seed.cln` aborts the build: `build limit 'max-file-size-mb' exceeded: 6291456 > 4194304`.

**Example (fails — BLD001):** with the default `max-nesting-depth = 256`, a generated expression opening 300 nested parentheses aborts at the 257th `(`: `build limit 'max-nesting-depth' exceeded: 257 > 256`.

**Suggested fix:** none machine-applicable — raise the limit in `[compile.limits]` deliberately, or restructure the offending input.

---

## 13. Runtime Rules (RUN)


### RUN001 — Memory Violation
WASM execution attempted to access memory outside allocated bounds.

### RUN002 — Stack Error
WASM stack overflow or underflow during execution.

### RUN003 — Arithmetic Error

An integer arithmetic operation or a numeric conversion failed at runtime.

**Condition:** The following raise sites — and only these — raise `RUN003`:

1. `integer` division (`/`) with a zero right operand;
2. `integer` remainder (`%`) with a zero right operand;
3. `integer` division overflow — the minimum `integer` divided by `-1`, whose true quotient does not fit (`integer` remainder does not overflow: the minimum `integer` `%` `-1` is `0`);
4. `number.toInteger()` on NaN;
5. `number.toInteger()` on a value outside the `integer` range, the infinities included;
6. `string.toInteger()` or `string.toNumber()` on a string that is not a valid literal of the target type ([15 §Conversions](../04%20language/15-standard-library.md#conversions)).

**Boundary:** `number` arithmetic never raises. It follows IEEE 754 ([15 §Math Module](../04%20language/15-standard-library.md#math-module)): `1.0 / 0.0` is `Infinity`, `-1.0 / 0.0` is `-Infinity`, `0.0 / 0.0` is NaN, and `math.*` domain errors are NaN — a raising reading of "division by zero" applies to `integer` operands only.

**Message templates** (one per raise site, in the order above). These strings are the `error.message` an `onError` handler observes ([13 §ERH-04](../04%20language/13-error-handling.md#erh-04--the-error-binding-is-an-error-value)), so they are normative byte-for-byte:

1. `"division by zero"`
2. `"division by zero"`
3. `"integer overflow in division"`
4. `"cannot convert NaN to integer"`
5. `"number is out of the integer range"`
6. `"the string is not a valid integer literal"` for `toInteger`, `"the string is not a valid number literal"` for `toNumber`

**Primary label:** `"fails at runtime"`, anchored at the operation or conversion call when the runtime can map the failure to a source span; *not applicable* otherwise.

**Example (fails — RUN003):**
```clean
integer a = 10
integer b = 0
integer q = a / b        // raises RUN003 — message: division by zero
```

**Example (passes — no RUN003):**
```clean
number x = 1.0 / 0.0     // x is Infinity; number division is IEEE 754 and never raises
```

**Suggested fix:** guard the divisor or validate the string before converting, or catch with `onError` ([13 — Error Handling](../04%20language/13-error-handling.md)).

### RUN004 — Reference Error
A `none` or invalid reference was accessed at runtime. This is raised by the `!` operator on a `none` value; see [`13-error-handling.md`](../04%20language/13-error-handling.md) and [`04-type-system.md`](../04%20language/04-type-system.md) §"Optional and none".

### RUN005 — Assertion Failure
A `before` statement evaluated to false at runtime. See [`10-contracts.md`](../04%20language/10-contracts.md).

### RUN006 — Json Parse Error
Generic malformed JSON input to `json.textToData`. Raised when no more specific JSON code (RUN007–RUN010) applies. `json.tryTextToData` returns `none` under exactly the same conditions instead of raising. See [`../15-standard-library.md`](../04%20language/15-standard-library.md) §JSON Module and [`../11-testing.md`](../04%20language/11-testing.md) §Conformance Testing for Standard-Library Parsers.

### RUN007 — Json Invalid Number
**Condition:** A JSON number that is malformed, or whose value cannot be represented, MUST be rejected with `RUN007`. The complete list of rejected forms — this is the accept/reject boundary itself, not a summary of one held elsewhere:

- leading zeros on a multi-digit integer (`01`)
- a missing integer digit (`.5`)
- a trailing decimal point (`5.`)
- a missing exponent digit (`1e`, `1e+`)
- a magnitude that binary64 cannot hold without becoming infinite (`1e999`). It is rejected rather than read as infinity: silently substituting an unbounded value for a bounded one loses the fact that the input was out of range.

`-0` is **accepted** and yields the binary64 negative zero, which compares equal to `0`. It is a representable value of the target type, and rejecting it would refuse input that round-trips correctly.

**Message template:** `"invalid JSON number at offset {offset}: {reason}"`
**Primary label:** *not applicable* — the failure is in parsed data, not in source text.

**Example (passes):** `-0`, `1e308`, `0.5`.

**Example (fails — RUN007):** `1e999` — `invalid JSON number at offset 0: magnitude exceeds the range of number`.

**Suggested fix:** none machine-applicable — the input is data, and correcting it is outside the program.

### RUN008 — Json Invalid String
A JSON string is not a well-formed UTF-8 sequence, contains a lone surrogate, uses an unrecognized `\` escape, or is not terminated before end of input.

### RUN009 — Json Invalid Structure
**Condition:** A JSON document that is structurally invalid MUST be rejected with `RUN009`: an unmatched `[`, `]`, `{` or `}`; a missing or extra comma; non-whitespace content after the root value; or **a duplicate object key**.

Duplicate keys are rejected rather than resolved. Last-wins and first-wins are equally arbitrary and both discard data the input contained, without the program ever learning that two values were offered for one key. Rejecting is the only resolution that loses nothing silently, and it matches the stance the ecosystem already takes on ambiguous input elsewhere — `RQD002` refuses a request document with an unknown key rather than ignoring it.

**Message template:** `"invalid JSON structure at offset {offset}: {reason}"`
**Primary label:** *not applicable* — the failure is in parsed data, not in source text.

**Example (passes):** `{"a": 1, "b": 2}`.

**Example (fails — RUN009):** `{"a": 1, "a": 2}` — `invalid JSON structure at offset 9: duplicate key "a"`.

**Suggested fix:** none machine-applicable.

### RUN010 — Json Depth Exceeded
**Condition:** JSON nesting deeper than **1000 levels**, arrays and objects combined, MUST be rejected with `RUN010`. The limit is fixed here and is the same on every host: the parser is compiled to WASM once and is not routed through the bridge ([Platform 02 §2.2.1](./02-host-bridge.md#221-portable-l2-in-every-world)), so no host can widen or narrow it.

**Message template:** `"JSON nesting exceeded {limit} levels"`
**Primary label:** *not applicable* — the failure is in parsed data, not in source text.

**Example (fails — RUN010):** 1001 nested arrays.

**Suggested fix:** none machine-applicable.

### RUN011 — Contract Violation

An `after:` postcondition or an `always` invariant evaluated to false at runtime.

**Condition:** When an `after:` expression evaluates to false on a return path, or an `always` invariant evaluates to false at one of its check points (after the constructor, before/after every public method call — [10 — Contracts](../04%20language/10-contracts.md)), execution MUST stop and the failure MUST be reported as `RUN011`, naming the contract kind, the location, and the failing expression. This includes entity `always:` invariants evaluated by `Database.save` before persisting: on failure, `RUN011` is raised and the entity is **not** persisted ([data library](../02%20components/framework/libraries/04-data.md)).

**Boundary:** a `before` precondition that evaluates to false is [`RUN005`](#run005--assertion-failure), never RUN011. In a build compiled with `--strip-checks`, `after` and `always` expressions are not emitted at all, so no RUN011 can be raised ([10 — Contracts §6](../04%20language/10-contracts.md)); `before` — and therefore RUN005 — is unaffected.

**Message template:**
```
Contract violation: {after|always} failed at {location}
  Expression: {expression}
```

**Example (triggers — RUN011 at runtime):**
```clean
class BankAccount
	always balance >= 0

	number balance

	void withdraw(number amount)
		balance = balance - amount    // withdrawing more than the balance
                                      // leaves balance < 0 → RUN011 after the call
```

**Suggested fix:** none — a contract violation is a program bug at the call site or in the method body; the diagnostic's `note:` carries the receiver's state summary when the host can render it.

### RUN012 — Time Budget Exceeded

*(Runtime — raised by the host's epoch enforcement, not by the compiler.)*

**Condition:** Request- and frame-scoped instances MUST be bound to a wall-clock budget ([03 §3.5](./03-memory-model.md#35-host-backing--observable-contract)); the per-invocation defaults are owned by `clean.toml [runtime] epoch-ms` ([07 §7.2](./07-build-config.md#72-schema--top-level)). When an invocation exhausts its budget, the host MUST trap the instance and report the trap as `RUN012`, naming the configured budget and the elapsed time.

**Message template:** `"time budget exceeded: invocation ran {elapsed} ms, budget is {budget} ms"`

**Example (triggers — RUN012 at runtime):** under the server default `epoch-ms = 5000`, a request handler that loops without completing is trapped at 5 s; the request fails with RUN012 and the per-request arena is reset as usual.

**Suggested fix (rendered as `hint:`):** raise `[runtime] epoch-ms` deliberately if the workload genuinely needs longer, or bound the offending loop / move the work to an async task.

---

### RUN013 — Index Out Of Range

A collection or string access is outside the valid range at runtime.

**Condition:** an index passed to `items.get`, `items.remove`, `text.charAt` or `text.charCodeAt` is negative or not less than the length; or `first()`, `last()`, `remove()` or `peek()` is called on an empty collection.

**Message:** `"Index {index} is out of range for a {kind} of length {length}"`

`{kind}` names the receiver's kind (`list`, `string`, and so on). For the empty-collection arm — `first()`, `last()`, `remove()` or `peek()` on an empty collection — the fill values are fixed: `{index}` is `0` and `{length}` is `0`, rendering, for a list, `Index 0 is out of range for a list of length 0`. The statement is literally true of the failed access (index 0 does not exist in a length-0 collection), and one template per code beats a second template whose only gain is prose.

**Primary label:** `out of range`

**Example (fails — RUN013):**
```clean
list<integer> items = [1, 2, 3]
integer x = items.get(5)
```

**Example (passes):**
```clean
list<integer> items = [1, 2, 3]
integer x = items.get(2)
```

**Suggested fix:** check `length()` first, or guard with `isEmpty()`. Catchable with `onError` ([13 — Error Handling](../04%20language/13-error-handling.md)).

### RUN014 — File Operation Failed

A `file.*` operation did not complete.

**Condition:** the host reports failure for a read, write or append — the path does not exist, permission was denied, or the write could not be completed.

**Message:** `"File operation failed on '{path}': {reason}"`

**Primary label:** `file operation failed`

**Example (fails — RUN014):**
```clean
string content = file.read("/does/not/exist.txt")
```

**Example (passes):**
```clean
string content = file.read("/does/not/exist.txt") onError ""
```

**Suggested fix:** handle the failure with `onError`, or test with `file.exists` first.

### RUN015 — Http Request Failed

An `http.*` request did not complete.

**Condition:** the host reports that the request could not be completed — the connection failed, the request timed out, or the response could not be read. A completed request carrying a non-2xx status is **not** this code: the response is returned and the status is the caller's to inspect.

**Message:** `"HTTP request to '{url}' failed: {reason}"`

**Primary label:** `request failed`

**Example (fails — RUN015):**
```clean
string body = http.get("https://unreachable.invalid/data")
```

**Example (passes):**
```clean
string body = http.get("https://unreachable.invalid/data") onError ""
```

**Suggested fix:** handle the failure with `onError`.

### RUN016 — Matrix Shape Mismatch

**Condition:** A `matrix<T>` operation MUST trap with `RUN016` when the shapes it is given do not admit it: `A * B` where the column count of `A` differs from the row count of `B`; `A + B` or `A - B` where the two shapes differ; `inverse()` or `determinant()` on a matrix that is not square. `matrix<T>` is dynamically sized, so shape is not part of the type and this cannot be decided before the values exist ([15 §Matrix Module](../04%20language/15-standard-library.md)).

**Message template:** `"matrix operation '{op}' does not admit shapes {left} and {right}"`, and for the single-operand cases `"matrix operation '{op}' requires a square matrix, got {shape}"`
**Primary label:** `"shapes are {left} and {right}"`

**Example (passes):** a 2×3 multiplied by a 3×2 yields a 2×2.

**Example (fails — RUN016):** a 2×3 multiplied by a 2×3 — `matrix operation '*' does not admit shapes 2x3 and 2x3`.

**Suggested fix:** none machine-applicable — the shapes are a property of the data, and correcting them is a change of algorithm, not of syntax.

### RUN017 — Matrix Singular

**Condition:** `inverse()` MUST trap with `RUN017` when the matrix is square but its determinant is zero, so no inverse exists. This is distinct from [`RUN016`](#run016--matrix-shape-mismatch): the shape is admissible and the values are not.

**Message template:** `"matrix is singular: determinant is zero, so it has no inverse"`
**Primary label:** `"inverse() of a singular matrix"`

**Example (passes):** the 2×2 matrix `[[1, 0], [0, 1]]` has determinant 1 and inverts to itself.

**Example (fails — RUN017):** `[[1, 2], [2, 4]]` — the second row is twice the first, the determinant is zero, and `inverse()` traps.

**Suggested fix:** none machine-applicable — test with `determinant()` before calling `inverse()` where the matrix may be singular.

### RUN018 — Unhandled Error

**Condition:** A failure that propagates to the top of the program with no enclosing `onError` MUST end execution with `RUN018` ([ERH-05](../04%20language/13-error-handling.md#erh-05--an-unhandled-failure-ends-the-program)). The report MUST carry both fields of the `Error`: its message, and its code where the failure was raised by the runtime. This MUST NOT be reported at compile time — under [ERH-01](../04%20language/13-error-handling.md#erh-01--raising-an-error) no signature records that a function can fail, so whether a given call fails is not statically decidable.

**Message template:** `"unhandled failure: {message}"`, with `" ({code})"` appended when the code is not empty
**Primary label:** *not applicable* — the failure surfaces where it escaped, which is the program's entry point, not the expression that raised it.

**Example (passes):** `integer v = riskyCall() onError 0` — the failure is handled and execution continues.

**Example (fails — RUN018):** a `start:` block calling `file.read(path)` with no `onError` anywhere, on a path that does not exist — `unhandled failure: no such file (RUN006)`.

**Suggested fix:** none machine-applicable — handling the failure is a design decision about what the program should do instead.

### RUN019 — Read Of Cancelled Task

**Condition:** Reading a deferred (`later`) binding whose task was cancelled with `cancel()` MUST raise `RUN019` ([ASY-03](../04%20language/18-async.md#asy-03--cancelling-and-failing)). A cancelled task produces no value, so there is nothing for the read to block on and nothing to return. Cancelling a task that has already completed is not an error and does not make its binding unreadable.

**Message template:** `"'{name}' was cancelled and has no value to read"`
**Primary label:** `"read of a cancelled task"`

**Example (passes):** cancelling a binding that is never read afterwards; or cancelling one whose task had already completed, then reading it.

**Example (fails — RUN019):** `page.cancel()` followed by `print(page)`.

**Suggested fix:** none machine-applicable — a program that may cancel must decide what to use instead of the value, which the compiler cannot choose.

## 14. Memory Rules (MEM)


Policy home: [05 — Memory Policy](./05-memory-policy.md). MEM001 and MEM003 are enforced by the host at runtime; MEM002 is a compile-time warning.

### MEM001 — Tier Exceeded

*(Runtime — raised by the host's memory enforcement, not by the compiler.)*

**Condition:** Every guest instance's memory MUST be capped at its tier's `max_bytes` ([05 §5.1](./05-memory-policy.md#51-memory-tiers)). A `memory.grow` beyond the tier limit MUST trap at the offending call site — trapping is not optional, and no grow-failure return value is observable by guest code ([05 §5.3](./05-memory-policy.md#53-enforcement)). The trap MUST be reported through the [error reporting pipeline](./06-error-reporting.md) as `MEM001` with `severity: crash`, attributed by span to `compiler|library|application`.

**Message template:** `"Memory grow to {n} bytes exceeded tier limit {m} bytes"`

**Example (triggers — MEM001 at runtime):** under `tier = "standard"` (32 MiB maximum), a loop that keeps appending to a `list<string>` eventually forces a grow past 32 MiB; the grow traps at that append.

**Suggested fix (rendered as `hint:`):** pick a higher tier in `clean.toml [memory]` if the workload genuinely needs it, or bound the allocation (the metrics of [05 §5.5](./05-memory-policy.md#55-observability) show peak usage and rejected grows).

### MEM002 — Arena Escape

*(Warning.)* A value allocated in a reset-scoped arena is stored in a persistent structure that outlives the arena.

**Condition:** The compiler MUST emit a `MEM002` warning when it detects a value allocated in a request-, frame-, or task-scoped arena ([05 §5.4](./05-memory-policy.md#54-reset-policies)) being stored in a persistent structure that survives the arena's reset (e.g. module-level `state:`). Dereferencing such a value after the reset traps at runtime; the warning surfaces the bug class "works for the first request, mysteriously broken for the second" at compile time.

**Message template:** `"value allocated in the {reset_policy} arena is stored in '{name}', which outlives the arena reset"`
**Primary label:** `"this value does not survive the next {reset_policy} reset"` — placed at the stored expression.
**Secondary label:** `"'{name}' is persistent"` — placed at the persistent declaration.

**Example (fails — MEM002):**
```clean
state:
	list<string> seen = []

functions:
	void handle(string requestBody)
		seen.add(requestBody)    // warning: request-scoped value stored in persistent state
```

**Suggested fix:** `MaybeIncorrect` — copy the value into persistent storage explicitly (e.g. `seen.add(requestBody.copy())`) when retention is intended.

### MEM003 — Arena Imbalance

*(Runtime — raised by the runtime's arena discipline, not by the compiler.)*

**Condition:** Code that calls `arena-push` / `arena-pop` directly (via `clean:bridge/mem`) MUST balance every push with exactly one pop and MUST NOT pop past a save-point it did not receive ([03 MMD-03](./03-memory-model.md#mmd-03--arena-discipline-every-push-balanced-by-exactly-one-pop)). An `arena-pop` without a balanced push, or a pop whose save-point lies outside the arena stack the caller owns, MUST trap and be reported as `MEM003`. Scopes that are pushed and never popped do not trap — they surface as the `clean_wasm_arena_leak_bytes` metric on instance drop ([03 §3.9](./03-memory-model.md#39-debugging-and-observability)).

**Message template:** `"arena imbalance: {pop without matching push | pop past a save-point not owned by the caller} at {location}"`

**Example (triggers — MEM003 at runtime):** a library captures the save-point of its caller's outer scope and pops it from inside a nested handler, invalidating allocations its caller still holds — the pop traps with MEM003.

**Suggested fix:** none — pair each `arena-push` with its `arena-pop` in the same scope, and only pop save-points received from your own push.

---

## 15. Configuration Rules (CFG)


Enforced by Clean Framework and Clean Manager when validating `clean.toml` / `library.toml` ([07 §7.10](./07-build-config.md#710-validation)) — the compiler never reads these files; it validates the request document instead (§16).

### CFG001 — Manifest Schema Violation

**Condition:** A manifest that fails schema validation — an unknown key, a value of the wrong type, or a missing required field — MUST be rejected with `CFG001`, naming the file, the offending key path, and the reason. There is no "ignore-and-continue" for unknown keys.

**Message template:** `"invalid {file}: {reason} at '{key_path}'"`
**Primary label:** `"unknown key"` / `"expected {expected_type}, got {actual_type}"` / `"required key missing"` — placed at the offending line of the manifest.

**Example (fails — CFG001):**
```toml
[build]
targett = "wasm32-server"    # error: unknown key 'build.targett'
```

**Suggested fix:** `MaybeIncorrect` — when the unknown key is within edit distance 2 of a schema key, offer the rename (`targett` → `target`).

### CFG002 — Manifest Constraint Violation

**Condition:** A manifest whose keys are individually valid but jointly inconsistent MUST be rejected with `CFG002`, naming every key involved in the violated constraint. Registered constraints include: `[memory] memory64 = true` without `build.memory64 = true` ([07 §7.3](./07-build-config.md#73-memory--full-schema)); memory tier disagreements; folder path conflicts in `[folders]` ([07 §7.10](./07-build-config.md#710-validation)); and a project tier below a dependency library's declared minimum — the build is rejected, not warned ([05 §5.1](./05-memory-policy.md#51-memory-tiers)).

**Message template:** `"configuration constraint violated: {constraint} (keys: {key_list})"`

**Example (fails — CFG002):**
```toml
[memory]
memory64 = true      # error: requires build.memory64 = true, which is absent

[build]
target = "wasm32-server"
```

**Suggested fix:** `MachineApplicable` when the constraint has a single completing edit (add `build.memory64 = true`); otherwise `MaybeIncorrect`.

### CFG003 — Manifest Warning

*(Warning.)* The manifest is valid but contains a suspicious declaration.

**Condition:** Validation MUST emit a `CFG003` warning — and MUST NOT reject the manifest — for semantic-warning findings: a deprecated key, or a custom profile shadowing a built-in ([07 §7.10](./07-build-config.md#710-validation)).

**Message template:** `"{finding} — {consequence}"` (e.g. `"key 'x' is deprecated — use 'y' instead"`)

**Suggested fix:** `MachineApplicable` for deprecated-key renames; none otherwise.

> **Boundary (resolved 2026-08-01).** "Project tier below a library's declared minimum" is an **error**, not a warning: the case belongs to [`CFG002`](#cfg002--manifest-constraint-violation) (the build is rejected, per [05 §5.1](./05-memory-policy.md#51-memory-tiers)). [07 §7.10](./07-build-config.md#710-validation) has been corrected to list it under CFG002; CFG003 never covers it.

### CFG004 — Lockfile Mismatch

**Condition:** A CI build MUST fail with `CFG004` when `clean.toml` and `.cln/lock.toml` disagree — a dependency present in one but not the other, or a declared version constraint that the locked version no longer satisfies ([07 CONF-04](./07-build-config.md#77-dependencies); the lockfile is written by Clean Manager, [Manager §00.3.2](../02%20components/manager/00-manager.md#0032-dependencies)). The diagnostic MUST name every diverging dependency with its declared constraint and its locked version. The remedy is regenerating the lock, never hand-editing it.

**Message template:** `"lockfile out of date: '{name}' declared {constraint} but locked at {locked_version}"`
**Primary label:** `"declaration disagrees with .cln/lock.toml"` — placed at the `[dependencies]` entry in `clean.toml`.

**Example (fails — CFG004):** `clean.toml` bumps `"data" = "1.5.0"` but the committed `.cln/lock.toml` still pins `data 1.4.0`; CI fails with `lockfile out of date: 'data' declared 1.5.0 but locked at 1.4.0`.

**Suggested fix:** none machine-applicable in CI — run `cln lock` locally and commit the regenerated `.cln/lock.toml`.

### CFG005 — File Encoding Invalid

**Condition:** A component that reads a project file from disk or from an editor buffer — Clean Framework, Clean Manager, the language server, or any harness assembling a compilation request — MUST validate the raw bytes as well-formed UTF-8 before decoding them, and MUST refuse the file with `CFG005` when they are not ([TXT-01](./17-text-encoding.md#txt-01--every-ecosystem-text-file-is-utf-8), [TXT-02](./17-text-encoding.md#txt-02--the-reader-validates-at-the-moment-it-reads)). The diagnostic MUST name the path and the byte offset of the first ill-formed sequence. The reader MUST NOT substitute, strip, or replace the offending bytes, and MUST NOT emit `U+FFFD`: a file that fails this check is refused whole, never repaired. The rule covers every file the toolchain reads — `.cln` sources, `clean.toml`, `library.toml`, lockfiles — and no file a Clean program reads at runtime ([17 §17.1](./17-text-encoding.md#171-scope)).

**Message template:** `"file is not valid UTF-8: {path} (first invalid byte at offset {offset})"`
**Primary label:** *not applicable* — the file cannot be decoded, so no source span can be rendered; the diagnostic is emitted without a caret block, like the whole-build case of [`BLD001`](#bld001--build-limit-exceeded).

**Example (passes):** `app/main.cln` saved as UTF-8, containing `print("año")` — the `ñ` occupies the two bytes `C3 B1`, a well-formed sequence.

**Example (fails — CFG005):** the same file saved as Latin-1, where `ñ` is the single byte `F1`. `F1` announces a four-byte sequence, but the byte after it is the ASCII `o`, which is not a continuation byte: `file is not valid UTF-8: app/main.cln (first invalid byte at offset 12)`.

**Suggested fix:** none machine-applicable — the correct original text cannot be recovered by guessing a table. Re-save the file as UTF-8 in the editor that produced it.

---

## 16. Compilation Request Rules (RQD)


The compilation request document is the compiler's only input ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs), [CMP-01](./14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)). These rules fire before any source is parsed.

### RQD001 — Request Integrity Failure

**Condition:** The compiler MUST verify that every `sources[].sha256` is the hex-lowercase SHA-256 of that entry's decoded UTF-8 `content`. On any mismatch the compiler MUST refuse the *entire* request with `RQD001`, naming the path and both hashes. No partial compilation is attempted.

**Message template:** `"request integrity failure: '{path}' declares sha256 {declared}, content hashes to {actual}"`

**Example (fails — RQD001):** a caller edits `app/main.cln`'s `content` in the request without recomputing its `sha256`; the compiler refuses the request and names `app/main.cln`.

**Suggested fix:** none user-facing — the caller (Clean Framework, or an AI harness) must rebuild the request; a hand-assembled request recomputes the hash.

### RQD002 — Request Schema Violation

**Condition:** The compiler MUST reject a request document with `RQD002` when it contains an unknown top-level key, an unknown key inside a well-known section (error scoped to that section), a missing required field, or a malformed value ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)). There is no ignore-and-continue for schema drift. A `spec_version` the compiler does not support is likewise rejected under this rule, naming the highest supported version.

**Message template:** `"invalid compilation request: {reason} at '{json_path}'"`

**Example (fails — RQD002):** a request carrying `"target": "wasm32-server"` at top level (the target lives inside `build`) is rejected with `unknown top-level key at '$.target'`.

**Suggested fix:** none user-facing — the request is produced by tooling; the diagnostic is aimed at the calling component's maintainer.

---

## 17. Capability Wiring Rules (CAP)

*Normative.* Concerns: [C-04](../01%20governance/05-concerns.md), [C-13](../01%20governance/05-concerns.md), [C-15](../01%20governance/05-concerns.md).

Emitted by Clean Framework at Moment 1 (`cln build`) while resolving each capability the guest imports to a bridge backend ([framework 11 §11.10](../02%20components/framework/11-build-orchestration.md#1110-host-configuration-generation)). All three are build-time errors raised before the host is ever started; none of them replaces the runtime's own refusal to start with an unsatisfied import ([SRVH-02](../02%20components/hosts/clean-server/01-server.md#srvh-02--absent-configuration-means-the-capability-is-off)).

**Boundary with `COM014`:** if the target host's `host.wit` does not declare the interface at all, the failure is [`COM014`](#com014--world-mismatch) — the host cannot host that capability under any backend. CAP codes apply only when the host *does* provide the interface and the missing piece is the implementation behind it.

### CAP001 — No Backend Available

**Condition:** The framework MUST emit `CAP001` when the guest imports a capability the target host provides, but no backend implementation exists for that (host, capability) pair in the [compatibility matrix](../02%20components/hosts/01-compatibility-matrix.md). This is the ❌ case of that matrix — a combination ruled out by the host's shape, not by what the developer has installed. No `backend` value can satisfy it.

**Message template:** `"capability '{capability}' has no backend available on host '{host}'"`

**Example (fails — CAP001):** a project targeting `wasm32-browser` whose guest imports `clean:mail/send`. Browsers do not send mail directly; the matrix marks the pair ❌.

**Suggested fix:** applicability `MaybeIncorrect` — the alternatives are architectural (call a backend endpoint that sends the mail, or change the build target), so the diagnostic names them and does not offer an automatic edit.

### CAP002 — Backend Not Installed

**Condition:** The framework MUST emit `CAP002` when the guest imports a capability whose selected backend — declared in `clean.toml` or defaulted to the reference backend per [FRM-BO-13](../02%20components/framework/11-build-orchestration.md#frm-bo-13--backend-selection-is-declared-per-capability-and-defaults-to-the-reference-backend) — is not installed under `~/.cln/bridges/` at a version satisfying the project's constraint. The diagnostic MUST name the capability, the source location that requires it, and the `cln add` invocation that installs a backend.

**Message template:** `"capability '{capability}' is imported but no backend is installed"`

**Example (fails — CAP002):** a handler calls `mail.send(...)` in a project with no `[mail]` block and no mail backend installed. The build fails naming `clean:mail/send` and offering `cln add mail:postmark` (production) and `cln add mail:drop` (development).

**Suggested fix:** applicability `MachineApplicable` when exactly one backend is installed for the capability and only the `clean.toml` selection is missing — the suggestion inserts the `[<capability>] backend = "..."` block. When nothing is installed, applicability is `HasPlaceholders`: the fix requires running `cln add`, which the diagnostic names rather than performs, because installing a production backend is a decision the developer makes.

### CAP003 — Backend Unknown

**Condition:** The framework MUST emit `CAP003` when `[<capability>] backend` names a backend that does not exist for that capability. The diagnostic MUST list the backends that do exist, so the developer can correct a misspelling without leaving the terminal.

**Message template:** `"unknown backend '{backend}' for capability '{capability}'"`

**Example (fails — CAP003):** `[session] backend = "redes"`. The diagnostic reports the valid set for `session` — `inproc`, `redis`, `sql`, `dynamodb` — with `redis` offered as the nearest match.

**Suggested fix:** applicability `MachineApplicable` when exactly one known backend is within a small edit distance of the given value; otherwise `MaybeIncorrect` with the full list.

---

## Changelog

- 2026-08-20 — [`BLD001`](#bld001--build-limit-exceeded) gains the new `max-nesting-depth` limit ([07 §7.8](./07-build-config.md#78-compile-time-limits), default 256), from the compiler's Milestone 9 (`clean-language-compiler/docs/DISCOVERIES-M9.md` §2, via [work/2026-08-20-structural-nesting-limit.md](../work/2026-08-20-structural-nesting-limit.md)): the counting rule (one uniform depth over expression, statement/block, and type-expression nesting — operator chains included, so no uncounted form can reproduce the stack-overflow abort the limit exists to prevent), the enforcement point (no later than the end of parsing; later passes may assume bounded depth), the deterministic `{actual} = max + 1` observed value, a nesting-specific primary label, and a worked example. Message template and ID unchanged; the compiler had pinned nothing (DIA-01 forbade a local code), so no migration.
- 2026-08-20 — [`RUN003`](#run003--arithmetic-error) upgraded from stub ([RUL-03](#rul-03--a-stub-is-a-spec-bug)) to a full entry, and [`RUN013`](#run013--index-out-of-range)'s empty-collection fill stated, from the compiler's Milestone 8 post-work (`clean-language-compiler/docs/DISCOVERIES-M8.md` §9, via [work/2026-08-20-runtime-error-message-wordings.md](../work/2026-08-20-runtime-error-message-wordings.md)). RUN003: the raise-site list is exhaustive — integer `/` and `%` by zero, integer division overflow, `number.toInteger()` on NaN / out of range, `string.toInteger()`/`string.toNumber()` parse failure — and its condition scopes to *integer* arithmetic: `number` arithmetic is IEEE 754 and never raises (the old stub's unqualified "division by zero" read as covering `1.0 / 0.0`). The compiler's five pinned wordings are **ratified byte-for-byte** as the normative templates (bare-phrase voice — already the registry's majority voice; RUN013's sentence style is the outlier, and restyling would have migrated five pinned fixtures for prose); the `%`-by-zero site (wording shared with `/`) and the no-overflow note on `%` are registered here beyond the compiler's pin list. RUN013: the empty-collection arm fills `{index} = 0, {length} = 0` — ratifying the compiler's pin; one template per code kept.
- 2026-08-19 — [`COM003`](#com003--memory-layout-error) upgraded from stub ([RUL-03](#rul-03--a-stub-is-a-spec-bug)) with its first registered condition, from the compiler's Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, item 4): static data exceeding the fixed data region `[DATA_SECTION_START, HEAP_START)` of [MMD-01](./03-memory-model.md#mmd-01--layout-and-guest-visible-constants) is a user-program rejection owned here, with headline template and program-level-span rule; until now no diagnostic owned the case and the compiler surfaced it as a `COM013` internal error — that interim is superseded.
- 2026-08-18 — `LIB004` gains a compiler-boundary note, from the compiler's Milestone 5 (`clean-language-compiler/docs/DISCOVERIES-M5.md`, item 6): the rule body was written for a component that reads `library.toml`, but the compiler receives only the lowered `library_manifests[]` entry of the request ([CMP-01](./14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)) and holds no path — there, `{path}` is filled with the entry's `name`. Template, condition list, and ID unchanged ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)); the compiler's pinned fixture wording is ratified.
- 2026-08-17 — M4 registry pass, from the compiler's Milestone 4 (`clean-language-compiler/docs/DISCOVERIES-M4.md`; codes registered/withdrawn in [09](./09-error-codes.md) the same day). **New rules:** [`SEM027`](#sem027--lossy-integer-promotion) `LossyIntegerPromotion` (Warning — the [TYP-06](../04%20language/04-type-system.md#typ-06--type-conversion) lossy-promotion warning finally has a code, with a decidable condition: compile-time-evaluable value beyond 2⁵³; item 1), [`SEM028`](#sem028--undefined-field) `UndefinedField` (missing *field* on a class; `SEM022` stays methods-only; item 12), [`FUNC015`](#func015--duplicate-start-block) `DuplicateStartBlock` (FNC-01's "one per file" had no owner; item 4). **Withdrawn:** [`FUNC001`](#func001--function-must-be-defined-before-use-withdrawn) — its define-before-use condition contradicted chapter 09 (forward references and mutual recursion are legal) and its real case is `SEM019`'s (item 18); [`CLASS007`](#class007--contract-block-out-of-position-withdrawn) — every case in its condition list is parser-owned (`SYN005`) or `CLASS005`'s, leaving no reachable trigger (item 11). **Stub upgrades** (per [RUL-03](#rul-03--a-stub-is-a-spec-bug), wording adopted from the compiler's DIA-06-pinned local adoptions, item 2): `SEM004` (per-operator template plus non-iterable-source and no-text-form context wordings), `SEM009` (unknown type, repeated `?`, host-only integer widths, invalid TYP-05 chains), `IDX001`–`IDX003`. **Ratified adoptions:** `SEM023` widened to own non-boolean `before:`/`after:` contract lines, with `CLASS006` keeping `always:` (item 10); `FUNC002`/`FUNC011` reworded to the `required..=total` arity range [FNC-04](../04%20language/09-functions.md#fnc-04--default-parameter-values) rule 4 implies (item 18); `SEM024`'s "compile-time evaluable" defined conservatively (item 19); `CLASS005` given its full entry with the ownership boundary against the parser. Templates authored here (SEM009's four, CLASS005, CLASS006, SEM004's two context wordings) are normative — where the compiler's pinned fixture wording differs, the fixture conforms to this file.
- 2026-08-15 — `LIB017`'s message template corrected to placeholder form: `"Folder scope '{folder}' maps to library '{lib}' which is not a declared dependency"`. The entry carried the literal values of its own example (`app/ui`, `clean.ui.v2`) where every sibling rule — compare [`LIB018`](#lib018--folder-scope-ambiguous) — declares `{placeholder}` slots. Condition, example, and ID unchanged ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)). Discovered while implementing the diagnostics-code registry in the compiler's Milestone 2 (`clean-language-compiler/docs/DISCOVERIES-M2.md`, item 1).
- 2026-08-10 — New section **§17 Capability Wiring Rules (CAP)** with rule bodies for `CAP001` (`NoBackendAvailable`), `CAP002` (`BackendNotInstalled`), and `CAP003` (`BackendUnknown`), each in the [RUL-01](#rul-01--mandatory-entry-format) entry format, backing the range registered in [09 §3.18](./09-error-codes.md) under [ADR-0032](../01%20governance/decisions/0032-capability-wiring-generated-host-toml.md). The boundary with [`COM014`](#com014--world-mismatch) is stated in the section preamble: a missing *interface* is COM014, a missing *backend behind a provided interface* is CAP. §1 range list updated (CAP001–CAP003).

- 2026-08-02 — [ADR-0010](../01%20governance/decisions/0010-implementation-defined-parser-decisions.md) closed here rather than in a new document. `RUN007`, `RUN009` and `RUN010` had delegated their accept/reject boundary to an implementation-defined decisions file that does not exist; the boundary **is** each rule's condition, which [RUL-01](#rul-01--mandatory-entry-format) already requires these entries to carry. All three are now written in full and in the [RUL-01](#rul-01--mandatory-entry-format) format, and the two open cases are settled: `-0` is accepted, duplicate object keys are rejected.
- 2026-08-02 — Rule body for `RUN019` (`ReadOfCancelledTask`) added to §13 in the [RUL-01](#rul-01--mandatory-entry-format) entry format, under [ADR-0012](../01%20governance/decisions/0012-async-cancellation-and-failure.md).
- 2026-08-02 — Rule body for `STATE006` (`StateRuleViolated`) added to §8 in the [RUL-01](#rul-01--mandatory-entry-format) entry format, under [ADR-0011](../01%20governance/decisions/0011-state-rules-runtime-semantics.md).
- 2026-08-02 — `BLD001`'s condition cited [`LIB009`](#lib009--compiletime-budget-exceeded-withdrawn) as the code enforcing per-handler budgets. `LIB009` was withdrawn as a duplicate of `BLOCK005` and its number is never reused, so the sentence pointed a reader at a code that no longer exists — a dangling reference, not just a dangling anchor. It now cites `BLOCK005`, whose rule body lives in [21 §21.6](../04%20language/21-block-handlers.md#216-diagnostics-from-compile-time-functions).
- 2026-08-02 — Link repair: the `#companion-access` fragment used to reach [CLS-05](../04%20language/14-classes-and-objects.md#cls-05--companion-access) resolved to nothing — that section gained a rule ID and the anchor was never updated. It was broken the same way in ten documents. No normative change.
- 2026-08-02 — Rule body for `RUN018` (`UnhandledError`) added to §13 in the [RUL-01](#rul-01--mandatory-entry-format) entry format, under [ADR-0016](../01%20governance/decisions/0016-error-value-or-signal.md). §1 range list updated (RUN001–RUN018).
- 2026-08-02 — Rule body for `SEM026` (`LiteralOutOfRange`) added to §3 in the [RUL-01](#rul-01--mandatory-entry-format) entry format, under [ADR-0019](../01%20governance/decisions/0019-precision-modifiers.md). Its condition carries the post-fold requirement that [ADR-0014](../01%20governance/decisions/0014-source-text-encoding-and-identifier-charset.md) settled, so the asymmetry of a signed range does not make the documented minimum unwritable. §1 range list updated (SEM001–SEM026).
- 2026-08-02 — Rule bodies for `RUN016` (`MatrixShapeMismatch`) and `RUN017` (`MatrixSingular`) added to §13 in the [RUL-01](#rul-01--mandatory-entry-format) entry format, backing the Matrix module under [ADR-0018](../01%20governance/decisions/0018-matrix-operator-overloading.md). §1 range list updated (RUN001–RUN017).
- 2026-08-02 — Rule body for `SEM025` (`ControlFlowOutsideLoop`) added to §3 in the [RUL-01](#rul-01--mandatory-entry-format) entry format, backing [FLW-03](../04%20language/12-control-flow.md#flw-03--break-and-continue) under [ADR-0017](../01%20governance/decisions/0017-break-and-continue.md). §1 range list updated (SEM001–SEM025).
- 2026-08-02 — Rule body for `CFG005` (`FileEncodingInvalid`) added to §15 in the [RUL-01](#rul-01--mandatory-entry-format) entry format, backing [TXT-01](./17-text-encoding.md#txt-01--every-ecosystem-text-file-is-utf-8) and [TXT-02](./17-text-encoding.md#txt-02--the-reader-validates-at-the-moment-it-reads) in the new [17 — Text Files](./17-text-encoding.md). §1 range list updated (CFG001–CFG005). Matrix re-verified: 152 codes ↔ 146 rule bodies here plus the 6 `BLOCK` bodies in [21](../04%20language/21-block-handlers.md).
- 2026-08-01 — Rule body for `SCOPE006` written in the [RUL-01](#rul-01--mandatory-entry-format) entry format. Matrix re-verified: 151 codes ↔ 145 rule bodies plus the 6 `BLOCK` bodies in [21](../04%20language/21-block-handlers.md).
- 2026-08-01 — Sixteen rule bodies written for the codes registered in [09](./09-error-codes.md) on this date, each in the [RUL-01](#rul-01--mandatory-entry-format) entry format. Four rules withdrawn as duplicates of `BLOCK` codes (`LIB005`, `LIB007`, `LIB008`, `LIB009`); `LIB007`'s `readSpecFile` filesystem exception is **not** carried over to `BLOCK006` — the primitive is defined in no chapter of the specification. `LIB018`'s boundary with `BLOCK001` stated (a colliding name is `BLOCK001`; an ambiguous namespace mapping is `LIB018`). `SYN007` no longer restates the section order and the framework-block placement — [08 — File Structure](../04%20language/08-file-structure.md) is their home. `IDX001`/`IDX003` rewritten with their corrected names and, for `IDX003`, the generic key type.
- 2026-08-01 — [ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md) applied (Accepted 2026-08-01): `SEM010` rewritten against the standard-library pattern vocabulary — the argument to `string.matches()` MUST be one of the pattern constants declared in [15 — Standard Library §String Patterns](../04%20language/15-standard-library.md#string-patterns) (identifier check); the bare-name catalog (`"email"`, …) and pattern packs are retired; the "(pattern vocabulary: pending)" marker removed; passes/fails examples updated to the constant form.
- 2026-08-01 — Technical-debt closure pass (lote X, approved 2026-08-01): `IMPORT005` **withdrawn** — its full entry (resolve-pass detection, message template, example) absorbed by [`IMPORT001`](#import001--circular-dependency), which was upgraded from a stub to the RUL-01 entry format; §IMPORT005 reduced to the withdrawal note (number never reused, DOC-13), closing the IMPORT001↔IMPORT005 boundary conflict. CFG003's tier-below-library-minimum boundary note resolved: the case is a [`CFG002`](#cfg002--manifest-constraint-violation) constraint **error** ([05 §5.1](./05-memory-policy.md#51-memory-tiers) wins; 07 §7.10 corrected), and CFG002's condition now lists it as a registered constraint. Three new full rule entries for the codes registered in 09 on this date: [`CFG004`](#cfg004--lockfile-mismatch) Lockfile Mismatch (CI build where `clean.toml` and `.cln/lock.toml` disagree), [`MEM003`](#mem003--arena-imbalance) Arena Imbalance (unbalanced `arena-pop` / pop past a foreign save-point, [03 MMD-03](./03-memory-model.md#mmd-03--arena-discipline-every-push-balanced-by-exactly-one-pop)), [`RUN012`](#run012--time-budget-exceeded) Time Budget Exceeded (wall-clock/epoch budget exhaustion, [03 §3.5](./03-memory-model.md#35-host-backing--observable-contract) + [07 `[runtime]`](./07-build-config.md#72-schema--top-level)). §1 coverage list updated (RUN→012, MEM→003, CFG→004; IMPORT005 marked withdrawn).
- 2026-08-01 — Fase 4 (lote 1): full rule entries added for every code formally registered in 09 on this date — `IMPORT005` (§9, with a logged boundary conflict against IMPORT001), `LIB019` (§10.4), `COM009`–`COM017` (§11, covering bridge resolution/linking, the world import check, the codegen invariant, and the Moment 1/2/3 checks of 16 — with the framework/host-emitted rules marked as such), `BLD001` (§12, replacing the placeholder), `RUN011` (§13, `after`/`always` runtime violations; boundary with RUN005 and `--strip-checks` semantics), and new sections §14 MEM (`MEM001`, `MEM002`), §15 CFG (`CFG001`–`CFG003`, with a logged 05-vs-07 conflict on the tier-below-library-minimum case), §16 RQD (`RQD001`, `RQD002`). `STATE003` heading renamed to the ratified registry name `CircularStateDependency` (old name withdrawn per DOC-13); SEM018's cross-link updated. Traceability compliance pass: claimed rule prefix `RUL-`; minted `RUL-01` (mandatory entry format), `RUL-02` (1:1 with the registry), `RUL-03` (stub = spec bug) with concern citations; catalog sections §2–§16 marked *Normative.*, §1 and the preamble marked *Informative.*; §1 coverage list updated to the new totals (125 rules, 1:1 with 09's 125 non-BLOCK codes).
- 2026-08-01 — Fase 3 remediation per the approved conflict log (0.2, 0.3, 0.5, P16.7, P16.9, P16.10): §1 reduced to a citation of 09 §1 (DOC-14) and corrected to include SYN100/SYN101 and the MEM/BLD/BLOCK/CFG ranges; LIB012/LIB013 rule bodies rewritten to the LBS-02 `host interface` / `host function` grammar (no `from "..."` clause, no explicit `result<>`, no `_underscore` names); `wasm-browser` → `wasm32-browser`; config keys normalized to the schema of 07 (`[compile.limits]`, `[compile.env]`, `[security]` with 07 cited as home); `capability Persist:` → `can Persist:` (real syntax, 04 language/14); SEM010 retitled "Invalid Match Pattern" (registry name) and its pattern-name catalog replaced by a pending marker ([ADR-0009](../01%20governance/decisions/0009-string-pattern-vocabulary.md), Draft); SEM018↔STATE003 boundary: STATE003 narrowed to circular-dependency only, duplicated type-mismatch example removed, reciprocal boundary notes added; boundary notes added to SEM002 (vs SCOPE001), SEM003 (vs SCOPE002/IMPORT004), SEM019 (vs FUNC001); STATE001–STATE005 now cite [20 — State Management](../04%20language/20-state-management.md) as syntax home for `guard <expr> else` and `state: rules:`; COM006 and LIB014 relocated into numeric order (§10 restructured into 10.5 Resource Limits / 10.6 Capabilities / 10.7 Folder Scope); "lenient section ordering" exception removed from SYN007 (mode with no flag and no observable).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler and runtime maintainers implementing diagnostics; anyone amending or adding a rule
- **Rule prefix:** `RUL-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [09 — Error Codes](./09-error-codes.md), [13 — Diagnostic Format](./13-diagnostic-format.md), [06 — Error Reporting](./06-error-reporting.md), [21 — Block Handlers](../04%20language/21-block-handlers.md)
