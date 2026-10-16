---
id: SPR-003
title: "Samplers with golden vectors"
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

> **BLUF:** Vendor the distribution samplers the systems need, so that every draw is bit-identical across releases and machines. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-003 — Samplers with golden vectors

- **Roadmap item:** `docs/11` P0.3 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-002 · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** ADR-0006; `docs/14` (vendored rand_distr: BTPE, ZIGNOR)
- **Requirements covered:** DET-3

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery) |
| Planes touched (ADR-0024) | A (machinery) |
| Owned columns written | none |
| RNG purposes | uses SPR-002 streams |
| Golden-hash effect | none yet |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-09 (bit-equal golden vectors for every sampler) |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: Binomial
- **Deliverable:** BTPE plus inversion for small n·p
- **Acceptance:** Golden vectors; chi-square sanity against the exact distribution
- **Status:** [ ] not started

### T-02: Poisson
- **Deliverable:** PTRS
- **Acceptance:** Golden vectors; moment tests
- **Status:** [ ] not started

### T-03: Normal
- **Deliverable:** Ziggurat
- **Acceptance:** Golden vectors; tail test
- **Status:** [ ] not started

### T-04: Beta and Gamma
- **Deliverable:** Marsaglia–Tsang Gamma; Beta from two Gammas
- **Acceptance:** Golden vectors; moment tests
- **Status:** [ ] not started

### T-05: No floating-point drift
- **Deliverable:** Fixed operation order; no fused multiply-add; documented
- **Acceptance:** V-DET-19 passes; golden vectors identical on a second Linux machine
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-003` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-003` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
