# ADR-0005 — Server world interface additions and naming alignment

The `server` world's WIT vocabulary was described by four different documents with four different inventories — the central host↔guest contract had no single authoritative list. This ADR extends the canonical interface vocabulary in [15 §0.3](../../03%20platform/15-component-model-architecture.md#03-wit-package-and-world-naming) with four missing interfaces (`auth`, `admin`, `mcp`, `diagnostics`), aligns names across the tree, and reasserts that the vocabulary has one home while each interface's content lives in its owning chapter.

---

## Context

[15 §0.3](../../03%20platform/15-component-model-architecture.md#03-wit-package-and-world-naming) is the home of the WIT interface vocabulary, and it is deliberately a closed list: an interface that is not named there does not exist. The `server` world, however, was described by four documents with four different inventories — 15 §0.3's canonical list, [02 §2.2.2](../../03%20platform/02-host-bridge.md#222-host-specific-l3)'s seven entries, [12 §13](../../03%20platform/12-server-extensions.md#13-world-declaration)'s world declaration (with `sessions` plural and `roles`, `auth`, `handler`, `dev-mode` outside every list), and [clean-server §1.3.1](../../02%20components/hosts/clean-server/01-server.md) (Accepted) with twelve capability domains including `auth`, `admin`, `mcp`, and `diagnostics`. The WIT of the server world is the central host↔guest contract ([C-15](../05-concerns.md)); four versions of it means no contract.

Two adjacent membership defects surfaced with it: `clean:bridge/files` is consumed in two Accepted places ([clean-server §1.4](../../02%20components/hosts/clean-server/01-server.md) and the host-contract-validation chapter) but appears in no catalog, and the error-report schema in [06 §6.3](../../03%20platform/06-error-reporting.md#63-report-schema) uses the package family `clean:reporting@2.0.0`, which falls outside the closed package scheme of 15 §0.3 (`wasi:*`, `clean:bridge`, `clean:host`, `clean:library/*`).

The components conflict log (decision C1) already fixed the process: interfaces that 15 §0.3 does not know are proposed as additions to 15 *by ADR*. This is that ADR for the server world.

## Options considered

**A — Keep each document's own inventory and reconcile ad hoc.** No single authority; every reader must diff four lists. Rejected — this is the defect, not an option.

**B — Make 12 §13's world declaration authoritative.** 12 owns the *content* of each server-only interface, but its declaration drifted from every other document, contains names with no consumer anywhere else (`roles`, `handler`), and restating the vocabulary there would put the same fact in two homes — 15 §0.3 is the declared home.

**C — Extend 15 §0.3's vocabulary by ADR, aligning names to the Accepted clean-server chapter.** 15 §0.3 remains the single home of the interface vocabulary; additions go through an ADR; 12 keeps ownership of each server interface's content; 02 §2.2.2 and 15 §5.2 cite and exemplify without restating. Chosen.

## Decision

**Option C.** The `clean:host` interface vocabulary in 15 §0.3 gains four interfaces, each backed by the Accepted [clean-server §1.3.1](../../02%20components/hosts/clean-server/01-server.md): **`auth`** (authenticated identity, role-based checks, guard registration), **`admin`** (rate-limit and CORS configuration, runtime toggles), **`mcp`** (MCP transport into the running application), and **`diagnostics`** (dev-mode snapshot capture, error-dashboard forwarding, structured logging).

Naming aligns as follows:

- **`sessions` → `session`** (rename; 15 §0.3 and clean-server already use the singular).
- **`dev-mode` → `diagnostics`** (rename; it is the same capability clean-server calls `diagnostics`).
- **`roles` merges into `auth`** and **`handler` merges into `routing`** (neither has a consumer outside 12 §13's world declaration). Within the merged `auth`, the former `roles.register` becomes **`register-roles`** — a bare `register` inside `auth` would be ambiguous (ratified 2026-08-01).

**`clean:bridge/files`** is added to the portable L2 catalog in [02 §2.2.1](../../03%20platform/02-host-bridge.md#221-portable-l2-in-every-world) — it has two Accepted consumers and no home.

The error-reporting package **`clean:reporting` is renamed to `clean:host/reporting`** (server-side, within the permitted package scheme); 06 §6.3 updates its WIT accordingly.

## Consequences

**Easier.** One authoritative inventory for the server world; `cln check` and host-contract validation have a single list to verify against; the clean-server chapter and the platform vocabulary stop contradicting each other.

**Harder.** Every occurrence of the retired names (`sessions`, `dev-mode`, `roles`, `handler` as interfaces, `clean:reporting`) is now a defect to sweep; 12 §11's `dev-mode` WIT interface must be re-titled `diagnostics` in the same change.

**Now required (DOC-07).** 15 §0.3's interface list gains `auth`, `admin`, `mcp`, `diagnostics`; 15 §5.2's example world and 12 §13's world declaration are rewritten to the aligned names (and 12 §13 becomes content-home, not a rival vocabulary); 02 §2.2.1 gains the `clean:bridge/files` row and §2.2.2 cites 15 §0.3 instead of restating; 06 §6.3 renames its package to `clean:host/reporting`.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [15 — Component Model Architecture §0.3, §5.2](../../03%20platform/15-component-model-architecture.md#03-wit-package-and-world-naming), [02 — Host Bridge §2.2.1–2.2.2](../../03%20platform/02-host-bridge.md#22-interface-catalog), [12 — Server Extensions §13](../../03%20platform/12-server-extensions.md#13-world-declaration), [06 — Error Reporting §6.3](../../03%20platform/06-error-reporting.md#63-report-schema)
