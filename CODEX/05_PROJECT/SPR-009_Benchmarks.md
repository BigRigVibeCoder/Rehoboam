---
id: SPR-009
title: "Benchmark harness and regression gate"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, RUN-001, GOV-008]
created: 2026-10-16
updated: 2026-10-16
version: 1.3.0
---

> **BLUF:** Measure speed from the start: criterion benches, a 10% regression gate on instruction counts (stable on shared runners and laptops), wall-clock trends on the on-demand VM, serial-fraction instrumentation and the vitals line. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-009 — Benchmark harness and regression gate

- **Roadmap item:** `docs/11` P0.7 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-007 (benches 7 and 8 measure its plane hashes and keyframes) and SPR-008 (T-03 and T-05 run on its `smoke` world). T-04 also needs `adjutant` and the director's laptop (RUN-001, RUN-002). Architect, 2026-10-16: it may start once SPR-007 merges, alongside SPR-008, and merges after it · **Plan approved by the director 2026-10-16** (P0 batch)
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
| Tests that gate this sprint | V-PERF-04 (a deliberate 15% slowdown, measured in instructions, fails CI; a speed-up never re-baselines automatically); V-PERF-11 (allocation budget per tick) |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: criterion benches
- **Deliverable:** Benches 4, 7 and 8 of `08` §9 against the smoke world
- **Acceptance:** Baselines recorded
- **Status:** [ ] not started

### T-02: Regression gate
- **Deliverable:** Two gates. (a) **PR gate on instruction counts** (`iai-callgrind`, valgrind as an outside tool): more than 10% more instructions fails; deterministic to well under 1%, so it does not flake on a hosted runner or a laptop. (b) **Wall-clock benches** (criterion) run in each `adjutant` session (on demand, RUN-002 §4), are tracked as trends, and alert above 10%; they gate only on `adjutant`, once T-04 shows its spread is well under 10%. No automatic re-baseline
- **Acceptance:** V-PERF-04 positive control on gate (a); a run of gate (a) twice on the same commit gives identical counts
- **Status:** [ ] not started

### T-03: Serial fraction and vitals
- **Deliverable:** Per-system timings; the vitals line of GOV-006 §1
- **Acceptance:** Vitals printed by `rh-cli run smoke`
- **Status:** [ ] not started

### T-04: Machine ratios and wall-clock baselines
- **Deliverable:** The `adjutant` runner and its idle shutdown (RUN-002 §3). The benches of T-01 run on `adjutant` in one VM session (RUN-002 §4–5) and on the laptop (RUN-001 §6). `adjutant` becomes the wall-clock baseline machine; its run-to-run spread decides whether a wall-clock gate is possible there. Both machines' ratios to the `08` reference desktop are recorded in `08` as [M]
- **Acceptance:** Ratios, spreads and run conditions (machine type, CPU model, power profile, temperature) are in `VER-SPR-009`
- **Status:** [ ] not started

### T-05: Allocation budget per tick (RT-2026-10-16)
- **Deliverable:** A counting global allocator, enabled only in bench builds, that reports heap allocations per steady-state tick of the `smoke` world; buffer growth on capacity doubling counted separately; the budget recorded per bench (default 0 for P0 machinery) and gated like instruction counts
- **Acceptance:** V-PERF-11 positive control: a fixture system that allocates in its tick fails the gate; two runs on the same commit report identical counts
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
