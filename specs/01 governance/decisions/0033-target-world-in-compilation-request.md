# ADR-0033 — The compilation request carries the target world

The compiler is required to validate every host-function call site against the target world, and forbidden from fetching that world itself — but the request document it is handed has no field carrying one. The obligation is therefore unimplementable as specified. This ADR adds a `target_world` field to the request-document schema, carrying the host's WIT declaration verbatim, the name of the world within it to validate against, and the identity of the declaration it came from, and assigns population of that field to Clean Framework, which already fetches the same file one step earlier in the build.

---

## Context

Two Accepted rules in [Platform 14](../../03%20platform/14-compiler-architecture.md) describe the compiler's relationship to the target world, and together they close a door with nothing behind it.

[CMP-01](../../03%20platform/14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else) requires the compiler to obtain every input — and names "target world WIT" explicitly among them — from the request document alone, with no filesystem discovery, no network, and no registry lookup. [CMP-03](../../03%20platform/14-compiler-architecture.md#cmp-03--every-import-is-verified-against-the-world-in-the-request) requires it to verify every `host function` call site against "the target world's WIT as delivered in the compilation request," emitting `COM012` and aborting before codegen on any call site the world does not contain. [HCV-04](../../03%20platform/16-host-contract-validation.md#hcv-04--the-scope-split-who-validates-what) states the same boundary from the validation side: the compiler validates against the world it is handed and never downloads a host contract.

**The request-document schema in [§14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs) has no field for it.** The schema carries `sources`, `build`, `folders`, `dependencies`, `compile_limits`, `telemetry`, `library_manifests`, and `overrides`. The only WIT in the document is `library_manifests[].wit`, which is library WIT — the interfaces a library declares — not the world the host fulfills. Those are different documents answering different questions, and [16 §16.2](../../03%20platform/16-host-contract-validation.md#162-core-idea--two-wit-documents) keeps them distinct by design.

So the compiler is told to obtain the world from the request, forbidden from obtaining it any other way, and never given it. The World Import Check ([14 §14.4.2](../../03%20platform/14-compiler-architecture.md#1442-detailed-pass-responsibilities) pass [9]) cannot be written. `COM012` cannot fire.

What is lost while the gap stands is a real safety property, not a formality. The check is what turns "this program calls a server-only function but targets the browser" into a compile-time diagnostic pointing at the call site, instead of an instantiation failure at Moment 3 ([HCV-01](../../03%20platform/16-host-contract-validation.md#hcv-01--three-check-moments-each-with-its-actor-and-its-code)) telling the developer only that the component did not load. Both refuse the program. Only one says where the problem is.

Three facts shape the fix.

**The framework already has the world.** Moment 1 obliges it to fetch the target's `host.wit` and validate the project against it *before the compiler is invoked* ([HCV-01](../../03%20platform/16-host-contract-validation.md#hcv-01--three-check-moments-each-with-its-actor-and-its-code)). Clean Manager caches it under `~/.cln/host-wit/` and records its hash in `.cln/lock.toml` ([BVER-03](../../03%20platform/08-bridge-versioning.md#84-host-declaration)). The world is in hand, verified and pinned, at the moment the request document is assembled. Nothing new has to be fetched — the file simply is not passed on.

**The world is already determined by the request.** `build.target` names a `(architecture, host-world, ABI)` triple, and [07 §7.2](../../03%20platform/07-build-config.md) maps each built-in target to exactly one component-model world: `wasm32-server` → `server`, `wasm32-browser` → `browser`. What is missing is not the *choice* of world but its *content* — the WIT text saying which interfaces that world contains.

**Determinism constrains the shape.** [CMP-02](../../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out) requires byte-identical outputs for byte-identical requests, and [14 §14.15.1](../../03%20platform/14-compiler-architecture.md#14151-key-structure) keys the build cache on the canonical serialization of the whole request. A field that carries a reference to something fetched later — a URL, a registry coordinate, a cache path — would let two identical requests compile against different worlds. Whatever the compiler validates against must be *in* the request, by value.

## Decision

**The request document carries the target world by value, in a new `target_world` field, populated by Clean Framework.**

Four parts. Part 1 carries the concrete field shape, settled with the framework session on 2026-08-11 and amended into this ADR before acceptance.

1. **The field.** `target_world` is added to the [§14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs) schema as a required top-level object. It carries the fetched `host.wit` **verbatim**, the name of the world within it that `build.target` selects, and the identity of the host declaration it was taken from: host name, resolved version, and the SHA-256 the manager recorded in `.cln/lock.toml` per [BVER-03](../../03%20platform/08-bridge-versioning.md#84-host-declaration). The WIT is carried **by value** — inline text, not a path, URL, or cache reference — so that CMP-01 holds without qualification and the request stays self-describing for the build cache, `cln repro build`, and any later audit of what a shipped component was actually checked against.

   The agreed shape, settled with the framework session on 2026-08-11:

   ```json
   "target_world": {
     "host": "clean-server",
     "version": "0.1.0",
     "world": "server",
     "sha256": "9f2b1c...",
     "wit": "package clean:host@0.1.0;\nworld server { ... }\n"
   }
   ```

   Two of those five fields were not in this ADR's first draft and were added at the framework session's request. Both close a hole that would otherwise have been discovered mid-implementation.

   **`world` — the selector.** A `host.wit` is a WIT *package*, and a package may declare more than one world; [BVER-03](../../03%20platform/08-bridge-versioning.md#84-host-declaration) further allows a host to declare multiple package versions in the same file. "The WIT text of the world named by `build.target`" therefore does not identify a document on its own. Two ways to resolve it: the framework extracts the single named world and sends only that, or it sends the file whole and names the world. Extraction was rejected — it makes the framework rewrite the host's declaration, so the request records framework output rather than what the host published, and `cln repro build` loses the ability to show the contract as shipped. Naming it keeps the framework a courier. Without the selector the compiler must derive the world from `build.target` using its own table, which is Option B below, rejected there for the same reason it would be wrong here.

   **`version` is resolved, not requested.** `[target].version` in `clean.toml` is a semver *constraint* — [16 §16.5](../../03%20platform/16-host-contract-validation.md#165-where-host-wit-lives) shows `version = "0.1.x"`. A constraint in the request would let two byte-identical requests denote different host versions, which is a [CMP-02](../../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out) break arriving by a side door. The field carries the concrete version the lockfile pinned.

   `sha256` is redundant with `wit` for cache-keying, and deliberately so: it is the forensic link back to `.cln/lock.toml`, and it lets a disagreement between the pinned hash and the inlined text be caught rather than silently tolerated.

2. **The producer.** Clean Framework populates it, from the `host.wit` it already fetched for its own Moment 1 check. This assigns no new capability to any component: the framework holds the file, [HCV-04](../../03%20platform/16-host-contract-validation.md#hcv-04--the-scope-split-who-validates-what) already makes fetching it the framework's job, and it already assembles the request document ([15 §12.2](../../03%20platform/15-component-model-architecture.md#122-the-four-components)). A request that reaches the compiler without the field is malformed and is refused with `RQD002`, exactly as any other schema violation — [RQD002](../../03%20platform/10-semantic-rules.md#rqd002--request-schema-violation) already names "a missing required field" among its conditions, so the refusal needs no new code. The compiler does not degrade to skipping the check, and does not go looking for a world of its own.

3. **The identity triple is not decoration.** Recording host name, resolved version, and hash alongside the WIT text is what lets the build manifest state *which* contract a component was validated against, keeps `cln repro build` honest when a host republishes its WIT under the same version, and gives Moment 3 a way to explain a load-time refusal in terms of what the build believed. The WIT text alone would validate correctly and forensically explain nothing.

4. **`spec_version` stays at `"1"`.** [14 §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs) makes incrementing it a breaking change governed by this process. This addition does not break a consumer, because there is no consumer: `clean-compiler` does not exist yet, and no shipped artifact reads or writes a request document. The field lands as part of the initial `"1"` surface rather than as a break in it. **This reasoning expires the moment a compiler ships** — a later field addition, with a real implementation in the field, is a different decision.

The alternative readings of "self-contained" are worth naming, because the phrase can be stretched. A request carrying a *reference* the compiler resolves is not self-contained: CMP-01 forbids the resolution step, whatever it points at. A request carrying a *world name* the compiler expands from built-in knowledge is not self-contained either — it makes the compiler's own table the authority on what a host provides, which is precisely the coupling [BVER-03](../../03%20platform/08-bridge-versioning.md#84-host-declaration) exists to prevent by making the published `host.wit` the single declaration. By value is the only reading that satisfies both rules as written.

## Options considered

- **A — Leave the gap; let Moment 3 catch it.** The host already refuses a non-conforming component at load with `COM017`, so nothing unsafe ships. Costs: the developer learns at instantiation, not at the call site; `COM012` stays a registered code no path can emit; two Accepted rules stay unimplementable, which is a standing invitation for an implementer to "fix" them by making the compiler fetch. Rejected — the gap does not stay quietly open, it gets closed wrongly.

- **B — The compiler resolves the world from `build.target` using built-in knowledge.** No schema change; the target-to-world mapping in [07 §7.2](../../03%20platform/07-build-config.md) is already a table. Costs: it makes the compiler binary the authority on what each host provides, so a host adding an interface requires a compiler release, and a host at two versions cannot be distinguished at all. This directly contradicts [BVER-03](../../03%20platform/08-bridge-versioning.md#84-host-declaration) — `host.wit` is the single declaration, and the mapping table names the world, never its contents. Rejected on those grounds.

- **C — The field carries a reference (URL, or a path into `~/.cln/host-wit/`).** Smaller requests, and the manager's cache is already populated and hash-pinned. Costs: the compiler must resolve the reference, which is the filesystem or network access [CMP-01](../../03%20platform/14-compiler-architecture.md#cmp-01--the-request-document-is-self-contained-the-compiler-touches-nothing-else) forbids in as many words; two byte-identical requests could compile against different worlds if the cache changed between them, breaking [CMP-02](../../03%20platform/14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out) and poisoning the [CMP-06](../../03%20platform/14-compiler-architecture.md#cmp-06--a-cache-hit-must-be-byte-identical-to-a-cache-miss) cache-parity invariant. Rejected — it reintroduces exactly the ambient dependency the purity rule was written to remove.

- **D — Carry the declaration by value with a world selector and source identity, populated by the framework (chosen).** Satisfies CMP-01 as written, keeps determinism and the cache key sound, assigns the work to the component that already holds the file, and makes the build manifest able to state which contract was checked. Cost: request documents grow by the size of a host contract, and the framework must pass along something it previously consumed and discarded.

- **E — Carry it by value but make the field optional, skipping the check when absent.** Would let a first compiler milestone ship before the framework side lands. Rejected: an optional safety check is one that is off in exactly the conditions where nobody notices, and "compiled without world validation" is not a state a build manifest should be able to describe silently. The two sides land together.

- **F — Carry only the selected world, extracted by the framework.** Added 2026-08-11. A variant of D raised during the framework agreement: rather than sending `host.wit` whole and naming a world, the framework parses the file and inlines just the one world `build.target` selects. Smaller requests, and no selector field. Rejected: it makes the framework rewrite the host's declaration, so the request records framework output rather than the published artifact — `cln repro build` can no longer show the contract as the host shipped it, and a framework parser bug becomes indistinguishable from a host contract change. Carrying the file verbatim keeps the framework a courier and the audit trail exact.

## Consequences

**What becomes easier:**

- **`COM012` becomes implementable.** Step 7 of the compiler's first milestone (`work/2026-08-11-compiler-component-model-emission.md`) is unblocked, and with it the acceptance path that ends in `clean-server` serving a request from a `cln`-built guest.
- **World mistakes move to the call site.** A browser-targeted program calling a server-only function gets a diagnostic naming the function and the world, at build time, instead of a load-time refusal.
- **Builds become auditable against a specific contract.** The request — and through it the build manifest and the cache key — records exactly which host declaration, at which version and hash, a component was validated against.
- **`cln repro build` stays honest across host republication.** Reproducing an old build re-validates against the recorded world, not against whatever the host publishes today.

**What becomes harder:**

- **Request documents grow.** The host's whole WIT declaration is inlined into every request, and into every cache key computed from one — the file as published, not a slice of it, since the framework names the world rather than extracting it. This is the cost of purity; it is bounded by the size of a host contract and is paid in memory and hashing, not in I/O.
- **The framework must retain what it used to discard.** It fetches `host.wit` for Moment 1 today and does not carry it forward. It now threads the text and its identity into the request document it assembles.
- **Two components must land the change together.** A compiler reading a field no framework populates fails every build with `RQD002`; a framework populating a field no compiler reads does nothing. Neither half is independently shippable, which is a real sequencing constraint on two separate repos.

**What must now be done:**

- [14 §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs) gains the `target_world` field in the schema, its rules paragraph, and a changelog entry.
- **`RQD002` already covers the missing field; §14.1.1 just does not say so.** The framework session read §14.1.1's rules bullet — "unknown top-level keys are a hard error (`RQD002`)" — and objected that nothing there gives a code for a *missing required* key. The objection was withdrawn on checking the owning documents: [RQD002](../../03%20platform/10-semantic-rules.md#rqd002--request-schema-violation) is defined as covering "an unknown top-level key, an unknown key inside a well-known section, **a missing required field**, or a malformed value," and [09 §RQD002](../../03%20platform/09-error-codes.md) registers `RequestSchemaViolation` with the same scope. So this ADR's first draft was right that no new code is needed. What remains is a summary that under-describes the rule it summarizes: §14.1.1's bullet names only the unknown-key half. The amendment should widen that bullet to match RQD002 as owned, which is an editorial fix, not a new rule.
- The exact field shape is agreed with the framework session before either side builds against it. This ADR fixed the *content* — WIT text by value, plus host name, version, and hash — and left key names and nesting to that agreement, because the framework owns the producing side and a shape settled by one party alone is a shape the other works around. **That agreement closed on 2026-08-11**; the resulting five-field shape, and the two fields the framework session added to it, are recorded in Decision part 1 above.
- [02 components / compiler 01 §11](../../02%20components/compiler/01-specification.md#11-open-questions) records this gap as an open question and is updated to cite this ADR once it is Accepted. The related open question there — whether `build.target` names the world or carries it — is answered by this decision: the target names it, `target_world` carries it.
- **Verification is blocked downstream, not by this decision.** No compiler exists to read the field and no framework code writes it yet, so the specification work can complete while end-to-end proof waits on the first milestone. The acceptance check is the one already written into that brief: `cln build tests/cln/component/import-not-in-world.cln` produces `COM012` and no `dist/app.wasm`.

---

## Metadata

- **Status:** Accepted
- **Date:** 2026-08-11
- **Accepted:** 2026-08-11
- **Amended:** 2026-08-11 — field shape settled with the framework session and folded in: `world` selector and resolved-not-requested `version` added (Decision part 1), extraction recorded as rejected Option F, the §11.4 sibling-step question closed, and §14.1.1's schema-violation bullet flagged as under-describing `RQD002`.
- **Supersedes:** None
- **Spec impact:**
  - [03 platform / 14 — Compiler Architecture §14.1.1](../../03%20platform/14-compiler-architecture.md#1411-inputs) — `target_world` added to the request-document schema as a required top-level object with five string fields (`host`, `version`, `world`, `sha256`, `wit`), the declaration carried by value and the world named rather than extracted; `spec_version` remains `"1"`. The amendment also widens §14.1.1's schema-violation bullet, which names only the unknown-key half of [RQD002](../../03%20platform/10-semantic-rules.md#rqd002--request-schema-violation) though the rule as owned also covers missing required fields.
  - [02 components / compiler 01 — Compiler Specification](../../02%20components/compiler/01-specification.md) — §11 open question closed by citation once Accepted; CCMP-03 gains a concrete field to name.
  - [02 components / framework 11 — Build Orchestration §11.4](../../02%20components/framework/11-build-orchestration.md#114-lowering-cleantoml-to-the-request-document) — the framework threads the `host.wit` it fetches for Moment 1 into the request document it assembles. `target_world` does not fit the lowering table: every row there projects a `clean.toml` section into a request key, whereas `target_world` originates in the fetched `host.wit`. `[target]` names *which* host contract to fetch; it does not carry the contract. **The framework session's call, taken on 2026-08-11: a sibling step, not a table row.** The table keeps its current shape and its "mechanical and lossless projection of `clean.toml`" contract intact, and a new §11.4.1 beside it states that `[target]` selects a contract rather than carrying one, that the WIT enters the request from the Moment 1 fetch, and that `target_world` is required in the request despite no `clean.toml` section mapping onto it. The rejected alternative — a table row with a provenance column — was judged worse documentation: it would make every other row's provenance implicit and therefore unstated.
  - No change to [16 — Host Contract Validation](../../03%20platform/16-host-contract-validation.md): [HCV-04](../../03%20platform/16-host-contract-validation.md#hcv-04--the-scope-split-who-validates-what)'s scope split is what this ADR makes implementable, not what it revises.
