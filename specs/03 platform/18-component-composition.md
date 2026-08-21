# 18 — Component Composition

Under the Component Model, "composition" means resolving one component's imports against another component's exports at load time — so the resulting artifact executes without an intermediary. This chapter defines the two composition patterns Clean uses (bridge composition and middleware chains), the WASI 0.3 `wasi:http/middleware@0.3.0` world that unlocks direct component-to-component request pipelines, and the operational tradeoffs an operator faces when deciding between an in-process composed chain and a traditional networked service-to-service call. The rules here fix *what may be composed* and *how the framework and host cooperate* to lay out the graph; individual bridge contracts live in [02 — Host Bridge](./02-host-bridge.md), and the host-side WIT contract lives in [16 — Host Contract Validation](./16-host-contract-validation.md).

---

## 1. Two composition modes

Under the Component Model, "composition" means resolving one component's imports against another component's exports at load time so the resulting artifact executes without an intermediary. Clean uses two composition patterns:

### 1.1 Bridge composition

A guest imports a capability contract (`clean:session/store`, `clean:data/store`, etc.). `clean-host-core` composes a bridge component that exports that contract. The guest-to-bridge call is a direct Wasm-to-Wasm function invocation. This is how every capability under `bridges/` reaches the guest.

Documented in [Platform 15 — Component Model Architecture §5](./15-component-model-architecture.md) and each bridge spec's §6 (Composition).

### 1.2 HTTP middleware composition

`wasi:http/middleware@0.3.0` is a superset of `wasi:http/service@0.3.0`. Where `service` describes a component that *exports* an HTTP `handler` (leaf endpoint), `middleware` additionally *imports* a `handler`. A component targeting the `middleware` world sits between an inbound request and another handler:

```
inbound ──►  middleware component  ──►  handler component
             (imports handler)         (exports handler)
```

At composition time, `clean-host-core` wires the middleware's imported `handler` to another composed component's exported `handler`. The forwarded call is a direct Wasm-to-Wasm function invocation — no TCP, no HTTP serialization, no loopback interface.

---

## 2. When middleware composition applies


Middleware composition is appropriate when **both** apply:

1. **[COMP-01]** Two components ship in the same deployment unit and are versioned together.
2. **[COMP-02]** The interaction between them is on the request hot path (high fanout, latency-sensitive, or every-request).

Concrete cases inside Clean today:

- **`clean-server` + `mcp-bridge` (HTTP-SSE transport).** The MCP bridge's handler is composed directly into `clean-server`'s HTTP chain; MCP notifications flow through in-process function calls rather than a loopback socket.
- **`clean-server` + `data-bridge` HTTP-driver variants (PlanetScale, D1, Turso).** Where the HTTP driver runs as a separate Wasm component that speaks the provider's protocol, composing it as middleware avoids the loopback overhead a network-based driver would incur. Applies only in single-node deployments; edge deployments already pin drivers to the isolate.
- **Auth middleware.** A separately-published auth-decision component (validates JWTs, checks claims) can be middleware-composed in front of a guest, so every request runs the auth check as an in-process Wasm call before hitting guest code.

Cases where middleware composition is **not** appropriate:

- **[COMP-03]** The counterpart is operationally separate (different team, different scaling unit, different deploy cadence). Compose = single deploy unit; that's the wrong constraint here.
- **[COMP-04]** The counterpart is not a Wasm component (external SaaS, an existing HTTP service you're not rewriting).
- **[COMP-05]** You want to A/B test or gradually roll out the counterpart. A network hop is the honest boundary.

---

## 3. Deployment tradeoffs

| Property | Composed (middleware) | Networked (HTTP) |
|---|---|---|
| Per-call latency | ~microseconds (Wasm call) | ~hundreds of µs to ms (TCP + parse) |
| Artifact size | Both components' code in one image | Each ships independently |
| Deploy cadence | Both replaced together on any change | Independent |
| Fault isolation | Crashes affect the shared instance | Process boundary contains crashes |
| Observability | Shared tracing/logging plumbing | Standard HTTP tools apply naturally |
| Scaling | Both scale as one unit | Each scales independently |
| A/B testing | Requires a build; roll all-or-nothing | Standard traffic splitting |

Neither is universally better. The choice tracks **whether the two components are one operational unit or two**. Composition is the right call when the answer is unambiguously "one."

---

## 4. Composition mechanics

`clean-host-core` resolves middleware composition during startup, alongside bridge composition (see [clean-host-core §5](../02%20components/hosts/clean-host-core/01-specification.md#5-bridge-discovery-and-composition)). The mechanics:

### 4.1 Configuration

The composer's `host.toml` lists which components take part in the HTTP chain:

```toml
[http-chain]
# Order matters: request flows top-to-bottom, response bottom-to-top.
middleware = [
    { component = "./bridges/auth-decision.wasm",       imports = "handler" },
    { component = "./bridges/rate-limit.wasm",          imports = "handler" },
    { component = "./bridges/mcp-http-sse.wasm",        imports = "handler" },
]
guest      = { component = "./dist/app.wasm",           exports = "handler" }
```


**[COMP-06]** Each middleware component's imported `handler` is wired to the next entry down. **[COMP-07]** The final middleware's `handler` is wired to the guest's exported `handler`. **[COMP-08]** If the guest is *itself* a middleware (declares `include wasi:http/middleware;`), the chain can nest further — but the last link MUST export a plain `handler` from `wasi:http/service`; something has to be the leaf.

### 4.2 Load-time verification


Per [Platform 16 — Host Contract Validation](./16-host-contract-validation.md), the composer MUST verify:

1. **[COMP-09]** Every middleware component imports `wasi:http/handler@0.3.0` (from the `middleware` world) exactly once.
2. **[COMP-10]** The final middleware's imported `handler` resolves to a component that exports `handler` from `wasi:http/service@0.3.0` (or `middleware`, if nesting).
3. **[COMP-11]** No import is left unresolved. **[COMP-12]** Failure is a startup error; the host does not run with a partially-wired chain.

### 4.3 Runtime behavior


- **[COMP-13]** Requests enter the topmost middleware's exported `handler`.
- **[COMP-14]** Each middleware invokes its imported `handler` as a direct function call; the call returns the composed component's `response`.
- **[COMP-15]** Middleware components can short-circuit (return a response without invoking the imported handler — e.g. rate-limit rejections).
- **[COMP-16]** Responses flow back up in reverse.
- **[COMP-17]** Errors thrown by a middleware or the leaf handler surface at the client as the response the topmost middleware chose to emit — the composer does not inject its own error page.

**[COMP-18]** The composed chain runs on a single guest instance per request per [clean-host-core §5.4](../02%20components/hosts/clean-host-core/01-specification.md#54-instantiation-and-pooling); the instance pool's `instances-max` bounds concurrent chains, not per-middleware concurrency.

### 4.4 Composition transport


The mechanism `clean-host-core` uses to compose the graph is **WAC** (the Bytecode Alliance's WebAssembly Composition tool). The `[http-chain]` block in §4.1 is a declarative front-end over WAC — it covers the linear-middleware case and is expected to be sufficient for the majority of deployments. When the graph shape outgrows the front-end, the operator provides a WAC script directly. See [ADR-0025](../01%20governance/decisions/0025-wac-as-composition-transport.md) for the decision rationale.

**[COMP-20]** When `[http-chain]` declares only `middleware = [...]` and `guest = { ... }` (the shape in §4.1), the framework generates a WAC script from the TOML and passes it to WAC. Users see zero surface change.

**[COMP-21]** When `[http-chain] wac-script = "path/to/chain.wac"` is set, WAC is invoked on the operator-supplied script and `middleware` / `guest` in the same block MUST be absent — mixing the two is a startup error. The escape hatch exists for graphs the TOML front-end cannot express (non-linear branches, shared sinks, chains that consume more than one imported `handler`).

**[COMP-22]** WAC's own composition-time checks satisfy [COMP-09..COMP-12]. The framework does not re-run those checks after WAC accepts the composition; a WAC error is surfaced to the operator with the framework's diagnostic wrapper so [`COM017` `InstantiationFailure`](./09-error-codes.md) remains the top-level error code for composition failures.

**[COMP-23]** The composed artifact MUST be byte-identical between the two paths for any graph the TOML front-end can express. Same guest + same bridge components + same pinned WAC version + equivalent composition script (TOML-generated or user-supplied) → byte-identical `.wasm`. This extends [CMP-02](./14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out) to composition and is guarded by the same determinism suite.

**[COMP-24]** WAC's version MUST be pinned in the reference stack ([ADR-0002](../01%20governance/decisions/0002-clean-server-reference-stack.md), [ADR-0006](../01%20governance/decisions/0006-compiler-reference-stack.md)) and recorded in the build manifest. The lockfile checksum verification of [ADR-0022 §8](../01%20governance/decisions/0022-foundational-technology-stack.md) applies to WAC as it does to any other pinned tool.

An example `wac-script` for the linear chain in §4.1 — provided so operators can see what the TOML front-end generates and use it as a starting point when they need to drop into WAC:

```wac
package clean:server-chain;

let auth       = new bridges:auth-decision   { ... };
let rate-limit = new bridges:rate-limit      { handler: auth.handler };
let mcp        = new bridges:mcp-http-sse    { handler: rate-limit.handler };
let app        = new app:app                 { handler: mcp.handler };

export app.handler as wasi:http/service@0.3.0/handler;
```

The `package clean:server-chain` name, the `bridges:` and `app:` package prefixes, and the exported interface name are conventions of the WAC surface, not Clean-specific rules; refer to WAC's own documentation for the authoritative syntax.

---

## 5. Bridge inventory


Not every bridge is a good candidate for middleware composition. The following are the ones for which composition is explicitly supported:

| Bridge | Composition posture |
|---|---|
| [mcp-bridge](../02%20components/bridges/mcp-bridge/) | **HTTP-SSE transport composes middleware.** Enables direct in-process composition with `clean-server`; the network hop for co-located MCP servers is eliminated. Stdio and WebSocket transports do not compose. |
| [data-bridge](../02%20components/bridges/data-bridge/) | HTTP-driver variants (PlanetScale, D1, Turso) MAY compose middleware in single-node deployments. Sidecar/native variants do not (their native side owns the socket). |
| [mail-bridge](../02%20components/bridges/mail-bridge/) | HTTP-provider variants (Postmark, SendGrid, SES-HTTP) MAY compose in single-node deployments; typically not worth the coupling since mail is not hot-path. |
| session, kv, jobs, realtime, auth, roles, i18n | Do not compose HTTP middleware. Their calls flow through the bridge-composition path (§1.1), not the HTTP chain. |

**[COMP-19]** New bridges that want middleware composition MUST declare `include wasi:http/middleware;` in their world and document the composition posture in their §10 (Portability Guarantees).

---

## 6. Interaction with the bridge composition path

Middleware composition and bridge composition are **orthogonal**. A guest can simultaneously:

- Compose `clean:session/store`, `clean:data/store`, etc. via the bridge-composition path (§1.1).
- Sit inside an HTTP middleware chain with an MCP transport component composed in front of it (§1.2).

`clean-host-core` handles both in one composition pass. The composed component is a single Wasm artifact regardless of how many middleware layers wrap it or how many bridges it consumes.

---

## 7. Open questions

- **Ordering vs concurrency.** The middleware chain is strictly sequential today. Whether to allow parallel middleware (auth + tracing + rate-limit running concurrently over the same request) is deferred until measurement shows the wins.
- **Middleware-scoped configuration.** Each middleware component sees the same `clean:host/config` view as the guest; whether to introduce per-middleware config namespaces is undecided.
- **Cross-chain bridges.** If two middlewares independently compose `clean:kv/store`, do they share an instance or get separate ones? Current answer: separate (bridge composition is per-import, not per-chain). Whether to allow explicit sharing is open.
- **Dev-mode short-circuit.** For local development, whether to skip composition and let each middleware bind its own port is a UX question deferred to the framework.


---

## Changelog

- 2026-08-05 — Added §4.4 **Composition transport**; minted **COMP-20..COMP-24**. Names WAC (Bytecode Alliance's WebAssembly Composition tool) as the mechanism `clean-host-core` uses to compose the graph. `[http-chain]` in §4.1 is now framed as a declarative front-end over WAC — the framework generates a WAC script from the TOML in the linear-middleware case (COMP-20). New escape hatch: `[http-chain] wac-script = "path/to/chain.wac"` overrides `middleware` / `guest` when the graph shape outgrows the front-end (COMP-21). WAC's composition-time checks satisfy COMP-09..COMP-12 (COMP-22); framework does not re-run them. Composed artifact is byte-identical between the two paths for any TOML-expressible graph (COMP-23), extending [CMP-02](./14-compiler-architecture.md#cmp-02--same-request-in-byte-identical-outputs-out) to composition. WAC version is pinned in the reference stack and recorded in the build manifest (COMP-24). Rationale in [ADR-0025](../01%20governance/decisions/0025-wac-as-composition-transport.md). No rule renumbering; COMP-01..COMP-19 unchanged.
- 2026-08-05 — Claimed prefix `COMP-`; minted COMP-01..COMP-19 across normative sections. Added `Satisfies:` blockquote (INTEROP-01/02/06) and marked normative sections with *Normative.* + concern citations. No behavior changes; rule text preserved.
- 2026-08-05 — Renumbered from `17-component-composition.md` to `18-component-composition.md` during upstream merge (remote's `17-text-encoding.md` already occupies slot 17). Content unchanged; cross-references updated in `02 components/hosts/clean-server/01-server.md` and `02 components/hosts/clean-host-core/01-specification.md`.

---

## Metadata

- **Status:** Draft (2026-08-05)
- **Audience:** Framework and host implementors laying out component graphs; operators choosing composed vs. networked service topologies
- **Rule prefix:** `COMP-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [02 — Host Bridge](./02-host-bridge.md), [15 — Component Model Architecture](./15-component-model-architecture.md), [16 — Host Contract Validation](./16-host-contract-validation.md), [ADR-0025 — WAC as Composition Transport](../01%20governance/decisions/0025-wac-as-composition-transport.md)
- **Satisfies:** INTEROP-01, INTEROP-02, INTEROP-06
