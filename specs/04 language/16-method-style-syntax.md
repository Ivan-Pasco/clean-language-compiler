# 16. Method-Style Syntax

Clean has three ways to call an operation: `value.method()` when the operation acts on a value, `module.function(a, b)` when it produces something new from peers, and operators (`+`, `==`, `and`) for arithmetic, comparison, and logic. This chapter fixes which style applies to which operation, states the "one name per operation" rule that bans aliases, and points to the standard-library catalog where each name is looked up.

Clean Language follows the "one way to do things" principle. Every operation has exactly one name and one preferred calling style.

### Method Style — The Primary Pattern

When an operation acts on a specific value, use method-style syntax. The value comes first, followed by a dot and the operation name:

```clean
// String operations
string text = "Hello World"
integer length = text.length()
string upper = text.toUpperCase()
string lower = text.toLowerCase()
string trimmed = text.trim()
boolean found = text.contains("World")
list<string> words = text.split(" ")
string cleaned = text.replace("Hello", "Hi")

// List operations
list<integer> numbers = [1, 2, 3]
integer count = numbers.length()
boolean empty = numbers.isEmpty()
numbers.add(4)
numbers.remove(0)
boolean has = numbers.contains(2)
list<integer> sorted = numbers.sort()

// Value conversions
integer age = 25
string ageText = age.toString()
number decimal = age.toNumber()

// Object properties
user.name
user.age
user.toString()
```

**Test:** Can I read the call as "*subject-verb-object*" where the value is the subject? Then it is method style.
- `text.length()` — "text has length"
- `numbers.add(4)` — "numbers add 4"
- `user.toString()` — "user converts to string"

### Namespace Functions — For Utilities Only

Namespace functions are used only when an operation does not belong to a single value — typically utility functions with multiple independent inputs:

```clean
// Math utilities (no single owner)
math.sqrt(16)
math.max(10, 20)
math.absInteger(-5)

// Creating new collections
list.concat(listA, listB)
list.range(1, 10)
list.fill(5, 0)

// Combining values that have no single owner
string.concat("Hello", " World")
list.join(words, ", ")
```

**Test:** Do I have to pick a "subject"? If the operation reads naturally as `module.action(inputs)` without any input being privileged, it is namespace style.
- `math.max(10, 20)` — neither number is the "subject"
- `list.range(1, 10)` — creates a new list from two integers, no existing list to be the subject
- `list.concat(a, b)` — combines two peers into a third

### CALL-01 — Method style, namespace style, and one name per operation

- If the operation acts **on a value** → use method style: `value.operation()`
- If the operation **creates something new** from multiple inputs → use namespace: `module.operation(a, b)`
- Every operation has **one name** — no aliases, no shortcuts, no alternate forms

### Why This Rule Exists

**One name, one call site.** Without this rule, a language accumulates duplicates: `concat(a, b)`, `a.concat(b)`, `String.concat(a, b)`, `a + b`. Clean Language picks one form for each operation and forbids the others. This makes code:

- **Uniform to read** — you know how to spell any operation without checking docs.
- **Uniform to search** — grepping for `.length()` finds every length check in the codebase.
- **Uniform for tooling** — the LSP has one signature to autocomplete, not three.

**Exponentiation is `^`, not `math.pow`.** Basic arithmetic uses operators (`+`, `-`, `*`, `/`, `^`, `%`); advanced math uses `math.` functions; there is no `math.add`, `math.multiply`, or `math.pow`. See [2 — Language Design Rules](./02-language-design-rules.md) Rule 8.

### Quick Reference

| The operation… | Style | Shape |
|----------------|-------|-------|
| acts on one value, which is the subject | method | `value.operation(args)` |
| produces something new from several peers, none of them the subject | namespace | `module.operation(a, b)` |
| is basic arithmetic, comparison, or logic | operator | `a + b`, `a == b`, `a and b` |

This chapter fixes *which style* an operation uses. It does not catalogue the operations themselves — the surface of each module is specified once in [15 — Standard Library](./15-standard-library.md), which is where a name is looked up.

## Changelog

- 2026-08-01 — Fase 3/4 (L24): the operation catalogue retired — its home is [15](./15-standard-library.md) — which also removed the chapter's self-contradiction: it forbade aliases while listing `list.join` and `string.join` for one operation. The dead `foundation/docs/…` path replaced by a link to [2 — Language Design Rules](./02-language-design-rules.md). The rule now has one home here, and [LDR-08](./02-language-design-rules.md) cites it. Rule `CALL-01` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users choosing between `value.op()` and `module.op(a, b)`; library authors naming operations
- **Rule prefix:** `CALL-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Language Design Rules](./02-language-design-rules.md) (LDR-08), [Standard Library](./15-standard-library.md) (the catalogue this rule governs the shape of)
