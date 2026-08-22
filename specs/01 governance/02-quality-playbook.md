# Clean Language V2 — Quality Playbook

Every component in the Clean Language ecosystem ships through the same quality regime: one command (`cln ship`), one shared MCP server exposing validators, one set of git and agent hooks that block bad work before it lands. This document is the complete list of those mechanisms and how each component wires them up. The design principle behind all of it: **hooks enforce; prompts request** — anything a human or an agent could forget must be mechanically blocked, not asked for politely.

---

## Part 0 — Introduction

### 0.1 Purpose and philosophy

Clean Language V2 is built primarily by AI agents. V1 taught us that when guardrails live in prose (CLAUDE.md, memory files, "please remember to…"), agents route around them. The cost was months of dashboard entries, library regressions, and spec drift.

V2 fixes this by adopting one rule: **hooks enforce; prompts request.** Everything a human or agent could forget must be mechanically blocked by an MCP tool, a git hook, an agent Stop hook, or CI. Prose exists to explain the *why*, not to be the *gate*.

Three decisions anchor the whole playbook:

1. One deterministic pipeline per component: `cln ship`. Built fresh in V2. `comita` becomes a V1 alias only.
2. One MCP server exposing many validator tools. Every agent (Claude Code, Codex CLI, Cursor, Aider) hits the same substrate.
3. Mutation score replaces line coverage. Snapshots + conformance tests handle WASM-side quality where mutation tooling doesn't exist.

Everything below follows from those three.

---

### 0.2 The 14 Elements (recap)

| # | Element | Layer | Enforced by |
|---|---------|-------|-------------|
| E1 | `cln ship` — single command | Foundation | Component Makefile / script |
| E2 | Pinned toolchain | Foundation | `toolchain.toml` |
| E3 | `cln strict` — single toggle | Foundation | Compiler flag / runtime function |
| E4 | MCP validators | Substrate | `cln mcp` server |
| E5 | Pre-commit hooks | Substrate | `.githooks/pre-commit` |
| E6 | Pre-push hooks | Substrate | `.githooks/pre-push` |
| E7 | Agent Stop hooks | Substrate | `.claude/hooks/`, `.codex/hooks/` |
| E8 | PreToolUse blocks | Substrate | `.claude/hooks/pretool.sh` |
| E9 | CI mirrors `cln ship` | Substrate | `.github/workflows/ship.yml` |
| E10 | Snapshot tests | Checks | `insta` (Rust), `vitest` snapshots (TS) |
| E11 | Property tests + fuzz | Checks | `proptest`, `cargo-fuzz` |
| E12 | Mutation score | Checks | `cargo-mutants`, `stryker-js` |
| E13 | Spec-implementation parity | Checks | `cln mcp` + CI |
| E14 | Library conformance + semver | Checks | `cargo-semver-checks`, `conformance/` dirs |

---

## Part 1 — Cross-Component Contracts

These contracts are identical across every component. A new component is not "V2-compliant" until it implements all of them.

### 1.1 `cln ship` — the one command

Every component ships a `cln ship` entry point that runs its full quality pipeline in strict order. Failure at any stage exits non-zero. There are no sub-commands, no flags for skipping stages (except `--fast` for Stop hooks, which runs a defined subset).

**Standard stages, in order:**

```
1. format-check     — file formatting matches canonical style
2. lint             — style + smell + banned-pattern checks
3. type-check       — full type analysis, -D warnings
4. spec-parity      — implementation matches foundation/spec/
5. unit             — unit tests
6. snapshot         — snapshot tests (insta / vitest)
7. property         — property tests (proptest)
8. integration      — integration tests
9. mutation         — mutation score gate
10. doc-code-check  — every fenced cln block in docs compiles
11. host-parity     — bridge signatures match `host function` declarations in library source
12. conformance     — library conformance corpus
```

A component may **skip** stages that don't apply (e.g. the docs component has no `mutation`), but it may not **add** custom stages outside this list without a Principle-25-style approval. Uniformity is the point.

**`--fast` subset** (for Stop hooks): stages 1, 2, 3, 4, 6, 10. Under 30 seconds on typical hardware. Full `cln ship` runs under 5 minutes.

### 1.2 `toolchain.toml` — pinned environment

Every component root contains `toolchain.toml`:

```toml
[toolchain]
rust = "1.85.0"
node = "22.13.0"
deno = "2.1.4"
library_abi = "2.0.0"
cln = "2.0.0"

[ci]
runner = "ubuntu-24.04"
image_digest = "sha256:..."
```

CI uses `image_digest`, not the tag. Toolchain upgrades are a deliberate commit, not a silent CI drift.

### 1.3 `cln strict` — the one toggle

One flag/function turns on every strict mode. In `cln.toml`:

```toml
[strict]
enabled = true
```

When enabled: compiler emits `-D warnings`, LSP disables fallback highlighting, library sandbox denies all unlisted host imports, framework rejects lazy-loading and missing-attribute access, ORM disables destructive operations without explicit opt-in, doc-code-validation runs at check time, coverage/mutation gates enforced.

Default: `enabled = true` in dev, `enabled = false` (log-only) permitted in production migration windows only. Turning strictness off requires a comment referencing an issue and an expiration date.

### 1.4 MCP validators — the cross-agent substrate

`cln mcp` (one server, multiple tools) exposes the enforcement layer to any agent. Required tools:

| Tool | Purpose |
|------|---------|
| `check` | Fast type-check a file or snippet |
| `validate` | Full pipeline check on a file (all stages 1-4) |
| `strict_check` | Verify `cln strict` compliance |
| `spec_parity` | Report drift between code and `foundation/spec/` |
| `conformance` | Run library conformance corpus |
| `mutation_preview` | Run mutation testing on changed files |
| `doc_check` | Compile every fenced block in a docs path |
| `report_error` | File a bug to the errors dashboard (existing) |
| `list_server_diagnostics` | Read runtime diagnostics (existing) |

Agents cannot claim "done" without a green `validate` on the changed surface. This is enforced at the Stop-hook layer.

### 1.5 Git hooks — the local floor

**`.githooks/pre-commit`** (fast, staged-file-scoped, <500ms):
- format-check on staged files
- lint on staged files
- banned-pattern check (JS in `.cln`, TODO without dashboard ref, `todo!()` in Rust tests)
- spec-parity delta for staged spec files

**`.githooks/pre-push`** (full, ~5 min):
- Runs `cln ship` end-to-end
- Exit non-zero blocks push

Both hooks activated by a `make init-hooks` at component setup, verified by CI (a "hooks-installed" check on each commit).

### 1.6 Agent hooks — the harness enforcement

**Claude Code** (`.claude/hooks/`):

- `SessionStart` — load STRATEGIC_FOCUS.md, current component context, open dashboard bugs
- `PreToolUse` — block writes to `foundation/spec/`, other components' folders, JS in `.cln`, destructive git commands. Narrow matchers only.
- `PostToolUse` — best-effort format-on-save. Nothing load-bearing here (known reliability bugs).
- `Stop` — run `cln ship --fast` on the changed component. `decision: "block"` on failure. `STOP_HOOK_ACTIVE` guard against loops.

**Codex CLI** (`.codex/hooks/`):

- `AfterAgent` — the Stop-hook equivalent. Same `cln ship --fast` invocation.
- `AfterToolUse` — same PostToolUse pattern.

Codex has no PreToolUse-block equivalent — Claude Code's PreToolUse is a bonus, not the primary defense. The primary defense for cross-agent parity is: MCP validators + git hooks.

### 1.7 CI — the merge gate

One workflow per component, ~15 lines:

```yaml
name: ship
on: [push, pull_request]
jobs:
  ship:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: ./.github/actions/setup-toolchain  # reads toolchain.toml
      - run: cln ship
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: mutation-report
          path: target/mutants/
```

No custom logic. If CI needs to do something local can't, it's a hidden gate — fix `cln ship` instead.

### 1.8 Test file layout

- All test files co-located with source in a `tests/` subdirectory of the module they test.
- Snapshot files live next to the tests that produce them (`__snapshots__/` or `*.snap`).
- Conformance corpus lives in `conformance/` at the library/component root.
- Fixture data lives in `tests/fixtures/`.
- The old `tests/cln/` and `tests/output/` global folders are V1 patterns and do not exist in V2.

### 1.9 Conformance testing for standard-library format parsers

A standard-library module that parses an external, standardised format — `json` today; TOML, YAML, URL or regex later — cannot be validated by hand-written cases alone. The examples an author remembers to write are not the surface of the format. Four layers are required, and each catches a class of defect the others cannot:

1. **External conformance corpus.** The format's own de-facto oracle, vendored at a pinned commit — never a floating clone. For JSON that is [JSONTestSuite](https://github.com/nst/JSONTestSuite): its `y_` cases MUST be accepted, its `n_` cases MUST be rejected, and its `i_` cases (implementation-defined) MUST match the conditions of the owning rules (layer 4 below). The corpus runs in the component repository's CI. **Exception — the specification outranks the corpus:** where an Accepted language rule rejects a case the upstream corpus marks `y_`, the rule wins; the case is recorded in the corpus `SOURCE.md` as an *expected divergence* and CI treats its rejection as the passing verdict. For JSON there are exactly two, both duplicate-key documents rejected by [`RUN009`](../03%20platform/10-semantic-rules.md)'s Accepted condition.
2. **Round-trip property tests.** `parse → serialize → parse` MUST yield a value equal to the first parse, over both the corpus and a randomised generator with bounded depth and mixed types. This catches serializer defects that a parse-only corpus cannot see.
3. **Hand-written Clean-side tests.** The Clean semantics the corpus knows nothing about: the type mapping onto Clean values, and which diagnostic each failure class raises. The codes themselves are owned by the specification ([04 language / 15 — Standard Library](../04%20language/15-standard-library.md) and the [Error Codes](../03%20platform/09-error-codes.md) registry), not by this playbook.
4. **Pinned decisions.** Resolved by [ADR-0010](decisions/0010-implementation-defined-parser-decisions.md): there is **no separate decisions document**. An implementation-defined case's resolution lives in the condition of the specification rule that rejects it — for JSON, [`RUN007`](../03%20platform/10-semantic-rules.md), [`RUN009`](../03%20platform/10-semantic-rules.md) and [`RUN010`](../03%20platform/10-semantic-rules.md) in Platform 10. The corpus asserts against those conditions; vendoring a new `i_*` case whose verdict no rule condition decides requires amending the owning rule in the same change. A parser's behaviour on a case no rule condition covers remains unspecified and MUST NOT be inferred from the implementation ([SDD-12](03-spec-driven-design.md)).

Layers 1–3 are CI configuration and live in the component repository ([EXE-04](04-execution-model.md)); layer 4 lives in the specification's rule entries.

### 1.10 Bug lifecycle (unchanged from V1)

Every discovered bug: `report_error` → `fix_committed` → `fix_released` → `fix_installed` → `resolved`. `cln ship` includes the version bump, tag, push, and `/resolve-fix` sequence. `TASKS.md` does not exist in V2 — the dashboard is the only queue.

---

## Part 2 — Per-Component Playbooks

Each playbook lists which `cln ship` stages apply, the specific tools, and the checks unique to that component. All playbooks assume the cross-component contracts from Part 1 are already in place.

### 2.1 Compiler (`cln-compiler`)

**Role:** Parse Clean Language → semantic analysis → emit WASM. Layer 0 in the execution model.

**`cln ship` stages:** all 12.

**Tooling:**
- Format: `cargo fmt --check`
- Lint: `cargo clippy -- -D warnings` with pinned toolchain
- Type-check: `cargo check --all-targets`
- Snapshots: `cargo insta test` for AST output, MIR output, WASM imports, LSP responses
- Property tests: `proptest` in `tests/properties/`
- Fuzz: `cargo-fuzz` in a nightly CI job (not blocking PRs)
- Mutation: `cargo-mutants` on `src/parser/`, `src/semantic/`, `src/codegen/`. Gate: 75% at V2 launch, ratchet to 85% by V2.5.
- Spec-parity: `cln mcp` compares `foundation/spec/grammar/grammar.ebnf` against parser productions, `foundation/spec/stdlib-reference.md` against registered builtins.

**Component-unique checks:**
- **Parse→format→parse identity** (property test): any AST that round-trips through the formatter must reparse to the same AST.
- **Semantic idempotence** (property test): running the semantic pass twice on the same AST produces identical output.
- **WASM imports match library `host function` declarations** (spec-parity): every generated WASM import must have a matching typed `host function` declaration in some installed library, and vice versa.
- **No library logic in compiler** (banned-pattern grep in pre-commit): matches Principle 26; blocks commits that add HTML/DSL/template logic to `src/libraries/host_adapter.rs` beyond stubs and escaping.

**Snapshots to maintain:**
- `tests/snapshots/parser/` — one snapshot per `foundation/spec/grammar/*.ebnf` production
- `tests/snapshots/semantic/` — one per numbered rule in `semantic-rules.md`
- `tests/snapshots/codegen/` — one per stdlib function group
- `tests/snapshots/wasm-imports/` — one per library ABI

**Agent hook additions:**
- `PreToolUse` blocks: `src/libraries/host_adapter.rs` requires a `principle-26-approved:` marker in the commit-message-in-progress.

### 2.2 Server (`cln-server`)

**Role:** Run WASM modules, provide HTTP + host bridge (Layer 2/3). Rust-only.

**`cln ship` stages:** all 12 except `mutation` (partial — native code only, not the WASM pathway).

**Tooling:**
- Same Rust toolchain as compiler.
- Snapshots: HTTP response fixtures per endpoint contract, bridge-function invocation traces.
- Property tests: request idempotence for `GET` endpoints, session-token round-trip.
- Host-parity: existing `check_host_parity.py` becomes `cln mcp`'s `strict_check --host server`. No standalone script.

**Component-unique checks:**
- **Bridge signatures match compiler-generated imports** (spec-parity): every WASM import the compiler emits must have a server-side implementation. Automated by walking `foundation/spec/platform/HOST_BRIDGE.md`.
- **No library knowledge in server** (banned-pattern): server implements the *bridge*, not library-specific behavior. Blocks commits that add library DSL parsing.
- **Response contract snapshots**: every route's response shape snapshotted per version. Changes require a semver bump.

**Snapshots to maintain:**
- `tests/snapshots/bridge/` — one per host bridge function
- `tests/snapshots/http/` — one per endpoint declared in any bundled library

### 2.3 Framework Libraries (`clean-framework/libraries/*`)

**Role:** Extend Clean Language with DSL blocks (`data:`, `endpoints:`, `component:`, etc.). Each library is a WASM module built from Clean source, plus a `library.toml` manifest and typed `host function` declarations.

**`cln ship` stages:** all except `mutation` (mutation tooling doesn't exist for WASM). Replaced by expanded `conformance` and `snapshot`.

**Tooling:**
- Format/lint: `cln fmt --check`, `cln lint` (Clean-native tools).
- Snapshots: library exports (function signatures), example-file outputs.
- Semver ABI: `cargo-semver-checks` on the set of typed `host function` declarations exported by the library (spec 23 §8) compared against the last released version.
- Conformance: every library ships `conformance/` with `.cln` files that any future version must still compile and execute identically.

**Component-unique checks:**
- **Method-collision guard** (banned-pattern): library-reserved names (`delete`, `exists`, `list`, `find`, `update`, `count`) blocked from user code when the library is loaded. Enforced by the compiler for every library in scope for the file's folder (`clean.toml [folders]`).
- **Bridge-declaration parity**: every host bridge function called in library code must have a matching typed `host function` declaration in library source (spec 23 §8). Every declaration must be used.
- **No JS anywhere** (banned-pattern in pre-commit and PreToolUse): `.js`/`.ts` files rejected in any library directory. If a browser interaction isn't in the `_ui_*` bridge, file `report_error` — do not write JS.
- **Sandbox capabilities**: `library.toml` must declare `[capabilities]` (filesystem paths, network origins, host functions). Loader denies unlisted capabilities at *load* time, not call time. No `--allow-all` equivalent.

**Per-library snapshots to maintain:**
- `conformance/` — canonical DSL usage examples
- `tests/snapshots/exports/` — WASM export signatures
- `tests/snapshots/expansions/` — DSL block → generated Clean code

### 2.4 Studio (`cln-studio`)

**Role:** SaaS IDE for Clean Language. Web frontend + local orchestration.

**`cln ship` stages:** format, lint, type-check, unit, snapshot, integration, doc-code-check. No mutation (JS side uses `stryker-js` in a separate nightly job — non-blocking). No `host-parity` (Studio doesn't ship a host).

**Tooling:**
- TypeScript with `strict: true`, `noUncheckedIndexedAccess: true`, `exactOptionalPropertyTypes: true`.
- Snapshots: Playwright screenshots for every page state (empty, populated, error, loading), Vitest snapshots for component render output.
- Integration: Playwright end-to-end tests for the golden path of each M-milestone feature.
- Doc-code-check: every `.cln` block in Studio-facing docs compiles.

**Component-unique checks:**
- **Route reachability** (property test): every declared route must be reachable from at least one link starting from `/`. No orphan routes.
- **Project-template compiles** (integration): every "New Project" template Studio can create must compile via `cln check` in the test suite.
- **AI-generated code always goes through `validate`**: Studio's AI-generation surface may not write files without a passing `validate` call. Enforced by a wrapper in the AI service layer.
- **PROJECT_STRUCTURE.md read enforcement** (Studio-specific): the AI orchestration layer must inject `PROJECT_STRUCTURE.md` into every generation prompt. Verified by a snapshot on the prompt-assembly function.

**Snapshots to maintain:**
- `tests/snapshots/pages/` — Playwright screenshots
- `tests/snapshots/prompts/` — AI prompt assembly output

### 2.5 VSCode Extension (`cln-extension`)

**Role:** Thin LSP client. No language logic.

**`cln ship` stages:** format, lint, type-check, unit, snapshot, integration.

**Tooling:**
- Same TS strict config as Studio.
- Snapshots: LSP request/response fixtures.
- Integration: VSCode Test Runner exercising the golden path.

**Component-unique checks:**
- **Thin-client guard** (banned-pattern in pre-commit): rejects any TypeScript file containing keyword lists, type definitions that duplicate the language server, or `.cln` parsing logic. `src/` matches against a small allowlist of terms (`connection`, `client`, `command`, `statusBar`, `activate`, `deactivate`). Anything else fails the check.
- **LSP protocol conformance**: extension must speak only stable LSP methods; custom methods require a spec entry.
- **No fallback highlighting**: TextMate grammar must be minimal (supplementary only). Semantic tokens come from the language server. Enforced by a snapshot on the grammar file size (hard cap).

### 2.6 CLI (`cln-cli`)

**Role:** User-facing command dispatch (`cln check`, `cln build`, `cln ship`, `cln strict`, library management).

**`cln ship` stages:** all except `mutation` (thin dispatch layer — mutation gives low signal) and `host-parity`.

**Tooling:**
- Same Rust toolchain.
- Snapshots: `cargo insta` on command output for every subcommand + help text.

**Component-unique checks:**
- **Every subcommand has a `--help` snapshot** (property test): iterates all registered subcommands, snapshots the help output. No undocumented commands.
- **Every subcommand has an integration test**: touches the actual filesystem in a temp directory. No pure-unit tests for CLI logic — the whole point is the shell contract.
- **Exit codes are semantic**: 0 success, 1 user error, 2 tool error (blocked by hook / policy), 3 internal error. Snapshot on the exit-code table.

### 2.7 Documentation (`foundation/docs/`)

**Role:** Language and platform documentation. The specification files under `foundation/docs/specification/` are the authoritative source of truth.

**`cln ship` stages:** format, lint, doc-code-check, spec-parity. Very focused.

**Tooling:**
- Format: markdownlint with a pinned config.
- Link check: `lychee` on every commit.
- Doc-code-check: `cln check` on every fenced `cln` block. Non-cln blocks (bash, toml) are checked for syntax where possible.
- Spec-parity: cross-references between `foundation/docs/specification/` and `foundation/spec/*` files must remain in sync.

**Component-unique checks:**
- **Every companion file exists**: `foundation/README.md` declares a 1-to-1 mapping between technical files and human companions. CI walks the mapping and fails if any file lacks its counterpart.
- **No unauthorized spec edits**: any diff touching `foundation/docs/specification/` or `foundation/spec/grammar/*.ebnf` must carry a `spec-approved:` trailer in the commit message signed by the language developer. PreToolUse blocks the edit otherwise.
- **Fenced-block compilation is the quality bar**: this is the single check that turns spec-parity from a prose principle into a mechanical guarantee.

---

## Part 3 — The MCP Server (`cln mcp`)

One server, many tools, shared across all agents.

### 3.1 Required tools

| Tool | Input | Output | Used by |
|------|-------|--------|---------|
| `check` | file path or snippet | diagnostics list | All agents, IDE, CI |
| `validate` | changed file paths | pass/fail + report | Stop hook, CI |
| `strict_check` | component name, target (server/browser) | pass/fail + missing list | Pre-push, CI |
| `spec_parity` | component name | drift report | Pre-push, CI |
| `conformance` | library name, version | pass/fail per test | CI, library release |
| `mutation_preview` | changed file paths | mutation score delta | Pre-push (optional) |
| `doc_check` | docs path | per-block compile result | Pre-push, CI |
| `report_error` | error metadata | dashboard fingerprint | All agents |
| `list_server_diagnostics` | component name | recent diagnostics | All agents |

### 3.2 Discovery and versioning

The MCP server declares its schema at startup. Every tool has a semver-versioned contract. Agents can query `mcp_version` to detect capability drift. Tools may be added without a major bump; removed only on major.

### 3.3 Instructions payload

The MCP server ships an `instructions` block loaded by every agent that connects. Its content:

1. Call `check` on any file before considering an edit complete.
2. Call `validate` on the changed component before ending a turn.
3. On bug discovery, call `report_error`. Never edit `TASKS.md`.
4. `cln strict` is on by default. If you need to disable it, ask the developer first.
5. Do not write JS in `.cln` projects. If a `_ui_*` bridge doesn't exist, call `report_error`.
6. Do not edit files in other components' folders.

These are the same rules as prose CLAUDE.md today, but delivered via MCP means every agent, not just Claude Code, receives them.

---

## Part 4 — Rollout Plan

V2 is a from-scratch build, so the rollout is a component-by-component enablement rather than a migration.

### 4.1 Phase 1 — Substrate (weeks 1-2)

1. Ship `cln mcp` with `check`, `validate`, `report_error`, `list_server_diagnostics` (the four load-bearing tools).
2. Ship `cln ship` skeleton — the command exists, runs whatever stages are configured, exits properly. Empty pipeline is OK.
3. Ship `.githooks/pre-commit` and `.githooks/pre-push` templates.
4. Ship `.claude/hooks/` and `.codex/hooks/` templates.
5. Ship the CI workflow template.

At end of phase 1: every component can adopt V2 tooling incrementally by wiring its stages into `cln ship`.

### 4.2 Phase 2 — Compiler + Server (weeks 3-6)

The compiler is the anchor. Every other component depends on it. Order:

1. Compiler: format, lint, type-check, unit, snapshot (parser first, then semantic, then codegen), spec-parity, mutation, property.
2. Server: format, lint, type-check, unit, snapshot, spec-parity, host-parity, integration.
3. `cln mcp` gains `strict_check`, `spec_parity`, `mutation_preview`.

At end of phase 2: `cln ship` on compiler and server is fully populated. Every other component has a working reference to copy.

### 4.3 Phase 3 — Libraries + CLI (weeks 7-9)

1. Every framework library gains `conformance/` corpus and semver ABI check.
2. `cln mcp` gains `conformance`.
3. CLI ships with help-text snapshots and integration tests.

### 4.4 Phase 4 — Studio + Extension + Docs (weeks 10-12)

1. Studio: TypeScript strict, Playwright suite, AI-generation wrapper.
2. Extension: thin-client guard, LSP fixtures.
3. Docs: fenced-block compilation, companion-file check.
4. `cln mcp` gains `doc_check`.

At end of phase 4: every component in the platform runs `cln ship`, every agent hits `cln mcp`, every merge goes through the same gate.

### 4.5 Ratcheting

Mutation-score gate starts at 75%, ratchets to 85% over V2.0 → V2.5 via monthly bumps of ~2%. Coverage-of-record is mutation; line coverage is not tracked.

Snapshot count is expected to grow monotonically per component. A PR that reduces snapshot count without a corresponding feature removal fails a CI check.

---

## Part 5 — Anti-Patterns (do not adopt)

These were considered and explicitly rejected:

- **100% line-coverage gates.** Inflated by assert-free AI-generated tests. Use mutation score instead.
- **LLM-as-reviewer as a gate.** Non-determinism at the enforcement layer.
- **Cursor rules / prose files as enforcement.** Prompts are requests; only hooks enforce.
- **Test-in-grammar** (test blocks as first-class syntax). Ergonomic sugar with no enforcement benefit. Keep tests as separate files.
- **TLA+ for compiler correctness.** Cost too high, ROI too low outside distributed systems.
- **PostToolUse for anything critical.** Documented reliability bugs. File already on disk. Load-bearing checks go in Stop.
- **AI-generated test suites** without mutation gating. Predictable failure mode.
- **`--no-verify`, `--allow-all`, blanket `#[allow]`.** No blanket escape hatches. Individual `#[allow(..., reason = "...")]` at call site is fine.
- **Baseline tolerance without an expiry.** Debt-carrying files must have a dashboard-tracked expiry date.

---

## Part 6 — Governance

### 6.1 Who can change this playbook

The playbook itself is a spec-adjacent document. Changes require:

- Wording, examples, phase-ordering: any maintainer, no approval.
- Adding/removing an element (E1-E14) or a component playbook: language developer approval, same as `foundation/spec/`.
- Changing the `cln ship` stage order or the required MCP tool list: language developer approval.

### 6.2 How this playbook is enforced

- CI runs a `playbook-parity` check on every commit: does each declared component ship the elements listed in its Part 2 section? Failure blocks the merge.
- New components added to the platform must ship a Part 2 playbook entry before their first `cln ship` run.
- Quarterly audit: mutation-score trend per component, snapshot-count trend, spec-parity drift count. Reported to the strategic-focus document.

### 6.3 Handoff to V1

V1 keeps running until every V2 component reaches Stable maturity. During the transition:

- V1's `comita` remains functional as an alias for `cln ship` on V1 components.
- V1's `TASKS.md` sweep continues in `comita` for as long as V1 lives.
- V1's dashboard entries continue to be resolved via `/resolve-fix`.
- V1 components do not adopt V2 elements piecemeal — the migration is per-component and complete.

---

## Appendix A — File layout of a V2-compliant component

```
cln-<component>/
├── cln.toml                    # component manifest, includes [strict]
├── toolchain.toml              # pinned toolchain
├── Cargo.toml / package.json   # native manifest
├── .githooks/
│   ├── pre-commit
│   └── pre-push
├── .claude/
│   └── hooks/
│       ├── session-start.sh
│       ├── pre-tool-use.sh
│       ├── post-tool-use.sh
│       └── stop.sh
├── .codex/
│   └── hooks/
│       ├── after-agent.sh
│       └── after-tool-use.sh
├── .github/
│   └── workflows/
│       └── ship.yml
├── scripts/
│   └── ship.sh                 # entry point for `cln ship`
├── src/
│   └── ...
├── tests/
│   ├── unit/
│   ├── integration/
│   ├── properties/
│   ├── snapshots/
│   └── fixtures/
├── conformance/                # library components only
│   └── *.cln
└── docs/
    └── ...                     # component-local docs; language docs live in foundation/docs/
```

## Appendix B — `cln ship` reference implementation (bash sketch)

```bash
#!/usr/bin/env bash
set -euo pipefail

FAST=false
[[ "${1:-}" == "--fast" ]] && FAST=true

run() {
  local name="$1"; shift
  echo "== $name =="
  "$@" || { echo "FAIL: $name"; exit 1; }
}

run format-check   scripts/stages/format-check.sh
run lint           scripts/stages/lint.sh
run type-check     scripts/stages/type-check.sh
run spec-parity    scripts/stages/spec-parity.sh

$FAST && { run snapshot scripts/stages/snapshot.sh; run doc-code-check scripts/stages/doc-code.sh; exit 0; }

run unit           scripts/stages/unit.sh
run snapshot       scripts/stages/snapshot.sh
run property       scripts/stages/property.sh
run integration    scripts/stages/integration.sh
run mutation       scripts/stages/mutation.sh
run doc-code-check scripts/stages/doc-code.sh
run host-parity    scripts/stages/host-parity.sh
run conformance    scripts/stages/conformance.sh

echo "SHIP OK"
```

Stages a component doesn't apply return exit 0 with a "not applicable" line, keeping the pipeline uniform.

---

**End of playbook.**

---

## Changelog

- 2026-08-19 — §1.9 brought up to date with the compiler's Milestone 6 (`clean-language-compiler/docs/DISCOVERIES-M6.md`, item 6n). Layer 4 rewritten: [ADR-0010](decisions/0010-implementation-defined-parser-decisions.md) resolved 2026-08-02 that implementation-defined verdicts live in the conditions of the owning Platform 10 rules, not in a separate decisions document — this section still described that document as pending. Layer 1 gains the expected-divergence rule the JSON gate needed: an Accepted language rule outranks an upstream `y_` verdict (JSON: two duplicate-key documents rejected by `RUN009`), with the divergence recorded in the corpus `SOURCE.md` and CI treating rejection as the pass.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Anyone building or maintaining a V2 component (compiler, server, framework libraries, Studio, extension, CLI, docs)
- **References:** [Documentation Principles](00-documentation-principles.md), [Architecture Boundaries](01-architecture-boundaries.md), [ADR-0022 — Foundational Technology Stack](decisions/0022-foundational-technology-stack.md)
