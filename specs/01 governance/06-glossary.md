# Glossary — Clean Language Ecosystem

This is the single home for the controlled vocabulary of the Clean Language ecosystem. Every term used across the specifications, governance documents, and ADRs is defined here exactly once and used identically everywhere. Introducing a synonym for an existing term is a defect — this file exists so that "class", "capability", "companion", "principal", "session", and every other named concept have one and only one meaning across the tree.

---

## Part 0 — Purpose

This document is the single home for the controlled vocabulary of the ecosystem. Each term is defined once here, and used identically everywhere; introducing a synonym for an existing term is a defect.

Each entry names the term, defines it in one or two sentences, links to the document that owns the underlying rule, and — where a synonym was in circulation — records the rejected form so the defect is not reintroduced.

An entry here never restates a rule. It fixes what a word means; the owning document fixes what is true.

---

## Part 1 — Language

**Block.** An indented, structured construct introduced by `identifier:` or `namespace.identifier:`. The parser recognizes it generically; a library gives it meaning. Owner: [09 — Libraries Specification §3.1](../02%20components/framework/09-libraries-specification.md).
*Do not use:* "framework block", "DSL block", "custom block", "declarative block" — all denote this same concept.

**Block handler.** A compile-time function registered against a block name with `handles block`, which turns that block into typed IR. Owner: [21 — Block Handlers](../04%20language/21-block-handlers.md).

**Compile-time function.** A Clean function marked `compiletime` that runs during compilation, receives a typed `BlockAST`, and returns typed `IR`. Owner: [21 — Block Handlers](../04%20language/21-block-handlers.md).
*Do not use:* "`compiletime` function" as a distinct term — it is the same thing.

**Capability.** A named contract of method signatures, with no bodies, that a class claims with `can`. Owner: [14 — Classes and Objects](../04%20language/14-classes-and-objects.md).
*Do not use:* "trait" or "interface" when referring to a Clean capability.

**Companion.** A class reachable through another class's field by writing `ClassName.fieldName` (companion access), used as a namespace for that type's static methods. Owner: [14 — Classes and Objects §Companion Access](../04%20language/14-classes-and-objects.md#cls-05--companion-access).
*Qualify when ambiguous:* a **data companion** is the persistence companion of an entity ([data library](../02%20components/framework/libraries/04-data.md)); a **page loader** is the `.cln` file paired with an HTML page ([ui library](../02%20components/framework/libraries/10-ui.md)) — a page loader is not a companion.

**Entity.** A domain class that holds fields, invariants, and business methods, and carries no persistence behavior. Owner: [data library §2](../02%20components/framework/libraries/04-data.md).

**Literal.** A value written directly in source text, which the compiler reads without evaluating anything: `42`, `"hola"`, `true`, `3.14`, `[1, 2]`. A value produced by a computation or held in a variable is not a literal. Owner: [03 — Lexical Structure](../04%20language/03-lexical-structure.md#lex-06--literal-forms).

**`list<T>`.** The homogeneous resizable collection type. Owner: [04 — Type System](../04%20language/04-type-system.md).
*Do not use:* `Array<T>` — it is not a Clean type, and "array" is not a Clean word for it.

**`pairs<K,V>`.** The key–value associative type — Clean's map. `K` is a free type parameter, not fixed to `string`. Owner: [04 — Type System](../04%20language/04-type-system.md).
*Do not use:* `map<K,V>` — no such type exists or is planned.

**List behavior.** A suffix on a list type (`.line`, `.pile`, `.unique`) that fixes which element the position-dependent operations act on. Owner: [04 — Type System §List Behaviors](../04%20language/04-type-system.md).
*Do not use:* "modifier", "property", or "suffix" as a separate term for this — they all denote the behavior.

**Apply-block.** A block whose header is a callable followed by `:`, applying that callable to each indented item (`items.add:`). It is one kind of [Block](#part-1--language); a **library block** is the other, given meaning by a block handler. Owner: [05 — Apply-Blocks](../04%20language/05-apply-blocks.md).

**Precision modifier.** A width suffix naming the WIT width of a `host function` parameter or return type (`integer:32`). It is a property of the host boundary and not a type of the surface language, where the only numeric types are `integer` and `number`. Owner: [Libraries Specification §8.3](../02%20components/framework/09-libraries-specification.md#8-host-bridge-as-typed-host-function-declarations) ([ADR-0019](decisions/0019-precision-modifiers.md)).

**Contextual keyword.** A reserved word that is reserved *only* as a block header (`state:`), and is an ordinary identifier anywhere else. Distinct from a **hard keyword**, which is reserved everywhere. Owner: [03 — Lexical Structure §Keywords](../04%20language/03-lexical-structure.md).

---

## Part 2 — Libraries and packaging

**Library.** A Clean package, distributed as Clean source, that extends the framework through blocks, compile-time functions, capabilities, and host function declarations. Owner: [09 — Libraries Specification](../02%20components/framework/09-libraries-specification.md).

**Plugin.** An installable dependency that connects a project to an external service, resolved by `cln add` from the same `[dependencies]` table as a library. A plugin is distinguished from a library by what it provides — a service connector rather than language surface — not by how it is installed. Owner: [Manager §00.3.2](../02%20components/manager/00-manager.md).

**Core / Community / Local.** The three library certification tiers. Owner: [09 — Libraries Specification §10.2](../02%20components/framework/09-libraries-specification.md).
*Do not use:* "official libraries" — the tier is **Core**.

**Frame.** The full-stack framework for Clean Language. Owner: [Framework overview](../02%20components/framework/00-overview.md).
*Do not use:* "Clean Framework" or "Frame Framework" in prose. `clean-framework` remains correct as the binary name.

**Project manifest.** `clean.toml` at the project root. **Library manifest.** `library.toml` in a library package. Owners: [09 §5–§6](../02%20components/framework/09-libraries-specification.md), [Platform 07](../03%20platform/07-build-config.md).

---

## Part 3 — Platform and hosts

**Host.** A runtime that loads and executes a compiled Clean `.wasm` component and provides the imports it declares. Owner: [15 — Component Model Architecture](../03%20platform/15-component-model-architecture.md).

**World.** A WIT declaration naming the interfaces a component may import and a host must provide (`server`, `cli`, `cli-default`, `browser`, `worker`, `edge`, `embedded`). Every host world is declared inside the single `clean:host` package, and a world is never part of a package or interface path — `clean:host/cli` is a prohibited form. Owner: [15 §0.3](../03%20platform/15-component-model-architecture.md).

**Interface.** A named group of functions inside a WIT package, cited as `package/interface` — two levels, never three. Owner: [15 §0.3](../03%20platform/15-component-model-architecture.md).

**Host function.** A function the host implements and the component calls, declared in a library's `host_bridge.cln` with the `host function` grammar. Owner: [09 §8](../02%20components/framework/09-libraries-specification.md).
*Do not use:* underscore-prefixed names (`_ui_*`, `_db_*`) in user-facing text — they exist only inside synthesized WIT ([09 §8.6](../02%20components/framework/09-libraries-specification.md)).

**Host bridge.** The WIT import layer through which a component reaches host capabilities. Owner: [Platform 02](../03%20platform/02-host-bridge.md).
*Do not use:* "host bridge" for a platform-specific native wrapper (Capacitor, Tauri); call that a **platform wrapper**.

**Host WIT.** The WIT document a host publishes declaring what it provides. **Project WIT.** The WIT embedded in a compiled `.wasm` component declaring what it needs. Compliance is checked between the two. Owner: [Platform 16 §16.2](../03%20platform/16-host-contract-validation.md#162-core-idea--two-wit-documents).

**clean-cli.** The canonical name of the reference CLI host — the runtime that fulfills the `cli` and `cli-default` worlds of `clean:host` ([CMOD-01](../03%20platform/15-component-model-architecture.md#cmod-01--one-wit-naming-scheme-extended-only-by-adr)). Owner: [Hosts overview](../02%20components/hosts/README.md), spec at [hosts/clean-cli](../02%20components/hosts/clean-cli/01-specification.md).
*Do not use:* "wasmtime CLI" or `wasmtime_runner` — both name a Wasm engine, not a host role. `wasmtime_runner` is **retired** as of 2026-08-12; a developer never needs to know which engine runs their CLI app. *Do not use:* `clean-runtime` for this host either — that is the name of the binary containing every host (see below), not of any one host.

**Build target.** The `(architecture, host-world, ABI)` triple selected by `--target`, e.g. `wasm32-server`. Owner: [Platform 07 §7.4](../03%20platform/07-build-config.md).
*Do not use:* platform names (`web`, `pwa`, `mobile`, `desktop`) as target values — those are packaging flows ([08 — Platforms §2](../02%20components/framework/08-platforms.md)).

**Guard.** Qualify every use: a **state guard** is the `guard <expr> else "<msg>"` clause on a state declaration ([20 — State Management](../04%20language/20-state-management.md)); a **route guard** is the inline modifier on an endpoint; a **route directive** is the `guard:` entry in `routes.cln`; a **companion guard** is the `guard()` function on a page loader. Owners: [20 — State Management](../04%20language/20-state-management.md), [server library §5, §17](../02%20components/framework/libraries/08-server.md), [ui library §5.2](../02%20components/framework/libraries/10-ui.md).

---

## Part 4 — Tooling

**Clean Manager.** The single user-facing binary, `cln`. It owns argv parsing, dispatch, version resolution, and the on-disk layout. Owner: [Manager](../02%20components/manager/00-manager.md).

**MCP server.** The single Model Context Protocol server the framework exposes for AI clients and IDEs. Owner: [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md).
*Do not use:* "framework MCP", "the MCP", "Clean MCP server" as if they were distinct things — there is one.

**Diagnostic code.** An identifier matching `PREFIX###` registered in [Platform 09](../03%20platform/09-error-codes.md). Diagnostics emitted by libraries travel as `LIB010` with a library-supplied sub-label; libraries do not own code prefixes.

**Rule ID.** A stable identifier of the form `PREFIX-NN` naming one normative rule, registered in the [prefix registry](README.md). Distinct from a diagnostic code. Owner: [DOC-13](00-documentation-principles.md).

**Request document.** The self-contained JSON input the compiler accepts — every source file inline, every configuration value resolved; the compiler reads nothing else. Owner: [Platform 14 §14.1.1](../03%20platform/14-compiler-architecture.md#1411-inputs).

**Telemetry.** Clean Manager's adoption heartbeat (versions, OS, an opaque installation UUID), managed with `cln telemetry <on|off|status>`. Owner: [Manager §00.10](../02%20components/manager/00-manager.md#0010-telemetry).
*Do not use:* "telemetry" for the error-reporting consent level — that is **report consent**.

**Report consent.** The user-set consent level governing what an error report may contain, set with `cln report consent <level>`. Owner: [Platform 06 §6.6](../03%20platform/06-error-reporting.md#66-privacy-and-consent).
*Do not use:* "telemetry" for this setting — telemetry is the Manager's adoption heartbeat, a separate system.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Anyone writing or reviewing documentation across the Clean Language ecosystem
- **References:** [Documentation Principles](00-documentation-principles.md), [Architectural Concerns](05-concerns.md)
