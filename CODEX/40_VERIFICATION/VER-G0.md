---
id: VER-G0
title: "Gate G0: end of P0 (draft tracker)"
type: record
status: DRAFT
owner: architect
agents: [architect]
tags: [verification, roadmap]
related: [BCK-001, BCK-002, PRJ-001, SPR-001, SPR-002, SPR-003, SPR-004, SPR-005, SPR-006, SPR-007, SPR-008, SPR-009, SPR-010]
created: 2026-10-16
updated: 2026-10-16
version: 0.3.0
---

> **BLUF:** DRAFT tracker; this is not a verdict. G0 closes when SPR-001 to SPR-010 are closed and the seven criteria of `docs/11` pass, and the red-team inbox is folded in with a dispositions record. Status on 2026-10-16:
> - six sprints are closed (SPR-001 to SPR-005, SPR-010);
> - SPR-006 and SPR-007 are in progress;
> - SPR-008 and SPR-009 have not started.
>
> One roadmap item is owned by no sprint, and part of it conflicts with ADR-0035 §6 (§3). It needs a director ruling.

# VER-G0 — Gate G0 (draft tracker)

## 1. Gate criteria (`docs/11` P0)
| Criterion | Test | Owning sprint | Status |
|---|---|---|---|
| Determinism: identical plane-A hashes at 1, 2, 4 and N threads, 3 repeats, every sim-month for 1,000 years | V-DET-01, V-DET-02 | SPR-008 (with SPR-005 to SPR-007) | Not started |
| Keyframes: write, load, advance 10 years; equal to a continuous run | V-DET-05 | SPR-007 | Not started |
| Samplers: golden vectors bit-equal | V-DET-09 | SPR-003 | Passing in `check_all` and CI (opt-level 0 and 3, GitHub's runner); nightly statistics green |
| Bisect: names the injected month, table and row | V-DET-11 | SPR-008 | Not started |
| Specs: `val_khor` and `aethelgard` validate, `invalid_draft_style` fails | V-WC-04 | SPR-001 (Python); SPR-004 (Rust parity) | Passing in `check_all`: Python, and Rust parity line for line |
| Lints: zero violations | V-PLN-01, V-DET-19, V-INSP-03 | SPR-001; SPR-010 (floats); SPR-003 (sampler float zone) | Passing in `check_all` |
| CI: per-PR CI ≤ 15 min; baselines for benches 4, 7 and 8 | V-PERF-04 | SPR-009 | CI: about 1 min today. Baselines: not started |

## 2. Sprints
| Sprint | Scope | Status | Audit | Escapes found later |
|---|---|---|---|---|
| SPR-001 | Repository bootstrap, lints, CI | Closed | `VER-SPR-001` PASS | 0 |
| SPR-002 | `rh-ids`: IDs, H, streams, salts | Closed | `VER-SPR-002` PASS | 1 (documentation only) |
| SPR-003 | Samplers, `rh-sample` float zone | Closed | `VER-SPR-003` PASS | 0 |
| SPR-004 | Registry and manifest loaders, ownership lint, `rh-manifest` | Closed | `VER-SPR-004` PASS | 0 |
| SPR-005 | Time, phases, SystemModel, AccessSpec, scheduler | Closed | `VER-SPR-005` PASS | 0 |
| SPR-006 | Agenda, lifecycle queues, event bus, invariants | In progress | — | — |
| SPR-007 | Plane hashes, keyframes, journal | In progress (T-02 waits on SPR-006) | — | — |
| SPR-008 | Golden-run CI, bisect | Waits on SPR-006 and SPR-007 | — | — |
| SPR-009 | Benchmarks, regression gate | Waits on SPR-007 and SPR-008; T-04 needs `adjutant` and the laptop | — | — |
| SPR-010 | `rh-core::fixed` | Closed | `VER-SPR-010` PASS | 0 |

The escape-rate count (BCK-002 A-05) stands at 1 escape across 6 closed sprints.

## 3. P0 work packages that no sprint owns
| Item (`docs/11` P0) | Gap | Recommendation | Confidence |
|---|---|---|---|
| P0.7 "Rerun logging hooks" | Owned by no sprint. Logging from the engine into a visualisation tool conflicts with ADR-0035 §6 ("the engine only engines": the World Stream is the engine's only output) | Drop it from P0. A Rerun view, if wanted, is a downstream consumer of the World Stream (`python/`). The `docs/11` edit is a roadmap change, so it needs the director | 85% |
| P0.7 "Textual TUI stub" | Owned by no sprint and not a G0 criterion. `rh-tui` is a stream viewer, not an engine crate (`05.0` §6) | Move it to P1a as a consumer of the first real World Stream, where it has something to show | 75% |

## 4. Before the gate
- **Red-team inbox:** fold batches 01 to 04 into a dispositions record (`docs/red_team/RT-<date>-G0-dispositions.md`), starting from the Architect's intake notes. The director rules on anything that changes an ADR, the roadmap or `CLAUDE.md`.
- **Architect backlog that must close before G0:** A-09, this record. (A-20, the plane-A external dependency allow-list, is done.)
- **Director:** T-00 of SPR-001 (`RUN-001` on the laptop) is still open (BCK-002 A-16). The laptop and one `adjutant` session are also needed for SPR-008 T-02 (cross-machine hashes) and SPR-009 T-04 (wall-clock baselines).
