# Compiler — Specification

**Status:** Draft — the component described here does not exist yet. Every other component it names now does, and each has been built and tested against a compiler stand-in that implements the process contract in §4. Where those components exercised this specification, §12 records what held and what needed correcting.

The Clean compiler is the component that turns Clean source into a runnable WebAssembly component. It is handed one self-contained document describing everything to compile, and it hands back a component, a reproducibility record, and a list of diagnostics. It reads no files of its own, reaches no network, and knows nothing about projects, folders, registries, or the machine it runs on — everything it needs arrives in the request, and everything it produces goes where the caller asked. This document says what the component owns, what it refuses to own, and how it sits beside Clean Framework, Clean Manager, and the hosts.

---

## 1. What this component is

The compiler is one of the four managed toolchain artifacts a developer's machine holds — compiler, framework, runtime, manager ([MGR-01](../manager/00-manager.md#mgr-01--one-front-door)). It is the only one that understands what Clean code *means*.

Its neighbours, and the seam with each:

- **Clean Framework** reads `clean.toml`, walks the project, resolves libraries, compiles block handlers, and lowers all of it into the compilation request ([CMP-01](../../03%20platform/14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else)). The compiler never does that work and never re-derives it.
- **Clean Manager** owns `cln`, resolves which compiler version a project is pinned to, and invokes the binary ([MGR-01](../manager/00-manager.md#mgr-01--one-front-door)). The compiler has no user-facing command surface of its own.
- **The hosts** load the emitted component and verify it against their own contract at instantiation ([HCV-04](../../03%20platform/16-host-contract-validation.md#hcv-04--the-scope-split-who-validates-what)). The compiler never runs what it emits.
- **Libraries** contribute compile-time block handlers that the compiler *executes* in a sandbox but does not *contain* ([Architecture Boundaries §2.1](../../01%20governance/01-architecture-boundaries.md), [ADR-0004](../../01%20governance/decisions/0004-block-handler-execution-model.md)).

The compiler is a binary and a library. It is not a build system, not a package manager, and not a runtime.

---

## 2. Scope

**What lives in this component:**

- The Clean front end: lexing, parsing, name resolution, type checking.
- Execution of library-declared block handlers in the compile-time sandbox.
- Lowering to intermediate representations and optimization.
- World validation of every host-function call site.
- Component Model emission: a core module, wrapped as a component with the target world's WIT attached.
- Diagnostics: every finding the passes above produce, in the format [Platform 13](../../03%20platform/13-diagnostic-format.md) defines, carrying codes from the registry in [Platform 09](../../03%20platform/09-error-codes.md).
- The language server (§9).

**What does not live here, and where it lives instead:**

| Not here | Home |
|---|---|
| Reading `clean.toml`, walking the project, resolving dependencies | Clean Framework ([Architecture Boundaries §2.4](../../01%20governance/01-architecture-boundaries.md)) |
| The `cln` command surface and version resolution | Clean Manager ([MGR-01](../manager/00-manager.md#mgr-01--one-front-door)) |
| The build cache | Clean Framework ([14 §14.15](../../03%20platform/14-compiler-architecture.md#1415-external-build-cache)) |
| Fetching or validating a concrete host's contract | Clean Framework and the hosts ([HCV-04](../../03%20platform/16-host-contract-validation.md#hcv-04--the-scope-split-who-validates-what)) |
| Executing the emitted component | The hosts ([Hosts](../hosts/README.md)) |
| Domain meaning of `data:`, `endpoints:`, `canvas:` and other blocks | Library source, via block handlers ([BLK-](../../04%20language/21-block-handlers.md)) |

---

## 3. Design rules

These are the invariants that make the component what it is. Breaking one is a redesign, not a patch.

### CCMP-01 — The component is a pure function of its request

The compiler MUST derive every behaviour from the compilation request it was handed and MUST NOT read ambient state — no filesystem discovery, no network, no registry, no environment beyond the single variable determinism requires. This is [CMP-01](../../03%20platform/14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else) stated as a component boundary: anything the compiler needs that is not in the request is a defect in the caller, never a reason for the compiler to go looking.

**Why:** purity is what makes the compiler testable without a machine, drivable by an agent or a CI job as easily as by a developer, and reproducible years later from a manifest alone. A single ambient read forfeits all three at once.

### CCMP-02 — Determinism is a shipped property, not a best effort

Byte-identical requests MUST produce byte-identical artifacts, on every platform declared reproducible ([CMP-02](../../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out)). The component ships the test suite that proves it, and a determinism regression is a release blocker rather than a known issue.

**Why:** every downstream guarantee — the build cache ([CMP-06](../../03%20platform/14-compiler-architecture.md#cmp-06--a-cache-hit-must-be-byte-identical-to-a-cache-miss)), build reproduction, request replay, supply-chain attestation — is a corollary of this one property. Nothing above it survives its loss.

### CCMP-03 — The compiler validates against the world it is handed, and never fetches one

The compiler MUST validate the program against the target world delivered in the request, and MUST NOT fetch, download, resolve, or infer a host contract from any other source ([CMP-03](../../03%20platform/14-compiler-architecture.md#cmp-03--every-import-is-verified-against-the-world-in-the-request), [HCV-04](../../03%20platform/16-host-contract-validation.md#hcv-04--the-scope-split-who-validates-what)). A request that fails to carry a world is a malformed request, not an invitation to find one.

**Why:** this is the single boundary most likely to be eroded by a well-meaning convenience — "just fetch the host WIT if it's missing" would move host trust decisions into the component least able to make them, and would silently break CCMP-01.

### CCMP-04 — No user-facing command surface

The compiler binary MUST NOT be documented, taught, or scripted as a user-facing command; every developer-visible verb belongs to `cln` ([MGR-01](../manager/00-manager.md#mgr-01--one-front-door)). Operations this component implements are named in [14 §14.14](../../03%20platform/14-compiler-architecture.md#1414-compiler-api-operations--v1-requirements) as API operations, and the `cln` verbs that reach them are Manager's to define.

**Why:** version pinning only works if invocation goes through the resolver. A documented direct invocation is a documented way to bypass it.

### CCMP-05 — Failure is total and diagnostic

A failed compilation MUST leave no partial component behind and MUST explain itself in structured diagnostics rather than a message ([CMP-05](../../03%20platform/14-compiler-architecture.md#cmp-05--outputs-are-all-or-nothing-and-land-only-where-the-caller-pointed), [CMP-04](../../03%20platform/14-compiler-architecture.md#cmp-04--internal-failures-are-com013-never-a-user-error)). An internal invariant breach is presented as a compiler bug, never as the developer's error.

**Why:** a half-written artifact is worse than none — it is an artifact something downstream will eventually try to load. And a compiler that blames the user for its own broken invariant sends every one of those reports to the wrong place.

---

## 4. Inputs and outputs

The compiler accepts one compilation request document and produces one artifact set. Both shapes are owned by Platform 14 and are not restated here:

- **Input** — the request document: [14 §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs). It carries sources with their hashes, fully-resolved build configuration, the target world contract (`target_world`), the library manifest closure, compile limits, and the override audit trail.
- **Output** — the artifact set: [14 §14.1.2](../../03%20platform/14-compiler-architecture.md#1412-outputs). On success: the component, the build manifest, diagnostics, and (profile permitting) a source map. On failure: diagnostics and a non-zero exit.

The component is invocable as a library and as a process, both over the same request ([14 §14.2](../../03%20platform/14-compiler-architecture.md#142-invocation-surface)). Neither is a user surface (CCMP-04).

---

## 5. What it emits

The compiler emits a **Component Model component**, not a bare core module.

- The emission step produces a core WebAssembly module and then wraps it as a component, attaching the target world's WIT so the runtime can verify conformance at instantiation ([14 §14.4.2](../../03%20platform/14-compiler-architecture.md#1442-detailed-pass-responsibilities) pass [10], [15 §6.1](../../03%20platform/15-component-model-architecture.md#61-compile)).
- Every import in the emitted component is interface-qualified in the one naming scheme the ecosystem uses; no other form is permitted anywhere ([CMOD-01](../../03%20platform/15-component-model-architecture.md#cmod-01--one-wit-naming-scheme-extended-only-by-adr)), at the version baseline [CMOD-02](../../03%20platform/15-component-model-architecture.md#cmod-02--every-clean-package-sits-at-the-08-80-baseline) fixes.
- The emitted component is what the host conformance gate ([CMOD-03](../../03%20platform/15-component-model-architecture.md#cmod-03--conformance-is-the-shipping-gate-for-hosts)) is run against; the compiler is upstream of that gate, not a participant in it.

The compiler emits imports; it never emits implementations of them ([15 §3 P2](../../03%20platform/15-component-model-architecture.md#p2-the-compiler-emits-imports-never-implementations)).

---

## 6. World validation

Before codegen, the compiler checks every `host function` call site against the target world and refuses to proceed if any call site is not in it — `COM012`, aborting before any component is emitted ([CMP-03](../../03%20platform/14-compiler-architecture.md#cmp-03--every-import-is-verified-against-the-world-in-the-request)).

The scope split this sits inside is [HCV-04](../../03%20platform/16-host-contract-validation.md#hcv-04--the-scope-split-who-validates-what), and it is worth stating plainly because the middle role is the one that gets misread:

| Actor | Obligation |
|---|---|
| Framework | Fetches host WIT; runs the pre-compile checks (`COM014`, `COM015`) |
| **Compiler** | **Validates the program against the world handed to it in the request (`COM012`); fetches nothing** |
| Host | Ships its own contract and refuses a non-conforming component at load (`COM017`) |

The world the compiler validates against arrives as `target_world` in the request — the host's WIT carried by value, with `target_world.world` naming which world inside it applies ([14 §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs), [ADR-0033](../../01%20governance/decisions/0033-target-world-in-compilation-request.md)). A request without that field is refused with `RQD002`; there is no fallback that compiles without a world.

---

## 7. What the compiler does not do

Enumerated, because an unstated boundary is a boundary that erodes:

- **[CCMP-06]** Does not read the filesystem for inputs. Sources arrive in the request; no discovery, no walk, no include path.
- **[CCMP-07]** Does not access the network, for any purpose, at any point in a compilation.
- **[CCMP-08]** Does not resolve dependencies or select versions. It validates the manifest closure it was given against the declared dependencies; solving happens in the caller.
- **[CCMP-09]** Does not read `clean.toml` or any other configuration file. It reads the resolved projection of one, delivered in the request.
- **[CCMP-10]** Does not fetch, cache, or validate a concrete host's contract (CCMP-03).
- **[CCMP-11]** Does not run the component it emits, and holds no runtime, host, or bridge implementation.
- **[CCMP-12]** Does not implement any host function, in any world.
- **[CCMP-13]** Does not know about project folder structure, page/route/model discovery, or any Frame layout convention.
- **[CCMP-14]** Does not contain domain logic for any library block. It executes the library's block handler; it does not reimplement what that handler computes ([Architecture Boundaries §4](../../01%20governance/01-architecture-boundaries.md)).
- **[CCMP-15]** Does not maintain the build cache, or consult one. Caching lives in the framework and is invisible to the compiler ([14 §14.15](../../03%20platform/14-compiler-architecture.md#1415-external-build-cache)).
- **[CCMP-16]** Does not write outside the output directory the caller named, and does not mutate its inputs ([CMP-05](../../03%20platform/14-compiler-architecture.md#cmp-05--outputs-are-all-or-nothing-and-land-only-where-the-caller-pointed)).
- **[CCMP-17]** Does not carry an incremental-compilation query engine, an on-disk cache, or a persistent database ([14 §14.11](../../03%20platform/14-compiler-architecture.md#1411-non-goals)).
- **[CCMP-18]** Does not accept native compiler plugins. Libraries influence compilation only through the block-handler contract.
- **[CCMP-19]** Does not emit for any target other than the WebAssembly Component Model.
- **[CCMP-20]** Does not scaffold projects, add dependencies, or perform any other project-management action ([14 §14.14](../../03%20platform/14-compiler-architecture.md#1414-compiler-api-operations--v1-requirements)).

---

## 8. Versioning and installation

- **[CCMP-21]** The compiler is installed, pinned, switched, and removed by Clean Manager, never by itself and never by a developer placing a binary on `PATH` ([Manager §00.3.3](../manager/00-manager.md#0033-toolchain-versions)).
- **[CCMP-22]** Installed versions live under the manager-owned layout; the compiler MUST NOT depend on being at any particular path, nor write anywhere outside what the caller specified (CCMP-16, [Manager §00.2](../manager/00-manager.md#002-on-disk-layout)).
- **[CCMP-23]** A project pins an exact compiler version, not a range, and that pin is what Manager resolves before every dispatch ([Manager §00.3.3](../manager/00-manager.md#0033-toolchain-versions)).
- **[CCMP-24]** A change in emitted bytes for an unchanged request is a semver event for this component, because it invalidates every cache entry and every reproduction keyed on the prior version ([CMP-02](../../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out), [14 §14.15.1](../../03%20platform/14-compiler-architecture.md#14151-key-structure)).

---

## 9. The language server

**The language server belongs to this component family.** It is built from the same source as the batch compiler, ships in the same distribution, and is resolved and launched at the project's pinned compiler version.

- **[CCMP-25]** The language server MUST share the compiler's lexer, parser, and type checker rather than reimplementing any of them — what the editor understands and what compiles are the same code, at the same version.
- **[CCMP-26]** The language server ships with the compiler and is resolved through Clean Manager at the project's pin; editor extensions MUST NOT bundle either ([LSP-05](../../03%20platform/04-ide-lsp-architecture.md#lsp-05--the-extension-never-bundles-a-compiler), [Platform 04 §4.6](../../03%20platform/04-ide-lsp-architecture.md#46-multi-version-compiler-support)).

The LSP contract — which capabilities are served, how diagnostics and code actions are threaded, what an extension may and may not do — is owned by [Platform 04](../../03%20platform/04-ide-lsp-architecture.md) and is not restated here. This component owns the implementation and its packaging; Platform 04 owns the protocol.

**Why this split:** the editor and the batch build must never disagree about what a program means, which makes co-location a correctness property rather than a packaging convenience. But the protocol is a contract with editors, whose other party is not this component — so it stays a rung up.

---

## 10. Reference implementation

The compiler is implemented as a library with a thin process adapter over it ([14 §14.3](../../03%20platform/14-compiler-architecture.md#143-implementation-packaging)). Internal structure, dependency choices, and their version pins are implementation decisions recorded in [ADR-0006](../../01%20governance/decisions/0006-compiler-reference-stack.md), not part of this specification.

The repository does not exist yet. When it does, its README documents API stability guarantees, as the other component repos do.

---

## 11. Open questions

- ~~**The request document has no field for the target world.**~~ **Closed 2026-08-11 by [ADR-0033](../../01%20governance/decisions/0033-target-world-in-compilation-request.md)** (Accepted). [14 §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs) now carries `target_world`: the host's `host.wit` verbatim, the name of the world within it to validate against, and the host name, resolved version, and hash it came from. Clean Framework populates it from the contract it already fetches at Moment 1; the compiler still fetches nothing. The related question of whether `build.target` names the world or carries it is answered the same way — the target names it, `target_world` carries it. The World Import Check (pass [9]) and CCMP-03 are implementable as of that amendment.
- **Whether the language server versions independently.** §9 pins it to the compiler's version. Whether a language-server-only fix may ship without a compiler release — and what that would mean for CCMP-25's shared-source guarantee — is undecided, and does not block a first release.
- **Where the block-handler sandbox's conformance suite lives.** Handlers are library code executed by this component under limits neither party owns alone ([ADR-0004](../../01%20governance/decisions/0004-block-handler-execution-model.md)). Whether the suite proving sandbox behaviour ships with the compiler or with the library system is unsettled.
- **Bridge stub distribution.** [14 §14.14.5](../../03%20platform/14-compiler-architecture.md#14145-library-author-testing--bridge-stub-components) puts bridge stub components in the compiler tarball while requiring them to live beside their interface WIT so drift is impossible. Those two placements are in tension; which one governs is undecided.

---

## 12. What building the rest of the toolchain established

Between 2026-08-10 and 2026-08-15, Clean Framework, Clean Manager, Clean Runtime, and Clean Cloud were built and tested end to end against a **compiler stand-in** implementing the process contract in §4 — `compile --stdout-tar` over the request document, `--version`, exit 0 on success and non-zero with diagnostics on failure. Sources reached a package, a package reached a host, and a real component served HTTP traffic. Only this component was substituted.

That is worth recording because a specification written before its dependents is usually wrong somewhere, and the useful question is *where*. This section is the answer, so an implementer knows which parts are load-bearing and which were never exercised.

### What held unchanged

- **CCMP-01 (pure function of the request).** Nothing downstream ever wanted the compiler to read a file. The framework's own rule that it must place every needed file into the request ([FRM-BO-02](../framework/11-build-orchestration.md#frm-bo-02--the-framework-never-lets-the-compiler-see-the-filesystem)) is what made hosted builds possible at all: Clean Cloud compiles for a browser-based caller by materialising sources into a scratch directory and invoking the framework, and the compiler's purity is why that is safe rather than a second code path.
- **CCMP-02 (determinism).** Load-bearing further downstream than this document claims. Package archives are content-addressed, so two builds of identical sources must produce identical bytes for Cloud's store to deduplicate them — and a developer pressing Test repeatedly is the case that made it matter. `SOURCE_DATE_EPOCH` is what the framework and Cloud both pin to achieve it, which makes §14.5's "sole environment variable" an external contract, not an internal detail.
- **CCMP-04 (no user-facing surface).** Held without strain. Every invocation in the shipped toolchain goes through the framework, which resolves the version from the project pin.
- **CCMP-05 (failure is total and diagnostic).** The framework surfaces the compiler's diagnostics verbatim rather than re-rendering them, and Cloud returns them unchanged to its callers. A failed compile is a `200` with `ok: false` at the HTTP boundary — a normal answer to a valid request. That only works because the failure is structured.

### What the specification did not say, and needed to

- **`--stdout-tar` is not optional for the process adapter.** [14 §14.1.2](../../03%20platform/14-compiler-architecture.md#1412-outputs) mentions it parenthetically; in practice it is the entire process contract, and a stand-in that accepted `compile` without it silently produced no artifact. An implementer should treat the flag as required rather than as a mode.
- **`--version` must be parseable by position.** Two components parse it: the framework records the compiler version in the build manifest, and Cloud's node agent probes the runtime's equivalent to satisfy package pins. Both take the last whitespace-separated token, so `clean-compiler 1.2.3` works and any format without the version last does not. Neither §14.14 nor this document says so.
- **The installed path is a contract, not a convention.** CCMP-22 says the compiler must not depend on being at a particular path — true of the binary, but the *caller* does depend on it: the framework resolves `<toolchain-root>/versions/compiler/<version>/clean-compiler`, where the root is `$CLN_HOME` or `~/.cln`. A distribution that installs elsewhere is not loadable, whatever the binary itself assumes.

### Not exercised, and therefore unproven

Everything here was stood in for and remains untested against a real implementation:

- Block-handler execution and the compile-time sandbox (CCMP-14, [ADR-0004](../../01%20governance/decisions/0004-block-handler-execution-model.md)). The limits are specified — `handler_timeout_ms`, `handler_memory_mb`, validated wasm, deterministic flags — and Clean Cloud's decision to run hosted builds behind a plain process boundary rather than a container **rests on those limits being real**. If the sandbox ships weaker than specified, that decision needs revisiting.
- World validation against `target_world` (CCMP-03, §6). The stand-in validated request integrity and source ordering, never call sites.
- Component Model emission (§5). The stand-in emitted a component preamble, and later a pre-compiled component, but never *produced* one from Clean source.
- Determinism across platforms (CCMP-02). Proven only for a stand-in whose output was trivially deterministic.

### One defect found, at the source

[ADR-0004](../../01%20governance/decisions/0004-block-handler-execution-model.md) cites `14 §14.13.2` for the reference sandbox configuration. That subsection does not exist: §14.13's subsections were replaced by a citation of [ADR-0006](../../01%20governance/decisions/0006-compiler-reference-stack.md) in the 2026-08-01 remediation pass, and the ADR's citation was not updated with them. The limits themselves are specified in [14 §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs) and pass [6], so nothing is unenforced — but an implementer following the citation finds nothing.

---

## Contract participation

This component participates in the host contract defined in [Platform 16](../../03%20platform/16-host-contract-validation.md) in exactly one role, defined by [HCV-04](../../03%20platform/16-host-contract-validation.md#hcv-04--the-scope-split-who-validates-what):

- It validates the program against the target world delivered in the request, emitting `COM012` on any call site the world does not contain (CCMP-03, §6).
- It embeds the project WIT into the emitted component, which is the artifact the host compares against its own contract at load time.
- It performs no other check in the chain. Moments 1 and 2 belong to the framework; Moment 3 belongs to the host.

Interface-level obligations and the framework and host halves of the split are owned by Platform 16 and are not restated here.

---

## Metadata

- **Status:** Draft
- **Kind:** Semantic Rules (spec chapter)
- **Audience:** Compiler implementers; framework, manager, and host authors working against the compiler boundary
- **Rule prefix:** `CCMP-`
- **Part of:** [Clean Language Specification — Components](../README.md)
- **Cites grammar:** none
- **Cites schema:** none — the request document and artifact shapes are owned by [14 §14.1](../../03%20platform/14-compiler-architecture.md#141-inputs-and-outputs)
- **References:** [Platform 14 — Compiler Architecture](../../03%20platform/14-compiler-architecture.md), [Platform 15 — Component Model Architecture](../../03%20platform/15-component-model-architecture.md), [Platform 16 — Host Contract Validation](../../03%20platform/16-host-contract-validation.md)
- **Last reconciled:** 2026-08-15 — §12 added after the framework, manager, runtime, and Cloud were built end to end against a compiler stand-in. Three under-specified points recorded (`--stdout-tar` as a required flag rather than a mode; `--version` parsed by trailing token; the installed path as a caller-side contract), four rules confirmed load-bearing, four areas marked unproven. The dangling `14 §14.13.2` citation in [ADR-0004](../../01%20governance/decisions/0004-block-handler-execution-model.md) was repaired at its source.
