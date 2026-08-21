# 07. Statements

A statement changes the state of the program without producing a value: an assignment, a `return`, a call whose result is discarded. This chapter defines the shape of those statements, states that assignment is a statement (never an expression), and covers the language-level parts of `print` and `input` — the standard-library surface is homed in chapter 15.

### STM-01 — Declarations are type-first

```clean
// Type-first variable declarations
integer x = 10
number y = 3.14
string z
boolean flag = true
```

### STM-02 — Assignment is a statement, never an expression

```clean
x = 42              // Simple assignment
arr[0] = value      // List element assignment
obj.property = val  // Property assignment
```

### Console Output and Input

`print`, `print:` and `input` are the console surface. Their catalogue — every call, every argument, every failure — lives in [15 — Standard Library §Console Module](./15-standard-library.md), which is the single home of every standard-library surface.

Two things about them belong to the language rather than the library, and stay here:

- **`print:` is a block, not an apply-block.** It writes each indented expression on its own line. A `print:` block whose body is empty, or that contains a statement rather than an expression, is [`SYN008`](../03%20platform/09-error-codes.md#31-syntax-codes-syn). See [5 — Apply-Blocks](./05-apply-blocks.md) for why it is a separate construct.

  ```clean
  print:
  	"User: {username}"
  	"Score: {score}"
  ```

- **`print` is a hard keyword** ([3 — Lexical Structure](./03-lexical-structure.md)), which is what allows the block form above to be recognised.

### STM-03 — `return` and its forms

```clean
return              // Return void
return value        // Return a value
return expression   // Return expression result
```

## Changelog

- 2026-08-17 — Erratum: the `print:` example wrote `"Score: " + score` with `score` reading as a number, but `+` is defined only on two strings, on `integer`/`number`, and on matrices ([6 §Operators on built-in types](./06-expressions.md#operators-on-built-in-types)), and `integer` → `number` is the only implicit conversion ([TYP-06](./04-type-system.md#typ-06--type-conversion)) — as written, the line is [`SEM004`](../03%20platform/09-error-codes.md#32-semantic-codes-sem). Both lines now use interpolation (`"Score: {score}"`), the supported spelling. Found by the compiler's Milestone 4 (`clean-language-compiler/docs/DISCOVERIES-M4.md`, item 3).
- 2026-08-01 — Fase 3/4 (L13, L16): **`print` has one form** — `print(x)`, with `newline: false` for the no-newline case. The bare `print "x"` form and the `+` suffix are gone: the suffix collided with the addition operator, and the parenthesis-free form contradicted [LDR-02](./02-language-design-rules.md). 47 call sites corrected repo-wide. The console surface (160 of 197 lines) moved to [15 §Console Module](./15-standard-library.md); what stays here is what belongs to the language — the `print:` block form and its `SYN008` diagnostic. Rules `STM-01`..`STM-03` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users writing everyday code; anyone learning the boundary between statements and expressions
- **Rule prefix:** `STM-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Expressions](./06-expressions.md), [Standard Library — Console Module](./15-standard-library.md), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md)
