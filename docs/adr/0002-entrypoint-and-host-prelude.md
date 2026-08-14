# ADR 0002 — Milestone 1 conventions: entry points, host-interface resolution, boundary type projection

- **Status:** Accepted (2026-08-14) — scoped to Milestone 1; every decision here is written to be superseded by framework/foundation specs and is mirrored into the Discoveries section of the milestone brief.

## Context

The Milestone 1 acceptance guest is an *application* compiled without Clean
Framework: the request document is assembled by hand, and no `clean:library/*`
synthesis happens. Three questions the Accepted specs leave to the framework
(or do not yet answer) must be fixed locally to proceed:

1. How the guest's `init` / `handle` exports come into existence (route
   discovery by folder convention is framework territory, CCMP-13).
2. How a `host interface` declaration relates to the interfaces the target
   world already declares (LBS-02 synthesizes `clean:library/<name>` WIT for
   libraries — but a guest importing `clean:host/*` must match the *host's*
   contract, not synthesize its own).
3. How Clean types project onto the WIT types `clean-server/host.wit` actually
   uses. The LBS-02 mapping covers signed widths (`integer:32` → `s32`),
   `bytes`, `T?`, and class-with-fields → record, but is silent on unsigned
   widths (`u8`/`u16`/`u32`/`u64`), enums, and referencing types the world
   declares (`method`, `options`, `level`, `field`).

## Decisions

1. **Entry points.** A Milestone 1 guest defines ordinary functions named
   `init` (no parameters, void) and `handle` (one integer parameter, void)
   inside `functions:`. The compiler exports them under the names the target
   world imports. No folder discovery, no `start:` involvement.

2. **Host-interface resolution.** A `host interface <name>` block (LBS-02
   grammar, unchanged) whose kebab-case name equals an interface exported by
   the target world resolves to that world interface: call sites import
   `clean:host/<name>@<version>` — never a synthesized `clean:library/*`
   package. The declared Clean signature must project (per decision 3) onto
   the WIT signature the world declares, field for field; a mismatch is a
   compile error at the declaration site. The `version "x.y.z"` and
   `requires host worlds [...]` clauses are parsed and checked for shape but
   the version authority is the world in the request (ADR-0033).

3. **Type projection extensions** (additions to the LBS-02 table, following
   its own conventions):

   | Clean type (host-function position only) | WIT type |
   |---|---|
   | `integer:u8` / `integer:u16` / `integer:u32` / `integer:u64` | `u8` / `u16` / `u32` / `u64` |
   | identifier naming a type declared by the same world interface (`method`, `options`, `level`, `field`) | that WIT type |

   At call sites:
   - An **enum-typed** parameter accepts a compile-time string literal naming
     a case (`"get"`, `"info"`); the compiler lowers it to the case's
     discriminant and rejects unknown case names at compile time.
   - A **record-typed** parameter accepts an instance of a Clean class whose
     kebab-cased name and typed fields structurally match the WIT record
     (the LBS class↔record projection read in the other direction).
   - Range checks at the boundary (LBS-02: "the compiler checks a value's
     range as it crosses the boundary") apply to every width-suffixed
     integer.

## Consequences

- The acceptance guest's `host_bridge.cln` is honest, typed Clean source
  under Accepted grammar; nothing about the *language* surface is invented —
  only the projection table grows, in the direction the framework spec
  already points.
- When Clean Framework lands its own host-prelude generation, decisions 1–2
  become its job; this ADR then records history, not behaviour.
- The projection extensions are reported back to foundation via the brief's
  Discoveries section as a candidate LBS-02 amendment.
