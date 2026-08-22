# ADR-0023 — Secret-handling strategy

Values known to be secret must not appear in logs, diagnostics, error payloads, or default `toString` output, but the ecosystem's current mechanism (name-pattern redaction in crash dumps only) leaves log output, HTTP serialisation, and default string representations uncovered. This ADR introduces a first-class `secret` type into the Clean language: taint is tracked by the compiler across assignment, passing, return, interpolation, and concatenation, and can only be stripped by an explicit `.reveal()` call site — one grep-able declassification point per codebase, mechanically enforced without framework cooperation at each new serialisation surface.

---

## Context

[SEC-07](../08-security-principles.md) requires that values known to the framework as secret MUST NOT appear in diagnostic messages, error payloads, log output, serialised debug representations, crash reports, or telemetry. A secret can be surfaced only through a named, explicit operation whose call site is grep-able.

A pre-decision audit (see work note) confirmed that a partial convention already exists: secrets enter through environment variables, the auth library consumes them via `env.get("JWT_SECRET")`, and the error-reporting layer strips bytes matching name-pattern heuristics from core dumps. That convention covers crash-dump redaction but leaves log output, HTTP response serialisation, and the default `toString` of a secret-typed value unprotected.

A decision on how to close those gaps was required before implementation could proceed. Four options were analysed (see work note §"Decision needed"). This ADR records the chosen approach.

## Decision

Adopt **Option A — a language-level `secret` type with compiler-tracked taint.**

`secret` is a first-class type in the Clean Language type system, analogous to `none` in that it is non-parameterised and non-constructible by user code. The compiler tracks taint through variables; a value derived from a `secret` through an assignment, pass, return, or obvious transformation remains `secret` until explicitly declassified at a single, grep-able call site.

### Taint rules (authoritative; the type-system spec cites this section)

1. **`secret` is a non-parameterised built-in type.** It is not a class, not a user-constructible generic, and does not conflict with [LANG-12](../07-language-principles.md) (which forbids *user-defined* generics). The compiler knows about it the same way it knows about `none`.

2. **Production.** `secret` values are produced only by host bridges that are declared to return `secret`. The canonical producer is `env.get(name)`: when `name` matches the secret name pattern (default: names ending in `_SECRET`, `_TOKEN`, `_KEY`, or `_PASSWORD`), the call returns `secret`; otherwise it returns `string`. No other expression produces a `secret` unless a declared bridge function carries a `secret` return type.

3. **Taint propagation.** Taint is contagious across these operations:
   - **Assignment**: `secret x = someSecret` — `x` is `secret`.
   - **Function parameter passing**: a parameter that receives a `secret` argument is `secret` inside the callee.
   - **Return**: a function that returns a `secret` value has return type `secret`.
   - **String interpolation**: a string containing a `secret` interpolation is `secret`.
   - **String concatenation**: `someString + someSecret` is `secret`.
   - **Container membership**: an element extracted from a `list<secret>` is `secret`; a value read from a container that holds any `secret` element is `secret`.

4. **Declassification.** Taint is stripped only by `.reveal() -> string`. This is the sole declassification point. Every call site is a review target under SEC-07; the name is chosen to be grep-able across any codebase.

5. **Default representations.** The default `toString()`, debug repr, and JSON/serialisation of a `secret` all emit the literal string `"[REDACTED]"`. These overrides apply mechanically at every serialisation surface — the framework does not need to cooperate separately.

6. **Equality.** Comparing two `secret` values uses constant-time comparison of the underlying bytes, satisfying [SEC-09](../08-security-principles.md)'s requirement for timing-safe comparisons.

7. **Safe query operations.** `is_empty() -> boolean` is safe to call on a `secret` without declassifying it; it returns whether the underlying value has zero length, leaking no content.

## Why Option A over the alternatives

**Option B (stdlib wrapper class)** places `secret` in the standard library as a class whose redacted `toString` and serialisation overrides the framework honours. The guarantee survives assignment and passing but is lost the moment a value flows through a transformation the framework's serialisation layer does not recognise. A developer can inadvertently escape the wrapper by passing the secret to any function that accepts `any`. The compiler has no visibility into the invariant.

**Option C (extend the env-var naming convention to log and HTTP surfaces)** requires the framework to track env-var provenance through the log-argument path for the lifetime of every value. Provenance is lost on any transformation — `token.substring(0, 8)` produces an untracked string. Best-effort coverage with no compile-time visibility.

**Option C-minus (rewrite SEC-07 to match the current partial convention)** accepts that log output, serialisation, and default `toString` leak secrets. Defensible only if the ecosystem is comfortable calling secret hygiene a developer responsibility outside crash reporting. It is not.

Option A's mechanical enforcement survives assignment, passing, return, interpolation, and concatenation — the full gamut of transformations a value undergoes in real code — without requiring framework cooperation at each new serialisation surface.

## Consequences

**Positive**

- Mechanical, whole-ecosystem enforcement. Any new serialisation surface inherits the guarantee automatically via the type's overridden `toString` and JSON representations — no opt-in, no framework update required.
- Grep-able declassification. Every `.reveal()` call site in any codebase is a candidate for security review. Tooling can flag them.
- Compile-time visibility. The compiler can warn when a `secret` value is passed to a function whose signature accepts `string` without an intervening `.reveal()`, surfacing accidental declassification at author time.

**Negative**

- Biggest single language change to date. Compiler implementation is required before any application can use the `secret` type in production. Auth-library signatures change from `string` to `secret` for parameters that previously received the raw env-var value.
- Existing code that passes `env.get("JWT_SECRET")` directly to a function expecting `string` will fail to compile once the type is introduced. Migration requires adding `.reveal()` at each call site that genuinely needs the raw bytes.

**Neutral**

- Auth-library function signatures (e.g. the JWT signer) change from `string` to `secret` for the secret parameter. Code that currently does `env.get("JWT_SECRET")` and immediately passes the result to the signer does not need `.reveal()` — the signer accepts `secret` directly. Only call sites that ultimately hand raw bytes to a third-party function expecting `string` need the explicit declassification.

## Alternatives considered

**Option C-minus — Rewrite SEC-07 to match current reality.** Reduces SEC-07 to "secrets enter through env vars and are stripped from crash reports by name-pattern heuristics." Rejected: leaves log output, HTTP response serialisation, and `toString` uncovered; endorses the weakest possible story.

**Option C — Extend the env-var convention to log and HTTP surfaces.** Framework tracks env-var provenance and applies redaction at log and serialisation layers. Rejected: provenance is lost on any transformation; best-effort coverage leaves gaps that are invisible at compile time.

**Option B — Standard-library `secret` wrapper class.** Redacted `toString` and JSON serialisation overrides that the framework honours. Rejected: guarantee is lost when a value flows through a `any`-typed parameter or through code the framework's serialisation layer does not recognise; compiler has no visibility; wrapper can be unwrapped without `.reveal()` by a developer who imports the class directly.

## Rollout

1. **This ADR is Accepted.** The decision is recorded.
2. **Spec edits** are performed per §"Files touched" in the work note: type-system spec, stdlib spec, error-reporting spec, auth-library spec, and SEC-07 body.
3. **Compiler implementation** follows as a separate task in the compiler roadmap. No compiler code is changed by this ADR. The spec edits are the record of intent; implementation is tracked separately.
4. Until the compiler implements `secret`, the existing env-var convention (name-pattern redaction in core dumps) remains the operative mechanism. No existing code breaks before the compiler change lands.

---

## Metadata

- **Status:** Accepted (2026-08-04)
- **Date:** 2026-08-04
- **Supersedes:** None
- **Implements:** [SEC-07](../08-security-principles.md)
- **Work note:** [work/2026-08-02-sec-07-decide-secret-type-strategy.md](../../work/2026-08-02-sec-07-decide-secret-type-strategy.md)
- **Spec impact:** [04 language / 04 — Type System](../../04%20language/04-type-system.md), [04 language / 15 — Standard Library](../../04%20language/15-standard-library.md), [02 components / framework / libraries / 01 — Auth](../../02%20components/framework/libraries/01-auth.md), [03 platform / 06 — Error Reporting](../../03%20platform/06-error-reporting.md)
