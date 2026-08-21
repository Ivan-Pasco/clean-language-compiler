# 01 — Compiler Testing

The compiler is deterministic by design — the same source, the same flags, the same commit yields the same bytes. This chapter is the inventory of test types that hold the compiler to that promise: which layers cover the parser, the type checker, the semantic validator, codegen, diagnostics, and the compiler-as-pure-function API; which corpora and snapshot tools those layers run against; which known bug shapes each layer is expected to catch. Read [00 — Testing Strategy Overview](./00-testing-strategy-overview.md) first for the layer taxonomy and [10 — Test Types Reference](./10-test-types-reference.md) for what a good test at each layer looks like; this chapter binds that shared vocabulary to the compiler's surface.

## 1. Surface being tested

The parts of the compiler this document covers:

- **Parser** — Clean source → AST. Grammar in `foundation/04 language/03-lexical-structure.md`, `06-expressions.md`, `07-statements.md`, `08-file-structure.md`.
- **Type checker** — AST → typed HIR. Rules in `foundation/04 language/04-type-system.md`.
- **Semantic validator** — typed HIR against the rule registry in `foundation/03 platform/10-semantic-rules.md` (1:1 with error codes in `09-error-codes.md`).
- **Codegen** — HIR → WASM component. Layers and worlds in `foundation/03 platform/00-overview.md`, `01-execution-layers.md`, `15-component-model-architecture.md`.
- **Diagnostics** — every error produces a `Diagnostic` value matching `foundation/03 platform/13-diagnostic-format.md`.
- **Compiler-as-pure-function API** — the single JSON request document defined in `foundation/03 platform/14-compiler-architecture.md §14.1`.
- **Block-handler expansion** — `apply` / `html:` / `sql:` / `tests:` etc., per `foundation/04 language/05-apply-blocks.md` and `21-block-handlers.md`.

## 2. Consumers and blast radius

Consumers: `cln` (manager), `clean-server` (loads generated `.wasm`), every framework plugin (compiled with `built_with_compiler = "X.Y.Z"` stamp), Clean Studio, every Clean user app.

Blast radius of a broken release: every downstream project must skip the version. Since `cln install latest` is called at the end of `comita`, a bad compiler tag propagates in minutes.

## 3. Layers in use

| Layer | Tool | Runs on | Notes |
|---|---|---|---|
| L1 Unit | `cargo test` | Every push | Parser combinators, type-inference unit tests, HIR transforms |
| L2 Snapshot | `insta` | Every push | AST, HIR, generated WAT, and — critically — every diagnostic per error code |
| L3 Property | `proptest` | Every push | Type-system laws (subtyping transitive, substitution preserves types), round-trip AST ↔ source for canonicalised inputs |
| L4 Integration | Cargo test + `wasmtime` in-process | Every PR | `.cln` fixture compiles → runs under Wasmtime → asserts output |
| L5 Conformance | Spec fixtures + JSONTestSuite (see [06](./06-stdlib-conformance-testing.md)) | Every PR | Per-error-code fixture in `tests/spec/` |
| L6 E2E | — | — | Skipped; user-facing E2E lives in Studio / user-app strategies |
| L7 Fuzz | `cargo-fuzz` + tree-sitter Clean grammar | Nightly | Parser panic-freedom, type-checker panic-freedom; grammar-based fuzzer preferred over byte-level |
| L7 Differential | Two-backend diff (WAT vs component-model output) | Nightly | Catches codegen drift between backends |
| L8 AI review | Codegen reviewer + Spec reviewer (see [09 §3](./09-ai-review-agent-strategy.md)) | Pre-PR + on PR | High blast-radius component; ensemble worth the cost |

## 4. Golden bugs

Failure classes the compiler is expected to see, and the layer intended to catch each. This is a design-time list; every entry becomes a regression test the day its layer ships. As real fingerprints accumulate on the dashboard, replace the placeholders with the actual fingerprint.

- **Codegen crashes on generic-container iteration** (e.g. `iterate` over `list<string>`). Layer: L2 snapshot per generic-container shape, L4 integration compiling and running each shape.
- **String-literal parser mishandles `{` outside `html:` blocks.** Layer: L2 snapshot per lexer edge-case.
- **`built_with_compiler` stamp mismatch** between plugin `.wasm` and the compiler version in `plugin.toml`. Layer: L4 integration via `comita` STEP 6.
- **Bridge string-semantics divergence** — `"string"` param interpreted differently across the WIT registry, the framework rule, and codegen (`expand_strings`). Layer: L2 snapshot of every generated WIT-adapter signature + L8 Codegen reviewer with spec cross-reference.
- **Block-handler expansion collides with user method names** — e.g. class methods named `delete/exists/list/find/update/count` in files that opt into a plugin whose block handler produces those names. Layer: L1 unit test on the block expander + L8 Plugin-Bridge reviewer.
- **Visibility-default change silently reclassifies user APIs.** Layer: L2 snapshot audit of every visibility-annotated fixture; L4 integration on a corpus of pre-existing public APIs.
- **Diagnostic quality regression** — wrong span, missing suggestion, missing `doc_url`. Layer: L2 snapshot per error code (this is the enforcement point of the "every error code has a snapshot test" guiding principle).
- **Miscompilation** — the emitted `.wasm` produces the wrong runtime output for a program that type-checks. Layer: L4 integration with a diverse fixture corpus; L7 differential when a second backend or previous-version compiler is available.

Each row in the compiler repo's `TESTING.md` §3 must have: fingerprint (once one exists), one-line root cause, which layer would or did catch, current regression-test location.

## 5. Boundary contracts

The compiler *reads*:

- Clean source (`*.cln`) — grammar in language spec.
- `library.toml` / `project.toml` — schema in `foundation/02 components/framework/07-library-authoring.md`.
- Compiler request JSON (§14.1) — schema owned by platform.

The compiler *writes*:

- `.wasm` WASI 0.3 / Component Model 0.3.0 components — WIT worlds in `foundation/03 platform/15-component-model-architecture.md`.
- `Diagnostic` values — schema in `foundation/03 platform/13-diagnostic-format.md`.
- Manifest metadata (`built_with_compiler`, `handles_blocks`, etc.).

**Drift detection:**

- Every WIT world change requires a `wit-bindgen` regeneration test that diffs the generated Rust/host bindings. Failing this must file `report_error`, not just fail CI.
- Every `Diagnostic` schema change requires re-snapshotting all diagnostic fixtures and updating the LSP mapping (see [05](./05-extension-lsp-testing.md)).
- Every error code addition to `09-error-codes.md` requires (a) a matching rule in `10-semantic-rules.md`, (b) at least one snapshot test producing it. CI fails otherwise. This is the enforcement point of the 1:1 rule.

## 6. Fingerprint discipline

- **Compile-time panic** — always file via `capture-compile` (see the `fixer` skill). Never swallow.
- **Miscompilation (wrong runtime output)** — file with a minimum reproducer `.cln`, expected output, actual output, and the wasm dump. Fingerprint prefix owned by compiler.
- **Diagnostic quality regression** (wrong span, missing suggestion) — file, do not just adjust the snapshot.
- **Spec ambiguity** — do NOT file as a compiler bug; escalate via `team-prompt` to the spec owner (see `team-prompt` skill).

## 7. Review-agent config

Pre-PR (`comita` STEP 0.6): **Codegen reviewer** + **Spec reviewer** from [09 §3](./09-ai-review-agent-strategy.md).

RAG index (see [09 §4](./09-ai-review-agent-strategy.md)) MUST include:

- All of `foundation/03 platform/` (chunked per section).
- All of `foundation/04 language/` (chunked per section).
- `09-error-codes.md` and `10-semantic-rules.md` chunked *per code*, so a diff touching `PARSE042` retrieves exactly that entry.
- WIT files for every world.
- `feedback_no_workarounds.md`, `feedback_no_direct_compiler_build.md`, `feedback_clean_runtime_quirks.md`, `project_bridge_string_semantics.md`, `feedback_frame_data_method_collision.md`.

Explicit rejection: no generic-security agent for this component. Clean's user-visible security surface lives in the host and plugins, not the compiler.

## 8. Known gaps

- No cross-engine WASM parity yet (Wasmtime only). Adopt when a second engine becomes a supported target. Track under a compiler fingerprint tagged `engine-parity`.
- No IR-level differential test between compiler versions. Add once a stable IR dump format lands.
- Fuzzer corpus not yet seeded from user-reported crashes. Seed once the dashboard fingerprint→minimum-reproducer flow is automated.

---

Sources referenced: rustc `compiletest` UI tests; Roc's `crates/compiler/test_gen`; `insta` snapshot patterns; `cargo-fuzz` + Gramatron for grammar-based fuzzing; Csmith-style differential compiler testing.

---

## Metadata

- **Status:** Draft (2026-08-04)
- **Audience:** Compiler maintainers and reviewers configuring or reading the compiler's test surface
- **References:** [`README.md`](./README.md), [`00-testing-strategy-overview.md`](./00-testing-strategy-overview.md), [`10-test-types-reference.md`](./10-test-types-reference.md)
