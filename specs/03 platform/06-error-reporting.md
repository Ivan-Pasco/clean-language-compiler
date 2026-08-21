# Platform 06. Error Reporting

Every Clean program, every library, and every host produces errors — parse failures, type mismatches, memory traps, bridge violations, business-logic exceptions. This chapter defines how those errors are captured, structured, transmitted, and closed out. The mechanism is a **feedback loop**: users hit bugs, structured reports reach the maintainers, fixes ship, and users are notified the fix is available. That loop is what makes a compiler and framework improve; the rules below are what makes it reliable, respectful of user privacy, and cheap enough to run always-on.

---

## 6.1 The Five Lifecycle Stages


Every error in Clean has exactly five stages. A fix is not "done" until stage 5.

| Stage | Meaning | Verified by |
|-------|---------|-------------|
| 1. `reported` | Bug logged via `report_error` MCP tool or CLI | MCP call succeeds |
| 2. `fix_committed` | Code fix pushed to git referencing the error's fingerprint | commit-scan matches SHA to fingerprint |
| 3. `fix_released` | Tagged release with CI pass | `git tag` present + CI green |
| 4. `fix_installed` | Fix active in the user's local dev environment | `cln --version` reports a version ≥ fix version |
| 5. `resolved` | Dashboard closed + affected users notified | `/resolve-fix` backend acknowledgment |

The lifecycle is enforced by tooling. A dashboard entry stuck at stage 2 for weeks is visible; a fix shipped without stage 5 leaves users unaware. See [§6.8](#68-tooling) for the CLI and MCP surface that walks entries through the stages.

---

## 6.2 Error Categories and Origins

| Origin | Layer | Examples | Reported by |
|--------|-------|----------|-------------|
| Compiler | L0 | Parse error, type mismatch, unresolved symbol, IR validation failure | Compiler CLI or LSP → `report_error` MCP tool |
| Runtime (WASM trap) | L1 | Out-of-bounds access, integer divide by zero, unreachable, stack overflow | Host translates wasmtime trap → structured report |
| Bridge violation | L2/L3 | Missing import, signature mismatch, WIT resource misuse | Host at instantiation or first call |
| Library expansion | L4 | Block handler emitted `error(code, message, span)` | Block handler → compiler → normal diagnostic path |
| Application logic | L5 | Uncaught `onError`, contract violation (`before`/`after`/`always`) | Runtime `panic` handler → structured report |
| Tooling | — | CLI crash, LSP crash, MCP crash, editor extension crash | Local crash handler → deferred report |

Every category funnels into the same report schema (§6.3) and the same lifecycle (§6.1). Downstream tooling can filter by category; upstream, they are one pipeline.

---

## 6.3 Report Schema


### REP-01 — The report schema


Every report MUST carry this shape (WIT-typed, in the `reporting` interface of the `clean:host` package — `clean:host/reporting@0.1.0`, proposed in [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)):

```wit
// interface reporting — package clean:host@0.1.0

record error-report {
    // Identity
    error-code: string,                     // PREFIX### per 09 §1 (e.g. "SEM001"); library-emitted
                                            // diagnostics travel as "LIB010" plus a library sub-label
    fingerprint: string,                    // Deterministic hash of the identity fields; see §6.4
    component: string,                      // compiler | server | browser | library:<name> | app
    severity: severity,                     // See below

    // Content
    error-message: string,                  // Human-readable summary
    expected-behavior: option<string>,      // What should happen
    actual-behavior: option<string>,        // What happened
    minimal-repro: option<string>,          // Smallest reproducing code (compile-time errors only)
    spec-reference: option<string>,         // Section of the spec the bug violates

    // Provenance
    compiler-version: string,               // Semantic version of the compiler that produced or observed this
    host-world: option<string>,             // World name per 15 §0.3: "server", "browser", etc.
    library-versions: list<library-version>,  // Every library in the compile
    target: string,                         // wasm32-server, wasm32-browser, etc.

    // Diagnostic bundle (schema v2.0.0 addition)
    diagnostic-data: option<diagnostic-bundle>,

    // AI-assisted context
    ai-analysis: option<ai-analysis>,

    // Privacy
    consent-level: consent-level,           // See §6.6
    anonymous-id: string,                   // Rotating per-session, not tied to identity
    user-contact: option<contact-info>,     // Only if consent-level >= identified
}

variant severity { crash, bug, regression, unexpected, enhancement }

// Ordered, lowest to highest: each level sends everything the previous level
// sends, plus its own additions (§6.6). Comparisons like "consent-level >=
// identified" refer to this declaration order.
variant consent-level { error-only, error-with-code, full, full-with-diagnostics, identified }

variant diagnostic-bundle {
    compiler-parse-failure(record {
        grammar-rule-stack: list<string>,
        token-at-failure: token-info,
        partial-ast: option<string>,          // Serialized AST built so far
    }),
    runtime-trap(record {
        trap-kind: trap-kind,
        wasm-backtrace: option<wasm-backtrace>,
        core-dump: option<list<u8>>,          // Guest linear memory dump if consent allows
    }),
    bridge-mismatch(record {
        expected-wit: string,
        actual-wit: string,
        interface: string,
    }),
    library-expansion(record {
        library-name: string,
        library-version: string,
        block-name: string,
        handler-name: string,
    }),
}

variant trap-kind {
    memory-out-of-bounds,
    integer-divide-by-zero,
    integer-overflow,
    invalid-conversion-to-integer,
    unreachable-executed,
    call-indirect-oob,
    stack-overflow,
    memory-grow-failure,       // From tier exhaustion — §05.3
    epoch-interruption,        // From time-budget exhaustion — §03.5
    host-error(string),
}

record ai-analysis {
    confidence: confidence-level,
    suggested-fix: option<string>,
    suggested-component: option<string>,
    suggested-file: option<string>,
}

variant confidence-level { high, medium, low }
```

All optional fields MUST be omitted (not sent as null) at consent levels that exclude them ([REP-02](#66-privacy-and-consent)).

---

## 6.4 Fingerprinting

Each report has a **fingerprint** — a 12-character hex prefix of SHA-256 over the identity fields:

```
input = error-code
      + "|" + component
      + "|" + major.minor version
      + "|" + normalized(error-message)
```

Normalization: strip file paths, strip line/column numbers, strip identifiers matching common naming patterns (replace with `_`), lowercase, collapse whitespace.

Fingerprints deduplicate. Two users hitting the same bug produce the same fingerprint; the dashboard groups their reports as a single entry with an occurrence count. Fingerprint stability across patch releases is what allows a fix committed against `#1f887f527998` to auto-resolve every user's report of the same bug.

---

## 6.5 Priority Scoring

The backend's **ready queue** orders open reports by a score derived from occurrence count, unique affected users, severity, AI-analysis confidence, and recency. The observable contract:

- Ordering is deterministic: the same set of reports always produces the same queue order.
- Scoring weights are stable within a major spec version.
- The `/fix` workflow (see [§6.8](#68-tooling)) works the queue top-down.

The concrete scoring formula and weights are backend design, owned by the `clean-errors` component ([ADR-0008 — Error-Reporting Backend](../01%20governance/decisions/0008-error-reporting-backend.md); extracted text in `work/2026-08-01-clean-errors-extraction-06.md`).

---

## 6.6 Privacy and Consent


### REP-02 — Report consent bounds every transmission


Every report carries a **consent level** the user has explicitly set. The compiler and hosts MUST NOT transmit fields beyond the consent scope. The five levels:

| Consent level | Sent | Not sent |
|---------------|------|----------|
| `error-only` (opt-out disabled) | Nothing. No reports transmitted. | Everything |
| `error-with-code` (default) | error-code, fingerprint, component, severity, compiler-version, host-world, target, anonymous-id | source snippets, message details, diagnostic bundle, contact |
| `full` | + error-message, expected/actual, minimal-repro (if compile-time), spec-reference, library-versions, ai-analysis | diagnostic bundle, contact |
| `full-with-diagnostics` | + diagnostic-data (including guest core dump for traps) | contact |
| `identified` (opt-in extra) | + user-contact | — |

Consent levels are totally ordered — `error-only < error-with-code < full < full-with-diagnostics < identified` — and each level sends everything the previous one sends.

- Default is `error-with-code`. The user is prompted on first CLI run and can change via `cln config set telemetry <level>`.
- The `error-only` level completely disables the pipeline. No fingerprint, no metrics, no dashboard visibility for that user.
- Contact info retention is bounded to 90 days on the backend; individual reports retained 1 year; aggregated metrics retained indefinitely.
- Pending local reports (queued while offline) live in `~/.cln/reports/pending/` for at most 30 days; older entries are deleted unsent.
- Core dumps at `full-with-diagnostics` level are stripped of any bytes matching heuristics for credentials (patterns matching common token shapes, entries under environment variable names like `*_TOKEN`, `*_KEY`, `*_SECRET`). This heuristic is a defence-in-depth backup for raw crash-dump bytes — the primary mechanism for secret redaction is the language-level `secret` type ([ADR-0023](../01%20governance/decisions/0023-secret-handling-strategy.md), [04 language / 04 — Type System § The secret type](../04%20language/04-type-system.md)), whose overridden `toString`, debug repr, and JSON serialisation emit `"[REDACTED]"` mechanically across every surface the compiler can track. The byte-pattern heuristic remains necessary as a belt-and-braces layer for bytes the compiler could not track: values that entered memory via FFI, unsafe host bridges, or code paths that predate the `secret` type.

**Report consent is not telemetry.** Clean Manager's `cln telemetry <on|off|status>` governs the adoption heartbeat ([Manager §00.10](../02%20components/manager/00-manager.md#0010-telemetry)); the consent level here governs error reports only. The two settings are independent.

- The default MUST be `error-with-code`. The user is prompted on first CLI run and can change via `cln report consent <level>` (command surface home: [Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface); behavior home: this section).
- The `error-only` level completely disables the pipeline: no report, no fingerprint, no metrics, no dashboard visibility for that user.
- The backend MUST NOT retain contact info beyond 90 days nor individual reports beyond 1 year; aggregated metrics MAY be retained indefinitely.
- Pending local reports (queued while offline) live in `~/.cln/reports/pending/`; entries older than 30 days MUST be deleted unsent.
- Core dumps transmitted at `full-with-diagnostics` MUST be redacted before transmission: the values of environment variables whose names match `*_TOKEN`, `*_KEY`, or `*_SECRET` MUST NOT appear in the transmitted dump. Additional token-shape patterns form a closed redaction list — (pending; the list is registered in this section when defined). Until that list exists, only the environment-variable rule is normative; any wider heuristic stripping is best-effort and grants no additional guarantee.
- A field added to the report schema MUST be assigned to a consent level in the same commit that adds it.

The privacy model is a **hard specification requirement**, not an implementation courtesy.

---

## 6.7 Rate Limits

Report submission is rate-limited. The observable contract:

- **Rate limiting never drops reports — it batches.** Reports over a limit are held locally and transmitted when the window opens.
- Limits are honored client-side first (a rolling counter under `~/.cln/reports/`); backend enforcement is a second line of defense.

The concrete limit values (per-IP, per-`anonymous-id`, global circuit breaker) are backend design, owned by the `clean-errors` component ([ADR-0008](../01%20governance/decisions/0008-error-reporting-backend.md); extracted text in `work/2026-08-01-clean-errors-extraction-06.md`).

---

## 6.8 Tooling

The reporting pipeline is exposed through three surfaces:

### 6.8.1 MCP tools (for AI clients)

These tools are served by the single Clean MCP server owned by the framework ([ADR-0001 — Single MCP Server](../01%20governance/decisions/0001-single-mcp-server.md)); the tool catalog home is [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md). This section owns their reporting behavior.

- `report_error(component, error_code, severity, error_message, expected_behavior, actual_behavior, minimal_repro, ai_analysis)` — file a new report.
- `check_reported_fixes(include_all?)` → `{fixes, pending, current_version, latest_version, has_updates}` — see if fixes shipped since the local version.
- `list_component_bugs(component)` → open reports for a component (session-start context).
- `list_server_diagnostics(host?)` → structured diagnostics captured by a running host.

### 6.8.2 CLI (for humans)

Command surface home: [Clean Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface); behavior home: this document.

- `cln report consent <level>` — set report-consent level (§6.6).
- `cln report` — interactive filing.
- `cln fixes` — list fixes released since your installed version.

Maintainer-side queue tooling (the dashboard's dev queue consumed by CI/`cln ship` release gating) is backend design owned by the `clean-errors` component ([ADR-0008](../01%20governance/decisions/0008-error-reporting-backend.md)); it is not part of the `cln` user surface.

### 6.8.3 HTTP API (for hosts and dashboards)

- `POST /api/v2/reports` — submit a report.
- `GET /api/v2/reports/:id/status` — query lifecycle stage.
- `GET /api/v2/fingerprints/:fp/status` — dedupe against an existing entry.
- `POST /api/v2/fingerprints/:fp/resolve` — mark stage 5 with the shipping version.
- `POST /api/v2/reports/commit-scan` — invoked by CI on push; matches SHAs to fingerprint references in commit messages and advances stage 2.
- `POST /api/v2/reports/release-tag` — invoked when a tag lands; advances stage 3.

All endpoints are versioned under `/api/v2/` and follow [§08 Bridge Versioning](./08-bridge-versioning.md) rules for compatibility.

---

## 6.9 Host-Side Trap Capture


### REP-03 — A host that observes a trap reports it


Every host MUST translate WebAssembly traps and bridge failures into `error-report`s. The observable contract:

- **A host that observes a trap MUST invoke `report-error`** (subject to the user's consent level — at `error-only`, nothing is transmitted). Observing a trap without attempting a report is silently swallowing signal. The contract test suite ([§02.7](./02-host-bridge.md#27-host-contract-testing)) verifies each host's trap-to-report translation.
- The report carries the structured trap kind, the guest backtrace, and — only at `full-with-diagnostics` consent — a core dump of guest memory.
- The failure is attributed to a component (`compiler | server | browser | library:<name> | app`) via the compiler's emitted debug symbols.
- Reports flow through the same rate-limited transport as every other report (§6.7).

The reference capture mechanism (engine configuration, unwind-catching per host language) is recorded in [ADR-0008](../01%20governance/decisions/0008-error-reporting-backend.md); extracted text in `work/2026-08-01-clean-errors-extraction-06.md`.

---

## 6.10 Runtime-Sourced Metadata

The reporting schema draws structured signal directly from the runtime rather than parsing free-form error strings — the trap kind, backtrace, consent-gated core dump, time-budget exhaustion ([03 §3.5](./03-memory-model.md#35-host-backing--observable-contract)), tier-exceeded grow ([§05.3](./05-memory-policy.md#53-enforcement)), and instance-linking mismatches ([§15.4](15-component-model-architecture.md)) each map to a dedicated report field. Using structured runtime signals is what makes the reports actionable at scale.

The engine-feature-to-field mapping table is reference design, recorded in [ADR-0008](../01%20governance/decisions/0008-error-reporting-backend.md); extracted text in `work/2026-08-01-clean-errors-extraction-06.md`.

---

## 6.11 Non-Goals

- **Real-time user visibility.** The dashboard is for maintainers. Users see fixes via `cln fixes`, not real-time streams.
- **Blaming.** The `component` field attributes a bug to whichever layer owns the fix. It is not a judgment about who is responsible.
- **Product analytics.** The pipeline reports errors, not feature usage. No "which functions are most popular" telemetry.
- **Automatic fix suggestions to users.** AI-assisted analysis is captured in the report to aid maintainers; the fix that reaches users comes from a released compiler version, not from a chatbot.

---

## 6.12 Deferred Refinements

1. **Federated dashboards.** Teams running their own hosts (enterprise deployments) may run their own dashboards. The API is designed for self-hosting. Optional forwarding of anonymized fingerprints to the public dashboard is a per-instance config knob, not a spec requirement.
2. **Fix-recommendation lookup during compile.** The compiler does not automatically query the dashboard for known fixes when emitting an error — automatic lookups would add network latency to every compile. Fix recommendations are surfaced only through the MCP tool `check_reported_fixes`.

---

## 6.13 Diagnostic Level → Report Severity Mapping


Diagnostics have a display **level** (`error / warning / info / help`) defined in [`13-diagnostic-format.md §3`](./13-diagnostic-format.md). Reports have a lifecycle **severity** (`crash / bug / regression / unexpected / enhancement`) declared in the report schema ([REP-01](#63-report-schema)). The two are different vocabularies for different audiences — display for the developer at their editor, severity for the maintainer triaging the dashboard.

### REP-04 — Every report originates from a coded diagnostic


Every report submitted through the pipeline MUST originate from a diagnostic that carries a code from [`09-error-codes.md`](./09-error-codes.md). Free-form user reports without a code are stored but MUST NOT enter the ready queue and MUST NOT receive a fingerprint.

When a diagnostic is filed as a report (via `report_error` or an automatic capture), the level MUST map to a severity by this table:

| Diagnostic level (from §13) | Compiler state | Report severity (§6.3) |
|-----------------------------|----------------|------------------------|
| `error` | compiler process itself crashed / aborted | `crash` |
| `error` | normal compile failure or runtime trap | `bug` |
| `error` | code compiled before, fails now on same input | `regression` |
| `warning` | valid code the compiler suspects is unintended | `unexpected` |
| `info` | observation, not a defect | `enhancement` |
| `help` | actionable suggestion attached to another diagnostic | (not reportable — filed as part of the parent) |

---

## Changelog

- 2026-08-01 — Technical-debt closure pass: REP-01 and REP-02 now also cite the new **C-22 — Privacy** concern ([05-concerns](../01%20governance/05-concerns.md)). Removed `cln dev-queue list` from the §6.8.2 CLI surface — it was maintainer tooling for the errors dashboard, extracted to the `clean-errors` component per [ADR-0008](../01%20governance/decisions/0008-error-reporting-backend.md); a pointer note replaces the verb.
- 2026-08-01 — Governance compliance (traceability pass): registered rule prefix `REP-` and minted REP-01 (report schema, C-02/C-20), REP-02 (consent bounds every transmission, C-14/C-19), REP-03 (host trap capture MUST report, C-15), REP-04 (every report originates from a coded diagnostic and maps level→severity by table, C-02/C-20) — all reusing the existing normative text. Sections §6.3, §6.6, §6.9, §6.13 marked *Normative*; §6.1 (lifecycle narrative, including "stuck for weeks is visible") marked *Informative*. §6.6 vague prose made checkable: retention and pending-report expiry as MUST NOT bounds; the credential-strip heuristic replaced by a minimal normative redaction rule (env-var names matching `*_TOKEN`/`*_KEY`/`*_SECRET` MUST NOT appear in a transmitted dump) with the wider token-shape pattern list left as a pending closed list; schema additions MUST be consent-classified in the same commit.
- 2026-08-01 — Conflict-log remediation (Fase 3): consent renamed to **report consent** per P12 — the surface is `cln report consent <level>` (Manager owns the command surface; Manager's `cln telemetry` heartbeat is a distinct, independent system). The `consent-level` WIT variant gained the fifth case `identified` that §6.6 already defined, with the total order documented explicitly. Report schema repackaged from `clean:reporting@2.0.0` to `clean:host/reporting@0.1.0` per [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md) and the 08 §8.0 baseline. The `error-code` example corrected from kebab (`FRAME-DATA-E028`) to the real `PREFIX###` format of 09 §1 (`LIB010` + sub-label for libraries). §§6.5, 6.7, 6.9–6.10 reduced to their observable contracts — the scoring formula, concrete rate limits, and engine capture mechanism extracted verbatim to `work/2026-08-01-clean-errors-extraction-06.md`, destined for the `clean-errors` component doc ([ADR-0008](../01%20governance/decisions/0008-error-reporting-backend.md)); the §6.9 trap-must-be-reported rule kept as a MUST contract (checkability: Fase 4). §6.8.1 MCP tools now cite the framework's single MCP server ([ADR-0001](../01%20governance/decisions/0001-single-mcp-server.md)) and 10-mcp as catalog home; the V1 alias `comita` replaced by `cln ship`; world-name comment normalized per 15 §0.3.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler, framework, host and library maintainers implementing the reporting pipeline; anyone wiring `report_error`
- **Rule prefix:** `REP-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Diagnostic Format](./13-diagnostic-format.md), [Error Codes](./09-error-codes.md), [Semantic Rules](./10-semantic-rules.md), [ADR-0008 — Error Reporting Backend](../01%20governance/decisions/0008-error-reporting-backend.md)
- **Satisfies:** LANG-03, LANG-16, SEC-07, SEC-09
