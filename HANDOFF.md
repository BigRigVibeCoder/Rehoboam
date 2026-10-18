# HANDOFF: Rehoboam, 2026-10-18 (the Architect's lane, paused at the usage limit)

> **Read this first, then `AGENTS.md` and `CLAUDE.md`.** This file is a snapshot of in-flight work for the developer who picks up the project. The repository's own records (CODEX, `docs/`, PRs) are authoritative; this file tells you where they are and what to do next, in order.
>
> - **How to work alongside the other agents** is now written down:
>   - `CODEX/10_GOVERNANCE/GOV-009`: Git and GitHub (branches, commits, PRs, merges);
>   - `GOV-010`: CI;
>   - `GOV-011`: ownership, independence, records and handoffs.
>
>   §9 below applies them to this handoff. `CLAUDE.md` binds every agent whatever its vendor; `GEMINI.md` points here.
> - **main:** `66ec301` (the merge of PR #101, 2026-10-18), then this handoff's own merge; check it with `git ls-remote origin main`. Every in-flight branch is pushed, and every remote session pushed its handoff branch except SPR-020's (§6).
> - **Priority (the director's D62, binding):**
>   - **P1:** the engine core (P1a "Bloodlines", gate G1a), which wins every contention for CI minutes, review time and merge slots.
>   - **P2:** the voxel viewer lane.
>   - **P3:** era packs, after G2.
> - **Waiting on the director:** D58, D59, D61 and D63 (`docs/decisions_pending.md`), and one question: what is the repository's real GitHub Actions allowance (D61 item 8)?

---

## 1. What you need to know in five minutes

- **What it is.** Rehoboam is a deterministic historical-simulation engine (Rust workspace in `crates/`), with exact Python reference implementations of every algorithm (`specs/`), executable contracts, and a strict governance model in `CODEX/`. The plan is in `docs/` (start with `docs/00_executive_summary.md`). The director (a human, they/them) owns ADRs, `docs/11_roadmap.md`, the D-items and the non-negotiables in `CLAUDE.md`.
- **How work runs** (ADR-0032; `AGENTS.md`, `CODEX/80_AGENTS/`, `CODEX/10_GOVERNANCE/`):
  - **The Architect** (the role being handed over) writes or commissions plans and spec PRs, rules on findings, audits every sprint and merges every PR.
  - **A Developer** implements a sprint on `sprint/SPR-NNN-*`.
  - **A Tester**, on a different model from the Developer's, verifies independently and writes `CODEX/40_VERIFICATION/VER-SPR-NNN-TEST.md`.
  - **Every plan and spec PR** gets an independent review, recorded as `REV-*`.
  - **The Architect's audit** is `VER-SPR-NNN.md`.
- **The checks.** Run `bin/check_all.sh` (three parts: `--part core`, `--part golden`, `--part stream`; CI runs them as parallel jobs). Every gate also runs on a planted violation that must FAIL. `CLAUDE.md` lists every check. The Python environment: `uv sync --frozen --project python`, then `python/.venv/bin/python`.
- **Hard rules you will hit:**
  - **Determinism:** keyed RNG only, fixed point for thresholds, no `HashMap` iteration in authoritative code.
  - **Generated files are never typed.** Golden baselines change only in a `[C3]` commit via `bin/regen_baseline.sh`. The amendments lock changes only via `lint_odd.py --next … --write-amendments-lock`. Goldens change only via each reference's `--write`.
  - `CODEX/00_INDEX/MANIFEST.yaml` is generated and **never committed**.
  - **Merging `main` into a sprint branch:** `bin/check_all.sh` and `python/tests/test_check_all_inventory.py` conflict often. Resolve **by gate name** (rule at `bin/check_all.sh:14–17`): keep every gate from both sides, each in exactly one part. A plain text merge can silently drop a re-wrapped gate; `python -I bin/check_all_selftest.py inventory` must say "both ways".
  - **Git (GOV-009):**
    - never push to `main`;
    - never force-push, and never rebase or amend a pushed branch;
    - bring `main` in with `git merge origin/main`;
    - merge PRs with a merge commit, pinned to the full head SHA that CI passed (`gh pr merge <N> --merge --match-head-commit <sha>`).
  - **PR body:** follow `.github/pull_request_template.md`: Sprint (with the new `Agent:` line), Compliance, Tasks, Verification, Merge route.
  - **Commits:** this session ended its commits with its own attribution trailers. Use your own tool's attribution, and put no model identifiers anywhere else in the repository.
  - **Disk:** a cargo `target/` is 8–11 GB, and two concurrent `check_all` runs filled a 40 GB container today. Run one cargo-building `check_all` at a time per machine, and delete `target/` dirs you no longer need.
  - **CI time:** each part's target is 12 minutes, the hard limit 15.

## 2. State of `main` and what merged on 2026-10-18

| PR | What |
|---|---|
| #89 | The third CI part (stream), path-scoped references, the two-way inventory, baseline controls (BCK-002 A-63..A-66) |
| #92 | D62 recorded (the director's priorities and viewer answers) |
| #93 | Minds sprint plans SPR-023, SPR-024a/b, SPR-025 (REV-MINDS; D59 to the director) |
| #94 | A-62: M2's elite fertility read on the couple |
| #95 | Wave-7 plans SPR-021a/b, SPR-022a/b (REV-WAVE7; D58), D60 approved (SPR-038 plan) |
| #96 | Red-team batch-08 dispositions |
| #97 | Wave-8 plans SPR-028, SPR-029, SPR-030 (REV-WAVE8; D61) |
| #98 | **T-1A** spec: `specs/systems/next/T1.minds.odd.toml` 0.4.0 A-steps, `specs/minds/selftest_minds_v0.py`; REV-T1A; SPR-023 0.3.1 |
| #99 | **T-00a** spec: EVO-002 (the instant log, the genealogy index), `next/M7` 0.6.0, the `[[observes]]` lint, `specs/engine/instant_log_model.py`; REV-T00a |
| #100 | Viewer plans SPR-033a/b, SPR-034, SPR-035 (revision 2); PRJ-003 v1.4.2; REV-VIEWER CLOSED; D63 filed; BCK-002 1.22.1 (A-68..A-71). Merged `fc979ee` |
| #101 | PY-PKG: the uv project packaged (`uv_build==0.11.32`, `rehoboam_tools`, layout tests); fixes P1–P7. Unblocks SPR-027 T-05, SPR-028 T-02, SPR-029 T-02. Merged `66ec301` |
| this PR | `HANDOFF.md`, `docs/handoff/2026-10-18/`, GOV-009, GOV-010, GOV-011, GOV-005 2.0.0, `GEMINI.md` |

## 3. Open PRs (on GitHub, `BigRigVibeCoder/Rehoboam`)

| PR | Branch @ head | State | Next action |
|---|---|---|---|
| #83 (draft) | `sprint/SPR-014-lifecycle-queues` @ `1a67ab9` | **P1 critical path.** Tester PASS on `d1e79e4`. The code-review fix batch (K1, K2, K3, K6, K7, K10, K11, K13; one commit each) plus a merge of `main` `3d26090` by gate name are on `backup/SPR-014-dev-handoff-20261018` @ `28ab29e`, a fast-forward of the sprint head. The full `check_all` on `28ab29e` is not confirmed | §5 step 1 |
| #82 (draft) | `sprint/SPR-026-stream-v1` @ `2de1841` | World Stream v1 live edge. Tester phase 4 done (GAP-8 met, merge faithful). Code review: 1 MAJ (C1 quadratic append) + MINs. The developer's C1 fix is a WIP commit, **gates not run**, on `backup/SPR-026-dev-handoff-20261018` @ `2323640` (a fast-forward) | §5 step 4 |
| #84 (draft) | `sprint/SPR-020-envelope` @ `512432a` | The event envelope in the engine. Waits for SPR-014; then merge main, regenerate baselines `[C3] [perf-baseline]` | §5 step 3 |
| #88 (draft) | `sprint/SPR-016-mortality-f1` @ `da6c74d` | Stage 1 (exp/ln Q32_32, the F1 schedule) Tester PASS; stage 2 held until SPR-015 merges | After SPR-015 |
| #90 (draft) | `sprint/SPR-017-effect-language` @ `34a7f56` | The effect language v0: T-01..T-09 done, CI green (golden 14:37 > 12 target). `7d83b2e` (the fuzz control on seeds 1–4) is on `backup/SPR-017-dev-handoff-20261018`, a fast-forward. The Tester's pass-2 draft is on `review/SPR-017-tester-handoff-20261018` @ `0bc28fc`. The independent code review was cut off by the limit | §5 step 5 |

**Branches with work but no PR:**
- `sprint/SPR-038a-per-instant-cost` @ `2116418`: stage A done, built on SPR-014's `1a67ab9`. Opens its PR after SPR-014 merges. `backup/SPR-038a-dev-handoff-20261018` @ `789c138` (a fast-forward) adds `160be40`, the Architect's conditions (T-04), and a WIP draft of the PR body's evidence.
- `arch/AN-0-annals-fold` @ `cfe5961`: the multi-segment annals fold; the reviewed fixes are a WIP commit.
- `arch/AN-1-annals-m7-fixtures` @ `733960b`: annals fixtures for M7 F1. Committed, but not reviewed, and its gates were not fully run.
- `arch/M7-0.6-T-00b` @ `8a5b1ea`: M7 amendments for SPR-022. WIP; its gates were not run.
- `backup/SPR-014-pre-audit` @ `1d351f5`: SPR-014's review fix batch, contained in `28ab29e`.
- `backup/*-handoff-20261018` and `review/*-handoff-20261018`: what the remote sessions pushed at the handoff (§6).

## 4. Records you will need (all in the repo)

- **Plans:**
  - `CODEX/05_PROJECT/SPR-*.md`, each with its specs table, tasks, acceptance and merge route.
  - `CODEX/05_PROJECT/PRJ-002_P1aPlan.md` (P1a: order, graph, §6 spec PRs).
  - `CODEX/05_PROJECT/PRJ-003_BevyVoxelViewer.md` (the viewer lane).
  - `CODEX/05_PROJECT/BCK-002_ArchitectBacklog.md` (the Architect's backlog; A-68..A-71 are on PR #100).
- **Reviews and audits:** `CODEX/40_VERIFICATION/` (`REV-*`, `VER-*`).
- **Director items:** `docs/decisions_pending.md` (waiting) and `docs/decisions-log.md` (decided).
- **Handoff-only records** (`docs/handoff/2026-10-18/`), which exist nowhere else:
  - `spr014_code_review.md` and `spr014_rulings.md`: SPR-014's code review and the Architect's rulings (the developer's fixes are `1d351f5`).
  - `spr026_code_review.md` and `spr026_rulings.md`: SPR-026's code review, the Tester's DEF-2, and the rulings.
  - `an0_drafter_report.md`, `an0_review.md` and `an0_rulings.md`: AN-0 (R1–R11 to apply).
  - `pypkg_drafter_report.md` and `pypkg_review.md`: PY-PKG (already applied on PR #101).
  - `rulings_by_message.md`: the SPR-038a and SPR-017 rulings sent by message (copy them into those audits).
  - `architect_log.md`: the Architect's terse running log, 2026-10-16 to 2026-10-18.

## 5. What to do next, in order (P1 first)

Each step ends with the Architect's audit (a `VER-` record) and a merge with a merge commit. Testers re-check by scope after each fix push.

1. **SPR-014 (lifecycle queues; C3 `[perf-baseline]`), merge first.**
   1. Fast-forward the sprint branch to `28ab29e` (`git push origin 28ab29e:sprint/SPR-014-lifecycle-queues`; it is a descendant, so no force is needed). It already holds the fix batch and a merge of `main` `3d26090` by gate name.
   2. Merge the current `main` (#100, #101 and this handoff: docs and Python packaging, no gate changes expected). Check the baselines with `bin/check_baseline.py`; they should not move.
   3. Run the full `check_all`. The Tester's probes and draft re-check scope are on `review/SPR-014-tester-handoff-20261018` @ `b03cc79` (under `tester-probes/SPR-014-handoff/`; evidence, not for merging).
   4. Push, and mark PR #83 ready.
   5. The Tester re-checks K1, K2, K3, K6, K7, K10 and K11 by scope (the list is in `spr014_rulings.md`).
   6. Write `VER-SPR-014`.
   7. File in BCK-002:
      - K4 and K9, to SPR-038b T-07;
      - K8, to SPR-021a;
      - K5's figures, into A-60, marked part done;
      - K12, to SPR-015 N7;
      - the "a event" wart;
      - the sealed constructor.
   8. Ask the director to delete `review/SPR-014-tester-probes`.
   9. Merge.
2. **SPR-038a (per-instant cost, stage A; C2 `[perf-baseline]`).** Fast-forward the sprint branch to `789c138` (the conditions `160be40` and the evidence draft). Merge main, re-run `check_all`, and open the PR with the before/after table. Then the Tester's phase 2. Check every condition in `rulings_by_message.md`. Audit and merge. SPR-038b follows (D60).
3. **SPR-020 (the event envelope; C3 `[perf-baseline]`).** Merge main by gate name, regenerate the baselines in the final `[C3]` commit, then the Tester, the audit and the merge.
4. **SPR-026 (World Stream v1; C2).**
   1. Start from `2323640` (the developer's WIP C1 fix, gates not run) and finish the batch in `spr026_rulings.md`:
      - **C1:** no per-append copy of the chunk list. Re-measure; at most 5% overhead on solo 3,000 years and hamlet 6,000 years with `--ladder monthly`.
      - **DEF-2:** the accounting test and its two mutants.
      - **The MIN and cheap NIT fixes.**
   2. Merge main.
   3. The Tester re-checks with their "before" probes (`review/SPR-026-tester-handoff-20261018` @ `041659a`).
   4. Write `VER-SPR-026`. File in BCK-002:
      - GAP-2..GAP-7;
      - A-67, with the about 5 MB plateau and C11;
      - C10, to SPR-027 T-04;
      - C16, to the P2 WebSocket.
   5. Merge.
   6. **This merge starts the viewer lane** (SPR-033a), once D63 is approved.
5. **SPR-017 (effect language; C2).**
   1. Rerun the independent code review of `34a7f56`; the first one was cut off.
   2. Let the Tester finish pass 2.
   3. Send one batch: the review findings plus `7d83b2e` (the fuzz control on seeds 1–4, on `backup/SPR-017-dev-handoff-20261018`; fast-forward the sprint branch to it).
   4. If golden is still over 12:00, move the two fuzz gates to part stream.
   5. Write `VER-SPR-017` (rule on deviations 1–10), merge, and ask the director to delete `review/SPR-017-tester-cases`.
6. **Then the wave order** (PRJ-002 §3–§4; the cross-plan ruling X9): SPR-015 (births F1), then SPR-016 stage 2, then wave 6 (SPR-018, SPR-019), then wave 7 (SPR-021a/b, SPR-022a/b, once D58 is approved), and so on. Minds (SPR-023 onward) need D59; wave 8 needs D61.
7. **Spec PRs in flight** (each: finish, independent review on a different model, rulings, close with a `REV-`, PR, merge):
   - **AN-0** (`arch/AN-0-annals-fold`): apply R1–R11 (`an0_rulings.md`); the WIP commit has started R1–R3. Then verify, write `REV-AN0`, open the PR and merge, **before AN-1**.
   - **AN-1** (`arch/AN-1-annals-m7-fixtures`): run its gates, review, merge after AN-0 (the conflicts are mechanical: `annals_v0.md`'s status line and §14, the `CLAUDE.md` annals line, `check_all`'s annals block). AN-0's control needs `solitary` to keep its two segments at y000000 and y000003.
   - **T-00b** (`arch/M7-0.6-T-00b`): finish the draft (its brief is in the commit message and in PRJ-002 §6's T-00b row), regenerate the lock and the model's goldens with their tools, review, merge. It must merge before SPR-021a's T-01.
   - **T1-I/S** (the minds I- and S-steps): after SPR-023's T-01 promotes T1 0.4.0.
8. **The Architect's plan edits that are owed:**
   - **SPR-029:** port AN-0's R7 calendar check into T-03; R12's wording; use `python -P -s -m` for the two-process determinism check (PY-PKG review P3); swapped blocks are refused with canonical's message (AN-0 F2).
   - **SPR-028:** the same `-P -s` note; new workflows run `uv sync` and `python/.venv/bin/python`, with `setup-uv` pinned to 0.11.32.
   - **SPR-027 T-05:** in PY-PKG's layout, one `module-name` line plus the textual pin; `__main__ --help` exits 0.
   - **SPR-038b's T-list:** stage A's remainder (the O(N+L) checks, the guard's O(L) checks), K4 and K9.
   - **GOV-008 §2:** the layout line, at its next revision (director-owned).

## 6. Remote work sessions (Claude Code on the web) at the handoff

Each session was asked to push its unpushed work, reply and stop. These arrived (checked with `git ls-remote` at about 12:00 UTC). Each `backup/` and `review/` head is a fast-forward of its sprint branch's head.

| Role | Branch @ head | What it holds |
|---|---|---|
| SPR-014 developer | `backup/SPR-014-dev-handoff-20261018` @ `28ab29e` | The fix batch `1d351f5` plus a merge of `main` `3d26090` by gate name |
| SPR-014 tester | `review/SPR-014-tester-handoff-20261018` @ `b03cc79` | Unpushed probes (`tester_death.rs`, `tester_spr014.rs`), a draft `VER-SPR-014-TEST` and the re-check scope, under `tester-probes/SPR-014-handoff/` |
| SPR-026 developer | `backup/SPR-026-dev-handoff-20261018` @ `2323640` | WIP: C1, a bounded `LiveEdge` append. **Gates not run** |
| SPR-026 tester | `review/SPR-026-tester-handoff-20261018` @ `041659a` | The re-check probes and the "before" readings |
| SPR-017 developer | `backup/SPR-017-dev-handoff-20261018` @ `7d83b2e` | T-08: the fuzz control on seeds 1–4 |
| SPR-017 tester | `review/SPR-017-tester-handoff-20261018` @ `0bc28fc` | The pass-2 draft of `VER-SPR-017-TEST` on `34a7f56`, and the probes |
| SPR-038a developer | `backup/SPR-038a-dev-handoff-20261018` @ `789c138` | `160be40` (the Architect's conditions) and a WIP evidence draft |
| SPR-020 developer | **no handoff push** | `sprint/SPR-020-envelope` @ `512432a` is the last known state |

These sessions are stopped. They push nothing more unless the director restarts them (§9).

## 7. Director decisions waiting (`docs/decisions_pending.md`)

| ID | What | Architect's recommendation |
|---|---|---|
| D58 | Wave-7 plans SPR-021a/b, SPR-022a/b (M7 succession, ruler exits, dormancy); sizes, classes, spec prep, Registry defaults, requirement texts, CI, shared owners | Approve (65%) |
| D59 | Minds plans SPR-023, SPR-024a/b, SPR-025; the ADR-0002 §2 addendum (columns, not `bevy_ecs`, in P1a) | Approve (70%) |
| D61 | Wave-8 plans SPR-028, SPR-029, SPR-030 (the M9 harness, family annals, the G1a run); **plus the question of the real Actions allowance** | Approve (70%) |
| D63 | Viewer plans SPR-033a/b, SPR-034, SPR-035 (on PR #100) | Approve (75%) |

No sprint starts without its plan's approval (GOV-005 §2). Spec PRs (AN-0, AN-1, T-00b) may proceed meanwhile.

## 8. Known risks and lessons from this session

- **CI minutes.** The PR tier ran 462 times in 7 days; the real allowance is unknown (D61 item 8). The viewer lane's scheduled runs are twice a week by design (SPR-033 F17).
- **G1a performance.** `kingdom_10k` must run 1,000 sim-years in at most 60 s. After SPR-038a it is about 1,530 s; the projection after SPR-038b is 80–117 s. BCK-002 A-68 (the residual digest) is ruled on SPR-030a's probe. Expect a miss; plan for it.
- **Merges of `main` into long-lived sprint branches** are the most common source of silent breakage (the inventory trap). Always merge by gate name and run the inventory self-check.
- **Independent reviews find real defects.** Today's included a quadratic append in the live edge, a fail-open registry, and a pty corrupting a data stream. Do not skip the review step for spec PRs or the code review at audit.

## 9. Working alongside the other agents (GOV-009, GOV-010, GOV-011)

- **You are the Architect** from this handoff until the director says otherwise (GOV-011 §2). Every merge, ruling and audit is yours, under the same limits:
  - the director approves C2/C3 plans, ADRs, D-items, `GOV-` and `AGT-` documents, and the non-negotiables;
  - nobody types a generated file.
- **Ownership of the branches above.**
  - Every Claude session is stopped, so every branch in §3 and §6 is yours to assign.
  - When you or a Developer you start take one, record it with the PR's `Agent:` line (the PR template now has one) or by opening its draft PR (GOV-011 §3).
  - Fold `backup/` heads into their sprint branches by fast-forward (§5), then delete the `backup/` branches.
  - Leave `review/` branches for the director to delete when their sprint closes.
- **If the Claude lane comes back** (its weekly limit resets 2026-10-24, 02:00 UTC):
  - it starts by reading this file, the open PRs and your log;
  - it takes only what you assign it;
  - it never pushes to a branch whose `Agent:` line names you.
  - Use it, or any other model family, as the **independent** Tester or reviewer of your work (GOV-011 §4); you can do the same for its work.
- **Keep a dated log** in `docs/handoff/<date>/`. Replace this file at your own handoff (GOV-011 §7), so the next agent, of any vendor, can pick up from it.
- **Tools.** This session used GitHub's API through its own tools. The `gh` equivalents:

  | Task | Command |
  |---|---|
  | Open PRs | `gh pr list` |
  | CI on a PR's head | `gh pr checks <N>`, or `gh pr view <N> --json headRefOid,statusCheckRollup` |
  | Per-gate report | `gh run download <run-id> -n check-report-<part>` |
  | Open a draft PR | `gh pr create --draft --base main --head <branch> --title "…" --body-file <file>` |
  | Merge | `gh pr merge <N> --merge --match-head-commit <full-sha> --subject "Merge PR #<N>: … [C<n>]"` |

