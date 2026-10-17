# Architect summary: 2026-10-16/17 (G0 passed; P1a starting)

*A living handoff from the Architect (AGT-001) to the director, and to any later Architect session. It records the state of the work, so the work survives the loss of a cloud container. Decisions are recorded in `docs/decisions-log.md`; this file only points to them.*

## 1. Where P0 stands

| Sprint | What | State | Audit |
|---|---|---|---|
| SPR-001 | Repository bootstrap, lints, CI | Merged | VER-SPR-001 PASS |
| SPR-002 | `rh-ids`: IDs, keyed hash, streams, salts | Merged | VER-SPR-002 PASS |
| SPR-003 | Samplers in `rh-sample`, plane A's only float zone | Merged (#16) | VER-SPR-003 PASS |
| SPR-004 | Registry and manifest loaders, ownership lint | Merged (#6) | VER-SPR-004 PASS |
| SPR-005 | Time, phases, `SystemModel`, `AccessSpec`, a parallel scheduler identical at every thread count | Merged (#21) | VER-SPR-005 PASS |
| SPR-006 | Agenda of timers, lifecycle command queues, event bus, invariants with replayable crash artifacts | Merged (#26) | VER-SPR-006 PASS |
| SPR-007 | Plane hashes, keyframes (Arrow IPC with zstd), the SQLite journal, the unsynced guard, `resume` | Merged (#25) | VER-SPR-007 PASS |
| SPR-008 | Golden-run CI: smoke world, hash ladders, bisect | Merged (#28, `3bd2262`); first nightly green. The cross-machine check waits for your session | VER-SPR-008 PASS |
| SPR-009 | Benchmarks and the regression gate | Merged (#34). T-04 (wall-clock baselines) waits for your session | VER-SPR-009 PASS |
| SPR-010 | `rh-core::fixed` | Merged (#5) | VER-SPR-010 PASS |

Escapes found after a sprint closed: 2 across 10 sprints (SPR-002 documentation; SPR-008 a CI concurrency gap, fixed in SPR-009).

**G0 passed on 2026-10-17** (`VER-G0` 1.0.0): all ten P0 sprints closed, all seven criteria pass. P1a starts with SPR-011 (engine prerequisites); SPR-012 and SPR-013 follow. Your session on the laptop and `adjutant` still owes the cross-machine golden check, SPR-009 T-04 and RUN-001; none is a G0 criterion.

**P1a specs on `main`:** `T1.minds` and `M7.dynasty` frozen 0.3.1 (PR #32), M2 rung 0 frozen 0.2.0 with the skeleton specs (PR #37), your G0 rulings applied across the plans and docs (PR #42), and the effect-language review record (PR #43, `REV-SPR-017` CLOSED).

## 2. SPR-007 in detail

- **Round 1** (hashes, journal, unsynced guard): passed independent test. The Tester's own XXH3 Merkle implementation reproduced every layout line and stub-world hash. The journal survived 60 kill-9 rounds and caught 22 tamper cases.
- **T-02** (keyframes) passed V-DET-05: a loaded keyframe advanced ten years equal to the continuous run, at 47 cut points.
- **Round 2** found:
  - **GAP-6** (my own ruling, corrected): undrained events were in `state_hash`, which made the hash depend on when the host drains them. They are now carried in keyframes, outside the hash.
  - **DEF-3**: a forged Arrow body under a valid envelope crashed the loader, aborts included. Fixed with a metadata guard ahead of Arrow, which held over all 93,648 single-bit flips.
  - **DEF-3b**, found in my review: the guard itself panicked on a negative length, a value no single bit flip can reach. Fixed; a structured forge test covers 56 fields.
  - **DEF-4, GAP-7, GAP-8, GAP-9** (fixed in `24d2262`): keyframe history taken from the engine, a journal-anchor check, all five history fields tested, and restore refusing a timer dated in the past.
  - **DEF-5** (high; found by the Tester and, independently, by the new write-time read-back): keyframes of any world with 150 or more rows failed to load, because agenda tombstones were decoded in the wrong order. Fixed in `24d2262`.
  - **GAP-10** (found by the Tester, widened by me): the journal position counted *applied* inputs. Replaying the journal from a keyframe would have applied queued inputs twice, and nothing tested that path. The position is now the count of *accepted* inputs, and `rh_snapshot::resume` is the one host loop that replays from a keyframe.
  - **GAP-11** (test coverage): the guard's size bounds are now pinned by a test; I checked that the mutant fails.
- **Final tester evidence:** about 157,000 forged keyframes in child processes, with 0 panics, 0 aborts and 0 wrong loads; 29,926 dense keyframes; 160 resumes through the journal, all equal.
- **Lessons:**
  - Every keyframe test used a 48-row world, so the tests now vary world size.
  - The journal-replay path had no test at all until GAP-10.

## 3. Your rulings at G0 (2026-10-16)

All are recorded in `docs/decisions-log.md`:
- the GOV-005 sampler rule is kept;
- Laws 2 and 3 are adopted narrowly, as principles 8 and 9 of `docs/01` §5;
- item 15 goes into a GOV-003 §6 failure-modes table;
- Rerun is dropped from P0, and the TUI stub moves to P1a;
- bench 4's baseline moves to the first P1a field sprint;
- at most 3–4 agent sessions run at once (later relaxed: "cap reset, should be good to churn those tokens");
- Fable sessions take independent reviews, spec drafting and Testers of Opus-developed sprints; a Tester always runs on a different model from its Developer;
- you open the laptop `adjutant` session when SPR-008 is ready.

## 4. Waiting on you

| Item | Recommendation | Confidence |
|---|---|---|
| **The laptop:** RUN-001 (BCK-002 A-16) and one `adjutant` session of about 2 hours, for SPR-008 T-02 and SPR-009 T-04 | I will ping you when SPR-008 is ready | — |
| **D37, the Registry effect language v0** (PR #41, v0.5.0, 558 golden cases), with D14 (title blood over the last 16 holders, which needs worldgen to back-fill pre-epoch history in SPR-019), D4 (sex rules) and D12 (bounds) | Approve. Four review rounds on a different model from the drafter's ended with 0 blockers and 0 majors (`REV-SPR-017`) | 75% |

Resolved since the first version of this summary:
- you approved the P1a plan, D30 to D36 (PRJ-002 and SPR-011 approved; PR #32 and PR #37 merged in that order);
- you confirmed my reading of Laws 2 and 3;
- you took the demography dossier in as a proposal (red-team batch 05, intake notes N13 to N17).

## 5. Risks

- **The usage limit.** Six agent sessions plus the Architect hit the five-hour limit once (10:03–10:07Z). You later relaxed the 3–4 session cap, and Fable's separate weekly limit now carries reviews and spec drafting.
- **Tests on small worlds.** DEF-5 escaped two rounds of testing because every keyframe test used one small world. Testers are now asked to vary world size and seed in every property test.
- **Container loss.** Drafts in a cloud container are lost if it is reclaimed. Working state is committed here.

## 6. For a later Architect session

- Sprint sessions report to the Architect by `send_message`. Their roles:

  | Sprint | Developer (AGT-002-CORE) | Tester (AGT-003-DET) |
  |---|---|---|
  | SPR-007 | `session_01WmNv3YSUNTk24yQRdQxAgt` | `session_018tMdk4RQu94uNuTxuxCAER` |
  | SPR-008 | `session_01SoW2LBVGVDsxGFnMPnkoCx` | `session_01MLXhAvqPwGMV9FyKu2uiU6` |
  | SPR-009 | `session_018ggWNPkngHp6tw4r1dimut` | `session_01GwtdHCuGkDRRwPKeF2y8H3` |

  P1a spec sessions:

  | Role | Session | Work |
  |---|---|---|
  | Spec drafter: `T1.minds`, `M7.dynasty` | `session_0197w1yjvJqAKATkCbw9KanD` | PR #32, merged (frozen 0.3.1) |
  | Spec drafter: skeleton specs (M2 freeze, Registry verbs, channel A, `life_ends`, rung manifests) | `session_012dXsVBVUA3yTBQybqVbxe6` | PR #37, merged |
  | Spec drafter (Fable): the Registry effect language v0, with the kinship relations | `session_017JVNkiac6bxeP4MpihBfxd` | PR #41, final; awaits D37 |
  | Independent reviewer (Fable) | `session_0172pYwXgkfpB3tkErgMDHsc` | REV-P1a, rounds 1 and 3 |

- **P1a sessions (2026-10-17):**

  | Role | Session | Work |
  |---|---|---|
  | SPR-011 Developer | `session_01BNeZLa54C84WkCSFyGLyCw` | **Merged** (PR #46, `afca1c6`); `VER-SPR-011` PASS |
  | SPR-011 Tester | `session_01TL91hSqyKqgnewrVi44NrR` | Done: `VER-SPR-011-TEST` PASS (phases 2a, 2b, 3) |
  | SPR-012 Developer | `session_01HayofV6CwcYjQMoz46fUHo` | `solo` lives and dies, `sprint/SPR-012-solo`; scope adds A-47 and A-49 |
  | SPR-012 Tester | `session_01NmJhgWg2RtddDxjWoKF3VP` | Probe plan written from the contracts (with A-47, A-49); waits for the head |
  | SPR-013 spec drafter | `session_0127pPFgvbRfd3gRk2vQrxxY` | PR #45, merged |
  | EVO-001 drafter | `session_01KHKcdcgSqTBuEw1RS6XvH8` | **Merged** (PR #49, `0398a4e`, `05.1` v1.12); `REV-EVO-001` closed after four rounds |
  | M2 F1 drafter | `session_01X232tohbeVxBfHxZkFYQVq` | Births (PR #50): round 3 approved; merging `main` after EVO-001, then the Architect verifies and merges. Q5 (tfr 4.9) and Q6 (V-SF-24/25 gating) then go to the director |
  | SPR-016 drafter | `session_01Na65ZokwKyd82SPMcAGLLn` | **Merged** (PR #51, `513e281`, `05.1` v1.10); next: folding mortality into `next/` M2 after births merges |
  | SPR-018 drafter | `session_01EZrn9bjDkDgj6kEQsYpCuD` | **Merged** (PR #52, `e9e0fe3`); `REV-SPR-018` closed |

- **Review records:** `REV-SPR-016`, `REV-SPR-018` and `REV-EVO-001` are closed. `REV-SPR-015` (`review/SPR-015-r1`) closes when the births spec merges.
- **`05.1` versions:** v1.10 is `exp`/`ln` (PR #51), v1.11 is SPR-011's rule 6, v1.12 is EVO-001's rule 2.
- **M7.dynasty spec version:** EVO-001 took 0.3.2. PR #41 (effect language, awaiting D37) takes 0.3.3, merges `main` and moves its gates to `--part core` before it merges.
- **Outcome-carrying commands** (`ScheduleDeath`, `Marry`) are submitted only by their sender's timer at the outcome's instant (REV-EVO-001 R9). D39 was withdrawn.
- **Messages to a working session:** send with the default priority or `now`. A `next` message waits for the target's turn to end.

- **PR #28 and PR #41 contain merge commits:** merge them with a merge commit, not a rebase.
- **After SPR-008 merges:** dispatch `nightly.yml` on `main`, then give the director the commit and the nightly run for the cross-machine session (`worlds/smoke/README.md`).
- **SPR-008:** its keyframe-assisted bisect must go through `rh_snapshot::resume`, which calls `Replay::check_anchor` (SPR-007 GAP-7, GAP-10). Its journaled input follows the host order: check, journal, submit, one row per input.
- **BCK-002 A-33** (filed): when non-material journaled inputs reach the engine, count them as accepted inputs, like material ones.
