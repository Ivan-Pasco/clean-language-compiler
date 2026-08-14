# ADR 0001 — Reference stack for this implementation

- **Status:** Accepted (2026-08-14)
- **Context:** Foundation [ADR-0006](../../../clean-language-foundation/01%20governance/decisions/0006-compiler-reference-stack.md) fixes the reference shape: a three-crate Cargo workspace, a hand-written recursive-descent lexer/parser, the wasm-tools crate family pinned together, wasmtime for the compile-time block-handler sandbox, `ena` for inference, `clap`/`toml` confined to the binary adapter, and `anyhow`/`tracing`/`rayon` excluded from v1. ADR-0006 declares its crate names and versions non-normative; the observable contracts (Platform 14) are what bind.

## Decision

Adopt ADR-0006 as written, with three deviations recorded here:

1. **wasm-tools family at `0.256`, not `^0.220`.** The family must stay pinned together (one `wasm-tools` workspace release); `0.256` is the current stable line and matches the `wasm-tools 1.252+` CLI used by the acceptance checks and by clean-server's own tooling. Bumping the family later is a deliberate act: it can change emitted bytes, which is a semver event for this component (CCMP-24).
2. **wasmtime at `47`, not `^38`.** clean-host-core — the runtime that will instantiate what we emit — pins wasmtime 47 (and drives the workspace `rust-version = 1.94`). Testing the Canonical ABI against a different major than the real host would validate the wrong thing. wasmtime is a dev-dependency until pass [6] (block handlers, M5) makes it a production dependency with the ADR-0004 sandbox configuration.
3. **`serde_path_to_error` added.** `RQD002`'s message template (Platform 10 §16) names the JSON path of the offending key (`… at '$.target'`); serde alone reports line/column. This is a small, boundary-only dependency used exclusively in pass [1] intake.

`wac-graph`/`wac-parser` (foundation ADR-0026, Draft) are not yet needed: composition is the framework's job; this repo emits a single component. They enter if/when a compiler-side composition surface is specified.

## Consequences

- The workspace tracks clean-host-core's wasmtime major from the start; drift between "what we test against" and "what hosts run" is a conscious decision, not an accident.
- Any future family bump lands with the determinism suite proving whether emitted bytes changed, and a version bump sized accordingly (CCMP-24).
