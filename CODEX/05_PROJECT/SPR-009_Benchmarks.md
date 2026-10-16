---
id: SPR-009
title: "Benchmark harness and regression gate"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** Measure speed from the start: criterion benches, a 10% regression gate, serial-fraction instrumentation and the vitals line. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-009 — Benchmark harness and regression gate

- **Roadmap item:** `docs/11` P0.7 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-005
- **Binding contracts:** `docs/08` §9; GOV-006
- **Requirements covered:** PERF-4

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C1 |
| Planes touched (ADR-0024) | none (tooling) |
| Owned columns written | none |
| RNG purposes | none |
| Golden-hash effect | none |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-PERF-04 (a deliberate 15% slowdown fails CI; a speed-up never re-baselines automatically) |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: criterion benches
- **Deliverable:** Benches 4, 7 and 8 of `08` §9 against the smoke world
- **Acceptance:** Baselines recorded
- **Status:** [ ] not started

### T-02: Regression gate
- **Deliverable:** > 10% slower fails; no automatic re-baseline
- **Acceptance:** V-PERF-04 positive control
- **Status:** [ ] not started

### T-03: Serial fraction and vitals
- **Deliverable:** Per-system timings; the vitals line of GOV-006 §1
- **Acceptance:** Vitals printed by `rh-cli run smoke`
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-009` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-009` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
