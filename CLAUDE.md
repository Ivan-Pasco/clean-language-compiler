# clean-language-compiler

The Clean compiler: one self-contained request document in, one WebAssembly
Component Model component out. This is the reference implementation of the
compiler specified by the Clean Language foundation; it is spec-driven — the
spec decides, the code follows.

## Where the truth lives

All paths relative to the sibling checkout `../clean-language-foundation/`:

- `03 platform/14-compiler-architecture.md` — the contract (`CMP-01..06`): request schema, outputs, the 10-pass pipeline, determinism, build manifest, v1 API operations.
- `02 components/compiler/01-specification.md` — component boundary (`CCMP-`): what this component owns and the 20 things it refuses to do.
- `04 language/` — 21 Accepted chapters; `04 language/grammar/*.ebnf.md` is the **source of truth for syntax** (DOC-15) — parse from the EBNF, never from prose.
- `03 platform/09-error-codes.md` + `10-semantic-rules.md` — every code, with the **literal message template** the compiler must emit. Copy templates verbatim; never redact.
- `03 platform/13-diagnostic-format.md` — the `Diagnostic` value, NDJSON serialization, CLI rendering, DIA-06 fixture discipline.
- `01 governance/decisions/` — ADRs. 0006 (reference stack), 0004 (block-handler sandbox), 0033 (`target_world`).
- `work/2026-08-11-compiler-component-model-emission.md` — the Milestone 1 brief this repo is executing. Record discoveries in its Discoveries section.

Local decisions that deviate from or refine an ADR live in `docs/adr/`.

## Invariants that outrank convenience

1. **CMP-01** — everything comes from the request document. No filesystem discovery, no network, no environment beyond `SOURCE_DATE_EPOCH`. If something is missing from the request, that is the caller's defect, not a reason to go looking.
2. **CMP-02** — byte-identical request ⇒ byte-identical outputs. No hash-order iteration in emitting paths (`IndexMap`/`BTreeMap` or explicit sort), no wall time in outputs, no absolute paths, reduce parallel results by `sources[]` index.
3. **CMP-05** — failure writes `diagnostics.json` and exit 1; never a partial `component.wasm`; never a write outside the caller's output directory.
4. **DIA-01** — every diagnostic carries a registered code. No stringly-typed errors. Internal invariant breaches are `COM013`, presented as a compiler bug, never a user error (CMP-04).
5. **Bug found = report + stop that thread** (CT-H-09). No fallback paths, no disabled tests, no markers promising later fixes — the workaround-detector hook enforces this mechanically.

## Layout

Four crates: `crates/clean-compiler-types` (stable value types: spans, diagnostics, request, manifest), `crates/clean-compiler` (the pipeline behind `compile()`; one module per pass, one IR owned by exactly one module), `crates/clean-compiler-bin` (thin process adapter; the only place clap/toml exist) — the ADR-0006 trio — plus `crates/clean-language-server` (the LSP surface over the same pipeline, CCMP-25/26; local ADR 0006). The language server owns no language knowledge: diagnostics come from `check`, hover/definition from a `check_with` observer, and the contract test `tests/parity*.rs` keeps LSP diagnostics ≡ `cln check` over the DIA-06 corpus.

The binary is **not** a user-facing command (CCMP-04): every developer verb belongs to `cln` (Clean Manager). Do not document `clean-compiler` invocations as UX.

## Working here

- `cargo test --workspace` — the suite; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` gate CI.
- Test fixtures: request JSONs under `tests/fixtures/requests/`, WIT under `tests/fixtures/wit/` (vendored `host.wit` from clean-server — refresh deliberately, never casually, and update the recorded sha256), `.cln` sources under `tests/cln/`.
- Snapshots via `insta` under `tests/snapshots/<layer>/`. A surprising snapshot diff is a design question, not a nuisance — never accept blindly.
- Diagnostics are gated 1:1 (DIA-06): every compiler-emittable code has a byte-exact triple under `tests/cln/diagnostics/` (`<CODE>.cln` + `.stdout.txt` + `.json`) or a line in `unimplemented.txt`, which only ever shrinks. Regenerate snapshots deliberately with `UPDATE_DIAG_FIXTURES=1 cargo test --test diagnostics_fixtures`; the registry↔spec leg (`registry_spec.rs`) runs when `../clean-language-foundation` is present.
- Sibling checkouts `../clean-language-foundation`, `../clean-server`, `../clean-host-core`, `../clean-language-compiler-old` are **read-only** from this repo's sessions (hook-enforced). Spec gaps become task briefs in foundation `work/`, written from a foundation session.
- The retired compiler (`../clean-language-compiler-old`) is archaeology: its `KNOWLEDGE.md` lists real traps (heap-pointer init order, string comparison inversion, control-flow lowering bugs); read it before writing the corresponding subsystem, but never port code or unspecified behaviour.
