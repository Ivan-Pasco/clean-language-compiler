# Discoveries — Milestone 5 (block handlers sandbox)

Spec gaps and under-specifications found while implementing M5. Each item
either becomes a task brief in foundation `work/` (written from a
foundation session — this repo never writes there) or records a local
adoption that stays in force until foundation resolves it.

## Status

Open. The three M3 block briefs and the seven M4 briefs in foundation
`work/` were re-checked on 2026-08-18: all still **Ready** (none
executed), so every related M3/M4 adoption stays in force and the new
adoptions below join them.

## 1. The request schema names the handler wasm by hash but gives its bytes no home

Platform 14 §14.1.1 models `library_manifests[].compiletime_wasm_sha256`
— the hash of the framework-compiled handler artifact (ADR-0004) — but no
field carries the artifact itself, and CMP-01 forbids obtaining it
anywhere else (no filesystem, no network, no cache lookup). As specified,
pass [6] can *name* the handler it must execute but can never *load* it.

**Local adoption:** a provisional request field
`library_manifests[].compiletime_wasm` — the wasm bytes, base64-encoded
(standard alphabet, padded), optional, hash-checked against
`compiletime_wasm_sha256` at intake (mismatch is `LIB004`, see item 6).
Needs a foundation brief amending §14.1.1 (and the `RQD002`
`deny_unknown_fields` posture makes this a deliberate, versioned schema
change for any producer).

## 2. The compiler↔handler wire ABI is unspecified

Chapter 21 §21.3/§21.4 and `schema/block-ast.md` fix the *typed* contract
(`BlockAST` in, `IR` out, BLK-03 diagnostics) but no document says how
those values cross the wasm boundary: exports, memory discipline,
serialization, diagnostic transport. **Local adoption:** the
implementation-defined ABI of docs/adr/0003-handler-wire-abi.md (core
module; `alloc`/`expand`/`memory` exports; `BlockAST` as JSON with the
schema's field names; `{"ir", "diagnostics"}` envelope out). Needs a
foundation brief so framework and compiler converge on one normative ABI.

## 3. `wasmtime-wasi` is not part of the sandbox

The M5 plan row said "wasmtime+wasmtime-wasi". Implementing BLK-04 showed
the second half contradicts the spec it serves: everything WASI provides
(clocks, randomness, fds, environment) is exactly the `BLOCK006` list,
and chapter 21 §21.7 stubs *all* host imports. ADR-0004's "no wall clock,
seeded randomness" is realized as *absence*: the sandbox links wasmtime
only, provides zero imports, and stubs whatever the artifact declares.
Recorded in docs/adr/0003-handler-wire-abi.md; no spec action needed
(ADR-0004 already reads this way — the plan row was over-specified).

## 4. BLOCK001–BLOCK006 have no literal message templates

Their rule bodies live in chapter 21 §21.6 (declared ERC-02 exception),
which defines conditions and symbolic names but — unlike Platform 10
rules — no message templates. **Local adoption:** local wordings in the
house style, pinned byte-exactly by the DIA-06 triples landed with each
emission (continues the M2–M4 stub-rule convention, DISCOVERIES-M4
item 2). Needs a foundation brief adding templates to §21.6 (or to a
Platform 10 section if the exception is revisited).

## 5. LIB014's per-library limits have no request home

Chapter 21 §21.7 and Platform 07 §7.8 make `library-heap-mb` (512 MiB)
and `max-ir-nodes` (500 000) configurable per project, but the request's
`compile_limits` (Platform 14 §14.1.1) carries only five keys —
`handler_timeout_ms`, `handler_memory_mb`, `total_timeout_min`,
`max_file_size_mb`, `max_import_depth`. The per-library caps therefore
cannot arrive in a request. **Local adoption:** the spec defaults are
hard-coded; heap is accounted as the sum over a library's invocations of
the linear-memory size at the end of each call. Needs a foundation brief
either adding the two keys to `compile_limits` or stating that the
defaults are not per-project-configurable at the compiler boundary.

## 6. Manifest integrity and validity wordings

`LIB004`'s template is `"Library '{path}' has invalid manifest: {reason}"`
— written for a `library.toml` path the compiler never sees (CMP-01: it
receives the lowered manifest). **Local adoption:** `{path}` is filled
with the library's `name`, and the `compiletime_wasm`-vs-hash mismatch of
item 1 is a `LIB004` reason (`"compiletime_wasm does not match
compiletime_wasm_sha256"`) rather than a stretched `RQD001`, whose
registered condition is sources-specific. Pinned by the LIB004 fixture.

## 7. What "explicit library import" means in §21.2 rule 1

Chapter 21 §21.2 resolves block names first by "explicit library import"
(`import data.experimental`), but chapter 17 imports name *modules from
sources[]*, not libraries, and no text connects an import path to a
`library_manifests[]` entry. **Local adoption:** an import whose path (or
its dotted prefix) equals a library manifest's `name` brings that library
into explicit scope for the importing file; sources-module resolution is
tried first, so a name collision between a module and a library is
resolved in the module's favor (chapter 17 owns the surface). Interacts
with the still-Ready import-visibility brief (M4 item 9). Needs the
foundation brief that decides §21.2/chapter-17 layering.

## 8. Folder-scope matching for `request.folders`

Platform 07 §7.6 shows `[folders]` keys like `"app/data/**"`; Platform 14
§14.1.1's example shows keys without globs (`"app/data"`). No text
defines the match rule the compiler applies to `sources[].path`. **Local
adoption:** a key matches a source if, after stripping one trailing
`/**`, it path-prefix-matches on whole segments (`"app/data"` matches
`app/data/User.cln` and `app/data/sub/X.cln`, never `app/database/X.cln`);
keys are compared case-sensitively as POSIX paths. Pinned by the
resolution tests. Needs a one-line ruling in Platform 07.

## 9. Platform 03 §3.8's per-call 16 MiB `mem-alloc` cap is unobservable at the sandbox boundary

"No `mem-alloc` may exceed 16 MiB in a single call" is a rule about the
Clean memory-model allocator *inside* handler code. The sandbox sees only
`memory.grow`; individual `mem-alloc` calls are internal to the artifact
and invisible for hand-written or optimized wasm. **Local adoption:** not
enforced in M5; the 128 MiB store cap is the enforced boundary. Becomes
enforceable if/when handlers are Clean-compiled by this compiler and the
allocator is ours. Worth a clarifying note in Platform 03.

## 11. LIB010's `{function}` slot cannot be filled from the request

Platform 10 §10.3 formats LIB010 as `[LIB010 via {library}::{function}]`,
but `library_manifests[]` (Platform 14 §14.1.1) carries no handler
function names — `handles_blocks` maps block names to one artifact hash.
The sub-label registry (`library.toml [mcp.diagnostics]`) is equally
absent from the request, so the "unregistered sub-label warns at library
load" clause of BLK-03 is unenforceable at the compiler boundary.
**Local adoption:** attribution is `[via {library}::{block}]`, the
kebab-case sub-label rides as the primary label, and no
unregistered-sub-label warning is emitted. Pinned by the LIB010 fixture.
Needs the same §14.1.1 brief as item 1 (manifest surface).

## 12. Pass [5] must be provisional when the program declares blocks

Platform 14 orders type-check ([5]) before expansion ([6]), yet the point
of chapter 21 is that handlers emit companions the user's code refers to
(`class UserData can Persist`, generated functions). A pass [5] that
hard-fails on symbols a handler is about to provide would make every such
program uncompilable. **Local adoption:** when `Item::LibraryBlock`s are
present, pass [5]'s findings are held back (the pass still runs — its
TypedAST is the pass-[6] input the spec names) and the re-validation of
the expanded program inside pass [6] is authoritative; a re-validation
*error* anchored inside an expanded block is reported as `BLOCK004`
(§21.4's malformed-IR case), while errors anchored in user code keep
their own codes and spans. Needs a foundation clarification in 14 §14.4.2
pass [6] (the "TypedAST'" arrow implies this but no text says it).

## 13. Lowerer depth cap (ICE prevention, implementation-defined)

A handler can nest `concat`/`with_span` arbitrarily deep within the
500 000-node budget; a recursive lowerer overflows the compiler's stack
(an ICE, breaching CMP-04) long before the budget fires. **Local
adoption:** IR nesting beyond 128 levels is malformed IR (`BLOCK004`,
"nesting exceeds the depth limit"). No legitimate §21.4 composition
approaches this. Worth a sentence in §21.7's budget list.

## 14. The `ir.field` builder has no visibility parameter

§21.4's `ir.field(string name, TypeRef type)` offers no way to declare an
emitted field public or private, while chapter 14 classes default to
private with `public:` wrappers. A companion type with all-private fields
is unusable. **Local adoption:** handler-emitted fields are public;
emitted functions are module-local (not `public:`-exported). Needs a
foundation ruling on the builder surface.

## 15. BLK-01's checker rules have no diagnostic codes

`grammar/21-block-handlers.ebnf.md` §1 assigns three restrictions to "the
checker": exactly one parameter of type `BlockAST`, `returns IR`
mandatory, and the block-name string being a qualified identifier. No
BLOCK or SEM code covers a violation, and DIA-01 forbids inventing one.
**Local state:** the missing-handler leg of BLK-01 ("must reference a
`compiletime function` defined in the same library") is emitted as
`SEM019`, whose registered template ("I cannot find a function named…")
states exactly that failure; reserved names are `BLOCK003` (LEX-05, at
the declaration, with `project.name` as the library); the arity/return/
name-form violations are currently accepted without a diagnostic. Needs a
foundation brief registering codes for the BLK-01 checker rules.

## 16. Self-hosted handler execution is blocked by the spec, not the milestone

Two independent gaps keep `compiletime function` bodies on the
Unsupported channel (note: "compiletime function bodies") and
`test.compiletime` helpers as a frontier:

- **No pattern-match syntax exists.** `BlockNode`/`BlockArg`/`Token` are
  sum types a handler must "pattern-match on the variant" (schema), but
  the grammar file itself flags (⚠, §2) that no chapter defines a
  `match`/`case` construct. A conforming handler body that walks
  `ast.body` cannot be written under the Accepted grammar, so
  type-checking such bodies has no spec to check against.
- **Execution belongs to a different compilation mode.** ADR-0004 gives
  handler compilation (Clean source → sandbox wasm) to the framework at
  library-install time; the compiler *executes* precompiled artifacts
  from `library_manifests`. Compiling a handler body to the ADR-0003 ABI
  is a distinct target surface (TYP-04 value marshalling, `ir` builder
  runtime, envelope emission) that no platform document specifies.

M5 therefore executes framework-precompiled handlers only. Needs the
pattern-match brief before any of the rest can move.

## 10. Registry inconsistency: LIB020 is both registered and reserved

Platform 09 §3.9 registers `LIB020` (`SourceBlockMalformed`, with a rule
body in Platform 10 §LIB020) while §5 Reserved still lists
"LIB020–LIB099 Reserved". One of the two rows is stale. No compiler
impact (the registry crate carries LIB020 as emittable); needs a
one-line foundation fix to §5 (`LIB021–LIB099`).
