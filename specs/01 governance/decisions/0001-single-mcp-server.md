# ADR-0001 — One MCP server, with a non-MCP language-info API in the compiler

An AI assistant working on a Clean project needs two kinds of answers — project-shaped (what libraries are in scope, what does `clean.toml` say) and language-shaped (what does an error code mean, does this snippet typecheck) — and each kind lives in a different component. This ADR settles that the toolchain exposes exactly one MCP server, hosted by the framework: the compiler stays a pure function and exposes a plain language-info API that the framework wraps, so an AI client sees a single unified tool namespace.

---

## Context

An AI assistant or IDE working on a Clean project needs two very different kinds of answer:

- **Project-shaped:** which host does this project target, which libraries are in scope for this folder, what does `clean.toml` say, what diagnostics are outstanding. This data lives in framework state.
- **Language-shaped:** what are the built-in functions, what does error `SEM014` mean, what does the spec say about indentation, does this snippet typecheck. This data lives in the compiler, and must reflect the compiler version the project has pinned.

Two components own the data. The question was whether both should speak MCP.

The pressure toward two servers is that each component would then serve its own data directly. The pressure against is that an AI client would have to discover, launch, authenticate and reconcile two tool namespaces, and would have to know which server answers which question — a distinction that is an implementation detail of our toolchain, not of the user's problem.

Concern [C-12](../05-concerns.md) states that every capability an AI needs is exposed through the Clean MCP server; concern [C-20](../05-concerns.md) requires answers to reflect the exact pinned versions.

## Decision

**Option C.** There is exactly one MCP server in the toolchain: the framework MCP. The compiler exposes a language-info API that is an ordinary CLI and linkable-library surface, not MCP. The framework calls it and presents a single unified tool namespace to the client.

Every MCP tool belongs to exactly one component; there is no shared ownership and no duplicated tool. A tool whose data comes from the compiler is still served by the framework MCP.

## Options considered

**A — Two MCP servers, one per component.** Each component serves what it owns. Direct, but the AI client must run and reconcile two servers, and every tool call needs a routing decision the client is not equipped to make. Tool-name collisions become a cross-component coordination problem.

**B — One MCP server in the compiler.** The compiler would have to read `clean.toml`, resolve libraries and know about projects — violating [C-08](../05-concerns.md) (the compiler is a pure function with no filesystem access and no library awareness).

**C — One MCP server in the framework; the compiler exposes a plain language-info API (chosen).** The framework is the sole MCP speaker. For language-shaped questions it calls a non-MCP surface the compiler exposes (`list_builtins()`, `get_specification(section)`, `check(source)` …) and wraps the result. The compiler stays a pure function; the AI sees one namespace.

## Consequences

**Easier.** One endpoint to configure (`cln mcp install`), one namespace to discover, no routing decision at the client. Language answers automatically track the pinned compiler version, because the framework dispatches to the version the project resolves to — satisfying [C-20](../05-concerns.md).

**Harder.** Every language-backed tool costs an extra hop, and the framework must keep its wrapper in step with the compiler's API. The compiler's language-info API becomes a public contract with its own versioning obligations, even though it is not MCP.

**Now required.** [10 — MCP Server Architecture](../../02%20components/framework/10-mcp-server-architecture.md) specifies the resulting surface — tool ownership, shared on-disk state, and the lifecycle — and cites this ADR as its rationale rather than re-arguing the decision ([DOC-07](../00-documentation-principles.md#doc-07--the-ladder-of-intent)).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Date:** 2026-08-01
- **Supersedes:** None
- **Spec impact:** [02 components / framework 10 — MCP Server Architecture](../../02%20components/framework/10-mcp-server-architecture.md) (the surface it specifies), [Manager §00.3.6](../../02%20components/manager/00-manager.md) (`cln mcp`), [04 language / 19 — AI Integration](../../04%20language/19-ai-integration.md) (added 2026-08-01 — the chapter described this decision's arrangement without citing it, and the omission was mutual)
