# ADR 0006 — Language server crate and LSP stack

- **Status:** Accepted (2026-08-19)
- **Context:** Compiler component spec §9 places the language server in this
  component family: built from the same source as the batch compiler, shipped
  in the same distribution, at the same version (CCMP-25, CCMP-26). Platform 04
  owns the protocol contract (LSP-01..05); Platform 13 §7 owns the normative
  `Diagnostic` → LSP mapping. Foundation ADR-0006 fixes a three-crate workspace
  for the *compiler* but declares crate names and internal structure
  non-normative — the observable contracts bind, the shape does not.

## Decision

1. **A fourth workspace crate, `crates/clean-language-server`.** It depends on
   `clean-compiler` as a library and calls the same `check` entry point the
   batch `--check` path uses — the lexer, parser, and type checker are shared
   by construction, not by discipline (CCMP-25). The crate carries a library
   (the server loop, testable in-process) and a thin binary target
   (`clean-language-server`), mirroring the compile-side split: if a bug
   appears only through stdio and not through the library API, the bug is in
   the transport.
2. **LSP stack: `lsp-server` 0.10 + `lsp-types` 0.97.** The rust-analyzer
   scaffold: a synchronous, channel-based message loop over stdio with an
   in-memory transport for tests, and the typed protocol structs. No `tokio`,
   no `tower-lsp` — foundation ADR-0006 excludes ambient async/parallelism
   from v1, and a single-threaded loop keeps diagnostic emission ordered and
   reproducible. Both crates enter the workspace dependency table pinned like
   every other dependency.
3. **Binary name `clean-language-server`.** No spec names the binary (Platform
   04 says only "the language server binary"; Manager resolves it at the pin).
   Like `clean-compiler`, it is not a user-facing command (CCMP-04): editors
   reach it through Clean Manager's resolution, never by hand-configured
   absolute paths — the README examples in the retired repo that document
   per-editor manual wiring are exactly what LSP-05 forbids.

## Consequences

- The language server versions with the workspace: one version, one tag, one
  distribution (CCMP-26). A language-server-only fix is a workspace release —
  the open question in component spec §11 stays open at the spec level, but
  this repo's answer until it closes is "no independent versioning".
- Every diagnostic the server publishes is produced by the pipeline and mapped
  per Platform 13 §7; the server crate itself registers no diagnostic codes
  and owns no language knowledge (LSP-01) — no keyword lists, no completion
  tables, no message rewording. The retired repo's server hardcoded all three
  and drifted; the contract test (LSP diagnostics ≡ `check` diagnostics on the
  same request) makes that drift a test failure here.
- `lsp-types` pins the protocol at LSP 3.17 (`positionEncoding`,
  `Diagnostic.data`). Position encoding is negotiated at `initialize`;
  conversion from the compiler's 1-based character columns (Platform 13 §2)
  happens at the emission boundary, per Platform 13 §7.
