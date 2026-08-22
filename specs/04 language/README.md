# 04 — Clean Language

This folder is the specification of the Clean Language itself — its tokens, types, expressions, statements, and the constructs a program is built from. The runtime contract every host must satisfy is [03 platform](../03%20platform/README.md); the things that ship are [02 components](../02%20components/README.md). Read the chapters below in order for the full language manual, or jump to the one topic you need.

## Contents

| Chapter | Purpose |
|---------|---------|
| [00 — Scope and Conformance](./00-scope-and-conformance.md) | What the spec covers, the normative vocabulary, and what "conforming implementation" means |
| [01 — Overview](./01-overview.md) | What Clean Language is and its design goals |
| [02 — Language Design Rules](./02-language-design-rules.md) | The "one way to do things" rules |
| [03 — Lexical Structure](./03-lexical-structure.md) | Tokens, keywords, whitespace, literals |
| [04 — Type System](./04-type-system.md) | Primitive and composite types, optionals, conversions |
| [05 — Apply-Blocks](./05-apply-blocks.md) | The `identifier:` block that applies to each indented item |
| [06 — Expressions](./06-expressions.md) | Precedence, arithmetic, comparison, logical, none-handling |
| [07 — Statements](./07-statements.md) | Declarations, assignment, `return` |
| [08 — File Structure](./08-file-structure.md) | The top-level sections of a `.cln` file and their order |
| [09 — Functions](./09-functions.md) | Declaration, parameters, return values, `start:` |
| [10 — Contracts](./10-contracts.md) | `before`, `after`, `always` |
| [11 — Testing](./11-testing.md) | The `tests:` block |
| [12 — Control Flow](./12-control-flow.md) | `if`, `iterate`, `while` |
| [13 — Error Handling](./13-error-handling.md) | `onError` and error propagation |
| [14 — Classes and Objects](./14-classes-and-objects.md) | Class syntax, inheritance, capabilities, companion access |
| [15 — Standard Library](./15-standard-library.md) | The built-in modules and their surfaces |
| [16 — Method-Style Syntax](./16-method-style-syntax.md) | When to use method style and when to use a namespace |
| [17 — Modules and Imports](./17-modules-and-imports.md) | File-level imports and module visibility |
| [18 — Asynchronous Programming](./18-async.md) | `start`, `later`, `background` |
| [19 — AI Integration](./19-ai-integration.md) | The `spec`, `intent` and `source:` metadata statements |
| [20 — State Management](./20-state-management.md) | Declaration, mutation, observation, computed state |
| [21 — Block Handlers](./21-block-handlers.md) | How libraries define new DSL blocks |

## Related contracts

The language does not stand alone. The documents most often needed alongside these chapters:

- [03 platform / 09 — Error Codes](../03%20platform/09-error-codes.md) — the single registry of every diagnostic code this specification cites.
- [03 platform / 10 — Semantic Rules](../03%20platform/10-semantic-rules.md) — one rule body per code: message template, examples, suggested fix.
- [02 components / framework 09 — Libraries Specification](../02%20components/framework/09-libraries-specification.md) — how libraries extend the language with blocks, capabilities and host functions.
- [01 governance / 06 — Glossary](../01%20governance/06-glossary.md) — the controlled vocabulary these chapters must use.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Language users, compiler and tooling maintainers, spec editors
- **References:** [the repository index](../README.md), [01 governance / 00 — Documentation Principles](../01%20governance/00-documentation-principles.md)
