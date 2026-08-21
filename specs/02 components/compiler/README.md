# Compiler

This folder specifies the **Clean compiler** — the component that turns Clean source into a WebAssembly Component Model component, and the only part of the toolchain that understands what Clean code means. It is handed one self-contained compilation request and returns a component, a reproducibility record, and diagnostics; it reads no files and reaches no network of its own. The language server ships from the same source and is documented here as part of this component family.

## Sections

- [01-specification.md](./01-specification.md) — what the component owns, its design rules (`CCMP-`), what it emits, what it explicitly does not do, how it is installed and pinned, where the language server sits, and (§12) what building the rest of the toolchain against a compiler stand-in established about this specification.

## Where the rest of the compiler's story lives

The compiler's **contract with the rest of the toolchain** is not here — it is a platform concern, because framework, manager, and the hosts all build against it:

- [Platform 14 — Compiler Architecture](../../03%20platform/14-compiler-architecture.md) — the request document, the outputs, determinism, the pass pipeline, diagnostics, the API operations (`CMP-`).
- [Platform 15 — Component Model Architecture](../../03%20platform/15-component-model-architecture.md) — the WIT vocabulary, world naming, and the conformance model the emitted component targets (`CMOD-`).
- [Platform 16 — Host Contract Validation](../../03%20platform/16-host-contract-validation.md) — the three check Moments and the scope split that fixes which one the compiler performs (`HCV-`).
- [Platform 04 — IDE and Language Server Architecture](../../03%20platform/04-ide-lsp-architecture.md) — the LSP contract the language server implements (`LSP-`).

## Relationship to other components

- **Clean Framework** assembles the compilation request and owns the build cache: [framework/](../framework/README.md).
- **Clean Manager** owns `cln`, and installs and pins compiler versions: [manager/](../manager/README.md).
- **Hosts** load and verify the emitted component: [hosts/](../hosts/README.md).
- Responsibility boundaries across all of them: [Governance — Architecture Boundaries](../../01%20governance/01-architecture-boundaries.md).

---

## Metadata

- **Status:** Draft
- **Kind:** Definition
- **Audience:** Anyone navigating to the compiler's specification — component authors, integrators, and maintainers of the components on either side of its boundary
- **Part of:** [../README.md](../README.md)
- **References:** [Platform 14 — Compiler Architecture](../../03%20platform/14-compiler-architecture.md), [Governance — Architecture Boundaries](../../01%20governance/01-architecture-boundaries.md)
