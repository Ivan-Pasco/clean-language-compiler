# 14 classes-and-objects — Grammar

Companion grammar file for [14 — Classes and Objects](../14-classes-and-objects.md). Defines the shape of class declarations (fields, constructor, methods, `always:` invariant), the `is` inheritance clause, capability declarations (`can` blocks — contracts without bodies), capability claim clauses (`can C1, C2`), and companion access. Semantic rules CLS-01..CLS-05 live in the companion chapter.

Two related declarations live at the top level and are defined here:
- `class Name` — an ordinary class declaration (`ClassDeclaration`)
- `can Name:` — a capability declaration (`CapabilityDeclaration`)

Both are referenced from [08-file-structure.ebnf.md](./08-file-structure.ebnf.md) as `ClassOrCapabilityDeclaration`.

---

## 1. Class declaration

```ebnf
(* CLS-01: class definition.
   CLS-02: optional inheritance via `is Parent`.
   CLS-03: optional capability claim via `can C1, C2, ...`.
   Order (CLS-03 prose): "class Name [is Parent] [can C1, C2, ...]"
   — `is` clause before `can` clause when both present. *)

ClassDeclaration = "class", Identifier,
                   [ InheritanceClause ],
                   [ CapabilityClaimClause ],
                   NEWLINE, INDENT, ClassBody, DEDENT ;

InheritanceClause = "is", Identifier ;
                    (* Single inheritance per CLS-02.  Parent name
                       is any Identifier resolving to a class. *)

CapabilityClaimClause = "can", CapabilityName, { ",", CapabilityName } ;

CapabilityName  = Identifier ;
                    (* CLS-03 stylistic convention: capability names
                       are bare verbs — Draw, Print, Serialize.
                       Not enforced by grammar. *)
```

## 2. Class body

```ebnf
(* Order within the body is FIXED per CTR-03 (always after fields)
   and per the chapter's shown examples (Point, BankAccount, User):
     1. Field declarations           (zero or more)
     2. always: block                (optional, at most one)
     3. Constructors                 (zero or more — overloading allowed)
     4. functions: block             (optional)
     PublicWrapper may appear at any position — it is a section-
     scoping construct, not a body member.
   A class with `functions:` before `constructor` looks wrong to a
   reader; enforcing the order at grammar level gives a clean parse
   error and matches the intended house style.  Constructor
   overloading (multiple constructors distinguished by parameter
   list) is permitted; the type checker resolves the call site
   against the parameter-list arities and types. *)

ClassBody       = { FieldDeclaration },
                  [ AlwaysBlock ],
                  { Constructor },
                  [ FunctionsBlock ],
                  { PublicWrapper } ;
                  (* PublicWrapper — grammar in
                     17-modules-and-imports.ebnf.md.  Appears zero
                     or more times, interleavable at any position;
                     the wrapper marks visibility of the members it
                     contains, not their position in the ordering. *)

FieldDeclaration = TypedDeclaration, NEWLINE ;
                   (* TypedDeclaration from 04-type-system.ebnf.md.
                      A field may or may not have an initialiser. *)
```

## 3. Constructor

```ebnf
(* CLS-01 shows constructor with parameters and body but does not
   explicitly address overloading.  Multiple constructors per class
   are ALLOWED, distinguished by parameter list.  The type checker
   resolves call sites against arity and parameter types.
   This is a language-design choice that follows Java/C# convention;
   the alternative (single constructor + factory functions on the
   companion per CLS-05) remains available for classes that prefer
   it, but the language does not restrict either style. *)

Constructor     = "constructor", "(", [ ParameterList ], ")",
                  NEWLINE, INDENT, StatementSequence, DEDENT ;

(* The `base(...)` call for parent constructor invocation
   (CLS-02) is a Call whose callee is the hard keyword `base`.
   Grammatically it participates in the expression grammar; no
   dedicated production needed. *)
```

## 4. Capability declaration

```ebnf
(* CLS-03: `can Name:` block declares a capability — a named
   contract of method signatures.  Bodies inside a `can:` block
   are SEM014 (prohibited).  Signatures use arrow-return syntax
   per FNC-03. *)

CapabilityDeclaration = "can", CapabilityName, ":",
                        NEWLINE, INDENT,
                        CapabilityMethodSignature, NEWLINE,
                        { CapabilityMethodSignature, NEWLINE },
                        DEDENT ;

(* CapabilityMethodSignature is defined in 09-functions.ebnf.md:
     Identifier, "(", [ ParameterList ], ")", "->", ReturnType
   No body — the checker enforces SEM014 if one appears. *)
```

## 5. `this` and `base` — reserved names in class scope

```ebnf
(* CLS prose: `this` is available inside all class methods.
   `base` is used to call the parent constructor.  Both are hard
   keywords per LEX-04.  Grammatically they appear in Primary
   position (as Identifier alternatives) inside class-method
   bodies.  No new productions needed here — they participate in
   the expression grammar as ordinary Identifiers, and the parser
   recognises them when they appear in class-method context. *)
```

## 6. Companion access (CLS-05)

```ebnf
(* CLS-05: field access on a class NAME (not an instance) resolves
   to the field's TYPE used as a namespace.  Grammatically the
   syntax `Outer.field` is indistinguishable from ordinary member
   access — same `.Identifier` postfix operator (in
   06-expressions.ebnf.md).  The distinction is resolved by the
   type checker at name resolution: if the LHS of `.` is a class
   name (not an instance), the RHS resolves against the field's
   type as a namespace.

   No grammar production is needed here — companion access piggy-
   backs on ordinary MemberAccess.  Its semantics are CLS-05's job. *)
```

---

## Changelog

- 2026-08-07 (afternoon, second pass) — Resolved the second `⚠` marker: constructor overloading is ALLOWED — multiple constructors per class, distinguished by parameter list, resolved by the type checker. Follows Java/C# convention. Production change: `[ Constructor ]` → `{ Constructor }` in the ClassBody sequence.
- 2026-08-07 (afternoon) — Resolved the first `⚠` marker: `ClassBody` ordering is now STRICT — fields → always → constructor → functions → public wrappers (any position). Every chapter example follows this order; a class with `functions:` before `constructor` looks wrong to a reader. Production change: replaced `ClassBody = { FieldDeclaration }, [ AlwaysBlock ], { ClassBodyMember }` with a fixed sequence; `ClassBodyMember` production removed as unused.
- 2026-08-07 — File minted. Productions derived from CLS-01..CLS-05 in [14-classes-and-objects.md](../14-classes-and-objects.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; anyone declaring classes or capabilities
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [14-classes-and-objects.md](../14-classes-and-objects.md) (CLS-01..CLS-05)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [04-type-system.ebnf.md](./04-type-system.ebnf.md) (TypedDeclaration), [06-expressions.ebnf.md](./06-expressions.ebnf.md), [09-functions.ebnf.md](./09-functions.ebnf.md) (CapabilityMethodSignature, ParameterList, FunctionsBlock), [10-contracts.ebnf.md](./10-contracts.ebnf.md) (AlwaysBlock), forward reference to [17-modules-and-imports.ebnf.md](./17-modules-and-imports.ebnf.md) (PublicWrapper)
