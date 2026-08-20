# Discoveries — Milestone 8 (API operations v1)

Spec gaps and under-specifications found while implementing M8. Each item
either becomes a task brief in foundation `work/` (written from a
foundation session — this repo never writes there) or records a local
adoption that stays in force until foundation resolves it.

## Status

M8 complete (2026-08-19, 7 stages). Every Platform 14 v1 API operation
ships with a contract test over the same request document (the M7 parity
model), and the milestone gate
(`clean-compiler-bin/tests/operations_gate.rs`) runs the full circle —
build → check → why → repro build → replay → bridge stub → JSON-RPC — on
one document, asserting every surface names the same build:

- **§14.14.1 `cln why`** — `why::why`, corpus-scale contract
  (`tests/why_operation.rs`), adapter `--why` (`cli.rs`).
- **§14.14.3 watch-mode** — a contract without an API
  (`watch_rebuild.rs`): warm rebuild ≡ cold full build, stateless loop.
- **§14.14.4 `cln check`** — shipped in M4; its contract rides the DIA-06
  harness and the M7 parity gate.
- **§14.14.6 repro build** — `repro::repro_build` + `InputResolver`
  (`tests/repro_build.rs`, `repro_cli.rs`, local ADR 0007).
- **§14.14.6 replay** — `replay::replay` + replay host over the component
  runtime (`tests/replay_operation.rs`), provisional trace schema.
- **§14.14.5 bridge stubs** — `stub::generate_stub`
  (`tests/bridge_stubs.rs`); the normative WIT catalog is missing (§7).
- **§14.2.3 JSON-RPC / MCP** — the bin's `--serve` (`serve_rpc.rs`), MCP
  tools wrapping the identical handlers.

spec_version stayed frozen at "1": the request schema is untouched; the
one output-schema deviation (build manifest) is local ADR 0007.

**Round-trip CLOSED** (2026-08-19, foundation HEAD e2afcb9; report
received the same day). No local adoption was invalidated; no change to
§14.8, to the §13 Diagnostic value, or to the trace schema;
`spec_version` stays "1". Per item:

- §1 + §2 → brief `work/2026-08-19-diagnostic-pass-provenance.md`
  (registry derivation vs an optional `pass` field on §13 — pending).
- §3 → **erratum applied** to Platform 14 §14.14.3 (foundation 19e2ae9):
  watching and `[dev]` config attributed to Clean Framework; the
  compiler-side obligations (no watch API, no `[dev]` request section,
  observationally stateless warm loop) now normative — the
  `watch_rebuild.rs` adoption is **ratified verbatim**. Foundation also
  noted `[dev] watch-exclude` is absent even from Platform 07 §7.2.
- §4 + §5 → brief `work/2026-08-19-repro-manifest-request-identity.md`,
  taking local ADR 0007 verbatim as the draft shape. ADR 0007 in force.
- §6 → brief `work/2026-08-19-request-trace-schema-home.md`; proposed
  home `clean-server spec schema/request-trace.json.md`, with the
  `replay.rs` pinned schema as the draft. Adoption in force.
- §7 → brief `work/2026-08-19-bridge-wit-catalog-and-stub-fixtures.md`.
  **Correction of fact** to this file's §7: foundation *does* have a
  `wit/` directory, but as an unpopulated placeholder at
  `03 platform/wit/` (README only) — not at the repo root Platform 02
  §2.2 names. WIT authoring is sequenced behind the open M6 brief
  `2026-08-19-bridge-runtime-and-import-architecture.md` (which may
  redraw math/string signatures for determinism); expect the catalog
  only after that resolves. Generator, generation-time fixtures, and the
  TraceValue fixture encoding stay in force.
- §8 → brief `work/2026-08-19-rpc-protocol-surface.md` (informational,
  low urgency). The `--serve` surface stays in force.

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

## 7. The bridge WIT catalog §14.14.5 stubs depend on does not exist

§14.14.5 requires a stub component per `clean:bridge/*` interface, living
"in the same repository as the interface WIT so drift is impossible" and
shipping in the compiler tarball at `dist/bridges/stubs/`. Platform 02
§2.2 says "the full WIT source of each interface lives in the `wit/`
directory at the root of this repository" — but the foundation repository
has no `wit/` directory: the seven-interface L2 catalog (console, db,
crypto, mem, math, string, files) exists only as a table. No stub can be
generated against WIT sources that do not exist, and authoring them here
would be exactly the drift §14.14.5 forbids.

**Local adoption (in force, `stub.rs`):** the *generator* — WIT interface
in, stub component out — with the three §14.14.5 semantics pinned by
`tests/bridge_stubs.rs` (recorded-call log, fixture-driven canned
responses, non-fixture calls trap), exercised against the informative
`clean:bridge/console` shape of Platform 02 §2.1. The catalog feeds the
generator when foundation lands the WITs. Two provisional choices:

- **Fixture at generation time, not instantiation.** "Handed to the stub
  at instantiation" would force a JSON parser into every stub component
  or a host-side fixture import that `cln test` must wire; baking at
  generation keeps stubs dependency-free and the test loop identical
  (regenerating is milliseconds). Brief candidate alongside the catalog.
- **Fixture values reuse the replay trace's typed encoding**
  (`replay::TraceValue`), so §14.14.6 traces and §14.14.5 fixtures speak
  one value language. v1 shape subset: scalars, strings, enums /
  payload-less variants; records/lists/options are typed `Unsupported`
  refusals until the real catalog fixes the needed shapes.

Adapter surface: `--bridge-stub <interface> --wit <path> --fixture
<path>`, writing the dist-layout name `clean-bridge-<interface>-stub.wasm`.

## 8. §14.2.3 fixes the wire payload but not the protocol surface

§14.2.3 requires the JSON-RPC / MCP adapter and fixes one thing: "the
wire format is the request document unchanged." Method names, framing,
the outcome model, and the MCP dialect's shape are unspecified.

**Local adoption (in force, `clean-compiler-bin/src/serve.rs`, behind
`--serve`):** one JSON-RPC 2.0 message per line over stdio (the MCP stdio
framing, so one loop serves both dialects). Direct methods `compile`,
`check`, `why`, `reproBuild`, `replay`, `bridgeStub`; MCP `initialize` /
`tools/list` / `tools/call` wrap the identical handlers, so the two
surfaces cannot diverge (pinned by `serve_rpc.rs`). Outcome model: an
operation that ran returns a result even when the program failed (a
rejected compile carries its diagnostics; a diverged replay says so);
JSON-RPC errors are reserved for protocol misuse. Component bytes ride
base64. Everything arrives in the message and leaves in the response —
the serve loop touches no files (CMP-01 with nothing to point at).

## 9. Error lowering (post-M8 backlog): runtime-message wordings are unspecified

Chapter 13 defines the failure semantics completely (ERH-01..05) and the
compiler now implements them: an error channel (three wasm globals —
flag, message, code), `error(...)` raising, general suffix `onError`
catching any failing expression with the `error` binding (message +
optional code) copied into locals so nested catches cannot clobber an
outer binding, propagation out of callees via post-call flag checks, and
the ERH-05 top: entry shims trap when the flag survives to the boundary
(the RUN018 shape). RUN003 (division/remainder by zero, division
overflow, number→integer domain, string→number parse) and RUN013 (list
index, empty-collection access, string code-point index) raise
catchably; RUN013 uses the Platform 10 template filled at raise time.

What the spec does not say — local pinnings, in force
(`tests/error_lowering.rs` pins each):

- **RUN003 has no message template** (Platform 10 stub): local wordings
  "division by zero", "integer overflow in division", "cannot convert
  NaN to integer", "number is out of the integer range", "the string is
  not a valid integer literal" / "…number literal".
- **Empty-collection access** (`first()`/`last()`/`remove()`/`peek()`)
  fills the RUN013 template with index 0, length 0.
- **Host failures** (LBS §8.3: the payload never surfaces): the binding
  carries "host function `{cleanName}` failed" and `code = none`.
- **Number division by zero** stays IEEE (infinity): wasm f64 division
  never traps and chapter 15 does not classify it as RUN003; only
  integer arithmetic raises.
- **Block-form `onError:`** is parsed and type-checked but its lowering
  remains `note_unsupported` (pre-existing frontier, unchanged by this
  work).
