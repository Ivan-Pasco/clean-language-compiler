# 08. File Structure

A Clean source file (`.cln`) has a fixed skeleton: `import:` on top if you have any, then optional sections for constants, state, classes, functions, watchers, tests, and finally `start:` at the bottom. This chapter names every section that may appear at the top level, the order they must appear in, and the diagnostics you get when a file is out of order or contains loose code.

### FIL-01 — Top-level sections appear in a fixed order

A Clean Language file (`.cln`) is organized into top-level sections. Each section is optional, but when present, they must appear in this order:

| Order | Section | Purpose |
|-------|---------|---------|
| 1 | `import:` | Bring in code from other modules, and explicitly import a library (see [17 — Modules and Imports](./17-modules-and-imports.md)) |
| 2 | `source:` | Specification provenance (see [19 — AI Integration](./19-ai-integration.md)) |
| 3 | `constant:` | File-level constant declarations (see [5 — Apply-Blocks](./05-apply-blocks.md)) |
| 4 | `state:` | Variables that persist and can be watched |
| 5 | `class` / `can` | Type declarations: classes and the capabilities they claim (see [14 — Classes and Objects](./14-classes-and-objects.md)) |
| 6 | `functions:` / `compiletime function` / `handles block` / `host function` | Callable declarations. Compile-time functions and their `handles block` registrations are specified in [21 — Block Handlers](./21-block-handlers.md); `host function` declarations live in a library's `host_bridge.cln` ([Libraries Specification §8](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations)) |
| 7 | `watch <name>:` | Reactive observers of `state:` variables (see [20 — State Management](./20-state-management.md)) |
| 8 | `tests:` | Test blocks that exercise this file's declarations (see [11 — Testing](./11-testing.md)) |
| 9 | `start:` | Where your program begins running |

`public:` is not a section. It is a wrapper that appears *inside* a section to mark what that section exports (see [17 — Modules and Imports](./17-modules-and-imports.md)).

Framework blocks contributed by libraries in scope (e.g. `endpoints:`, `data:`, `component:`) appear at the top level too; their position in the section order is defined by the library's `library.toml`. When unspecified, they sit between `functions:` and `watch:`.

A library is **not** brought into scope by a section of this file. Folder scope in `clean.toml [folders]` is what puts a library in scope, and an explicit `import` is what overrides it — see [LBS-01](../02%20components/framework/09-libraries-specification.md) and [FRM-01](../02%20components/framework/01-framework-specification.md).

### Why Order Matters

Clean Language enforces section order to keep code consistent and readable. When you open any `.cln` file, you always know where to find things.

If sections are out of order, the compiler tells you exactly what's wrong:

A section in the wrong position is [`SYN007`](../03%20platform/09-error-codes.md#31-syntax-codes-syn) `SectionOutOfOrder`:

```
Error: 'state:' must appear after 'import:' block
Error: 'start:' must be the last section in the file
```

### A Complete Example

Here's a file that uses several sections in the correct order:

```clean
import:
	utils
	mathHelpers

state:
	integer count = 0
	string username = ""

class Point
	integer x
	integer y

functions:
	integer add(integer a, integer b)
		return a + b

tests:
	"add sums two integers": add(2, 3) == 5

start:
	print("Hello, World!")
	integer result = add(5, 3)
	print(result)
```

### FIL-02 — Only the listed forms may appear at the top level

Only the sections listed above can appear at the top level. You can't write loose statements like assignments, function calls, or loops outside of a block. A top-level construct that is not one of them is not "out of order" — it has no place in the file at all, and is reported as [`SYN009`](../03%20platform/09-error-codes.md#31-syntax-codes-syn) `NotATopLevelForm`, distinct from the ordering diagnostic above.

```clean
// ❌ Invalid - can't have loose code at top level
integer x = 5
print("hello")

// ✅ Valid - code goes inside start: block
start:
	integer x = 5
	print("hello")
```

## Changelog

- 2026-08-07 — `screen <Name>:` withdrawn from FIL-01's section table per [ADR-0030](../01%20governance/decisions/0030-withdraw-screen-from-language.md). The section-order table shrinks from 10 to 9 slots. `screen` is not a keyword of any kind; the ui library does not register it as a block name either. It is a free identifier for user code.
- 2026-08-01 — Fase 3/4 (L4, L11): the `libraries:` section **removed** — folder scope in `clean.toml` is the only source of implicit library scope ([LBS-01](../02%20components/framework/09-libraries-specification.md), [FRM-01](../02%20components/framework/01-framework-specification.md)), and no example in the repository ever used the block. The section table completed with the seven top-level forms five other chapters define and this one declared illegal (`constant:`, `can`, `compiletime function`, `handles block`, `host function`); `public:` clarified as a wrapper, not a section. "Not a top-level form" given its own diagnostic, [`SYN009`](../03%20platform/09-error-codes.md#31-syntax-codes-syn) — `SYN007` covers order only. Rules `FIL-01`, `FIL-02` minted.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Clean Language users learning how to organize a `.cln` file; tool authors that parse or navigate one
- **Rule prefix:** `FIL-`
- **Part of:** [Clean Language Specification — Language](./README.md)
- **References:** [Modules and Imports](./17-modules-and-imports.md), [State Management](./20-state-management.md), [Testing](./11-testing.md), [Block Handlers](./21-block-handlers.md), [Platform 09 — Error Codes](../03%20platform/09-error-codes.md)
