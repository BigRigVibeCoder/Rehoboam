---
id: SPR-005
title: "rh-core: time, pulses, phases, SystemModel, AccessSpec, scheduler"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001]
created: 2026-10-16
updated: 2026-10-16
version: 1.1.0
---

> **BLUF:** Build the core's time model and the contract every system implements, with a scheduler derived from declared reads and writes. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-005 — rh-core: time, pulses, phases, SystemModel, AccessSpec, scheduler

- **Roadmap item:** `docs/11` P0.4 (part a) · **Phase:** P0 (gate G0)
- **Depends on:** SPR-002, SPR-004 · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** `05.1` §2 (SystemModel), §4.1–4.2 (pulses, phase order including 5a Activity); ADR-0005; ADR-0033 (generated skeletons)
- **Requirements covered:** SIM-2, DET-2

## Architect rulings, 2026-10-16 (before the sprint starts)
1. **`Fixed` in generated skeletons is `rh_core::fixed::Q16_16`.** `lint_odd.py --rust` emits `pub e0: Fixed` for every `[[params]]` entry, and no contract defined `Fixed`. `rh-core` exports `pub type Fixed = Q16_16;`. The parameter ranges in the specs fit it: Brass α from −1.36 to 0.55, TFR up to about 8, e0 up to about 85. A spec that later needs another width declares a type per parameter, as an ODD-R extension.
2. **RNG purposes come from the specs** (BCK-002 A-18; `VER-SPR-002` GAP-3). The generated `RngPurpose` enum is the only way a system names a purpose. Registering a system checks its declared purposes against the spec's `[[rng]]` table, so an undeclared purpose is a registration error. See T-06.
3. **`AccessSpec` and the ownership lint agree.** T-04's conflict checker uses the same single-writer rule as SPR-004's ownership lint in `rh-registry`: one owner per column, reads declared, no plane-B or plane-C read from plane A. Where they meet, the lint reads the specs and the checker reads the registered code. The lint's "0 access declarations from code" line becomes real once systems register here.
4. **Authored numbers enter plane A only as fixed point** (`05.5` §3.2, SPR-004 ruling A). Nothing in this sprint reads `rh-manifest`.

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery) |
| Planes touched (ADR-0024) | A |
| Owned columns written | core tables only |
| RNG purposes | none |
| Golden-hash effect | phase order becomes part of the hash contract |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-01/02 on a stub world; `AccessSpec` conflict tests; `lint_odd.py --rust` skeletons compile |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: SimTime and calendar
- **Deliverable:** `SimTime` as i64 minutes with a signed epoch; 360/365-day calendars; pulses Year to Hour
- **Acceptance:** Property tests on calendar arithmetic
- **Status:** [ ] not started

### T-02: Phase order
- **Deliverable:** The 12 phases of `05.1` §4.2, including 5a Activity, as a fixed enum
- **Acceptance:** Snapshot test of the phase table
- **Status:** [ ] not started

### T-03: SystemModel trait
- **Deliverable:** The trait from `05.1` §2; generated skeleton modules from `lint_odd.py --rust` compile against it
- **Acceptance:** The S5 and M2 skeletons compile
- **Status:** [ ] not started

### T-04: AccessSpec and conflict checker
- **Deliverable:** Declared reads and owned writes; conflicting writers rejected at registration
- **Acceptance:** Two systems writing one column fail registration
- **Status:** [ ] not started

### T-05: Derived scheduler
- **Deliverable:** Topological order within a phase from read/write sets; declared lags for back-edges
- **Acceptance:** Order is identical across runs and thread counts
- **Status:** [ ] not started

### T-06: RNG purposes from the specs
- **Deliverable:** registration checks a system's RNG purposes against its spec's `[[rng]]` table (ruling 2)
- **Acceptance:** a system that draws under an undeclared purpose fails registration, and a spec purpose with no code use is reported
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-005` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-005` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
