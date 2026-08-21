# 20 state-management — Grammar

Companion grammar file for [20 — State Management](../20-state-management.md). Defines the shape of every state-related construct: the top-level `state:` block, guard clauses on individual declarations, the `rules:` sub-block, `computed:` derived state, `watch:` observers (single-variable and multi-variable), and the `reset` statement (variable or whole-state). Semantic rules SMG-01..SMG-05 live in the companion chapter.

State-block grammar is used by [08-file-structure.ebnf.md](./08-file-structure.ebnf.md) at the top level (`StateSection`). The former screen-scoped state and language-level `screen <Name>:` construct were withdrawn per [ADR-0030](../../01%20governance/decisions/0030-withdraw-screen-from-language.md).

---

## 1. `state:` block

```ebnf
(* SMG-01: state: block.  Contains state variable declarations,
   an optional rules: sub-block, and an optional computed: sub-
   block.  Body order (from chapter examples):
     - state variable declarations (with optional guard clauses)
     - computed: sub-block (optional)
     - rules: sub-block (optional)
   ⚠ The chapter's examples show `rules:` and `computed:` at
   various positions; strict ordering not fixed.  Encoded here
   as any interleaving. Needs review. *)

StateBlock      = "state", ":", NEWLINE, INDENT, StateBody, DEDENT ;

StateBody       = { StateBodyMember } ;

StateBodyMember = StateVariableDeclaration
                | ComputedBlock
                | RulesBlock ;
```

## 2. State variable declaration + guard clauses

```ebnf
(* SMG-01: state variables require initial values (chapter Rules).
   Grammar-wise this is a TypedDeclaration with a required
   initialiser. *)

StateVariableDeclaration = TypeExpression, Identifier, "=", Expression,
                           NEWLINE, [ GuardClauses ] ;

(* SMG-02: guard clauses are indented lines directly beneath a
   declaration.  A declaration may carry more than one — evaluated
   in written order, first-failure-wins semantics. *)

GuardClauses    = INDENT, GuardClause, NEWLINE,
                  { GuardClause, NEWLINE }, DEDENT ;

GuardClause     = "guard", Expression, "else", StringLiteral ;
                  (* Expression MUST be a pure boolean expression
                     (STATE001 semantic check).  Message MUST be a
                     string literal — chapter §Rules requires the
                     `else "<message>"` clause be mandatory. *)
```

## 3. `computed:` sub-block

```ebnf
(* SMG-05: computed: — read-only derived values.  Each entry is
   a typed name with an indented body that returns the computed
   value.  Body may span multiple lines. *)

ComputedBlock   = "computed", ":", NEWLINE, INDENT,
                  ComputedDeclaration, NEWLINE,
                  { ComputedDeclaration, NEWLINE },
                  DEDENT ;

ComputedDeclaration = TypeExpression, Identifier, NEWLINE,
                      INDENT, StatementSequence, DEDENT ;
                      (* The body's return type must match the
                         declared type (SEM018 checker rule). *)
```

## 4. `rules:` sub-block

```ebnf
(* SMG-03: rules: — boolean expressions over the state in scope.
   Each line is one boolean Expression, checked when a function
   that assigned to any variable in the block returns. *)

RulesBlock      = "rules", ":", NEWLINE, INDENT,
                  RuleExpression, NEWLINE,
                  { RuleExpression, NEWLINE },
                  DEDENT ;

RuleExpression  = Expression ;
                  (* Must be boolean — STATE005 checker rule. *)
```

## 5. `watch:` block

```ebnf
(* SMG-04: watch — react to state changes.  Two shapes:
     watch fieldName:       (single variable)
     watch (a, b, c):       (multiple variables) *)

WatchBlock      = "watch", WatchTarget, ":", NEWLINE,
                  INDENT, StatementSequence, DEDENT ;

WatchTarget     = Identifier
                | "(", Identifier, { ",", Identifier }, ")" ;
```

## 6. `reset` statement

```ebnf
(* Chapter §State Reset:
     reset fieldName    — reset one variable to its initial value
     reset state        — reset all state in scope
   `reset` is a hard keyword (LEX-04). *)

ResetStatement  = "reset", ResetTarget ;

ResetTarget     = "state"
                | Identifier ;
                  (* `state` here is the contextual "reset all"
                     literal.  It clashes with `state` as a
                     contextual keyword (LEX-04) but only in
                     value position after `reset`; the parser
                     recognises `reset state` as one form. *)
```

## 7. `WatchBody` alias (referenced from other files)

```ebnf
WatchBody       = StatementSequence ;
```

## 8. `StateBody` alias (referenced from 08-file-structure.ebnf.md)

The `StateBody` production defined in §1 above is the same one 08-file-structure.ebnf.md references via `StateSection`.

---

## Changelog

- 2026-08-07 (afternoon, third pass) — `ScreenBlock`, `ScreenBody`, `ScreenBodyMember` productions removed per [ADR-0030](../../01%20governance/decisions/0030-withdraw-screen-from-language.md). The language-level `screen <Name>:` construct is withdrawn and `screen` is not a keyword of any kind; the ui library does not register it either. UI-local scoping is an application-level concern, not a language or framework construct. Sections renumbered (former §7 → §6, §8 → §7, §9 → §8). Also resolved the §1 `⚠` marker on StateBody ordering — no strict ordering imposed, StateBodyMember alternatives may interleave freely (matches how the chapter presents examples).
- 2026-08-07 — File minted. Productions derived from SMG-01..SMG-05 in [20-state-management.md](../20-state-management.md) Accepted 2026-08-01.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Parser implementers; anyone writing state, guards, rules, computed, watches
- **Notation:** EBNF (ISO/IEC 14977)
- **Part of:** [04 language / grammar / README.md](./README.md)
- **Rules referencing this grammar:** [20-state-management.md](../20-state-management.md) (SMG-01..SMG-05)
- **References:** [03-lexical-structure.ebnf.md](./03-lexical-structure.ebnf.md), [04-type-system.ebnf.md](./04-type-system.ebnf.md), [06-expressions.ebnf.md](./06-expressions.ebnf.md), [07-statements.ebnf.md](./07-statements.ebnf.md) (StatementSequence), [09-functions.ebnf.md](./09-functions.ebnf.md) (FunctionsBlock), [08-file-structure.ebnf.md](./08-file-structure.ebnf.md) (StateSection)
