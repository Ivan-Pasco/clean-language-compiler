# Milestone 1 — Discoveries (ready to paste into the brief)

For `foundation/work/2026-08-11-compiler-component-model-emission.md`
§Discoveries. This repo's session is path-allowlisted out of foundation
(CT-H-16); a foundation session should carry these over.

1. **The `targets -w server` acceptance check reads the wrong world.** A
   guest cannot target `server` — that is the world the *host* implements
   (exports the interfaces, imports init/handle). The operative check is
   `component targets` against the guest's mirror world (`clean:guest/app`
   with host.wit as a dep, the fake-guest's own shape), plus the host's
   Moment 3 gate. The brief's check 2b should be reworded.
2. **host.wit uses types beyond the brief's list.** `u16` (set-status),
   `u64` (socket/stream ids), `result<_, E>` and `variant` (ws/sse/
   session-envelope). Scalars were absorbed in step 6; `result`/`variant`
   ride with the 9b routes (M6).
3. **LBS-02's type table needs unsigned widths and world-type references.**
   Adopted locally (ADR-0002): `integer:u8|u16|u32|u64`, and identifiers in
   host-function positions resolving to types the world interface declares
   (`method`, `options`, `level`, `field`); enum parameters take a
   compile-time string literal naming a case; classes match records
   structurally (kebab-cased). Candidate LBS-02 amendment.
4. **String satisfies bytes at the host boundary** (identical (ptr,len)
   UTF-8 representation) — adopted until the cap-15 conversion surface
   lands. Candidate LBS-02 note.
5. **Entry points**: `init`/`handle` are ordinary `functions:` entries the
   compiler exports, with a u32→integer widening shim on `handle`. To be
   superseded by framework route discovery (CCMP-13).
6. **§14.8 timings vs CMP-02.** Byte-identical `build-manifest.json` and
   wall-clock `timings` cannot coexist; M1 emits zeros. The spec should
   pick (exclude timings from the identity, or drop them).
7. **`compiler.sha256`** in the manifest needs a producer: a library cannot
   hash its own binary. M1 derives it from the crate version; the release
   pipeline (or the process adapter) should stamp the real binary hash.
8. **Reference-stack versions moved** (recorded in docs/adr/0001):
   wasm-tools crate family 0.256 (ADR-0006 said ^0.220), wasmtime 47
   aligned with clean-host-core (ADR-0006 said ^38), plus
   `serde_path_to_error` for RQD002 JSON paths.
9. **`clean-host-core` is private** and clean-server needs it as a path
   dep — acceptance check 6 is blocked on repo access for any machine
   without the deploy key.
