# Platform 12. Server Extensions

Server extensions are the host functions a Clean program can call *only* when it runs on a server — HTTP request handling, SSE streams, WebSocket frames, sessions, JWT, background jobs, email, i18n. They are Layer 3 in the execution model: not portable across every host, so they live in the `server` world of the `clean:host` package rather than in the portable L2 bridge. This chapter is the single source of truth for that world's WIT contract and wire format; the Clean-language surface a developer writes (`endpoints:` blocks, `stream.server.*`, request/response helpers) is owned by the `server` library, and the Clean examples below are illustrative restatements of that surface.

---

## 0. Contract Rule


### SVX-01 — WIT signatures are the wire contract


The WIT blocks in §1–§8, §11, and §13 are the authoritative wire contract of the server layer. Every function is a typed WIT member of the `clean:host@0.1.0` package. A conforming server host MUST implement every function in these interfaces with the exact declared signature and the documented observable behavior; it MAY change internally without breaking components that rely only on the declared contract. Host bindings are generated from these WIT files (via `wit-bindgen` or equivalent); hand-transcribed copies of the signatures are prohibited (single-source rule: [15 §3 P1](./15-component-model-architecture.md#3-architectural-principles)).

---

## 1. HTTP Server Functions (8)


```wit
package clean:host@0.1.0;

interface routing {
    use request-context.{http-method};

    /// Opaque reference to a WASM export the runtime can dispatch to.
    /// Concrete resolution (function-table index, export name, or a
    /// resource) is left to the component-model binding — pick one
    /// consistently in the host.
    type handler-ref = u32;

    variant route-error {
        invalid-method,
        invalid-path,
        duplicate-route,
        empty-role,
        empty-handler-name,
    }

    variant cors-error {
        negative-max-age,
        credentials-with-wildcard,
    }

    variant rate-limit-error {
        non-positive-count,
        non-positive-window,
        unknown-strategy(string),
    }

    variant listen-error {
        invalid-host,
        port-out-of-range,
    }

    variant rate-strategy {
        ip,
        user,
    }

    /// Register the HTTP listen port. Called during module initialization;
    /// does not actually start listening (that happens after init).
    /// Clean surface: the `server:` block's `port:` field
    /// ([08-server §23.5](../02%20components/framework/libraries/08-server.md)).
    listen: func(port: u16);

    /// Configure both host and port. Supersedes `listen` when the WASM
    /// module declares `server: host: ...`. Empty `host` falls back to
    /// existing config host. Port must be in 1..=65535.
    ///
    /// WASM-declared values override defaults; explicit CLI flags merged
    /// into config before init still apply because they write to the
    /// same field.
    listen-on: func(host: string, port: u16) -> result<_, listen-error>;

    /// Register a route handler.
    ///
    /// Path parameters: `:name` captures a segment (`/users/:id` matches
    /// `/users/123`); `*` is a wildcard.
    /// Clean surface: an `endpoints:` declaration such as
    /// `GET "/users/:id" :` ([08-server §4](../02%20components/framework/libraries/08-server.md)).
    register-route: func(
        method: http-method,
        path: string,
        handler: handler-ref,
    ) -> result<_, route-error>;

    /// Register a protected route requiring authentication.
    /// `role` is the required role; empty string means any authenticated user.
    /// Clean surface: the `[guard]` route modifier, e.g.
    /// `GET "/admin/dashboard" [admin] :` ([08-server §5](../02%20components/framework/libraries/08-server.md)).
    register-protected-route: func(
        method: http-method,
        path: string,
        handler: handler-ref,
        role: string,
    ) -> result<_, route-error>;

    /// Register a Server-Sent Events (STREAM) route.
    ///
    /// Method is always GET at the HTTP level. The runtime sets
    /// `Content-Type: text/event-stream` and `Cache-Control: no-cache`
    /// on the response, and keeps the connection open until the handler
    /// calls `sse.close` or the client disconnects. Handlers should
    /// poll `sse.is-connected` to detect early disconnects.
    ///
    /// The `server` library emits this call automatically for `STREAM`
    /// endpoints ([08-server §18](../02%20components/framework/libraries/08-server.md)).
    register-sse-route: func(
        path: string,
        handler: handler-ref,
    ) -> result<_, route-error>;

    /// Configure CORS headers honoring the `server: cors:` block.
    /// Default permissive behavior (`Any`/`Any`/`Any`) only applies when
    /// this function is not called.
    ///
    /// Empty lists or `["*"]` allow Any. Per CORS spec,
    /// `allow-credentials = true` is incompatible with Any-origin — in
    /// that case only the explicit list applies (empty list = no origins
    /// allowed). `max-age-secs = 0` omits the header.
    configure-cors: func(
        origins: list<string>,
        methods: list<string>,
        headers: list<string>,
        max-age-secs: u32,
        allow-credentials: bool,
    ) -> result<_, cors-error>;

    /// Install a fixed-window rate limiter. Each unique key is allowed
    /// `per-window` requests inside a rolling `window-secs` window;
    /// further requests receive HTTP 429 with a `Retry-After` header.
    ///
    /// `ip` keys by `X-Forwarded-For`/`X-Real-IP`. `user` keys by the
    /// `session=` or `sid=` cookie value, falling back to IP when absent.
    configure-rate-limit: func(
        per-window: u32,
        window-secs: u32,
        strategy: rate-strategy,
    ) -> result<_, rate-limit-error>;

    /// Register the WASM export name invoked when a route handler errors.
    /// The runtime forwards the error message in the `X-Clean-Error`
    /// request header before dispatching the global handler; the handler
    /// reads it via `request-context.header("X-Clean-Error")`.
    ///
    /// If the global error handler itself fails, the runtime logs the
    /// secondary failure and falls back to the default JSON 500 response.
    set-global-error-handler: func(export-name: string) -> result<_, route-error>;
}
```

**Clean surface examples** — *Informative* (restatement of the 08-server surface; home: [08-server §23.5 and §18](../02%20components/framework/libraries/08-server.md)):
```clean
server:
	host: "127.0.0.1"
	port: 3000
	cors:
		allowedOrigins: ["https://example.com"]
		allowedMethods: ["GET", "POST"]
		allowedHeaders: ["Content-Type", "Authorization"]
		maxAge: 86400
		allowCredentials: true
	rateLimit:
		perMinute: 60
	handle:
		any err:
			return error("internal: " + err.message)

endpoints:
	STREAM "/api/events" :
		stream.server.emit(json.dataToText({ status: "connected" }))
		stream.server.close()
```

---

## 2. SSE Functions (5)


These functions are only valid inside a `STREAM` handler (a route registered via `routing.register-sse-route`). Calling them outside an active SSE connection returns `sse-error.not-in-stream`.

The wire format below is owned by this document. The Clean surface for these functions is `stream.server.*`, owned by [08-server §18](../02%20components/framework/libraries/08-server.md).

```wit
interface sse {
    variant sse-error {
        not-in-stream,
        client-disconnected,
    }

    /// Send a raw data event to the connected client.
    /// Wire format: `data: {payload}\n\n`.
    /// Clean surface: `stream.server.emit(data)`.
    emit: func(data: string) -> result<_, sse-error>;

    /// Send a named event to the connected client.
    /// Wire format: `event: {name}\ndata: {payload}\n\n`.
    /// Clean surface: `stream.server.emitEvent(name, data)`.
    emit-event: func(name: string, data: string) -> result<_, sse-error>;

    /// Gracefully close the SSE stream.
    /// Clean surface: `stream.server.close()`.
    close: func() -> result<_, sse-error>;

    /// Instruct the client to reconnect after a delay if disconnected.
    /// Wire format: `retry: {ms}\n\n`.
    /// Clean surface: `stream.server.retry(ms)`.
    retry: func(ms: u32) -> result<_, sse-error>;

    /// Check whether the client is still connected. Use in long-running
    /// loops to abort processing when the client has gone away.
    /// Clean surface: `stream.server.isConnected() returns boolean`.
    is-connected: func() -> bool;
}
```

---

## 3. Request Context Functions (11)


These functions access the current HTTP request during handler execution.

```wit
interface request-context {
    /// HTTP methods accepted by the routing layer. The runtime rejects
    /// any request whose method is not one of these variants.
    variant http-method {
        get,
        post,
        put,
        patch,
        delete,
        head,
        options,
    }

    record header {
        name: string,
        value: string,
    }

    /// Streaming handle for bodies larger than 8 KB. The concrete choice
    /// of `resource` vs `stream<u8>` defers to WASI Preview 3 async
    /// stabilization.
    resource incoming-body {
        /// Read up to `max` bytes. Empty list signals end-of-body.
        read: func(max: u32) -> result<list<u8>, body-error>;
        /// Fully materialize into memory. Fails with `too-large` past
        /// the runtime's cap.
        consume: func() -> result<list<u8>, body-error>;
        /// Server-computed lowercase-hex SHA-256 of the full raw body.
        /// Stable across repeat calls; equivalent to hashing what
        /// `consume` would return, but does not require materializing
        /// the bytes in linear memory. Empty body hashes to
        /// `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
        sha256-hex: func() -> string;
    }

    variant body-error {
        too-large,
        client-disconnected,
        already-consumed,
    }

    /// Get a path parameter by name.
    ///   Route: /users/:id → param("id") == "123" for /users/123
    param: func(name: string) -> option<string>;

    /// Get a query parameter by name.
    ///   URL: /users?page=2&limit=10 → query("page") == "2"
    query: func(name: string) -> option<string>;

    /// Get the request body as a UTF-8 string. Use `body-stream` when
    /// the body might exceed 8 KB or be non-UTF-8.
    body: func() -> string;

    /// Get the raw request body as an opaque byte sequence — no UTF-8
    /// decoding, no normalization, no encoding assumption. Use this for
    /// binary uploads (`application/octet-stream`, images, tarballs)
    /// where a UTF-8 detour would corrupt the payload and invalidate
    /// integrity checks the handler needs to perform.
    ///
    /// Guarantees:
    /// - The returned length equals `Content-Length` when present.
    /// - Empty bodies return an empty list (never a null handle).
    /// - `body()` semantics are unchanged — this is additive.
    body-bytes: func() -> list<u8>;

    /// Streaming view of the same bytes `body-bytes` would return.
    /// Prefer this over `body-bytes` for bodies above 8 KB.
    body-stream: func() -> incoming-body;

    /// Server-computed SHA-256 (lowercase hex, 64 chars) of the raw
    /// pre-parse request body.
    ///
    /// Hashes the exact byte source used by `body-bytes`. No mutation
    /// of the request context — repeat calls return the same digest
    /// and remain composable with `body`, `body-bytes`, and form-field
    /// access. Empty body hashes to
    /// `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
    ///
    /// Prefer this over the two-step `body-bytes → crypto.sha256-bytes`
    /// path when the handler does NOT need to persist or forward the
    /// raw bytes.
    body-sha256-hex: func() -> string;

    /// Get a request header by name (case-insensitive).
    header: func(name: string) -> option<string>;

    /// List every header on the request in the order the client sent
    /// them. Values with commas are pre-joined (RFC 7230 style).
    headers: func() -> list<header>;

    /// Get the HTTP method.
    method: func() -> http-method;

    /// Get the request path (without query string).
    path: func() -> string;

    /// Get a cookie value by name. Cookie names are case-sensitive.
    /// Parses cookies from the `Cookie` header.
    cookie: func(name: string) -> option<string>;
}
```

**Clean surface example** — *Informative* (restatement of the 08-server surface; home: [08-server §6 — Request Access Helpers](../02%20components/framework/libraries/08-server.md)):
```clean
endpoints:
	GET "/api/users/:id" :
		integer userId = req.params.id       // path parameter ("123" for /users/123)
		string? page = req.query.page        // query parameter
		Headers headers = req.headers        // request headers (name lookup is case-insensitive)
		bytes raw = req.body                 // raw request body
		CreateUser dto = req.json(CreateUser) // typed JSON body
		return json(dto)
```

**Note on `bytes` surface:** `body-bytes` and `body-sha256-hex` are reachable from bridges that traffic in `list<u8>` (SHA-256 hashers, file writers, HTTP forwarders), and are also directly consumable from the Clean language surface via the first-class `bytes` type ([§14.14.3](./14-compiler-architecture.md#14142-first-class-bytes-type)): `bytes payload = req.body` ([08-server §6](../02%20components/framework/libraries/08-server.md)). See [`./02-host-bridge.md`](./02-host-bridge.md) for the wire format and [15 — Component Model Architecture](15-component-model-architecture.md) for the component-model binding.

---

## 4. Session Management Functions (7)


These functions provide key-value session storage, CSRF token management, and cookie control. The interface is named `session` (singular), matching the vocabulary of [15 §0.3](./15-component-model-architecture.md) and [01-server §1.3.1](../02%20components/hosts/clean-server/01-server.md); the earlier `sessions` plural is gone ([ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)).

```wit
interface session {
    variant session-error {
        no-active-session,
        expired,
        invalid-id,
    }

    /// Cookie options serialized in `Set-Cookie` style. Semicolon-joined
    /// attributes (`Path=/; HttpOnly; Secure; Max-Age=3600`) accepted
    /// as `attributes` for callers that already have a formatted string.
    record cookie-options {
        path: option<string>,
        domain: option<string>,
        max-age-secs: option<u32>,
        http-only: bool,
        secure: bool,
        same-site: option<same-site>,
        attributes: option<string>,
    }

    variant same-site {
        strict,
        lax,
        none,
    }

    /// Store data by session ID. Payload is opaque to the runtime
    /// (typically JSON-encoded).
    store: func(id: string, data: string) -> result<_, session-error>;

    /// Get session data by ID. Returns `none` when not found or expired.
    get: func(id: string) -> option<string>;

    /// Delete session data by ID.
    delete: func(id: string) -> bool;

    /// Check if a session exists.
    exists: func(id: string) -> bool;

    /// Store a CSRF token for the current session (identified via auth
    /// context or cookie). Fails with `no-active-session` when neither
    /// exists.
    set-csrf: func(token: string) -> result<_, session-error>;

    /// Get the CSRF token for the current session. Returns `none` when
    /// no token is set or no session is active.
    get-csrf: func() -> option<string>;

    /// Set a response cookie with name, value, and options. Emitted in
    /// the outgoing `Set-Cookie` header.
    set-cookie: func(
        name: string,
        value: string,
        options: cookie-options,
    ) -> result<_, session-error>;
}
```

**Clean surface example** — *Informative* (surface home: the `server` and `auth` libraries — cookie emission happens through the session/auth surface, not through an imperative `http.*` API):
```clean
session.store(sessionId, json.dataToText(userData))
```

---

## 5. Role-Based Permissions (3)


These three functions are members of the `auth` interface — the earlier standalone `roles` interface was merged into `auth` per [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md) (it had no consumer outside this file). They are listed separately here for readability.

```wit
// Members of clean:host/auth (former standalone `roles` interface,
// merged per ADR-0005).

    /// A named role and the permissions it grants.
    record role-definition {
        name: string,
        permissions: list<string>,
    }

    variant roles-error {
        invalid-definition(string),
        unknown-role(string),
    }

    /// Register role definitions. Replaces any previously registered set.
    register-roles: func(definitions: list<role-definition>) -> result<_, roles-error>;

    /// Check if a role has a specific permission.
    has-permission: func(role: string, permission: string) -> bool;

    /// Get all permissions for a role. Empty list when the role is unknown.
    get-permissions: func(role: string) -> list<string>;
```

---

## 6. Session Authentication Functions (12)


These functions manage cookie-based session authentication. They work alongside the key-value session storage above.

```wit
interface auth {
    /// The active session, when one exists. Absence is modeled as
    /// `option<session-info>` — see [`04-type-system.md`](../04%20language/04-type-system.md).
    record session-info {
        user-id: u32,
        role: string,
        session-id: string,
        claims: list<tuple<string, string>>,
    }

    record new-session {
        user-id: u32,
        role: string,
        claims: list<tuple<string, string>>,
    }

    variant auth-error {
        not-authenticated,
        insufficient-role(string),
        insufficient-permission(string),
        invalid-payload,
        no-active-session,
    }

    variant reset-token-error {
        invalid-user-id,
        invalid-ttl,
        already-consumed,
        expired,
        unknown-token,
    }

    variant jwt-error {
        invalid-token,
        expired,
        missing-jti,
        replayed,
        unsupported-algorithm(string),
    }

    /// Get the current session information, or `none` when no session
    /// is active.
    get-session: func() -> option<session-info>;

    /// Check if the current request is authenticated.
    require-auth: func() -> result<_, auth-error>;

    /// Check if the user has a specific role.
    require-role: func(role: string) -> result<_, auth-error>;

    /// Check if the user has a permission (role-based). The `admin`
    /// role has all permissions; otherwise the check is
    /// role-equals-permission unless a richer mapping was registered
    /// via `register-roles` (§5).
    can: func(permission: string) -> result<_, auth-error>;

    /// Check if the user has any of the specified roles.
    has-any-role: func(roles: list<string>) -> bool;

    /// Create a new typed session and set the auth context + cookie.
    set-session: func(session: new-session) -> result<_, auth-error>;

    /// Clear the current session (logout). Removes session data and
    /// sets a clear-cookie header. Errors with `no-active-session`
    /// when there is nothing to clear.
    clear-session: func() -> result<_, auth-error>;

    /// Get the current authenticated user's ID.
    user-id: func() -> option<u32>;

    /// Get the current authenticated user's role.
    user-role: func() -> option<string>;

    /// Generate a cryptographically-random password reset token for
    /// `user-id`, persist `(sha256(token), user-id, expires-at)` in the
    /// server-side `password_resets` map, and return the plaintext
    /// token to the caller. The plaintext is never stored — only its
    /// SHA-256 digest — so a memory dump does not leak usable tokens.
    ///
    /// Token entropy: 32 random bytes (2× UUID v4) hex-encoded to 64
    /// chars, safe to embed in a URL. Storage is in-memory in the
    /// `SessionStore`; reset tokens do not survive a server restart,
    /// matching the durability of session cookies.
    ///
    /// Backs `auth.createResetToken(userId, ttlSeconds)`.
    create-reset-token: func(
        user-id: u32,
        ttl-seconds: u32,
    ) -> result<string, reset-token-error>;

    /// Atomically validate the presented token and remove the stored
    /// row on success. Returns the associated `user-id`, or an error
    /// if the token is invalid, expired, or already consumed.
    ///
    /// Atomicity comes from holding the `SessionStore` write lock
    /// across the check-and-remove, so a second caller racing on the
    /// same token observes the row already gone.
    ///
    /// Backs `auth.consumeResetToken(token)`.
    consume-reset-token: func(token: string) -> result<u32, reset-token-error>;

    /// Atomically verify a JWT refresh token, mark its `jti` as
    /// consumed in the server-side revocation list, and return a
    /// freshly-signed token with a new `jti`, `iat`, and
    /// `exp = now + new-ttl-seconds`. Enforces single-use rotation —
    /// replaying an already-rotated token returns `jwt-error.replayed`.
    ///
    /// The token being refreshed **must** carry a `jti` claim. Refresh
    /// tokens signed without one cannot be tracked for replay and are
    /// rejected with `missing-jti`.
    ///
    /// The revocation entry for the consumed `jti` lives for the
    /// remainder of the original token's `exp` window (capped at 30
    /// days), so an attacker who captures a rotated token cannot
    /// replay it up to natural expiry.
    ///
    /// Original claims (`sub`, `role`, custom fields) are preserved on
    /// the new token; only `iat`, `exp`, and `jti` are refreshed.
    refresh-and-rotate-jwt: func(
        token: string,
        secret: string,
        algorithm: string,
        new-ttl-seconds: u32,
    ) -> result<string, jwt-error>;
}
```

---

## 7. Response Manipulation Functions (2)


These functions allow handlers to control HTTP response headers and redirects.

```wit
interface response {
    use request-context.{http-method};

    /// Redirect status codes supported by the runtime. Any other status
    /// is rejected at the WIT boundary.
    variant redirect-kind {
        /// 301 — Moved Permanently (cacheable, may change method to GET)
        moved-permanently,
        /// 302 — Found (temporary, may change method to GET)
        found,
        /// 303 — See Other (always use GET for redirect)
        see-other,
        /// 307 — Temporary Redirect (preserves HTTP method)
        temporary-redirect,
        /// 308 — Permanent Redirect (preserves HTTP method)
        permanent-redirect,
    }

    variant response-error {
        invalid-header-name,
        invalid-header-value,
        header-already-sent,
    }

    /// Set a custom response header. Common uses: CORS
    /// (`Access-Control-Allow-Origin`), cache control (`Cache-Control`,
    /// `ETag`, `Expires`), security headers (`X-Frame-Options`,
    /// `Content-Security-Policy`), custom content types.
    set-header: func(name: string, value: string) -> result<_, response-error>;

    /// Send an HTTP redirect response.
    ///
    /// When a redirect is set, the handler's return value (body) is
    /// ignored. Cookies set via `session.set-cookie` or
    /// `auth.set-session` are still included in redirect responses, as
    /// are custom headers set via `set-header`.
    redirect: func(url: string, kind: redirect-kind) -> result<_, response-error>;
}
```

**Clean surface example** — *Informative* (restatement of the 08-server surface; home: [08-server §7 — Response Helpers](../02%20components/framework/libraries/08-server.md)):
```clean
endpoints:
	GET "/old-dashboard" :
		return redirect("/dashboard")

	GET "/api/data" :
		return header("Cache-Control", "max-age=3600"), json(data)
```

Numeric status arguments are not part of the surface. The wire contract is the `redirect-kind` variant above: the library maps its redirect forms onto `moved-permanently` (301), `found` (302), `see-other` (303), `temporary-redirect` (307), or `permanent-redirect` (308); any other status is rejected at the WIT boundary.

---

## 8. Handler Function Signature


Route handlers are component-model exports registered with `routing.register-route` (or the protected/SSE variants). The runtime dispatches to them by `handler-ref`. These declarations are members of the `routing` interface — the earlier standalone `handler` interface was merged into `routing` per [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md) (it had no consumer outside this file). Each handler returns a response payload:

```wit
// Members of clean:host/routing (former standalone `handler`
// interface, merged per ADR-0005).

    variant handler-error {
        internal(string),
    }

    /// A handler produces either a UTF-8 response body (typically JSON)
    /// or an error the runtime forwards to the global error handler
    /// registered via `routing.set-global-error-handler`.
    type handler-func = func() -> result<string, handler-error>;
```

The compiler assigns each handler function a unique integer index, exports it as `handle_event_<index>` (the single generated-export scheme — [Libraries Specification §8.6](../02%20components/framework/09-libraries-specification.md)), and populates the `handler-ref` used at registration time.

**Example handler** — *Informative* (restatement of the 08-server surface; home: [08-server §4](../02%20components/framework/libraries/08-server.md)):
```clean
endpoints:
	GET "/api/users/:id" :
		integer id = req.params.id
		list<map<string, any>> rows = db.query:
			sql: "SELECT * FROM users WHERE id = ?"
			params: [id]
		return json(rows[0])
```

Raw SQL on the application surface is legitimate as the escape hatch of [data §8](../02%20components/framework/libraries/04-data.md): it crosses the driver ABI verbatim through the dedicated `execute_raw` vtable entry, and the dialect of the hand-written text is the author's responsibility ([ADR-0003](../01%20governance/decisions/0003-sql-dialect-resolution.md), companion decision). DSL-declared queries never carry SQL text — they cross as query IR.

---

## 9. Request/Response Flow


```
1. HTTP Request arrives
   │
2. Server creates RequestContext
   │  - method, path, headers, body
   │  - parsed params, query
   │
3. Route matcher finds handler
   │  - Checks if protected
   │  - Validates authentication
   │  - Checks role requirements
   │
4. Create fresh WASM instance (or reset memory)
   │
5. Set request context in the host state exposed to
   │  the `request-context` interface
   │
6. Call handler function by `handler-ref`
   │
   ├─── Standard route ────────────────────────────────────────┐
   │    Handler reads via the `request-context` interface       │
   │    Handler returns `result<string, handler-error>`         │
   │    Runtime turns the payload into the HTTP response        │
   │    Send HTTP response to client                            │
   │                                                            │
   └─── SSE route (routing.register-sse-route) ───────────────┐ │
        Runtime sets Content-Type: text/event-stream           │ │
        Keeps connection open                                  │ │
        Handler calls `sse.emit` / `sse.emit-event`            │ │
        Handler polls `sse.is-connected` in a loop             │ │
        Handler calls `sse.close` when done                    │ │
        Connection closed                                      │ │
        ───────────────────────────────────────────────────────┘ │
                                                                 │
        ─────────────────────────────────────────────────────────┘
```

---

## 10. Implementation in Different Runtimes


Hosts implement this world via `wit-bindgen`; see [15 — Component Model Architecture](./15-component-model-architecture.md) for the code-generation flow.

---

## 11. Diagnostics Capture Interface


The `diagnostics` interface (renamed from `dev-mode` per [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md); it is the capability [01-server §1.3.1](../02%20components/hosts/clean-server/01-server.md) calls `clean:host/diagnostics`) is the server-side snapshot bridge behind the error-reporting pipeline. This document owns the WIT contract below. Everything downstream of it — the capture endpoint, tarball layout, `pass_criteria` schema, and retest sandbox — is internal tooling of the `clean-errors` component and is documented in that component's repository (see [ADR-0008 — Error reporting backend reference design](../01%20governance/decisions/0008-error-reporting-backend.md)).

```wit
interface diagnostics {
    record source-file {
        path: string,
        content: string,
    }

    record request-log-entry {
        method: string,
        path: string,
        status: u16,
        duration-ms: u32,
        captured-at: string,
        headers: list<tuple<string, string>>,
        body: string,
        body-truncated: bool,
    }

    record snapshot {
        source-tree: list<source-file>,
        current-wasm: list<u8>,
        last-log-lines: string,
        request-log: list<request-log-entry>,
        db-schema: string,
        project-hash: string,
        component-versions: list<tuple<string, string>>,
        captured-at: string,
    }

    variant capture-error {
        /// `CLEAN_DEV` env var is unset or not exactly "1".
        not-in-dev-mode,
        /// Serialized capture would exceed 32 MB.
        payload-too-large,
    }

    /// Return the current development-mode snapshot. Gated on
    /// `CLEAN_DEV=1`; returns `not-in-dev-mode` otherwise.
    snapshot: func() -> result<snapshot, capture-error>;

    // TODO: `report-panic`, `capture-render`, and `replay-input` from
    // the error-reporting reproduction pipeline are referenced by the
    // capture flow but not yet specified at the WIT level. Their shapes
    // will be pinned once the clean-errors sandbox contract stabilizes
    // (ADR-0008); until then, capture is served entirely by `snapshot`.
}
```

**Bridge-level observable constraints:**

- Cap `source-tree` at **200 files** and **4 levels deep**. Skip `.git/`, `target/`, `node_modules/`, `tests/output/`, and anything matching a top-level `.gitignore`.
- Cap `current-wasm` at **8 MB** raw. Larger WASM binaries return an empty list and add a warning to `last-log-lines`.
- `last-log-lines` is the last **100 lines** of stderr+stdout concatenated. Do not include ANSI escape codes — strip them at emission time.
- `request-log` is the last **20 requests** in chronological order (newest last), shaped as the `request-log-entry` record above. Each entry's `body` is truncated at **8 KB**; when truncated, `body-truncated` is `true`. Header redaction follows SVX-03.
- `db-schema` is `""` when no database is attached. When attached, it is the **driver-emitted, dialect-neutral structured schema description** — every user-defined table with its columns and indexes — serialized as canonical JSON. It is **not** SQL text: `SHOW CREATE TABLE` (MySQL-specific) is gone per [ADR-0003](../01%20governance/decisions/0003-sql-dialect-resolution.md). The exact shape is owned and versioned by the driver ABI ([04 — Database Driver ABI](../02%20components/framework/04-database-libraries.md), `DRV-`).
- `project-hash` is `SHA-256(git_remote_origin_url + "|" + git_rev_parse_show_toplevel)`. Matches the formula Clean Manager and the compiler use so bugs from the same project cluster on the errors dashboard.
- When `CLEAN_DEV` is unset or not exactly `"1"`, the bridge returns `capture-error.not-in-dev-mode` (SVX-04).

### SVX-03 — Sensitive headers are redacted at the bridge


In every `request-log-entry` the `diagnostics` bridge emits, the values of the `Cookie` and `Authorization` headers MUST be replaced with the literal string `<redacted>` **at this bridge, before the framework sees the values** — the framework cannot distinguish a real header value from a redacted one and cannot re-apply redaction correctly. Check: no `snapshot` result contains a non-`<redacted>` value for either header, regardless of what the client sent.

### SVX-04 — Snapshot capture is bounded and dev-gated


`diagnostics.snapshot` MUST enforce every cap in the constraint list above (200 files / 4 levels, 8 MB WASM, 100 log lines, 20 request-log entries with 8 KB bodies) and MUST fail with `capture-error.payload-too-large` when the serialized capture would exceed **32 MB**. It MUST return `capture-error.not-in-dev-mode` unless the `CLEAN_DEV` environment variable is exactly `"1"`. Check: a request with `CLEAN_DEV` unset never yields a snapshot; a capture exceeding any cap yields the truncated/empty form documented above, never an unbounded payload.

---

## 12. Function Count Summary


| Section | Count |
|---------|-------|
| HTTP Server (routing) | 8 |
| SSE | 5 |
| Request Context | 11 |
| Session Management | 7 |
| Role-Based Permissions (in `auth`) | 3 |
| Session Authentication | 12 |
| Response Manipulation | 2 |
| Diagnostics Capture | 1 |
| **Total** | **49** |

All 49 functions are Layer 3 (server-only). Every server runtime that ships Clean Language HTTP support implements them (SVX-01, SVX-02); portable Layer 2 functions live in [`02-host-bridge.md`](./02-host-bridge.md).

---

## 13. World Declaration


The `server` world composes every interface in this document with the WASI and cross-cutting Clean bridge packages a server runtime needs. The interface *vocabulary* is owned by [15 §0.3](./15-component-model-architecture.md) and extended by [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md) (which adds the interfaces backed by [01-server §1.3.1](../02%20components/hosts/clean-server/01-server.md) and merges `roles` into `auth` and `handler` into `routing`); this document owns the *contents* of each server-only interface.

```wit
world server {
    import wasi:filesystem/types@0.3.0;
    import wasi:cli/stdout@0.3.0;
    import wasi:cli/stderr@0.3.0;
    import wasi:clocks/wall-clock@0.3.0;
    import wasi:clocks/monotonic-clock@0.3.0;
    import wasi:random/random@0.3.0;
    import wasi:http/handler@0.3.0;                 // outbound `handle` (async)
    import wasi:logging/logging@0.2.0;              // 0.3 cut not yet shipped upstream

    import clean:bridge/db@1.0.0;

    import clean:host/routing@0.1.0;
    import clean:host/request-context@0.1.0;
    import clean:host/response@0.1.0;
    import clean:host/sse@0.1.0;
    import clean:host/ws@0.1.0;
    import clean:host/session@0.1.0;
    import clean:host/auth@0.1.0;
    import clean:host/jobs@0.1.0;
    import clean:host/email@0.1.0;
    import clean:host/i18n@0.1.0;
    import clean:host/diagnostics@0.1.0;

    export wasi:http/service@0.3.0;                 // inbound service (async, replaces incoming-handler)
}
```

Notes on membership:

- `auth` and `session` are **L3 host interfaces** (`clean:host/*`), not L2 bridge interfaces — the earlier `clean:bridge/auth` and `clean:bridge/session` imports were membership errors (the L2 catalog home is [02 §2.2.1](./02-host-bridge.md)). `clean:bridge/db` stays: `db` is in the L2 catalog.
- `ws`, `jobs`, `email`, and `i18n` are imported because the server host provides them ([01-server §1.3.1](../02%20components/hosts/clean-server/01-server.md)); their function contents are specified by their owning libraries and are outside this document's scope.
- `sessions` (plural), `roles`, `handler`, and `dev-mode` no longer exist as interfaces (renames and merges per [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)).

### SVX-02 — The `server` world composes exactly the interfaces listed here


The world declaration above is exhaustive. A conforming server host MUST provide every listed import and consume the exported `wasi:http/service@0.3.0`; it MUST NOT silently accept a component that imports an interface outside this list. Adding, removing, or renaming an interface in the `server` world requires an ADR amending the vocabulary of [15 §0.3](./15-component-model-architecture.md#03-wit-package-and-world-naming) (the pattern set by [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)). A component whose imports the host cannot satisfy is refused at instantiation — the Moment 3 check of [16 §16.4](./16-host-contract-validation.md#164-the-three-check-moments) — with the structured error [`COM017` `InstantiationFailure`](./09-error-codes.md).

---

## Changelog

- 2026-08-02 — Link repair: the `bytes` deferred-refinement citation pointed at §14.14.3; the section is §14.14.2. No normative change.
- 2026-08-01 — Applied [ADR-0003](../01%20governance/decisions/0003-sql-dialect-resolution.md) (Accepted), resolving both "(open question — ADR-0003, Draft)" markers: the §11 `db-schema` field is now the driver-emitted, dialect-neutral structured schema serialized as canonical JSON (shape versioned under the driver ABI's `DRV-` contract) instead of `SHOW CREATE TABLE` output; the §8 raw-SQL example is legitimate as the [data §8](../02%20components/framework/libraries/04-data.md) escape hatch crossing the ABI through `execute_raw` (companion decision), and was rewritten from the removed positional `db.query(sql, params)` form to the V2 block form.
- 2026-08-01 — Technical-debt closure pass: SVX-03 now cites the new **C-22 — Privacy** concern ([05-concerns](../01%20governance/05-concerns.md)) alongside C-02; the "no dedicated privacy concern" note is retired.
- 2026-08-01 — Fase 3 remediation per the approved conflict log (P5, P6, P9, P15, P16.6, P16.11, resolution 0.1): removed the three "code wins"/"code is authoritative" sentences (they inverted SDD-08); extracted §11 (clean-errors capture tooling: tarball layout, `pass_criteria`, retest sandbox) to `work/2026-08-01-clean-errors-extraction-12.md` — destination: the clean-errors component docs per [ADR-0008](../01%20governance/decisions/0008-error-reporting-backend.md); §11 now holds only the `diagnostics` WIT interface (renamed from `dev-mode` per [ADR-0005](../01%20governance/decisions/0005-server-world-interface-additions.md)) and its observable bridge constraints; §13 world aligned with 15 §0.3 + ADR-0005 (`sessions`→`session`, `dev-mode`→`diagnostics`, `roles` merged into `auth`, `handler` merged into `routing`, `clean:bridge/auth`/`clean:bridge/session` removed as L2 membership errors, `ws`/`jobs`/`email`/`i18n` added per 01-server §1.3.1); Authority header recast — the world is `server` inside `clean:host`, not a package; Clean surface examples rewritten to the 08-server surface (`endpoints:` blocks, `stream.server.*`, `req.*` helpers, response helpers) and the imperative server-side `http.get/get_protected/listen/setCookie` API removed (name collision with the outbound HTTP client disqualified it); invalid example syntax fixed (`let`/`func … -> {}`/snake_case gone); `use` direction of `http-method` corrected (defined in `request-context`; `routing` and `response` import it); `redirect(url, 302)` → the `redirect-kind` variant; generated exports `__route_handler_N` → `handle_event_<index>` (LBS §8.6); §12 counts corrected (Request Context 11, total 49); raw-SQL touchpoints marked "(open question — [ADR-0003](../01%20governance/decisions/0003-sql-dialect-resolution.md), Draft)"; stale internal citations (`§10.6`, "§11 Open Question 1") repaired or removed.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Server-host implementors (Rust `clean-server`, Node, Go, etc.) and framework authors targeting the `server` world
- **Rule prefix:** `SVX-`
- **Part of:** [Clean Language Specification — Platform](./README.md)
- **References:** [01 — Execution Layers](./01-execution-layers.md), [02 — Host Bridge](./02-host-bridge.md), [15 — Component Model Architecture](./15-component-model-architecture.md), [server library](../02%20components/framework/libraries/08-server.md)
