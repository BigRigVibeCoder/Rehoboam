---
id: VER-SPR-005
title: "Audit of SPR-005"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-005, VER-SPR-005-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** PASS. SPR-005 gives `rh-core` its time model and the contract every system implements:
> - `SimTime` with 360/365-day calendars and pulses from Year to Hour;
> - the twelve phases of `05.1` §4.2;
> - the `SystemModel` trait, which the generated skeletons compile against;
> - `AccessSpec`, with a registration checker that shares one rule with SPR-004's ownership lint;
> - a scheduler derived from declared reads and writes, which runs dependency levels in parallel and gives identical results at every thread count.
>
> The Developer's own decisions became contract text by Architect ruling: the canonical order (levels, then Registry id), how a system's RNG `system_id` is derived, the frozen `step` signature, and the lag kinds. V-DET-01 and V-DET-02 pass on a stub world; they count toward G0 only on SPR-007's real hashes. Change class C3. The Architect merges (GOV-005 §6).

# VER-SPR-005 — Audit of SPR-005

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | PASS (141 checks) | Developer and Tester at `fc635dc`; Architect at the audit commit |
| CI on GitHub-hosted runners | PASS at `fc635dc` (run 38889074740) | PR #21 |
| Change class | C3: determinism machinery (phase order and scheduler order are part of the hash contract); no golden world exists yet | — |

## 2. Tester verdicts
| Round | Head | Result |
|---|---|---|
| 1 | `1e00b2b` / `ed9bd5b` (docs only) | All six tasks met their acceptance tests. 107 checks; 28 gate mutations. DEF-1 to DEF-5 (low: diagnostics, a lint/checker disagreement, docs). DEF-6 (medium): six plausible breakages survived the whole suite. OBS: `check_all` ignored `CARGO_TARGET_DIR`. Gaps G-a to G-d |
| 2 | `f89c791` (the rebase onto main `026ddb7` as `b5b91bc`, plus the fixes) | DEF-1 to DEF-6 closed. **DEF-7 (medium):** the Architect's coarse-first pulse ruling let a `same_instant` reader on a coarser pulse see the previous day's value |
| 3 | `fc635dc` (report `e493bf1`) | **PASS.** DEF-7 closed: the Hour→Day→Month→Year chain gives 749 observations, 0 stale. A 600-world serial oracle agrees on acceptance, levels, order and state. 601 monthly hashes are identical at 1, 2, 3, 4, 7 and 8 threads × 3 repeats, in debug and release. Every DEF-6 mutation dies, and 50 more planted mutations all die. 20,000 garbage registrations are refused cleanly. The rebase diff is clean |

## 3. Rulings during the sprint (Architect)
| # | Question | Ruling |
|---|---|---|
| 1 | Where the shared ownership rule lives | `rh_core::ownership`, pure functions over data. The crate edge runs rh-registry → rh-core, and rh-core has no rh-registry edge, not even a dev edge; real-spec tests live in `rh-registry/tests` |
| 2 | A system's RNG `system_id` | The high 32 bits of `h64_bytes(tag("rehoboam/rh-core/system/v1"), id)`. A collision is a registration error. Pinned by golden lines from the independent `gen_rng_golden.py` (`05.1` §6 item 2) |
| 3 | The canonical order | Within a phase, the scheduling unit is the **binding** (system, pulse). A `same_instant` read gives an edge and a lag none. Levels (the longest path) are computed once over every binding in the phase. Ties go to the coarser pulse, then the Registry id. A level is split where one system's second binding appears. EventIds follow this order at any thread count (`05.1` §2 rule 4). A diamond-plus-chain test fails under Kahn-by-smallest-id. An earlier ruling, coarsest pulse first, was withdrawn after DEF-7 |
| 4 | The `step` signature | `step(binding: PulseBinding, &mut SystemView, &mut StepCtx)`, with `invariants(&WorldTables)` and `rng_purposes()`. A system may bind one pulse in two phases, and `SystemView` enforces `AccessSpec` at run time (`05.1` §2) |
| 5 | The lag kinds | `same_instant` creates an edge. `lagged`, `start_of_day` and `previous_pulse` do not, and are defined in `05.1` §2 rule 4. P0 reads all three at the start of the instant (documented); exact semantics follow in BCK-002 A-28 before P1a |
| 6 | Code versus spec | Registration compares each system's accesses, RNG purposes, pulse bindings and read lags with its spec; any mismatch is a registration error |
| 7 | Step errors and columns | A step error poisons the engine, and later advances return the original error. Columns are materialised only at registration, and a run-time access to a missing column is fatal |
| 8 | Stub world and carry-forwards | Accepted for P0: an i64 store, a stub hash and epoch 0. `AccessSpec` commands and event types go to SPR-006; the `include!` escape to BCK-002 A-27 |

## 4. Defects and gaps: disposition
| Item | Disposition |
|---|---|
| DEF-1 the cycle message names systems downstream of the cycle | Fixed (`0687f04`): one violation per strongly connected component of size 2 or more |
| DEF-2 duplicate or self-referential violation lines | Fixed (`bda13d3`): rule `duplicate`; no system paired with itself; each violation reported once |
| DEF-3 the lint and the checker disagree on a duplicate write | Fixed (`c47224b`): both run on one fixture set; new control `ownership/duplicate_write.access.toml` |
| DEF-4 a README row outside its table | Fixed (`c08c378`) |
| DEF-5 `CLAUDE.md` lists 3 of 6 rh-core test binaries | Fixed (`5e1a703`) |
| OBS `check_all` ignores `CARGO_TARGET_DIR` | Fixed (`dedca52`); round 3 ran with a fresh `CARGO_TARGET_DIR` |
| G-a schedules and lags not compared with the spec | Ruling 6; fixed (`5d90e56`, registration rule `spec`) |
| G-b lag kinds undefined | Ruling 5; documented (`7525609`). **Residual:** the approximation is exact for S5 (`start_of_day` on a Day pulse), but not for `previous_pulse` (S6, S7) or for a `start_of_day` reader on an Hour pulse. Ruling: as a first step of BCK-002 A-28, `Engine::new` refuses those reads with an explicit `Unsupported` error until the exact semantics land, so no system can run on the approximation unawares. This lands in SPR-006's fix round |
| G-c silent column creation | Ruling 7; fixed (`c380a4c`) |
| G-d advance after a step error | Ruling 7; fixed (`c37d6b3`) |
| DEF-6 six plausible breakages survived (lag semantics, the instant in draw keys, error order, pulse order, the stub-hash golden, system planes) | Fixed (`f89c791`): `tests/engine_contract.rs` (a serial oracle plus one case each) and a golden of 120 stub-world hashes; every mutation now dies |
| DEF-7 a stale `same_instant` read across pulses (caused by the Architect's own pulse ruling) | Fixed (`fc635dc`): the binding is the scheduling unit (ruling 3) |
| GAP-5 season starts and no leap years were in code only | Closed in this audit: `05.1` §4.1 |
| GAP-6 EventId order untestable before events exist | SPR-006 repeats the thread-count probe on EventIds |
| OBS-1 to OBS-6 (an equivalent mutant; CRLF normalised by `gen_skeletons --check`; first-match lag comparison; odd names accepted when the spec agrees; undeclared pre-existing columns; a conservative `same_instant` refusal) | No action. OBS-5 is revisited when typed tables arrive |

**Escapes found later:** none so far.

## 5. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-01 | Property tests on calendar arithmetic | Yes (an odometer oracle over 4 calendars, ±3 years, every minute; extremes; negative time) |
| T-02 | Snapshot of the phase table | Yes (snapshot, `Ord`, names and the `05.1` table agree) |
| T-03 | The S5 and M2 skeletons compile against the trait | Yes (all four skeletons S5, M2, S6 and S7, regenerated independently; 12 compile-fail probes) |
| T-04 | Two writers of one column fail registration | Yes (each conflict refused once, with its rule named, in any registration order) |
| T-05 | Order identical across runs and thread counts | Yes (601 monthly hashes at 6 thread counts × 3 repeats; a 600-world oracle) |
| T-06 | An undeclared RNG purpose fails registration; an unused spec purpose is reported | Yes (an undeclared purpose is refused; an unused one is reported; system_id reproduced independently) |

## 6. Contract edits in this sprint
- `05.1` (v1.5): §2 (the frozen trait; rule 4, the canonical order over bindings and the lag kinds), §4.1 (season starts, no leap years; added in this audit) and §6 item 2 (system_id).
- BCK-002 (v1.4.0): A-28 (exact `start_of_day` and `previous_pulse`), A-29 (draws keyed by entity epoch), A-30 (same-instant conflicts at lifecycle application; filed from SPR-006).
- Decisions log: the SPR-005 rulings.

## 7. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** |
| Merge route | The Architect merges (GOV-005 §6) |
| Carried forward | SPR-006 (AccessSpec commands and events); BCK-002 A-27, A-28; V-DET-01/02 on real hashes at SPR-008 |
| Escapes found later | — |
