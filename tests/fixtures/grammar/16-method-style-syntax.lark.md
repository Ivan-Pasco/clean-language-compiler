# 16 method-style-syntax — Grammar

**Title:** 16 method-style-syntax — Grammar

**Id:** `CALG-`

**Purpose:** Companion grammar file for 16 — Method-Style Syntax. That chapter's single semantic rule (CALL-01) is a convention for which existing call shape applies to which operation — method style, namespace style or operator form — and introduces no syntactic form of its own, so this file defines no production. It exists to satisfy the companion-file requirement of DOC-15 and to say so explicitly: it names the home of each call shape in `06-expressions.lark.md` and states that the method/namespace distinction is semantic and conventional, not syntactic, so that a future author looking for a `MethodCall` production knows where to find the shape and why none lives here. It is written for authors and implementors of the grammar, and rests on 16 — Method-Style Syntax and 06-expressions.lark.md.

---

### CALG-01 — Call shapes and their homes

**Id:** CALG-01

**Kind:** rule

**Name:** Call shapes and their homes

**Question:** In which grammar file and under which production does each call shape — method style, namespace style, operator form — live, and does this companion define a production of its own for them?

**Norm:**

| Shape | Home |
|---|---|
| Method style — `value.operation(args)` | 06-expressions.lark.md — `postfix_expression`, its `member_access` alternative then its `call` alternative |
| Namespace style — `module.operation(a, b)` | 06-expressions.lark.md — same `member_access` + `call` pattern, distinguished only by the LHS being a namespace name |
| Operator form — `a + b`, `a == b`, `a and b` | 06-expressions.lark.md — the precedence ladder |

There is no CALL-specific grammar production. This file exists to satisfy DOC-15's companion-file requirement and to explicitly state "no new productions" so a future author looking for a `MethodCall` production knows to find it in `06-expressions.lark.md` under `postfix_expression`.

**Why:** the chapter's one rule chooses among call shapes that other grammar files already define, so this companion points at each shape's home instead of restating a production; a `MethodCall` production here would be a second definition of `PostfixExpression`.

---

### CALG-02 — 1. Nothing to define here

**Id:** CALG-02

**Kind:** rule

**Name:** 1. Nothing to define here

**Question:** Is the difference between a method-style call and a namespace-style call a difference of grammar, or a semantic and conventional one that leaves the two shapes syntactically identical?

**Parent:** LANG-04

**Norm:**

CALL-01 is a call-site *choice* rule enforced by the language's `LDR-08` "one way to do things" principle plus the type system. Grammar-wise, `text.length()` and `math.max(10, 20)` are identical shapes — both are `postfix_expression "." IDENTIFIER "(" argument_list? ")"`. The distinction between method-style and namespace-style is:

- **Semantic**, not syntactic — the LHS being an instance vs. a module.
- **Convention**, not enforcement — CALL-01's "one name" rule is a style guide backed by the standard library's design (no aliases exported).

Nothing is left open here — nothing is under-specified because nothing is defined here.

**Why:** a companion that exists only to satisfy DOC-15 must say so explicitly, or a future author looking for a `MethodCall` production would read its absence as an omission; the section states that the method/namespace distinction is semantic and conventional, not syntactic.

---
