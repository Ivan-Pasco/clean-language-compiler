# Discoveries — Milestone 7 (language server)

Spec gaps and under-specifications found while implementing M7. Each item
either becomes a task brief in foundation `work/` (written from a
foundation session — this repo never writes there) or records a local
adoption that stays in force until foundation resolves it.

## Status

M7 complete (2026-08-19, 5 stages). The milestone gate — LSP diagnostics
≡ `cln check` diagnostics: same list, same codes, same spans, over the
same request document — holds as a contract test on hand-written cases
(`tests/parity.rs`) and over the full DIA-06 fixture corpus
(`tests/parity_corpus.rs`). Diagnostics push, hover, and go-to-definition
serve from one pipeline run per edit (`check_with`).

Items 1 and 2 were carried to foundation on 2026-08-19 and are decision
briefs, both Ready (report received the same day; neither was resolvable
as a direct erratum, and no spec correction falls on the local
adoptions — both stay in force unchanged until the briefs execute):

- Item 1 → `work/2026-08-19-editor-mode-request-document.md`
- Item 2 → `work/2026-08-19-request-level-diagnostics-editor-placement.md`

The briefs are cross-linked (the `requestDocumentUri` designation in
item 2 rides item 1's channel decision) and may resolve together.
Items 3–7 are local pinnings or milestone scoping that dissolve as the
surface grows.

## 1. Editor-mode request construction is unspecified

Platform 14 §14.1 names the LSP as one of the compiler's callers — driven
through the same request document, "without any of them having to touch the
filesystem in an ambient way" — and Platform 04 owns the protocol. Neither
says how the language server *obtains* a request document in editor mode:
`didOpen`/`didChange` deliver file contents, but the request also carries
`target_world`, dependencies, limits, and the full source list.

**Local adoption (in force, `session.rs`):**

- `initializationOptions.requestDocument` carries the request document
  verbatim — the same JSON `cln` would hand `clean-compiler --check`. The
  extension (or `cln` acting for it) composes it; the server composes
  nothing and discovers nothing (CMP-01).
- `didOpen`/`didChange` (full sync) overlay `sources[].content` for the
  matching path; the server recomputes that entry's `sha256` because the
  overlaid request is one the server is composing as a caller. RQD001
  keeps guarding the base document exactly as delivered.
- `didClose` drops the overlay, reverting to the base document content —
  never to the filesystem.
- A document whose URI does not resolve (against the workspace root) to a
  `sources[]` path is not compiled; the server says so via
  `window/logMessage`. The request decides the compilation unit.
- Missing `requestDocument` entirely is the caller's defect: the server
  serves protocol lifecycle only and logs why once.

**Carried to foundation (2026-08-19):**
`work/2026-08-19-editor-mode-request-document.md` (Ready). The brief asks
foundation to fix (a) the normative delivery channel (extension via
`initializationOptions` with `cln` composing / server pulls the Framework
lowering / a `cln` dev daemon serves and pushes), (b) who recomposes the
request on project-shape changes — file create/delete/rename, dependency
change, `clean.toml`/lockfile edits — and over what mid-session channel
the running server receives it (`initializationOptions` is
initialize-time only, so a second channel is needed under any option),
and (c) ratification of the overlay semantics above. No correction on
this adoption; it stays in force until the brief executes.

## 2. Where request-level diagnostics publish

Diagnostics with `primary_span.file == "<request>"` (RQD codes, COM005,
COM003's request-shaped cases) have no source file to publish under, and
LSP has no file-less diagnostics channel.

**Local adoption (in force):** they publish under
`initializationOptions.requestDocumentUri` when the caller names one, else
under the synthetic URI `clean:request`. Spans convert to the zero range.
A span naming a file that is neither `<request>` nor a `sources[]` entry
joins the same bucket rather than vanish (parity over placement); no such
span is currently emitted.

**Carried to foundation (2026-08-19):**
`work/2026-08-19-request-level-diagnostics-editor-placement.md` (Ready).
The brief asks foundation to fix (a) the wire representation — the
`<request>` sentinel is itself unregistered: 13 §6.1 shows `primary_span`
as always present and the only spec language for the case is COM003's
"program-level with no primary span" prose, so the brief asks to register
the sentinel or make `primary_span` optional under DIA-04 — (b) the LSP
destination (caller-designated URI with `clean:request` fallback — this
adoption — vs. always `clean:request`; a heuristic real-file anchor is
listed for rejection), (c) normative parity rules (no diagnostic dropped
for lack of a URI, zero range, unknown-file spans join the same bucket),
and (d) whether the rule mints as DIA- in 13 §7 or LSP- in 04. No
correction on this adoption. **Migration watch:** if foundation makes
`primary_span` nullable instead of registering the sentinel, the DIA-06
fixtures and the sentinel-based bucketing here need a migration (the
brief requires a migration note in that case). Cross-linked with item 1:
the `requestDocumentUri` designation rides its channel decision.

## 3. Notes/helps append format on the wire

Platform 13 §7: `notes`/`helps` are "appended to `message` when the editor
does not fetch code actions", with no format given. This server does not
yet advertise `codeAction`, so appending is the only delivery.

**Local adoption (in force, `convert.rs`):** each note appends as
`"\nnote: {note}"`, each help as `"\nhelp: {help}"` — the CLI renderer's
prefixes. When `codeAction` lands, appending becomes conditional on the
client's declared capability, per §7's reading.

## 4. Pre-v1 `Unsupported` in the editor

`CompileError::Unsupported` entries carry no registered code (DIA-01
forbids publishing them as diagnostics), mirroring the batch adapter's
stderr + exit 3. **Local adoption:** they surface as `window/logMessage`
warnings naming construct and location, and every diagnostics bucket
publishes empty. Pre-v1 only; dissolves when the surface completes.

## 5. `initialized` is consumed by the transport

`lsp-server`'s `initialize_finish` consumes the client's `initialized`
notification internally. The first diagnostics push therefore happens
immediately after the handshake returns, not in an `initialized` handler —
observable only as "diagnostics arrive without waiting for an edit", which
is what Platform 04 §4.1 wants anyway. Implementation note, not a spec gap.

## 6. Hover answers only from an authoritative typed program

Platform 04 §4.1 wants hover on "the type of the expression under the
cursor" but does not say what hover shows while the program is ill-typed.
The typed program the pipeline produces is authoritative only after pass
[6] re-validation (§14.4.2) — before that there is nothing the type
checker stands behind.

**Local adoption (in force, `driver.rs::check_with` + `analysis.rs`):**
the hover/definition index is captured by an observer that runs once,
immediately after pass [6], inside the same `check` run that produced the
diagnostics — one pipeline, one request, one answer (CCMP-25). When the
run stops earlier (parse or type errors), hover returns null rather than
a stale or guessed answer (LSP-04's rationale). The index does survive a
*later*-stage stop (pre-v1 `Unsupported`, COM003), where the typed
program is valid. Call and host-call expressions hover as the callee's
Clean-surface signature; every other expression hovers as `Ty::display()`.

## 7. Definition coverage in M7

Platform 04 §4.1 wants definition "across files and into libraries".
What the typed program resolves directly is covered: user-function calls
(cross-file included), locals and parameters (the TIR's `Local` gained a
declaration span for this), state variables, and host-function calls.
Methods, fields, constructors, and class names index the resolver's
`Declarations`, which does not surface their declaration spans yet —
they return null, never a guess. Library jumps wait for the library
system's IR spans (§21.5). Milestone scoping, not a spec gap; the
remainder lands with the resolver surfacing declaration spans.

## 8. Binary name

No spec names the language-server binary (Platform 04 says "the language
server binary"; Manager resolves it at the pin). **Local adoption (ADR
0006):** crate and binary are `clean-language-server`; not a user-facing
command (CCMP-04) — editors reach it through Clean Manager (LSP-05).
