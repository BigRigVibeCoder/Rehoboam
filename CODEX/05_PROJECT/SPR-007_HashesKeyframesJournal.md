---
id: SPR-007
title: "Plane hashes, keyframes and the input journal"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.1
---

> **BLUF:** Make every state reproducible and checkable: three plane hashes, Arrow keyframes and the SQLite input journal. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-007 — Plane hashes, keyframes and the input journal

- **Roadmap item:** `docs/11` P0.5 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-005 · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** ADR-0006, ADR-0024 (plane hashes), ADR-0007; `05.9` IF-2 (journal record)
- **Requirements covered:** DET-1, DET-7, DET-11

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery) |
| Planes touched (ADR-0024) | A, B, C (hashes) |
| Owned columns written | none |
| RNG purposes | none |
| Golden-hash effect | defines the hashes |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-05 (keyframe round-trip), V-DET-10 (unsynced guard with a positive control) |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: Plane hashes
- **Deliverable:** Chunked Merkle hashes with dirty tracking, one per plane; the hash function licence-checked (`docs/14` xxhash note)
- **Acceptance:** Hash stable across runs; a one-row change alters exactly one chunk
- **Status:** [ ] not started

### T-02: Keyframes
- **Deliverable:** Arrow IPC write/read of plane-A tables plus pending timers; zstd
- **Acceptance:** V-DET-05: load, advance 10 years, equal to a continuous run
- **Status:** [ ] not started

### T-03: Input journal
- **Deliverable:** SQLite (WAL) journal of push inputs (pull inputs retired, ADR-0034) per IF-2, with tick, seq, class and scope
- **Acceptance:** Round-trip; a replay reads only the journal
- **Status:** [ ] not started

### T-04: Unsynced guard
- **Deliverable:** Debug-build check that read-only APIs never mutate state
- **Acceptance:** V-DET-10: an injected write trips the guard
- **Status:** [ ] not started

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-007` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-007` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
