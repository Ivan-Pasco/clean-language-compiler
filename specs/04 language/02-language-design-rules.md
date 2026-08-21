# 02. Language Design Rules

Clean Language is opinionated about a small number of things and lets everything else follow from them. The rules in this chapter are those opinions in one place: where functions live, when parentheses are required, what the `any` escape hatch is for, how namespaces are named, and the "one way to do things" principle that keeps the surface small. Every other language chapter presumes these; a program that violates one violates a rule the compiler enforces at parse or type-check time.

### LDR-01 — Functions live in a `functions:` block

- No standalone `function name(...)` at top level
- Use `functions:` for top-level and class functions
- Entry point uses `start:` block (not a function)
```clean
// ❌ Invalid
function myFunc()
	return 42

// ✅ Valid
functions:
	integer myFunc()
		return 42
```

### LDR-02 — Calls carry parentheses

- ✅ `x.toString()`
- ❌ `x.toString`
```clean
value = 42
text = value.toString()  // ✅ Correct
```

### LDR-03 — `any` is the escape hatch of last resort

- `any` tells the compiler to skip type checking for that value; the developer takes responsibility for it being right.
- Use `any` only when the type genuinely cannot be known at compile time — a library return, a parsed JSON document, external data.
- For a collection whose element type *is* known, use it: `list<integer>`, `list<string>`. Reach for `list<any>` only when the collection is genuinely heterogeneous.

The type itself is specified in [TYP-02](./04-type-system.md#typ-02--composite-and-generic-types); this rule governs when to reach for it.
```clean
functions:
	any identity(any value)
		return value
```

### LDR-04 — Class methods live in a `functions:` block

- All class methods go inside a `functions:` block
```clean
class MyClass
	integer value
	
	functions:
		void setValue(integer newValue)
			value = newValue
```

### LDR-05 — Namespace names are lowercase

- Use `math.sqrt()`, `string.concat()`, `list.concat()` — not `Math.sqrt()`, `String.concat()`
- Uppercase namespace names are not valid in Clean Language

### LDR-06 — Generic containers are built in; user generics are not

- ✅ `list<integer>`, `matrix<number>`, `pairs<string, any>` — the built-in generic containers take a type parameter in angle brackets.
- ❌ You cannot declare your own generic classes or functions. Angle brackets in user code are reserved for the built-in generic containers; there is no `class MyBox<T>` form.
- Use `any` ([LDR-03](#ldr-03--any-is-the-escape-hatch-of-last-resort)) when you need to hold a value of unknown type.

### LDR-07 — `any` is a compile-time concept *(withdrawn)*

**Withdrawn 2026-08-01.** This rule and [LDR-03](#ldr-03--any-is-the-escape-hatch-of-last-resort) stated the same thing in near-identical words, in the same document — one fact with two homes. LDR-03 is retained. The identifier `LDR-07` is never reused ([DOC-13](../01%20governance/00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)).

### LDR-08 — One way to do things

Every operation has exactly one name and one calling style. Which style applies to which operation, and the reasoning behind the split, is specified in [16 — Method-Style Syntax](./16-method-style-syntax.md), the home of this rule.

## Changelog

- 2026-08-01 — Fase 3/4 (L24): the eight numbered rules became `LDR-01`..`LDR-08` with concern citations; prefix `LDR-` registered. **LDR-07 withdrawn** — it and LDR-03 stated the `any` rule twice in near-identical words in the same file (single-home discipline); LDR-03 is retained and now cites [TYP-02](./04-type-system.md) as the type's home. Rule 8's catalogue moved to [16](./16-method-style-syntax.md), the home of the rule.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users writing everyday code; anyone reviewing why a program's shape is enforced
- **Rule prefix:** `LDR-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Type System](./04-type-system.md) (TYP-02, the home of `any`), [Method-Style Syntax](./16-method-style-syntax.md) (home of LDR-08's catalogue), [Glossary](../01%20governance/06-glossary.md)
