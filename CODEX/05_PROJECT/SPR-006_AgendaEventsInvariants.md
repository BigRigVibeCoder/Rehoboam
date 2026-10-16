---
id: SPR-006
title: "Agenda, timers, lifecycle queues, event bus, invariants"
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

> **BLUF:** Add scheduled outcomes as timers, lifecycle command queues, the event bus with emission-order EventIds, and the invariant framework. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-006 — Agenda, timers, lifecycle queues, event bus, invariants

- **Roadmap item:** `docs/11` P0.4 (part b) · **Phase:** P0 (gate G0)
- **Depends on:** SPR-005
- **Binding contracts:** ADR-0025 (scheduled outcomes, no back-dating); `05.1` §4.3 (agenda), §2 (lifecycle queues), §9 (invariants)
- **Requirements covered:** DET-13, SIM-2

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery) |
| Planes touched (ADR-0024) | A |
| Owned columns written | core tables only |
| RNG purposes | none |
| Golden-hash effect | event order becomes part of the hash contract |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-12 (no back-dating property test); invariant framework self-test |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: Agenda heap
- **Deliverable:** Binary-heap queue keyed by `(time, phase, entity_id, seq)`
- **Acceptance:** Deterministic order with ties; property test
- **Status:** [ ] not started

### T-02: Timers
- **Deliverable:** Insert, cancel and re-sample by dependency scope (epoch-keyed)
- **Acceptance:** Timer fires at its own tick in every mode
- **Status:** [ ] not started

### T-03: Lifecycle command queues
- **Deliverable:** Typed commands applied in `(phase, PersonId)` order by owners
- **Acceptance:** A non-owner cannot apply a command directly
- **Status:** [ ] not started

### T-04: Event bus
- **Deliverable:** `EventDraft` → EventId in emission order `(tick, phase, seq)`; causal edges
- **Acceptance:** V-DET-12: EventIds strictly increase; no event before the current instant
- **Status:** [ ] not started

### T-05: Invariant framework
- **Deliverable:** Per-system invariants run in Bookkeeping; a violation is fatal with a crash artifact (GOV-004 §4)
- **Acceptance:** An injected violation produces a crash artifact that replays
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-006` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-006` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
