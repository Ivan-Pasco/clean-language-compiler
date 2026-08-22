# Documentation Principles — Clean Language Ecosystem

The Clean Language ecosystem is built primarily by AI agents working from written specifications. When those specifications are unclear, agents guess; when they contradict each other, agents pick arbitrarily; when they carry ceremony that no tool consumes, agents and humans both waste attention on it. This document is the small set of rules that keep the documentation useful — for the person reading it today, for the agent generating code from it tomorrow, and for the validator that will one day check it mechanically.

The rules that follow are grouped in three parts: how documents are shaped, how they relate to each other, and how they are cited. Each rule has a stable ID (`DOC-01` … `DOC-19`) that other documents cite by ID rather than paraphrase.

---

## Part 1 — How documents are shaped

### DOC-01 — Reader-first opening

Every document opens with a plain-language paragraph — two to five sentences — that explains what the document is and why it exists. This opening MUST be understandable to a reader who has never seen the spec framework, does not know the ID system, and has not read any linked document. Headers, metadata blocks, and status lines come after the opening paragraph, never before it.

**Why:** if the reader cannot form a mental model of the document in the first thirty seconds, nothing that follows lands, no matter how precise. A document that opens with eight lines of metadata is telling the reader "you are not the audience" — even when they are.

### DOC-02 — Plain language, precise where it needs to be

Prose is written in the plainest language that carries the meaning exactly. Technical vocabulary is used where it is *load-bearing* (a WIT `world`, an `Argon2id` hash, an `SYN004` error) and avoided where it is decoration. When a term is used for the first time in a document, it is defined inline or linked to its definition.

The rule is not "make it simple." The rule is: **remove every word that does not do work**, and never sacrifice precision for accessibility. If a concept is unavoidably technical, lead with a one-sentence human summary, then go deep.

**Why:** documents that read as if they were generated to satisfy a template are ignored. Documents written for a human reader get read, and get followed.

### DOC-03 — Metadata after content, not before

Status, rule prefix, canonical references, principle satisfaction, and any other machine-oriented metadata MUST appear either at the bottom of the document under a `## Metadata` heading, or in a clearly demarcated block after the opening paragraph — never as the first content the reader sees. Metadata serves tooling and cross-reference; it does not serve the reader trying to understand what the document says.

A `**Status:**` line MAY appear immediately under the title when a document's status materially changes how it should be read (a `Draft` warning, a `Superseded by` redirect). No other metadata gets that position.

**Why:** the top of the page is the most valuable real estate in the document. Spend it on the reader.

### DOC-04 — One reader profile per document

Each document declares its intended audience in one line — the compiler maintainer, a library author, an agent generating task briefs, a plugin operator. Documents that try to serve every audience serve none. When a document genuinely needs to serve more than one audience, split it, or clearly section it by audience with the technical detail deferred until after the shared context.

### DOC-05 — Right-sized, not atomized

A document exists when its topic can be read as a single sitting. As a rough guide: 200–600 lines of substantive prose. Below that, merge with a sibling. Above that, split — but only into *distinct topics*, never into "part 1 / part 2."

Folder depth is capped at two levels (`04 language/03-lexical-structure.md` is fine; `04 language/lexical/comments/block.md` is not). Deeper structures require an ADR.

**Why:** documentation navigability collapses when concepts are shattered across many small files. A reader who has to open six siblings to understand one thing cannot hold the shape of the system in their head.

### DOC-06 — Fixed templates for standard document types

ADRs, task briefs, and spec chapters follow the templates in Part 4 of this document, in the given section order. Empty sections stay with the note "None." rather than being deleted — the shape is part of the contract.

Other document types (governance chapters, reference catalogues, guides) have no fixed template. Their shape is dictated by DOC-01..DOC-05 alone.

---

## Part 2 — How documents relate to each other

### DOC-07 — The ladder of intent

Development moves down a ladder of altitudes. Each rung is a distinct document type, and no actor — human or agent — may jump more than one rung in a single step.

| Rung | Document type | Answers | Home |
|------|--------------|---------|------|
| Why | Governance | Why does this exist; what are the boundaries | `01 governance/` |
| What | Specification | What must be true, observably | `02 components/`, `03 platform/`, `04 language/` |
| How (decided) | Design decision (ADR) | Which approach was chosen, and why | `01 governance/decisions/` |
| How (this batch) | Task brief | What changes, in what order, with what checks | `work/` |
| Code | Source + tests | — | Component repositories |

The two rules that make the ladder deterministic:

1. **An agent writing code reads the task brief and the code it touches.** If the brief is insufficient, the brief is defective — fix the brief, do not improvise from the spec.
2. **An agent writing a task brief reads Accepted specs and ADRs.** If the spec is insufficient, the spec is defective — fix the spec first.

An actor MUST treat insufficiency at its own rung as a defect one rung up, and say so, rather than improvising.

### DOC-08 — Abstraction per rung

Each rung of the ladder speaks at its own level and cites the rung above it, never further:

- **Governance** states principles that any component would need to know. Never names specific files, functions, or code paths.
- **Specs** state observable behavior. Cite governance principles they inherit; never name implementation classes or file paths.
- **ADRs** state a chosen approach. Cite the specs they change and the principles they satisfy.
- **Task briefs and code** cite specs and ADRs. They do **not** cite governance directly — if code needs to cite a governance principle, the spec is missing a rule.

**Why:** skipping abstraction levels is how implementation details leak into governance and how governance rules end up hard-coded in tests. Each rung is a firewall.

### DOC-09 — Status gates: only Accepted may be built from

Every durable document carries a `**Status:**` of `Draft`, `Proposed`, `Accepted (YYYY-MM-DD)`, or `Superseded by <link>`. Agents MUST NOT generate task briefs, code, or downstream specs from any document that is not `Accepted`. A decision that exists only in conversation, memory, or a Draft has no force.

This is the cheapest gate available. It prevents "the AI implemented a spec I was still thinking about."

### DOC-10 — Canonical upstream references, declared

Every document declares its canonical upstream references in the metadata — typically one to three documents. Body citations SHOULD stay within that declared set. Adding a reference outside the declared set is allowed, but should be rare; if it becomes common, the declared set is wrong and should be revised.

**Why:** without this rule, citation webs sprawl until no document has a comprehensible foundation. A reader who has read the top-declared references should have the context needed to understand the body.

### DOC-11 — Explicit precedence when documents conflict

When documents conflict, this order decides, higher overrides lower:

1. Governance (`01 governance/`)
2. Specifications (`02 components/`, `03 platform/`, `04 language/`)
3. ADRs (`01 governance/decisions/`) — within specs' bounds; an Accepted ADR that contradicts a spec means the spec update was missed
4. Task briefs (`work/`)
5. Reference catalogues (`execution/`) — informative infrastructure inventories
6. Code comments

A conflict is itself a defect: the actor who detects it MUST log it (as a task brief or issue) and resolve by precedence in the meantime — never paper over it by guessing intent.

### DOC-12 — Working documents stay in `work/`

Disposable documents (task briefs, plans, analyses) live only in `work/` (active) and `work/archive/` (done). Generated outputs live in `reports/`. They MUST NOT be linked from durable documents, and MUST NOT be the home of any fact. Anything in a task brief worth keeping is promoted to a spec or ADR before the brief is archived. A task brief is archived when its acceptance checks pass, in the same change that completes it.

---

## Part 3 — How rules are named and cited

### DOC-13 — Stable IDs for checkable rules; no ceremony for prose

Rules that state observable behavior — the kind a test can verify or a validator can check — carry stable IDs within their document (`DOC-05`, `LEX-01`, `SEC-07`, error codes as in [Error Codes](../03%20platform/09-error-codes.md)). Prose that provides context, rationale, or explanation does not get an ID.

Concretely:

- Governance principles, spec normative rules, and error codes carry IDs. Architecture prose, design goals, and rationale do not.
- IDs are never renumbered or reused. A retired rule keeps its ID, marked withdrawn.
- Each document owns one ID prefix (2–4 uppercase letters), registered in the [governance README](README.md). Claiming a new prefix means adding it to the registry first.
- The prefix is mnemonic of the topic (`LEX-` for lexical structure), not derived from the file's numeric prefix. Files can be renumbered; IDs cannot.

**Why:** IDs exist so that citations survive editing and so that mechanical checks (parity, coverage, cross-reference) become possible. Attaching an ID to every paragraph turns the document into noise. Attaching one to every checkable rule pays for itself.

### DOC-14 — Cite by ID and range, not by anchor link soup

Citations to other documents use rule IDs, not paraphrase. When citing multiple rules from the same document, use ranges or lists of plain IDs (`INTEROP-01..03, SEC-07`), not one anchor link per ID. Per-anchor links belong inline in prose when the reader benefits from jumping straight to one rule ("indentation follows [LEX-01](…)"), not in header metadata blocks that list seven of them.

The prefix registry names the file each prefix lives in — readers do not need per-ID anchors to find them. In task briefs, tests, commit messages, and code, plain-text IDs suffice (`// verifies LEX-01`), because IDs are globally unique and grep-able.

A citation is an assertion that a specific rule applies. If the citation is decorative — added because "documents in this folder cite three principles" — it is a defect and should be removed.

### DOC-15 — Grammar is the source of truth for syntax; specs cite it

Every syntax-bearing chapter has a companion grammar file at `<folder>/grammar/<chapter-slug>.ebnf.md`. Grammar productions live only in that file. The chapter describes semantic rules attached to productions and cites productions by name; it does not restate the grammar in prose and does not embed productions inline. Semantic rules attached to productions carry IDs (`LEX-`, `SYN-`, `TYPE-`, `SEM-`); these are what compiler tests reference. A syntax-bearing chapter without a companion grammar file, or a grammar production restated in the chapter, is a defect.

**Why:** hand-maintaining "the spec says X and the grammar says X" across two documents drifts on the first refactor. One source, cited from the other, does not. The companion-file location is fixed so tooling (parsers, validators, RAG retrievers) can find it without configuration.

### DOC-16 — History lives in git; changelogs are for normative changes

Documents MUST NOT carry a changelog for editorial changes — typos, prose polish, link fixes, section reorganizations that preserve meaning. Git is the record.

Normative spec chapters MAY carry a `## Changelog` section, and entries are limited to changes in MUST/SHOULD content: date, rule ID, one line on what changed. Governance documents, ADRs, and reference catalogues do NOT carry changelogs. ADRs are immutable by DOC-07; governance changes are rare and traceable in git.

**Why:** a changelog that records every edit is git with worse tooling. A changelog that records normative changes only is a curated audit trail worth reading.

### DOC-17 — Ceremony pays for itself only when consumed

This is the meta-rule that governs the others. Documentation ceremony — headers, metadata blocks, ID annotations, cross-reference matrices — is only worth its reading cost when something mechanically consumes it (a validator, a generator, a checker). When the tooling that would consume it is more than one milestone away, defer the ceremony.

If a piece of metadata cannot be pointed to a consumer, it is decoration, and decoration is trimmed. Adding new ceremony requires naming the consumer.

### DOC-18 — Structured-data artifacts have a single canonical schema file

Every structured-data artifact (TOML manifest, WIT world, JSON schema, canonical field/type table) has one canonical schema file. The artifact's home folder contains a `schema/` subfolder holding one file per artifact — for example, `02 components/framework/schema/plugin.toml.md`, `03 platform/wit/http-envelope.wit.md`. Spec chapters cite fields from the schema file by name; they do not restate the schema. A structured-data artifact defined in more than one place is a defect; the redundant definitions collapse into the canonical file and other chapters cite it.

**Why:** the same TOML field defined in three chapters produces three subtly different constraints after the first refactor. Generators and validators built against the divergent copies produce divergent code. One source, cited from the others, does not drift. The `schema/` subfolder location is fixed for the same reason DOC-15 fixes `grammar/` — tooling can find it without configuration.

### DOC-19 — Doc-type metadata trailers are required

Documents in the foundation take one of five doc types: Definition (BA-readable narrative), Semantic Rules (rule-IDed normative content), Grammar (source-of-truth productions per DOC-15), Schema (canonical structured-data definitions per DOC-18), or Execution (patterns, tutorials, reference catalogues, AI-derived artifacts). Each type carries a metadata trailer with the required fields for that type — back-links, prefix, notation, illustrates, source-of-truth, kind — as defined in the taxonomy. Every trailer field has a named consumer per DOC-17. A durable document with missing required fields for its type is a defect.

**Why:** the doc types exist so mechanical tooling can route retrieval, validation, and generation correctly — a code-generator loads Grammar and Semantic Rules; a spec-reviewer loads Definition and Execution examples; a backlink hook validates that `Illustrates:` and `Source of truth:` targets still resolve. If the metadata that identifies a document's type and back-links is absent, the tooling cannot do its job and the type distinction collapses. The consumer for each field is named in the taxonomy — no field is required for decorative reasons alone.

---

## Part 4 — Templates

### 4.1 Spec chapter shape

```markdown
# <Chapter title>

<Opening paragraph (2–5 sentences). Plain language. What this chapter
covers and why it matters. Understandable without any prior context
from the spec framework.>

---

## 1. <First substantive section>

...

## N. <Last substantive section>

---

## Changelog

<Normative changes only. Format:
- YYYY-MM-DD — RULE-ID changed from X to Y. Reason.
Omit the section entirely if the chapter has no normative changes yet.>

## Metadata

- **Status:** Draft | Proposed | Accepted (YYYY-MM-DD) | Superseded by <link>
- **Audience:** <who this chapter is written for>
- **Rule prefix:** <PREFIX-> (if the chapter owns one)
- **References:** <1–3 canonical upstream documents>
- **Satisfies:** <governance principle IDs, e.g. LANG-01..03, SEC-07>
```

### 4.2 ADR template

Location: `01 governance/decisions/NNNN-short-title.md`, where `NNNN` is the next sequential number.

```markdown
# ADR-NNNN — <Short decision title>

<Opening paragraph. What was decided and why in plain language.
Two or three sentences. Enough that a reader can decide whether
they need to read further.>

---

## Context

<The problem and the forces at play. One or two paragraphs.
Enough that a reader five years from now understands why this
needed deciding.>

## Decision

<The chosen option, stated normatively. One paragraph.>

## Options considered

<Each option in one short paragraph, with its main trade-off.
Include the rejected ones — they are the point.>

## Consequences

<What becomes easier, what becomes harder, what must now be done.>

---

## Metadata

- **Status:** Draft | Proposed | Accepted (YYYY-MM-DD) | Superseded by [ADR-MMMM](MMMM-title.md)
- **Date:** YYYY-MM-DD
- **Supersedes:** ADR-MMMM | None
- **Spec impact:** <links to spec sections that must change when this is accepted> | None
```

### 4.3 Task brief template

Location: `work/YYYY-MM-DD-short-title.md`. Archived to `work/archive/` on completion.

```markdown
# Task — <Short title>

<Opening paragraph. What this task delivers and why it exists now.
Two or three sentences.>

---

## Scope

<What this task delivers, in observable terms.>

## Non-goals

<What this task deliberately does not touch. Never empty — if truly
nothing is excluded, the scope is too big.>

## Files touched

<Expected files to create/modify. A file outside this list appearing
in the diff means the brief was wrong — stop and fix the brief.>

## Steps

<Ordered, mechanical steps. Each step small enough to verify alone.>

## Acceptance checks

<Commands to run and their expected results. The task is Done when
these pass — not before, not "mostly.">

## Discoveries

<Filled during execution: anything learned that belongs in a spec or
ADR. Promoted before archiving; empty means "None.">

---

## Metadata

- **Status:** Draft | Ready | Done
- **Date:** YYYY-MM-DD
- **Implements:** <spec sections / rule IDs>
- **Inputs:** <the Accepted documents this brief was derived from>
```

---

## Part 5 — How agents apply these rules

An AI agent working in this repository:

1. Reads the target document's opening paragraph and Status before doing anything else. If Status is not Accepted, work stops (DOC-09).
2. Never skips a rung of the ladder. Missing rung → defect one level up, reported and stopped (DOC-07).
3. Never restates a rule from another document; cites by ID (DOC-14).
4. Never adds a piece of metadata, an ID annotation, or a cross-reference without a consumer for it (DOC-17). If the consumer doesn't exist, the ceremony waits.
5. Logs conflicts instead of resolving them silently (DOC-11).
6. Promotes durable discoveries out of disposable documents before archiving (DOC-12).

These rules are prose today. Each SHOULD graduate into a mechanical check (MCP validator, pre-commit hook, or CI step) — status-line linting, template-section linting, ID-reference checking, canonical-reference enforcement. Until then, this document is the checklist.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Audience:** Anyone — human or AI — writing, editing, or building from documentation in this repository
- **Rule prefix:** `DOC-`
- **References:** [Quality Playbook](02-quality-playbook.md), [Architectural Concerns](05-concerns.md)
- **Supersedes:** Prior revision Accepted 2026-08-06 (`DOC-01`..`DOC-17`). DOC-15 amended (companion grammar file now required, not optional); DOC-18 added (canonical schema files); DOC-19 added (doc-type metadata trailers). No renumbering. Motivating rationale in `foundation/work/2026-08-07-docs-readiness-program.md`.
