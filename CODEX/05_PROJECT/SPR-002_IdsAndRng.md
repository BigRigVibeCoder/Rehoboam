---
id: SPR-002
title: "rh-ids: typed IDs, keyed hash, counter RNG, branch salts"
type: how-to
status: CLOSED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001]
created: 2026-10-16
updated: 2026-10-16
version: 1.2.0
---

> **BLUF:** Build the identity and randomness foundation every later system uses: the keyed hash H, counter-based RNG streams, branch salts and genealogical PersonIds. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-002 — rh-ids: typed IDs, keyed hash, counter RNG, branch salts

- **Roadmap item:** `docs/11` P0.2 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-001 · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** ADR-0006 (determinism contract); ADR-0026 (genealogical identity); `05.1` §6.9 (salts); `docs/14` (Random123, rand_chacha)
- **Requirements covered:** DET-2, DET-3

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C3 (determinism machinery: the director approved the spec; the Architect merges after the Tester and the audit, GOV-005 §6) |
| Planes touched (ADR-0024) | A (machinery) |
| Owned columns written | none |
| RNG purposes | defines the RNG API itself: `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)` |
| Golden-hash effect | defines hash inputs |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-DET-09 (known-answer vectors), V-DET-16 (branch-salt properties), V-DET-19, V-DET-20 (statistical stream independence); Random123 Philox known-answer tests if ported |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: Typed IDs
- **Deliverable:** `PersonId`, `RegionId`, `EventId`, `MatchId`, `TimelineId` newtypes; no implicit integer conversion
- **Acceptance:** compile-fail tests for mixing ID types
- **Status:** [x] done (VER-SPR-002)

### T-02: Keyed hash H
- **Deliverable:** The keyed hash from ADR-0006 with fixed byte layout; documented endianness
- **Acceptance:** Golden vectors: fixed keys → fixed outputs, committed
- **Status:** [x] done (VER-SPR-002)

### T-03: Counter RNG
- **Deliverable:** ChaCha8 streams keyed by H, plus SplitMix64 for light uses; purpose registry API (purposes declared per system spec)
- **Acceptance:** Streams are independent of call order (property test); known-answer vectors
- **Status:** [x] done (VER-SPR-002)

### T-04: Branch salts and lineage
- **Deliverable:** `branch_salt(t)` and salt lineage per `05.1` §6.9; `resample_on_fork` flag per purpose
- **Acceptance:** V-DET-16 property test over random fork trees
- **Status:** [x] done (VER-SPR-002)

### T-05: Genealogical PersonId
- **Deliverable:** PersonId derivation from parents and birth per ADR-0026, with deterministic collision rehash
- **Acceptance:** Collision test with forced collisions; IDs stable across runs
- **Status:** [x] done (VER-SPR-002)

### T-06: Statistical stream independence (RT-2026-10-16)
- **Deliverable:** A test harness drawing from streams with distinct (system, purpose, entity, t, k) keys and sibling branch salts, including adjacent entities and adjacent t, with pairwise correlation, byte chi-square and runs statistics
- **Acceptance:** V-DET-20: every statistic inside its 99.9% band, with no more exceedances than a binomial 99.9% allows across all pairs; runs in the PR tier in under 30 s (a larger sweep may run nightly)
- **Status:** [x] done (VER-SPR-002)

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-002` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-002` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
