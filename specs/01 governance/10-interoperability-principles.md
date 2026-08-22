# Interoperability Principles — Clean Language

Clean Language compiles to a portable component that a host loads and runs, and the seam between that component and everything outside it — other components, the host process, external systems, the developer's toolchain — is where interoperability lives or dies. The ten principles in this document define that seam: the interface is always machine-checkable and versioned, hosts are swappable by construction, capabilities travel through function signatures, and nothing about the language quietly changes shape because of who happens to be loading it. Part 0 names the boundaries in scope and the ones deliberately excluded, so every INTEROP-NN can be checked against a concrete seam rather than an unbounded notion of "compatibility."

---

## Part 0 — What "interoperable" means for Clean Language

Clean Language compiles to a portable component format that is loaded and executed by hosts. Every boundary between a component and anything outside it is described by a **machine-checkable interface language**, and composition of components inside a host follows a **declared composition model**. The concrete technologies that fill these roles today (compilation target, boundary-description language, composition model, versioning scheme) are foundational technology choices recorded in [ADR-0022](decisions/0022-foundational-technology-stack.md); the principles here are the durable rules those technologies exist to satisfy.

Interoperability principles answer questions that Language, Security, and Performance principles do not:

- *"What is the seam between a Clean component and everything outside it?"* — A machine-checkable interface, versioned and verified mechanically ([C-15](05-concerns.md)).
- *"What can a host provide, and what must it not assume about its component?"* — Anything declared in the interface contract; nothing else.
- *"Can a new host run every existing Clean component?"* — Yes, iff it implements the interface contract at the declared version. That property is not a nice-to-have; it is the definition of "a Clean host" ([C-16](05-concerns.md)).
- *"Can a component detect which host it's running on and behave differently?"* — No. That would defeat swappability and turn host-specific quirks into implicit contracts.

### Trust boundaries around the interface contract

| Actor | What they can rely on | What they cannot rely on |
|-------|----------------------|--------------------------|
| **Compiled Clean component** | Every function declared in its target interface, at the pinned version | Any host-specific behaviour outside the interface; any other component's memory; the identity of the host |
| **Host implementor** | Component imports exactly what its target interface declares; component memory layout follows the platform memory model | Any Clean-language-specific implementation detail leaking across the bridge; components sharing memory with the host |
| **Library author** | Declared bridge signatures as the boundary with the host | Any host-specific behaviour behind those signatures; any assumption about which host is loaded |
| **External system** (database, network peer, filesystem) | Contract-typed data structures the framework hands them; nothing about the component that produced them | Any host-specific extension protocol; direct component invocation |

### In-scope boundaries

1. **Component ↔ host boundary.** The interface contract, memory model, capability passing, versioning.
2. **Component ↔ component boundary.** How two Clean components in the same host address each other; how a component composed of multiple modules links.
3. **Library ↔ host bridge.** How a Clean library declares host functions it needs; how those declarations combine into a target interface.
4. **Toolchain ↔ external toolchain.** How the Clean toolchain interoperates with build systems, package managers, IDEs, CI, and container runtimes it does not own.
5. **Data-format boundaries.** How Clean's standard-library parsers and serializers interoperate with established external formats.

### Out of scope

- **Foreign-function interface to arbitrary native libraries.** The boundary is the interface contract, always. Host implementors MAY provide bindings to native code inside their host process, but the component sees only the declared interface.
- **Component sharing internal state.** Components communicate through host-mediated capabilities, not by touching each other's linear memory.
- **Language-level RPC or serialization protocols.** Clean does not ship a proprietary wire protocol. External communication uses standard protocols implemented on top of the host bridge.

---

## Part 1 — The Principles

Each principle names the concern, LANG principle, or SEC principle it exists to preserve. Interoperability principles are not aspirational — they are the specific consequences of the foundational choice of a portable-component + machine-checkable-interface stack ([ADR-0022 §§1–3](decisions/0022-foundational-technology-stack.md)). Abandoning any of them would abandon that choice.

#### INTEROP-01 — The interface contract is the boundary, always

*(Preserves: [C-15](05-concerns.md), [C-16](05-concerns.md), [SEC-01](08-security-principles.md); Addresses: C-06, C-08)*

Every boundary between a Clean component and anything outside it MUST be described by a machine-checkable interface, at a declared version. There MUST NOT be a second, out-of-band channel — no shared memory with the host outside the memory model, no side-channel via ambient state, no undeclared host imports the component "just happens" to know about.

A capability the component holds is a function in its imports. A value the component returns is a type in its exports. If a behaviour cannot be expressed in the interface language, it MUST NOT be part of the component's contract with the outside world.

**Why this principle exists.** [C-15](05-concerns.md) says host correctness is verified mechanically against the interface. That verification is worthless if half the contract lives outside it. Same for [SEC-01](08-security-principles.md): capabilities cross the boundary explicitly. An undeclared side-channel is undeclared authority.

The specific interface language is recorded in [ADR-0022 §2](decisions/0022-foundational-technology-stack.md); the concrete host-bridge mechanics live in [03 platform / 02 — Host Bridge](../03%20platform/02-host-bridge.md).

Source: [03 platform / 02 — Host Bridge](../03%20platform/02-host-bridge.md); [ADR-0022 §2](decisions/0022-foundational-technology-stack.md).

#### INTEROP-02 — Hosts are swappable by construction

*(Preserves: [C-16](05-concerns.md); Addresses: C-15)*

Any host implementation that implements every function in a target interface at the declared version MUST be able to run every Clean component built for that interface, without modification to the component. "It happens to run on the reference host" is not a valid definition of a Clean component — "it runs on any conforming host" is.

Components MUST NOT be able to detect which host they are running on. Host identity is not part of the interface contract, and any behaviour that varies with it is a defect on the host side (a host adding unadvertised behaviour) or the component side (a component probing for it).

New hosts join the ecosystem by implementing the interface contract. They do not require changes to the compiler, framework, or manager.

**Why this principle exists.** Swappability is the defining property of the composition model recorded in [ADR-0022 §3](decisions/0022-foundational-technology-stack.md). Without INTEROP-02, "Clean Language" would silently collapse into "Clean Language on the reference host" — the ecosystem would fragment along host boundaries. [C-16](05-concerns.md) is not a suggestion; INTEROP-02 is what makes it enforceable.

Source: [C-16](05-concerns.md); [02 components / hosts](../02%20components/hosts/); [ADR-0022 §3](decisions/0022-foundational-technology-stack.md).

#### INTEROP-03 — Interface contracts are versioned; evolution is explicit

*(Preserves: [C-15](05-concerns.md), [C-05](05-concerns.md); Addresses: C-06)*

Every interface MUST carry a version. A component pins the versions of the interfaces it targets; a host declares the versions it exports. A component and a host are compatible iff their pinned versions resolve under the documented compatibility rules.

A breaking change to an interface (removed function, changed signature, semantic redefinition) MUST become a new major version. A backward-compatible addition MUST become a new minor version. Silent evolution — changing what a function does without a version bump — is a defect that breaks every downstream contract at once.

Deprecation is a first-class step: a function marked deprecated in version N MUST still work; it MAY be removed only in the next major version.

**Why this principle exists.** [C-15](05-concerns.md) requires mechanical verification; that verification needs a stable target. [C-05](05-concerns.md) promises library authors that a compile-time function written today still works on future compiler versions within the same major — the same guarantee, applied to interface contracts, keeps hosts and components from drifting apart silently.

The specific versioning scheme is recorded in [ADR-0022 §4](decisions/0022-foundational-technology-stack.md); the compatibility resolution rules live in [03 platform / 08 — Bridge Versioning](../03%20platform/08-bridge-versioning.md).

Source: [03 platform / 08 — Bridge Versioning](../03%20platform/08-bridge-versioning.md); [ADR-0022 §4](decisions/0022-foundational-technology-stack.md); [C-05, C-15](05-concerns.md).

#### INTEROP-04 — Host-specific behaviour lives in the host, never in the language

*(Preserves: [C-11](05-concerns.md), [C-16](05-concerns.md); Addresses: C-08)*

The compiler MUST NOT contain code paths that vary based on the target host. The framework MUST NOT expose Clean-language primitives whose semantics change based on which host loads the component. Host-specific behaviour MUST be reached only through libraries whose bridge is declared in that host's target interface.

If a Clean program uses only interfaces from the core language contract, it MUST behave identically on every host that implements that contract. Host-specific programs opt in visibly, by importing a host-specific library that declares the host functions it needs.

**Why this principle exists.** [C-08](05-concerns.md) says the compiler is a pure function of its inputs — it does not know or care about hosts. [C-11](05-concerns.md) says the framework is an orchestrator, not a compiler; it composes interface targets, it does not fork them. If host-specific behaviour leaked into the language itself, both properties would be lost.

The specific set of interface targets and the libraries that opt into host-specific ones are enumerated in [02 components / framework](../02%20components/framework/) and the corresponding library specifications.

Source: [C-08, C-11, C-16](05-concerns.md); [02 components / framework / 01 — Framework Specification](../02%20components/framework/01-framework-specification.md).

#### INTEROP-05 — Capabilities cross the boundary through function signatures

*(Preserves: [SEC-01](08-security-principles.md), [SEC-10](08-security-principles.md); Addresses: C-06)*

A component's access to anything outside itself — filesystem, network, database, wall clock, randomness, secrets — MUST arrive as a function in its interface imports, or as a parameter passed through such a function. There MUST NOT be an ambient capability that the component acquires just by being loaded.

Composing components composes capabilities: if component A imports a capability and component B does not, wiring B to receive only what A exports MUST NOT grant B that capability transitively unless it is passed explicitly.

The framework MAY provide ergonomic sugar (per-request injection, dependency scoping), but the underlying model at the interface layer MUST remain "capability passed explicitly."

**Why this principle exists.** [SEC-01](08-security-principles.md) and [SEC-10](08-security-principles.md) require that authority be visible at the call site. The interface contract is the only place that guarantee can be enforced mechanically across the whole ecosystem; if capabilities leaked around the interface, both SEC principles would be defensible only by convention.

Source: [SEC-01, SEC-10](08-security-principles.md); [03 platform / 02 — Host Bridge](../03%20platform/02-host-bridge.md).

#### INTEROP-06 — Composition is a first-class operation

*(Preserves: [C-11](05-concerns.md); Addresses: C-06)*

Two Clean components in the same host MUST be composable through the composition model's declared operations: A's exports connect to B's imports through interface-typed wires. The framework MUST support this composition without a custom protocol layered on top.

A program's shape at composition time MUST be inspectable — which components are wired together, at which interface versions — through the framework's introspection surface.

Components MUST NOT communicate through Clean-specific back-channels (a shared magic global, a proprietary IPC, a language-level RPC). If two components need to talk, the composition wires them through the declared interface.

**Why this principle exists.** [C-11](05-concerns.md) says the framework orchestrates; it doesn't invent parallel communication mechanisms. Every back-channel added is a place the framework has to know about and every other tool (IDE, introspection server, debugger) has to learn about. The composition model already provides the mechanism; the framework MUST use it.

The specific composition model is recorded in [ADR-0022 §3](decisions/0022-foundational-technology-stack.md); the framework's introspection surface is specified in [02 components / framework / 10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md).

Source: [C-11, C-12](05-concerns.md); [03 platform / 02 — Host Bridge](../03%20platform/02-host-bridge.md); [ADR-0022 §3](decisions/0022-foundational-technology-stack.md).

#### INTEROP-07 — External data formats meet developers where they are

*(Preserves: [LANG-01](07-language-principles.md), [LANG-04](07-language-principles.md); Addresses: C-01)*

Where Clean interacts with an established external format, the standard library's parsing and serialisation MUST produce output that is *conformant* to the external standard — not a Clean-flavoured near-miss. A document produced by Clean MUST be accepted by every conforming implementation of that standard.

Where the standard is ambiguous or has multiple accepted dialects, the standard-library module MUST document its choice explicitly (see the conformance-testing requirement in the testing chapter) and MUST NOT silently mix dialects.

Clean-specific extensions to external formats are forbidden. If a feature Clean wants does not exist in the external standard, the answer is a Clean-native construct that compiles down to the standard, not an extension of the standard.

**Why this principle exists.** Interoperability with the rest of the world is the reason developers can trust Clean at a boundary. A serializer that emits "almost-standard" output would make every external consumer a bug report. [LANG-01](07-language-principles.md) applies past the Clean boundary too: what the developer reads in an external spec is what they get from the Clean standard library.

The specific external standards Clean interoperates with (data formats, network protocols, query languages), the dialects chosen, and the conformance-testing regime are defined in [04 language / 11 — Testing](../04%20language/11-testing.md) and [04 language / 15 — Standard Library](../04%20language/15-standard-library.md).

Source: [04 language / 11 — Testing](../04%20language/11-testing.md); [04 language / 15 — Standard Library](../04%20language/15-standard-library.md).

#### INTEROP-08 — The toolchain interoperates through documented file formats

*(Preserves: [C-03](05-concerns.md), [C-04](05-concerns.md); Addresses: C-14)*

The Clean toolchain MUST NOT require a Clean-specific IDE, shell, CI, or build orchestrator. Every input and output the toolchain touches is either (a) a standard format documented outside Clean, or (b) documented in the specification with a stable schema.

The developer's existing tools (editors, terminals, CI systems, artifact registries) MUST work with Clean without Clean-specific plugins for basic operations. Plugins MAY enhance the experience; they MUST NOT be a precondition for it.

**Why this principle exists.** [C-03](05-concerns.md) says a single command surface is the entire user surface — but that only works if the developer's *other* tools already speak the formats Clean produces. Otherwise the developer is trapped in a walled garden, and [C-04](05-concerns.md)'s reproducibility promise becomes "reproducible if you use the same setup as us."

The specific formats and protocols the toolchain uses (build configuration, diagnostic format, language-server protocol, model-context protocol, compiled-artifact format, container-artifact format) are enumerated in [03 platform / 07 — Build Configuration](../03%20platform/07-build-config.md) and [03 platform / 13 — Diagnostic Format](../03%20platform/13-diagnostic-format.md).

Source: [C-03, C-04, C-14](05-concerns.md); [03 platform / 07 — Build Configuration](../03%20platform/07-build-config.md); [03 platform / 13 — Diagnostic Format](../03%20platform/13-diagnostic-format.md).

#### INTEROP-09 — Every host bridge declaration is expressible in Clean and the manifest

*(Preserves: [C-07](05-concerns.md); Addresses: C-05)*

Everything a library needs to declare to add host functions — signatures, target interfaces, versions, resource types — MUST be expressible in the library manifest and Clean source. Hand-written interface-language files are not part of the authoring surface for library authors; the framework generates the interface contract from the library's declarations.

Library authors do not learn the underlying interface language to ship a library. They learn Clean and the library-manifest schema. The framework takes on the translation, and the toolchain verifies that the generated contract matches what the host actually exports (see [C-15](05-concerns.md)).

**Why this principle exists.** [C-07](05-concerns.md) requires the library authoring surface to be complete without hand-written interface files. If any interop concern required dropping to raw interface syntax, the library ecosystem would fork between "regular libraries" and "libraries with host bridges," and only specialists could ship the latter.

The specific manifest schema and the generation pipeline are defined in [02 components / framework / 09 — Libraries Specification](../02%20components/framework/09-libraries-specification.md).

Source: [C-07](05-concerns.md); [02 components / framework / 09 — Libraries Specification](../02%20components/framework/09-libraries-specification.md).

#### INTEROP-10 — Interop failure is a first-class diagnostic

*(Preserves: [C-02](05-concerns.md), [SEC-09](08-security-principles.md); Addresses: C-15)*

When a component and a host fail to satisfy each other — interface version mismatch, missing export, capability the component imports but the host does not provide, type mismatch at the boundary — the toolchain MUST report which side is missing what, with a stable diagnostic code from the platform's error registry, before the component is instantiated. Silent partial loading, best-effort binding, or "it worked but that function traps on first call" are defects.

The developer reading the error MUST be able to tell (a) which side (component or host) is off-spec, (b) which interface is at fault, and (c) what version resolution would fix it.

**Why this principle exists.** [C-02](05-concerns.md) requires errors to tell the developer what to do next. Interop failures are the errors most likely to be blamed on "the framework is broken" when they are actually version mismatches — an unclear diagnostic here poisons trust in the whole ecosystem.

The specific diagnostic codes and the pre-instantiation validation pipeline are defined in [03 platform / 09 — Error Codes](../03%20platform/09-error-codes.md) and [03 platform / 16 — Host Contract Validation](../03%20platform/16-host-contract-validation.md).

Source: [C-02, C-15](05-concerns.md); [03 platform / 09 — Error Codes](../03%20platform/09-error-codes.md).

---

## Part 2 — How interoperability principles relate to other principles

INTEROP principles sit alongside LANG, SEC, and PERF — not above or below them. Each INTEROP-NN preserves a specific commitment from Concerns, LANG, or SEC, in the same way PERF principles do. The tiebreak rules that follow are consequences of that framing.

1. **INTEROP-NN and LANG-NN cannot conflict at the principle level.** INTEROP principles exist to make LANG principles (particularly [LANG-01](07-language-principles.md), [LANG-03](07-language-principles.md), [LANG-04](07-language-principles.md)) true across the boundary, not just inside a single Clean program. A perceived conflict means the implementation is failing to bridge both properly; the fix is in the implementation.

2. **INTEROP and SEC reinforce each other.** [SEC-01](08-security-principles.md) (sandboxed by default) and [SEC-10](08-security-principles.md) (authority through signatures) are the security manifestation of INTEROP-01 and INTEROP-05. A change that weakens one weakens the other; a proposal to add an ambient host capability MUST address both docs.

3. **INTEROP-01 (interface contract is the boundary) is non-negotiable within the current foundational stack.** Every other INTEROP principle follows from it. A proposal to add an out-of-band channel across the component/host boundary is a proposal to abandon the composition model recorded in [ADR-0022 §3](decisions/0022-foundational-technology-stack.md), and requires an ADR that supersedes INTEROP-01 and the relevant ADR-0004 subsection explicitly.

4. **A new INTEROP principle MUST name the concern, LANG, or SEC principle it preserves.** A principle with no such trace is decoration — remove it, or elevate the underlying commitment first.

5. **When a spec chapter is amended in a way that changes a boundary contract**, the chapter MUST cite the INTEROP-NN principle it derives from, alongside the LANG-NN, SEC-NN, and concern citations ([DOC-14](00-documentation-principles.md#doc-14)).

6. Retiring an INTEROP principle MUST NOT reuse its ID. The retired principle keeps its number and is marked *Withdrawn (YYYY-MM-DD, ADR-NNNN)*, with the ADR explaining what changed about the foundational stack, the host ecosystem, or the underlying commitment.

7. **Principles state policy; specs state mechanics; [ADR-0022](decisions/0022-foundational-technology-stack.md) records the foundational technology choices** (compilation target, boundary description language, composition model, versioning scheme, toolchain command surface, artifact layout, reproducibility mechanism). An INTEROP principle that names a specific interface language, composition-model term, format name, protocol, or product is a defect — that content belongs in the ADR or the spec chapter. Rewording forced by an ADR or spec change MUST NOT change the policy; if it does, an ADR that retires the affected principle is required.

---

## Metadata

- **Status:** Draft
- **Audience:** Compiler and framework maintainers, host implementors, library authors declaring host bridges, and reviewers of any change that touches boundary contracts, the host bridge, or the composition model
- **Rule prefix:** `INTEROP-`
- **References:** [Language Principles](07-language-principles.md); [Security Principles](08-security-principles.md); [Architectural Concerns](05-concerns.md); [ADR-0022 — Foundational Technology Stack](decisions/0022-foundational-technology-stack.md)
