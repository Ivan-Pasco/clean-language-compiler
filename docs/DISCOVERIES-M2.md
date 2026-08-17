# Milestone 2 — Discoveries

Spec findings made while building the diagnostics infrastructure (registry,
renderer, DIA-06 harness, `cln check`). This repo's session is
path-allowlisted out of foundation (CT-H-16); a foundation session carried
them over.

**Status (2026-08-15):** items 1, 2 and 4 landed in foundation as errata
(`46ad2f6`, `a05f7dd`, `0353ec6` — the compiler realigned to the LIB017
template and the §1.1 recount in the same day's commit); items 3, 5 and 6
became decision briefs in `foundation/work/` (`c4f2a3f`). Item 7 is this
repo's own M3/M4 backlog and involves no spec change.

1. **LIB017's message template carries example values, not placeholders.**
   Platform 10 §10.7 writes the template as `"Folder scope 'app/ui' maps to
   library 'clean.ui.v2' which is not a declared dependency"` — `app/ui`
   and `clean.ui.v2` are the example's values where every sibling rule has
   `{placeholder}` slots. Copied verbatim into `codes.rs` per the
   no-redaction rule; the spec entry should gain `{folder}` / `{lib}`
   slots (compare LIB018, which has them).
2. **Platform 13 §5.4's example block is internally inconsistent.** The
   `-->` line reads `app/reports/summary.cln:18:12` while the caret run
   and both suggestion spans place the flagged identifier at columns
   17..23. One renderer cannot produce both from one span; the caret/
   suggestion positions were taken as authoritative (the renderer tests
   reproduce the block with `:18:17`). The example's `-->` line should be
   corrected.
3. **`= suggestion:` rendering is underspecified.** §4.2's line format
   shows `= suggestion: <replacement snippet>` but no rule for multi-
   replacement suggestions, and §5.4 renders suggestions as a separate
   numbered list with applicability tags instead. Adopted locally: the
   first replacement's new text, falling back to the suggestion message;
   revisit when `cln fix` lands.
4. **Platform 09 §1.1's "161 active" counts five retired identifiers.**
   The arithmetic only closes if `SCOPE005`, `LIB005`, `LIB007`, `LIB008`
   and `LIB009` — all marked withdrawn in their own rows — are counted as
   active, with only `IMPORT005` counted as withdrawn. `codes.rs` registers
   the same 162 rows but marks all six `Withdrawn` (none may be emitted,
   none reused); §1.1's phrasing should say "156 emittable" or count all
   six withdrawals.
5. **COM013 has no input-reachable trigger.** An internal-invariant breach
   cannot be provoked by a well-formed request, so the DIA-06 triple for
   COM013 cannot exist without a fault-injection hook in the compiler. It
   stays in `unimplemented.txt` with that rationale; DIA-06 may want a
   registered exception for ICE codes, or Platform 14 a debug-only fault
   hook.
6. **RQD fixtures need a request document, not a `.cln`.** DIA-06 mandates
   `tests/cln/diagnostics/<code>.cln`, but RQD001/RQD002 fire before any
   source is parsed. Adopted locally: an optional committed
   `<code>.request.json` that the harness feeds through the same intake
   path the binary uses; the `.cln` still exists as the embedded source.
   Candidate DIA-06 amendment.
7. **M1 span choices read poorly under the full renderer.** The M1
   typecheck emits SEM001 with the whole declaration as the primary span
   (spec's worked example puts the primary on the variable name and the
   RHS as secondary), so the caret run covers the entire statement and the
   secondary collapses onto its tail. Snapshot-locked as-is; span
   refinement belongs to the M3/M4 front-end work and will update the
   DIA-06 snapshots in the same commit.
