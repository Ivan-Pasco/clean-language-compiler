# Platform 16. Host Contract Validation

This chapter defines the mechanism that catches mismatches between what a Clean program needs and what a host provides — before the mismatch reaches production. Every host publishes a `host.wit` describing exactly which interfaces and versions it implements; every project declares its target host in `clean.toml`; a Clean program is validated against that host's WIT at three moments (`cln build`, `cln check <host>`, and instantiation), each producing a structured diagnostic with a stable code that names which side is off-spec. Without this contract, three failure modes silently reach production: missing interfaces, version-drifted signatures, and deployment against a host different from the one the developer targeted. With it, all three fail loudly and early.

---

## 16.1 Motivation


Without a structured awareness of the deployment target, three classes of host/component mismatch can silently reach production:

| Failure mode | Severity |
|---|---|
| **A. Missing capability** — host doesn't implement an interface the `.wasm` imports | Loud, safe (surfaces at load time) |
| **B. Version skew** — host implements an older/newer version of an interface than the `.wasm` was built against | Medium (signature mismatch trips at load; silent skew leaks to runtime) |
| **C. Semantic drift** — host implements the signature but behavior differs (isolation level, encoding, retry semantics) | Silent, worst (only surfaces under load) |

This section handles **A and B** cleanly by making host capabilities a first-class, machine-readable contract. **C — semantic drift — is out of scope here** and is addressed separately through host conformance testing ([15 §10](./15-component-model-architecture.md#10-testing-and-conformance)) and future semantic contract validation work.

---

## 16.2 Core Idea — Two WIT Documents


Two WIT documents, each with a clear owner and a clear moment they matter.

| WIT document | Owned by | Says | Lives in |
|--------------|----------|------|----------|
| **Project WIT** | The compiled `.wasm` component itself | "This is what I need from a host" | Embedded in the `.wasm` (component-model self-description); framework may emit a human-readable copy for tooling |
| **Host WIT** | The host implementation | "This is what I provide" | A file checked into the host's own repo (e.g. `clean-server/host.wit`) |

**Both are just WIT.** No new syntax, no parallel format. The component model already uses WIT for exactly this purpose — this section formalizes *when* Clean tooling reads it. The vocabulary of package families, worlds, and interface names is fixed by [15 §0.3](./15-component-model-architecture.md#03-wit-package-and-world-naming); versions follow the baseline in [08 §8.0](./08-bridge-versioning.md#80-v2-baseline-versions).

---

## 16.3 The Two Flows


Two independent flows use the same pair of documents:

### Flow 1 — Host emits WIT before the application exists

```
   Host author writes host.wit
              │
              ▼
   ┌─────────────────────┐
   │  clean-server/      │
   │    host.wit         │───────► published in repo
   └─────────────────────┘
              │
              │  developer / framework fetches it
              ▼
   ┌─────────────────────┐
   │   Framework         │
   │   knows host        │
   │   capabilities      │───────► typecheck, autocomplete,
   │   (WITHOUT running  │         "can I use this feature?"
   │    the host)        │
   └─────────────────────┘
```

A host is a viable target the moment its WIT is public — even before any runtime code exists. This mirrors how WASI evolves.

### Flow 2 — Host validates the application at install time

```
   Developer ships     .wasm ─────────────┐
                                          │
                                          ▼
                                ┌───────────────────┐
                                │   Host receives   │
                                │   the component   │
                                └────────┬──────────┘
                                         │
                                         │ compares
                                         ▼
                          ┌──────────────────────────────┐
                          │  Project WIT (from .wasm)    │
                          │              vs              │
                          │  Host WIT   (from host.wit)  │
                          └──────────┬───────────────────┘
                                     │
                        ┌────────────┴────────────┐
                        │                         │
                        ▼                         ▼
                    complies                  does NOT comply
                        │                         │
                        ▼                         ▼
                  instantiate               refuse to load
                                            structured error
                                            (not a WASM trap)
```

The host is the last line of defense. Even if the build passed, the host still verifies before granting the component any capability.

---

## 16.4 The Three Check Moments


### HCV-01 — Three check Moments, each with its actor and its code


Compliance (§16.6) is checked at three points, each catching a different class of mistake:

1. **Moment 1 — `cln build`.** The **framework** MUST fetch the target's `host.wit` per `clean.toml` and validate the project against it before the compiler is invoked. Failure is `COM014` (WorldMismatch); the compiler is never invoked.
2. **Moment 2 — `cln check <host>`.** An optional pre-deploy step: the developer MAY fetch the *live* host's WIT (URL or path) and re-validate the built `.wasm` against it. Failure is `COM015` (VersionMismatch).
3. **Moment 3 — `host.load(.wasm)`.** The **host** MUST read the project WIT embedded in the component, compare it to its own WIT, and refuse to instantiate on any non-compliance, with the structured error [`COM017` `InstantiationFailure`](./09-error-codes.md) (never a bare WASM trap).

Check: a project targeting a host that lacks an imported interface fails at Moment 1 with `COM014`; a component deployed to a host on an incompatible interface version is refused at Moment 3 with `COM017`.

```
   ┌───────────────────────────────────────────────────────────────┐
   │                                                                │
   │   Moment 1              Moment 2              Moment 3         │
   │   ─────────             ─────────             ─────────        │
   │   cln build             cln check <host>      host.load(wasm)  │
   │   (framework)           (pre-deploy)          (runtime)        │
   │                                                                │
   │   Fetches host.wit      Fetches live host     Reads component  │
   │   per clean.toml        WIT (URL or path)     imports          │
   │   target.                                                      │
   │                         Validates the built   Compares to      │
   │   Validates before      .wasm against it.     host's own WIT.  │
   │   the compiler even                                            │
   │   runs.                 Answers: "am I safe   Refuses to       │
   │                         to deploy?"           instantiate on   │
   │   Catches:                                    mismatch.        │
   │   misconfigured         Catches:                               │
   │   projects              deployment surprises  Catches:         │
   │                         (host on a differ-    host upgrades    │
   │                         ent version than the  between check    │
   │                         one you designed      and deploy       │
   │                         against)                               │
   │                                                                │
   └───────────────────────────────────────────────────────────────┘
```

Three checks may look like overkill, but each catches a mistake the others can't. Build-time catches "I designed against the wrong host." Pre-deploy catches "the live host isn't what I thought." Load-time catches "the host changed under me."

---

## 16.5 Where Host WIT Lives


**Decision: a file in the host's repo.**

### HCV-02 — `host.wit` is the host's declaration artifact


Every host MUST have exactly one `host.wit` file. This is the authoritative declaration of what the host provides — there is no parallel declaration format ([08 §8.4](./08-bridge-versioning.md): the manager consumes this same fetched file). Its location is fixed by the shape of the repository, and there are exactly two forms:

- **A repository that *is* one host** declares it at **`host.wit` in the repository root** (e.g. `clean-server/host.wit`).
- **A repository that *contains* hosts among other components** declares each at **`hosts/<host-name>/host.wit`**, where `<host-name>` is the host's canonical name ([glossary](../01%20governance/06-glossary.md)) — e.g. `clean-manager/hosts/clean-cli/host.wit`, `clean-framework/hosts/browser/host.wit`.

Which form applies is a property of the repository, not a fallback chain: a host has one declaration at one path, and the framework fetches it from exactly there. A repository MUST NOT declare the same host in both forms, and a `wit_source` URL MUST resolve to that host's single declaration at the pinned version. This is the property [BVER-03](./08-bridge-versioning.md#84-host-declaration) depends on — one host, one hashable file — and the second form does not weaken it. Signature verification for network-fetched WIT follows §16.11 rule 2. Check: for every registered host, the fetched `host.wit` parses as WIT, resolves to exactly one path, and the lockfile records its hash and verification status.

Rationale:
- Version-controlled together with the code that implements it — no drift possible.
- Simple, no infrastructure required.
- Framework fetches it via ordinary git or a plain URL to the raw file.
- Works offline once cloned/cached.

The framework caches fetched host WIT locally per version. `clean.toml` declares the target host and version:

```toml
[target]
host   = "clean-server"
version = "0.1.x"
wit_source = "https://github.com/clean-language/clean-server/blob/v0.1.0/host.wit"
```

The `wit_source` field is optional; when omitted, the framework resolves it from a known registry of official Clean hosts (`clean-server`, `clean-browser`, `clean-cli`). Third-party hosts must provide the URL explicitly.

---

## 16.6 Compliance — What It Means


### HCV-03 — Compliance is presence + version + signature identity, with no optional imports


A project WIT "complies with" a host WIT when **all** of the following hold:

| Check | Rule | Fails when |
|-------|------|-----------|
| **Interface presence** | Every import the project WIT declares appears in the host WIT | Host is missing an interface the project imports |
| **Version compatibility** | For each shared interface, the host's version satisfies the project's declared version range (semver) | Host is on a version outside the project's accepted range |
| **Signature identity** | For each function in a shared interface at a compatible version, the two WIT signatures are identical if and only if the **canonical printed forms of their parsed WIT are byte-identical** (canonical printing per the reference WIT tooling) — identity is a property of the parsed AST, not of the source text, so comments and whitespace never count | Host implements the same interface at the same version but with a different signature (indicates a versioning bug on one side) |

There are no optional imports: the world defines the contract, and a component's imports are exactly what its target world declares it may use ([15 §0.3](./15-component-model-architecture.md#03-wit-package-and-world-naming)). A program that wants a capability only some deployments provide targets a world that includes it, and the deployment's runtime status is inspected through the manifest mechanism in §16.12 — not through the WIT check.

Compliance is **one-way by default**: host may declare capabilities the project doesn't use — those are simply unused. `cln check --strict` additionally warns about unused host capabilities so the developer notices opportunities.

---

## 16.7 The Full Picture


```
┌────────────────────────────────────────────────────────────────────┐
│                         HOST AUTHOR                                 │
│                                                                     │
│   writes host.wit          ────────►    checks into repo            │
│                                                                     │
└────────────────────────────┬───────────────────────────────────────┘
                             │
                             │  publicly fetchable
                             ▼
                  ┌─────────────────────┐
                  │   host.wit          │◄──────────────────────┐
                  │   (file in repo)    │                       │
                  └──────────┬──────────┘                       │
                             │                                  │
                             │                                  │ fetched
                             │                                  │ at load
                             │ fetched at build                 │ time to
                             │ and pre-deploy                   │ self-check
                             ▼                                  │
┌────────────────────────────────────────────────────────────────────┐
│                        DEVELOPER MACHINE                            │
│                                                                     │
│   clean.toml                                                        │
│     [target] host = "clean-server", version = "0.1.x"               │
│                                                                     │
│   ┌───────────────────────────────────────────────┐                 │
│   │             cln build  (Moment 1)              │                 │
│   │                                                │                 │
│   │   1. framework reads clean.toml                │                 │
│   │   2. fetches host.wit for declared target      │                 │
│   │   3. validates project against host WIT        │                 │
│   │   4. if compliant → invoke compiler            │                 │
│   │   5. compiler emits .wasm with embedded WIT    │                 │
│   │      (project WIT)                             │                 │
│   └────────────────────┬──────────────────────────┘                 │
│                        │                                            │
│                        │  .wasm produced                            │
│                        ▼                                            │
│   ┌───────────────────────────────────────────────┐                 │
│   │         cln check <host>  (Moment 2)          │                 │
│   │                                                │                 │
│   │   optional pre-deploy step. fetches the LIVE   │                 │
│   │   host's WIT (may be different from the one    │                 │
│   │   used at build time) and re-validates.        │                 │
│   └────────────────────┬──────────────────────────┘                 │
│                        │                                            │
└────────────────────────┼────────────────────────────────────────────┘
                         │
                         │ deploy
                         ▼
┌────────────────────────────────────────────────────────────────────┐
│                          HOST INSTANCE                              │
│                                                                     │
│   ┌───────────────────────────────────────────────┐                 │
│   │           host.load(.wasm)  (Moment 3)         │                 │
│   │                                                │                 │
│   │   1. reads project WIT embedded in component   │                 │
│   │   2. compares to host's own WIT                │                 │
│   │   3. compliant  → instantiate                  │                 │
│   │      not       → refuse with structured error  │                 │
│   └───────────────────────────────────────────────┘                 │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
```

The three moments form a defense-in-depth chain. A mismatch introduced at any point is caught before user code ever runs.

---

## 16.8 A Concrete Example


**Developer writes:**

```clean
endpoints:
	GET /users/:id
		return db.users.find(id)
```

**Their `clean.toml`:**

```toml
[target]
host = "clean-server"
version = "0.1.x"
```

### Case A — everything matches

```
   Project WIT (embedded in .wasm)
     import clean:host/routing@0.1.0;
     import clean:bridge/db@0.1.0;

                        │
                        ▼
                  ✓ compare with
                        │
                        ▼

   Host WIT (clean-server)
     clean:host/routing@0.1.0   ✓ provided
     clean:bridge/db@0.1.0      ✓ provided
     clean:host/session@0.1.0   ← extra, unused, fine
```

Result: **compliant** at all three moments. Deploys and runs.

### Case B — developer targets the wrong host

Developer sets `host = "browser"` by mistake.

```
   Project WIT
     import clean:host/routing@0.1.0;
                        │
                        ▼
                  ✗ compare with
                        │
                        ▼
   Host WIT (browser)
     clean:host/dom@0.1.0       ✓ provided
     clean:host/storage@0.1.0   ✓ provided

     (no `routing` interface)
```

Result: **Moment 1 (cln build) fails** (`COM014` WorldMismatch):

```
error[COM014]: target host `browser` does not provide required interface
       `clean:host/routing@0.1.0`.
       This interface is needed by your `endpoints:` block(s):
         app/endpoints/users.cln:2

hint: either switch target to `clean-server`, add a server build target
      alongside `browser`, or remove the `endpoints:` block.
```

The compiler is never invoked.

### Case C — the live host is on a different version

The build succeeded against the clean-server release that ships `clean:bridge/db@0.1.0`. Ops upgraded prod to a newer clean-server release that bumped `clean:bridge/db` to `@0.2.0` (a breaking change under the pre-1.0 rules of [08 — Bridge Versioning](./08-bridge-versioning.md)).

```
   Project WIT (embedded in .wasm)
     import clean:bridge/db@0.1.0;

                        │
                        ▼
                  ✗ compare with
                        │
                        ▼
   Host WIT (upgraded clean-server)
     clean:bridge/db@0.2.0
```

Result: **Moment 3 (host.load) fails** (`COM017`, the structured host error of HCV-01):

```
error[COM017]: cannot instantiate component
       component requires: clean:bridge/db@0.1.0
       host provides:      clean:bridge/db@0.2.0
       (semver-incompatible)

hint: rebuild your component against the upgraded clean-server, or
      downgrade the host to a release providing clean:bridge/db@0.1.0.
```

The developer would have caught this earlier with `cln check https://prod.example.com` (Moment 2, `COM015` VersionMismatch) before pushing the deploy.

---

## 16.9 Non-Goals


**Semantic drift (failure mode C) is explicitly out of scope.**

WIT captures signatures, not behavior. This mechanism cannot detect that:

- A host implements `db.query` with SQLite semantics when your code assumes Postgres.
- A host implements `now()` in local time when your code assumes UTC.
- A host implements `http.fetch` without retries when your code assumes automatic retry.

Semantic contract validation requires either (a) semantic annotations extending WIT, (b) a conformance test suite each host must pass, or (c) runtime contract tests. Host conformance testing ([15 §10](./15-component-model-architecture.md#10-testing-and-conformance)) covers option (b) today; options (a) and (c) are future work not yet specified.

---

## 16.10 Component Responsibilities


### HCV-04 — The scope-split: who validates what


Host-contract validation is split across three actors:

- **Framework** MUST fetch host WIT and run the Moment 1 and Moment 2 checks defined in this document (`COM014`, `COM015`). The compiler never fetches host WIT.
- **Compiler** MUST NOT download or validate the concrete host. It MUST validate the program against the target world: every import is checked against the world WIT delivered in the compilation request document, emitting `COM012` on mismatch ([CMP-03](./14-compiler-architecture.md#146-diagnostics-and-error-handling); [14 §14.4.2 pass 9](./14-compiler-architecture.md#1442-detailed-pass-responsibilities); [15 §6.1](./15-component-model-architecture.md#61-compile)). It also emits the component-model WIT (project WIT) into the `.wasm`.
- **Hosts** MUST ship a `host.wit` file in their repo (HCV-02) and implement the Moment 3 check in their loader, at instantiation (HCV-01).

Check: the compiler makes no network access during any compilation (CMP-01); the framework's Moment 1 run and the host's Moment 3 refusal are each observable independently of the other two actors.

---

## 16.11 Design Rules


1. **Offline `host.wit` fetch.** `host.wit` is cached at `~/.cln/host-wit/<host>@<version>.wit`. The first fetch requires network; subsequent builds work offline. This mirrors the package-registry caching model.

2. **`host.wit` signing.** For third-party hosts fetched over the network, an unsigned WIT is a trust problem — anyone could serve a WIT that lies about capabilities. Hosts publish `host.wit` alongside a signature, and the framework verifies the signature on fetch (signature scheme and format: specification pending). Unsigned WIT installs with a one-time warning and is recorded as unverified in the lockfile.

3. **WASI requirements.** `wasi:*` packages behave the same as any other WIT package under the Moment 1 check: the project imports them, the host declares them, and mismatches are reported like any other missing interface. No special-case handling.

4. **Multi-host targets.** `clean.toml` may declare multiple `[target]` sections. The framework runs Moment 1 for each target. The compiler produces one `.wasm` per target, or one component satisfying the intersection when feasible; resolution details are a [15](./15-component-model-architecture.md) concern.

5. **LSP participation.** The reference language server performs the Moment 1 check continuously as the developer types. Using `endpoints:` in a browser-targeted project surfaces a red squiggle immediately, not on `cln build`.

---

## 16.12 MCP Integration — AI Host-Awareness


The `host.wit` mechanism only closes the AI feedback loop if the MCP server exposes it. Without host awareness, an AI assistant does not know which host the project targets, what that host can do, or whether a feature it is about to suggest is even reachable from the deployment — so it happily suggests `endpoints:` in a browser-targeted project and the developer only discovers it at `cln build` time.

This section defines the *semantics* of the host-awareness checks the MCP serves. The MCP tool catalog itself — tool names, schemas, request/response shapes — is owned by [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md); this document does not restate it.

### 16.12.1 Five host-awareness checks

Five tools cover the five questions an AI needs to answer at any point in a session (catalog home: [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md)). What each check *means*:

| Tool | Answers | Semantics defined here |
|------|---------|------------------------|
| `get_target_host` | "What host does this project target?" | Reads the `[target]` declaration(s) from `clean.toml` (§16.5) and reports the resolved `wit_source` and cache status. |
| `get_host_capabilities` | "What interfaces and functions does that host provide?" | Returns the parsed host WIT (§16.5) — the same document the framework validates against at Moment 1. |
| `get_host_runtime_status` | "What is actually turned on in the target deployment right now?" | Reads the `host-capabilities.cln` manifest emitted by a running server ([hosts/01-server §1.8a](../02%20components/hosts/clean-server/01-server.md)); see below. |
| `check_host_compatibility` | "Is this snippet compatible with the target?" | Runs the Moment 1 check (§16.4, §16.6) over a snippet or the current project and reports per-import compatibility. |
| `suggest_host_features` | "What capabilities are available but unused?" | The inverse of the Moment 1 check: host WIT minus project imports, ranked by relevance — the same information `cln check --strict` surfaces (§16.6). |

**Runtime status semantics.** Unlike `get_host_capabilities` (which answers "what does this host software support"), `get_host_runtime_status` answers "what is turned on in the deployment I'm building for." Its source is a local file path the developer configures per project in `clean.toml`:

```toml
[mcp.host_runtime]
manifest = "./deploy/prod-eu-1/host-capabilities.cln"
```

The path may point at a manifest copied from a running deployment, checked into the project, or generated locally by a dev-mode server. The MCP does not fetch over HTTP — deployment identity matters, and the developer must be explicit about which deployment they are targeting. If the path is unset or the file is missing, the tool reports the manifest as unavailable and the AI treats plugin-status questions as unknown (see §16.12.3).

The AI uses runtime status to distinguish three failure modes when a user asks for a feature: *host doesn't support it* (WIT gap — hard fix), *host supports but admin disabled it* (config fix — quotable reason), *host supports but no bridge composed* (deployment fix — different bridge component needed).

### 16.12.2 Shared cache with the framework

The MCP does not maintain its own copy of host WIT. It reads and writes the same cache the framework uses:

```
   MCP tool called
        │
        ▼
   read clean.toml → target host + version
        │
        ▼
   ┌─────────────────────────────────────────┐
   │  cache at ~/.cln/host-wit/            │
   │    <host>@<version>.wit                 │
   └────────┬────────────────────────────────┘
            │
            │  hit? use it. miss? fetch.
            ▼
   fetch from wit_source URL
   (github raw file, HTTPS, or registry)
        │
        ▼
   parse WIT → structured JSON → return
```

First `cln build` warms the cache; subsequent MCP calls are instant. Offline works after first fetch. Framework and MCP see the same host WIT — no possibility of drift between "what the AI thinks the host provides" and "what the framework checks against."

### 16.12.3 The AI-side rule


The MCP server instructions (delivered to every AI session that connects) add one rule alongside the existing "call `list_libraries` before using framework features":

> **Before generating any code that touches a host bridge (I/O, DB, HTTP, DOM, routing, files, crypto), call `get_target_host` and `get_host_capabilities`. If the requested feature is not in the returned WIT, do NOT write code that uses it. Tell the user their target host does not support it and offer alternatives: change target, use a different capability, or add a build target.**
>
> **Additionally, if `[mcp.host_runtime]` is configured in `clean.toml`, also call `get_host_runtime_status` and honor its status values. Do NOT write code that uses a plugin whose status is `disabled` or `unavailable` on the target deployment. Tell the user the plugin is off, quote the `reason` field back to them verbatim, and offer alternatives (ask the admin to enable it, use a different plugin, target a different deployment). If no manifest is configured, this additional check does not apply — the AI proceeds on WIT-only evidence.**

This is the AI-side equivalent of the framework's Moment 1 check (WIT gate) plus Moment 3 check (deployment gate). It closes the feedback loop from "runtime failure" back to "the AI never wrote the broken code in the first place."

### 16.12.4 Honesty principle — cite the WIT

AI training data is not authoritative about host capabilities. Hosts bump versions; interfaces are added and removed. A model trained six months ago may confidently claim "clean-server supports X" when the current version does not, or miss features that were added last week. Any AI guidance about host capabilities should be sourced from a WIT the MCP fetched *this session*, not from model memory — a conduct expectation, not a checkable rule. What *is* checkable is the attribution data the MCP must supply:

### HCV-05 — MCP host-awareness responses are attributable


Every response from a host-awareness MCP tool (§16.12.1) MUST include:

- The host name and version consulted.
- The WIT fetch timestamp (`fetched_at`).
- The cache hit/miss status.

Check: every §16.12.1 tool response parses with all three fields present. The AI is expected to reference them when explaining its suggestions ("per the clean-server host WIT fetched 2 minutes ago…"), so the developer can see whether guidance is fresh.

### 16.12.5 The full picture


```
┌────────────────────────────────────────────────────────────────┐
│                    AI CODING SESSION                            │
│                                                                 │
│   user: "add a GET /users endpoint"                             │
│                                                                 │
│   AI:                                                           │
│    1. call get_target_host                                      │
│         → { host: "clean-server", ... }                         │
│    2. call get_host_capabilities                                │
│         → sees clean:host/routing IS provided                   │
│    3. write endpoints: block                                    │
│    4. call check_host_compatibility on the written code         │
│         → compatible                                            │
│    5. present code to user, citing WIT source                   │
│                                                                 │
└──────────┬─────────────────────────────────────────────────────┘
           │
           │  tool calls
           ▼
┌────────────────────────────────────────────────────────────────┐
│                        MCP SERVER                               │
│                                                                 │
│   host-awareness tools (catalog: 10-mcp):                       │
│     get_target_host                                             │
│     get_host_capabilities                                       │
│     get_host_runtime_status                                     │
│     check_host_compatibility                                    │
│     suggest_host_features                                       │
│                                                                 │
│   reads: clean.toml, cached host.wit                            │
│   fetches: host.wit from wit_source URL                         │
│   caches: ~/.cln/host-wit/<host>@<version>.wit                │
│                                                                 │
└──────────┬─────────────────────────────────────────────────────┘
           │
           │  same cache as
           ▼
┌────────────────────────────────────────────────────────────────┐
│                       FRAMEWORK                                 │
│    (uses the same cached host.wit at Moment 1 checks)           │
└──────────┬─────────────────────────────────────────────────────┘
           │
           │  same cache as
           ▼
┌────────────────────────────────────────────────────────────────┐
│                    HOST'S REPO                                  │
│   host.wit  ← authoritative source; everything above derives    │
│              from here                                          │
└────────────────────────────────────────────────────────────────┘
```

The AI, the MCP, the framework, and the host all read the same source of truth. No parallel documentation, no version skew, no "training data says otherwise" failure mode.

---

## 16.13 Summary


- Two WIT documents: **project WIT** (in the `.wasm`) and **host WIT** (in the host's repo).
- Three check moments: **build**, **pre-deploy**, **load**.
- Compliance = interface presence + version compatibility + signature identity.
- Hosts publish `host.wit` as a file in their repo — no infrastructure required.
- Solves failure modes A (missing capability) and B (version skew) end-to-end.
- Explicit non-goal: semantic contract drift (failure mode C) — covered by host conformance testing ([15 §10](./15-component-model-architecture.md#10-testing-and-conformance)), not by this mechanism.
- Every host repo mechanically enforces its own conformance in CI (§16.14).

---

## 16.14 Host-Side Self-Conformance Enforcement


The §16.4 three-check chain is only sound if every host actually (a) ships a `host.wit`, (b) ships one whose declarations match the interfaces its code registers with the wasmtime `Linker` (or the equivalent import table on non-Rust runtimes). Nothing in Moments 1–3 catches a host that skips (a) or lies at (b): if `host.wit` is absent the framework has nothing to compare against; if the host silently registers a no-op stub for an interface it doesn't really implement, WASI's load-time import check is defeated. Both failure modes have been observed in early V2 host implementations.

### HCV-06 — Every host repo MUST mechanically verify its own `host.wit` in CI


Every host repository MUST run two automated checks on every commit, both blocking merges on failure:

| Check | What it verifies | Fails when |
|-------|------------------|-----------|
| **Presence-and-parse** | `host.wit` exists at repo root (HCV-02) and parses as valid WIT under the reference WIT tooling. | The file is missing, misnamed, at a non-root path, or contains syntax errors. |
| **Registration parity** | For every interface and function declared in `host.wit`, the host's own code registers a real implementation in the runtime import table (wasmtime `Linker` for Rust hosts; the equivalent import wiring for JS, WASI-preview-3, or other runtimes). And, symmetrically, every registered import is declared in `host.wit`. | The `host.wit` promises `frame_canvas_path_begin` but the runtime never registers it; or the runtime registers a function `host.wit` does not declare; or the runtime registers a no-op / throwing stub for a declared function. |

These checks are **the host's own responsibility**, not the framework's — the framework validates *guests* against `host.wit`, not `host.wit` against the host implementation. Without HCV-06, `host.wit` is voluntary documentation instead of a mechanical contract, and the guarantees of HCV-01, HCV-02, and HCV-03 degrade to "trusted, not verified."

**Stub-import prohibition.** Registration parity explicitly forbids registering a no-op or throwing shim under a name declared in `host.wit`. The correct response to an unimplemented interface is to omit it from `host.wit` (so the framework catches it at Moment 1) or to not register it in the Linker (so WASI catches it at load). Stubs defeat both check moments and turn missing capabilities into silent runtime failures.

**Tooling is language-specific, contract is universal.** Rust hosts typically diff `wit-bindgen`-generated stubs against actual `Linker::instance` calls; JS/`jco` hosts diff the WIT interface list against the browser runtime's registered import object; WASI-preview-3 hosts inspect the world binding. The concrete script per host is out of scope for this section — hosts document their own approach in their spec.

Check: for every conformant host, its CI run includes both checks above, and both fail loudly (blocking merge) when the invariants are violated. A host that ships without HCV-06 in CI is not conformant, regardless of whether its `host.wit` is currently accurate.

---

## Changelog

- 2026-08-12 — **§16.5 (HCV-02) extended** to permit multi-host repositories, per `work/2026-08-12-host-wit-naming-decision.md`. The rule previously fixed a single path — `host.wit` at the repository root — with no fallback, which works for a repo that *is* a host (`clean-server`) but not for one that *contains* hosts among other components (`clean-manager`, `clean-framework`, whose `OFFICIAL_HOSTS` table already pointed at `hosts/<name>/host.wit`). There are now exactly two forms, selected by the shape of the repo, not by fallback: root for a single-host repo, `hosts/<host-name>/host.wit` otherwise. Each host still resolves to exactly one declaration at exactly one path — the one-host-one-hashable-file property [BVER-03](./08-bridge-versioning.md#84-host-declaration) depends on is unchanged, and the check now states it explicitly. §16.5 official-host registry list renamed `wasmtime_runner` → `clean-cli`.
- 2026-08-07 — Added §16.14 **Host-Side Self-Conformance Enforcement** and minted **HCV-06**: every host repo MUST run two CI checks — presence-and-parse of `host.wit`, and registration parity between `host.wit` and the runtime's actual import registrations — both blocking merges on failure. Explicit stub-import prohibition. Closes the gap where §16.4's three-check chain relied on hosts voluntarily conforming; without HCV-06, `host.wit` was documentation, not a mechanical contract.
- 2026-08-01 — Technical-debt closure pass: resolved the two DOC-13 pendings. §16.5 (HCV-02) canonical path FIXED — the host publishes its declaration as **`host.wit` at the root of its repository**, a single path with no fallback, and `wit_source` must resolve to that root file. §16.6 (HCV-03) "normalization" DEFINED — two WIT signatures are identical iff the canonical printed forms of their parsed WIT are byte-identical (canonical printing per the reference WIT tooling; identity is of the AST, so comments and whitespace never count), replacing the interim unnormalized byte comparison.
- 2026-08-01 — Conflict-log remediation pass (0.1, P8, P16.6, P16.12/13; work/2026-08-01-conflict-log-platform.md). Renumbered every section from `24.x` to `16.x` (suffixes preserved), including internal citations (the dangling "§16 §14" now points to §16.12.3). WIT examples rewritten to the 15 §0.3 vocabulary and the 08 §8.0 baseline: two-level paths without the world in the path (`clean:host/dom@0.1.0`, not `clean:host/browser/dom@0.5`), canonical import syntax (`import clean:host/routing@0.1.0;`), `@0.1.0` versions, clean-server without invented version numbers. `import?` and `has_capability()` removed (no such WIT syntax; optional imports do not exist — the world defines the contract, and runtime status is a §16.12 manifest concern). §16.10 rewritten per the P8 scope-split: the compiler does not fetch or validate the concrete host, but does validate program-vs-world against the WIT in the compilation request (`COM012`), citing 14 and 15; Moment 1/2 failures carry `COM014`/`COM015`. §16.12 cedes the MCP tool catalog to [10 — MCP Server Architecture](../02%20components/framework/10-mcp-server-architecture.md), keeping only the semantics of the checks; "Four MCP tools" corrected to five. Signature verification marked "(specification pending)". Navigation footer added.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Compiler and framework maintainers; host implementors publishing `host.wit`
- **Rule prefix:** `HCV-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [08 — Bridge Versioning](./08-bridge-versioning.md), [15 — Component Model Architecture](./15-component-model-architecture.md), [14 — Compiler Architecture](./14-compiler-architecture.md)
- **Satisfies:** INTEROP-02, INTEROP-10, LANG-16
