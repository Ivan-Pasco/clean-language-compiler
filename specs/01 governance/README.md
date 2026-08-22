# 01 — Governance

This folder holds the rules that govern how the Clean Language ecosystem is documented, decided, and built. Governance sits at the top of the precedence order — when documents conflict, governance wins ([DOC-11](00-documentation-principles.md)). The prefix registry lower down is the authoritative list of every stable rule prefix in the ecosystem; claiming a new one means adding a row here first.

## Contents

| Document | Purpose |
|----------|---------|
| [00 — Documentation Principles](00-documentation-principles.md) | The rules all documents follow: the ladder of intent, statuses, templates, rule IDs |
| [01 — Architecture Boundaries](01-architecture-boundaries.md) | Authoritative responsibility boundaries for every component |
| [02 — Quality Playbook](02-quality-playbook.md) | The complete quality regime: hooks enforce, prompts request |
| [03 — Spec-Driven Design](03-spec-driven-design.md) | How specs are designed and how code must respect them |
| [04 — Execution Model](04-execution-model.md) | The actors, sessions, repos, and gates that turn Accepted specs into code |
| [05 — Architectural Concerns](05-concerns.md) | The concerns the ecosystem is designed to address, grouped by stakeholder. Every normative rule cites at least one concern. |
| [06 — Glossary](06-glossary.md) | The controlled vocabulary: one term per concept, one definition per term |
| [07 — Language Principles](07-language-principles.md) | The durable design commitments that shape the Clean Language itself, above the language specification |
| [08 — Security Principles](08-security-principles.md) | The threat model and the security commitments the ecosystem makes; overrides LANG principles inside the declared threat surface |
| [09 — Performance Principles](09-performance-principles.md) | The performance model and cost commitments; preserves LANG and SEC principles at the implementation level |
| [10 — Interoperability Principles](10-interoperability-principles.md) | The boundary and composition commitments; how Clean components, hosts, libraries, and external systems fit together |
| `decisions/` | ADRs — append-only design decisions ([DOC-07](00-documentation-principles.md#doc-07--the-ladder-of-intent)). ADR-0022 records the foundational technology stack every principles doc depends on. |

## Rule ID prefix registry

Each document owns exactly one prefix ([DOC-13](00-documentation-principles.md#doc-13--stable-ids-for-checkable-rules-no-ceremony-for-prose)). Claim a new prefix by adding a row here before using it.

| Prefix | Owning document | Scope |
|--------|-----------------|-------|
| `DOC-` | [00 — Documentation Principles](00-documentation-principles.md) | Documentation and process rules |
| `SDD-` | [03 — Spec-Driven Design](03-spec-driven-design.md) | Spec design and code–spec fidelity rules |
| `EXE-` | [04 — Execution Model](04-execution-model.md) | Actors, sessions, and gates for spec-to-code execution |
| `C-` | [05 — Architectural Concerns](05-concerns.md) | Concerns the ecosystem addresses, grouped by stakeholder |
| `LANG-` | [07 — Language Principles](07-language-principles.md) | Durable design commitments that shape the Clean Language |
| `SEC-` | [08 — Security Principles](08-security-principles.md) | Threat model and security commitments |
| `PERF-` | [09 — Performance Principles](09-performance-principles.md) | Performance model and cost commitments |
| `INTEROP-` | [10 — Interoperability Principles](10-interoperability-principles.md) | Boundary and composition commitments across the component, host, library, and toolchain boundaries |
| `PREFIX###` diagnostic codes | [Error Codes](../03%20platform/09-error-codes.md) | Compiler/runtime diagnostics — the code pattern and range table of 09 §1 (e.g. `SYN001`, `LIB010`); distinct from rule IDs |
| `CNF-` | [04 language / 00 — Scope and Conformance](../04%20language/00-scope-and-conformance.md) | normative vocabulary, valid program, conforming implementation |
| `LDR-` | [04 language / 02 — Language Design Rules](../04%20language/02-language-design-rules.md) | the "one way to do things" rules |
| `LEX-` | [04 language / 03 — Lexical Structure](../04%20language/03-lexical-structure.md) | tokens, alphabet, keywords, literals, indentation |
| `TYP-` | [04 language / 04 — Type System](../04%20language/04-type-system.md) | core, composite and optional types; conversions |
| `APB-` | [04 language / 05 — Apply-Blocks](../04%20language/05-apply-blocks.md) | the `identifier:` apply-block form |
| `EXP-` | [04 language / 06 — Expressions](../04%20language/06-expressions.md) | precedence, associativity, operators |
| `STM-` | [04 language / 07 — Statements](../04%20language/07-statements.md) | declaration, assignment, `return` |
| `FIL-` | [04 language / 08 — File Structure](../04%20language/08-file-structure.md) | top-level sections and their order |
| `FNC-` | [04 language / 09 — Functions](../04%20language/09-functions.md) | declaration, parameters, `start:` |
| `CTR-` | [04 language / 10 — Contracts](../04%20language/10-contracts.md) | `before`, `after`, `always` |
| `TST-` | [04 language / 11 — Testing](../04%20language/11-testing.md) | the `tests:` block |
| `FLW-` | [04 language / 12 — Control Flow](../04%20language/12-control-flow.md) | `if`, `iterate`, `while` |
| `ERH-` | [04 language / 13 — Error Handling](../04%20language/13-error-handling.md) | `error(...)`, `onError`, the failure path |
| `CLS-` | [04 language / 14 — Classes and Objects](../04%20language/14-classes-and-objects.md) | classes, capabilities, companion access |
| `STD-` | [04 language / 15 — Standard Library](../04%20language/15-standard-library.md) | the standard-library catalog and string patterns |
| `CALL-` | [04 language / 16 — Method-Style Syntax](../04%20language/16-method-style-syntax.md) | method vs namespace call style |
| `MOD-` | [04 language / 17 — Modules and Imports](../04%20language/17-modules-and-imports.md) | modules, imports, module visibility |
| `ASY-` | [04 language / 18 — Asynchronous Programming](../04%20language/18-async.md) | `start`, `later`, `background` |
| `AIM-` | [04 language / 19 — AI Integration](../04%20language/19-ai-integration.md) | `spec`, `intent`, `source:` metadata |
| `SMG-` | [04 language / 20 — State Management](../04%20language/20-state-management.md) | state declaration, guards, computed state, `watch` |
| `BLK-` | [04 language / 21 — Block Handlers](../04%20language/21-block-handlers.md) | block handler declaration, resolution, execution environment |
| `ADR-` | `decisions/` | Design decisions, numbered sequentially |
| `FRM-` | [02 components / framework 01 — Framework Specification](../02%20components/framework/01-framework-specification.md) | Frame project structure, folder scope, `[folders]` schema, block ownership |
| `CLI-` | [02 components / framework 03 — CLI](../02%20components/framework/03-cli.md) | framework-side `cln` workflows (command surface home: Manager, `MGR-`) |
| `DRV-` | [02 components / framework 04 — Database Driver ABI](../02%20components/framework/04-database-libraries.md) | driver-side C-ABI vtable, result codes, migration state |
| `LBS-` | [02 components / framework 09 — Libraries Specification](../02%20components/framework/09-libraries-specification.md) | the library system: language additions, manifests, host bridge declarations, governance |
| `MCS-` | [02 components / framework 10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md) | framework MCP server surface and shared state |
| `AUTH-` | [02 components / libraries 01 — Auth](../02%20components/framework/libraries/01-auth.md) | auth library |
| `CNV-` | [02 components / libraries 02 — Canvas](../02%20components/framework/libraries/02-canvas.md) | canvas library |
| `CLNT-` | [02 components / libraries 03 — Client](../02%20components/framework/libraries/03-client.md) | client library |
| `DATA-` | [02 components / libraries 04 — Data](../02%20components/framework/libraries/04-data.md) | data library (two-object persistence model) |
| `JOBS-` | [02 components / libraries 05 — Jobs](../02%20components/framework/libraries/05-jobs.md) | jobs library |
| `LOC-` | [02 components / libraries 06 — Locale](../02%20components/framework/libraries/06-locale.md) | locale library |
| `MCP-` | [02 components / libraries 07 — MCP](../02%20components/framework/libraries/07-mcp.md) | mcp library (tools/resources/prompts blocks) |
| `SRV-` | [02 components / libraries 08 — Server](../02%20components/framework/libraries/08-server.md) | server library (endpoints, SSR, SSE/WS) |
| `STOR-` | [02 components / libraries 09 — Storage](../02%20components/framework/libraries/09-storage.md) | storage library |
| `UI-` | [02 components / libraries 10 — UI](../02%20components/framework/libraries/10-ui.md) | ui library |
| `AGENT-` | [02 components / libraries 11 — Agent](../02%20components/framework/libraries/11-agent.md) | agent library (declarative agents, tool dispatch, streaming, multi-agent) |
| `SRVH-` | [02 components / hosts — clean-server](../02%20components/hosts/clean-server/01-server.md) | the reference HTTP host |
| `CLNH-` | [02 components / hosts — clean-host-core](../02%20components/hosts/clean-host-core/01-specification.md) | shared host library: composition, bridge lifecycle, instance pooling, WASI stack, config parsing, capability manifest, reload machinery |
| `BRWH-` | [02 components / hosts — clean-browser](../02%20components/hosts/clean-browser/01-specification.md) | browser host: DOM/JS runtime for compiled `.wasm` components in the browser |
| `CLIH-` | [02 components / hosts — clean-cli](../02%20components/hosts/clean-cli/01-specification.md) | CLI host: terminal / stdio runtime for one-shot `.wasm` execution |
| `EDGH-` | [02 components / hosts — clean-edge](../02%20components/hosts/clean-edge/01-specification.md) | edge host: serverless / edge-worker runtime with per-request isolation |
| `WKRH-` | [02 components / hosts — clean-worker](../02%20components/hosts/clean-worker/01-specification.md) | background-worker host: long-running job execution outside an HTTP surface |
| `SESB-` | [02 components / bridges — session-bridge](../02%20components/bridges/session-bridge/01-specification.md) | session-bridge: `clean:session/store` + `http-envelope` |
| `DATB-` | [02 components / bridges — data-bridge](../02%20components/bridges/data-bridge/01-specification.md) | data-bridge: `clean:data/store`, `txn`, `types` |
| `JOBB-` | [02 components / bridges — jobs-bridge](../02%20components/bridges/jobs-bridge/01-specification.md) | jobs-bridge: `clean:jobs/queue` + `worker` |
| `KVB-` | [02 components / bridges — kv-bridge](../02%20components/bridges/kv-bridge/01-specification.md) | kv-bridge: `clean:kv/store`, `counter`, `cas` |
| `MAIB-` | [02 components / bridges — mail-bridge](../02%20components/bridges/mail-bridge/01-specification.md) | mail-bridge: `clean:mail/send` + `templates` |
| `RTB-` | [02 components / bridges — realtime-bridge](../02%20components/bridges/realtime-bridge/01-specification.md) | realtime-bridge: `clean:realtime/rooms`, `publish`, `subscribe`, `sockets` |
| `AUTB-` | [02 components / bridges — auth-bridge](../02%20components/bridges/auth-bridge/01-specification.md) | auth-bridge: identity, credentials, token issuance/verification |
| `I18B-` | [02 components / bridges — i18n-bridge](../02%20components/bridges/i18n-bridge/01-specification.md) | i18n-bridge: locale resolution, message catalogs, formatting |
| `MCPB-` | [02 components / bridges — mcp-bridge](../02%20components/bridges/mcp-bridge/01-specification.md) | mcp-bridge: MCP client/server transport and dispatch |
| `ROLB-` | [02 components / bridges — roles-bridge](../02%20components/bridges/roles-bridge/01-specification.md) | roles-bridge: role/permission storage and evaluation |
| `COMP-` | [03 platform / 18 — Component Composition](../03%20platform/18-component-composition.md) | component composition rules: two composition modes, contract satisfaction, load-time verification |
| `MGR-` | [02 components / manager 00 — Clean Manager](../02%20components/manager/00-manager.md) | `cln` binary: command surface, on-disk layout, dispatch, resolution |
| `CCMP-` | [02 components / compiler 01 — Compiler Specification](../02%20components/compiler/01-specification.md) | the compiler component: what it owns, what it refuses, how it sits beside framework, manager, and the hosts (API contract home: Platform 14, `CMP-`) |
| `LAY-` | [03 platform / 01 — Execution Layers](../03%20platform/01-execution-layers.md) | the layer model, boundaries, dead-import elision |
| `BRG-` | [03 platform / 02 — Host Bridge](../03%20platform/02-host-bridge.md) | bridge surface: WASI composition, L2 catalog, guarantees |
| `MMD-` | [03 platform / 03 — Memory Model](../03%20platform/03-memory-model.md) | linear-memory layout, allocator, representations |
| `LSP-` | [03 platform / 04 — IDE & LSP Architecture](../03%20platform/04-ide-lsp-architecture.md) | language-server contract, thin-client extensions |
| `TIER-` | [03 platform / 05 — Memory Policy](../03%20platform/05-memory-policy.md) | memory tiers, growth, reset policies |
| `REP-` | [03 platform / 06 — Error Reporting](../03%20platform/06-error-reporting.md) | report schema, consent, feedback loop |
| `CONF-` | [03 platform / 07 — Build Configuration](../03%20platform/07-build-config.md) | `clean.toml` schema and build determinism |
| `BVER-` | [03 platform / 08 — Bridge Versioning](../03%20platform/08-bridge-versioning.md) | WIT package SemVer, link/run checks |
| `ERC-` | [03 platform / 09 — Error Codes](../03%20platform/09-error-codes.md) | registry process rules (1:1, ranges, additions) |
| `RUL-` | [03 platform / 10 — Semantic Rules](../03%20platform/10-semantic-rules.md) | meta-rules of the rule catalog (entry format, stubs) |
| `VAL-` | [03 platform / 11 — Stdlib Validator](../03%20platform/11-stdlib-validator.md) | the `validator` namespace |
| `SVX-` | [03 platform / 12 — Server Extensions](../03%20platform/12-server-extensions.md) | server-only host interfaces and the `server` world WIT |
| `DIA-` | [03 platform / 13 — Diagnostic Format](../03%20platform/13-diagnostic-format.md) | diagnostic shape, rendering, JSON, style |
| `CMP-` | [03 platform / 14 — Compiler Architecture](../03%20platform/14-compiler-architecture.md) | compiler API contract: request document, outputs, determinism |
| `CMOD-` | [03 platform / 15 — Component Model Architecture](../03%20platform/15-component-model-architecture.md) | WIT vocabulary (§0.3), worlds, versioning architecture |
| `HCV-` | [03 platform / 16 — Host Contract Validation](../03%20platform/16-host-contract-validation.md) | host WIT, the three check Moments |
| `TXT-` | [03 platform / 17 — Text Files](../03%20platform/17-text-encoding.md) | the UTF-8 invariant for ecosystem files, read validation, write emission |

Every prefix above is in use: each owning document declares it with a `**Rule prefix:**` line and mints IDs under it. `STY-` (style rules, if promoted out of dev-guidelines), `PKG-` (packaging targets, if 08-platforms is promoted), `HTTPX-` (HTTP-shaped host interfaces, if 12-server-extensions.md is later split), and `DBGC-` (dev-mode capture, if extracted from 12-server-extensions.md §11) are anticipated but deliberately unclaimed — claim them here before first use.

## Policy, mechanics, and foundational technology

Governance documents in this folder are split by rung of the ladder ([DOC-07](00-documentation-principles.md)):

- **Principles docs (LANG, SEC, PERF, INTEROP)** state durable policy — *what MUST be true and why*. They do not name specific keywords, flags, file paths, benchmark numbers, or technology products. A principle that leaks a mechanism is a defect; the leak belongs in a spec chapter or an ADR.
- **Spec chapters** (`02 components/`, `03 platform/`, `04 language/`) state mechanics — *what is observably true in the implementation today*. Keyword names, syntax tokens, flag names, file paths, numeric budgets, and every other concrete detail live here.
- **ADRs** (`decisions/`) record design decisions — *why the specific mechanism was chosen, when, and what alternatives were rejected*. Foundational technology choices (compilation target, boundary description language, composition model, versioning scheme, toolchain command surface, on-disk artifact layout, reproducibility mechanism) are consolidated in [ADR-0022](decisions/0022-foundational-technology-stack.md); one ADR per subsequent decision, append-only.

When principle and spec disagree, the spec wins on mechanics and the principle wins on intent — reconcile by opening an ADR, never by silently changing either.

Note on shape: a rule ID is `PREFIX-NN` and always carries a hyphen. A diagnostic code is `PREFIX###` and never does. `CLS-` (the classes chapter) and `CLASS###` (the class diagnostic range) are therefore distinct namespaces, as are `STD-`/`STATE###`, `BLK-`/`BLOCK###` and `FNC-`/`FUNC###`.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Anyone reading or writing governance-level documentation
- **References:** [Documentation Principles](00-documentation-principles.md) — DOC-07 (ladder), DOC-11 (precedence), DOC-13 (rule prefixes)
