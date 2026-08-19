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
(`clean-compiler-bin/tests/watch_rebuild.rs`). Stage 3 (build
reproduction, §14.14.6 first half) landed as `repro::repro_build` with a
pluggable `InputResolver` (`tests/repro_build.rs`,
`clean-compiler-bin/tests/repro_cli.rs`, local ADR 0007). Stage 4
(request replay, §14.14.6 second half) landed as `replay::replay` over a
provisional trace schema (`tests/replay_operation.rs`).

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

## 4. §14.14.6 reproduction is unimplementable against the §14.8 manifest as written

The reproduction operation must "invoke `compile()` with the identical
request" reconstructed from the build manifest — but §14.8 records
neither `project` nor `target_world` (not even by hash), and the spec's
two-method `InputResolver` cannot refetch the world contract at all.
`resolved_config` is also resolved values, not request echoes (the memory
tier comes back resolved), so `request_sha256` cannot be re-derived from
a reconstruction even in principle.

**Local adoption (in force, local ADR 0007):** two optional manifest
additions (`inputs.project` verbatim; `inputs.target_world` as the four
identity fields, WIT refetched by hash) and a third resolver method
`fetch_world(host, version, sha256)`. Every fetched input is re-verified
against its recorded hash — the resolver is a store, never an authority.
The operation asserts `outputs.wasm_sha256` (the §14.14.6 assertion), not
`request_sha256`. Foundation brief candidate: either extend §14.8 with
these records or declare how else §14.14.6 names the request.

## 5. First-divergent-byte reporting needs the artifact the manifest only hashes

§14.14.6: "On mismatch, reports the first byte of divergence." The
manifest records `outputs.wasm_sha256` — a hash has no bytes to diff
against. **Local adoption (in force, `repro.rs`):** the operation takes
the originally shipped `component.wasm` as an optional input
(`--original` on the adapter); with it the divergence report carries the
first differing byte offset, without it the report carries the two
hashes. A provided original that does not hash to the manifest's record
is detected before any rebuild as corruption. Divergence and corruption
present as COM013 per §14.14.6 ("a compiler bug or a manifest
corruption") — note this COM013 path lives outside the DIA-06 check
harness, whose ledger tracks the build/check surface only.

## 6. The request-trace format §14.14.6 relies on is defined nowhere

§14.14.6 says the trace format "is defined in the clean-server spec
(compiler ships the schema for validation but does not define capture
semantics)" and §07 points `[dev] capture-traces`' behavior home back at
Platform 14. The clean-server spec defines no request-trace schema (its
`schema/` holds only the reload channel and the server block; the trap
snapshot of hosts/01 §Trap is a different artifact). The reference is
circular and the format does not exist.

**Local adoption (in force, `replay.rs`):** a provisional trace schema,
pinned byte-for-byte by `replay_operation::trace_schema_validation_is_strict`:
`spec_version` ("1", mirroring §14.1.1), `component_sha256`, `entry`
(world-level export plus arguments), `host_calls` (each
`<instance>#<function>` with captured arguments and results, in call
order), `response` (the entry's results). Values are a typed JSON
encoding of `component::Val` covering the kinds the vendored host
contract uses; anything else is a typed `UnsupportedValue` failure.
Unknown fields are refused like the request document's own intake.

The replay host composes with the component runtime (wasmtime, the
sandbox stack of local ADR 0001): every imported host function is wired
to serve the next recorded call — same function, same arguments — and
anything else (wrong call, wrong arguments, extra or leftover recorded
calls, response mismatch) is a typed divergence, presented per §14.14.6
as "a Clean runtime bug or a trace corruption", deliberately without a
diagnostic code: the runtime codes are host-emitted and none covers
this; inventing one would violate DIA-01.

Foundation brief candidate: give the request trace a normative home
(clean-server spec per §14.14.6's own pointer), taking this pinned shape
as the draft.
