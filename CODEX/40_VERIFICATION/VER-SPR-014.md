---
id: VER-SPR-014
title: "Audit of SPR-014"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-014, VER-SPR-014-TEST, BCK-002, EVO-001, REV-EVO-001, SPR-013, VER-SPR-013, SPR-020, SPR-026, SPR-038]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: PASS.** SPR-014 delivers EVO-001 (`05.1` §2 rule 2) into the engine core:
> - **T-01:** delivery to the single draining binding declared by the owner's spec;
> - **T-02:** event-typed payload slots rewritten at the merge, with hash-neutral encoding;
> - **T-03:** the dead-person drop (BCK-002 A-30), owner-reason drops, and the queue-age invariant (A-31);
> - **T-04:** absent-owner registration rules and neutral lookups;
> - **T-05:** M2's `Day`/`Vital` drain, ledger row insert for births, and single death path;
> - **T-06:** `rh-contracts` v0 macro-generated accessors for M2's columns;
> - **T-07:** A-50 build environment purity (`env!`/`option_env!` ban) and entity kind checks at draw;
> - **T-08:** `smoke` stays byte-identical; `solo`, `household`, and `hamlet` advance only through `engine.meta`'s cursor, re-recorded in the `[C3] [perf-baseline]` commit `f552f10`.
>
> Verification:
> - The Tester's verification on `d1e79e4` (`VER-SPR-014-TEST`) is accepted: 53 of 58 mutants killed, DEF-1 (accessors drift) and DEF-2 (widow spouse clearing) closed.
> - The independent code review on `5ad5288` (0 BLK, 0 MAJ, 7 MIN, 6 NIT) was ruled in `spr014_rulings.md`. The developer's fix batch `1d351f5` and merge of `main` `3d26090` (`28ab29e`), followed by the takeover merge of `main` `69ded24` (`7fc005c`), resolved all required fixes:
>   - K1: `submit` refuses `Arg::Event` when `decl.payload` is `None`;
>   - K2: `lint_forbidden.py` A-50 rule flags renamed `use` of `core::env`, `core::option_env`, `std::option_env`;
>   - K3: Engine enforcement of R9 (`SimError::Timer` for cross-instant outcome commands);
>   - K6: `drain_only` tied to spec step count;
>   - K7: Concurrency test across 1, 2, 4, N threads;
>   - K10 & K11: Registration validation on `key` and spec-less neutrals.
> - All automated checks pass cleanly: `bin/check_all.sh` 100% across all three parts (core, golden, stream), `check_all_selftest.py inventory` 533 gates both ways, pytest 767 passed.
>
> Change class C3 with `[perf-baseline]`. The Architect merges with a merge commit (GOV-005 §6, GOV-009 §6).

# VER-SPR-014: audit of SPR-014

PR #83 merges with a merge commit, because the branch carries merges of `main` and a `[C3] [perf-baseline]` commit whose ranges CI checks.

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh --part core` | **PASS**: 0 FAIL rows. Workspace compiles, all unit tests, spec reference models, and positive controls pass. | Architect local run on `7fc005c` |
| `bin/check_all.sh --part golden` | **PASS**: 0 FAIL rows. Release build, `smoke` (1T & NT), `solo`, `household`, `hamlet` golden runs, ladders, baselines, and V-PERF-11 allocation budgets pass. | Architect local run on `7fc005c` |
| `bin/check_all.sh --part stream` | **PASS**: 0 FAIL rows. Parquet segment verification via DuckDB, `rh-golden`, `rh-cli tail`, `rh-sifter` channel C twin check, and replay pass. | Architect local run on `7fc005c` |
| `python -I bin/check_all_selftest.py inventory` | **PASS**: 533 gates chained and listed, each once, both ways. | Clean inventory pass |
| `pytest python/tests` | **PASS**: 767 passed in 13.68s. | Pytest suite |
| `bin/gen_manifest.py --check` | **PASS**: builds cleanly (243 documents, 0 errors). | Manifest check |
| Golden baselines | `smoke` byte-identical to `main`. `solo`, `household`, and `hamlet` baselines advance only by cursor tick in `engine.meta` as ruled. | `check_baseline.py` |
| Change class | C3 with `[perf-baseline]` | — |

## 2. Tester verdicts
| Head | Result | Notes |
|---|---|---|
| `921d372` | Interim pass | Found DEF-1 (accessors drift test) and DEF-2 (widow's `person.spouse` not cleared). 37 of 56 mutants killed. |
| `d1e79e4` | **PASS** | DEF-1 and DEF-2 verified fixed. 53 of 58 mutants killed. Five non-blocking survivors: K4, M4, M5, M7, S2 (reachability documented in `VER-SPR-014-TEST` §3). |
| `28ab29e` / `7fc005c` | **PASS** | Developer fix batch and Architect takeover merge of `main` verified with full `check_all`. |

## 3. Findings and rulings disposition (K1–K13)
| ID | Finding | Ruling | Status in Head |
|---|---|---|---|
| K1 | `submit` validates payload only when `decl.payload.is_some()`. Untyped accepts `Arg::Event` | FIX. Refuse `Arg::Event` when payload is `None` with typed `SimError` | Fixed in `1d351f5` (`untyped_payload_with_event_rejected`) |
| K2 | `lint_forbidden.py` A-50 rule bypassed by macro renaming | FIX. Flag `use` of `core::env`, `core::option_env`, `std::option_env` and aliases | Fixed in `1d351f5` with planted controls |
| K3 | Engine did not enforce R9 (`outcome = true` cross-instant check) | FIX. Require drain pulse fires at `t` in later-or-equal phase or `SimError::Timer` | Fixed in `1d351f5` |
| K4 | Drain settlement check (taken = applied + dropped) not enforced | BCK. Filed to SPR-038b T-07 | BCK-002 A-72 |
| K5 | Per-instant cost measurements on `life_course` | Documented. Figures filed to BCK-002 A-60 and used as baseline for SPR-038a | BCK-002 A-60 |
| K6 | `drain_only` not tied to spec step count | FIX. Added `crates/rh-sys-demography/tests/drain_only.rs` | Fixed in `1d351f5` |
| K7 | `death_path.rs` concurrency test ran at 1 thread only | FIX. Test loops across 1, 2, 4, N threads asserting equal state and drops | Fixed in `1d351f5` |
| K8 | `DraftRef::from_index` public | BCK. Seal constructor or reword at SPR-021a | BCK-002 A-73 |
| K9 | Owner drop reasons unconstrained | BCK. Two-variant reason (`dead`, `ended`) filed to SPR-038b | BCK-002 A-72 |
| K10 | Rust loader failed open on mistyped `key` or malformed `payload` | FIX. Fail closed by name | Fixed in `1d351f5` |
| K11 | Integer neutral on spec-less column typed as raw bits | FIX. Left untyped (`None`) | Fixed in `1d351f5` |
| K12 | `Marry{a,a}` and stale former spouse admitted | BCK. Filed to SPR-015 N7 | BCK-002 A-74 |
| K13 | Plan BLUF and `05.1` changelog ordering | FIX. Text updated | Fixed in `1d351f5` |

## 4. Backlog items filed (BCK-002)
- **A-60:** Updated with K5 measurements (16/104/1,109 µs per instant at 100/1,000/10,000 rows; ~59 allocations per idle solo tick).
- **A-72:** K4 & K9 filed to SPR-038b T-07 (settlement check and two-variant drop reason).
- **A-73:** K8 filed to SPR-021a (sealed `DraftRef` constructor).
- **A-74:** K12 filed to SPR-015 N7 (`Marry{a,a}` validation).
- **A-75:** Tester's survivors (M4, M5, M7, S2, K4 pending test) and the "with a event" wart.

## 5. Audit conclusion
SPR-014 meets all functional, determinism, performance, and governance requirements under P1a. Status is **CLOSED**.
