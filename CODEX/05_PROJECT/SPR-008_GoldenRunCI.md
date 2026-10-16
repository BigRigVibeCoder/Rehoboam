---
id: SPR-008
title: "Golden-run CI, the smoke world and the bisect tool"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, RUN-002, GOV-008]
created: 2026-10-16
updated: 2026-10-16
version: 1.1.0
---

> **BLUF:** Prove determinism continuously: a smoke world with dummy systems runs a thousand years in CI with hash ladders, and a bisect tool names any divergence. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-008 — Golden-run CI, the smoke world and the bisect tool

- **Roadmap item:** `docs/11` P0.6 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-006, SPR-007
- **Binding contracts:** `docs/13` §4.3–4.5; ADR-0006
- **Requirements covered:** DET-4, DET-5

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery; creates the golden-world corpus) |
| Planes touched (ADR-0024) | A |
| Owned columns written | dummy systems only |
| RNG purposes | dummy purposes |
| Golden-hash effect | records the first golden baselines (through the merge queue) |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-01, V-DET-02 (1/2/4/N threads × 3 repeats, monthly for 1,000 years), V-DET-11 (bisect names an injected month, table and row) |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: Smoke world
- **Deliverable:** `worlds/smoke` with dummy systems exercising every phase, timers and lifecycle commands
- **Acceptance:** Runs 1,000 sim-years headless
- **Status:** [ ] not started

### T-02: Hash ladders in CI
- **Deliverable:** Yearly ladders on PRs (shorter horizons); monthly full length nightly; a cross-machine check nightly: `adjutant` (AMD Zen 4, self-hosted runner, RUN-002 §3) against a GitHub-hosted Ubuntu runner, with the director's laptop (Intel) on demand; different CPU vendors replace the cross-platform matrix per D5 (GOV-008 §1). Includes the `adjutant` runner and its instance schedule (RUN-002 §3)
- **Acceptance:** V-DET-01/02 green
- **Status:** [ ] not started

### T-03: Bisect tool v0
- **Deliverable:** Compares per-table Merkle roots between baseline and candidate; reports first divergent month, table, row
- **Acceptance:** V-DET-11 on injected perturbations
- **Status:** [ ] not started

### T-04: Baseline management
- **Deliverable:** Baselines recorded only at merge of approved C3 PRs (by the merge queue, or by the fallback in `docs/13` §4.4)
- **Acceptance:** A hand-edited baseline is rejected
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-008` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-008` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
