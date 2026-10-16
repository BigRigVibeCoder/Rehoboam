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
version: 1.2.0
---

> **BLUF:** Make every state reproducible and checkable: three plane hashes, Arrow keyframes and the SQLite input journal. Developer: AGT-002-CORE. Testers: AGT-003-DET. Scope-bounded; at most about 3 agent-days.

# SPR-007 — Plane hashes, keyframes and the input journal

- **Roadmap item:** `docs/11` P0.5 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-005; T-02 also on SPR-006, because a keyframe holds the pending timers (Architect, 2026-10-16: T-01, T-03 and T-04 may run alongside SPR-006; T-02 and the merge follow it) · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** ADR-0006, ADR-0024 (plane hashes), ADR-0007; `05.9` IF-2 (journal record)
- **Requirements covered:** DET-1, DET-7, DET-11

## Architect rulings, 2026-10-16 (binding; written before the sprint starts)
1. **The hash: XXH3-64 from `twox-hash`** (MIT; the `docs/14` preference), used in `rh-snapshot` only.
   - It enters `bin/lint_arch.py`'s `PLANE_A_EXTERNAL_DEPS` in this sprint, with its review note: exact pin, default features off, only the XXH3 feature, no floats.
   - The layout is versioned (`rehoboam/rh-snapshot/hash/v1`) and documented. It has four levels:
     - each row has a canonical little-endian encoding, in key order;
     - chunks are fixed-size runs of rows;
     - a table root hashes its chunk hashes in order;
     - a plane hash hashes `(table name, table root)` pairs in table-name byte order.
   - An independent reference (Python) generates golden vectors, as `bin/gen_rng_golden.py` does for `rh-ids`. Changing the layout is C3.
   - Planes B and C have no tables yet: their hashes are defined over empty table sets and tested with synthetic tables.
2. **Keyframes: Arrow IPC with zstd**, through the arrow-rs subcrates (`arrow-array`, `arrow-buffer`, `arrow-schema`, `arrow-ipc`). They are used in `rh-snapshot` only, with exact pins and minimal features, and reviewed into `PLANE_A_EXTERNAL_DEPS`.
   - The review note: Arrow can carry float arrays, but no P0 table has a float column, so `rh-snapshot` names no float type and `lint_forbidden` applies unchanged. `f32_field` columns come with BCK-002 A-22.
   - A keyframe holds the plane-A tables, SPR-006's pending agenda (ruling 7 there), the journal position and the history tuple.
   - Keyframe bytes need not be reproducible. The state they restore must hash equal, and advancing from it must equal the continuous run (V-DET-05).
3. **The journal: `rusqlite` with bundled SQLite (WAL)** in `rh-snapshot`, recording the IF-2 fields (`05.9`).
   - The journal stores no `REAL` values, only integers, text and blobs, so SQLite's floating point never reaches plane A.
   - Replay reads only the journal and hard-fails on a missing, extra or mismatched input (`05.1` §6 item 8).
4. **C dependencies are built with `-ffp-contract=off`** (`05.1` §6 item 4). This covers bundled SQLite and zstd.
   - Set it for the `cc` builds through `CFLAGS` in the repository's `.cargo/config.toml` `[env]`.
   - The fp-contract guard of `bin/check_all.sh` (SPR-003 GAP-6) then needs narrowing: it accepts exactly `-ffp-contract=off` in `CFLAGS`, and still fails on any other fp-contract value and on any fp-contract in a `RUSTFLAGS` variable.
   - Add controls both ways, and list the new checks in the gate inventory test.
5. **The unsynced guard (T-04)** hashes plane A before and after every read-only API call in debug and CI builds, and is compiled out of release builds. The positive control injects a write through a read-only path.
6. **Order of work.** T-01, T-03 and T-04 first. T-02 waits for SPR-006's agenda (its keyframes hold pending timers), and this sprint merges after SPR-006.

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
