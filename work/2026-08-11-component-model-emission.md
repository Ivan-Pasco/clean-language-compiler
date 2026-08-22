# Task — Compiler: Component Model emission

> **Relocated 2026-08-22** from `clean-language-foundation/work/` to this repo, per `clean-language-foundation EXE-04` (a brief lives in the repository it changes). Its foundation-relative links (`../03 platform/…`, `../01 governance/…`) date from that residence and do not resolve here; read them as qualified rule IDs against the sibling `../clean-language-foundation` checkout (commit pinned in CLAUDE.md). State at relocation: steps 1–8 done, step 9 blocked on `clean-host-core` access.

The `clean-compiler` component does not exist yet. It is the last unbuilt piece of the V2 toolchain and the critical path for the whole developer loop: the framework's capability wiring, `clean-server`'s acceptance suite, `cln run`, and the Moment 1/3 contract checks are all blocked until a Clean program can be compiled into a WebAssembly **Component Model component** whose imports are interface-qualified (`clean:http/routing@0.1.0`). This brief scopes the first milestone of that component: emitting a conformant guest for the `server` world.

---

## Scope

A `clean-compiler` binary, installed and dispatched like every other managed toolchain artifact ([MGR-01](../02%20components/manager/00-manager.md#mgr-01--one-front-door), [MGR-04](../02%20components/manager/00-manager.md#0013-runtime-management)), that turns a compilation request document into a Component Model component satisfying the `server` world.

Observable deliverables:

- Accepts **one request document** on the invocation surface of [Platform 14 §14.2](../03%20platform/14-compiler-architecture.md#142-invocation-surface) — sources inline, config resolved, nothing read from the filesystem or network ([CMP-01](../03%20platform/14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)).
- Emits a **component**, not a core module. Header `0061 736d 0d00 0100`.
- Host imports named by WIT interface and version, resolved against the target world delivered in the request — never a flat namespace.
- The target world's WIT embedded in the component so a host can run its Moment 3 check ([HCV-01](../03%20platform/16-host-contract-validation.md#hcv-01--three-check-moments-each-with-its-actor-and-its-code)).
- Canonical ABI lifting/lowering for the type surface `clean-server/host.wit` uses: `string`, `list<T>`, `option<T>`, `record`, `enum`, `u32`/`u8`.
- The World Import Check ([CMP-03](../03%20platform/14-compiler-architecture.md), pass [9]) emitting `COM012` for a call site absent from the target world.
- Byte-identical output for a byte-identical request ([CMP-02](../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out)).

Done means: `cln build` on the acceptance project produces a component that `clean-server` instantiates and serves a request from, replacing the hand-written WAT fixture at `clean-server/testing/fake-guest/`.

## Non-goals

- **Other worlds.** `browser`, `cli`, `worker`, `edge` come later. `server` first: it is the only world with a published `host.wit` and a running host to validate against.
- **The full language.** This milestone needs whatever surface the acceptance guest exercises — enough to register a route and return a response. Language completeness is a separate track.
- **Bridge composition.** The compiler emits a guest that *imports* capabilities. Composing bridges into it is `clean-host-core`'s job and already works.
- **`host.wit` fetching.** The compiler never touches network or filesystem for WIT ([CMP-01](../03%20platform/14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)). The framework fetches it and puts it in the request document.
- **Reusing the previous-generation compiler's codebase.** That project is out of scope for this brief. Any design carried forward is a deliberate decision recorded in the component's own ADR, not an assumption inherited by default.

## Prerequisite — a spec gap that blocks the World Import Check

[CMP-01](../03%20platform/14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else) (14 §14.3) requires the compiler to obtain the **target world WIT** from the request document, and [CMP-03](../03%20platform/14-compiler-architecture.md) requires validating every host-function call site against it.

**The request-document schema in [§14.1.1](../03%20platform/14-compiler-architecture.md#1411-inputs) has no field carrying it.** `library_manifests[].wit` is library WIT only.

The compiler cannot validate against a world it is never handed. Before the check can be written:

- Add a `target_world` field to the §14.1.1 schema — the WIT text of the world named by `build.target`, plus its source identity (host name, version, and the hash the framework recorded in `.cln/lock.toml` per [BVER-03](../03%20platform/08-bridge-versioning.md#84-host-declaration)).
- This is a request-document schema change: a spec edit to 14 §14.1.1, a `spec_version` consideration, and a framework-side change to populate it.

Agree the field shape with the framework session before building against it. Both sides land together or neither works.

## Steps

Each step is independently verifiable. Do not start the next until the previous one's check passes.

1. **Scaffold the component.** Cargo workspace, `clean-compiler` binary, CI, release-on-tag — matching how `clean-framework` and `clean-server` are set up so Manager can install it identically. An ADR pinning the reference stack (WIT tooling, encoder, versions) lands here, per the pattern of [ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md).

2. **Accept a request document and return a diagnostic.** Implement the §14.2 invocation surface end to end with no compilation behind it: parse the request, verify every `sources[].sha256`, reject unknown keys (`RQD001`/`RQD002`), return a well-formed diagnostics payload. This proves the seam the framework already speaks to before any codegen exists.

3. **Parse a world.** Read `clean-server/host.wit` (as a fixture) and assert the `server` world exposes the eight interfaces it declares. Proves the WIT dependency and the parse in isolation.

4. **Emit an empty component.** Produce a component that imports nothing and exports nothing, and assert the header is `0061 736d 0d00 0100`. Isolates "can we produce a component at all" from "are its imports right."

5. **Compile the minimum language surface** the acceptance guest needs, to a core module — whatever subset registers a route and returns a response.

6. **Canonical ABI: lift/lower.** In dependency order — `u32`/`u8`, then `string`, then `list<T>`, then `record`, `option<T>`, `enum`. Each type round-trips through the boundary in a test before the next is started. **This is the hard step; expect it to dominate the estimate.**

7. **Consume `target_world` and implement the World Import Check.** Blocked on §Prerequisite. Walk host-function call sites, verify each exists in the delivered world, emit `COM012` and abort before codegen on any miss.

8. **Interface-qualify the imports and embed the world WIT.** Host imports carry their WIT-derived interface names; the target world's WIT is attached to the component so the host's Moment 3 check has something to read.

9. **Replace the acceptance fixture.** Build the equivalent of `clean-server/testing/fake-guest/` from `.cln` source and confirm the server serves a request from it.

## Acceptance checks

Tooling verified present on the dev machine as of 2026-08-11: `wasm-tools 1.252.0` at `~/.cargo/bin/wasm-tools`, with the `component wit`, `component new`, `component embed`, and `component targets` subcommands.

```bash
# 1. Output is a component, not a core module.
xxd -l 8 dist/app.wasm            # expect: 0061 736d 0d00 0100

# 2. Imports are interface-qualified.
wasm-tools component wit dist/app.wasm | grep 'clean:http/'
wasm-tools validate dist/app.wasm

# 2b. The component conforms to the published world — stronger than grep.
wasm-tools component targets "$SERVER/host.wit" dist/app.wasm   # world: server

# 3. The world check actually rejects.
cln build tests/cln/component/import-not-in-world.cln   # expect: COM012, no dist/app.wasm

# 4. Round-trip: every type in the scope list survives the boundary.
cargo test canonical_abi

# 5. Determinism — same request in, byte-identical component out.
cargo test determinism

# 6. End to end — the real proof.
cargo run --bin clean-server -- testing/fixtures/hello-world/host.toml
curl http://127.0.0.1:3000/       # expect: hello world, from a cln-built guest
```

The task is Done when check 6 passes against a component produced by `cln build` — not before, and not with the WAT fixture in place.

Two existing gates should go green as a consequence, and are worth re-running:
- `clean-server conformance` currently reports `INCOMPLETE` and exits non-zero because `tests/cln/conformance/` cannot be populated without component output.
- `clean-server`'s CI runs it with `continue-on-error`; that line comes out when the corpus lands.

## Discoveries

Recorded 2026-08-15 by the compiler implementation session (steps 1–8 done,
step 9 blocked on `clean-host-core` access; full detail and rationale in
`clean-language-compiler/docs/DISCOVERIES-M1.md` and `docs/adr/0001`/`0002`).

1. **Acceptance check 2b reads the wrong world.** A guest cannot satisfy
   `wasm-tools component targets host.wit -w server` — `server` is the world
   the *host* implements (exports the interfaces, imports `init`/`handle`).
   The operative check is `component targets` against the guest's mirror
   world (`clean:guest/app` with host.wit as a dep — the shape of
   clean-server's own `testing/fake-guest`), plus the host's Moment 3 gate.
2. **host.wit uses types beyond this brief's scope list**: `u16`
   (`set-status`), `u64` (socket/stream ids), `result<_, E>` and `variant`
   (ws/sse/session-envelope). Scalars were absorbed in step 6;
   `result`/`variant` ride with the SSE/WS routes (M6).
3. **LBS-02's type table lacks unsigned widths and world-type references.**
   Adopted locally (compiler ADR-0002): `integer:u8|u16|u32|u64`;
   identifiers in host-function positions resolving to types the world
   interface declares (`method`, `options`, `level`, `field`); enum
   parameters as compile-time string literals naming a case; classes
   matching records structurally (kebab-cased). Candidate LBS-02 amendment.
4. **`string` satisfies `bytes` at the host boundary** (identical
   (ptr, len) UTF-8 representation) — adopted until the cap-15 conversion
   surface lands. Candidate LBS-02 note.
5. **Entry points are a local convention**: `init`/`handle` as ordinary
   `functions:` entries the compiler exports, with a u32→integer widening
   shim on `handle` (compiler ADR-0002). To be superseded by framework
   route discovery (CCMP-13).
6. **14 §14.8 `timings` and CMP-02 are in tension**: a byte-identical
   `build-manifest.json` cannot carry wall-clock timings. M1 emits zeros;
   the spec should either exclude timings from the identity or drop them.
7. **`compiler.sha256` in the manifest has no producer**: a library cannot
   hash its own binary. M1 derives it from the crate version; the release
   pipeline (or process adapter) should stamp the real binary hash.
8. **Reference-stack versions moved** (compiler ADR-0001): wasm-tools
   crate family 0.256 (ADR-0006 pinned ^0.220), wasmtime 47 aligned with
   clean-host-core (ADR-0006 pinned ^38), plus `serde_path_to_error` for
   RQD002 JSON paths.
9. **`clean-host-core` is private** and clean-server needs it as a path
   dep — acceptance check 6 is blocked on repo access for any machine
   without the deploy key.

Recorded 2026-08-20 by the compiler release-preparation session (commit
`clean-language-compiler@0a26375`, first release packaging).

10. **Release-asset names drifted from the manager's install contract.**
    The compiler's original release.yml shipped 2 targets under the old
    `clean-compiler-<version>-linux-x86_64.tar.gz` convention; the contract
    is exactly `linux-x64, macos-x64, macos-arm64, windows-x64`
    ([manager automation sheet](../02%20components/manager/automation.md)),
    with archives named `<component>-<version>-<target>` — `.tar.gz` on
    Unix, `.zip` on Windows — each alongside a `.sha256`
    ([Release Workflows — archive conventions](../05%20execution/automation/02-release-workflows.md#archive-conventions)).
    Fixed in 0a26375. The naming is convention, not linted ("drift here
    breaks the manager silently"), and its consumer does not exist yet:
    when the clean-manager implementation lands, its `cln install`
    resolver and the `install-uninstall-matrix` nightly (manager
    automation sheet) must be tested against these four names before the
    convention is trusted end to end.

---

## Metadata

- **Status:** Ready
- **Date:** 2026-08-11
- **Implements:** [CMP-01](../03%20platform/14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else), [CMP-02](../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out), CMP-03 / `COM012` (14 §14.4.2 pass [9], §14.6), pass [10] componentization (14 §14.4.2), [CMOD-03](../03%20platform/15-component-model-architecture.md) conformance gate
- **Inputs:** [14 — Compiler Architecture](../03%20platform/14-compiler-architecture.md) (Accepted), [15 — Component Model Architecture](../03%20platform/15-component-model-architecture.md) (Accepted), [16 — Host Contract Validation](../03%20platform/16-host-contract-validation.md) (Accepted 2026-08-01), [08 — Bridge Versioning](../03%20platform/08-bridge-versioning.md) (Accepted 2026-08-01), `clean-server/host.wit` (published contract, 8 interfaces)
- **Downstream:** [ADR-0032](../01%20governance/decisions/0032-capability-wiring-generated-host-toml.md) capability wiring — FRM-BO-12 cannot derive `[bridges]` until a component carries a readable import list.
