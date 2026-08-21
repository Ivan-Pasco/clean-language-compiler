# Platform 07. Build Configuration

> **Canonical schema:** [`../02 components/framework/schema/clean.toml.md`](../02%20components/framework/schema/clean.toml.md) is the DOC-18 source of truth for the `clean.toml` schema. This chapter defines the six rules (CONF-01..06) that govern configuration behavior; the field-level schema is consolidated in the schema file. When this chapter and the schema disagree, the schema wins.

Every Clean project has a **`clean.toml`** at its root — the single source of truth for how the project builds: which compiler, which target, which memory tier, which libraries, which optimization level, which folder layout. This chapter defines the rules that govern how `clean.toml` is read and applied; the field-level schema is consolidated in [`../02 components/framework/schema/clean.toml.md`](../02%20components/framework/schema/clean.toml.md) per DOC-18. Configuration lives in `clean.toml`, not in environment variables (except for secret injection), not in scattered CLI flags (except for one-off overrides), not in per-file directives; the binding rule is [CONF-01](#71-file-location-and-precedence).

---

## 7.1 File Location and Precedence


### CONF-01 — `clean.toml` is the only source of configuration


- `clean.toml` lives in the project root — the directory containing the `app/` folder.
- A single project has a single `clean.toml`. Multi-project workspaces have one per project plus (optionally) a `workspace.toml` at the parent level ([§7.9](#79-workspaces)).
- Command-line overrides use `--override "section.key=value"` and apply for that invocation only. Overrides MUST be logged in the build manifest so a reproducible replay is always possible.
- Environment variable overrides use `CLN_<UPPER_SECTION>_<UPPER_KEY>` (e.g. `CLN_MEMORY_TIER=heavy`) and MUST follow the same audit rule.

**Precedence, highest wins:** command-line flag > `--override` > environment variable > `clean.toml` > default.

No other input may influence the build: given the same `clean.toml`, the same `.cln/` contents, and the same recorded overrides, two builds on the same platform MUST be identical. A build that differs because of ambient configuration outside this precedence chain is a defect.

---

## 7.2 Schema — Top Level

This schema and [§7.3](#73-memory--full-schema) are the closed set of recognized sections and keys; validation against them is [CONF-06](#710-validation). The full field-level schema — every recognized section, every key, every type, every default — is in [`../02 components/framework/schema/clean.toml.md`](../02%20components/framework/schema/clean.toml.md).

The manifest's recognized sections, at a glance:

- `[project]` — identity (name, version, description, authors, license).
- `[build]` — entry point, target triple, optimization profile, `strip`, `strip_checks`, `component-model`, `memory64`.
- `[memory]` — memory tier and page bounds (§7.3).
- `[folders]` — folder-to-library implicit scoping (§7.6).
- `[dependencies]` — direct dependencies with version constraints (§7.7).
- `[compile.limits]` — hard caps on compile-time behavior (§7.8).
- `[compile.env]` — environment values visible to compile-time functions.
- `[security]` — `allowedHostModules` gate.
- `[runtime]` — per-invocation `epoch-ms` and other host-enforced runtime limits.
- `[report]` — report-consent level.
- `[target]` — target host contract (semantics: [Host Contract Validation §16.5](./16-host-contract-validation.md)).
- `[mcp.host_runtime]` — deployment-manifest pointer for MCP awareness.
- `[dev]` — development conveniences (`watch`, `lsp-log-level`, `hot-reload`, `capture-traces`).
- `[build.profile.<name>]` — custom optimization profiles.

**Toolchain versions are NOT in `clean.toml`.** Compiler and framework pins are written by `cln pin` to `.cln/version` and `.cln/frame-version` (Clean Manager §00.3.3): `clean.toml` holds what the human declares; `.cln/` holds what the toolchain resolves.

Sections omitted are treated as the built-in defaults. Sections present but empty are treated as "explicitly enabled with all defaults."

---

## 7.3 `[memory]` — Full Schema

Field list: [`../02 components/framework/schema/clean.toml.md` §`[memory]`](../02%20components/framework/schema/clean.toml.md#memory--memory-configuration). Bounded by [Platform 05 — Memory Policy](./05-memory-policy.md) tiers.

Validation constraints:

- `initial-pages <= maximum-pages`.
- `initial-pages` and `maximum-pages` MUST both fall within the tier's range.
- `growth-factor = "exact"` disables the amortization guarantee and may only be set alongside `tier = "embedded"` or `tier = "minimal"`.
- `memory64 = true` requires `build.memory64 = true`; setting one without the other is a configuration error (`CFG002`).

---

## 7.4 Build Targets


### CONF-02 — `build.target` names a declared target


A **target** is a `(architecture, host-world, ABI)` triple. `build.target` MUST be one of the built-in targets below or a third-party target declared through Clean Manager; any other value is a schema violation (`CFG001`, [CONF-06](#710-validation)). The compiler ships with these built-in targets:

| Target | Component-model world | Notes |
|--------|----------------------|-------|
| `wasm32-server` | `server` | Default for backend projects. Runs on clean-server. |
| `wasm32-browser` | `browser` | Runs in the browser via jco / component-model runtime. |
| `wasm32-cli` | `cli` | Runs on `clean-cli` locally. |
| `wasm32-embedded` | `embedded` | **(reserved)** — the embedded world is not shipped in V2 ([15 §0.3](15-component-model-architecture.md#03-wit-package-and-world-naming)). |
| `wasm64-server` | `server` | Same as `wasm32-server` with memory64 enabled. Adds `build.memory64 = true`. |

Adding a target is a spec change, not a config change. Third-party hosts declare a target name (`wasm32-partner-cloud`) and the WIT world it satisfies; that declaration is consumed through Clean Manager.

---

## 7.5 Optimization Profiles

| `optimization` | Emit debug info | Symbol names | Tree shake | Inline threshold | Time budget per module |
|----------------|------------------|--------------|-----------|------------------|------------------------|
| `debug` | full | full | no | conservative | none |
| `release` | line-only | mangled | yes | aggressive | 10 min (guidance) |
| `size` | none | mangled | yes | conservative | 10 min (guidance) |

The per-module time budget is operational guidance (for sizing CI), not an enforced limit; the enforced cap is the whole-build `total-timeout-min` in `[compile.limits]` ([§7.8](#78-compile-time-limits)), whose breach is `BLD001`.

Custom profiles are declared under `[build.profile.<name>]` with the same keys as `[build]`. `cln build --profile <name>` selects it. This is intentional — the built-in three are the common cases; anything else is explicit per project.

Optimization does not change program semantics. A `debug` build and a `release` build of the same source produce identical output for every test in the conformance suite. Optimization affects size, speed, and startup time only.

---

## 7.6 Folder-to-Library Mapping (`[folders]`)


`[folders]` is the mechanism by which library block handlers ([§21](../04%20language/21-block-handlers.md)) are scoped to specific parts of a project's source tree. It answers "in `app/data/User.cln`, which library owns the `data:` block name?"

```toml
[folders]
"app/data/**"   = ["data"]               # Only data's blocks resolve here
"app/server/**" = ["server"]
"app/ui/**"     = ["ui", "forms"]        # Both libraries in scope
"app/shared/**" = []                     # No libraries; language core only
```

### CONF-03 — `[folders]` schema; semantics owned by LBS-01


- Keys MUST be project-root-relative POSIX folder paths, optionally with one trailing `/**` — the two spellings are equivalent (both denote the folder's whole subtree), and no other glob form is admitted; the match rule is owned by [LBS-04](../02%20components/framework/09-libraries-specification.md#lbs-04--what-it-means-for-a-folders-pattern-to-match-a-file). Layout follows the canonical form in [15 §0.6](15-component-model-architecture.md#06-project-folder-layout) (per-library defaults: [15 §0.7](15-component-model-architecture.md#07-default-folder-scope-by-library)).
- The scoping semantics are owned by [LBS-01 — the project manifest is the only source of implicit scope](../02%20components/framework/09-libraries-specification.md#lbs-01--the-project-manifest-is-the-only-source-of-implicit-scope) ([09 §6](../02%20components/framework/09-libraries-specification.md#6-project-manifest-cleantoml)); this section defines only the schema and MUST NOT be read as a second semantics home.
- Adding a library to `[dependencies]` does not put it in scope anywhere by itself. `[folders]` is what puts it in scope (per LBS-01). This prevents accidental block-name conflicts across a large project.

See [LBS §6](../02%20components/framework/09-libraries-specification.md#6-project-manifest-cleantoml) for the library-side view of the same mapping.

---

## 7.7 Dependencies


`[dependencies]` lists direct dependencies with an exact or ranged version:

```toml
[dependencies]
"data"     = "1.4.0"                       # Exact
"server"   = "^1.4.0"                      # Compatible-with (SemVer minor)
"ui"       = ">=1.4.0, <2.0.0"             # Explicit range
"my-org.charts"  = { git = "https://...", tag = "v0.3.0" }
"local-shared"   = { path = "../shared" }        # Workspace-local
```

Version resolution follows Cargo-style SAT solving. Conflicting version constraints across the transitive graph are reported as a resolver error with the full path of each constraint.

### CONF-04 — The lockfile and toolchain pins close the build


Lock file: `.cln/lock.toml`, written by Clean Manager (`cln fetch` / `cln lock` — [Manager §00.3.2](../02%20components/manager/00-manager.md#0032-dependencies)). The lock MUST pin the exact version of every direct and transitive dependency, with checksums. CI builds MUST fail with [`CFG004`](./09-error-codes.md#316-configuration-codes-cfg) (`LockfileMismatch`) if `clean.toml` and `.cln/lock.toml` disagree. Compiler and framework pins are not part of `clean.toml`: they are written by `cln pin` to `.cln/version` and `.cln/frame-version` ([Manager §00.3.3](../02%20components/manager/00-manager.md#0033-toolchain-versions); see the note in [§7.2](#72-schema--top-level)).

---

## 7.8 Compile-Time Limits


```toml
[compile.limits]
handler-timeout-ms = 5000            # Per block-handler invocation (5 s). See §21.7.
handler-memory-mb  = 128             # Per block-handler invocation (128 MiB).
library-max-files  = 1000            # Source-tree file count per library.
library-heap-mb    = 512             # Compile-time heap across all of one library's handler invocations.
max-ir-nodes       = 500000          # IR nodes emitted by a single handler invocation.
total-timeout-min  = 10              # Whole-build wall clock. Default 10.
max-file-size-mb   = 4               # Rejects source files above this. Default 4.
max-import-depth   = 32              # Prevents pathological cycles in library dependency graphs.
max-nesting-depth  = 256             # Structural nesting (expression + block + type) per source file. Counting rule: 10 §BLD001.
```

The per-handler and per-library values are defaults; the home of their enforcement is `LIB009` (compiletime budget) and `LIB014` (library resource limit) in [10 — Semantic Rules](./10-semantic-rules.md). (`[compile.limits]` is the single canonical name for this table; earlier drafts also called it `[compiletime.limits]` or `[build.limits]`.)

### CONF-05 — Limits are hard caps; exceeding one is `BLD001`


Every limit here is a **hard cap**, not a soft warning. Exceeding a limit MUST produce a `BLD001` (BuildLimitExceeded) error carrying the limit name and the observed value.

---

## 7.9 Workspaces

A `workspace.toml` at a parent directory groups multiple `clean.toml` projects:

```toml
# workspace.toml
[workspace]
members = ["packages/api", "packages/web", "packages/shared"]

[workspace.dependencies]
"data"   = "1.4.0"
"server" = "1.4.0"
```

Members inherit `[workspace.dependencies]` unless they override in their own `clean.toml`. Workspace-wide compiler and framework pins live in `.cln/workspace.toml`, owned by Clean Manager ([Manager §00.11](../02%20components/manager/00-manager.md#0011-design-rules-and-deferred-refinements)) — not in `workspace.toml`. A workspace has one shared `.cln/lock.toml` at the workspace root; individual project lockfiles are absent inside a workspace.

---

## 7.10 Validation


Clean Framework and Clean Manager validate `clean.toml` — the compiler never reads it. The compiler validates the compilation request document it receives instead ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)).

### CONF-06 — The schema is closed; violations carry `CFG` codes


The schema of [§7.2](#72-schema--top-level)–[§7.3](#73-memory--full-schema) is closed: a key or section not defined there is not "reserved for future use" — it is invalid. Validation MUST report:

- **Schema violations** (unknown key, wrong type, missing required) — `CFG001` (ManifestSchemaViolation).
- **Constraint violations** (memory tier disagreement, folder path conflict, project tier below a dependency library's declared minimum — [05 §5.1](./05-memory-policy.md#51-memory-tiers)) — `CFG002` (ManifestConstraintViolation).
- **Semantic warnings** (deprecated key, custom profile shadowing a built-in) — `CFG003` (ManifestWarning).
- **Lockfile mismatch** (`clean.toml` and `.cln/lock.toml` disagree in a CI build — [CONF-04](#77-dependencies)) — `CFG004` (LockfileMismatch).

`cln config validate` runs validation without a build. `cln config show --resolved` prints the fully-resolved config after all merges and overrides — useful for debugging "why did CI use a different tier?".

---

## 7.11 Example — Small Server Project

```toml
[project]
name = "my-api"
version = "0.1.0"

[build]
target = "wasm32-server"
optimization = "release"

[memory]
tier = "standard"

[dependencies]
"data"   = "1.4.0"
"server" = "1.4.0"

[folders]
"app/data/**"   = ["data"]
"app/server/**" = ["server"]

[report]
consent = "full"
```

---

## 7.12 Non-Goals

- **Language-per-project overrides.** A project may not pin a Clean *language* version different from what the compiler binary supports; they are the same thing. Version pinning is compiler pinning.
- **Environment-specific configs.** No `clean.toml.production`, `clean.toml.staging`. Environment differences are expressed via profiles ([§7.5](#75-optimization-profiles)) and CLI overrides, not by shadow files.
- **Per-file configuration.** A `.cln` file has no `// @config` directives. All configuration flows through `clean.toml`.

---

## 7.13 Deferred Refinements

1. **Encrypted config sections.** `clean.toml` does not support an `[encrypted]` section referencing an external key. Secrets belong in the deployment platform's secret store, not in the build config.
2. **Config-driven feature flags at compile time.** V2 does not add a `[features]` section with `#[cfg(feature = "...")]`-style guards. Feature-flag use cases are handled through §09 Libraries — capabilities and companion types cover them without extending the config schema.

---

## Changelog

- 2026-08-20 — §7.8 gains `max-nesting-depth = 256`, from the compiler's Milestone 9 (`clean-language-compiler/docs/DISCOVERIES-M9.md` §2, via [work/2026-08-20-structural-nesting-limit.md](../work/2026-08-20-structural-nesting-limit.md)): nothing bounded expression/block/type nesting, so deep-but-legal input drove a recursive-descent implementation into a stack overflow — a process abort that satisfies neither CMP-04 nor CMP-05. The limit rides the existing CONF-05 → `BLD001` machinery (no new code, resolving the DIA-01 blocker); what counts toward the depth and the enforcement point are owned by [10 §BLD001](./10-semantic-rules.md#bld001--build-limit-exceeded); the request-document mirror gains `max_nesting_depth` ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)).
- 2026-08-18 — CONF-03's key-shape bullet made exact, from the compiler's Milestone 5 (`clean-language-compiler/docs/DISCOVERIES-M5.md`, item 8): "glob patterns" left the match rule undefined while the tree's own examples disagreed — globbed keys here (`"app/data/**"`), bare keys in the compilation-request example ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)). The bullet now states the admitted forms (bare folder path, or one trailing `/**`; equivalent) and cites the new [LBS-04](../02%20components/framework/09-libraries-specification.md#lbs-04--what-it-means-for-a-folders-pattern-to-match-a-file) as the match rule's home, keeping CONF-03's schema-only posture.
- 2026-08-07 (afternoon) — §7.2 top-level schema and §7.3 memory schema TOML skeletons removed per DOC-18 anti-redundancy rule; each now points at the corresponding subsection of [`../02 components/framework/schema/clean.toml.md`](../02%20components/framework/schema/clean.toml.md). CONF-01..06 rule text, validation constraints (§7.3), and non-schema narrative retained here. Section headings preserved so existing anchors (`#72-schema--top-level`, `#73-memory--full-schema`, `#77-dependencies`, etc.) still resolve.
- 2026-08-07 — Chapter gained a canonical-schema pointer to [`../02 components/framework/schema/clean.toml.md`](../02%20components/framework/schema/clean.toml.md) per DOC-18. The field-level schema now lives in that file; this chapter retains the CONF-01..06 rule text. No normative change; readers looking for exact field definitions are redirected.
- 2026-08-01 — `[build] strip_checks` registered: the key was documented in [04 language / 10 — Contracts](../04%20language/10-contracts.md) and existed in no schema, so a manifest written to that chapter was rejected by `CFG001`.
- 2026-08-01 — Technical-debt closure pass (lote X, approved 2026-08-01): §7.10 tier-below-library-minimum reclassified from `CFG003` warning to [`CFG002`](./09-error-codes.md#316-configuration-codes-cfg) constraint **error**, resolving the 05-vs-07 boundary conflict in favor of [05 §5.1](./05-memory-policy.md#51-memory-tiers) (build rejected). CONF-04's pending lockfile-disagreement marker closed with the newly registered [`CFG004`](./09-error-codes.md#316-configuration-codes-cfg) `LockfileMismatch` (CI build where `clean.toml` and `.cln/lock.toml` disagree); §7.10 gained the CFG004 bullet. §7.2/§7.11: `[telemetry] consent-level` renamed to **`[report] consent`** — the key configures report consent (behavior home: [06 §6.6](./06-error-reporting.md#66-privacy-and-consent)); Clean Manager's `cln telemetry` adoption heartbeat is a separate system that does not read `clean.toml`, so the schema no longer uses the "telemetry" name.
- 2026-08-01 — Governance compliance (traceability pass): registered rule prefix `CONF-` and minted CONF-01 (`clean.toml` single source + precedence chain + logged overrides, C-04/C-10), CONF-02 (valid build targets, C-16), CONF-03 (`[folders]` schema with semantics owned by LBS-01, C-04/C-07), CONF-04 (lockfile `.cln/lock.toml` + `cln pin` toolchain pins close the build, C-04/C-17), CONF-05 (compile-time limits are hard caps → `BLD001`, C-02/C-08), CONF-06 (closed schema; violations are `CFG001`/`CFG002`/`CFG003`, C-02/C-04) — all reusing the existing normative text. Sections §7.1–§7.4, §7.6–§7.8, §7.10 marked *Normative*. §7.5's "10 min soft" per-module time budget resolved as *informative* operational guidance (the enforced cap is `total-timeout-min`, `BLD001`); the intro's "different builds = bug" sentence replaced by a cite to CONF-01; lock/`clean.toml` disagreement made a MUST-fail with a pending diagnostic code marker.
- 2026-08-01 — Conflict-log remediation (Fase 3): lockfile corrected per P11 — `.cln/lock.toml` written by Clean Manager (was `clean.lock` in the project root); `[tools]` and `[workspace.tools]` removed from the schema — toolchain pins are `cln pin` → `.cln/version`/`.cln/frame-version` (principle: `clean.toml` holds what the human declares, `.cln/` holds what the toolchain resolves). §7.10 corrected per P3 — framework/manager validate `clean.toml`, the compiler validates the request document (14 §14.1.1). Diagnostic codes converted per the approved mapping (formal registration Fase 4): `BUILD-LIMIT-EXCEEDED` → `BLD001`, `CONFIG-SCHEMA-*` → `CFG001`, `CONFIG-CONSTRAINT-*` → `CFG002`, `CONFIG-WARN-*` → `CFG003`; the §7.3 memory64 mismatch is `CFG002`. Schema consolidation per P16.11: added `[compile.env]` (canonical name for `[compiletime.env]`), `[security] allowedHostModules` (enforcement home: 10), `[target]` and `[mcp.host_runtime]` (semantics home: 16), `[dev] hot-reload` (behavior home: hosts/01 §1.9) and `[dev] capture-traces` (behavior home: 14); `[compile.limits]` absorbed the per-library budgets of LIB009/LIB014 (defaults; enforcement home: 10) and is the single canonical name (was also `[compiletime.limits]`/`[build.limits]`). `[folders]` syntax aligned to the glob form of the home (LBS-01, LBS §6; layout per 15 §0.6-0.7) — the "longest-prefix match wins" rule removed, scoping semantics defer to the home. `[runtime] epoch-ms` declared the home of the epoch defaults; the empty cross-reference to 12 removed (dedup #7). "`src/` folder" → "`app/` folder"; `wasm32-embedded` marked reserved (P16.1); world names normalized per 15 §0.3; "version manager" → Clean Manager.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Project authors writing `clean.toml`; compiler, framework, and manager implementors reading and validating it
- **Rule prefix:** `CONF-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Memory Policy](./05-memory-policy.md), [Bridge Versioning](./08-bridge-versioning.md), [Libraries Specification](../02%20components/framework/09-libraries-specification.md)
- **Satisfies:** LANG-03, SEC-02, SEC-03, SEC-07
