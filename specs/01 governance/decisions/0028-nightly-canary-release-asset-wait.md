# ADR-0028 — Wait-for-release-assets pattern for nightly canaries

**Status:** Draft

Nightly canaries in `clean-framework` and `clean-server` were originally triggered by `schedule` + `push` on `v*` tags. They raced the release job's asset-upload step: the tag exists the moment `git push --tags` completes, but the release archives are still being built and uploaded across the matrix. `cln install latest` would either resolve the *previous* release (silent wrong version tested) or hit a 404 mid-upload. The workaround has been to make both workflows `workflow_dispatch`-only — which means canaries do not run unless someone remembers to click a button, defeating the whole purpose of a nightly gate. This ADR fixes the race with a wait-for-assets step and restores the automatic triggers.

---

## Context

The race is present in every workflow that installs a fresh release and then runs a corpus against it:

- `clean-framework` — Nightly Canaries (currently `dispatch` only)
- `clean-server` — Nightly Canaries (currently `dispatch` only)
- `clean-framework` — Test Framework (installs latest `cln` from releases; hits the race on tag-push runs but not on push-to-`main` runs)
- Any future canary or reporter-artifact workflow that consumes `cln install latest`

The pattern is: install shipped artifacts → run corpus against them → report pass/fail. The failure mode is deterministic — `cln install latest` resolves to the tag that exists on GitHub Releases, but a release without its expected assets is indistinguishable from an old release from `cln`'s point of view.

Making the workflows manual-only removes the false failures at the cost of removing the actual signal. That is not a fix — that is turning the gate off. See [`../../05 execution/automation/03-quality-gates.md#canaries-against-real-runtimes`](../../05%20execution/automation/03-quality-gates.md#canaries-against-real-runtimes) for how the manual-only state currently looks.

## Decision

Adopt a **wait-for-release-assets** GitHub Actions step, applied as the first step in any workflow that installs a released artifact. The step polls the GitHub Releases API for the target repo until every expected asset is present, then proceeds. It restores automatic triggers (`schedule` + `push` on `v*`) for every canary that was disabled to work around the race.

### The reference step

```yaml
- name: Wait for release assets
  uses: actions/github-script@v7
  env:
    TARGET_OWNER: clean-language
    TARGET_REPO: clean-language-compiler
    TARGET_TAG: ${{ github.ref_name }}       # or a resolved tag when not tag-triggered
    EXPECTED_TARGETS: 'linux-x64,macos-x64,macos-arm64,windows-x64'
    TIMEOUT_MINUTES: '30'
    POLL_SECONDS: '30'
  with:
    script: |
      const owner = process.env.TARGET_OWNER;
      const repo = process.env.TARGET_REPO;
      const tag = process.env.TARGET_TAG;
      const expected = process.env.EXPECTED_TARGETS.split(',').map(s => s.trim());
      const timeoutMs = parseInt(process.env.TIMEOUT_MINUTES) * 60 * 1000;
      const pollMs = parseInt(process.env.POLL_SECONDS) * 1000;
      const deadline = Date.now() + timeoutMs;
      while (Date.now() < deadline) {
        const rel = await github.rest.repos.getReleaseByTag({ owner, repo, tag })
          .catch(() => null);
        if (rel) {
          const names = rel.data.assets.map(a => a.name);
          const missing = expected.filter(t => !names.some(n => n.includes(t)));
          if (missing.length === 0) return;
          core.info(`Waiting for assets: ${missing.join(', ')}`);
        } else {
          core.info(`Release for tag ${tag} not yet visible`);
        }
        await new Promise(r => setTimeout(r, pollMs));
      }
      core.setFailed(`Assets for ${tag} not present after ${process.env.TIMEOUT_MINUTES} min`);
```

### Placement and parameters

- **Placement:** first step of the first job that consumes the released artifact. Before any `cln install`, before any sibling checkout that references the release.
- **`TARGET_TAG`:** for tag-triggered workflows, `${{ github.ref_name }}`. For `schedule`-triggered workflows, resolve to the latest tag with an explicit `gh release view --json tagName` step and pass the result in.
- **`EXPECTED_TARGETS`:** the list of platform identifiers whose archives must exist. Sourced from the release workflow's matrix — mismatch here is a bug.
- **`TIMEOUT_MINUTES: '30'`** — accommodates the slowest observed release matrix run (~18 min for `clean-server` 5-target matrix, with headroom).
- **`POLL_SECONDS: '30'`** — GitHub API is not the bottleneck; polling faster wastes API quota, polling slower delays the canary start.

### Where this step is added

All of the following gain the step in the same PR that lands this ADR:

| Workflow | Trigger restored to | Waits for release of |
|---|---|---|
| `clean-framework` Nightly Canaries | `schedule` + `dispatch` + `push` on `v*` | `clean-language-compiler` + `clean-framework` |
| `clean-server` Nightly Canaries | `schedule` + `dispatch` + `push` on `v*` | `clean-language-compiler` + `clean-framework` |
| `clean-framework` Test Framework | Unchanged | `clean-language-compiler` (only when tag-triggered) |
| `clean-framework` reporter-artifacts | `schedule` + `dispatch` (schedule restored) | `clean-language-compiler` |

### Reference implementation

The canonical version of this step lives at `foundation/scripts/reference-workflows/wait-for-release-assets.yml` (to be added). Every workflow above `uses:` that reference via a composite action or copies the step inline with a comment pointing back. Composite action is preferred once the pattern lands in three or more workflows.

## Options considered

- **A — Do nothing, leave canaries manual (status quo).** Zero engineering cost, permanent loss of automated coverage. Rejected — that is what triggered this ADR.

- **B — Add `sleep 10m` after tag push in the release workflow.** Simple. Wrong: couples the *producer* to the *consumer*, and 10 min is not always enough. Rejected on both counts — the ecosystem rule from [`../../05 execution/automation/04-cross-repo-triggers.md`](../../05%20execution/automation/04-cross-repo-triggers.md#rules-that-hold-across-all-four-mechanisms) is that failing cross-repo work does not block the producer.

- **C — Poll for assets in the consumer (chosen).** Adds ~2–5 min per canary run in the median case (assets already present), up to 30 min in the pathological case. Keeps producer decoupled. Composable as a reusable action.

- **D — Fire `repository_dispatch` from the release workflow's final job.** Cleaner in principle: the consumer subscribes to "release complete", not "tag pushed". More invasive — every consumer becomes a listener, and the failure mode when the dispatch is dropped is silent (no canary run at all). Rejected as follow-up work — the poll approach is strictly simpler and can be replaced by dispatch later without changing the consumer's shape.

## Consequences

**What becomes easier:**

- **Nightly canaries actually run nightly.** The signal returns. Regressions in a fresh release are caught within 24 hours instead of at the next manual trigger.
- **Reporter-artifact replay resumes on a schedule.** Same mechanism; the `clean-framework` reporter-artifacts workflow was disabled from schedule for the same asset-race reason and can be re-enabled with the same step.
- **The manual-only workaround comes off the table across the ecosystem.** Future workflows that install `cln` artifacts have a known-good pattern to follow.

**What becomes harder:**

- **Every canary run pays a 2–5 min wait in the median case.** Acceptable given canaries are already 10–30 min runs.
- **Rare pathological case: 30-min timeout expires.** Happens if the release matrix itself is broken. The canary fails visibly with `"Assets for vX.Y.Z not present after 30 min"`, which is the correct signal — the release is broken, canaries should not have run.
- **`EXPECTED_TARGETS` must be kept in sync with each release's matrix.** Drift here is a silent bug: adding a new target to the release matrix without updating the wait step means canaries can proceed with a partial install. Mitigation: the reference workflow includes a comment tying the value back to the release workflow's matrix, and this ADR's follow-up work includes a lint that fails PRs modifying a release matrix without a matching `EXPECTED_TARGETS` update.

---

## Metadata

- **Status:** Draft
- **Date:** 2026-08-07
- **Supersedes:** None
- **Spec impact:** Nightly canaries in `clean-framework` and `clean-server` restore `schedule` + `push` triggers with the wait-for-assets step added. Reference step lives at [`../../scripts/reference-workflows/wait-for-release-assets.yml`](../../scripts/reference-workflows/wait-for-release-assets.yml).
