# Discoveries — Milestone 4 (type system + resolver)

Spec gaps and under-specifications found while implementing M4. Each item
either becomes a task brief in foundation `work/` (written from a
foundation session — this repo never writes there) or records a local
adoption that stays in force until foundation resolves it.

## Status

Nothing taken to foundation yet; all items open, adoptions local.

## 1. TYP-06's lossy-promotion warning has no registered code

`04 language/04-type-system.md` §TYP-06: the implicit `integer` → `number`
conversion "carries a compile-time warning when the compiler can see the
value may exceed 2⁵³". DIA-01 requires every diagnostic to carry a
registered code, and Platform 09 registers no warning code for a lossy
promotion — so the warning as specified cannot be emitted without
inventing a code.

**Local adoption:** the promotion is implemented (folded for literals,
`IntToNumber` coercion otherwise); the warning is not emitted. Needs a
foundation brief registering a code (or withdrawing the sentence).

## 2. Stub rules: local wording pinned by DIA-06 fixtures (continues the M2/M3 convention)

Platform 10 (RUL-03) still carries one-line stubs — no message template —
for most of the M4 rule families (SEM003/004/006–014, SCOPE001–004,
FUNC001–012, CLASS001–006, IDX001–003, IMPORT002–004). A stub is a spec
defect; the compiler still has to emit. As with SEM003/SEM004 in M2, each
stub rule this milestone implements gets locally-adopted wording in the
spirit of the neighbouring templated rules, pinned byte-exactly by its
DIA-06 fixture triple. Adopted this milestone so far:

- **SEM004** — per-operator wording `` operator `<op>` is not defined for
  type `<T>` `` (replaces M1's separate "arithmetic is not defined"
  variant), plus context wordings for non-iterable sources and
  non-textable interpolation/print operands.
- **SEM009** — three wordings: repeated `?` ("absence does not stack"),
  integer widths outside host signatures, and invalid TYP-05 behavior
  chains (second removal discipline / repeated `.unique`).
- **IDX001/IDX002** — "list/matrix index must be `integer`, found `<T>`".
- **IDX003** — `` `pairs<K, V>` is indexed with `<K>`, found `<T>` ``.

The wording moves to Platform 10 verbatim (or gets replaced) when the
stubs are upgraded; either way the fixtures pin today's behaviour.

## 3. Chapter 07's print example contradicts chapter 06's `+` table

`04 language/07-statements.md` §Console shows

```clean
print:
	"User: " + username
	"Score: " + score
```

with `score` reading as a number, while `06-expressions.md` §Operators on
built-in types (2026-08-02, ADR-0018) fixes `+` on **two strings**, on
`integer`/`number`, and on matrices — there is no `string + integer`
overload and no implicit conversion to string (TYP-06 names integer →
number as the only implicit conversion).

**Local adoption:** `+` follows the chapter-06 table; `"Score: " + score`
with an integer `score` is SEM004. The chapter-07 example needs an
erratum (interpolation `"Score: {score}"` is the supported spelling).

## 4. Duplicate `start:` blocks have no registered code

FNC-01: `start:` is "one per file". The parser accepts any number of
`Item::Start` blocks (the grammar has no cardinality), and no SEM/FUNC
code covers "more than one start block" (FUNC006/007 cover parameters and
return type). M4's resolver records every `start:` and the checker checks
each as a parameterless void body; no diagnostic is emitted for
duplicates.

Needs either a registered code or an explicit statement of which existing
code owns the violation.

## 5. `iterate … step` over non-range sources is unspecified

The grammar (`12-control-flow.ebnf.md`) allows `step` on any `iterate`
source; FLW-02's prose and examples only define `step` for ranges
(`1 to 10 step 2`). What `step` means over a list, string, or matrix
source is unstated.

**Local adoption:** the checker types `step` as `integer` wherever it
appears and preserves it in the TIR; semantics for non-range sources stay
open until foundation rules.

## 6. MOD-01's worked examples contradict MOD-02

Every multi-file example in `17-modules-and-imports.md` §MOD-01 (and the
§Example project: `utils.cln` / `mathHelpers.cln`) imports a module and
calls functions that are **not** inside a `public:` wrapper — under MOD-02
("private by default") every one of those calls is an IMPORT003/SEM019.
The compiler enforces MOD-02 as minted; the chapter's examples need
`public:` wrappers (erratum).

## 7. How a class is exported is unspecified

MOD-02 says "functions and classes are module-local unless declared
inside a `public:` block", and the grammar's `PublicDeclaration` union
admits `ClassOrCapabilityDeclaration` — but `public:` "is NOT a top-level
section itself" and classes only appear at the top level, so there is no
legal position that exports a class. The chapter prose also shows a
top-level `public:` wrapping a `functions:` block, which the grammar
forbids; per DOC-15 the parser follows the grammar.

**Local adoption:** classes are exported by default (visible wherever the
module is imported); `public:` wrappers govern functions (inside
`functions:`) and fields (inside a class body) exactly as the grammar
places them. Needs a foundation brief pinning the class-export surface.

## 8. Module-name → file mapping is framework territory the compiler must approximate

MOD-03: the framework resolves names to files before the compiler runs,
and the compiler "follows import statements across `sources[]`" — but a
request carries no name→file mapping, so the compiler must re-derive it.

**Local adoption (purely textual, deterministic):** a dotted name maps to
`<dots-as-slashes>.cln` and resolves in order — relative to the importing
file's directory, then from the request root, then by unique `…/name.cln`
suffix anywhere in `sources[]`; an ambiguous suffix resolves to nothing
(IMPORT002). The request schema should eventually carry the framework's
resolution (new field) or bless this derivation.

## 9. Boundary and scope adoptions

- **Host interfaces are request-global** (no import needed, no `public:`):
  they describe the world boundary (ADR-0002), which is a request-level
  property, not a module-level one.
- **Imports are non-transitive** for name visibility (chained file imports
  still link everything into the artifact, ch. 17 §Chained Imports).
- **Shadowing:** a locally declared name wins over an imported one; two
  imports bringing the same name keep the first, in entry order. No code
  exists for import-name collisions; needs foundation wording.
- **SEM003 is module-scoped** for functions/classes (two modules may each
  declare a private `helper`); host-function names still clash globally.
