# 14 classes-and-objects — Grammar

**Title:** 14 classes-and-objects — Grammar

**Id:** `CLSG-`

**Purpose:** Companion grammar file for 14 — Classes and Objects, written for the implementors of the parser and for readers of the language chapter who need the exact shape behind its examples. It defines the two declarations that stand at the top level of a file and that 08-file-structure.lark.md references as `class_or_capability_declaration`: the class declaration — its header with the `is` inheritance clause and the `can` capability claim in a fixed order, and its body with fields, the `always:` block, constructors and the `functions:` block in a fixed order — and the capability declaration, a `can Name:` block of method signatures without bodies. It also settles what the language chapter leaves open at the level of syntax — that constructors may be overloaded, that `this` and `base` are hard keywords standing where an identifier stands, and that companion access on a class name is ordinary member access needing no production of its own — and names which productions it borrows from the type-system, functions, expressions and modules grammar files. The semantic rules of classes, inheritance, capabilities and companions live in the language chapter (`CLS-`), not here.

---

### CLSG-01 — 1. Class declaration

**Id:** CLSG-01

**Kind:** rule

**Name:** 1. Class declaration

**Question:** What written shape does a class declaration's header take, and when both the `is` inheritance clause and the `can` capability claim are present, in which order must they stand?

**Norm:**

```lark
// CLS-01: class definition.
// CLS-02: optional inheritance via `is Parent`.
// CLS-03: optional capability claim via `can C1, C2, ...`.
// Order (CLS-03 prose): "class Name [is Parent] [can C1, C2, ...]" —
// `is` clause before `can` clause when both present.

class_declaration: _CLASS IDENTIFIER inheritance_clause? capability_claim_clause? _NEWLINE _INDENT class_body _DEDENT

// Single inheritance per CLS-02.  The parent name is any IDENTIFIER
// resolving to a class.

inheritance_clause: _IS IDENTIFIER

capability_claim_clause: _CAN capability_name ("," capability_name)*

// CLS-03 stylistic convention: capability names are bare verbs — Draw,
// Print, Serialize.  Not enforced by grammar.

capability_name: IDENTIFIER
```

**Why:** the header carries two optional clauses and CLS-03's prose fixes their order — `is` before `can` — so the production spells that order out instead of admitting both; what is convention, bare-verb capability names, is marked as such and kept out of the grammar.

---

### CLSG-02 — 2. Class body

**Id:** CLSG-02

**Kind:** rule

**Name:** 2. Class body

**Question:** In which fixed order must a class body's field declarations, `always:` block, constructors and `functions:` block stand, and does the `public:` wrapper hold a position in that ordering?

**Parent:** LANG-07

**Norm:**

```lark
// Order within the body is FIXED per CTR-03 (always after fields) and
// per the chapter's shown examples (Point, BankAccount, User):
//   1. Field declarations           (zero or more)
//   2. always: block                (optional, at most one)
//   3. Constructors                 (zero or more — overloading allowed)
//   4. functions: block             (optional)
// public_wrapper (17-modules-and-imports.lark.md) may appear at any
// position — it is a section-scoping construct, not a body member: it
// marks the visibility of the members it contains, not their position
// in the ordering, so it may stand before, between or after the parts
// above, zero or more times.  A class with `functions:` before
// `constructor` looks wrong to a reader; enforcing the order at
// grammar level gives a clean parse error and matches the intended
// house style.  Constructor overloading (multiple constructors
// distinguished by parameter list) is permitted; the type checker
// resolves the call site against the parameter-list arities and types.

class_body: public_wrapper* _field_part* _always_part? _constructor_part* _functions_part?

_field_part: field_declaration public_wrapper*
_always_part: always_block public_wrapper*
_constructor_part: constructor public_wrapper*
_functions_part: functions_block public_wrapper*

// typed_declaration from 04-type-system.lark.md.  A field may or may
// not have an initialiser.

field_declaration: typed_declaration _NEWLINE
```

**Why:** a class with `functions:` before its constructor looks wrong to a reader, so the order the chapter's examples show is enforced by the grammar, where the failure is a clean parse error; `public:` stays outside the ordering because it scopes visibility and is not a member.

---

### CLSG-03 — 3. Constructor

**Id:** CLSG-03

**Kind:** rule

**Name:** 3. Constructor

**Question:** May a class declare more than one constructor, told apart by its parameter list, and does the parent-constructor call `base(...)` need a production of its own?

**Norm:**

```lark
// CLS-01 shows constructor with parameters and body but does not
// explicitly address overloading.  Multiple constructors per class are
// ALLOWED, distinguished by parameter list.  The type checker resolves
// call sites against arity and parameter types.  This is a
// language-design choice that follows Java/C# convention; the
// alternative (single constructor + factory functions on the companion
// per CLS-05) remains available for classes that prefer it, but the
// language does not restrict either style.

constructor: _CONSTRUCTOR "(" parameter_list? ")" block

// The `base(...)` call for parent constructor invocation (CLS-02) is a
// call whose callee is the hard keyword `base` (CLSG-05).
// Grammatically it participates in the expression grammar; no
// dedicated rule needed.
```

**Why:** the chapter shows a constructor but never addresses overloading, so the choice is settled here — several constructors told apart by parameter list, resolved by the type checker — and `base(...)` needs no production because it is a call whose callee is a hard keyword.

---

### CLSG-04 — 4. Capability declaration

**Id:** CLSG-04

**Kind:** rule

**Name:** 4. Capability declaration

**Question:** Which lines may stand inside a `can Name:` capability declaration block, and is a body written on one of them refused by the grammar or by the checker?

**Norm:**

```lark
// CLS-03: `can Name:` block declares a capability — a named contract
// of method signatures.  Bodies inside a `can:` block are SEM014
// (prohibited).  Signatures use arrow-return syntax per FNC-03.

capability_declaration: _CAN capability_name ":" _NEWLINE _INDENT capability_item+ _DEDENT

// capability_method_signature is defined in 09-functions.lark.md:
//     IDENTIFIER "(" parameter_list? ")" "->" return_type
// A body under a signature is read so the checker can name it: SEM014.
capability_item: capability_method_signature _NEWLINE
               | capability_method_signature _NEWLINE _INDENT function_body _DEDENT  -> capability_item_with_body
```

**Why:** a capability is a contract of signatures without bodies, so the block admits only `CapabilityMethodSignature` lines and borrows that production from 09-functions.lark.md rather than restating it; a body that appears is the checker's SEM014, not a parse error.

---

### CLSG-05 — 5. `this` and `base` — reserved names in class scope

**Id:** CLSG-05

**Kind:** rule

**Name:** 5. `this` and `base` — reserved names in class scope

**Question:** Do the hard keywords `this` and `base` get productions of their own in the expression grammar, or do they stand where an `Identifier` stands and get recognised by class-method context?

**Norm:**

```lark
// CLS prose: `this` is available inside all class methods.  `base` is
// used to call the parent constructor.  Both are hard keywords per
// LEX-04, so neither ever arrives as an IDENTIFIER; they stand in
// primary position, where an identifier stands, and that is all the
// grammar says of them.  That they are meaningful only inside a class
// method is the checker's, not the grammar's.

%extend primary_expression: _THIS  -> this_reference
                          | _BASE  -> base_reference
```

**Why:** both names are hard keywords, yet they stand where an Identifier stands, so treating them as ordinary Primary operands recognised by context spares the expression grammar two productions that would only duplicate `Identifier`.

---

### CLSG-06 — 6. Companion access (CLS-05)

**Id:** CLSG-06

**Kind:** rule

**Name:** 6. Companion access (CLS-05)

**Question:** Does companion access on a class name, `Outer.field`, take a production of its own, or is it the ordinary `.Identifier` member-access postfix with the distinction left to name resolution?

**Norm:**

```lark
// CLS-05: field access on a class NAME (not an instance) resolves to
// the field's TYPE used as a namespace.  Grammatically the syntax
// `Outer.field` is indistinguishable from ordinary member access — the
// same member_access postfix (in 06-expressions.lark.md).  The
// distinction is resolved by the type checker at name resolution: if
// the left of `.` is a class name (not an instance), the right resolves
// against the field's type as a namespace.
//
// No grammar rule is needed here — companion access piggy-backs on
// ordinary member_access.  Its semantics are CLS-05's job.
```

**Why:** `Outer.field` is the same `.Identifier` postfix as any member access, so a dedicated production would only duplicate it; the distinction — a class name on the left resolving against the field's type as a namespace — is name resolution and stays CLS-05's job.

---
