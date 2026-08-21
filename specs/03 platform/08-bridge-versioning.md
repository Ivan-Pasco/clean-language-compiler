# Platform 08. Bridge Versioning

Bridge interfaces evolve — WASI adds functions, Clean-specific packages gain new capabilities, libraries publish new versions of their host-function surfaces. This chapter defines how those evolutions happen without breaking programs already in production. The mechanism is **explicit versioning of every WIT package**, enforced at the compile boundary by the compiler and at the run boundary by the host: both check the same rules, so a program that links successfully is guaranteed to run against a matching host. The rules below are simple to state and expensive to violate — every V2 host, library, and toolchain must obey them.

---

## 8.0 V2 Baseline Versions


### BVER-01 — One baseline table


Every reference to a `clean:*` package version in the specification MUST use these baseline values. When a version is bumped, the value here MUST be updated first; downstream files re-quote from this table.

| Package | Baseline version |
|---------|------------------|
| `clean:bridge` | `@1.0.0` |
| `clean:host` | `@0.2.0` |
| `clean:library/<name>` | `@0.1.0` (per library, bumped independently thereafter) |
| Component Model level (LTS floor) | `0.3.0` (Preview 3 / Canonical ABI v2, WASI 0.3 ratified 2026-06-11) |

WASI package versions track upstream. As of the V2 floor: `wasi:filesystem@0.3.0`, `wasi:http@0.3.0`, `wasi:clocks@0.3.0`, `wasi:sockets@0.3.0`, `wasi:cli@0.3.0`, `wasi:random@0.3.0`. **Exception:** `wasi:logging` remains pinned to `@0.2.0` — no 0.3 cut has shipped upstream; when it does, Clean tracks it in a minor floor bump. Do not confuse WASI version bumps with Clean version bumps — they are unrelated.

**Component Model floor.** The Component Model level is the Canonical ABI revision a component was compiled against. It sits below the WIT interface layer: two components with compatible `clean:bridge@1.0.0` signatures can still fail to link if they were compiled against different Canonical ABI revisions. Every host declares the highest Component Model level it supports in its [`host.toml`](#84-host-declaration); every guest declares the level it requires in its `clean.toml`. Guests omitting the field default to the LTS floor above and run on every conformant host. Guests reaching for a feature above the floor (later ABI revisions, future resource-model extensions) opt in explicitly so the mismatch surfaces at build time, not at instantiation. Native async, `future<T>`, and `stream<T>` are part of the floor itself — they are not opt-in ([ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md)).

---

## 8.1 What Gets Versioned

Every WIT package has a semantic version. Versions apply at these levels:

| Package category | Example | Owner | Update cadence |
|------------------|---------|-------|----------------|
| WASI standard | `wasi:filesystem@0.3.0` | WebAssembly WG | External; Clean tracks upstream releases |
| Clean bridge (portable) | `clean:bridge@1.0.0` | Clean spec | Aligned with major Clean releases |
| Clean host (worlds + interfaces) | `clean:host@0.1.0` | Clean spec | Aligned with major Clean releases |
| Library-contributed | `clean:library/data@0.1.0` | Library author | Per library release |

The individual interfaces inside a package inherit the package version — there is no per-interface versioning independent of the package.

---

## 8.2 The Compatibility Rules


### BVER-02 — Every WIT change is classified patch, minor, or major


Every change to a WIT package MUST be classified by the effect it has on existing components. The classification determines the version bump.

#### 8.2.1 Patch (`X.Y.Z` → `X.Y.Z+1`)

Non-observable changes. Reserved for documentation edits, wire-format-neutral clarifications, and reference implementation improvements.

- **Allowed:** clarifying prose in `///` doc comments, adding examples, correcting typos.
- **Not allowed:** any change to the shape of an interface, function, type, or resource.

#### 8.2.2 Minor (`X.Y.Z` → `X.Y+1.0`)

Additive changes. Old programs continue to link and run; new programs may take advantage of the additions.

- **Allowed:** adding a new interface to the package, adding a new function to an interface, adding a new field to a record with a specified default marshalling, adding a new case to a variant, adding a new resource, adding a new option or result flavor.
- **Not allowed:** removing or renaming anything, changing a signature, changing a variant case name or payload, changing a resource's method set.

#### 8.2.3 Major (`X.Y.Z` → `X+1.0.0`)

Breaking changes. Old programs no longer link against the new package version.

- Every change not covered by patch or minor.
- Removing an interface, function, field, or variant case.
- Renaming any exported symbol.
- Changing a parameter or return type.
- Changing a resource's lifetime or ownership rules.

**Atomic-rollout test (decides minor vs. major):** a change MAY be classified minor only if review can show that a host running the previous version can deploy it while guest components built against that previous minor continue to link and run unchanged. A change that any real host implementation cannot roll out atomically with its guest components MUST be classified **major**. When review cannot demonstrate the test, the classification defaults to major — let SemVer resolvers handle the fallout.

---

## 8.3 Compiler Resolution

When compiling, the compiler:

1. Reads the target world declaration (see [§15.4](15-component-model-architecture.md)) — the target world specifies which packages and which version constraints it satisfies.
2. Reads each library dependency's WIT — each library declares which package versions it uses.
3. Solves the SAT problem: pick a single version of each package that satisfies every constraint.
4. Emits the guest component's imports against the resolved versions.

**Constraint syntax** (used in `library.toml` and world declarations):

```
"clean:bridge"        = "^0.1.0"        # >= 0.1.0, < 0.2.0
"wasi:filesystem"     = "0.2.0"         # Exact
"clean:library/data"  = ">=1.4.0"       # Any version at or above
"clean:library/other" = ">=1.4.0, <2.0" # Ranged
```

The `clean:library/*` versions shown are **illustrative future versions — post-baseline independent bumps**: per the [§8.0 baseline](#80-v2-baseline-versions), every `clean:library/<name>` package starts at `@0.1.0` and is bumped independently thereafter.

If no valid assignment exists, the compiler emits a `COM009` (BridgeResolveError) listing every conflicting constraint. The error names the packages and the constraints as ranges, not as SAT solver internals.

---

## 8.4 Host Declaration


### BVER-03 — `host.wit` is the single declaration of what a host provides


A host declares what it provides by publishing its **`host.wit`** — the WIT document naming the world it fulfills (e.g. `world server`) and every package it implements, with versions. The WIT *is* the declaration; there is no parallel TOML manifest to drift from it. See [16 §16.2 — Two WIT Documents](./16-host-contract-validation.md#162-core-idea--two-wit-documents) for the model and [16 §16.5 — Where Host WIT Lives](./16-host-contract-validation.md#165-where-host-wit-lives) for where it is published and fetched.

- The published `host.wit` MUST name every package the host implements, at the exact minor versions supported. A missing package means the host does not provide it.
- A host MAY support multiple minor branches of a package by declaring both versions in its WIT. The compiler picks the highest that satisfies the guest's constraints.
- A host MUST NOT declare a package version it does not fully implement. Partial support is a bug. If an interface is only partially implemented, the host declares the version prior to the addition of the missing member.
- Interfaces inherit their package's version (§8.1). Per-interface version entries do not exist.

Clean Framework fetches the target host's `host.wit` at build time (Moment 1 — [16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments)); Clean Manager caches it under `~/.cln/host-wit/` ([Manager §00.2](../02%20components/manager/00-manager.md#002-on-disk-layout)) and MUST record its hash in the project lockfile (`.cln/lock.toml`), so every build is reproducible against a pinned host contract.

**Guest opt-in.** A project targets the LTS floor (§8.0) by default and never declares a Component Model level. Reaching for a feature above the floor requires a single line in `clean.toml`:

```toml
[project]
name = "my-app"
component_model = "0.3.0"    # Opt in above the LTS floor
```

When the field is present, `cln build` intersects it with the local host's `[host].component_model` and with every deploy target listed in `clean.toml`. A mismatch produces a `BRIDGE-RESOLVE-COMPONENT-MODEL` error at build time naming the guest's requested level and the host's advertised level, with an actionable next step (upgrade the host or remove the feature). The developer never reaches an instantiation-time failure.

---

## 8.5 Link-Time Verification


### BVER-04 — Link-time verification; failures are `COM010`


Before emitting the final `.wasm` component, the compiler MUST perform link-time verification:

1. Every WIT interface the guest imports MUST be listed in the target world at the resolved version.
2. Every WIT type used across the boundary MUST have the same shape in the guest's and the host's version.
3. Every resource used MUST match on lifetime and method set.

Failures MUST produce `COM010` (BridgeLinkError) diagnostics with side-by-side WIT excerpts showing the mismatch. The error output is machine-parseable so the language server can render it as a code action.

---

## 8.6 Runtime Version Check


### BVER-05 — Instantiation re-verifies against the running host; mismatch is `COM011`


At instantiation, the host MUST perform a second-line verification — the Moment 3 check defined in [16 §16.4 — The Three Check Moments](./16-host-contract-validation.md#164-the-three-check-moments):

1. Read the guest component's declared imports (embedded in the component's custom section).
2. Match each against the host's own `host.wit` (§8.4).
3. Reject instantiation if any import is missing or has a different version than declared.

This is redundant with the compiler's check for correctly-built components but catches:

- Components built against a different `host.wit` than the one now running (upgrade drift).
- Components with hand-tampered WIT sections.
- Components built by an older compiler that predates a signature change.
- Components moved to a host whose Component Model level is lower than the one they were built for.

Rejection MUST be reported through the [error reporting pipeline](./06-error-reporting.md) with `error_code: "COM011"` (BridgeRuntimeMismatch), including the guest's expected WIT and the host's actual WIT.

---

## 8.7 Deprecation Protocol


### BVER-06 — Deprecation precedes removal by two minors or six months


To remove an interface member without an immediate major bump:

1. In minor version `X.Y.0`: mark the member `@deprecated` in the WIT source with a message pointing to the replacement. Continues to work.
2. In every subsequent minor `X.Y+n.0`: keep the member functional; the compiler emits a warning at every call site.
3. In the next major `X+1.0.0`: remove the member.

The deprecation-to-removal gap MUST span at least two minor versions or six months, whichever is longer. This ensures libraries and downstream projects have time to migrate.

---

## 8.8 Library Bridge Compatibility


Libraries publish their own bridge packages under `clean:library/<name>`. The same versioning rules apply. Additionally:

- A library's `library.toml` declares which host worlds it is compatible with, at which versions of the `clean:host` package (world names per [15 §0.3](15-component-model-architecture.md#03-wit-package-and-world-naming); versions per the §8.0 baseline):

  ```toml
  [compatibility]
  server  = "^0.1.0"
  browser = "^0.1.0"
  ```

- A library adding a new host-function that requires a version of a host world newer than its previous declaration MUST bump its own major version. Users get an explicit choice at upgrade time.

- Libraries MUST NOT depend on unversioned or pre-release host worlds (e.g. `1.0.0-alpha`) in a published release. Pre-release dependencies are permitted only in libraries themselves marked pre-release.

---

## 8.9 Governance


Changes to `clean:bridge/*` and `clean:host/*` packages require an **ADR** and user approval, per the decision-record and precedence rules of governance ([DOC-07, DOC-11](../01%20governance/00-documentation-principles.md)) and the component boundaries in [01 — Architecture Boundaries](../01%20governance/01-architecture-boundaries.md). The ADR must include:

- The proposed WIT diff.
- The version bump classification (patch / minor / major) with reasoning.
- A migration guide if major.
- A conformance test addition or update.

Changes to `wasi:*` packages are governed upstream; Clean tracks the [WebAssembly WG](https://github.com/WebAssembly/WASI) releases and adopts new versions in aligned Clean releases.

Library bridge packages are governed by their maintainers, but must obey §8.2 — the SemVer contract is a language-level guarantee, not a per-library choice.

---

## 8.10 Non-Goals

- **Semver of the Clean language itself.** The language version is coupled to the compiler binary. It follows SemVer but is not the same thing as bridge SemVer.
- **Rolling backports of major changes.** Once a package hits `X+1.0.0`, breaking changes are not backported into `X.Y.z`. Users who need the fix migrate.
- **Automatic minor upgrades at instantiation.** A host that provides `clean:bridge@1.1.0` does NOT silently satisfy a guest built against `clean:bridge@1.0.0` if the guest's WIT does not explicitly express `^1.0`. Compatibility is declared, not inferred.

---

## 8.11 Deferred Refinements

1. **Cross-package coupling.** Some interfaces logically require another (e.g. `clean:host/sse` only makes sense with `clean:host/routing`). Packages declare peer-requirements via a `[requires]` section in the WIT package metadata. The concrete schema for that section is defined per-package alongside the WIT it describes.
2. **Prerelease naming.** Clean uses standard SemVer prereleases (e.g. `1.4.0-rc.1`) without adjustment.

---

## Changelog

- 2026-08-01 — Technical-debt closure pass (lote X, approved 2026-08-01): §8.3 constraint-syntax examples annotated as illustrative future versions (post-baseline independent bumps per §8.0), so the `>=1.4.0` figures no longer read as a baseline violation.
- 2026-08-01 — Governance compliance (traceability pass): registered rule prefix `BVER-` and minted BVER-01 (single baseline table, C-10), BVER-02 (patch/minor/major classification, C-05/C-06), BVER-03 (`host.wit` as the single host declaration with its hash pinned in `.cln/lock.toml`, C-15/C-17), BVER-04 (link-time verification → `COM010`, C-15), BVER-05 (runtime Moment 3 re-verification → `COM011`, C-15), BVER-06 (deprecation-to-removal gap, C-05) — all reusing the existing normative text. Sections §8.0, §8.2, §8.4–§8.9 marked *Normative*. The "if in doubt, treat as major" judgment call rewritten as the decidable atomic-rollout test: minor MAY be claimed only when review demonstrates atomic host rollout with unchanged guests; otherwise the classification defaults to major.
- 2026-08-01 — Conflict-log remediation (Fase 3): §8.4 rewritten per the approved P7 resolution — the host declares what it provides by publishing its `host.wit` (16 §16.2/§16.5); `host.toml` and its per-interface version entries are gone (they contradicted §8.1's package-level versioning). The manager consumes the fetched `host.wit` and records its hash in `.cln/lock.toml`. §8.6 now cites the Moment 3 check of 16 §16.4. Internal baseline violations fixed against §8.0: `^0.2.0` → `^0.1.0` in §8.3; §8.8 `[compatibility]` rewritten to bare canonical world names (15 §0.3) at the `@0.1.0` baseline. Diagnostic codes converted per the approved mapping (formal registration Fase 4): `BRIDGE-RESOLVE-*` → `COM009`, `BRIDGE-LINK-*` → `COM010`, `BRIDGE-RUNTIME-MISMATCH` → `COM011`. §8.9: the nonexistent "Principle 25" replaced by a cite to the governance decision-record and precedence rules and the architecture boundaries; "spec RFC" → ADR.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Bridge maintainers evolving WIT packages; host implementors publishing `host.wit`; toolchain authors resolving versions
- **Rule prefix:** `BVER-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Host Bridge](./02-host-bridge.md), [Component Model Architecture](./15-component-model-architecture.md), [Host Contract Validation](./16-host-contract-validation.md), [ADR-0022 §4](../01%20governance/decisions/0022-foundational-technology-stack.md)
- **Satisfies:** INTEROP-02, INTEROP-03, SEC-04
