# Project status report: 2026-10-18 (Architect, AGT-001)

*Evidence date 2026-10-18 about 19:45 local. `main` is `01690f6d`. Every figure below was read from the repository, GitHub or a command run for this report; anything not verified says so.*

## 1. Bottom line

1. **Nothing is merged since the five foundation sprints.** Wave 5 (SPR-015, SPR-016) is the critical path to G1a, and both are blocked: SPR-015 is red on all three CI parts, and SPR-016 is 232 commits behind `main` and held behind SPR-015.
2. **G1a is the risk, not the schedule.** `kingdom_10k` must run 1,000 sim-years in 60 s. HANDOFF §8 puts it at about 1,530 s after SPR-038a and projects 80 to 117 s after SPR-038b, a miss even if the projection holds. SPR-015 then raised the quiet-instant cost by 62% (§4). The director should expect to be asked to move the G1a performance bar or accept A-68's design change; the Architect brings that with a recommendation at SPR-030a, not before.
3. **The lanes the inbox called "active" are thinner than reported.** No remote branch exists for SPR-027 or SPR-021a; the red-team probe and dossier are on an unmerged branch (§5).
4. **Records are behind the merges.** SPR-038a and SPR-020 merged with no Tester record and (038a) no `VER-` file on disk (§6).

## 2. Sprint ledger

Statuses are the sprint documents' own `status:` fields, checked against branches and PRs.

| Sprint | Doc status | Reality | Blocker / next |
|:---|:---|:---|:---|
| SPR-000 to SPR-014 | CLOSED | Merged | None |
| SPR-015 births F1 | APPROVED | PR #107 draft, head `cda5d16d`, T-01..T-04 committed, **CI red (3/3)** | `VER-SPR-015-INTERIM` §3. Developer fixes first; Tester pass is owed |
| SPR-016 mortality F1 | APPROVED | PR #88 draft, head `da6c74d`, stage 1 only, **232 commits behind `main`** | Waits for SPR-015. Merging `main` in (by gate name) will be large; plan for it |
| SPR-017 | CLOSED | Merged (#90) | None |
| SPR-018 worldgen-lite | APPROVED | No branch | Wave 6; after SPR-016 |
| SPR-019 population synthesis | APPROVED | No branch | Wave 6; stage B waited on A-62 (done) |
| SPR-020 | CLOSED | Merged (#84); no Tester record (§6) | Backfill |
| SPR-021a/b, 022a/b | APPROVED | **No branch.** Specs unblocked (T-00b merged) | Needs SPR-019 stage A first (D58 item 5) |
| SPR-023 to SPR-025 minds | APPROVED | No branch | T1-I/S spec PR first |
| SPR-026 | CLOSED | Merged (#82) | None |
| SPR-027 materializer | APPROVED | **No remote branch.** `pm_inbox.md` reports a local one at T-03 | Push it, or it does not exist for handoff purposes (GOV-011) |
| SPR-028 to SPR-030 | APPROVED | No branch | Wave 8; SPR-030a probe decides A-68 |
| SPR-033a viewer core | APPROVED | PR #111 draft, head `17f6ca7`: core, golden, viewer checks green, **stream red** | §3 |
| SPR-033b, 034, 035 | APPROVED | No branch | 033b's Bevy 0.20.0 enters the lock at or after 2026-10-22 22:31Z |
| SPR-038a | APPROVED (the plan) | Merged (#104); no `VER-SPR-038a` | §6 |
| SPR-038b | (in SPR-038 plan) | Not started | Needed for G1a; carries K4, K9 (A-74) |

Open PRs: #107 (SPR-015), #88 (SPR-016), #111 (SPR-033a), #112 (this record).

## 3. PR #111 (viewer core)

- Core, golden and `bin/check_viewer.sh` are green. Stream failed: `rh-cli::serve probe_c_the_ladder_with_viewers_attached_is_the_ladder_without` hit `ConnectionRefused` in `tests/support/mod.rs:173`. The PR touches no `crates/` path, so a defect it introduced is unlikely, and a timing race in the serve test's startup is likely. **Not verified.** A rerun of the one job is cheap; I have not spent the minutes (D62: the engine core wins contention) and recommend one rerun when PR #107 is quiet. If it fails twice, it is a flaky-test defect on `main` (A-81).
- Isolation, per the diffstat: viewer code is under `viewer/`; the root changes are the lint, scope rule, workflow, docs and fixtures. I have not run `bin/check_viewer.sh` locally.
- **Class check:** the PR edits `bin/check_all.sh`, `bin/lint_arch.py` and `CLAUDE.md` and adds gates. `docs/13` §4.2 keeps C0 for tooling "that does not change a gate", so the PR is above C0; whether C1 fits a change to the shared gate list is for the Architect to rule at audit (the title says C1).

## 4. Code status (what I measured on `main` `01690f6d`)

| Check | Result |
|:---|:---|
| `bin/lint_arch.py` | 17 crates, 72 workspace edges, 15 engine crates against 26 deny-listed crates, **0 violations** |
| `bin/lint_forbidden.py` | Passes. Two logged escapes, both tests reading an env override for a golden file |
| `bin/gen_manifest.py --check` | 256 documents, 0 errors |
| `cargo fmt --all --check` | Clean |
| CI on `main` | Last three runs green (`01690f6d`, `6b1970db`, `d50f1af5`) |
| `unsafe` | Plane-A crates carry `#![forbid(unsafe_code)]` (spot check: `rh-worldgen`). The only real `unsafe` is `rh-bench/src/alloc_count.rs` (18 sites, the counting allocator, bench-only) |
| `.unwrap()` in `src/` | 64, and in the three largest files every one is after `#[cfg(test)]`, so none in shipping paths. Not checked for the other files |
| `TODO`, `FIXME`, `todo!`, `unimplemented!` | 0 |

Size (lines of Rust, `crates/`): `rh-core` 21.3k, `rh-manifest` 11.9k, `rh-golden` 9.5k, `rh-snapshot` 9.4k, `rh-registry` 8.3k, `rh-stream` 8.1k, `rh-cli` 4.7k, `rh-protocol` 3.2k, `rh-ids` 2.7k, `rh-bench` 2.6k, `rh-events` 2.2k, `rh-worldgen` 2.2k, `rh-sample` 2.2k, `rh-sys-demography` 1.9k, `rh-sys-smoke` 1.8k, `rh-sifter` 1.3k, `rh-contracts` 0.9k; about 94k in all. No simulation system beyond M2 rung 0 exists yet: the engine core is large, and the game on top of it is small. That is the right order, but it means G1a's content (births, dynasties, minds) is all still ahead.

**What this is not.** It is not a code review. I read no function bodies on `main`, and I did not run `cargo clippy`, `cargo nextest` or any golden gate; those are CI's job and `main`'s CI is green. The independent code review that GOV-011 §4 requires at audit has not been done for SPR-015, which is not ready for it.

**SPR-015 head `cda5d16d`** (reviewed by diffstat and CI only): `rh-sys-demography/src/demography.rs` grows by 1,151 lines and M2's spec by 979. That is one file carrying fertility, partnering and households. GOV-003 asks for small, readable units; the audit will check function length and complexity (the `ruff`-style limits do not cover Rust, so this is a judgement). The quiet-instant cost (+62%) is the first thing to explain.

## 5. Findings that change the plan

| # | Finding | Evidence | Action |
|:---|:---|:---|:---|
| F1 | SPR-015 T-01 was a copy, not a move; `next/M2` remains | Pytest failure reproduced; `lint_odd --next` output | Work order rev 2, step 1 |
| F2 | The first CI run's registry fixture failure is real, but it was only one of two Python failures | Reproduced | Work order rev 2, step 2 |
| F3 | Quiet instant +62% (14,306 to 23,230) with no work in it | `V-PERF-04` report on `cda5d16d` | Explain before re-recording (work order step 3) |
| F4 | Red-team probe `B09-01` and Batch 09 dossier are on `arch/red-team-batch-04` only, 43 commits ahead, no PR, not in any gate | `git cat-file`, `grep` of `check_all.sh` | A-80: open a PR for the dossier; decide whether the probe becomes a gate |
| F5 | The Stage A checkpoint was skipped: T-04 pushed within the hour | `git log` times | Recorded in the work order |
| F6 | SPR-027 and SPR-021a have no remote branch | `git ls-remote` | Ask the owners to push, or reassign |
| F7 | SPR-016 is 232 commits behind `main` | `git rev-list` | Merge `main` after SPR-015 |
| F8 | PR #111's stream gate failed on a probable flaky serve test | CI report | A-81 |

## 6. Governance gaps (records, not code)

- **SPR-038a:** merged as PR #104 with no `VER-SPR-038a` and no Tester record on disk. HANDOFF §5 step 2 called for a Tester phase 2 and the Architect's audit; the rulings were relayed by message (`rulings_by_message.md`), which GOV-011 §1 says binds only once recorded.
- **SPR-020:** `VER-SPR-020` lists `tester` among its agents, but its evidence rows are Architect runs; no `VER-SPR-020-TEST` exists.
- **Sprint documents lag:** SPR-038's status is `APPROVED` though stage A is merged.
- **Proposal (A-78):** the Architect backfills `VER-SPR-038a` from `rulings_by_message.md` and the PR, and commissions an independent Tester pass on SPR-020 and SPR-038a (different model from the developers'), cheap now and expensive after Wave 6 builds on them. I do not mark either sprint audited retroactively.

## 7. Backlog (BCK-002)

- 77 items before this session (A-01 to A-77); this session files A-78 to A-82.
- Items open and aimed at a named sprint: A-74 (to SPR-038b), A-75 (SPR-021a), A-76 (SPR-015 N7), A-77 (survivors), A-67 and A-68 (G1a performance), A-71 (SPR-034).
- Open with no sprint: A-42's remainder, A-69, A-70, A-72, A-73. These need an owner by the Wave 6 plan, or an explicit deferral.
- BCK-001 is `ACTIVE` but unread in this pass.

## 8. Decisions for the director

None are blocking this week. The queue in `docs/decisions_pending.md` is clear. Two to expect, with my recommendation:

1. **G1a's performance bar** (when SPR-030a reports): move the bar or approve A-68's incremental digest (C3). I believe the digest change, 60% confidence, because the arithmetic in SPR-038's plan puts the residual cost there.
2. **Retroactive Tester passes for SPR-020 and SPR-038a** (§6): I recommend yes, 85%. It costs a Tester session each and removes a hole under the whole wave.

## 9. Next seven days

1. Developer fixes SPR-015 per work order rev 2 (Exuberant).
2. Tester session on SPR-015 Stage A+B once CI is green.
3. Architect: audit `VER-SPR-015`, merge; then SPR-016 merges `main` by gate name and finishes stage 2.
4. Open the PR for `arch/red-team-batch-04`; decide the probe's gate.
5. Rerun PR #111's stream job once; audit and merge it.
6. Ask the SPR-027 and SPR-021a owners to push.
7. Plan Wave 6 sprints (SPR-018, SPR-019) for dispatch the day SPR-016 merges.
