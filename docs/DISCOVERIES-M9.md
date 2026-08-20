# Discoveries — Milestone 9 (Endurecimiento)

Spec findings made while executing M9. Each item is a candidate for a
foundation `work/` brief or erratum, written from a foundation session; the
"local pin" column records what this repo does meanwhile.

## 1. Grammar-notation defects in `04 language/grammar/` (DOC-15 source files)

Found while seeding the M9 grammar fuzzer, which reads the vendored EBNF
(`tests/fixtures/grammar/`, byte-pinned to foundation) with a strict
ISO/IEC 14977 reader per the grammar README's own notation table.

| # | File | Defect | Local pin (fuzzer) |
|---|------|--------|--------------------|
| 1a | `18-async.ebnf.md` (OnErrorTail) | Concatenation by juxtaposition — `":" NEWLINE INDENT StatementSequence DEDENT` without the `,` the README requires | Reader treats juxtaposition as concatenation |
| 1b | `19-ai-integration.ebnf.md` (§3 comment) | Comment prose contains `(0..*)`; its `*)` terminates the comment early under ISO 14977 | Reader tracks same-line prose parens inside comments so `*)` first closes a pending prose `(` |
| 1c | `08-file-structure.ebnf.md:45` | `ConstantBody` referenced, defined nowhere | Pinned as `{ VariableDeclaration, NEWLINE }` (the 07-statements shape the reference implementation parses) |
| 1d | `08-file-structure.ebnf.md:91` | `TestsBody` referenced with "(\* Body in 11-testing.ebnf.md \*)", but 11 defines `TestsBlock` (header included) and no `TestsBody` | Pinned as TestsBlock's interior: `TestDeclaration, NEWLINE, { TestDeclaration, NEWLINE }` |
| 1e | `18-async.ebnf.md:19,61` | `CallExpression` defined only by prose comment ("any expression whose top-level operation is a call") | Pinned as `Identifier, "(", [ ArgumentList ], ")"` |
| 1f | `08-file-structure.ebnf.md:87` vs `20-state-management.ebnf.md:96` | `WatchBlock` defined in **two** files with diverging shapes (08: `watch Identifier :`; 20: `watch WatchTarget :` where WatchTarget admits a parenthesized identifier list) — a production defined in more than one place is a defect per DOC-15 | First definition in filename order (08) wins for generation |
| 1g | `21-block-handlers.ebnf.md:62` (BlockArgType) | References `ExpressionType` and `IdentifierType`, which no grammar file defines | `BlockArgType` and (through it) `CompileTimeFunctionDeclaration` are ungeneratable; pinned in `grammar_loads_and_root_is_generatable` |

Non-defect but generation-relevant: `LibraryBlock`'s body is
`? handler-defined body ?` (08 §LibraryBlock) and
`HostFunctionDeclaration` defers to the framework's host-bridge grammar —
both are deliberately outside the language grammar and stay ungeneratable.

## 2. No nesting limit anywhere: deep expressions abort the process

`compile_limits` (07 §7.x, mirrored in the request schema) bounds handler
time/memory, file size, and import depth — but nothing bounds **expression
nesting**, and no registered diagnostic code covers a structural-depth
limit. The reference implementation's recursive-descent parser (and the
recursive passes behind it) therefore converts deep-but-legal input into a
**stack overflow / process abort** rather than any diagnostic:

- Measured 2026-08-19 (release build, 2 MiB thread stack — the default for
  non-main threads): `check()` survives 700 nested parentheses and aborts
  at 800.
- CMP-05 ("failure writes diagnostics.json and exit 1") and CMP-04
  ("internal failures are COM013") both assume the process can answer; an
  abort satisfies neither, and a library caller (LSP, --serve loop) takes
  the whole process down with it.

DIA-01 forbids inventing a code locally, so this repo does **not** pin a
guard; the fuzzer bounds its own generation depth (64) to stay inside
measured-safe territory. Needs a foundation decision: a `compile_limits`
nesting bound (with default) + a registered code + message template, or an
explicit statement that callers own stack sizing.

## 3. Coverage baseline at M9 activation (ADR-0027 Tier 1)

Measured locally 2026-08-19 (`cargo llvm-cov --workspace --summary-only`,
`CARGO_PROFILE_DEV_DEBUG=0`, macOS aarch64, suite at e71104d):
**87.73 % line, 86.99 % function, 85.92 % region** — above the Tier 1
target floor (80 % line). CI enforcement activates at the Tier 1 target
(80) rather than the local baseline because the CI-measured number differs
structurally (the `registry_spec` leg self-skips without the foundation
checkout); the baseline comment in the workflow records the local
measurement, to be superseded by the first live CI measurement once GitHub
billing recovers.
