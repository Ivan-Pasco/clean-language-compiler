# ADR 0003 — Wire ABI between the compiler and compile-time handler wasm

- **Status:** Accepted (2026-08-18)
- **Context:** ADR-0004 (foundation) decides *where* handlers run — a
  sandboxed wasmtime sub-instance inside the compiler's pass [6] — and
  *what* they exchange at the language level: a typed `BlockAST` in, typed
  `IR` out (chapter 21 §21.3/§21.4, `schema/block-ast.md`). No document
  specifies the *wire* form of that exchange: how a `BlockAST` enters a
  wasm instance, how the `IR` and the handler's `error/warning/info`
  emissions come back, or what the artifact must export. The framework
  that will compile handlers does not exist yet, so this repo defines the
  ABI its sandbox consumes and the fixture handlers exercise.

## Decision

1. **Artifact form.** `library_manifests[].compiletime_wasm` is a core
   wasm module (not a component). It must export:
   - `memory` — a linear memory;
   - `alloc: func(i32) -> i32` — returns a pointer to `size` writable
     bytes;
   - `expand: func(i32, i32) -> i32` — receives (pointer, length) of the
     input document, returns a pointer to an 8-byte header: `u32` LE
     out-pointer followed by `u32` LE out-length.
2. **Input document.** UTF-8 JSON serialization of the `BlockAST`, field
   names exactly as `schema/block-ast.md` spells them (`name`,
   `arguments`, `body`, `attributes`, `span`, `byteRange`). `BlockNode`
   is tagged `{"kind": "line" | "block"}` — the `Statement` variant is
   unproducible at parse time (M3 discovery 2; foundation brief still
   Ready), so the ABI does not carry it.
3. **Output envelope.** UTF-8 JSON: `{"ir": <IR>, "diagnostics": [...]}`.
   `<IR>` is a tree of builder nodes tagged by `"kind"`, one per §21.4
   builder (`class`, `function`, `field`, `method`, `return`, `assign`,
   `if`, `block`, `call`, `literal_integer`, `literal_string`,
   `variable`, `field_access`, `concat`, `empty`, `with_span`).
   `diagnostics` entries are `{severity, code, message, span}` per the
   schema's `Diagnostic` — the BLK-03 `error/warning/info` calls are
   *collected* by the handler and returned in the envelope, not streamed
   through imports. This keeps the sandbox import surface empty, which is
   what chapter 21 §21.7 requires anyway ("all host imports stubbed").
4. **No imports, no WASI.** The sandbox links `wasmtime` only —
   **not `wasmtime-wasi`**. Everything WASI could provide (clocks, random,
   fds, environment) is exactly what BLK-04 forbids (`BLOCK006`), so the
   deterministic realization of ADR-0004's "no wall clock, seeded
   randomness" is that no such import exists at all: every function
   import a handler declares is stubbed to a trap that records the name
   (`BLOCK006` if called); non-function imports are artifact defects.
   Determinism flags on the engine: NaN canonicalization, deterministic
   relaxed-simd, epoch interruption (epoch over fuel, ADR-0004).

## Consequences

- Fixture handlers are hand-written WAT against this ABI until the
  framework exists; when foundation specifies a normative ABI, this ADR
  is superseded and the sandbox adapts behind `blocks::sandbox`.
- The envelope (not imports) carrying diagnostics means a handler that
  traps loses its pending diagnostics — acceptable: a trapped handler is
  `BLOCK004` and its partial output is unusable by CMP-05 discipline.
- Gap reported to foundation via docs/DISCOVERIES-M5.md (wire ABI, and
  the `compiletime_wasm` bytes field the request schema lacks).
