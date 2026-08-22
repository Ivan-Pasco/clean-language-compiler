# Security Principles — Clean Language

Clean Language commits to a specific way of behaving under adversarial conditions — sandboxed by default, capabilities passed through function signatures, secrets that redact themselves in every default representation, artifacts that never load without a checksum match. This document is the ten durable rules those commitments come down to, and the threat model each rule is answering. Part 0 names the actors, surfaces, and things explicitly out of scope, so that every SEC-NN in Part 1 can be checked against a concrete adversary rather than pattern-matched against a vibe of "seems secure."

---

## Part 0 — Threat model

Every security principle below is a response to a concrete adversary and a concrete surface. Naming them here is what makes each SEC-NN checkable rather than aspirational. Rules that address no actor and no surface in this model MUST be removed.

### Trust boundaries

| Actor | Trust level | Why |
|-------|-------------|-----|
| **Language user** (developer writing Clean code) | Trusted-but-fallible | Owns the machine, wrote the code. Protected *from their own mistakes*, not from themselves. |
| **Host implementor** (server, browser, CLI runtime) | Trusted | Loads and executes Clean components. A malicious host defeats every other guarantee; hosts are audited, not sandboxed. |
| **Compiler and framework maintainers** | Trusted | Ship the toolchain. Protected by governance (ADR review, signed releases), not by runtime checks. |
| **Library author** | Semi-trusted | Distributed code that other developers pull in. Assumed non-malicious by default but not blindly trusted; libraries are constrained by what they can declare in their manifest and their sandboxed compile-time handlers. |
| **Compile-time handler** (DSL block handler running at compile time) | Untrusted at execution time | Runs inside the compiler. Even a well-intentioned handler is executed in a sandbox with hard time and memory budgets and no I/O. |
| **On-disk artifact** (downloaded component, plugin, library) | Untrusted until verified | Anything fetched over the network into the toolchain's local storage MUST be checksum-verified against a lockfile before use. |
| **Runtime input** (network request, file contents, user text) | Fully untrusted | Standard-library APIs that accept external data treat every byte as adversarial by default. |
| **Runtime peer** (another concurrent request, another background task) | Isolated | Requests, tasks, and tenants MUST NOT observe each other's state through language-level means. |

### In-scope surfaces

1. **Compile-time handler execution.** A malicious or buggy handler trying to read the compiler's environment, exhaust memory, spin forever, or corrupt the IR of an unrelated module.
2. **Component-to-host boundary.** Functions the component calls into the host: memory-safety of the boundary, capability leakage across the boundary, version drift between what a component declares and what the host provides.
3. **Artifact supply chain.** Everything the toolchain installs: tampered downloads, missing signatures, silent updates, dependency-confusion attacks on library names.
4. **Standard-library defaults.** APIs that touch untrusted input — their default behaviour, error messages, and logging must not create a class of vulnerability by accident.
5. **Secret material.** Values the developer marks as secret (or that the framework knows are secret: session tokens, API keys, passwords) MUST NOT appear in diagnostics, logs, or error payloads without an explicit opt-in.

### Out of scope

- **Malicious hosts.** A host that lies about the world it presents to a component defeats every language-level guarantee. Host correctness is verified by [C-15 (mechanical interface conformance)](05-concerns.md), not by runtime checks inside the component.
- **Physical access, side-channel attacks, and hardware faults.** Not a language-level concern.
- **Kernel-level malware on the developer's or operator's machine.** If the toolchain's local storage can be modified by an attacker with local root, the checksum lockfile is moot.
- **Cryptographic primitive design.** Clean Language does not invent primitives; it uses vetted libraries. Choosing which library is a policy question, not a language one.
- **Application-level authorisation.** Clean provides the tools; it does not enforce a particular access-control model.

---

## Part 1 — The Principles

### Security as Developer Experience

The framing that governs every principle below: **security failures on the golden path are DX failures.** A framework that lets a developer accidentally log a session token, ship a plugin with unbounded compile time, or run an unpinned dependency has failed the developer as badly as one with cryptic syntax. Every SEC-NN below either (a) makes the secure choice the default, or (b) makes the insecure choice visible and effortful.

Where security and DX genuinely conflict, the tiebreak in Part 2 applies: **inside the threat surface declared in Part 0, SEC wins; everywhere else, [LANG-01](07-language-principles.md) wins.**

#### SEC-01 — Sandboxed by default; capabilities cross the boundary explicitly

*(Addresses: C-05, C-08, C-15)*

Code that runs inside a sandbox — compile-time handlers, components running in a host — MUST have no ambient access to the outside world. Every external resource (filesystem, network, environment, time, randomness) is a capability that MUST be passed in through an explicit function signature, never implicitly available through globals or magic imports.

The default of every sandbox is empty: no I/O, no clock, no state. Every capability the sandbox is granted MUST be visible to a reader of the code and audit-able by tooling.

The specific boundary mechanism (imports, host bridge, capability types) is defined in [03 platform / 02 — Host Bridge](../03%20platform/02-host-bridge.md) and grounded in [ADR-0022 §3](decisions/0022-foundational-technology-stack.md).

Source: [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md); [03 platform / 02 — Host Bridge](../03%20platform/02-host-bridge.md); [LANG-19](07-language-principles.md).

#### SEC-02 — Compile-time handlers live inside hard budgets

*(Addresses: C-05, C-08)*

Every compile-time handler execution MUST be bounded by a compiler-enforced wall-clock time budget and a memory budget. Exceeding either MUST fail the compilation with a diagnostic that names the handler and the budget it hit — never a silent kill, never an unbounded wait, never an out-of-memory panic in the compiler process.

Budgets are set by the compiler and MUST NOT be raiseable by the handler itself. A project MAY raise them for a specific handler through the build configuration (surface visible; opt-in explicit); the compiler MUST refuse to silently accept implicit raises from library manifests.

The specific build-configuration surface and default budget values are defined in [03 platform / 07 — Build Configuration](../03%20platform/07-build-config.md).

Source: [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md); [LANG-19](07-language-principles.md).

#### SEC-03 — The compiler is a pure function of its inputs

*(Addresses: C-08, C-10)*

The compiler MUST NOT perform filesystem I/O outside its declared input paths, MUST NOT make network requests, MUST NOT read environment variables not declared in the build configuration, and MUST NOT execute code from libraries outside the sandboxed compile-time handler runtime.

Reproducibility is a security property: two builds on the same inputs, on the same platform, MUST produce byte-identical output. A build that varies with wall-clock time, ambient environment, or network availability is a defect and a supply-chain risk.

The specific input-declaration surface and reproducibility rules are defined in [03 platform / 07 — Build Configuration](../03%20platform/07-build-config.md); the reproducibility mechanism is recorded in [ADR-0022 §8](decisions/0022-foundational-technology-stack.md).

Source: [C-08, C-10](05-concerns.md); [03 platform / 07 — Build Configuration](../03%20platform/07-build-config.md).

#### SEC-04 — Every on-disk artifact is checksum-verified against a lockfile

*(Addresses: C-17, C-04)*

No artifact under the toolchain's local storage MUST be loaded, executed, or trusted without a checksum match against the project's lockfile. Signature verification is REQUIRED for core artifacts (compiler, framework, toolchain binary) and one-time-warned for community artifacts (third-party libraries) — a warning that a developer explicitly acknowledged, never a silent trust-on-first-use.

A missing lockfile is not a permissive default; it is a hard error that requires an explicit toolchain command to remedy.

The specific storage layout, lockfile format, and verification commands are defined in [02 components / manager / 00 — Manager](../02%20components/manager/00-manager.md); the layout and reproducibility mechanism are recorded in [ADR-0022 §§7–8](decisions/0022-foundational-technology-stack.md).

Source: [C-17](05-concerns.md).

#### SEC-05 — Nothing self-updates; every update is explicit and named

*(Addresses: C-19)*

No component of the ecosystem — compiler, framework, plugin, library, host runtime — updates itself in the background. The only update mechanism is an explicit user command, invoked by the developer or operator.

Auto-update-on-network-detected, "phone home" version checks, and silent background refetches are permanently forbidden. This applies equally to development machines, CI, and production hosts.

The specific update commands and their semantics are defined in [02 components / manager / 00 — Manager](../02%20components/manager/00-manager.md).

Source: [C-19](05-concerns.md).

#### SEC-06 — Untrusted input is untrusted by construction

*(Addresses: C-02)*

Standard-library APIs that receive data from an untrusted source MUST return typed values that carry their untrusted origin in their type or through a required parsing step. There MUST NOT be an implicit coercion from an untrusted value to a value used in a sensitive sink (SQL, HTML, filesystem path, shell command) without an explicit named conversion.

The compiler and framework SHOULD provide named safe sinks (parameter-binding query builders, auto-escaping HTML blocks, path-join helpers that reject traversal) so that the safe path is the shortest path. A developer choosing the unsafe path does so visibly and locally, where a reviewer can see it.

The specific typed-value scheme, safe-sink APIs, and named-conversion syntax are defined in [04 language / 15 — Standard Library](../04%20language/15-standard-library.md) and in the relevant library specifications under [02 components/framework/libraries/](../02%20components/framework/libraries/).

Source: [C-02](05-concerns.md); [04 language / 15 — Standard Library](../04%20language/15-standard-library.md).

#### SEC-07 — Secrets never leak through defaults

*(Addresses: C-02, C-17)*

Values known to the framework as secret (session tokens, API keys, password fields, or values declared secret in the type system) MUST NOT appear in:

- diagnostic messages,
- error payloads returned to a client,
- default log output,
- serialised debug representations of an object,
- crash reports or telemetry.

A secret can be surfaced only through a named, explicit operation whose call site is grep-able. The default string, debug, and serialisation representations of a secret MUST be a redacted placeholder.

SEC-07 is backed by the language-level `secret` type, adopted in [ADR-0023](decisions/0023-secret-handling-strategy.md). `secret` is a first-class, taint-tracked type: `env.get(name)` returns `secret` when `name` matches the secret name pattern (ending in `_SECRET`, `_TOKEN`, `_KEY`, or `_PASSWORD`), and taint propagates through assignment, passing, return, string interpolation, concatenation, and container membership. The sole declassification point is `.reveal() -> string` — a grep-able name chosen precisely so every call site is auditable. The default `toString()`, debug representation, and JSON serialisation of any `secret` value emit `"[REDACTED]"` without any framework opt-in.

The mechanics are defined in [04 language / 04 — Type System § The secret type](../04%20language/04-type-system.md) (taint rules) and [04 language / 15 — Standard Library § secret operations](../04%20language/15-standard-library.md) (reveal, equality, is_empty, worked examples). Auth-library signature updates — including `env.get("JWT_SECRET")` now returning `secret` — are documented in [02 components / framework / libraries / 01 — Auth §§ 6, 11](../02%20components/framework/libraries/01-auth.md). The error-reporting layer additionally applies byte-pattern heuristics to core dumps at `full-with-diagnostics` level as a defence-in-depth backup for bytes the compiler cannot track (FFI, unsafe bridges); see [03 platform / 06 — Error Reporting §6.6](../03%20platform/06-error-reporting.md).

Source: [C-02, C-17](05-concerns.md).

#### SEC-08 — Concurrent peers do not observe each other's state

*(Addresses: C-02)*

State declared at request scope, task scope, or tenant scope MUST NOT be observable across those scopes by language-level means. Two concurrent requests, two background tasks, two tenant partitions MUST NOT be able to read each other's state, in-flight results, or in-flight mutations even accidentally.

Global mutable state that would let peers cross-observe is a defect (already forbidden by [LANG-14](07-language-principles.md)); this principle extends that to per-request and per-task isolation as a security guarantee, not just a code-hygiene preference.

The specific scoping rules and isolation mechanics are defined in [04 language / 20 — State Management](../04%20language/20-state-management.md) and [04 language / 18 — Async](../04%20language/18-async.md).

Source: [04 language / 20 — State Management](../04%20language/20-state-management.md); [04 language / 18 — Async](../04%20language/18-async.md); [LANG-14, LANG-15](07-language-principles.md).

#### SEC-09 — Errors help the developer, not the attacker

*(Addresses: C-02)*

Diagnostic and runtime error messages MUST tell the *developer* what to do next while telling the *attacker* nothing exploitable.

- Stack traces, internal paths, generated queries, and configuration values MUST NOT appear in errors returned to a remote client by default. The framework MUST route detailed errors to logs and return a stable error code (see [C-02](05-concerns.md)) to the client.
- Timing-sensitive comparisons (password checks, token matches, signature verifications) MUST use constant-time helpers from the standard library; naïve equality on a secret is a defect.
- Rate-limit and lockout diagnostics MUST NOT distinguish "user does not exist" from "credential wrong" to a remote caller.

The developer's local run output MAY be verbose. Production defaults MUST NOT.

The specific constant-time helpers and production error-shaping rules are defined in [04 language / 15 — Standard Library](../04%20language/15-standard-library.md) and [03 platform / 06 — Error Reporting](../03%20platform/06-error-reporting.md).

Source: [C-02](05-concerns.md); [03 platform / 06 — Error Reporting](../03%20platform/06-error-reporting.md).

#### SEC-10 — Authority flows through function signatures, not globals

*(Addresses: C-05, C-08, C-11)*

A function's ability to touch the outside world (database handle, network client, filesystem, secret) MUST be visible in its signature. A reader looking at a function's parameter list can tell what capabilities it holds.

Ambient authority (global database connection, implicit HTTP client, thread-local secret) is a defect. This is the language-level foundation for auditability: if capability doesn't cross the boundary in the signature, the boundary isn't real.

The framework MAY provide ergonomic sugar (per-request injection, dependency scoping), but the underlying model MUST remain "capability passed explicitly." A developer reading a function they've never seen before MUST be able to answer "what can this reach?" from the signature alone.

Source: [04 language / 09 — Functions](../04%20language/09-functions.md); [C-05, C-11](05-concerns.md).

---

## Part 2 — How security principles interact with other principles

1. **Inside the threat surface declared in Part 0, SEC-NN overrides LANG-NN.** A security principle MAY require ceremony (explicit capability passing, named unsafe escapes, verbose error handling) that would otherwise be a LANG-01 violation. The ceremony is the point: making the risky path visible.
2. **Outside the threat surface, [LANG-01](07-language-principles.md) wins.** A security principle MUST NOT be used to justify cryptic syntax, hidden behaviour, or friction on the golden path when the surface it names is not actually in play.
3. **The threat surface is closed.** Adding a new surface (e.g. "cross-component provenance," "confidential computing enclaves") requires an ADR that extends Part 0 in the same commit as the SEC principle that guards it. A SEC-NN that addresses no surface in Part 0 is a defect.
4. When a spec chapter is amended in a way that changes an SEC-relevant default, the chapter MUST cite the SEC-NN principle it derives from, alongside the LANG-NN and concern citations ([DOC-14](00-documentation-principles.md#doc-14)).
5. Retiring an SEC principle MUST NOT reuse its ID. The retired principle keeps its number and is marked *Withdrawn (YYYY-MM-DD, ADR-NNNN)*, with the ADR explaining what changed about the threat model that made the principle obsolete.
6. **Principles state policy; specs state mechanics; [ADR-0022](decisions/0022-foundational-technology-stack.md) records foundational technology choices.** A principle that names a specific API, flag, file path, or command is a defect — that content belongs in the spec chapter or the ADR. Rewording forced by a spec or ADR change MUST NOT change the policy; if it does, an ADR that retires the affected principle is required.

---

## Metadata

- **Status:** Draft
- **Audience:** Compiler and framework maintainers, host implementors, library authors, and reviewers of any change that touches the compile-time handler runtime, the host bridge, on-disk artifacts, or default library behaviour
- **Rule prefix:** `SEC-`
- **References:** [Language Principles](07-language-principles.md); [Architectural Concerns](05-concerns.md); [ADR-0022 — Foundational Technology Stack](decisions/0022-foundational-technology-stack.md); [ADR-0023 — Secret Handling Strategy](decisions/0023-secret-handling-strategy.md)
