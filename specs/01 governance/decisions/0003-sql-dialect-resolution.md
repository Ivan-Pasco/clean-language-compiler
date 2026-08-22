# ADR-0003 — Where the SQL dialect is resolved

**Status:** Superseded by [ADR-0024](./0024-sql-dialect-resolution-under-wit-bridges.md) — the C-ABI driver model this ADR analyzed has been retired. Database backends are now WIT bridges (see [bridges/data-bridge](../../02%20components/bridges/data-bridge/01-specification.md)). Under the new mechanism the tradeoffs favor Option C (per-driver dialect profile, single renderer in `data`) over the Option A adopted here. This document is retained for historical rationale.

A `data:` companion declares persistence in motor-agnostic terms, but somewhere between that declaration and the database a concrete SQL statement in a concrete dialect has to be produced. This ADR decided — under the retired C-ABI driver model — that the driver receives a serialized typed query tree and each driver renders its own dialect, so adding a new backend meant writing a driver rather than modifying the `data` library.

---

## Context

Raised by the `02 components/` compliance audit ([report](../../reports/2026-07-31-components-compliance-audit.md)). This is a gap, not a contradiction between documents: no chapter answers the question at all, which [SDD-09](../03-spec-driven-design.md) classifies as a defect.

A `data:` companion declares persistence in motor-agnostic terms — a table, columns, indexes, and queries written in the `queries:` DSL ([data §4](../../02%20components/framework/libraries/04-data.md)). Somewhere between that declaration and the database, a concrete SQL statement in a concrete dialect must be produced.

The C-ABI vtable in [04 — Database Driver ABI §3](../../02%20components/framework/04-database-libraries.md) settles what the driver receives:

```rust
pub query: extern "C" fn(conn, sql: *const c_char, params_json: *const c_char, result_out) -> c_int
```

The driver is handed **SQL that is already written**, as text. So the dialect is resolved above the driver — but no document says by whom, or how that layer knows which motor is in play. The only candidate is the `data` library, and its specification never mentions dialects.

This collides with the stated goal of the driver contract ([§1](../../02%20components/framework/04-database-libraries.md)):

> *"The driver contract exists so that new database backends can be added without changing either the `data` library or the compiler."*

With the current vtable that promise does not hold: if `data` writes the SQL, adding Oracle means changing `data`.

Three further facts constrain any answer:

- **The motor is not fixed at compile time.** `[data.dev]` may select SQLite while `[data]` selects Postgres, resolved at runtime per `CLEAN_ENV` ([data §12](../../02%20components/framework/libraries/04-data.md)). The *set* of possible motors is closed and declared, but the choice is not.
- **Conformance to ISO/IEC 9075 does not give portability.** The standard form of paging is `FETCH FIRST n ROWS ONLY`; MySQL and older SQLite accept only the non-standard `LIMIT n`. The same holds for generated keys and upsert. The standard fixes semantics well; it is not a portable wire format.
- **A raw-SQL escape hatch exists.** `db.query:` / `db.queryAs(T):` blocks carry hand-written SQL text ([data §8](../../02%20components/framework/libraries/04-data.md)), and the dev-mode diagnostics wire mandates `SHOW CREATE TABLE` — a MySQL-specific statement — for its `db-schema` field ([12 §11](../../03%20platform/12-server-extensions.md)). Both touchpoints were marked as open questions pointing at this ADR.

## Decision

Option A. The driver vtable MUST NOT receive SQL as text for DSL-declared queries; it receives the **typed query tree** — the query IR produced by the `queries:` DSL ([data §4](../../02%20components/framework/libraries/04-data.md)) — serialized across the C-ABI, and **each driver emits the SQL of its own dialect**. The serialization of the query IR is part of the driver ABI: an addition to the vtable, versioned under the `DRV-` contract of [04 — Database Driver ABI](../../02%20components/framework/04-database-libraries.md).

Whichever option is chosen must also state where **DDL** is resolved. Schema statements (`CREATE TABLE`, `ALTER TABLE`) diverge between motors more than queries do, and migrations are generated once and reviewed by hand — so they may warrant a different answer from queries.

## Options considered

**A — The driver emits the dialect.** The vtable changes to accept a serialized typed query tree — the query IR (intermediate representation) produced by the `queries:` DSL vocabulary of [data §4.1–4.8](../../02%20components/framework/libraries/04-data.md) — instead of a SQL string; each driver renders its own dialect. Honours the §1 goal exactly: adding a motor means writing a driver, nothing else. Cost: the query-IR serialization becomes a new stable contract in the driver ABI, and every driver implements a dialect emitter — N drivers are N renderers that can diverge, and rendering moves to runtime.

**B — The `data` library emits the dialect.** Dialect knowledge lives inside `data`, either as a dialect module per motor or as one renderer parameterized by driver-published dialect profiles. In the per-motor form, every new engine forces a change to `data`, so the §1 goal would have to be rewritten to match reality. In the profile form, `data` never becomes dialect-free either: the profile contract grows with every construct the profile fields do not cover, and the dialect boundary stays smeared across two components.

**C — Status quo.** The vtable keeps receiving SQL as text with no specified owner of dialect resolution. This is the defect that motivated the ADR, not an answer: the §1 goal keeps overstating what the architecture delivers, and no chapter can state in checkable terms what a driver author must implement.

## Consequences

**Easier:**

- The §1 goal of the driver ABI — *new backends without changing either the `data` library or the compiler* — now genuinely holds: adding a motor means writing a driver, nothing else.
- The `data` library is dialect-free: it lowers the `queries:` DSL to the query IR and stops there. The runtime motor switch per `CLEAN_ENV` ([data §12](../../02%20components/framework/libraries/04-data.md)) becomes unproblematic, since the dialect is resolved by whichever driver is loaded.

**Harder:**

- The stable serialization of the query IR must be specified and versioned in the driver ABI — a new contract that did not exist before.
- Each driver implements a dialect emitter (joins, relations, paging, upsert, generated keys) in native code; N drivers are N renderers, so a driver conformance suite becomes necessary to keep them from diverging.

**Follow-up spec edits on acceptance ([DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent)):**

- [04 — Database Driver ABI §1](../../02%20components/framework/04-database-libraries.md): remove the open-question note under the goal statement. §3: extend the vtable with the query-IR entry point and its serialization, versioned per `DRV-`.
- [data §4](../../02%20components/framework/libraries/04-data.md): state that the `queries:` DSL lowers to the query IR consumed by drivers; `data` owns no dialect.
- [12 — Server Extensions](../../03%20platform/12-server-extensions.md), both ADR-0003 markers: the §11 `db-schema` field of the dev-mode diagnostics wire exposes the schema in a **structured, dialect-neutral form emitted by the driver**, not `SHOW CREATE TABLE`; the §8 raw-SQL marker is resolved against this decision in the same edit.

**Companion decisions (ratified 2026-08-01, same approval):**

- **The raw-SQL escape hatch** ([data §8](../../02%20components/framework/libraries/04-data.md)) crosses the ABI through a dedicated **`execute-raw(sql-text)`** vtable entry: the text passes through verbatim, and its dialect is the responsibility of the author who chose to write SQL by hand (documented in data §8). Only the DSL query path is IR-only.
- **DDL for migrations** stays hand-written dialectal `.sql`, executed through the same raw entry — the driver does not emit DDL. This ADR's IR path covers queries and the dev-mode schema exposure; migration authorship is unchanged.

---

## Metadata

- **Status:** Superseded by [ADR-0024](./0024-sql-dialect-resolution-under-wit-bridges.md) on 2026-08-05
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [02 components / framework 04 — Database Driver ABI §1, §3](../../02%20components/framework/04-database-libraries.md), [libraries / 04 — Data §4](../../02%20components/framework/libraries/04-data.md), [03 platform / 12 — Server Extensions §8 raw-SQL marker and §11 `db-schema`](../../03%20platform/12-server-extensions.md)
