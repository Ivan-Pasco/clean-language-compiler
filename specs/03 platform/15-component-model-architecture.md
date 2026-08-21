# Platform 15. Component Model Architecture

Clean compiles to WebAssembly components, packaged and typed using the Component Model and WIT. This chapter is the home of the WIT vocabulary — the package families (`wasi:*`, `clean:bridge`, `clean:host`, `clean:library/*`), the world names (`server`, `browser`, `cli`), the interface naming convention, and the way versions are resolved at build time. It sits between [02 — Host Bridge](./02-host-bridge.md), which defines the bridge function surface, and [08 — Bridge Versioning](./08-bridge-versioning.md), which defines how those packages evolve — this chapter is where a compiler maintainer, a host implementor, or a library author learns how their pieces are named and how they compose into a component the host can instantiate.

---

## 0. Canonical Reference


This document is the home of the WIT vocabulary — package families, worlds, interface names, and import syntax — declared in §0.3. The version baseline it quotes in §0.4 has its own home in [Platform 08 §8.0](./08-bridge-versioning.md#80-v2-baseline-versions). Other documents cite these sections instead of restating them; conflicts between documents are resolved by the precedence rule in [DOC-11](../01%20governance/00-documentation-principles.md).

### 0.1 Toolchain Roles


The role split of the toolchain — Manager, Framework, Compiler, hosts — is owned by [Architecture Boundaries §2](../01%20governance/01-architecture-boundaries.md), and the `cln` command surface by [Manager MGR-01](../02%20components/manager/00-manager.md). This section does not restate those tables. What this document relies on:

- **`cln` is the only command a user ever types** ([MGR-01](../02%20components/manager/00-manager.md)). Component binaries (`clean-framework`, `clean-compiler`) are implementation details, never invoked directly. Clean Manager reads the project's pinned versions from `.cln/version` and `.cln/frame-version` and dispatches to the matching installed component binaries.
- **The compiler is a pure function of the compilation request document** ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)): every source file and every configuration value arrives inline in the request. The compiler has no filesystem access and no library awareness.
- **Block handlers are distributed as Clean source; the framework compiles them to WASM at install time and caches the artifacts; the compiler executes the cached handler WASM in its sandbox during block expansion, taking a typed BlockAST and returning a typed IR fragment** ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)).

### 0.2 CLI Vocabulary


Every user-facing command is `cln <verb>`. The command surface is flat — no nested subcommand trees except where a namespace (like `db`) genuinely groups a family of operations.

The command surface itself is owned by [Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface) (concern [C-13](../01%20governance/05-concerns.md): adding a user-facing verb requires a Manager change). This section does not restate the verb table; it fixes only the shape of the surface.

### 0.3 WIT Package and World Naming


### CMOD-01 — One WIT naming scheme, extended only by ADR


There is exactly one naming scheme. It MUST be used in every WIT import, every doc example, and every code path; the prohibited forms below MUST NOT appear anywhere in the tree. Extending the vocabulary — a new package family, world, or interface name — requires an ADR (the pattern set by [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)). Check: `grep` for the prohibited forms returns nothing; every interface name in use appears in the list below or in an Accepted ADR.

**Packages:**

- `wasi:*@x.y.z` — standard WASI 0.3 (Preview 3), used verbatim. `wasi:logging` is the sole exception, pinned at `@0.2.0` until an upstream 0.3 cut ships.
- `clean:bridge@x.y.z` — portable Clean interfaces (L2).
- `clean:host@x.y.z` — host-specific interfaces and worlds (L3).
- `clean:library/<name>@x.y.z` — library-declared interfaces (L4).

**Worlds** (declared inside `clean:host`):

- `world server` — HTTP server runtime.
- `world browser` — browser runtime.
- `world cli` — CLI runtime, named-subcommand mode.
- `world cli-default` — CLI runtime, default-handler mode ([CLIH-06](../02%20components/hosts/clean-cli/01-specification.md) selects between the two by inspecting the guest's exported world).
- `world worker` — background job runtime.
- `world edge` — edge/CDN runtime.
- `world embedded` — embedded runtime (reserved).

A world name is a **bare identifier inside `clean:host`**. It never appears as a package path: `clean:host/cli` and `clean:host/server` are prohibited forms (see below).

**Interfaces** (also inside `clean:host`, flat namespace):

`routing`, `request`, `response`, `request-context`, `sse`, `ws`, `websocket`, `session`, `session-envelope`, `realtime-sockets`, `fetch`, `log`, `env`, `commands`, `default`, `jobs`, `email`, `i18n`, `dom`, `nav`, `router`, `storage`, `toast`, `prompt`, `lifecycle`, `events`, `render`, `handler`, `dispatch`, `config`, plus `auth`, `admin`, `mcp`, `diagnostics` (added by [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)).

**Import syntax — always:**

```wit
import clean:host/routing@0.1.0;
```

Package/interface, one level, no three-level paths.

**What `clean:host` is not.** `clean:host` holds interfaces a *host* provides or a guest exports *to a host*. It does not absorb the bridge packages. The capability bridges under [02 components / bridges](../02%20components/bridges/) each own their package — `clean:data`, `clean:session`, `clean:jobs`, `clean:kv`, `clean:mail`, `clean:auth`, `clean:roles`, `clean:i18n`, `clean:realtime`, `clean:mcp` — declared in their own DOC-18 schema files and composed per [Platform 18 §1.1](./18-component-composition.md). A world in `clean:host` importing `clean:data/store` is correct and MUST NOT be rewritten to `clean:host/store`.

**Prohibited** (delete every occurrence):

- `clean:host/server/routing` — three-level paths do not exist in WIT.
- `clean:host/server` as a package — `server` is a world, not a package.
- `clean:host/cli`, `clean:host/browser`, `clean:host/worker`, `clean:host/edge` as packages — same reason; all are worlds.
- `clean:http` as a package — the HTTP surface (`routing`, `request`, `response`, `fetch`, …) lives inside `clean:host`.
- `clean:cli`, `clean:browser`, `clean:worker`, `clean:edge` as packages — one package per host is the same error as one package per host-only interface.
- Anything using a separate package per host-only interface.

Check: `grep -rn 'clean:http\|clean:cli@\|clean:browser@\|clean:worker@\|clean:edge@\|clean:host/cli\|clean:host/server@' ` across the tree returns nothing.

### 0.4 Version Baseline


### CMOD-02 — Every `clean:*` package sits at the 08 §8.0 baseline


The V2 baseline is **`@0.1.0`** for every `clean:*` package (`clean:bridge`, `clean:host`, `clean:library/*`). The canonical source is [Platform 08 §8.0 — V2 Baseline Versions](./08-bridge-versioning.md#80-v2-baseline-versions); version bumps follow the rules in [Platform 08 — Bridge Versioning](./08-bridge-versioning.md). A `clean:*` version outside the baseline MUST NOT appear in any spec example or shipped WIT without a corresponding 08 §8.0 amendment. Check: `grep 'clean:.*@'` across the tree yields only baseline versions or versions recorded in 08 §8.0.

References to `@0.2.0` in earlier drafts are premature and must be moved to a "next version" appendix.

### 0.5 On-Disk Paths


All Clean Manager state lives under `~/.cln/` on the user's machine and `.cln/` inside a project. The directory name mirrors the command.

The layout itself is owned by [Manager §00.2 — On-Disk Layout](../02%20components/manager/00-manager.md), which holds the complete tree for both the per-user and the per-project state. This section does not restate it.

What matters at this layer: every artifact the toolchain installs is under one of those two roots, and nothing else on the user's machine is modified (concern [C-14](../01%20governance/05-concerns.md)).

### 0.6 Project Folder Layout


The project folder layout — the canonical tree and its rules (entities vs. data companions, `app/ui/<platform>/…`, `public/` as a sibling of `app/`) — is owned by [Framework 01 §6 — Project Structure](../02%20components/framework/01-framework-specification.md#6-project-structure). Deviations from the canonical layout require explicit `[folders]` overrides in `clean.toml` ([LBS-01](../02%20components/framework/09-libraries-specification.md): the project manifest is the only source of implicit scope). This section does not restate the tree.

### 0.7 Default Folder Scope by Library


The default folders each library brings into scope, and the `app/services/` rule for external-service integrations, are owned by [Framework 01 §6 (FRM-01)](../02%20components/framework/01-framework-specification.md#frm-01--folder-scope-replaces-per-file-imports) together with [LBS-01](../02%20components/framework/09-libraries-specification.md). This section does not restate the table.

### 0.8 MCP Tool Naming


MCP tools use **bare names** — no `cln-` prefix. Rationale: they are already scoped by MCP server namespace, and the shorter names match the ~60 tools shipped in V1 (`mcp__clean-language__*`).

Full tool catalog and V1→V2 migration table live in [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md).

### 0.9 Diagnostic Code Format


Every diagnostic code matches the pattern `PREFIX###`. The registry — including the list of valid prefixes — is owned by [Platform 09 §1 — Error Code Registry](./09-error-codes.md); this section does not restate it. Library-emitted diagnostics travel as `LIB010` with a library-supplied sub-label field (also 09 §1).

---

## 1. Overview


The bridge contract between the Clean compiler and its hosts has a single source of truth: WIT interface files. Rich types (`string`, `list<T>`, `record { ... }`, `variant`, `result<T, E>`, `option<T>`) cross the bridge with generated marshalling. The compiler and every host generate their glue code from the same WIT files, so signature drift is a compile-time error rather than a runtime bug. The Clean runtime inherits the WASI ecosystem for I/O, tooling, and conformance; the reference toolchain choices are recorded in [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md) and the host reference stacks (e.g. [ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md)).

The three properties this architecture is designed to guarantee — **predictability, security, stability** — are addressed in §7, §8, and §9 respectively.

---

## 2. Terminology


| Term | Meaning |
|------|---------|
| **WIT** | WebAssembly Interface Type. Text-format interface definition language used by the Component Model. |
| **Component Model** | WebAssembly specification layer above core WASM that adds typed interfaces, resources, and cross-language composition. |
| **Interface** | A named group of typed functions and resources declared in a WIT package. |
| **World** | A WIT declaration listing the interfaces a component imports and exports. Every Clean program compiles to a component that fits a specific world. |
| **Package** | A namespaced collection of WIT interfaces and worlds, e.g. `clean:bridge@1.0.0`, `wasi:filesystem@0.3.0`. |
| **Host** | Any runtime that instantiates and executes a Clean component: `clean-server`, `clean-browser`, `clean-cli`, third-party embedders. |
| **Bridge** | The typed contract between compiler-emitted imports and host-provided exports, defined by WIT. |
| **Layer** | See [Platform 01 — Execution Layers](./01-execution-layers.md). |

---

## 3. Architectural Principles


These principles are load-bearing. Every design decision below traces to one of them.

### P1. One source of truth for every bridge signature.
Each bridge function is declared exactly once, in a WIT file. Compiler expected-imports, host registered-exports, library manifests, documentation examples, and MCP tool schemas are all generated from those WIT files. Hand-transcribed signatures are prohibited.

### P2. The compiler emits imports, never implementations.
Layer 0 (compiler) knows the shape of every bridge function but implements none of them. See [Platform 01 — Execution Layers](./01-execution-layers.md).

### P3. WIT-typed rich values cross the bridge.
`string`, `list<T>`, `record`, `variant`, `option`, `result`, and resources cross bridge calls with marshalling generated from the WIT declarations. Manual pointer + length pairs are prohibited.

### P4. WASI-standard interfaces are used where they fit.
Filesystem, stdio, clocks, random, environment, sockets, and outbound HTTP use `wasi:*` packages verbatim. Clean-specific portable interfaces are declared in `clean:*` packages alongside them; the L2 catalog is owned by [Platform 02 §2.2.1](./02-host-bridge.md#221-portable-l2-in-every-world): `clean:bridge/console`, `db`, `crypto`, `mem`, `math`, `string` (`clean:bridge/files` is pending [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)). Host-specific interfaces (`ui`, `canvas`, `auth`, `session`, …) are `clean:host/*` (L3), not bridge interfaces. Clean-specific behavior is never smuggled into a `wasi:*` package.

### P5. Every library declares its bridge surface in source.
Libraries declare `host function` signatures in `host_bridge.cln`; the framework synthesizes the corresponding WIT ([LBS §8.1](../02%20components/framework/09-libraries-specification.md); concern [C-07](../01%20governance/05-concerns.md): no hand-written WIT). Library-declared interfaces live in the `clean:library/<name>` package namespace; the compiler consumes library WIT from the compilation request during resolution.

### P6. Every host implements one world.
A host is defined by the WIT world it fulfills, and every one of those worlds lives inside the single `clean:host` package (§0.3). `clean-server` implements the `server` world. `clean-browser` implements the `browser` world. `clean-cli` implements the `cli` world (and `cli-default`). A Clean program targets a world; the compiler rejects programs whose imports are not satisfied by the target world.

### P7. Capabilities are explicit and least-privilege.
Component instantiation requires the host to hand each imported interface to the component. A component that does not import `wasi:filesystem` cannot open files, period — there is no ambient authority. This is a property of the Component Model, not a Clean layer; §8 explains why it matters for security.

### P8. Contract tests are mandatory, per host.
Every host proves it satisfies its world by running a shared conformance suite in CI. A host that does not pass conformance does not ship. See §9.

### P9. Backwards compatibility is a version property, not a hope.
WIT packages are versioned (`clean:bridge@1.0.0`). Removing or changing an interface member is a major version bump. Adding an interface member is a minor bump. Hosts declare which minor versions they support; the compiler emits against the intersection. See §9.

### P10. The toolchain is split into four cooperating components with narrow contracts.
The build pipeline is not a monolithic compiler. It is four components, each with a single responsibility, connected by versioned contracts. The compiler is a pure function `(compilation request document) → wasm` ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)), standalone-callable, with no filesystem awareness or library knowledge. §12-§15 define the split, the contracts, the build sequence, and the isolation guarantee.

### P11. The compiler owns the compile boundary. Hosts own the run boundary. Neither crosses.
The compiler has no runtime. Hosts have no compilation. A single `.wasm` component executes in every environment (server, browser, CLI) without re-compiling.

---

## 4. Layered Structure


The six-layer model (L0–L5; see [Platform 01 — Execution Layers](./01-execution-layers.md)) organizes what each layer contains under the Component Model.

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 5  Frameworks & Applications                              │
│          Clean source code (.cln), user programs                │
├─────────────────────────────────────────────────────────────────┤
│ Layer 4  Libraries                                              │
│          library.toml + Clean source (WIT synthesized, LBS §8.1)│
│          Contributes clean:library/<name> interfaces            │
├─────────────────────────────────────────────────────────────────┤
│ Layer 3  Host Extensions                                        │
│          Host-specific interfaces: clean:host/*                 │
│          (routing, request-context, session, dom, …)            │
├─────────────────────────────────────────────────────────────────┤
│ Layer 2  Portable Bridge                                        │
│          wasi:filesystem, wasi:cli, wasi:clocks, wasi:random,   │
│          wasi:http, wasi:sockets                                │
│          clean:bridge/console, db, crypto, mem, math, string    │
│          (catalog home: Platform 02 §2.2.1)                     │
├─────────────────────────────────────────────────────────────────┤
│ Layer 1  WASM Runtime                                           │
│          Pure computation. Core WASM instructions.              │
├─────────────────────────────────────────────────────────────────┤
│ Layer 0  Compiler                                               │
│          Emits components conforming to a target world.         │
│          Generates import tables from WIT. Implements nothing.  │
└─────────────────────────────────────────────────────────────────┘
```

**Layer contract:** each layer may import interfaces from the layers immediately below it via its component world. Skipping layers upward (e.g. Layer 5 importing `wasi:filesystem` directly) is legal but discouraged — the framework should mediate.

---

## 5. Package and World Layout


All WIT files live under the platform WIT tree (see [Platform 08 — Bridge Versioning](./08-bridge-versioning.md) for the canonical layout). Package structure:

```
platform/wit/
├── clean/
│   ├── bridge/              # L2 catalog home: Platform 02 §2.2.1
│   │   ├── console.wit      # clean:bridge/console@0.1.0
│   │   ├── db.wit           # clean:bridge/db@0.1.0
│   │   ├── crypto.wit       # clean:bridge/crypto@0.1.0
│   │   ├── mem.wit          # clean:bridge/mem@0.1.0
│   │   ├── math.wit         # clean:bridge/math@0.1.0
│   │   └── string.wit       # clean:bridge/string@0.1.0
│   ├── host/                # worlds + host interfaces (§0.3)
│   │   ├── server.wit       # world server (in clean:host@0.1.0)
│   │   ├── browser.wit      # world browser (in clean:host@0.1.0)
│   │   └── cli.wit          # world cli (in clean:host@0.1.0)
│   └── library/
│       └── <name>.wit       # per-library, synthesized by the framework (LBS §8.1)
└── deps/                    # vendored wasi:* packages, pinned by version
    ├── wasi-filesystem-0.2.0/
    ├── wasi-cli-0.2.0/
    └── ...
```

### 5.1 Example: `clean:bridge/db`

```wit
package clean:bridge@1.0.0;

interface db {
    variant db-error {
        connection-failed(string),
        query-failed(string),
        constraint-violated(string),
    }

    record row {
        columns: list<tuple<string, value>>,
    }

    variant value {
        null,
        boolean(bool),
        integer(s64),
        number(f64),
        text(string),
        blob(list<u8>),
    }

    query: func(sql: string, params: list<value>) -> result<list<row>, db-error>;
    execute: func(sql: string, params: list<value>) -> result<u64, db-error>;
    transaction: func() -> result<transaction, db-error>;

    resource transaction {
        query: func(sql: string, params: list<value>) -> result<list<row>, db-error>;
        execute: func(sql: string, params: list<value>) -> result<u64, db-error>;
        commit: func() -> result<_, db-error>;
        rollback: func() -> result<_, db-error>;
    }
}
```

Parameters are typed, errors are structured, transactions are resources with a lifetime the host tracks, and there is no manual string marshalling.

### 5.2 Example: the `server` world (illustrative)

The full `server` world declaration — its interface inventory and the content of each server-only interface — is owned by [Platform 12 §13 — World Declaration](./12-server-extensions.md#13-world-declaration), with the interface vocabulary fixed by §0.3 (as amended by [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)). Illustrative fragment only:

```wit
package clean:host@0.1.0;

world server {
    // wasi:* imports per the L2 catalog (Platform 02 §2.2.1)
    import wasi:http/handler@0.3.0;

    // portable bridge imports (L2)
    import clean:bridge/db@1.0.0;

    // server-only host interfaces (§0.3 + ADR-0005)
    import clean:host/routing@0.1.0;
    import clean:host/session@0.1.0;
    import clean:host/auth@0.1.0;

    export wasi:http/service@0.3.0;
}
```

A Clean program compiled for the server world may import any interface the world declares. The server host guarantees it provides all imports and consumes the exported `wasi:http/service`. A program that imports an interface not in the world — for example `clean:host/dom` — fails to compile against it (`COM012`).

### 5.3 Version selection


WIT packages are semver-versioned. A world pins a specific version per import. When a host advertises support for a range (`clean:bridge/db@^0.1.0`), the compiler emits against the newest compatible version the host supports. Version negotiation happens at compile time, not runtime.

---

## 6. Compile and Instantiate Flow


### 6.1 Compile

```
compilation request document (14 §14.1.1)
  { sources inline, resolved config, target world WIT }
     │
     ▼
┌────────────┐
│  Compiler  │
│  (Layer 0) │
└─────┬──────┘
      │
      │ 1. Parse program
      │ 2. Resolve imports against target world
      │ 3. Reject any use of interfaces not in world (COM012)
      │ 4. Generate core WASM
      │ 5. Wrap as Component Model module
      ▼
program.wasm  (component)
```

The compiler validates the program's imports against the target world WIT delivered in the compilation request document ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)); it never fetches a host's WIT itself (scope split: [16 §16.10](./16-host-contract-validation.md)).

### 6.2 Instantiate

```
program.wasm  (component)
     │
     ▼
┌────────────┐   provides   ┌─────────────────────────┐
│    Host    │◄─────────────│  Host-side WIT bindings │
│            │              │  (generated per host)   │
└─────┬──────┘              └─────────────────────────┘
      │
      │ 1. Load component
      │ 2. Verify component's imports are subset of host's world
      │ 3. Bind each import to host implementation
      │ 4. Instantiate with capability grants
      │ 5. Invoke exported entry point
      ▼
running program
```

Step 2 is a Component Model runtime check — mismatches fail loudly, before any code runs. Step 4 is the capability grant: `wasi:filesystem` is passed a preopened directory handle, `clean:bridge/db` is passed a connection pool, etc. What the host doesn't pass, the program can't use.

---

## 7. Predictability


Predictability means: given the same input, the same program produces the same output on every host that fulfills the same world.

### 7.1 Guarantees

- **Signature determinism.** A bridge function has exactly one signature, defined in WIT, generated identically for every host.
- **Type determinism.** WIT types have canonical serialization rules. A `string` is UTF-8. A `list<u8>` is a contiguous byte sequence. A `record` is a struct with declared field order. Hosts do not choose representations.
- **Error determinism.** Errors are `variant` types with declared cases. Hosts return one of the declared cases; a wasm-level trap raised inside a bridge component MUST be lifted into the `bridge-fault(fault-info)` case of the enclosing error variant per [Platform 02 §2.3.5 / BRG-06](./02-host-bridge.md#brg-06--a-bridge-components-trap-must-be-catchable-by-the-caller-as-a-declared-error-variant). Undeclared values MUST NOT reach the guest. Guest-side traps remain terminal for the current invocation and are handled per the host's instance-discard rules.
- **Version determinism.** The compiled component names the exact WIT package versions it was built against. A host running a different minor version negotiates or refuses at load time.

### 7.2 Non-guarantees

The following are explicitly not guaranteed by this architecture:

- **Timing determinism.** `db.query` may take milliseconds on one host and seconds on another. Timing is a host concern.
- **Ordering across interfaces.** Two independent async calls may complete in any order. Programs that require ordering must sequence explicitly.
- **Floating-point bit-identity.** WASM floats are IEEE 754. Bit-identical results across hosts are guaranteed for the same instruction sequence, but hosts may use different libm implementations for transcendentals invoked via bridge calls. Programs requiring bit-identity should use Layer 1 math intrinsics.

---

## 8. Security


Security under this architecture rests on the Component Model's capability discipline. Clean adds no ambient authority on top.

### 8.1 Capability model

- **No ambient authority.** A component starts with no filesystem access, no network access, no environment access, no clock access. To use any of these, it must import the corresponding interface *and* the host must supply it at instantiation.
- **Granularity is per-handle, not per-interface.** `wasi:filesystem` is granted by handing the component a `descriptor` for a specific preopened directory. The component cannot escape that directory. Multiple descriptors grant multiple roots.
- **Interfaces are unforgeable.** A component cannot construct a `wasi:filesystem/descriptor` — it can only receive one from the host. Resources have host-managed identity.
- **Library bridges are capabilities too.** A component that imports `clean:bridge/db` receives a database handle from the host. The host chooses which database, which schema, which permissions. The component has no way to reach a different database.

### 8.2 Threat model

**In scope (must be prevented):**
- A Clean program reading files the host did not grant.
- A Clean program making outbound network calls the host did not grant.
- A Clean program observing wall-clock time when only monotonic time was granted.
- A library escalating its declared bridge surface to call undeclared host functions.
- A bridge function receiving a value that does not match its declared type (type confusion).

**Out of scope (host responsibility):**
- Denial of service via infinite loops (host must set fuel/epoch limits).
- Denial of service via memory exhaustion (host must set memory limits).
- Side-channel attacks (Spectre/Meltdown class) between components sharing a runtime (host must run untrusted components in separate processes or use hardware isolation).
- SQL injection inside `db.query` arguments (application responsibility; the bridge passes strings faithfully).

### 8.3 Library trust boundary

Libraries are less trusted than the host, more trusted than user programs. Under this architecture:

- A library's WIT declares exactly which interfaces it imports from the host and which it exports to user programs.
- The host verifies the library's imports are a subset of what the library's manifest claims.
- User programs receive only the library's exports, not the library's imports. A library that imports `wasi:filesystem` does not grant filesystem access to programs that use it.

### 8.4 Attestation and pinning

Compiled components carry the WIT package versions and hashes they were built against. Hosts may pin acceptable version ranges and refuse components outside them. Security fixes to a bridge interface propagate this way: bump the WIT version, hosts refuse the old version, programs recompile.

---

## 9. Stability


Stability means: a program that compiled and ran against a given host-world version continues to compile and run against future minor versions of that host-world.

### 9.1 Versioning discipline

- **Package versions are semver.** `clean:bridge/db@0.1.0` → `0.1.1` for additive changes, `0.2.0` for breaking changes.
- **Additive is defined.** Adding a new function, adding a new variant case to a *host-produced* type, adding an optional field to a *host-produced* record — additive.
- **Breaking is defined.** Removing anything. Renaming anything. Changing a parameter type. Adding a variant case to a *component-produced* type (the host now sees a case it didn't handle). Adding a required field to a *component-produced* record.
- **Pre-1.0 packages have relaxed guarantees.** During `0.x`, minor bumps may break. Programs pin to exact versions. This is the WIT convention.

### 9.2 Deprecation of interface members

An interface member (function, type, field) may be marked `@deprecated` in WIT with a replacement pointer. While deprecated, both the deprecated member and its replacement exist in the same package version. The compiler emits a warning when a program uses a deprecated member. Removal happens in the next major version bump.

### 9.3 Host support matrix

Each host publishes the list of WIT packages and version ranges it supports. This matrix is machine-readable and lives alongside the platform WIT tree — see [Platform 08 — Bridge Versioning](./08-bridge-versioning.md). The compiler consults it when the user names a host but does not pin package versions.

### 9.4 Test corpus for stability

A stability corpus — a set of `.cln` programs that must continue to compile and run identically across releases — lives at `tests/cln/stability/`. Regression against the corpus is a blocking CI failure. The corpus grows monotonically; entries are only removed when the language feature they exercise is formally removed via §9.2.

---

## 10. Testing and Conformance


### 10.1 Contract tests, per host

### CMOD-03 — Conformance is the shipping gate for hosts


Every host MUST ship a conformance test binary that:

1. Loads a canonical set of Clean-compiled components (`tests/cln/conformance/`).
2. Runs each and diffs stdout / structured output against expected.
3. Verifies that every WIT import in the host's advertised world is actually provided.
4. Verifies that no import outside the advertised world is silently accepted.

CI runs all host conformance suites on every release-candidate tag. A host that fails conformance MUST NOT ship. Check: every released host version has a green conformance run recorded against its release-candidate tag.

### 10.2 Property tests on bridge boundaries

Cross-bridge memory correctness is verified by property tests that:

- Generate random `string`, `list<T>`, and `record` values.
- Round-trip them through every bridge function that accepts them.
- Assert value equality after round-trip.
- Assert heap invariants (no leaked handles, no double-free, no use-after-free) after each round-trip.

Property tests run in CI and as an always-on mode in `wasmtime_runner`.

### 10.3 Cross-host equivalence

### CMOD-04 — The equivalence set is byte-identical across hosts


A subset of the conformance corpus — the *equivalence set* — MUST produce byte-identical output on every host. This catches host-specific bugs where an interface is implemented differently despite claiming the same WIT signature. The equivalence set explicitly excludes tests that depend on timing, file paths, or environment. Check: diffing the equivalence-set output of any two conforming hosts yields no differences.

### 10.4 Version compatibility tests

For each supported (host version, WIT version) pair, a matrix test verifies:

- A component compiled against the WIT version loads.
- Every function callable in the WIT version is callable.
- No deprecation warning is emitted for non-deprecated members.
- Every deprecated member emits its declared warning.

---

## 11. Design Rules and Deferred Refinements


The following rules resolve design questions that surfaced during §1–§10. Each is either a firm V2 decision or a deferred refinement noted for a later revision.

1. **Async model.** V2 targets WASI 0.3 (Preview 3, ratified 2026-06-11). Native async in WIT — `async func`, `future<T>`, and `stream<T>` as canonical-ABI types — is the baseline; the bridge is versioned `clean:bridge@1.0.0` from V2 onward ([ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md), [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md), [ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md)). Clean's `start`, `later`, and `background` keywords lower directly to the WIT-native async surface; the compiler emits no `poll`-loop shim, and the runtime carries no Preview 2 compatibility path. Guests written for Preview 2 are out of scope for V2 — the two ABIs are not maintained in parallel.
2. **Component composition for libraries.** The compiler composes the user program and its libraries into a single component before shipping. Composition-at-compile keeps hosts simple. Library hot-swap is not a V2 capability.
3. **Resource lifetime rules.** WIT resources have host-managed lifetimes. Clean maps its ownership model onto resource borrows: a function that receives a resource (e.g. `db.transaction`) borrows it by default. Explicit ownership transfer requires a `move` annotation, which is reserved for a future revision — V2 uses borrow-only semantics.
4. **Diagnostic surface for WIT errors.** The error taxonomy in [Platform 09 — Error Codes](./09-error-codes.md) covers WIT mismatches with `COM014` (WorldMismatch) and `COM015` (VersionMismatch) — raised by the Moment 1/2 checks of [16 — Host Contract Validation](./16-host-contract-validation.md) — and `COM016` (DeprecatedMemberUse, a warning). When a Clean program imports an interface not in its target world, the compiler surfaces the mismatch (`COM012`) as `"your program uses <interface> but you compiled for <world>, which does not provide it."`
5. **Host-support-matrix ownership.** The host-support matrix defined in [Platform 08 — Bridge Versioning](./08-bridge-versioning.md) is owned by the framework team. It is updated on every host release and on every WIT-package version bump; no scheduled sync is required.
6. **Block-handler execution.** Library block handlers are distributed as Clean source; the framework compiles them to WASM at install time and caches the artifacts; the compiler executes the cached handler WASM in its sandboxed runtime during block expansion, with typed BlockAST in and typed IR out ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)). The framework embeds no WASM runtime of its own.
7. **WIT ownership.** The compiler owns and ships the base language WIT (the core `clean:bridge/*` interfaces every Clean program can rely on). Each host — server, browser, cli, embedded — owns and ships its own WIT for the extra interfaces it provides. At build time the compiler receives the target world WIT inside the compilation request document ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)); it never fetches a host's WIT (scope split: [16 §16.10](./16-host-contract-validation.md)). No neutral shared folder; the compiler owns the language contract, each host owns its host contract, and the build target composes them.
8. **LSP composition.** The reference language server is a fifth component that composes the compiler (typechecking) and the framework (project awareness, block-expanded source) in-process. See [Platform 04 — IDE / Language Server](./04-ide-lsp-architecture.md).
9. **Component binary consolidation.** From the user's perspective, `cln build` is one command. Under the hood, Clean Manager dispatches to separate `clean-framework` and `clean-compiler` binaries. Whether these ever consolidate into a single binary is an implementation detail that will not affect the `cln`-based user surface.

---

## 12. Toolchain Architecture


The build pipeline is split into four cooperating components with narrow, testable contracts. This split is the concrete realization of Principle P10.

### 12.1 Motivation

A monolithic compiler that must know about library resolution, block expansion, folder conventions, and project layout is hard to maintain, hard to swap, and offers no clean seam for alternate frontends, alternate frameworks, or standalone compiler use. Splitting the toolchain into four components with narrow contracts preserves every guarantee in §1-§10 while making the compiler small enough for a solo maintainer to hold in their head.

### 12.2 The Four Components

| Component | Owns | Does NOT know about |
|-----------|------|---------------------|
| **Clean Manager** (`cln`) | User-facing CLI, argv dispatch, fetching library sources from a registry, versioning, lockfile, on-disk placement under `~/.cln/` | What Clean code means |
| **Library** | Block handlers (Clean source), `host function` declarations, suggested folder conventions, MCP documentation, default dependency set ([LBS](../02%20components/framework/09-libraries-specification.md)) | Individual projects |
| **Clean Framework** (build orchestrator, `clean-framework` binary) | Reading `clean.toml`, validating project structure, resolving libraries, compiling and caching block handlers ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)), assembling the compilation request document ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)), wiring frontend/backend outputs | How to parse or typecheck Clean |
| **Compiler** (`clean-compiler`, source → wasm) | Parsing, typechecking, block expansion (sandboxed, [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)), IR, WASM component emission | Filesystem layout, libraries, projects, package management |

Below these four sit the **hosts** (`clean-server`, `clean-browser`, `clean-cli`) which run the resulting `.wasm`. Hosts are outside the toolchain proper and are covered in the `hosts/` subfolder and [Platform 01 — Execution Layers](./01-execution-layers.md).

### 12.3 Component Definitions

The definitions of Library, Framework, and Compiler are owned by the [glossary](../01%20governance/06-glossary.md); their responsibility boundaries by [Architecture Boundaries §2](../01%20governance/01-architecture-boundaries.md) and [Manager MGR-01](../02%20components/manager/00-manager.md). This document adds no redefinitions; where the table in §12.2 abbreviates, those homes win.

### 12.4 Full Architecture (Visual)

```
┌─────────────────────────────────────────────────────────────────┐
│                         DEVELOPER                                │
│                    writes .cln files                             │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     PROJECT ON DISK                              │
│                                                                  │
│   clean.toml         ← declares libraries                        │
│   app/                                                           │
│     data/models/     ← frame.data library conventions            │
│     endpoints/       ← frame.server library conventions          │
│     ui/pages/        ← frame.ui library conventions              │
│   libs/                                                          │
│     http.cln         ← a library (just .cln)                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Clean Manager (`cln`)                         │
│                                                                  │
│   • fetches library sources from registry                        │
│   • writes lockfile                                              │
│   • places .cln files on disk                                    │
│                                                                  │
│   knows: versions, URLs, checksums                               │
│   doesn't know: what Clean code means                            │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                  FRAMEWORK  (build orchestrator)                 │
│                                                                  │
│   1. reads clean.toml                                            │
│   2. validates folder structure per active libraries             │
│   3. resolves imports → file paths                               │
│   4. compiles & caches block-handler WASM (ADR-0004)             │
│   5. assembles the COMPILATION REQUEST DOCUMENT                  │
│        (sources inline, config resolved, target world WIT)       │
│   6. hands request document to compiler                          │
│   7. collects .wasm outputs, wires frontend/backend              │
│                                                                  │
│   knows: project layout, libraries, MCP docs, build order        │
│   doesn't know: how to parse or typecheck Clean                  │
└─────────┬───────────────────────────────────────┬───────────────┘
          │                                       │
          │ uses                                  │ calls
          ▼                                       ▼
┌──────────────────────────┐        ┌────────────────────────────┐
│      LIBRARIES            │        │       COMPILER              │
│  (Clean source packages)  │        │  (pure cln → wasm)          │
│                           │        │                             │
│  frame.server:            │        │  Input:                     │
│    • folders: endpoints/  │        │    compilation request      │
│    • MCP docs             │        │    document (14 §14.1.1)    │
│    • block handlers       │        │                             │
│    • default deps         │        │  Does:                      │
│                           │        │    parse → typecheck        │
│  frame.data:              │        │    → expand blocks          │
│    • folders: models/     │        │      (sandbox, ADR-0004)    │
│    • block handler        │        │    → IR → WASM              │
│    • ORM deps             │        │                             │
│                           │        │  Output: .wasm              │
│  frame.ui:                │        │                             │
│    • folders: pages/      │        │  knows: Clean language      │
│    • block handler        │        │  doesn't know: filesystem,  │
│    • DOM bridge deps      │        │    libraries, projects      │
│                           │        │  ← standalone-callable      │
└──────────────────────────┘        └──────────────┬─────────────┘
                                                    │
                                                    │ emits
                                                    ▼
                                    ┌────────────────────────────┐
                                    │       .wasm component       │
                                    │   (portable, self-describing)│
                                    └──────────────┬─────────────┘
                                                    │
                                                    │ runs on
                                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                           HOSTS                                  │
│                                                                  │
│   clean-server         clean-browser          clean-cli          │
│   ─────────────        ──────────────         ───────────────    │
│   HTTP routing         DOM patching           interactive I/O    │
│   sessions             fetch API              file I/O           │
│                                                                  │
│   Each host provides bridge functions the .wasm imports.         │
│   Hosts don't know how the .wasm was built.                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 13. Toolchain Contracts


Three contracts hold the toolchain together. Each is a narrow, versioned interface.

### 13.1 Compilation Request Document — framework → compiler

The framework invokes the compiler with a single self-contained **compilation request document**: every source file inline (`sources[].content`), every configuration value resolved, the library manifests and the target world WIT included. The compiler does no filesystem discovery and never reads `clean.toml`. The schema is owned by [14 §14.1.1](./14-compiler-architecture.md#1411-inputs); this document does not restate it.

The request document fully specifies a compilation unit. This is what makes the compiler standalone-testable and swappable.

### 13.2 Block Handler Contract — framework/compiler → library

Block handlers are written and distributed as Clean source ([LBS §3.2](../02%20components/framework/09-libraries-specification.md)). The framework compiles them to WASM at install time and caches the artifacts, listing each handler's artifact hash in the request document (`library_manifests[].compiletime_wasm_sha256`). During its block-expansion pass the compiler instantiates the cached handler WASM in its sandbox, passes it the typed BlockAST subtree, and receives a typed IR fragment back, which it splices into the program ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md); [14 §14.4.2 pass 6](./14-compiler-architecture.md#1442-detailed-pass-responsibilities)).

### 13.3 Host Bridge — wasm → host

```
             ┌──────────────────────────────────┐
             │  HOST BRIDGE (WIT interface)     │
             │                                  │
   .wasm  ──►│  imports declared in WIT         │──► Host
             │                                  │
             │  Compiler emits the imports.     │
             │  Host provides the exports.      │
             │  Never handshaken at runtime.    │
             └──────────────────────────────────┘
```

The compiler emits WASM component imports declared in WIT; hosts provide matching exports. Version negotiation happens at compile time. See §5-§9 for the bridge surface and §16 for host contract validation.

---

## 14. Build Sequence


```
    Developer runs `cln build`
              │
              ▼
    ┌─────────────────┐
    │   Framework     │
    └────────┬────────┘
             │
             │ 1. "hey Manager, fetch missing libs"
             ▼
    ┌─────────────────┐
    │  Clean Manager  │──► network ──► libs on disk
    └────────┬────────┘
             │
             │ 2. libs are here
             ▼
    ┌─────────────────┐
    │   Framework     │
    │                 │
    │ 3. walk project │
    │ 4. compile+cache│──► block-handler WASM (ADR-0004)
    │    handlers     │
    │ 5. assemble     │
    │    request doc  │
    └────────┬────────┘
             │
             │ 6. compilation request document
             ▼
    ┌─────────────────┐
    │    Compiler     │
    │                 │
    │ parse → check   │
    │ → expand blocks │
    │ → IR → WASM     │
    └────────┬────────┘
             │
             │ 7. .wasm files
             ▼
    ┌─────────────────┐
    │   Framework     │
    │                 │
    │ 8. bundle       │
    │ 9. wire fe/be   │
    └────────┬────────┘
             │
             ▼
        ready to run
```

**Step-by-step:**

1. Clean Manager fetches any libraries missing from the local cache.
2. Libraries are on disk with a lockfile.
3. Framework walks the project directory and validates folder layout against active libraries.
4. Framework compiles any not-yet-cached block handlers to WASM and records their artifact hashes ([ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md)).
5. Framework produces one compilation request document per compilation unit (typically one per target world — server, browser, CLI), with sources inline ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)).
6. Request document goes to the compiler.
7. Compiler parses, typechecks, expands blocks in its sandbox, generates IR, and emits WASM component `.wasm` files.
8. Framework collects the outputs.
9. Framework wires frontend and backend artifacts (writes the server bundle, packages the browser bundle, etc.).

---

## 15. Isolation Guarantee


```
   ┌─────────┐    ┌───────────┐    ┌──────────┐    ┌──────┐
   │ Manager │───►│ framework │───►│ compiler │───►│ host │
   └─────────┘    └───────────┘    └──────────┘    └──────┘
        ▲              ▲                ▲              ▲
        │              │                │              │
        │              │                │              │
   swap for      swap for         swap for        swap for
   any package   any orchestr.    any compiler    any conforming
   manager       (frame, cli,     that speaks     runtime (see the
                  game, ...)      the request     host reference
                                  document        stacks, ADR-0002)
```

Every box in the pipeline can be replaced without touching the others, as long as it honors the contract on its arrow. Concrete implications:

- A **CLI-app framework** or **game framework** can reuse the same compiler with different libraries and folder conventions.
- A **custom compiler** (e.g. one that targets native code instead of WASM) can accept the same compilation request document.
- An **alternate package manager** (npm-style, git-based, monorepo-local) can replace Clean Manager without any framework changes.
- A **new host** only needs to implement the bridge; it doesn't need to know how the `.wasm` was built.

The design tradeoff is a slightly more complex build pipeline in exchange for a compiler that a solo maintainer can hold in their head, and clean seams for alternate tooling. The framework absorbs project validation, handler compilation and caching, and request assembly; the compiler stays a pure `(request document) → wasm` function.

### 15.1 Invariants

The four toolchain invariants:

- **Compiler is a pure function.** Given the same request document, it produces the same `.wasm`. No hidden state, no filesystem access — every input arrives inline in the request ([14 §14.1.1](./14-compiler-architecture.md#1411-inputs)).
- **Framework is the only component that knows about "projects."** Anything that needs `clean.toml`, folder conventions, or library metadata goes through the framework.
- **Libraries are Clean source packages, not runtime artifacts.** A library is source (block handlers, `host function` declarations) plus metadata (suggested folders, MCP docs) plus a dependency list ([LBS](../02%20components/framework/09-libraries-specification.md)) — nothing more.
- **Hosts stay downstream of the toolchain.** The `.wasm` is the boundary. Hosts don't participate in the build.

If those four invariants hold, alternate frameworks, alternate compilers, and alternate hosts can all appear without disturbing each other.

---

## 16. Relationship to Other Sections


| Section | Relationship |
|---------|--------------|
| [Platform 02 — Host Bridge](./02-host-bridge.md) | The bridge surface described here is materialized in WIT files as enumerated there. |
| [Platform 01 — Execution Layers](./01-execution-layers.md) | Defines the six-layer model (L0–L5) this section organizes. |
| [Platform 03 — Memory Model](./03-memory-model.md) | String and rich-value representation follow the WIT canonical ABI. |
| [Platform 08 — Bridge Versioning](./08-bridge-versioning.md) | Governs how WIT packages evolve, how hosts advertise supported ranges, and where the host-support matrix lives. |
| [Platform 12 — Server Extensions](./12-server-extensions.md) | Captured as the `server` world of `clean:host` definition. |
| [Platform 09 — Error Codes](./09-error-codes.md) | Registers the WIT mismatch codes `COM014`, `COM015`, `COM016` (see §11.4). |
| [09 — Libraries Specification](../02%20components/framework/09-libraries-specification.md) | Companion. The library model assumes this architecture for its bridge mechanism. |
| [16 — Host Contract Validation](./16-host-contract-validation.md) | Defines when tooling reads the WIT contracts specified here. |
| [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md) | The framework's MCP-serving role uses the toolchain split defined in §12-§15. |
| [hosts/01 — Server](../02%20components/hosts/clean-server/01-server.md) | Reference implementation of the `server` world of `clean:host`. |
| [00 — Manager](../02%20components/manager/00-manager.md) | Clean Manager owns the `cln` surface and the fetching/versioning role summarized in §12.2 (MGR-01). |

---

## Changelog

- 2026-08-12 — Ratified the one-package host-contract vocabulary decided in `work/2026-08-12-host-wit-naming-decision.md`; no rule renumbering, CMOD-01 amended in place. **§0.3 worlds** extended to the full sanctioned set — `server`, `cli`, `cli-default`, `browser`, `worker`, `edge`, `embedded` — closing the gap that let per-host packages drift in unnoticed (`cli-default`, `worker`, and `edge` had schema files but no sanctioned entry). **§0.3 interfaces** extended with what `clean-server/host.wit` actually declares plus the CLI/browser/worker/edge surfaces: `request`, `response`, `websocket`, `session-envelope`, `realtime-sockets`, `log`, `env`, `commands`, `default`, `fetch`, `router`, `lifecycle`, `events`, `render`, `handler`, `dispatch`, `config`. **§0.3 prohibited list** extended so a `grep` enforces it: `clean:http` as a package; `clean:cli`/`clean:browser`/`clean:worker`/`clean:edge` as packages; `clean:host/cli`/`browser`/`worker`/`edge` as packages (generalizing the existing server-is-a-world line). Added an explicit **"What `clean:host` is not"** paragraph fencing the bridge packages (`clean:data`, `clean:session`, `clean:jobs`, `clean:kv`, `clean:mail`, …) out of the fold — they own their own packages under `02 components/bridges/` and MUST NOT be rewritten into `clean:host`. §5 host-definition sentence and the §2 glossary/§10/§12 host lists renamed `wasmtime_runner` → `clean-cli` (retired: it names a Wasm engine, not a host role).
- 2026-08-05 — Rewrote **§11.1 (Async model)** to commit V2 to WASI 0.3 / Preview 3 native async as the baseline: `clean:bridge@1.0.0` from V2 onward, no `poll`-loop shim, no Preview 2 compatibility path — following the Component Model floor raise in [ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md). Rewrote **§7.1 (Error determinism)** to require wasm-level traps inside a bridge component be lifted into a `bridge-fault(fault-info)` case per the new [BRG-06](./02-host-bridge.md#brg-06--a-bridge-components-trap-must-be-catchable-by-the-caller-as-a-declared-error-variant); guest-side traps remain terminal per host instance-discard rules. Mechanical version-string sweeps: §0.3 `wasi:*` baseline note (WASI 0.3 with `wasi:logging@0.2.0` pinned), §2 glossary example, §5 package example (`clean:bridge@0.1.0` → `@1.0.0`), §5.2 illustrative `server` world WIT (`wasi:http/outgoing-handler@0.2.0` → `wasi:http/handler@0.3.0`, `wasi:http/incoming-handler@0.2.0` → `wasi:http/service@0.3.0`, `clean:bridge/db@0.1.0` → `@1.0.0`). No rule renumbering. No changes to CMOD-01..CMOD-04.
- 2026-08-01 — Conflict-log remediation pass (P1–P8, P16; work/2026-08-01-conflict-log-platform.md). §0 chapeau rewritten: this document is the single home of the WIT vocabulary (§0.3) and quotes the version baseline from 08 §8.0; the "this section wins" precedence clause removed (conflicts resolve per DOC-11). §0.1 reduced to citations of boundaries/MGR-01 and aligned with [ADR-0004](../01%20governance/decisions/0004-block-handler-execution-model.md) (framework compiles/caches handlers; compiler executes them sandboxed, typed IR out) and the request document of 14 §14.1.1. §0.3 server interface list amended per [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md) (`auth`, `admin`, `mcp`, `diagnostics`). §0.6–0.9 reduced to citations of their homes. P4/§4/§5 L2 catalog corrected to 02 §2.2.1 (`ui`/`canvas`/`auth`/`session` are `clean:host/*`, not bridge). P5 aligned with LBS §8.1/C-07 (framework-synthesized WIT). §5.2 reduced to an illustrative fragment citing 12 §13 as owner. §6.1/§11.7 corrected per P8 (compiler validates against the world WIT in the request document; never fetches host WIT). §11.4 codes replaced with `COM014`/`COM015`/`COM016`. §11.6/§13.2 rewritten per ADR-0004 (no "native modules", no "vanilla .cln text", no "no sandboxed sub-runtime"). §12.3 "Redefinitions"/"convention pack" removed in favor of glossary/boundaries/manager citations. §13.1/§14/§15 aligned to the compilation request document. Engine/toolchain names replaced by citations of [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md)/ADR-0002; "wasmtime CLI" → `wasmtime_runner`. "five-layer" → six (L0–L5). Book-style footer normalized to the platform tree.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler maintainers, host implementors (clean-server, clean-browser, clean-cli), library authors
- **Rule prefix:** `CMOD-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [02 — Host Bridge](./02-host-bridge.md), [08 — Bridge Versioning](./08-bridge-versioning.md), [14 — Compiler Architecture](./14-compiler-architecture.md), [16 — Host Contract Validation](./16-host-contract-validation.md), [ADR-0022 §3](../01%20governance/decisions/0022-foundational-technology-stack.md)
- **Satisfies:** INTEROP-01, INTEROP-02, INTEROP-04, INTEROP-06
