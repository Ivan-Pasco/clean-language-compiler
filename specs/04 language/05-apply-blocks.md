# 05. Apply-Blocks

An apply-block lets a developer write `identifier:` and then list values indented beneath it, and the compiler treats each indented value as a separate call to that identifier. It is the syntax behind `items.add: a b c`, behind grouped variable declarations like `integer: x = 0 y = 0`, and behind `constant:` blocks. Apply-blocks work with any single-argument function or method; `print:` looks similar but is a distinct construct with its own rule.

### APB-01 — An apply-block applies its header to each indented item

Apply-blocks work with any function or method that takes a single argument. Note that `print:` — while it looks like an apply-block — is a **separate syntactic construct** with its own semantic rule (see [Semantic Rules SYN008](../03%20platform/10-semantic-rules.md#syn008--invalid-print-block) and [7 — Statements](./07-statements.md), which is where the console surface is currently specified). A `print:` block prints each indented value one per line; it is not a general apply-block target. Example:

```clean
items.add:
	item1
	item2
	item3
// Equivalent to: items.add(item1), items.add(item2), items.add(item3)
```

For printing multiple values, use individual `print()` calls:

```clean
print("First line")
print(variable_name)
print(result.toString())
```

### Variable Declarations
```clean
integer:
	count = 0
	maxSize = 100
	currentIndex = -1
// Equivalent to: integer count = 0, integer maxSize = 100, integer currentIndex = -1

string:
	name = "Alice"
	version = "1.0"
// Equivalent to: string name = "Alice", string version = "1.0"
```

### Constants
```clean
constant:
	integer MAX_SIZE = 100
	number PI = 3.14159
	string VERSION = "1.0.0"
```

## Changelog

- 2026-08-01 — Fase 3/4: the dangling "Standard Library — Console I/O" reference repointed ([15](./15-standard-library.md) has no such section; the console surface is now [15 §Console Module](./15-standard-library.md)), and the `SYN008` citation given its anchor. Rule `APB-01` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users learning grouped declarations and single-argument application
- **Rule prefix:** `APB-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Statements](./07-statements.md) (`print:` block form), [Standard Library — Console Module](./15-standard-library.md), [Platform 10 — Semantic Rules](../03%20platform/10-semantic-rules.md) (`SYN008`)
