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

## 3. ADR-0027's tooling references vs. the 2026-08 Rust ecosystem

ADR-0027 (Draft) names enforcement tools that do not exist as specified:

- **MC/DC**: the cited `cargo-mcdc` is unpublished on crates.io, and rustc
  **removed** `-Z coverage-options=mcdc` — current nightly accepts only
  `block | branch | condition` (verified 2026-08-20; the flag errors with
  "incorrect value `mcdc`"). MC/DC on the named modules is therefore
  unimplementable with today's toolchain. Local stand-in (nightly.yml):
  the named modules are gated per-module on the Tier 1 **branch** floor
  (75 %) with `-Z coverage-options=condition` instrumentation — the
  nearest measure the toolchain offers. The ADR should name the
  ecosystem-endorsed equivalent (or park MC/DC until rustc's
  implementation returns).
- **Named-module mapping**: the ADR's Tier 1 MC/DC list ("codegen,
  type-checker, bridge marshalling, memory management") maps onto this
  component as `src/codegen/`, `src/typecheck/`, `src/layout.rs`; bridge
  marshalling is clean-server's Tier 1 surface, not the compiler's.
- **Mutation cadence**: `cargo-mutants` 27.1.0 counts 3 275 mutants over
  this workspace — far beyond the ADR's 30–90 min nightly window in one
  run. nightly.yml runs a rotating 1/8 shard (full set every 8 nights),
  floor 60 % per shard; timeouts count as caught. The ADR's
  two-consecutive-red-nights merge-block rule is noted in the workflow
  and left to the reader of the dashboard until enough CI history exists
  to automate it.
- **cargo-llvm-cov**: 0.6.x cannot find object files under the build-dir
  layout newer cargo emits; both workflows pin 0.9.0.

## 4. §14.9 performance budgets — measured, all within target

`cargo run --release -p clean-compiler --example perf_budget`, 2026-08-20,
M4 Mac (darwin 25.5), suite at a34a6a2. Synthetic projects: small 978 LOC /
4 modules, medium 18 033 LOC / 9 modules, 0 library manifests (none
released yet). Cold = fresh process per run, median of 5; the timed span is
the operation alone. Manifest `timings` stay zero (CMP-02); every clock
lives in the harness.

| Case | Target | Measured |
|---|---|---|
| compile small, debug, cold | < 500 ms | 6.9 ms |
| compile small, release, cold | < 1 500 ms | 4.2 ms |
| compile medium, debug, cold | < 3 000 ms | 38.7 ms |
| compile medium, release, cold | < 10 000 ms | 36.9 ms |
| `check` medium, cold (§14.14.4) | < 300 ms | 30.5 ms |
| `why` medium, cold (§14.14.1) | < 100 ms | ≈ 0.03 ms (10-diagnostic set) |
| watch rebuild medium, warm (§14.14.3) | < 500 ms | 35.9 ms |

debug ≈ release across the board because pass [8] runs no optimizations
yet (the differential suite pins that); the large size class (< 100k LOC)
is deferred until a real project of that size exists to calibrate against.
The §14.9 numbers move to the nightly log once CI runs (informative step,
never red).

## 5. Milestone gate status (2026-08-20)

- **Blocked**: GitHub billing is still down — every job dies with 0 steps
  (checked at 9e96d2e). The two-week green-nightly clock cannot start.
  nightly.yml is committed and `workflow_dispatch`-able; ci.yml coverage is
  blocking. When billing recovers: re-run the HEAD ci run, dispatch
  nightly once by hand, and start the two-week clock from the first green
  scheduled run.
- Local gates green at each M9 stage: fmt + clippy -D warnings + full
  workspace suite; 10 256 fuzz seeds and the three-profile differential
  swept clean; budgets measured above.

## 6. Coverage baseline at M9 activation (ADR-0027 Tier 1)

Measured locally 2026-08-19 (`cargo llvm-cov --workspace --summary-only`,
`CARGO_PROFILE_DEV_DEBUG=0`, macOS aarch64, suite at e71104d):
**87.73 % line, 86.99 % function, 85.92 % region** — above the Tier 1
target floor (80 % line). CI enforcement activates at the Tier 1 target
(80) rather than the local baseline because the CI-measured number differs
structurally (the `registry_spec` leg self-skips without the foundation
checkout); the baseline comment in the workflow records the local
measurement, to be superseded by the first live CI measurement once GitHub
billing recovers.
