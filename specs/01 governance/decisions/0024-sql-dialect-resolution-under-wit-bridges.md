# ADR-0024 — SQL dialect resolution under the WIT-bridge data model

The question of *where* the SQL dialect is chosen was answered under the retired C-ABI driver model by [ADR-0003](./0003-sql-dialect-resolution.md), which put a renderer inside each driver. Now that database backends are Wasm bridge components rather than C-ABI vtables, the tradeoffs shift: this ADR re-decides in favour of a single renderer in the `data` library parameterized by a small typed dialect profile each bridge publishes, so N bridges do not become N renderers and the hot path avoids per-request SQL rendering.

---

## Context

Given a motor-agnostic `data:` declaration and a `queries:` DSL, *where* in the stack is the concrete SQL dialect chosen? Database backends are [WIT bridge components](../../02%20components/bridges/data-bridge/01-specification.md) targeting `clean:data/store`, `clean:data/txn`, and `clean:data/types`. Three options define the tradeoff surface:

- **Option A (bridge renders).** The `store.query` / `store.execute` signatures accept a serialized query tree instead of `sql: string`, and each bridge emits its dialect from the tree. Under the sidecar pattern, dialect rendering can happen inside the sidecar's native code where mature libraries exist. Expensive along one axis: every bridge implementer must maintain a renderer, and cross-bridge divergence in query semantics becomes a support surface.
- **Option B (`data` library renders alone).** `data` carries a dialect module per motor. Adding a motor means modifying `data`; the goal of adding backends without changing the library cannot hold.
- **Option C (per-driver dialect profile).** Bridges expose their dialect profile through a small typed WIT function (`store.dialect-profile() -> profile`), and the `data` library has one renderer parameterized by that profile. The Wasm bridge boundary makes profile publication clean — it is a typed function call, not a text-format side-channel.

A key consideration is **portability of the guest across backends at runtime.** A guest built for `[data]` = Postgres and `[data.dev]` = SQLite (per [libraries/04-data.md §12](../../02%20components/framework/libraries/04-data.md)) needs SQL that works against both. Under Option A, the guest emits dialect-neutral query trees and each bridge renders appropriately — dialect-agnostic at the compile stage but with runtime rendering on the hot path. Under Option C, the renderer is invoked at compile time per declared environment (once for Postgres, once for SQLite), producing pre-rendered SQL the bridge just executes — dialect-specific at compile stage, no runtime rendering cost.

## Decision

Adopt **Option C — per-driver dialect profile, single renderer in the `data` library.**

The bridge publishes a small, typed dialect profile via a new WIT function on `store`. The `data` library owns one query renderer, parameterized by that profile, and produces pre-rendered SQL for each declared environment at compile time. The bridge receives SQL strings via `store.query` / `store.execute` and executes them.

### Rationale (why C, not A)

Three factors are decisive:

1. **Renderer maintenance cost.** Option A requires every bridge implementer to write and maintain a query renderer. There are already ten bridges in the tree; if half of them are data bridges (Postgres, MySQL, SQLite, D1, Turso, PlanetScale, DynamoDB, ...), that's ten renderers to keep in sync. One renderer with N profiles is cheaper to keep correct than N renderers.

2. **Runtime cost on the hot path.** Every guest query executed under Option A rewrites a query tree into SQL at request time. Every guest query under Option C uses SQL rendered once at build time. For a hot HTTP endpoint doing a query per request, C avoids per-request rendering entirely.

3. **Divergence risk.** Two independently-maintained renderers for MySQL will disagree on some edge case. Users hit it, file bugs, expect us to reconcile. Under C the renderer is one code path; the profile might disagree between drivers but the renderer's decisions are the same.

Option A retains one advantage: the bridge can use backend-specific optimizations (server-side prepared statement caching, backend-specific hints) the renderer doesn't know about. Under C, this is recovered by the bridge holding a small statement cache keyed by the SQL string the renderer produced. Not free, but not blocking.

### Where DDL is resolved

DDL (`CREATE TABLE`, `ALTER TABLE`, migrations) is also resolved via the same profile-parameterized renderer. Schema statements diverge more between backends than queries do, and migrations are generated once and reviewed by hand, so the renderer produces per-environment DDL text at build time and the [`clean:data/migrations`](../../02%20components/bridges/data-bridge/01-specification.md) apply path executes it. See the migrations §3.4 of the data-bridge spec.

### The profile

The profile is a small closed struct the bridge returns from `store.dialect-profile()`. It covers the divergence surface every dialect renderer needs:

```wit
// New in clean:data@1.1.0
record dialect-profile {
    // Identifier and literal quoting
    identifier-quote: string,          // "\"" for postgres/sqlite, "`" for mysql
    literal-quote:    string,          // "'"

    // Placeholder syntax for prepared statements
    placeholder-style: placeholder-style,

    // Paging: how "give me N rows starting at offset K" is written
    paging: paging-style,

    // Upsert / on-conflict form
    upsert: upsert-style,

    // Generated-key retrieval on INSERT
    returning-generated-key: returning-style,

    // Auto-increment column syntax for CREATE TABLE
    auto-increment-column: string,     // "SERIAL", "AUTOINCREMENT", "AUTO_INCREMENT"

    // Type map: Clean value variant → backend column type
    type-map: list<tuple<string, string>>,   // e.g. [("timestamp-secs", "TIMESTAMPTZ"), ("decimal", "NUMERIC")]

    // Booleans as native BOOL vs INTEGER
    native-boolean: bool,

    // Feature flags (what the backend supports)
    supports-transactions: bool,
    supports-savepoints:   bool,
    supports-jsonb:        bool,
    supports-arrays:       bool,
    supports-cte:          bool,

    // Escape hatch: additional bridge-specific hints the renderer may consult.
    // Renderer treats unknown keys as unsupported and falls back to the
    // profile's declared capabilities. Bridges MUST NOT rely on the renderer
    // acting on custom hints.
    hints: list<tuple<string, string>>,
}

variant placeholder-style {
    numbered,          // $1, $2, ...  (postgres)
    positional,        // ?, ?, ...    (sqlite, mysql)
    named(string),     // :name        (some backends; format string prefix)
}

variant paging-style {
    limit-offset,                                    // LIMIT N OFFSET K   (sqlite, mysql, postgres)
    offset-fetch,                                    // OFFSET K ROWS FETCH FIRST N ROWS ONLY  (standard SQL)
    row-number-window,                               // Subquery with ROW_NUMBER() (older SQL Server)
}

variant upsert-style {
    on-conflict-do-update,                           // Postgres, SQLite 3.24+
    on-duplicate-key-update,                         // MySQL
    merge,                                           // Standard SQL MERGE
    replace,                                         // SQLite REPLACE INTO (loses row versioning)
    not-supported,                                   // Fall back to SELECT + UPDATE/INSERT in a txn
}

variant returning-style {
    returning-clause,                                // Postgres, SQLite 3.35+
    last-insert-id,                                  // MySQL LAST_INSERT_ID(), SQLite last_insert_rowid()
    output-inserted,                                 // SQL Server OUTPUT INSERTED
    not-supported,                                   // Requires a follow-up SELECT
}
```

### The renderer

The `data` library grows a renderer module: `renderer(profile, query-tree) -> string`. It is:

- **Pure.** No I/O; the profile and the tree fully determine the output.
- **Deterministic.** Same input, same output byte-for-byte. This lets the build cache pre-rendered SQL keyed by tree + profile hash.
- **Tested per profile.** A golden-test suite renders every known query pattern against every profile in the reference set (Postgres, MySQL, SQLite) and diffs against committed expected output.

The renderer lives in the `data` library, not the compiler. This keeps compilation dialect-independent — the compiler produces a query tree; the `data` library specializes to concrete SQL at build time based on the environment's declared `[data]` block.

### Escape hatch

Some queries have no reasonable representation in the DSL (window functions with dialect-specific syntax, recursive CTEs, backend-specific full-text search). For these, the `queries:` DSL supports a `raw:` block that takes SQL text per environment:

```clean
queries:
    stats_by_month:
        raw:
            postgres: "SELECT date_trunc('month', created_at)..."
            mysql:    "SELECT DATE_FORMAT(created_at, '%Y-%m')..."
            sqlite:   "SELECT strftime('%Y-%m', created_at)..."
```

The renderer picks the right variant for the declared environment. Raw blocks are the acknowledged escape valve — they violate motor-portability but they exist for the cases where portability cannot be preserved. `cln check` warns when a query has a `raw:` block but not for every declared environment.

## Consequences

**Positive:**

- Adding a new backend means writing a bridge component that publishes a profile. It does not require modifying `data`. The design goal is stated directly: *new database backends can be added by shipping a bridge that publishes a dialect profile, without changing either the `data` library or the compiler.*
- Runtime cost of dialect selection is zero on the hot path.
- One renderer means one bug surface. When two backends produce subtly-different results, the disagreement is in the profile, not in independently-maintained render code.
- The escape hatch (`raw:`) makes intentionally non-portable queries explicit and greppable.

**Negative:**

- The renderer is a new module in the `data` library that must be written and tested. First-cut effort: substantial (~a couple of weeks including golden tests for three backends).
- Bridges must publish a profile even for backends where the renderer's decisions could be simple. This is a small tax for a stable interface.
- The profile itself is a new versioned surface. Adding a field is a minor bump of `clean:data`; changing a field's shape is major. Care needed in evolution.

**Neutral:**

- Migrations produce per-environment DDL at build time, same mechanism. See [data-bridge §3.4](../../02%20components/bridges/data-bridge/01-specification.md) for the migration interface.

## Implementation notes

- `store.dialect-profile()` is part of `clean:data/store` from `clean:data@1.1.0`. Bridges published against the earlier `1.0.0` interface (which has no dialect-profile call) are treated with conservative defaults: `placeholder-style = positional`, `paging = limit-offset`, `upsert = not-supported`, `returning-generated-key = last-insert-id`, `native-boolean = false`, all optional-feature flags = false. Upgrading to 1.1.0 unlocks the full renderer surface.
- The `data` library needs configuration to select the profile for the reference SQLite dev backend when `[data.dev]` is used. Recommend a `[data.dev] dialect = "sqlite"` explicit setting so the renderer picks the right profile at compile time for the dev environment.
- `cln check` includes a "dialect coverage" check: for every query in `queries:`, verify the renderer produces a valid output against every profile in `[data]` + `[data.dev]`. Failures are compile errors with the query name, the profile, and the renderer's diagnostic.
- The `raw:` escape hatch in the `queries:` DSL is a syntax element in the `data` library specified in [libraries/04-data.md §4](../../02%20components/framework/libraries/04-data.md).

---

## Metadata

- **Status:** Accepted (2026-08-05)
- **Date:** 2026-08-05
- **Supersedes:** [ADR-0003 — Where the SQL dialect is resolved](./0003-sql-dialect-resolution.md) (reopens and re-decides under the WIT-bridge model; ADR-0003 chose Option A when drivers were C-ABI vtables, this ADR chooses Option C now that bridges are Wasm components)
- **Spec impact:** [02 components / bridges / data-bridge](../../02%20components/bridges/data-bridge/01-specification.md), [02 components / framework / libraries / 04 — Data](../../02%20components/framework/libraries/04-data.md)
