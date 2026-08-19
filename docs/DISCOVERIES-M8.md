# Discoveries — Milestone 8 (API operations v1)

Spec gaps and under-specifications found while implementing M8. Each item
either becomes a task brief in foundation `work/` (written from a
foundation session — this repo never writes there) or records a local
adoption that stays in force until foundation resolves it.

## Status

In progress. Stage 1 (`cln why` re-projection, §14.14.1) landed with its
contract suite (`tests/why_operation.rs` at corpus scale, plus the adapter
half in `clean-compiler-bin/tests/cli.rs`). Stage 2 (watch-mode rebuild,
§14.14.3) landed as a contract without an API
(`clean-compiler-bin/tests/watch_rebuild.rs`).

## 1. The `Diagnostic` value carries no pass provenance, but §14.14.1 re-projects it

Platform 14 §14.14.1 says the re-projection reports "which pass rejected
it," and that "all the data required is already in the diagnostic — 
re-projection is a re-presentation, not a re-compilation." But the
Platform 13 §2 `Diagnostic` value has no pass field: nothing in
`diagnostics.json` records which §14.4 pass emitted a given diagnostic.
The two sections cannot both be literally true.

**Local adoption (in force, `why.rs`):** pass attribution is derived from
the code registry, not from the diagnostic instance. `why::passes_for`
maps each code to the §14.4 pass(es) able to emit it — per-code overrides
mirroring the actual emission sites, prefix defaults mirroring the
Platform 09 §3 section preambles. A code emittable from more than one
pass lists every candidate (e.g. `SYN002` → Lex or Parse) rather than
guessing; `COM013` maps to no pass (any pass can break an invariant,
CMP-04). The map is kept total over the compiler-emittable registry by
`why_operation::pass_map_covers_every_emittable_code`.

This stays honest with Platform 13 (no schema change, no extra serialized
field — DIA-04 tolerance is not exercised) at the cost of instance-level
precision. Foundation could instead add an optional pass field to the
§13 value; that is a normative schema decision, so it goes as a brief,
not a local field.

## 2. `--why` location addressing is unspecified

§14.14.1 shows `cln why app/main.cln:42` but does not define the location
syntax the compiler-side operation accepts, nor what "the most recent
build" resolves to (the compiler holds no build history — CMP-01).

**Local adoption (in force, `clean-compiler-bin`):** the operation takes
`<file>:<line>[:<column>]` (1-based, per Platform 13 §2; file is the
request-relative path verbatim) plus an explicit `--diagnostics <path>`
naming the NDJSON of the build to re-project — "most recent build" is the
caller's knowledge (Manager owns build history), never a compiler-side
search. No diagnostic at the location is an empty report with exit 0,
not a failure.

## 3. §14.14.3 names `[dev] watch` config the request document cannot carry

§14.14.3 says "watch-mode rebuilds respect `[dev] watch = true` and
`[dev] watch-exclude = […]` from §07" — but the §14.1.1 request schema
mirrors only `build`, `memory`, `folders`, `dependencies`,
`compile_limits`, and `telemetry`. There is no `[dev]` projection, and
watching files is filesystem discovery the compiler must not do (CMP-01):
both halves of that sentence describe the *caller* (Clean Framework reads
`clean.toml`, watches the tree, and lowers a fresh request per edit).

**Local adoption (in force):** the compiler ships no watch API, no watch
mode, and no `[dev]` request section. The compiler-side §14.14.3 contract
is exactly the sentence that is about the compiler — "a watch-mode
rebuild produces the same `component.wasm` bytes as a full `debug`
build" — pinned by `watch_rebuild.rs`: the warm in-process rebuild is
byte-identical to a cold process-adapter build (component and manifest),
and the loop leaves no state behind (cycling back to an earlier request
reproduces its earlier bytes). The rebuild-latency target is informative
(§14.9) and belongs to M9 measurement.
