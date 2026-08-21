# 06. Expressions

Expressions are how a Clean program computes a value: `a + b`, `user.name`, `list.contains(x)`, `f() onError 0`. This chapter fixes the precedence and associativity of every operator, the two shapes of the none-handling operators (`default` and postfix `!`), the syntax for expressions that span multiple lines, and what each operator means on each built-in type. It also states that Clean has no user-defined operator overloading: `+` and `*` mean what this chapter says they mean, and no class can give them a new meaning.

### EXP-01 — Operator precedence and associativity

**Operators with higher precedence bind more tightly than those with lower precedence.**

From highest to lowest precedence:

1. **Postfix** — `!` (required non-none assertion), applied to the primary immediately to its left
2. **Primary** — `()`, function calls, method calls, property access
3. **Unary** — `not`, `-` (unary minus)
4. **Exponentiation** — `^` (**right-associative**: `2^3^2` evaluates as `2^(3^2)` = 512, not `(2^3)^2` = 64)
5. **Multiplicative** — `*`, `/`, `%`
6. **Additive** — `+`, `-`
7. **Comparison** — `<`, `>`, `<=`, `>=`
8. **Equality and identity** — `==`, `!=`, `is`, `not` (infix)
9. **Logical AND** — `and`
10. **Logical OR** — `or`
11. **None-Coalescing** — `default`
12. **Assignment** — `=`
13. **Failure fallback** — `onError` (see [13 — Error Handling](./13-error-handling.md))

**Associativity.** Every binary operator is left-associative — `a - b - c` evaluates as `(a - b) - c` — except `^`, which is right-associative. Assignment is a statement, not an expression ([7 — Statements](./07-statements.md)): `a = b = c` is not a valid form, and an assignment never appears inside a larger expression. `onError` binds looser than assignment, so `x = f() onError 0` groups as `x = (f() onError 0)`.

### EXP-02 — A multi-line expression is parenthesized

**Rule**: If an expression spans multiple lines, it must be wrapped in parentheses.

**Syntax**:
```clean
// Single line expressions (no parentheses required)
result = a + b + c
value = functionCall(arg1, arg2)

// Multi-line expressions (parentheses required)
result = (a + b + c +
          d + e + f)

complex = (functionCall(arg1, arg2) +
           anotherFunction(arg3) *
           (nested + expression))

calculation = (matrix1 * matrix2 +
               matrix3.transpose() *
               scalar_value)
```

**What this means:**

- An expression written on one line needs no parentheses.
- An expression continued onto a further line must be enclosed in parentheses; the enclosing pair is what carries it across the line break, so indentation is never what joins the lines.
- The enclosed expression may contain further parenthesized sub-expressions to any depth.
- An unclosed parenthesis at end of file is [`SYN004`](../03%20platform/09-error-codes.md#31-syntax-codes-syn); a closing parenthesis with no opener is [`SYN002`](../03%20platform/09-error-codes.md#31-syntax-codes-syn).

**Examples**:

```clean
// ✅ Valid: Single line, no parentheses needed
total = price + tax + shipping

// ✅ Valid: Multi-line with parentheses
total = (price + tax + 
         shipping + handling)

// ✅ Valid: Complex multi-line expression
result = (calculateBase(width, height) +
          calculateTax(subtotal) +
          (shippingCost * quantity))

// ✅ Valid: Multi-line function call
value = functionCall(
	(arg1 + arg2),
	(arg3 * arg4),
	defaultValue
)

// ❌ Invalid: Multi-line without parentheses
total = price + tax + 
		shipping         // Compilation error

// ❌ Invalid: Unmatched parentheses
result = (a + b + c      // Compilation error: missing closing parenthesis
```

**Benefits**:
- **Clarity**: Explicit parentheses make multi-line expressions unambiguous
- **Consistency**: Clear rules for when parentheses are required vs. optional
- **Readability**: Developers can format complex expressions across multiple lines
- **Error Prevention**: Prevents accidental statement termination in multi-line expressions

### Arithmetic Operators

```clean
a + b       // Addition
a - b       // Subtraction
a * b       // Multiplication
a / b       // Division
a % b       // Modulo
a ^ b       // Exponentiation
```

### Comparison Operators

```clean
a == b      // Equal
a != b      // Not equal
a < b       // Less than
a > b       // Greater than
a <= b      // Less than or equal
a >= b      // Greater than or equal
a is b      // Identity: a and b are the same value
a not b     // Negated identity: a and b are not the same value
```

`is` and its negation `not` sit at level 8, alongside `==` and `!=`, and are left-associative like every other binary operator.

#### `not` in two positions

`not` is the one word in the language that is both a unary and a binary operator:

- `not b` — **unary** logical negation (level 3).
- `a not b` — **binary** negated identity (level 8), exactly equivalent to `not (a is b)`.

Which one applies is decided by **position**, not by lookahead:

- In **operand position** — at the start of an expression, or immediately after another operator — `not` is unary.
- In **operator position** — immediately after a complete operand — `not` is the binary form.

Every combination therefore has one reading:

```clean
not a           // unary: negation of a
a not b         // binary: a is not identical to b
not a not b     // (not a) not b — unary first, then binary
a not not b     // a not (not b) — binary, whose right operand is a negation
```

Because unary `not` binds at level 3 and binary `not` at level 8, `not a not b` groups as `(not a) not b` without any special rule; the position test alone resolves the token.

Writing `not (a is b)` is always available and is clearer when the operands are long expressions. Both forms are the same operation, so this is one operation with one meaning, not two ways to do it: `a not b` is the infix spelling and `not (a is b)` is the same expression written with the unary operator.

### Logical Operators

```clean
a and b     // Logical AND
a or b      // Logical OR
not a       // Logical NOT (unary prefix)
```

### EXP-03 — The none-handling operators `default` and `!`

Clean Language provides two operators for working with potentially none values:

#### Default Operator (`default`)

The `default` operator provides a fallback value when the left operand is `none`. This is also known as none-coalescing.

```clean
value default fallback    // Returns value if not none, otherwise fallback
```

**Important:** The `default` operator only checks for `none`, not for "falsy" values like `0`, `false`, or `""`.

```clean
// None-coalescing with 'default':
none default "x"           // Returns "x" (left is none)
"y" default "x"            // Returns "y" (left is not none)

// 'default' only coalesces none, NOT falsy values:
false default true         // Returns false (false is NOT none)
0 default 10               // Returns 0 (0 is NOT none)
"" default "fallback"      // Returns "" (empty string is NOT none)

// Boolean logic with 'or' remains unchanged:
false or true              // Returns true (traditional boolean OR)
true or false              // Returns true
```

**Use Cases:**
```clean
// Provide default values for optional data
string username = userData.name default "Guest"
integer count = config.maxItems default 100
number price = product.price default 0.0

// Chain multiple defaults
string value = primary default secondary default "final fallback"
```

#### Required Assertion Operator (`!`)
**`!` is a postfix operator that asserts a value is non-none at runtime, and narrows its type at compile time.**

The `!` operator is written immediately after the expression it applies to — it comes after the value, never before:

```clean
value!    // ✅ Correct postfix form
!value    // ❌ Not valid — ! is not a prefix here
```

**Runtime behavior:** If the value is `none` at the point of evaluation, execution halts with a runtime reference error (RUN004). If the value is not none, it is returned unchanged.

**Compile-time behavior:** After `!`, the compiler treats the result as a guaranteed non-none value for subsequent type checking. None checks and `default` branches that follow are not required.

```clean
// maybeNone has type string (optional)
string? maybeNone = getUser()

// After !, the compiler treats the result as string (non-none)
string name = maybeNone!    // Runtime check: halts if none; compile-time: treated as string

// Chaining with method calls
string upper = getText()!.toUpperCase()    // getText() is checked for none before .toUpperCase()

// Use when you're certain a value is not none
integer count = list.find(item)!
```

**When to Use:**
- Use `!` when you are certain a value is not none and want to express that intent explicitly
- Use `default` when you want to provide a fallback value instead of halting
- Prefer `default` for user-facing code; use `!` for internal assertions where none would indicate a programming error

### Operators on built-in types
**The language defines what each operator means for each of its built-in types. There is no user-defined operator overloading.**

An operator's meaning is therefore fixed by this specification and readable without knowing anything beyond the operand types. A class cannot give `+` a meaning, and no declaration form exists for doing so — the mechanism a future version would use is a capability with a self type, which capabilities do not have in v1 ([CLS-03](./14-classes-and-objects.md#cls-03--capabilities-are-contracts-without-bodies): not generic, not composable).

The built-in types that define operators beyond a single numeric meaning:

| Type | Operators | Meaning | Home |
|------|-----------|---------|------|
| `integer`, `number` | `+ - * / ^` | Arithmetic | [15 §Math Module](./15-standard-library.md) |
| `string` | `+` | Concatenation | [15 §String Module](./15-standard-library.md) |
| `matrix<T>` | `*` | Matrix multiplication | [15 §Matrix Module](./15-standard-library.md) |
| `matrix<T>` | `+ -` | Element-wise addition and subtraction | [15 §Matrix Module](./15-standard-library.md) |

```clean
matrix<number> product = a * b       // matrix multiplication
number scaled = x * y                // ordinary multiplication
string greeting = "hola " + name     // concatenation
```

Operations on a built-in type that are *not* operators are functions or methods, and each type's full surface is homed in [15 — Standard Library](./15-standard-library.md) — `A.transpose()`, `A.inverse()` and `A.determinant()` are specified there, not here.

**Why the language defines them rather than exposing a mechanism:** an operator whose meaning any type may redefine makes every expression's meaning depend on the types of its operands, which have to be resolved before the expression can be read at all. That cost is paid by every reader of every expression, and by every agent generating code, in exchange for expressiveness in a small number of types. Fixing the set here keeps `A * B` readable — the notation matrix arithmetic genuinely wants — without making `x * y` a question anywhere else in the language.

### Method Calls and Property Access

```clean
obj.method()            // Method call
obj.property            // Property access
obj.method(arg1, arg2)  // Method with arguments
"string".length()       // Method on literal
myList.get(0)           // Built-in method
```

### Function Calls

**Function arguments are evaluated left-to-right before the function is called.** Side effects in argument expressions (such as function calls that mutate state) occur in left-to-right order.

```clean
functionName()                     // No arguments
functionName(arg1)                 // Single argument
functionName(arg1, arg2, arg3)     // Multiple arguments
```

## Changelog

- 2026-08-02 — §Matrix Operations replaced by **§Operators on built-in types**, closing [ADR-0018](../01%20governance/decisions/0018-matrix-operator-overloading.md). The chapter had described "type-based operator overloading" through a single `matrix` example, which read as an unexplained exception and left it unstated whether user types could do the same. The rule is now general and the exception disappears: the language defines the operators of its built-in types and there is no user-defined overloading. Writing the table surfaced a second orphan — `+` on two `string` values is used in examples across several chapters and was defined in none; it is registered here. The `matrix` method surface moves to its home in [15 §Matrix Module](./15-standard-library.md).
- 2026-08-01 — Fase 5 (zero-debt pass): the parser algorithm ("tracks parentheses depth", "consumes tokens across lines") replaced by the observable rule and its two diagnostics, [`SYN004`](../03%20platform/09-error-codes.md#31-syntax-codes-syn) and [`SYN002`](../03%20platform/09-error-codes.md#31-syntax-codes-syn) (SDD-02).
- 2026-08-01 — Fase 5: the infix `not` operator specified rather than withdrawn (user decision). It appeared once in the whole repository, with no precedence row and no rule resolving it against the unary `not`. It now sits at level 8 beside `is`, and the unary/binary ambiguity is settled by operator-vs-operand position, which gives `not a not b` and `a not not b` one reading each.
- 2026-08-01 — Fase 3/4 (L18): `onError` added to the precedence table as the loosest level, resolving a chapter that legislated on an operator with no precedence row. Associativity restated once and correctly — the chapter had said "all binary operators" in one line and "levels 5–11" in another — and assignment declared a statement, not an expression, so `a = b = c` has a defined answer. `!` and `.` ordering corrected: the chapter required `getText()!.toUpperCase()` to check before the call while ranking `.` tighter. Rules `EXP-01`..`EXP-03` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users writing expressions; anyone reasoning about precedence, `none`, or the failure operator `onError`
- **Rule prefix:** `EXP-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Statements](./07-statements.md) (assignment is a statement), [Error Handling](./13-error-handling.md) (`onError`), [Standard Library](./15-standard-library.md) (per-type operator semantics), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md)
