---
id: VER-G0
title: "Gate G0: end of P0"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification, roadmap]
related: [BCK-001, BCK-002, PRJ-001, SPR-001, SPR-002, SPR-003, SPR-004, SPR-005, SPR-006, SPR-007, SPR-008, SPR-009, SPR-010]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF: PASS (2026-10-17).** All ten P0 sprints (SPR-001 to SPR-010) are closed with Tester passes and audits, and the seven criteria of `docs/11` pass. The red-team inbox is disposed of (`docs/red_team/RT-2026-10-16-G0-dispositions.md`), and the director's rulings at G0 (D30–D36, 2026-10-16) set the condition this record meets: G0 passes when SPR-008 and SPR-009 complete. Escapes found after a sprint closed: 2 across 10 sprints (one documentation-only, one CI-concurrency gap in SPR-008, both fixed).
>
> Still to come from the director's machines, which are not G0 criteria: the cross-machine golden check (SPR-008, `worlds/smoke/README.md`), SPR-009 T-04 (wall-clock baselines on `adjutant` and the laptop), and RUN-001 (BCK-002 A-16).

# VER-G0 — Gate G0

## 1. Gate criteria (`docs/11` P0)
| Criterion | Test | Owning sprint | Status |
|---|---|---|---|
| Determinism: identical plane-A hashes at 1, 2, 4 and N threads, 3 repeats, every sim-month for 1,000 years | V-DET-01, V-DET-02 | SPR-008 (with SPR-005 to SPR-007) | **Pass.** PR CI: 1,000 years yearly at 1 and N threads against the committed baseline (the N-thread run with the vitals observer), 7% of levels parallel. Nightly run 38967087391 on `main` (`3bd2262`): 12 monthly ladders of 1,000 years at 1, 2, 4 and N threads × 3, all equal; their yearly cut equals the baseline. Cross-machine: the director's session (not a criterion) |
| Keyframes: write, load, advance 10 years; equal to a continuous run | V-DET-05 | SPR-007 | **Pass.** In `check_all` on the life world (48 to 5,000 rows), also resumed through the journal; resume equals the continuous run on `smoke` (SPR-008) |
| Samplers: golden vectors bit-equal | V-DET-09 | SPR-003 | **Pass.** In `check_all` and CI (opt-level 0 and 3, GitHub's runner); nightly statistics green |
| Bisect: names the injected month, table and row | V-DET-11 | SPR-008 | **Pass.** 4 random cases per PR, 64 in nightly run 38967087391; the Tester's own harness named 66 of 66 plus 8 shapes, and CI's automatic bisect named a planted change |
| Specs: `val_khor` and `aethelgard` validate, `invalid_draft_style` fails | V-WC-04 | SPR-001 (Python); SPR-004 (Rust parity) | **Pass.** In `check_all`: Python, and Rust parity line for line (now also `uz_job`, `household`, `hamlet`) |
| Lints: zero violations | V-PLN-01, V-DET-19, V-INSP-03 | SPR-001; SPR-010 (floats); SPR-003 (sampler float zone) | **Pass.** In `check_all`, with planted controls for every rule |
| CI: per-PR CI ≤ 15 min; baselines for benches 7 and 8 (bench 4 moved to the first P1a field sprint: director, 2026-10-16) | V-PERF-04 | SPR-009 | **Pass.** A 15% instruction slowdown fails the gate; benches 7 and 8 have committed instruction-count baselines (10k rows), checked against CI's recomputation within R-8's band. Per-PR CI in two parallel jobs: 11 min 2 s on cold caches (run 38977023421), 6 min 39 s warm (run 38982157531). Wall-clock baselines on `adjutant`: SPR-009 T-04, the director's session |

## 2. Sprints
| Sprint | Scope | Status | Audit | Escapes found later |
|---|---|---|---|---|
| SPR-001 | Repository bootstrap, lints, CI | Closed | `VER-SPR-001` PASS | 0 |
| SPR-002 | `rh-ids`: IDs, H, streams, salts | Closed | `VER-SPR-002` PASS | 1 (documentation only) |
| SPR-003 | Samplers, `rh-sample` float zone | Closed | `VER-SPR-003` PASS | 0 |
| SPR-004 | Registry and manifest loaders, ownership lint, `rh-manifest` | Closed | `VER-SPR-004` PASS | 0 |
| SPR-005 | Time, phases, SystemModel, AccessSpec, scheduler | Closed | `VER-SPR-005` PASS | 0 |
| SPR-006 | Agenda, lifecycle queues, event bus, invariants | Closed | `VER-SPR-006` PASS | 0 |
| SPR-007 | Plane hashes, keyframes, journal | Closed | `VER-SPR-007` PASS | 0 |
| SPR-008 | Golden-run CI, bisect | Closed (PR #28) | `VER-SPR-008` PASS | 1 (CI concurrency: a pending push run could be replaced; fixed in SPR-009) |
| SPR-009 | Benchmarks, regression gate | Closed (PR #34); T-04 blocked on the director's machines (ruling 6) | `VER-SPR-009` PASS | 0 |
| SPR-010 | `rh-core::fixed` | Closed | `VER-SPR-010` PASS | 0 |

The escape-rate count (BCK-002 A-05): 2 escapes across 10 closed sprints, one documentation-only (SPR-002) and one CI gap (SPR-008), both found by later reviews and fixed.

## 3. P0 work packages that no sprint owns (ruled by the director, 2026-10-16)
Both recommendations were approved. `docs/11` v1.4 drops the Rerun hooks from P0 and moves the TUI stub to P1.17.

| Item (`docs/11` P0) | Gap | Recommendation | Confidence |
|---|---|---|---|
| P0.7 "Rerun logging hooks" | Owned by no sprint. Logging from the engine into a visualisation tool conflicts with ADR-0035 §6 ("the engine only engines": the World Stream is the engine's only output) | Drop it from P0. A Rerun view, if wanted, is a downstream consumer of the World Stream (`python/`). The `docs/11` edit is a roadmap change, so it needs the director | 85% |
| P0.7 "Textual TUI stub" | Owned by no sprint and not a G0 criterion. `rh-tui` is a stream viewer, not an engine crate (`05.0` §6) | Move it to P1a as a consumer of the first real World Stream, where it has something to show | 75% |

## 4. Before the gate
- **Red-team inbox:** done. Batches 01 to 04 are disposed of in `docs/red_team/RT-2026-10-16-G0-dispositions.md`. The director ruled on Laws 2 and 3 (principles 8 and 9 of `docs/01` §5) and item 15 (GOV-003 §6). A later dossier on demography and inequality is taken in as batch 05, a proposal for G1a (director, 2026-10-16).
- **P1a plan:** approved by the director at G0 (D30 to D36, 2026-10-16): `PRJ-002`, `SPR-011` as three sprints (SPR-011 to SPR-013), and the `T1.minds` and `M7.dynasty` specs (PR #32), which merge before the skeleton specs (PR #37).
- **Consistency fixes:** the Architect corrects the document inconsistencies within its authority listed in `PRJ-002` §10 before G0 (EventId layout, `schema.sql` floats, the engine-crate lists, statuses).
- **Architect backlog that must close before G0:** A-09, this record: done. (A-20, the plane-A external dependency allow-list, is done.)
- **Director:** T-00 of SPR-001 (`RUN-001` on the laptop) is still open (BCK-002 A-16). The laptop and one `adjutant` session are also needed for SPR-008 T-02 (cross-machine hashes) and SPR-009 T-04 (wall-clock baselines). The director will open a session of about 2 hours on the laptop when the Architect reports SPR-008 ready (2026-10-16); reported ready at 21:5xZ with nightly run 38967087391 on `3bd2262`. None of these is a G0 criterion.
