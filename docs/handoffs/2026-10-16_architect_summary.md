# Architect summary: 2026-10-16 (P0, before G0)

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
| SPR-007 | Plane hashes, keyframes (Arrow IPC with zstd), the SQLite journal, the unsynced guard | PR #25, final fix round, then Tester round 2b and the audit | Draft |
| SPR-008 | Golden-run CI: smoke world, hash ladders, bisect | PR #28. T-00, the smoke world and `StepCtx::stream` are done (CI green); the rest waits for SPR-007 to merge | — |
| SPR-009 | Benchmarks and the regression gate | Rulings merged (#29); launches when SPR-007 merges | — |
| SPR-010 | `rh-core::fixed` | Merged (#5) | VER-SPR-010 PASS |

Escapes found after a sprint closed: 1 (documentation only) across 8 closed sprints.

## 2. SPR-007 in detail

- **Round 1** (hashes, journal, unsynced guard): passed independent test. The Tester's own XXH3 Merkle implementation reproduced every layout line and stub-world hash. The journal survived 60 kill-9 rounds and caught 22 tamper cases.
- **T-02** (keyframes) passed V-DET-05: a loaded keyframe advanced ten years equal to the continuous run, at 47 cut points.
- **Round 2** found:
  - **GAP-6** (my own ruling, corrected): undrained events were in `state_hash`, which made the hash depend on when the host drains them. They are now carried in keyframes, outside the hash.
  - **DEF-3**: a forged Arrow body under a valid envelope crashed the loader, aborts included. Fixed with a metadata guard ahead of Arrow, which held over all 93,648 single-bit flips.
  - **DEF-3b**, found in my review: the guard itself panicked on a negative length, a value no single bit flip can reach. Its fix is in the final round.
  - **DEF-4, GAP-7, GAP-8, GAP-9** (fixed in `24d2262`): keyframe history taken from the engine, a journal-anchor check, all five history fields tested, and restore refusing a timer dated in the past.
  - **DEF-5** (high; found by the Tester and, independently, by the new write-time read-back): keyframes of any world with 150 or more rows failed to load, because agenda tombstones were decoded in the wrong order. Fixed in `24d2262`.
- **Lesson:** every keyframe test used a 48-row world. The final round adds a property test over world sizes from 48 to 5,000 rows.

## 3. Your rulings at G0 (2026-10-16)

All are recorded in `docs/decisions-log.md`:
- the GOV-005 sampler rule is kept;
- Laws 2 and 3 are adopted narrowly, as principles 8 and 9 of `docs/01` §5;
- item 15 goes into a GOV-003 §6 failure-modes table;
- Rerun is dropped from P0, and the TUI stub moves to P1a;
- bench 4's baseline moves to the first P1a field sprint;
- at most 3–4 agent sessions run at once;
- you open the laptop `adjutant` session when SPR-008 is ready.

## 4. Waiting on you

| Item | Recommendation | Confidence |
|---|---|---|
| **The laptop:** RUN-001 (BCK-002 A-16) and one `adjutant` session of about 2 hours, for SPR-008 T-02 and SPR-009 T-04 | I will ping you when SPR-008 is ready | — |

Resolved since the first version of this summary:
- you confirmed my reading of Laws 2 and 3;
- you took the demography dossier in as a proposal (red-team batch 05, intake notes N13 to N17).

## 5. Risks

- **The usage limit.** Six agent sessions plus the Architect hit the five-hour limit once (10:03–10:07Z). From now on at most 3–4 sessions are active at once.
- **Tests on small worlds.** DEF-5 escaped two rounds of testing because every keyframe test used one small world. Testers are now asked to vary world size and seed in every property test.
- **Container loss.** Drafts in a cloud container are lost if it is reclaimed. Working state is committed here.

## 6. For a later Architect session

- Sprint sessions report to the Architect by `send_message`. Their roles:

  | Sprint | Developer (AGT-002-CORE) | Tester (AGT-003-DET) |
  |---|---|---|
  | SPR-007 | `session_01WmNv3YSUNTk24yQRdQxAgt` | `session_018tMdk4RQu94uNuTxuxCAER` |
  | SPR-008 | `session_01SoW2LBVGVDsxGFnMPnkoCx` | `session_01MLXhAvqPwGMV9FyKu2uiU6` |

- **SPR-008, when it resumes:** its keyframe-assisted bisect is the first host to replay a journal from a keyframe. It must call `Replay::check_anchor` (SPR-007 GAP-7) before it does.
- **To file in the SPR-007 audit:** BCK-002 A-33. When non-material journaled inputs reach the engine, the engine counts every journaled input it applies, beside `applied_inputs`, and the keyframe anchor compares against that count.
