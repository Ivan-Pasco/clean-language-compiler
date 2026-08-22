# ADR-0027 — Tiered coverage policy with target enforcement

The ecosystem currently has ad-hoc coverage floors set per repo — 18% in the compiler, 40% in compiler-testing, nothing elsewhere. That is not a policy; it is what each repo happened to measure the day the workflow was written. This ADR replaces it with a **tiered policy** where every component knows its **target floor**, and every PR that touches an under-covered module must move it toward that target. The target is the enforcement point, not "someday" — AI agents and human contributors both need one number to converge on per module.

---

## Context

Clean Language is **mission-critical for its users but not life-certified**: user apps compile through it, their runtime binds through it, their production stacks depend on its bridges. A silent codegen bug corrupts every user program; a bridge marshalling bug silently corrupts every plugin call. That places the ecosystem closer to SQLite / Kubernetes than to DO-178C avionics.

Two industry facts shape the choice:

1. **100% line coverage is neither achievable nor useful.** It does not measure whether asserts are meaningful, whether error paths are hit, or whether concurrent code is safe. No serious mission-critical system uses "100% lines" as its bar. DO-178C Level A requires 100% branch + **MC/DC** (modified condition/decision coverage) on critical modules — a stronger, different measure. SQLite reaches ~100% branch + MC/DC on core, backed by extensive fuzzing.
2. **What you cover matters more than how much.** A 60% number that includes every error path and state transition beats 90% that misses one panic branch in codegen. Coverage without asserts is theatre; the counter-measure is **mutation testing**, which measures whether tests would actually fail if the code broke.

Existing measurements (2026-06-28): `clean-language-compiler` at 19.71% line, `clean-language-compiler-testing` at ~40%. Neither has branch or mutation numbers recorded. Neither is close to what mission-critical operation warrants. The gap is not "wait until we improve" — the gap is that nobody knows what "improved" would mean.

## Decision

Adopt a **four-tier coverage policy** with per-tier target floors, MC/DC on named critical modules, mutation-score gates on Tier 1 and Tier 2, and a **ratchet rule** that turns every PR into forward motion toward the target.

### Tier assignment by blast radius

| Tier | Blast radius | Components |
|---|---|---|
| **Tier 1 — Core correctness** | A bug silently corrupts every user program or every plugin call | `clean-language-compiler` (all crates), `clean-framework` core, `frame.data`, `frame.auth`, `clean-server` bridge layer |
| **Tier 2 — Hosts and CLI** | A bug breaks a component; users can pin an older version to recover | `clean-server` (non-bridge), `clean-manager` (`cln`), `clean-runner` |
| **Tier 3 — Plugins, extensions, dashboards** | A bug degrades a feature; the failure is contained to that plugin or surface | `frame.ui`, `frame.canvas`, `frame.locale`, `frame.jobs`, `frame.storage`, `frame.mcp`, `frame.client`, `clean-extension`, `clean-errors` dashboard, `clean-studio` |
| **Tier 4 — Tools, canaries, glue** | Coverage is not the right measure; correctness is proven by these tools testing other things | `clean-language-compiler-testing`, `canary_runner`, `verifier`, tooling scripts |

### Target floors per tier

The target floor is the number a component MUST reach and MUST NOT drop below once reached. It is what CI enforces the day the tier target is hit; it is what every PR is measured against.

| Tier | Line | Branch | Mutation score | MC/DC required on |
|---|---|---|---|---|
| **Tier 1** | **80%** | **75%** | **60%** | codegen, type-checker, bridge marshalling, memory management *(parked — see enforcement section)* |
| **Tier 2** | **70%** | **60%** | **50%** on critical paths (HTTP handling, install/uninstall, WASM loading) | — |
| **Tier 3** | **60%** | **50%** | — | — |
| **Tier 4** | No floor | No floor | — | — |

Rationale for the specific numbers:

- **80% line + 75% branch (Tier 1)** — the accepted threshold for serious open-source runtimes (Rust std, Go std, LLVM core). Below 80%, untested branches accumulate; above 90%, effort shifts to synthetic tests with diminishing return. The 5-point gap between line and branch is deliberate: branch is always harder because error paths and short-circuits are hard to hit, and forcing 75% branch is what makes error handling actually get tested — where mission-critical bugs live.
- **60% mutation score (Tier 1)** — mutation testing introduces small bugs (`+` → `-`, `>` → `>=`, `true` → `false`) and asks "would any test fail?". 60% means the ecosystem catches 60% of introduced bugs; anything below that means tests are running the code without meaningfully asserting on it. Industry reference: the `cargo-mutants` project cites 60–70% as "healthy" for Rust libraries.
- **MC/DC only on named critical modules** — MC/DC on the whole compiler is impossible without ~10× current test volume. On codegen and the type-checker specifically, it is achievable and it catches the exact bug class that would silently corrupt user programs. Named modules are enumerated in the workflow file, not "critical" as a loose adjective.
- **70/60 for Tier 2** — hosts have larger surface than compilers and more of it is glue (HTTP framing, filesystem I/O) that is not the crown-jewel logic. Setting the floor lower matches where testing effort has real yield.
- **60/50 for Tier 3** — plugins have small individual surfaces; forcing higher would mean writing tests for setup glue. 60% keeps discipline without wasting effort.
- **No floor for Tier 4** — testing infrastructure is judged by whether it catches bugs in the systems it tests, not by its own coverage.

### The ratchet rule (all tiers, always)

1. **Every Rust component records its current baseline** in its Coverage workflow file with a comment:
   `# baseline X% measured YYYY-MM-DD, target Y% per ADR-0027 Tier N`
2. **The floor can only go up, never down.** A PR that reduces coverage below the current baseline **fails CI**.
3. **A PR that improves coverage by ≥2 points MUST bump the baseline in the same PR.** Missed bumps are a review-comment issue.
4. **Any PR that adds or modifies code in a module below the tier target MUST bring that module's coverage to the tier target.** New code below target is not accepted, even if the whole-repo average stays above baseline.
5. **Quarterly review** — component owner reviews the delta between current baseline and tier target; if the gap has not narrowed, the component is flagged in the ecosystem quarterly report.

Rule 4 is what makes the target the enforcement point rather than a wishlist. Under this rule the whole-repo baseline drifts up naturally as work happens, but every new line of code is already at target — so as legacy modules get touched, the whole repo converges without a big-bang push.

### MC/DC and mutation-score enforcement

- **MC/DC is parked until the Rust toolchain offers a measure again** (toolchain reality verified 2026-08-20: `cargo-mcdc` was never published on crates.io, and rustc *removed* `-Z coverage-options=mcdc` — current nightly accepts only `block | branch | condition`). The named-modules *requirement* stands as policy; its enforcement is suspended, not waived. **Interim stand-in, normative until rustc's MC/DC returns:** each named module is gated *per-module* on the Tier 1 **branch floor (75%)** under `-Z coverage-options=condition` instrumentation — the strongest condition-sensitive measure the toolchain offers today. When an MC/DC-capable toolchain returns, the workflow switches the named modules back to MC/DC in a PR that must land with MC/DC already satisfied; the same rule applies to adding a new module to the named list.
- **Named-module → path mapping.** The Tier 1 MC/DC list ("codegen, type-checker, bridge marshalling, memory management") maps onto components as: `clean-language-compiler` — codegen → `src/codegen/`, type-checker → `src/typecheck/`, memory management → `src/layout.rs`; bridge marshalling is **`clean-server`'s** Tier 1 surface (its bridge layer), not the compiler's. Each component's workflow enumerates its own slice of the list by path.
- **Mutation score** is measured by `cargo-mutants` on Tier 1 and Tier 2, nightly (not per-PR, because it is slow). Where the mutant count exceeds a single nightly window (~30–90 min) — the compiler workspace counts 3 275 mutants under `cargo-mutants` 27.1.0 — the nightly run covers a **rotating shard** (compiler: 1/8, full set every 8 nights), with the tier mutation floor applied per shard and mutant timeouts counted as caught. Nightly failure opens a dashboard bug (`report_error`) rather than blocking merges. Merges are blocked only when the mutation score falls below the tier floor for **two consecutive nightly runs** — protects against flaky mutants. (Until enough CI history exists to automate the two-red-nights check, the workflow states the rule and the dashboard reader applies it.)
- **Operational note — `cargo-llvm-cov` version.** 0.6.x cannot locate object files under the build-dir layout newer cargo emits; coverage workflows pin 0.9.0 or later.

### Transition plan

Every component's Coverage workflow gains a `# baseline` and `# target` comment on the same PR that introduces this ADR. The initial baseline is whatever the workflow currently enforces (or the current measurement if none was set). The target is the tier floor for that component.

Convergence expectation, driven by rule 4:

- **Tier 1 components** — 4 quarters at current codebase-churn rate. The compiler has already converged ahead of that curve: from 19.71% at measurement (2026-06-28) to a blocking 80% line floor with baseline 87.73% (measured 2026-08-19), plus the nightly branch and mutation floors — see the reference-implementation note under *What becomes harder*. Remaining Tier 1 components (framework core, `frame.data`, `frame.auth`, `clean-server` bridge layer) keep the 4-quarter expectation.
- **Tier 2 components** — 2 quarters, smaller surface area.
- **Tier 3 components** — 1 quarter, most plugins are small enough that one focused PR-pass gets them there.

Convergence is a *prediction*, not a deadline. The enforceable rule is #4: no new code below target. Convergence follows mechanically from that.

## Options considered

- **A — One shared floor across every repo.** Simplest to communicate but wrong: compiler codegen has structural coverage properties fundamentally different from a plugin's public API. Forcing one number distorts what "well-tested" means per component and causes teams to game the number rather than improve tests. Rejected.

- **B — Per-component floors, each owner picks (status quo).** Zero coordination cost, but the current state is the demonstrated failure mode: 18% vs 40% with no principled reason. Rejected — this ADR exists specifically to fix this.

- **C — Tiered floors with ratchet rule (chosen).** Encodes blast radius as the driver of quality bar, gives every component one target to converge on, and rule 4 turns every PR into forward motion. Matches how mission-critical open-source runtimes (SQLite, Kubernetes core) organize their quality gates in practice.

- **D — MC/DC on everything.** DO-178C-style rigor. Would require ~10× test volume and dedicated testing headcount. Rejected: not proportionate to the criticality level; MC/DC on named modules captures the important payoff without the cost.

- **E — Mutation testing only, no coverage floors.** The purist argument: coverage is a proxy, mutation score is the truth. Rejected because mutation testing is slow (nightly at best) and expensive to debug; coverage floors give fast per-PR feedback that keeps mutation runs actionable rather than overwhelming.

## Consequences

**What becomes easier:**

- **One target per module for every contributor and every AI agent.** No more "is 22% good enough?" — the answer is "compare to the Tier N target in the workflow file".
- **The 18% vs 40% divergence problem is closed.** Each repo now has a principled assignment instead of an accidental number: the compiler is Tier 1 (80% line target plus the ratchet rule) and `clean-language-compiler-testing` is Tier 4 (no floor — it is judged by what it catches). The old percentages cease to be policy.
- **Rule 4 makes quality a per-PR habit.** New code lands at target; legacy gets fixed when it gets touched. No coordinated "coverage sprints" needed.
- **Mission-critical modules get condition-sensitive per-module gates** — today the 75% branch floor under `condition` instrumentation, and MC/DC (the strongest coverage measure short of formal verification) as soon as rustc offers a measure again — without imposing either on modules where it would be waste.

**What becomes harder:**

- **Every Rust component's Coverage workflow needs updating** to record baseline + target + tier reference and to enforce the ratchet rule and rule 4. Roughly one PR per component. The **working reference implementation is `clean-language-compiler`**, which already enforces the full amended policy: `ci.yml` (80% line floor, blocking per-push, baseline 87.73% measured 2026-08-19) and `nightly.yml` (75% branch workspace floor + per-named-module branch floors under `condition` instrumentation + 60% mutation floor on a rotating 1/8 shard with timeouts counted as caught). The template at [`foundation/scripts/reference-workflows/coverage.yml`](../../scripts/reference-workflows/coverage.yml) predates the 2026-08-20 amendment (it still invoked the nonexistent `cargo-mcdc`) and MUST NOT be copied until it is realigned with the amendment — realignment is tracked in the [rollout brief](../../work/2026-08-07-automation-rollout-per-component.md); acceptance of this ADR does not depend on it. Until then, new Tier 1/2 workflows copy the compiler's jobs.
- **`cargo-mutants` must be adopted** as an ecosystem tool (MC/DC tooling is parked — see enforcement above; the interim per-module branch gate uses the same `cargo-llvm-cov` the line/branch floors already require). Nightly runner time increases by ~30–90 min per Tier 1/2 component (sharded where the mutant count demands it). Runner cost is proportional and paid on Anthropic-owned runners.
- **Coverage measurement for `clean-framework` is currently absent.** Framework needs a coverage workflow written from scratch as a prerequisite to this ADR taking effect there. Tracked as a task in the framework component.
- **Contributors will occasionally hit rule 4 when touching legacy code.** The rule intentionally makes drive-by fixes to under-covered modules more expensive; the cost is real and is accepted as the mechanism of convergence. Reviewers may waive rule 4 with an explicit `coverage-waived: <justification>` line in the PR description — waivers are tracked in the quarterly review.

**Non-goals of this ADR:**

- **This is not a testing strategy.** Testing strategy per component lives in `foundation/05 execution/testing/`. This ADR sets the *bar*; the strategy documents describe *how* each component reaches it.
- **This does not replace fuzz, canaries, or reporter-artifact replay.** Coverage is one signal among many. See [`../../05 execution/automation/03-quality-gates.md`](../../05%20execution/automation/03-quality-gates.md) for the full gate inventory.

---

## Metadata

- **Status:** Accepted
- **Date:** 2026-08-07
- **Accepted:** 2026-08-20
- **Amended:** 2026-08-20 — toolchain reality check from the compiler's Milestone 9 (`clean-language-compiler/docs/DISCOVERIES-M9.md` §3): MC/DC parked pending rustc support (the cited `cargo-mcdc` does not exist; rustc removed `-Z coverage-options=mcdc`), with the per-module 75% branch floor under `condition` instrumentation as the normative interim gate; named-module → path mapping recorded (bridge marshalling assigned to clean-server); mutation runs sharded where the mutant count exceeds the nightly window; `cargo-llvm-cov` pinned ≥ 0.9.0.
- **Amended:** 2026-08-20 (acceptance pass) — stale claims aligned with the amendment before acceptance: the Consequences section no longer presents MC/DC as currently measurable (interim per-module branch gate named instead), `clean-language-compiler-testing` is correctly described as Tier 4 rather than sharing a Tier 1 target, the transition plan records that the compiler already enforces the full policy (ci.yml / nightly.yml named as the working reference implementation, baseline 87.73% line measured 2026-08-19), and the reference workflow template — which exists but predates the amendment — is quarantined pending realignment, tracked in the rollout brief. No floors, tiers, or mechanisms changed.
- **Supersedes:** None
- **Spec impact:** Every Tier 1–3 component gains a `Coverage` workflow enforcing tier target + ratchet + Rule 4. See [rollout brief](../../work/2026-08-07-automation-rollout-per-component.md) for per-component placeholder values.
