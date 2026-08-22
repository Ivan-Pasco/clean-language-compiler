# Architecture Boundaries — Clean Language Ecosystem

The Clean Language ecosystem is built as several independent components — the compiler, the framework, the manager, hosts, bridges, libraries — and each component owns a specific piece of the whole. This document is the authoritative list of who owns what. It exists to prevent responsibility drift: the slow slide where one component gradually absorbs work that belongs somewhere else, and the code that was supposed to be replaceable becomes load-bearing. Every component's `CLAUDE.md` references this file, and every session working in a component checks it before implementing new functionality.

Libraries are ordinary Clean packages, not a separate component. Compile-time functions defined by libraries run inside the compiler process; library-authoring discipline lives in the [Libraries Specification](../02%20components/framework/09-libraries-specification.md), not here.

## 1. Purpose

This document exists to prevent responsibility drift — where one component gradually absorbs logic that belongs in another. It is the enforcement mechanism that keeps each component focused on its declared responsibility.

## 2. Component Boundary Definitions

### 2.1 Compiler (`clean-compiler/`)

The compiler's contract with the rest of the toolchain — what it accepts, what it emits, and the rules it upholds — is owned by [Platform 14](../03%20platform/14-compiler-architecture.md), with the component-level view in [the compiler component specification](../02%20components/compiler/01-specification.md). This section states only the boundary.

**IS responsible for:**
- Parsing `.cln` source files into AST (parser strategy: reference choice recorded in [ADR-0006](decisions/0006-compiler-reference-stack.md))
- Executing `compiletime` functions defined in libraries against the typed AST/IR (in the sandboxed compile-time runtime — [ADR-0004](decisions/0004-block-handler-execution-model.md))
- Semantic analysis (type checking, scope resolution, inference)
- Typechecking `host function` declarations against WASM import signatures
- Emitting a **Component Model component** whose imports are interface-qualified against the target world, with that world's WIT attached (CMP-01, CMOD-01, CMOD-03)
- Validating every `host function` call site against the target world it was handed in the compilation request, failing the build when a call site is not in that world (CMP-03)
- Error recovery and diagnostic messages

**is NOT responsible for:**
- Implementing any I/O (console, file, HTTP, DB)
- Running WASM modules
- **Fetching, downloading, or resolving any host contract.** The compiler validates against the world delivered to it; obtaining that world is the framework's job, and verifying the concrete host at load time is the host's (HCV-04)
- Knowing about project folder structure (that's Clean Manager / Clean Framework — see [Framework CLI](../02%20components/framework/03-cli.md))
- HTTP routing or server behavior
- Domain-specific block interpretation (that's the library's compile-time function, which the compiler *runs* but does not *contain*)

**Boundary test:** If a function needs to access the filesystem, network, or any external resource at runtime → it does NOT belong in the compiler. If it needs to reach outside the compilation request to learn what the target world contains → it does not belong in the compiler either.

Compile-time functions defined by libraries run inside the compiler process, but they are ordinary Clean code the compiler executes — not code that lives in the compiler's own source. The compiler must not contain domain-specific logic for `data:`, `endpoints:`, `component:`, `canvas:`, or any other block. That logic lives in library source.

---

### 2.2 Libraries (`clean-framework/libraries/*`)

Libraries are Clean packages, not a separate component. This section describes the **coding standards for library authors**, not a component boundary.

**Library source IS responsible for:**
- Defining DSL blocks (`endpoints:`, `data`, `auth:`, `component:`, `canvas:`) via `handles block` declarations
- Interpreting DSL blocks into typed IR through `compiletime` functions
- Declaring `host function` signatures for calls the runtime provides
- Declaring folder conventions in `library.toml` for auto-scoping
- Validating DSL block syntax and producing typed diagnostics

**Library source is NOT responsible for:**
- Implementing host functions (that's the server / runtime)
- Parsing non-Clean files (HTML, CSS, SQL) unless that IS the library's purpose
- Running at runtime (compile-time functions execute during compilation only)
- Project scaffolding or file generation
- Direct filesystem operations at compile time

**Boundary test:** If a function runs after compilation is complete → it does NOT belong in library compile-time code. It belongs in host functions the library declares.

---

### 2.3 Clean Manager (`clean-manager/`, ships the `cln` binary)

Clean Manager owns the `cln` command — the single user-facing binary. It parses every `cln <verb>` invocation and dispatches to the appropriate component (Clean Framework, the compiler, database tooling, MCP entry point).

**IS responsible for:**
- Owning the `cln` command surface, argv parsing, and help
- Installing, switching, and listing compiler and framework versions
- Downloading and managing binaries (`cln` itself, `clean-compiler`, `clean-framework`, host runtimes)
- Shell PATH configuration and one-time developer setup
- Dependency resolution (reads `[dependencies]` in `clean.toml`, both libraries and plugins)
- On-disk layout under `~/.cln/` and per-project `.cln/`
- MCP client registration (`cln mcp install`)
- Environment diagnostics (`cln doctor`)
- Dispatching build/run/check/db/migrate verbs to Clean Framework
- Dispatching compilation to the compiler (via Clean Framework)

**is NOT responsible for:**
- Parsing or understanding Clean Language syntax
- Code generation of any kind
- Discovering or scanning project files for routes, components, models (that's Clean Framework)
- Template expansion or HTML transformation
- Running compile-time block handlers (that's Clean Framework)
- HTTP routing, server behavior, or runtime concerns

**Boundary test:** If a function reads, parses, transforms, or generates `.cln` source code → it does NOT belong in Clean Manager.

---

### 2.4 Clean Framework (`clean-framework/`, invoked by `cln`)

Clean Framework is the build orchestrator. It never has its own user-facing CLI — every framework operation is reached through `cln <verb>`, which Clean Manager routes to the framework binary.

**IS responsible for:**
- Reading and validating `clean.toml`; resolving library dependencies and plugins to file paths
- Project scaffolding (`cln new <name> --libraries=…`)
- Library authoring scaffolds (`cln library create`)
- Compiling and caching library block handlers; assembling the compilation request document handed to the compiler ([ADR-0004](decisions/0004-block-handler-execution-model.md))
- Bundling and wiring frontend/backend artifacts after compilation
- Running database migrations (`cln db migrate ...`)
- Development server orchestration when the project targets `server` (`cln dev`)
- Serving the Clean MCP endpoint (`cln mcp`)
- Knowing folder conventions (reads each library's `library.toml` for suggested folders)

**is NOT responsible for:**
- Version management or binary downloads (that's Clean Manager)
- Runtime host function implementations (that's the server or other host runtime)
- WASM code generation (that's the compiler)

---

### 2.5 IDE Extension (`clean-extension/`)

**IS responsible for:**
- Starting and communicating with the language server via LSP (thin client)
- Detecting whether `cln` (Clean Manager) is installed
- Guiding users through installation and setup when compiler is not available
- Providing UI commands (run, compile, build)
- Providing a **minimal** TextMate grammar for basic lexical tokens only (strings, comments, numbers, operators)
- Distributing via VS Code Marketplace and Open VSX Registry

**is NOT responsible for:**
- Hardcoding language keywords, types, framework blocks, or function names
- Loading or parsing `library.toml` files (that's the language server)
- Providing completions, hover, or diagnostics (that's the language server)
- Syntax highlighting of language-specific tokens (that's semantic tokens from the language server)
- Maintaining keyword lists that need updating with each framework or language release

**Boundary test:** If you are adding a keyword, type, function name, or framework block to the extension → STOP. It belongs in the language server. See [IDE Extension Architecture](../03%20platform/04-ide-lsp-architecture.md).

**Architectural rule:** The language server is the **single source of truth** for all language intelligence. The extension is a thin client. No fallback grammar needed — if the compiler isn't installed, the user can't code anyway. Guide them to install it instead.

---

### 2.6 Server (`clean-server/`)

**IS responsible for:**
- Loading and executing `.wasm` components (engine: reference choice in [ADR-0002](decisions/0002-clean-server-reference-stack.md))
- HTTP serving for incoming requests (stack: [ADR-0002](decisions/0002-clean-server-reference-stack.md))
- Route matching and handler dispatch
- Host function implementations (all `_*` functions declared by libraries)
- Database connections and query execution
- Authentication runtime (session storage, JWT validation)
- Serving static files

**is NOT responsible for:**
- Compilation or code generation
- Parsing Clean Language syntax
- Project scaffolding or file discovery
- Library management
- Version management

**Boundary test:** If a function runs before the `.wasm` file exists → it does NOT belong in the server.

---

### 2.7 Website / Frame Applications (`Web Site Clean/`, etc.)

**IS responsible for:**
- Following the correct Frame project structure exactly
- Using correct file extensions (`.html` for pages, `.cln` for logic, `.css` for styles)
- Placing files in the correct folders for auto-detection
- Separating concerns: pages = templates, api = logic, data = models

**MUST follow these rules:**
- Pages in `app/pages/` with `.html` extension only
- Components in `app/components/` with `.cln` extension
- API endpoints in `app/backend/api/` with `.cln` extension
- Data models in `app/data/` with `.cln` extension
- Styles in `public/css/` with `.css` extension only
- No explicit `libraries:` blocks in files where the library is inferred from the folder scope declared in `clean.toml` (auto-detected)
- No `<script>` tags (no JavaScript)
- No inline `<style>` tags (all CSS in external files)
- No `<script type="text/clean">` blocks in pages
- No business logic in page templates
- No duplicate config files

---

### 2.8 Error-Reporting Backend (`clean-errors/`)

The error-reporting backend receives structured error reports from the toolchain and hosts, and drives the maintainer-side half of the feedback loop. Its internal design is recorded in [ADR-0008](decisions/0008-error-reporting-backend.md); the observable reporting contracts live in [Platform 06](../03%20platform/06-error-reporting.md).

**IS responsible for:**
- Error-report ingestion (the report API endpoints and deduplication by fingerprint)
- The retest sandbox (reproducing reported bugs from uploaded capture bundles)
- The fix-notification pipeline (tracking lifecycle stages and notifying affected users)
- The maintainer dashboard (triage queue and report visibility)

**is NOT responsible for:**
- Emitting diagnostics (that's the compiler and the hosts)
- Defining diagnostic codes (that's [Platform 09](../03%20platform/09-error-codes.md))
- Report consent policy (that's [Platform 06](../03%20platform/06-error-reporting.md))

**Boundary test:** If a function runs on the developer's machine, or decides what a report may contain → it does NOT belong in clean-errors. clean-errors begins where a consented report arrives at the backend.

---

## 3. Boundary Violation Detection Checklist

Before implementing ANY new function or file, ask:

| Question | If YES → |
|----------|----------|
| Does this function parse `.cln` syntax? | Only belongs in the compiler |
| Does this function generate `.cln` code from a DSL block? | Only belongs in a library's compile-time function |
| Does this function know about `pages/`, `api/`, `data/` folders? | Only belongs in Clean Framework (see [Framework CLI](../02%20components/framework/03-cli.md)) |
| Does this function download or install binaries? | Only belongs in manager |
| Does this function implement a `_*` host function? | Only belongs in the server (or the host that provides that host function) |
| Does this function run at runtime after compilation? | Only belongs in the server |
| Does this function interpret DSL blocks? | Only belongs in a library's compile-time function |
| Does this function manage version switching? | Only belongs in manager |
| Does this add a keyword/type/function to the IDE extension? | Only belongs in language server (in compiler) |
| Does this load `library.toml` for IDE features? | Only belongs in language server (in compiler) |
| Does this function know about HTML tags, attributes, or template syntax? | Only belongs in the `ui` library |
| Does this function replicate logic that already exists in a library? | **STOP — fix the library, or the compiler bug that breaks the library's compile-time function** |

## 4. The Workaround Trap (CRITICAL)

**When a library's compile-time function produces incorrect output, the fix is NEVER to reimplement the library's logic in the compiler.** This is the most common form of boundary violation.

The correct response when library output is broken:

1. **Identify WHY the library output is wrong** — is it a library source bug or a compiler bug in how compile-time functions execute?
2. **If compiler bug:** Fix the compiler so compile-time functions execute correctly. Then re-run the library's compile-time function.
3. **If library source bug:** Fix the library, or report via cross-component prompt. Do not duplicate the logic in the compiler.
4. **NEVER copy library logic into the compiler** as a "workaround" — even temporarily. Workarounds become permanent, create maintenance burden, and violate the layering that makes libraries maintainable independently of the compiler.

If a library's compile-time function is broken, fix the library (or the compiler bug that runs it wrong). Never shadow-implement its logic in Rust inside the compiler. The boundary between compile-time and runtime is a language boundary; keep it clean.

## 5. What To Do When You Discover a Boundary Violation

1. **Do NOT fix it in the wrong component** — even if it's "quick"
2. Create a cross-component prompt in `foundation/archive/cross-component-prompts/`
3. Document exactly what code is misplaced and where it should go
4. Continue working within your component's boundaries

## 6. Violation Log

When a boundary violation is found and documented, add it to a table here for tracking (date, component, violation, status).

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Any session — human or AI — implementing functionality inside any Clean Language component
- **References:** [Documentation Principles](00-documentation-principles.md), [Compiler Specification](../02%20components/compiler/01-specification.md), [Libraries Specification](../02%20components/framework/09-libraries-specification.md), the ADRs in [decisions/](decisions/) covering per-component reference stacks
