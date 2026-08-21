# Platform 04. IDE and Language Server Architecture

This chapter defines how editors provide language intelligence for Clean programs — highlighting, completions, hover, jump-to-definition, quick fixes — through a single language server that ships with the compiler and speaks standard LSP. It is a contract: an editor extension that implements this chapter works against any compiler shipping the reference language server, and every library a user depends on gains editor support automatically with no per-library plugin work. The governing principle is [LSP-01](#lsp-01--extensions-are-thin-clients-of-the-language-server): the language server is the single source of truth for all language intelligence, and editor extensions are thin LSP clients that add no language knowledge of their own.

---

## 4.1 What the Language Server Provides


The reference compiler ships a **language server** implementing the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) (LSP). The server is compiled from the same source as the batch compiler; there is no divergence between "what the IDE understands" and "what compiles."

The server provides these LSP capabilities:

| LSP capability | What it provides |
|---|---|
| `textDocument/semanticTokens` | Syntax highlighting for keywords, types, functions, blocks, and library-defined block names. |
| `textDocument/completion` | Identifier, member, and block-name completion including library-contributed blocks. |
| `textDocument/hover` | Type of the expression under the cursor, function/method signatures, doc comments. |
| `textDocument/publishDiagnostics` | Parse, type-check, and library-emitted diagnostics ([§21.6](../04%20language/21-block-handlers.md#216-diagnostics-from-compile-time-functions)). Streamed as the user types. |
| `textDocument/definition` | Jump to definition across files and into libraries. |
| `textDocument/references` | Find all references to a symbol. |
| `textDocument/formatting` | Whole-file formatting to the canonical style. |
| `textDocument/rename` | Symbol rename with cross-file updates. |
| `textDocument/codeAction` | Compile-time-produced quick-fixes (add missing import, extract to function, apply library-provided fix hint). |
| `textDocument/inlayHint` | Inferred types and companion accesses shown inline. |
| `workspace/symbol` | Project-wide symbol search. |

Semantic tokens carry rich detail: a token identifies not just "keyword" but "block-name from library `data` version 1.4.0." Editors that support the modifier field render accordingly (colors, tooltips, gutter icons).

### 4.1.1 Diagnostic Data Threading


Every `Diagnostic` the compiler emits carries a structured `Suggestion` list with `Applicability` tags (see [`13-diagnostic-format.md`](./13-diagnostic-format.md) §5). LSP's `Diagnostic` type has no field for suggestions — they are delivered via `textDocument/codeAction` when the editor requests actions at a diagnostic's range.

To avoid re-running analysis every time the editor asks for actions at a diagnostic, the language server threads the untouched compiler `Diagnostic` value through LSP's `data` field on every `PublishDiagnosticsParams` entry. The protocol contract is:

1. **On `publishDiagnostics`**, the server serializes the full compiler `Diagnostic` (JSON schema from [`13-diagnostic-format.md`](./13-diagnostic-format.md) §6.1) into the LSP `Diagnostic.data` field. This field was added in LSP 3.16 precisely for round-tripping server-private state.
2. **On `codeAction`**, the editor sends back the diagnostics whose range the cursor is on, `data` field intact. The server deserializes `data` back into the compiler `Diagnostic`, reads `suggestions[]`, and emits an LSP `CodeAction` per suggestion — `title` from the suggestion's `message`, `kind` from its `Applicability` (`quickfix` for `MachineApplicable`, `quickfix.preferred: false` for `MaybeIncorrect`, `snippet` insertion for `HasPlaceholders`), and `edit` from the `replacements[]`.
3. **The server never re-runs the compiler for a `codeAction` request** unless the file has changed since the diagnostic was published. All information needed to produce actions was already computed at publish time.

### LSP-02 — Editors preserve `Diagnostic.data` verbatim


**Editor cache contract.** Editors MUST preserve `Diagnostic.data` verbatim — unstripped, unmutated, unreordered — between `publishDiagnostics` and `codeAction`. Editors that strip unknown fields break Quick Fixes silently; the server has no way to detect this and cannot fall back to re-analysis without a stale-cache risk.

**AI agent path.** Agents that consume the NDJSON diagnostic stream directly (via `cln check --diagnostic-format=json`; the `cln` command surface is owned by [Clean Manager §00.3](../02%20components/manager/00-manager.md#003-command-surface)) bypass LSP entirely and read `suggestions[]` inline. Agents in autonomous mode apply `MachineApplicable` edits without prompting; other applicability levels are surfaced to the user. This is the same contract as the editor, minus the LSP round-trip.

---

## 4.2 What the Editor Extension Provides

### LSP-01 — Extensions are thin clients of the language server


The language server is the single source of truth for all language intelligence; an editor extension MUST NOT add language knowledge of its own. An editor extension is responsible for **five things and only five things**:

1. **Register the `.cln` file extension** and any file-level associations (icons, editor mode).
2. **Launch the language server** as a child process (stdio) or WebSocket connection, and speak LSP to it.
3. **Provide a minimal TextMate grammar** covering only lexical primitives — string literals, line comments, numeric literals, operator characters — so the file is not blank while the server is starting up. See §4.3 for the strict list of what this grammar contains.
4. **Surface LSP features to the user** — highlighting from semantic tokens, popups for completions and hover, gutter marks for diagnostics, side panels for references.
5. **Provide editor-idiomatic commands** — "restart language server," "show output panel," "open compiler version selector," "run current file." These are UI, not language logic.

The extension MUST NOT do any of the following:

- Ship a keyword list, type list, or built-in function list in the extension code.
- Provide completions, hover, or diagnostics without asking the server.
- Recognize block names (`data:`, `endpoints:`, `component:`, library-defined blocks) at the extension layer.
- Load libraries or parse `.cln` source.
- Fall back to hardcoded highlighting when the server is unavailable — see §4.5.
- Strip, mutate, or reorder the `Diagnostic.data` field between `publishDiagnostics` and `codeAction`. This field is opaque server state — see §4.1.1.

---

## 4.3 The Minimal Startup Grammar

### LSP-03 — The startup grammar contains no language knowledge


The extension MAY ship a TextMate grammar so the file is readable during the ~100 ms window before the language server sends its first semantic-tokens response. This grammar is **strictly bounded** — it MUST contain only items from the "Allowed" column and MUST NOT contain any item from the "Not allowed" column:

| Allowed | Not allowed |
|---------|-------------|
| String literals (`"..."` including escapes) | Keywords (`if`, `iterate`, `class`, `function`, ...) |
| Character literals | Type names (`integer`, `string`, `list`, ...) |
| Line comments (`// ...`) | Built-in function names (`print`, `map`, ...) |
| Numeric literals (integer, float, hex, binary) | Block names (`data:`, `endpoints:`, ...) |
| Operator punctuation (`+`, `-`, `*`, `/`, `==`, ...) | Library-contributed block names |
| Bracket matching pairs | Anything else that the language server would communicate as a semantic token |

Once the server responds with semantic tokens for a file, the TextMate grammar's contribution is invisible — semantic tokens take precedence in every LSP-conformant editor.

**Enforcement:** the reference extension repository CI includes a lint that rejects any grammar file whose token list contains a Clean keyword or type. Adding a keyword to the extension is a rejected pull request.

---

## 4.4 How Libraries Automatically Gain Editor Support

This section describes consequences of [LSP-01](#lsp-01--extensions-are-thin-clients-of-the-language-server); the contracts it exercises are owned by the sections it links to.

Because all language intelligence flows through the server, and the server processes libraries the same way it processes user code, a library that ships a new block handler ([§21](../04%20language/21-block-handlers.md)) automatically produces:

- **Completions.** The server offers the block name when the user is at a statement position where a block is valid.
- **Semantic tokens.** The block name is tokenized as `block-name` with modifier `library:data@1.4.0`.
- **Hover.** The library's block handler carries a `description` string (see [§21.1](../04%20language/21-block-handlers.md#211-declaring-a-block-handler)) which the server returns as hover content.
- **Diagnostics.** `error(code, message, span)` calls from the handler flow through `textDocument/publishDiagnostics` unchanged; the editor renders them with the same UI as compiler errors.
- **Quick-fixes.** A library may emit `code-action-suggestion` metadata alongside a diagnostic; the server converts these into LSP code actions.
- **Rename and jump.** Symbols defined inside a block (companion classes, generated functions) participate in `textDocument/references` and `textDocument/definition` if the handler emits IR with proper spans ([§21.5](../04%20language/21-block-handlers.md#215-span-preservation)).

**No editor extension update is required** when a user adds a new library dependency. The extension does not know about `data`, `ui`, or any library — the server does, and the extension asks the server.

---

## 4.5 Server Unavailability

### LSP-04 — No synthesized intelligence when the server is unavailable


When the language server crashes, has not started, or is being restarted:

1. The extension displays a status-bar indicator showing the server state (`starting`, `running`, `stopped`, `crashed with reason X`).
2. LSP requests queued during unavailability MAY be dropped or held; the extension chooses per capability. Completions are dropped (they are stale by the time the server returns); diagnostics are held and re-requested on server ready.
3. The extension MUST NOT synthesize completions, hover, or diagnostics from a hardcoded list. Providing wrong information is worse than providing none.
4. The TextMate startup grammar (§4.3) remains active — the file remains readable.
5. When the server returns, the extension re-requests semantic tokens for all open documents. There is no "degraded mode" language state.

**Rationale** *(informative)*: an extension synthesizing its own completions or highlights inevitably drifts from what the compiler accepts. Wrong information is worse than none.

---

## 4.6 Multi-Version Compiler Support


A workspace may pin a specific compiler version via `cln pin`, which writes the pin to `.cln/version` (see [Clean Manager §00.3.3](../02%20components/manager/00-manager.md#0033-toolchain-versions)). The extension:

1. Resolves the pinned version through [Clean Manager](../02%20components/manager/00-manager.md).
2. Launches the language server binary matching that version — `~/.cln/versions/compiler/<version>/clean-language-server`, installed alongside `clean-compiler` from the same release archive ([CCMP-26](../02%20components/compiler/01-specification.md)). The layout is owned by [Clean Manager §00.2](../02%20components/manager/00-manager.md#002-on-disk-layout).
3. Reports the launched version in the status bar.

If two workspaces open in the same editor session pin different versions, the extension launches one server per workspace. Servers do not share state; each speaks to its own workspace.

### LSP-05 — The extension never bundles a compiler


The extension MUST NOT bundle a compiler or language-server binary. It relies on Clean Manager to resolve and provide the binary for the pinned version (`cln pin` → `.cln/version`, [Clean Manager §00.3.3](../02%20components/manager/00-manager.md#0033-toolchain-versions)).

---

## 4.7 Contract Tests


The reference extension includes a contract test suite that verifies each LSP capability against a running language server. Any editor claiming Clean Language support that wants to be listed as "conformant" must pass this suite. The suite covers:

- Semantic tokens for user syntax, blocks, and library-contributed blocks.
- Completions at cursor positions inside and outside blocks.
- Diagnostics for parse errors, type errors, and library-emitted errors.
- Definition and reference resolution across file boundaries.
- Rename correctness (no missed references, no false positives).
- Behavior under server restart.

The suite is compiled from Clean source and runs against the batch language server binary, so it exercises the same code the shipping IDE uses.

---

## 4.8 Non-Goals


- **Extension-side plugins.** An editor extension does not accept its own plugins or extensions. If someone wants to add a new capability to Clean's IDE support, it goes in the language server, not a per-editor extension. This keeps all editors at feature parity.
- **Editor-specific language behavior.** The server responds identically to every LSP client. No "VS Code offers X but JetBrains does not" divergence.
- **Third-party language servers.** There is one reference language server, shipped with the compiler. Alternate implementations are welcome but are treated as third-party — the specification does not obligate the compiler team to accept LSP requests other implementations happen to send.

---

## 4.9 Deferred Refinements


1. **MCP-backed completions.** The compiler does not ship an MCP server. There is exactly one Clean MCP server, owned by the framework ([ADR-0001 — Single MCP Server](../01%20governance/decisions/0001-single-mcp-server.md), Accepted); the compiler exposes the language-info API that the MCP server consumes ([10 — MCP Server Architecture §10.5](../02%20components/framework/10-mcp-server-architecture.md#105-the-compiler-language-info-api)). The language server does not proxy MCP tools as LSP code-actions in V2 — MCP is for AI clients, LSP is for editors, and the two stay decoupled.
2. **Incremental parse cache sharing.** Each workspace has its own server process and its own parse cache. V2 does not share parse caches across workspaces, even when they use the same compiler version.

---

## Changelog

- 2026-08-20 — §4.6 step 2 names the on-disk path of the language server binary: `~/.cln/versions/compiler/<version>/clean-language-server`, citing [Clean Manager §00.2](../02%20components/manager/00-manager.md#002-on-disk-layout), which gained the entry the same day. [CCMP-26](../02%20components/compiler/01-specification.md) ships the server with the compiler and this chapter launches "the binary matching that version", but no document said where the binary lives — a gap surfaced by the compiler's first release packaging (`clean-language-compiler@0a26375`), whose archive carries `clean-compiler` and `clean-language-server` side by side.

- 2026-08-01 — Conflict-log remediation (Fase 3): §4.9 corrected per [ADR-0001](../01%20governance/decisions/0001-single-mcp-server.md) (Accepted) — the compiler does not ship an MCP server; it exposes the language-info API consumed by the framework's MCP server (10-mcp §10.5). §4.6 compiler pinning corrected per P11 — `cln pin` / `.cln/version` (Clean Manager home), replacing the abolished `clean.toml [tools]` mechanism; the mislabeled `clean-manager` link now points at the Manager spec; "version manager" renamed to Clean Manager. "Framework blocks" renamed to blocks (P16.9). Stale `#24x` anchors repointed to the real 21.x headings. §4.1.1 notes Clean Manager as the home of the `cln check` command surface.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Editor extension authors and language-server maintainers
- **Rule prefix:** `LSP-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [Libraries Specification](../02%20components/framework/09-libraries-specification.md), [Block Handlers](../04%20language/21-block-handlers.md), [Diagnostic Format](./13-diagnostic-format.md)
- **Satisfies:** LANG-03, PERF-05, INTEROP-08
