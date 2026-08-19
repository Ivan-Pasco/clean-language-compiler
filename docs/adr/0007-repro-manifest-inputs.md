# ADR 0007 — Manifest records for build reproduction

- **Status:** Accepted (2026-08-19)
- **Context:** Platform 14 §14.14.6 requires the reproduction operation to
  read a build manifest, refetch every input at its recorded SHA-256, and
  invoke `compile()` with the identical request. But the §14.8 manifest
  shape cannot name the request it records: it carries neither `project`
  nor `target_world` (not even by hash — `inputs` hashes sources and
  library manifests only), and `resolved_config` deliberately records
  resolved values, not request echoes. The spec's two-method
  `InputResolver` (`fetch_source`, `fetch_library`) likewise has no way to
  refetch the world contract the build was compiled against. §14.14.6 is
  therefore unimplementable against §14.8 as written.

## Decision

1. **Two provisional additions to `Inputs` in `build-manifest.json`**,
   both optional on read so pre-existing manifests still deserialize:
   - `inputs.project` — the request's `project`, verbatim.
   - `inputs.target_world` — the four identity fields (`host`, `version`,
     `world`, `sha256`); never the WIT text, which reproduction refetches
     by that hash exactly as it refetches sources.
2. **A third resolver method**, `fetch_world(host, version, sha256)`,
   alongside the spec's two. Same trust model: the operation re-verifies
   the returned text against the recorded hash, so the resolver is a
   store, never an authority.
3. **What reproduction does *not* re-derive:** `request_sha256`. The
   reconstructed request carries the resolved memory tier (the manifest
   records no request echo), so its canonical hash may differ from the
   original's even when the rebuild is byte-identical. The §14.14.6
   assertion is `outputs.wasm_sha256`, and that is the one this
   implementation makes.

## Consequences

- The manifest schema deviates from §14.8 by two optional fields. This is
  a normative-schema deviation, recorded as DISCOVERIES-M8 §4 for a
  foundation brief; if foundation lands a different shape, the fields
  migrate and old manifests keep deserializing.
- A manifest written before this ADR is refused by reproduction with a
  typed `ManifestIncomplete` failure naming the missing record — never a
  guessed request.
- First-divergent-byte reporting (§14.14.6) needs the original artifact,
  which the manifest names only by hash; the operation takes it as an
  optional input (DISCOVERIES-M8 §5).
