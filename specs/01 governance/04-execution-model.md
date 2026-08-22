# Execution Model — Clean Language Ecosystem

The ladder of intent — governance to spec to ADR to task brief to code — is a design; this document is how that design is executed by real sessions in real repositories. It defines who does each step (the actors), what a session running that step is allowed to see (working-directory scope), where the resulting work lives (which repo), and which two points in the loop still require a human to say yes. Wherever the design could be enforced by controlling access rather than trusting a prompt, this document does that: the reading rules become physical constraints, not instructions the agent has to remember.

---

## Part 0 — Purpose

The [Documentation Principles](00-documentation-principles.md) define the document system and [Spec-Driven Design](03-spec-driven-design.md) defines what fidelity means. Both speak of actors — "the agent writing code," "the test-writing agent" — without defining them. This document defines them: who performs each rung transition, in what kind of session, in which repository, and behind which gates.

The ecosystem is multi-repository: this foundation repo holds governance and specifications; sibling component repositories (compiler, tooling, etc.) hold code. The execution model spans both.

---

## Part 1 — Actors

### EXE-01 — Every rung transition has exactly one actor

*(Addresses: C-25)*

Work moves down the ladder ([DOC-07](00-documentation-principles.md)) through four defined actors. Any work product of a type listed below that was not produced by its actor, under its listed inputs, is a defect.

| Actor | Reads (nothing else) | Produces | Exit gate |
|-------|----------------------|----------|-----------|
| **Spec Author** | Governance, Accepted ADRs, the document being written | Spec chapters, ADRs (`Draft` → `Proposed`) | Human sets `Accepted` (EXE-08) |
| **Brief Writer** | Accepted specs and ADRs, the target repo's code layout | Task brief (`Draft`) in the target repo's `work/` | Human sets `Ready` (EXE-08) |
| **Implementer** | One `Ready` brief, the code it touches | Code and tests citing rule IDs ([SDD-11](03-spec-driven-design.md)) | Brief's acceptance checks pass; brief set `Done` and archived |
| **Reviewer** | The work product under review, one rung of context above it | Approval, or defects reported | — |

- Test authoring is not a fifth actor: per [SDD-12](03-spec-driven-design.md), conformance tests are produced by an Implementer executing a test-authoring brief that cites rule IDs, not source files.
- The Reviewer at the two mandatory gates (EXE-08) MUST be human. Agent review MAY supplement it, never replace it.

**Why:** [SDD-06](03-spec-driven-design.md): the parties decouple only if each reads the contract, not each other. Naming the actors is what makes "who may read what" checkable.

### EXE-02 — One session, one actor

*(Addresses: C-25)*

An agent session MUST perform the work of exactly one actor. Changing actors — e.g. from writing a brief to implementing it — requires a new session.

**Why:** A session that both wrote the brief and implements it carries the spec in its context, and the brief stops being the interface ([DOC-07](00-documentation-principles.md) rule 1 is void). The gate between rungs only gates something if the rungs are separate sessions.

---

## Part 2 — Sessions

### EXE-03 — Session scope equals actor inputs

*(Addresses: C-25, C-23)*

A session MUST be opened with working-directory access equal to its actor's *Reads* column, and no more:

| Actor | Session working directories |
|-------|-----------------------------|
| Spec Author | `clean-language-foundation/` only |
| Brief Writer | The component repo, plus `clean-language-foundation/` as an additional directory |
| Implementer | The component repo only — foundation MUST NOT be accessible |
| Reviewer | Per the work product under review |

- An Implementer session with the foundation repo in scope is a defect, even if the agent never opens a spec file.
- Sessions covering the parent folder (all repos at once) are for human navigation and cross-repo audits only; no actor's work may be produced in one.

**Why:** [DOC-07](00-documentation-principles.md)'s reading rules, enforced physically. An Implementer that cannot see the spec cannot improvise from it; its only move on an insufficient brief is the correct one — stop and report one rung up.

### EXE-04 — A brief lives in the repository it changes

*(Addresses: C-25, C-23)*

A task brief MUST be created in the `work/` folder of the repository whose files appear in its *Files touched* section, and is archived to that repository's `work/archive/` per [DOC-12](00-documentation-principles.md). The foundation's `work/` holds only briefs that change foundation documents.

- Briefs in component repos cite foundation rules as plain-text qualified IDs (`clean-language-foundation LEX-01`), per [Documentation Principles §2.3](00-documentation-principles.md#23-citing-rules-from-other-documents).
- A brief whose *Files touched* spans more than one repository is a defect: split it, one brief per repository.

**Why:** Keeps every Implementer session self-contained in one repo (EXE-03), and keeps the brief and the diff it authorizes in the same git history.

---

## Part 3 — Repository mechanisms

### EXE-05 — Each component repo carries its operating rules

*(Addresses: C-23)*

Every component repository MUST contain a `CLAUDE.md` (or equivalent agent-instructions file) that states the Implementer's operating rules by citing `EXE-` and `SDD-` IDs. It MUST NOT restate the rules ([DOC-14](00-documentation-principles.md#doc-14--cite-by-id-and-range-not-by-anchor-link-soup)); the gloss-plus-ID citation form applies.

**Why:** The Implementer session cannot see the foundation (EXE-03), so the pointer to the law must travel with the repo. IDs are grep-able across repos; restated rules fork.

### EXE-06 — Actors are invoked as skills

*(Addresses: C-23, C-25)*

The Brief Writer and Implementer actors are realized as named skills, so invoking the actor is one deterministic command rather than an improvised prompt:

| Skill | Actor | Refuses to run unless |
|-------|-------|-----------------------|
| `/write-brief <spec sections or rule IDs>` | Brief Writer | Every cited input document is `Accepted` ([DOC-09](00-documentation-principles.md)) |
| `/implement-brief <brief path>` | Implementer | The brief's status is `Ready` |

A skill's refusal message names the failing precondition and the rule requiring it.

**Why:** [DOC-06](00-documentation-principles.md) applied to prompts: fixed invocation, fixed preconditions, deterministic behavior. The precondition checks are the status gate ([DOC-09](00-documentation-principles.md)) made mechanical.

### EXE-07 — Enforcement is mechanical

*(Addresses: C-26, C-24)*

The following checks enforce this document. Per [DOC-13](00-documentation-principles.md) they are stated as if they exist; until built, they are the Reviewer's checklist, and building them is standing work under the [Quality Playbook](02-quality-playbook.md).

- **Files-touched check:** a commit implementing a brief whose diff contains a file not listed in the brief's *Files touched* fails. (The brief is fixed first, per its template.)
- **Status-gate check:** `/write-brief` and `/implement-brief` preconditions (EXE-06), enforced by the skill itself and re-checked in review.
- **Parity check:** every normative rule ID has at least one citing test — this is [SDD-11](03-spec-driven-design.md), run in each component repo's CI.
- **Brief-lifecycle check:** a brief marked `Done` outside `work/archive/`, or an archived brief with a non-empty un-promoted *Discoveries* section, fails ([DOC-12](00-documentation-principles.md)).

**Why:** "Hooks enforce; prompts request" ([Quality Playbook](02-quality-playbook.md)). Every rule above that stayed prose-only would decay exactly the way [SDD-11](03-spec-driven-design.md) predicts.

---

## Part 4 — Gates and autonomy

### EXE-08 — Exactly two human gates

*(Addresses: C-25)*

Human approval is required at exactly two transitions, and MUST NOT be skipped:

1. **Spec gate:** a specification or ADR moves to `Accepted` only by explicit human decision.
2. **Brief gate:** a task brief moves from `Draft` to `Ready` only by explicit human decision.

No other transition requires human approval.

**Why:** These are the two points where a mistake's cost is still one page of prose. Gating more transitions than these trades away agent throughput for no additional safety; gating fewer lets errors compound into code.

### EXE-09 — Downstream of `Ready`, work is autonomous but bounded

*(Addresses: C-25, C-26)*

Once a brief is `Ready`, the Implementer runs without further approval, bounded by the brief. It MUST stop and report — not work around — when any of the following occurs:

- The brief is insufficient to proceed ([DOC-07](00-documentation-principles.md) rule 3: defect one rung up).
- The correct change requires a file outside *Files touched* (the brief was wrong; fix it first).
- Implementation reveals the spec should change ([SDD-10](03-spec-driven-design.md): spec-first, never code-first).
- An acceptance check cannot be made to pass within the brief's scope.

A stop is reported as a defect against the brief or spec, and the brief returns to `Draft` until a human re-gates it.

**Why:** Autonomy is safe exactly when its blast radius is pre-approved. The brief's *Files touched* and *Acceptance checks* are that pre-approval; anything outside them re-enters the gated zone.

---

## Part 5 — The loop, end to end

1. **Spec Author session** (foundation): design per [SDD-01..SDD-07](03-spec-driven-design.md); chapter reaches `Proposed`. → **Human: `Accepted`.**
2. **Brief Writer session** (component repo + foundation): `/write-brief LEX-01..LEX-06` → brief in the component repo's `work/`, status `Draft`. → **Human review: `Ready`.**
3. **Implementer session** (component repo only): `/implement-brief` → code and tests citing rule IDs → acceptance checks pass → brief `Done`, archived, discoveries promoted — all in the completing change.
4. **CI**: link, parity, and lifecycle checks (EXE-07) hold the line thereafter.

---

## Metadata

- **Status:** Accepted (2026-08-01)
- **Audience:** Anyone — human or AI agent — turning Accepted specifications into code, or configuring the tooling that does
- **Rule prefix:** `EXE-`
- **References:** [Documentation Principles](00-documentation-principles.md) — DOC-07, DOC-09, DOC-12; [Spec-Driven Design](03-spec-driven-design.md); [Quality Playbook](02-quality-playbook.md)
