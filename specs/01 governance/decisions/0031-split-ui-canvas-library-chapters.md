# ADR-0031 — Split the `ui` and `canvas` library specification chapters

The `ui` and `canvas` library chapters have grown to ~2000 lines each — far beyond DOC-05's 200–600 line guideline and past the point where a reader can hold the shape of the library in their head. This ADR proposes splitting each chapter into two sibling chapters at a natural thematic seam, so both halves fit the "single sitting" test while every existing rule ID and section anchor keeps resolving.

---

## Context

DOC-05 (`Right-sized, not atomized`) sets a rough guide of 200–600 lines of substantive prose per document. Above that, documents SHOULD split — but only into *distinct topics*, never into "part 1 / part 2." Below the guide, they SHOULD merge. Two library specs currently violate the upper bound by roughly 3.5×:

- `02 components/framework/libraries/10-ui.md` — **2085 lines, 32 sections.**
- `02 components/framework/libraries/02-canvas.md` — **2043 lines, 28 sections.**

Both were flagged during the 2026-08-07 documentation realignment pass. The realignment agent was told not to split without an ADR, so it left them intact and reported them for follow-up.

The size is not the only symptom. Both files bundle two visibly different topics:

- `ui` mixes **the UI language surface** (components, data binding, events, hydration, layouts, forms, styling) with **a runtime browser-API grab-bag** (DOM query helpers, iframe communication, incremental patching, live streaming, keyboard shortcuts, focus management, storage, clipboard, observers, toasts, error boundaries). The first is the language a Clean UI author uses to describe an interface. The second is the runtime toolbox a Clean UI author reaches into when the declarative surface is not enough.
- `canvas` mixes **the canvas language surface** (actors, scenes, primitives, transforms, effects) with **the animation system** (tweens, timelines, animation state machines, path animation, particles, sprites) and **the runtime integration** (audio, camera, collision, ui integration, host functions, future 3D). The first defines what you draw; the second and third define how you make it move and how it plugs into the wider app.

These are distinct topics per DOC-05, not "part 1 / part 2" arbitrary chunks. Splitting is warranted; the question is where the seams go and how citations survive.

## Decision

Split each chapter at its topic seam and place the sibling chapters in the same folder (`framework/libraries/`), keeping the numeric prefix system contiguous.

### `10-ui.md` → split into two chapters

- **`10-ui.md`** (retains number) — **The UI language surface.** Sections 1–15 stay: introduction, architecture, page structure, custom tags, data binding, event handling, hydration, layouts, forms, styling, accessibility, security, file structure, build output, examples. Estimated new length: ~1290 lines. Still above the 600-line guide, but every remaining section describes one coherent surface — the declarative UI language a Clean author writes.
- **`12-ui-runtime.md`** (new) — **Runtime UI toolbox.** Sections 16–32 move here, renumbered 1–17: DOM query functions, iframe communication bridge, incremental DOM patching, live streaming (`cl-stream`), context menu event, global keyboard shortcuts, CSS variable runtime manipulation, focus management, browser storage, file download trigger, clipboard API, resize/intersection observers, toast notification system, client-side navigation, error boundaries, library lifecycle integration, host function reference. Estimated length: ~790 lines.

### `02-canvas.md` → split into three chapters

- **`02-canvas.md`** (retains number) — **The canvas language surface.** Sections 1–12 stay: introduction, capability, actor classes, scene declaration, `scene:` block, shared state, assets, layers, drawing primitives, transforms, effects, gradients. Estimated new length: ~800 lines.
- **`13-canvas-animation.md`** (new) — **Animation and sprites.** Sections 13–19 move here, renumbered 1–7: tween system, timeline system, animation state machines, path animation, particle systems, sprites and animated sprites, audio. Estimated length: ~590 lines.
- **`14-canvas-runtime.md`** (new) — **Runtime and integration.** Sections 20–28 move here, renumbered 1–9: camera & viewport, collision detection, easing functions, scene management, integration with ui, complete example, performance guidelines, host function reference, future canvas3d. Estimated length: ~660 lines.

### Rule IDs and citation stability

- **Rule IDs are never renumbered** (DOC-13). A rule with ID `UI-14` stays `UI-14` regardless of which chapter it now lives in. Grep, cross-references, tests, and commit messages that cite `UI-14` continue to resolve.
- **Section numbers reset in the new chapters.** A section that was `## 16. DOM Query Functions` in `10-ui.md` becomes `## 1. DOM Query Functions` in `12-ui-runtime.md`. Any external doc that cites `10-ui.md §16` is broken by the move — those citations MUST be rewritten during the split.
- **Companion grammar files stay in place.** `framework/libraries/grammar/10-ui.ebnf.md` continues to cover the language surface. If the runtime toolbox has any grammar (unlikely — it is mostly helper functions), it gets its own companion file per DOC-15.
- **Rule prefixes stay per-topic, not per-file.** The `UI-` prefix continues to span both `10-ui.md` and `12-ui-runtime.md`. The `CNV-` prefix spans all three canvas chapters. Splitting a chapter does not fork the prefix.

### File numbering seams and future insertions

The chosen numbers (`12`, `13`, `14`) occupy the next-available slots in the current `framework/libraries/` numbering (which currently ends at `11-agent.md`). If a future library gets inserted below these, it does not renumber them — DOC-13's no-renumbering rule applies to file numbers as much as to rule IDs.

## Options considered

**Option A — Split at the identified topic seams (chosen).** Two ui chapters, three canvas chapters, all in the same folder. Pros: each resulting chapter passes the "single sitting" test; each has one topic; rule IDs stay stable; citation-rewrite scope is bounded (only cross-chapter `§N` references need updating). Cons: 5 files instead of 2; readers who used to grep one file now grep two or three.

**Option B — Split by rule-block density (rejected).** Chunk the files at ~600-line boundaries regardless of topic. Pros: mechanical; predictable file sizes. Cons: violates DOC-05's "distinct topics, never part 1 / part 2" rule outright. A ui chapter that ends mid-`## 8. Layouts` because line 600 fell there teaches nothing about the shape of the library.

**Option C — Move the runtime/toolbox halves into `05 execution/` (rejected).** Treat runtime helpers as Execution-tier reference catalogues rather than Semantic Rules. Pros: preserves the primary chapter as the pure language surface; runtime helpers stop competing with normative rule text for space. Cons: the runtime helpers are normative — they carry rule IDs, dictate observable behaviour, and their absence would break apps. Demoting them to Execution would drop them below the DOC-11 precedence line and effectively unspecify them. Not defensible.

**Option D — Leave both files intact and accept the DOC-05 violation (rejected).** Pros: zero migration cost. Cons: the underlying reader-comprehension problem stays, and both files will keep growing. DOC-05 exists specifically because 2000-line specs stop being usable regardless of how well-organized they are internally.

**Option E — Split only ui, leave canvas intact (rejected).** Both files trip the same rule and both have the same natural seams. Splitting one and not the other creates inconsistency that a reader has to memorize.

## Consequences

**Easier:**
- Each new chapter is ~600–1300 lines — closer to (or within) the DOC-05 guide.
- The language-surface / runtime-toolbox distinction becomes visible in the file structure, not just in the reader's head.
- New rules land in the appropriate chapter without further growing an already-oversized file.
- Grep hits are more targeted — a query about `cl-stream` returns lines from `12-ui-runtime.md` only, not intermixed with hits from the language surface.

**Harder:**
- Cross-chapter section citations must be rewritten. A `grep -rn "10-ui.md#16-dom-query-functions"` across `01 governance/`, `03 platform/`, `04 language/`, `05 execution/`, `work/`, and `02 components/` gives the migration checklist. Every hit that references a moved section needs its file path updated.
- The `framework/libraries/README.md` catalogue needs a new entry for each new chapter.
- The `framework/libraries/grammar/` folder gains no new files by default; if the runtime toolbox turns out to have any grammar productions (e.g. `cl-stream` attribute syntax), they get extracted per DOC-15.
- Commit history for a moved section shows as delete+add rather than rename unless `git mv`-style splits are used. Since these are content splits (not file renames), the history discontinuity is unavoidable; the mitigation is a changelog entry in each new chapter naming the source chapter and the original section numbers.

**Must now be done (task brief scope):**
1. Create the three new files (`12-ui-runtime.md`, `13-canvas-animation.md`, `14-canvas-runtime.md`) with the moved sections renumbered 1–N.
2. Trim `10-ui.md` and `02-canvas.md` to their new scope; add a "See also" pointer to the sibling chapters at the top.
3. Add each new file to `framework/libraries/README.md` with a one-line description.
4. Grep for `10-ui.md#(1[6-9]|2[0-9]|3[0-2])-` and `02-canvas.md#(1[3-9]|2[0-8])-` anchor patterns across the tree and rewrite each hit to the new file + new section number.
5. Add a changelog entry to each source chapter naming the ADR and listing which sections moved to which sibling file.
6. Rule IDs stay unchanged; verify no test or grep-based check breaks by searching for representative `UI-` and `CNV-` IDs post-split.

---

## Metadata

- **Status:** Accepted (2026-08-07)
- **Date:** 2026-08-07
- **Supersedes:** None
- **Spec impact:** [`02 components/framework/libraries/10-ui.md`](../../02%20components/framework/libraries/10-ui.md), [`02 components/framework/libraries/02-canvas.md`](../../02%20components/framework/libraries/02-canvas.md), [`02 components/framework/libraries/README.md`](../../02%20components/framework/libraries/README.md); creates `12-ui-runtime.md`, `13-canvas-animation.md`, `14-canvas-runtime.md`. Cross-tree citation rewrites required (see Consequences).
