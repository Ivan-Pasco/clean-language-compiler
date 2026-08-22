# ADR-0008 — Error-reporting backend: reference design lives with `clean-errors`

Two platform chapters had absorbed the internals of the `clean-errors` backend — tarball layouts, cron behaviour, dashboard scoring weights, wasmtime capture mechanics — with normative citations to specific code file line numbers and clauses declaring "when this document and the code disagree, the code wins." This ADR names `clean-errors` as a full component, moves its internal design to its own repository, and returns the platform chapters to observable contracts only, restoring the spec-over-code precedence.

---

## Context

The error-reporting feedback loop — users hit bugs, structured reports reach maintainers, fixes ship, users are notified — is implemented by a backend component, **`clean-errors`**, that runs the report ingestion API, the retest sandbox, the maintainer dashboard, and the fix-notification pipeline. Two spec chapters had absorbed that component's internals:

- [12 §11](../../03%20platform/12-server-extensions.md#11-dev-mode-capture) specified the retest sandbox's tarball layout, `pass_criteria` schema, and selection rules by citing `clean-errors` code paths with line numbers (`entrypoint.sh:120-212`, `retest-cron.sh:217-237`) as normative references — and declared "when this document and the sandbox code disagree, the code wins," inverting [SDD-08](../03-spec-driven-design.md) (rank-1 governance: when code and spec disagree, the code is wrong by definition).
- [06 §6.5 and §6.7](../../03%20platform/06-error-reporting.md#65-priority-scoring) specified the dashboard's priority-scoring weights and the backend's rate-limit numbers, and §6.9–6.10 the Rust-level trap-capture mechanics (`catch_unwind`, wasmtime feature tables) — all mechanism internal to one implementation, per [SDD-02](../03-spec-driven-design.md).

`clean-errors` also had no row in [Architecture Boundaries](../01-architecture-boundaries.md), so the responsibilities it absorbed had no declared owner. Conflict-log resolution P9 (approved 2026-08-01) removed the code-wins clauses and mandated this extraction.

## Options considered

**A — Keep the backend internals in the spec chapters.** Spec text pinned to code line numbers drifts on every commit, and the code-wins clauses were the symptom: the chapter had already surrendered authority it should never have claimed. Rejected.

**B — Promote the backend internals to full platform-spec status.** Would make the dashboard's scoring weights and cron behavior binding on any conformant implementation — but they are tunables of one service, not contracts anyone else builds against. Rejected.

**C — Record the design decision here; the detail lives in the `clean-errors` component repository; specs keep observable contracts only.** Chosen.

## Decision

**Option C.** The internal design of the error-reporting backend belongs to the **`clean-errors`** component and is documented in that component's own repository, not in the platform specification. That design comprises, as of this decision: the retest sandbox (tarball-driven reproduction with `pass_criteria` modes `compile_error` / `runtime_crash` / `wrong_output`, executed by the sandbox entrypoint), the retest cron that walks open reports, the maintainer dashboard with its priority-scoring queue, and the backend rate limits and ingestion endpoints.

The platform specification keeps only the **observable contracts**: in 06, the report schema and consent model, fingerprinting, the lifecycle stages, the CLI/MCP/HTTP surfaces, and the requirement that hosts translate traps into reports; in 12, the `diagnostics` WIT interface (renamed from `dev-mode` per [ADR-0005](0005-server-world-interface-additions.md)) and the wire shape of what the capture endpoint returns. Nothing in a platform chapter may cite `clean-errors` file paths or line numbers, and no spec text may declare shipped code authoritative over the spec ([SDD-08](../03-spec-driven-design.md)).

`clean-errors` becomes a named component in [Architecture Boundaries](../01-architecture-boundaries.md), responsible for error-report ingestion, the retest sandbox, the fix-notification pipeline, and the maintainer dashboard — and explicitly not responsible for emitting diagnostics, defining diagnostic codes, or setting consent policy.

## Consequences

**Easier.** The backend can tune scoring weights, rate limits, and sandbox mechanics without spec revisions; the spec chapters stop drifting against a moving codebase; the precedence inversion is gone.

**Harder.** The `clean-errors` repository must actually carry the extracted design documentation, and the boundary between "observable contract" (spec) and "backend behavior" (component doc) must be policed at review time — the tarball layout, for instance, is a producer↔sandbox contract whose home is now the component, with the spec keeping only what hosts and the framework must produce.

**Now required (DOC-07).** 12 §11 is reduced to the `diagnostics` interface and capture-payload contract, deleting the code-wins clauses and code-path citations; 06 §6.5 and §6.7 are replaced by pointers here; 06 §6.9–6.10 keep the host obligation ("every host translates traps and bridge failures into reports") and move the wasmtime-specific capture mechanics here; the `clean-errors` boundary row is added to [Architecture Boundaries §2](../01-architecture-boundaries.md).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [12 — Server Extensions §11](../../03%20platform/12-server-extensions.md#11-dev-mode-capture), [06 — Error Reporting §6.5, §6.7, §6.9–6.10](../../03%20platform/06-error-reporting.md) (internal mechanism moves out; the chapters keep the observable contracts)
