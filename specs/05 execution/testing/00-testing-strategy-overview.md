# 00 — Testing Strategy Overview

Testing in Clean Language is split across many components — a compiler, a framework of plugins, a host server, a manager CLI, an extension, a dashboard, user apps — and no single test suite covers them all. This document is the strategy that binds those component-level plans together: the shared layer taxonomy every component's `TESTING.md` inherits, the cross-component decisions the strategy rests on, the per-component template, and a summary of when each layer runs. Read this before opening any single-component chapter; the tools, corpora, and skills the strategy depends on live in [12 — Testing Infrastructure Catalog](./12-testing-infrastructure-catalog.md), and the ordered plan for standing them up lives in [13 — Roadmap](./13-roadmap.md).

Three documented decisions the strategy builds on (all still to build unless otherwise noted):

1. The `tests:` block as the language-level unit-test facility (spec drafted in `foundation/04 language/11-testing.md`; runner in catalog [1.1](./12-testing-infrastructure-catalog.md#11-tests-block-runner)).
2. The four-layer conformance strategy for stdlib parsers (spec drafted in `foundation/04 language/11-testing.md §Conformance`; runner in catalog [5.2](./12-testing-infrastructure-catalog.md#52-conformance-corpus-runner)).
3. The `report_error` → ready-queue → `commit-scan` → `resolve-fix` loop for structured bug tracking (dashboard and skills in catalog §10 and §11).

---

## How to read this folder

Read in this order, once:

1. This document — the ecosystem-wide strategy, principles, and template.
2. [12 — Testing Infrastructure Catalog](./12-testing-infrastructure-catalog.md) — every tool, script, and workflow the strategy depends on, with build status.
3. [13 — Roadmap](./13-roadmap.md) — ordered plan for standing that infrastructure up.
4. [10 — Test Types Reference](./10-test-types-reference.md) — the nine test layers used across every component doc.
5. [11 — Test Generation Prompts](./11-test-generation-prompts.md) — per-component prompts for AI (or human) authors generating tests.
6. [09 — AI Review Agent Strategy](./09-ai-review-agent-strategy.md) — how AI review fits into the workflow.
7. The individual component strategy documents (01–08) — read only the one for the component you own or are touching.

After the first read, treat 01–08 as reference. Each is self-contained; you can jump to any of them directly.

## Glossary of ecosystem terms used in this folder

All entries below are **to build or adopt** unless noted. Full definitions and status live in [12 — Testing Infrastructure Catalog](./12-testing-infrastructure-catalog.md); this glossary is only the short form.

- **`comita`** ([catalog 9.1](./12-testing-infrastructure-catalog.md#91-comita--commit-and-release-recipe)) — the developer's commit-and-release recipe. Runs preflight checks, commits, tags, pushes, waits for CI, installs the new release, rebuilds plugins, resolves fixed bugs.
- **`/fix`** ([catalog 11.1](./12-testing-infrastructure-catalog.md#111-fix)) — component-scoped skill that drains the ready queue for the component matching the current working directory.
- **`report_error`** ([catalog 10.1](./12-testing-infrastructure-catalog.md#101-report_error-mcp-tool)) — MCP tool that files a reproducible bug into the errors dashboard. Returns a stable fingerprint.
- **Fingerprint** ([catalog 10.7](./12-testing-infrastructure-catalog.md#107-fingerprint-algorithm)) — stable 12-hex-char identifier for a reported bug. Same reproducer → same fingerprint.
- **Ready queue** ([catalog 10.2](./12-testing-infrastructure-catalog.md#102-ready-queue-endpoint)) — ordered list of open bugs owned by a component.
- **`commit-scan`** ([catalog 9.3](./12-testing-infrastructure-catalog.md#93-commit-scanshsh)) — script that parses commit messages for fingerprint references and advances bugs from `reported` to `fix_committed`.
- **`tests:` block** ([catalog 1.1](./12-testing-infrastructure-catalog.md#11-tests-block-runner)) — the language's built-in test facility. Spec in `foundation/04 language/11-testing.md`.
- **WIT** — WebAssembly Interface Types. The interface-description language at the boundary between compiled Clean components and their host. Third-party, adopted.
- **Host bridge** — the set of WIT-declared functions a host provides to a Clean component. Spec in `foundation/03 platform/02-host-bridge.md` (to build).
- **Block handler** — a plugin-owned expansion for a named block (`data:`, `handle:`, `html:`, `tests:`, etc.) in Clean source. Owned by whichever framework plugin declares it.
- **LSP** — Language Server Protocol. Standard IDE↔language-server protocol; the VS Code extension is one consumer.
- **Memory files** — per-session persisted notes at `~/.claude/projects/-Users-earcandy-Documents-Dev-Clean-Language/memory/`. Referenced by slug (e.g. `feedback_no_workarounds`) — the slug is the filename without `.md`.
- **`errors.cleanlanguage.dev`** ([catalog §10](./12-testing-infrastructure-catalog.md#10-errors-dashboard-errorscleanlanguagedev)) — the dashboard where fingerprints live. Also a component being built ([08](./08-clean-errors-dashboard-testing.md)).

---

## Why this document exists

Two problems drove this:

- The project is being built from scratch. Without a shared testing strategy defined up front, each component's maintainer will make ad-hoc choices, coverage will be uneven, and the boundaries between components (compiler ↔ plugin ↔ host) — where regressions actually live — will be nobody's job.
- The AI Code Review course (Qodo / DeepLearning.AI, deep-dive in `foundation/ai_code_review_deep_dive.md`) surfaces techniques — pre-PR local review, reviewer/generator persona separation, RAG-backed context, multi-agent ensembles, feedback-loop governance — that fit Clean's planned machinery (error codes with a 1:1 rule mapping, fingerprinted bugs, WIT interfaces, `comita` release loop) especially well. Deciding on that fit now, before any of it is built, lets us design the machinery to be review-friendly rather than bolting review on later.

---

## Guiding principles

1. **Fix root causes, not tests.** When a test fails, fix the code, never the test. This applies to snapshot updates too — an unexpected diagnostic diff is a design question, not a "just accept the new snapshot" reflex.
2. **No workarounds.** Testing infrastructure must actively detect the "just work around the compiler bug" pattern. The workaround-detector agent ([09 §3](./09-ai-review-agent-strategy.md#3-the-ensemble-split-clean-specific)) is the enforcement point.
3. **Real dependencies over mocks at the boundary.** Integration tests hit real Postgres/MySQL via Testcontainers, real Wasmtime, real host bridges. Mocks are acceptable for pure-unit tests inside a single component, never at the seam where two components meet.
4. **Every error code has a snapshot test.** The error-code registry (`foundation/03 platform/09-error-codes.md`, to build) and the semantic-rule registry (`10-semantic-rules.md`, to build) keep a 1:1 mapping. This folder extends the rule to tests: every documented error code MUST have at least one snapshot test that produces it, and at least one test that verifies its diagnostic renders per `13-diagnostic-format.md`.
5. **Cross-component contracts are tested twice.** Once by the producer, once by the consumer. WIT signatures are the contract. `check_host_parity.py` ([catalog 9.4](./12-testing-infrastructure-catalog.md#94-check_host_paritypy)) is one instance of the pattern, applied to the host↔registry boundary.
6. **The dashboard is the test outcome.** A red CI run is a symptom; a `report_error`-filed bug is the outcome that survives. Every reproducible failure in CI must be filed with a fingerprint so it enters the ready queue and gets picked up by `/fix`.

---

## Component list

Each component owns its own `TESTING.md` in its repo, following the template in the last section of this file. Documents in this folder are the *strategy*, not the runbook — they say what to test and why. The per-repo `TESTING.md` says how.

| # | Component | Repo | Strategy doc |
|---|---|---|---|
| 1 | Compiler | `clean-compiler` | [01-compiler-testing.md](./01-compiler-testing.md) |
| 2 | Framework + plugins | `clean-framework` (contains `plugins/frame.*`) | [02-framework-and-plugins-testing.md](./02-framework-and-plugins-testing.md) |
| 3 | clean-server (host) | `clean-server` | [03-clean-server-host-testing.md](./03-clean-server-host-testing.md) |
| 4 | Manager (`cln`) | `clean-manager` | [04-manager-cli-testing.md](./04-manager-cli-testing.md) |
| 5 | VS Code extension / LSP | `clean-extension` | [05-extension-lsp-testing.md](./05-extension-lsp-testing.md) |
| 6 | Standard-library parsers | inside `clean-compiler` | [06-stdlib-conformance-testing.md](./06-stdlib-conformance-testing.md) |
| 7 | Clean Studio + Clean user apps | `clean-studio`, `travelows`, etc. | [07-clean-studio-and-user-apps-testing.md](./07-clean-studio-and-user-apps-testing.md) |
| 8 | Errors dashboard | `clean-errors` | [08-clean-errors-dashboard-testing.md](./08-clean-errors-dashboard-testing.md) |
| 9 | AI review agents (cross-cutting) | tooling, not a shipped component | [09-ai-review-agent-strategy.md](./09-ai-review-agent-strategy.md) |
| — | Test-types reference (cross-cutting) | — | [10-test-types-reference.md](./10-test-types-reference.md) |
| — | Test-generation prompts (cross-cutting) | — | [11-test-generation-prompts.md](./11-test-generation-prompts.md) |
| — | Testing infrastructure catalog | — | [12-testing-infrastructure-catalog.md](./12-testing-infrastructure-catalog.md) |
| — | Roadmap | — | [13-roadmap.md](./13-roadmap.md) |

Sibling runtimes (`clean-ui`, `clean-canvas`, `clean-llm`) reuse the plugin strategy in [02](./02-framework-and-plugins-testing.md); no separate document until their surface diverges.

---

## The test-layer taxonomy

Every component's strategy is expressed as which layers it invests in. The layers, in ascending cost and descending frequency:

| Layer | What it proves | Runs |
|---|---|---|
| **L0 — Type/compile** | The code compiles, the WIT world is satisfied. | Every save in the editor, every build. |
| **L1 — Unit** | A single function or module behaves per spec. | Every push, seconds. |
| **L2 — Snapshot** | A structured output (AST, diagnostic, generated code, rendered HTML) matches a reviewed baseline file checked into the repo. | Every push, seconds. |
| **L3 — Property** | An invariant holds over hundreds of generated inputs (round-trip, monoid laws, idempotence). | Every push, seconds-to-minutes. |
| **L4 — Integration** | Two or more real components interact correctly (compiler + host, plugin + bridge, HTTP handler + DB). | Every PR, minutes. |
| **L5 — Conformance corpus** | An external, authoritative test suite passes (JSONTestSuite, WAST, wit-bindgen runtime tests, Test262). | Every PR, minutes. |
| **L6 — E2E / user-facing** | A real user path works end-to-end in a real browser or CLI. | Every PR for user-facing components, minutes. |
| **L7 — Fuzz / differential** | Randomised inputs surface panics (fuzz); two implementations of the same contract are compared (differential). | Nightly. |
| **L8 — AI review** | An ensemble of specialised review agents inspects the diff with RAG-supplied context (see [09](./09-ai-review-agent-strategy.md)). | Pre-PR local (`comita` STEP 0.6) + on PR open. |

Each layer is defined in full — with worked examples and common mistakes — in [10 — Test Types Reference](./10-test-types-reference.md). Read that document once; treat it as the reference every component doc points back to.

No component runs all nine layers. Each strategy document declares which layers apply, why the others are skipped, and what would trigger adopting a skipped layer later.

---

## Per-component testing strategy template

Each per-repo `TESTING.md` MUST have these sections, in this order, at this depth:

```markdown
# <component> — Testing Strategy

## 1. Surface being tested
- Public API / WIT interface(s) fulfilled
- Consumers (which other components depend on this)
- Blast radius (what breaks if this ships broken)

## 2. Layers in use
Table: layer → tool → runs-on → owner. Justify absences.

## 3. Golden bugs
The classes of failure this component is expected to see, with the layer intended to catch each. Populate initially from design analysis; grow with real fingerprints as the component ships. Also serves as the training set for the AI reviewer benchmark ([09 §5](./09-ai-review-agent-strategy.md#5-benchmarking) / [catalog 8.4](./12-testing-infrastructure-catalog.md#84-golden-pr-benchmark-corpus)).

## 4. Boundary contracts
Every WIT interface, HTTP contract, or file-format this component reads/writes. For each: contract owner, drift-detection mechanism, test location.

## 5. Fingerprint discipline
Which failures file `report_error`, which don't, and why. Component-owned fingerprint prefix.

## 6. Review-agent config
Which agents from [09](./09-ai-review-agent-strategy.md) run pre-PR. Which spec sections and memory files are indexed into the RAG context.

## 7. Known gaps
Layers not yet in use, corpora not yet vendored, contracts not yet tested. Each gap linked to a dashboard fingerprint that tracks closing it.
```

---

## Where this fits in the workflow

Once the roadmap in [13](./13-roadmap.md) has advanced through Phase 2 and Phase 5:

- **Pre-PR:** `comita` STEP 0 (dev-queue preflight), STEP 0.5 (host-parity preflight), STEP 0.6 (AI reviewer ensemble, see [09](./09-ai-review-agent-strategy.md)).
- **On push:** L0–L3 in CI, minutes.
- **On PR / tag:** L4–L6 in CI. Failing runs file `report_error` automatically ([09 §6](./09-ai-review-agent-strategy.md#6-fingerprint-agent-feedback-loop)).
- **Post-merge:** `commit-scan` in `comita` STEP 3.5 advances any referenced fingerprints to `fix_committed`. `resolve-fix` closes them at STEP 7.
- **Nightly:** L7 fuzz, differential, cross-engine WASM parity when a second engine target is committed.

---

## Non-goals

- This folder does not standardise the choice of unit-test framework per language. Rust picks its own; TypeScript picks its own. What is standardised is the *layer coverage* and the *contract boundaries*, not the tooling.
- This is not a replacement for `foundation/04 language/11-testing.md`. That is the *language spec* for the `tests:` block. This is the *ecosystem strategy* for testing components.
- No component-level `TESTING.md` may lower the bar set here without an entry in that file's §7 (Known gaps) linking to a fingerprint tracking closure.

---

## Metadata

- **Status:** Draft (2026-08-04)
- **Audience:** Any maintainer authoring or reviewing a component's `TESTING.md`, or reading the ecosystem strategy end-to-end
- **References:** [`README.md`](./README.md), [`12-testing-infrastructure-catalog.md`](./12-testing-infrastructure-catalog.md), [`13-roadmap.md`](./13-roadmap.md)
