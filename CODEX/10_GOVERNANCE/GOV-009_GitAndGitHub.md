---
id: GOV-009
title: "Git and GitHub: branches, commits, pull requests and merges"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, workflow, agents]
related: [GOV-005, GOV-010, GOV-011, GOV-008, GOV-002]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF:** `main` changes only through a pull request that the Architect merges with a **merge commit**, pinned to the PR's exact head SHA, after CI is green on that head. Branches are named by purpose. A pushed branch's history is never rewritten, and generated files are never typed. GitHub Free enforces none of this (no branch protection, no required checks, no merge queue), so every agent does. Written on the director's instruction of 2026-10-18. It records the practice in force since SPR-008 and supersedes GOV-005 v1.3's "rebase and merge".

# GOV-009 — Git and GitHub

## 1. The platform

| Fact | Consequence |
|---|---|
| One private repository, `BigRigVibeCoder/Rehoboam`, on **GitHub Free** (GOV-008 §1) | No protected branches, required checks or merge queue. **Nothing stops a direct push to `main` or a red merge, so every agent must refuse both.** |
| "Automatically delete head branches" is on (decisions log, 2026-10-16) | A merged PR's branch disappears from GitHub. Never open a PR from a branch you still need, such as a `backup/` snapshot. |
| Agents may push under one GitHub identity | The author field does not tell you which agent did the work. The PR body's `Agent:` line and the commit trailers do (§4, GOV-011 §2). |
| About 2,000 Actions minutes a month (unconfirmed: D61 item 8) | CI runs only on pull-request events and pushes to `main` (GOV-010). Batch your pushes. |

## 2. Branches

| Prefix | For | Who pushes | How it ends |
|---|---|---|---|
| `main` | The only long-lived branch | **Nobody pushes to it directly.** It moves only by PR merges | — |
| `sprint/SPR-NNN-short-name` | A sprint: the Developer's tasks, then the Tester's test files and `VER-…-TEST` evidence | The sprint's Developer; its Tester (test files and evidence only) | Merged by its PR |
| `fix/DEF-NNN-short-name` | A defect fix outside a sprint | Its Developer | Merged by its PR |
| `arch/<topic>` | Architect work: plans, spec PRs, review records, rulings, micro-PRs, handoffs | The Architect, or a drafter it commissions for that topic | Merged by its PR |
| `review/<topic>` | An independent reviewer's or Tester's evidence that is not on the sprint branch (probes, reports) | That reviewer | Read and cited; deleted by the director when its sprint closes. Not merged |
| `audit/<topic>` | An audit record prepared apart from the sprint branch | The Architect | Merged through an `arch/` PR, or cherry-picked onto the sprint branch |
| `backup/<topic>` | A snapshot that must survive a session: a pre-audit fix batch, a handoff push | The session that owns the work | Its commits are fast-forwarded or merged into the real branch, then deleted. **Never opened as a PR** |

- **One owner per branch** (GOV-011 §2). Do not push to a branch another agent owns. Branch from it instead (`arch/<topic>-<suffix>`), or take it over as GOV-011 §3 describes.
- **Use a separate worktree per branch** when several sessions share a machine (`git worktree add`), never one shared working tree.
- **Deleting a branch** is the director's call when it holds evidence (`review/`) or someone else's work. Deleting your own merged or fast-forwarded `backup/` branch is fine.

## 3. Commits

- **Subject:** `type(scope): description`, where `scope` is the `SPR-`, `BCK-` item, `T-` id or topic.
  - Types in use: `feat`, `fix`, `test`, `perf`, `refactor`, `build`, `ci`, `docs`, `spec`, `plan`, `arch`, `verify`, `wip`.
  - A sprint task commit reads `type(SPR-NNN): T-NN description`.
  - End the subject with the change class tag group when it matters: `[C1]`, `[C2]`, `[C3]`, `[perf-baseline]`.
- **Body:** `Why:`, `What:`, `Refs:` (CODEX and `docs` ids). A `[perf-baseline]` commit also needs a `Reason:` line (`bin/perf_gate.py`).
- **Attribution:** end with your own tool's attribution trailer. Never copy another agent's trailers. Model identifiers appear nowhere else in the repository.
- **One commit per task** on a sprint branch. Spec and plan PRs commit per revision round.
- **A WIP commit** says so in its subject: `wip(<scope>): … (gates not run)`. Only a WIP commit may be pushed without its gates, and only to your own non-`main` branch.
- **Never commit:**
  - `CODEX/00_INDEX/MANIFEST.yaml` (generated; GOV-001 §3);
  - `artifacts/`, `target/`, `python/.venv/`;
  - secrets;
  - a hand-edited generated file (§5).

## 4. Pull requests

- **One PR per sprint or topic**, from its branch into `main`. Open it as a **draft** as soon as the first task lands: CI runs only on PR events, so a branch with no PR gets no CI (GOV-010 §1).
- **Title:**
  - a sprint: `SPR-NNN: <title> [C<n>]`;
  - other work: `<topic>: <title> [C<n>]`.
  - Add `[C3]` whenever a golden baseline under `worlds/` changes, and `[perf-baseline]` whenever a performance baseline changes. CI checks both (GOV-010 §4).
  - A title edit re-runs CI (the `edited` trigger).
- **Body:** the sections of `.github/pull_request_template.md`: Sprint (with the `Agent:` line), Compliance, Tasks, Verification, Merge route.
  - A `[perf-baseline]` PR adds a line `perf-baseline-reason: <why>`.
  - End the body with your tool's attribution.
- **Ready for review** only when `bin/check_all.sh` passes on the head and the Developer's report is attached. C2 and C3 work then gets the Tester's pass and the Architect's audit, committed to the branch before the merge (GOV-005 §5).
- **Edit only PRs you own.** Findings and rulings go into files (`REV-`, `VER-`, rulings records), not only into PR comments or chat. Keep PR comments few.

## 5. Keeping a branch current, and conflicts

- **Bring `main` in by merging it:** `git fetch origin main && git merge origin/main`. The subject names what came in, for example `Merge origin/main (PR #99: …) into <branch>`.
- **Never rewrite a pushed branch:**
  - no `rebase` of pushed commits;
  - no `commit --amend` of a pushed commit;
  - no `push --force` and no `--force-with-lease`.

  Testers, reviewers and audit records cite SHAs, and a rewrite breaks every one of them.
- **No bare `git stash`.** A stash is shared by every worktree of the clone, so another session can pop yours. Commit WIP to your own branch instead.
- **Conflicts in `bin/check_all.sh`** are resolved **by gate name**, never as text (the rule is in its header, lines 14–17):
  - take both sides of every hunk;
  - put each new gate in its part;
  - keep every `REQUIRED_GATES` entry from both sides.

  Then `python -I bin/check_all_selftest.py inventory` must pass both ways. A plain text merge can silently drop a re-wrapped gate.
- **Generated files in a conflict are never hand-merged.** Take either side, then regenerate with the tool:

  | File | Tool |
  |---|---|
  | Golden baselines `worlds/*/baselines/` | `bin/regen_baseline.sh`, in the branch's **final** commit, whose subject ends `[C3]` |
  | Performance baselines | `bin/perf_gate.py record` / `alloc-record`, in a `[perf-baseline]` commit |
  | Golden files of a reference | the reference's `--write` |
  | `specs/systems/next/amendments.lock.toml` | `lint_odd.py --next … --write-amendments-lock`, after the Architect has read the new `AMEND` lines |
  | `crates/rh-core` skeletons | `bin/gen_skeletons.py` |
  | `Cargo.lock`, `python/uv.lock` | `cargo`, `uv lock` |

## 6. Merging (the Architect only)

1. **Check the conditions** for the change class (GOV-005 §6):
   - C0/C1: green CI and the Architect's review;
   - C2: plus the Tester's pass and the audit `VER-`;
   - C3: plus the bisect and ensemble evidence, and an announcement to the director if the change is out of tolerance.
2. **CI must be green on the PR's current head:** all three parts `success`. A docs-only head shows golden and stream green within seconds, which is correct (GOV-010 §2). A head pushed after the run you read has not been checked.
3. **One merge at a time.** After each merge, the next PR in line merges `main` and gets a fresh green run before it merges. This stands in for "require branches to be up to date" (`docs/13` §4.4). At most one C3 PR is open for merge.
   - **The one exception, which saves runner minutes:** if every merge since the PR's green run was docs-only (GOV-010 §2), and none of them touched a file the PR touches (`comm -12` of the two `git diff --name-only` lists is empty), the PR may merge on its green run.
   - The push run on `main` after the merge is then the check, and a red one freezes merges (step 7).
4. **Method: a merge commit.** Never squash, never rebase-merge. Branches carry merges of `main` and `[C3]` and `[perf-baseline]` commits whose ranges CI's push rules re-check on `main`, and the SHAs that `VER-` and `REV-` records cite must stay valid (VER-SPR-008, -009, -011, -012, -013).
5. **Pin the head** to the full 40-character SHA you saw green, so a later push cannot slip in:
   - with the GitHub CLI: `gh pr merge <N> --merge --match-head-commit <sha>`;
   - through the API: `sha` / `expectedHeadSha`.
6. **Title:** `Merge PR #<N>: <summary> [C<n>]`.
7. **Afterwards:**
   - watch the push run on `main`;
   - a red `main` or a red nightly freezes C1–C3 merges until the cause is found (`docs/13` §4.4);
   - record C2 and C3 merges in `docs/decisions-log.md`.

## 7. Never

- Push to `main`, or merge a PR whose head is not green.
- Merge your own unreviewed C2 or C3 work.
- Force-push, or rewrite history someone else has fetched.
- Type a baseline or golden file, or "fix" a red golden run by re-recording it (`docs/13` §4.2).
- Skip, disable or weaken a test or gate to go green. Never hide a gate behind `|| true`: `bin/check_all_selftest.py verdict` catches it.
- Push an empty commit, or close and reopen a PR, to re-run CI.
- Delete or rewrite another agent's branch, PR or record.
