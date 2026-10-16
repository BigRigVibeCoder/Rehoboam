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
version: 1.3.0
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
- **Status:** [x] done (`965116d`, `b1c5608`, DEF-1 `7517673`, DEF-2 `4a54af8`, `e5dc2cb`: `rh-snapshot::hash`, layout `rehoboam/rh-snapshot/hash/v1`, `MerkleCache`; `tests/hash.rs`, `tests/golden.rs` against `bin/gen_hash_golden.py`, `tests/stub_world.rs` with the stub-world golden `tests/golden/stub_world_hashes.txt`)

### T-02: Keyframes
- **Deliverable:** Arrow IPC write/read of plane-A tables plus pending timers; zstd
- **Acceptance:** V-DET-05: load, advance 10 years, equal to a continuous run
- **Status:** [x] done (`1abe93f`, `16bef5b`, `f8a0b32`, `cd639a6`, `2ef968e`: `Engine::snapshot`/`Engine::restore`, crash artifact format 2, `rh-snapshot::state` (engine tables in `state_hash`), `rh-snapshot::keyframe` (format `rehoboam/rh-snapshot/keyframe/v1`), `rh-snapshot::store::KeyframeStore`, `journal::Scope::entity`; `tests/keyframe.rs` (V-DET-05 at two cuts × 120 monthly hashes, cadence and LRU, refusals, converged-run event hash), `rh-core/tests/agenda_events.rs` (restore continues the unbroken run; non-canonical parts refused), `rh-core/tests/invariants.rs`)

### T-03: Input journal
- **Deliverable:** SQLite (WAL) journal of push inputs (pull inputs retired, ADR-0034) per IF-2, with tick, seq, class and scope
- **Acceptance:** Round-trip; a replay reads only the journal
- **Status:** [x] done (`b63b151`, GAP-3 `4127373`, OBS-1 `8fd3969`, `d6e21fa`, GAP-5 `b7be162`: `rh-snapshot::journal`, schema `rehoboam/rh-snapshot/journal/v2`, row digests and a chain per timeline; `tests/journal.rs`)

### T-04: Unsynced guard
- **Deliverable:** Debug-build check that read-only APIs never mutate state
- **Acceptance:** V-DET-10: an injected write trips the guard
- **Status:** [x] done (`4855ef1`, GAP-4 `16bef5b`: `rh-snapshot::guard::read_only`, debug builds only; `tests/guard.rs` (injected-write control, release build compiles it out), `tests/unsynced_guard.rs` (read-only APIs fuzzed on the stub world))

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-007` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Carried and ruled (Architect, 2026-10-16)

| Item | What | Resolution | Commit |
|---|---|---|---|
| T-02 deviation 1 | Engine tables are named `rh-core/…`; `rh-core/engine.meta` and `rh-core/events.held` are added beyond the ruled list | Accepted: a collision with an `entity.column` world table is impossible, and both are plane-A state | `16bef5b` |
| T-02 deviation 2 | The keyframe header is the reserved table `rehoboam:keyframe`, not Arrow schema metadata (a hash map); one `(table Utf8, row Binary)` schema of canonical row bytes | Accepted for P0: Arrow is a container, and the keyframe uses the hash's own encoding; typed batches come with typed columns (`05.2` §6) | `f8a0b32`, `cd639a6` |
| T-02 deviation 3 | Integrity envelope: 8 magic bytes and the XXH3-128 of the Arrow bytes, checked before Arrow parses (Arrow 60's IPC reader panics on some corrupt input) | Accepted: it catches corruption, not forgery; the history check and the state hash bind a keyframe to its run | `f8a0b32` |
| T-02 deviation 4 | `KeyframeStore` LRU capacity defaults to 8 (`05.2` names no number) | Accepted until P1 measures keyframe sizes | `f8a0b32` |
| VER-SPR-007 OBS-5 | `MerkleCache::update`: a table that shrinks and regrows to the same count between updates needs `mark_rows` | Documented on `update` | `f8a0b32` |
| VER-SPR-007 GAP-4 | The unsynced guard must see engine-held state, not only `WorldTables` | The `Engine` guard state is `state_hash(&engine.snapshot())`; `tests/keyframe.rs` `the_guard_sees_engine_held_state` | `16bef5b`, `f8a0b32` |
| SPR-006 DEF-6 | The crash artifact keeps exactly the last 1000 diagnostic records | Test `the_crash_artifact_keeps_exactly_the_last_1000_diagnostic_records` (fails at 1001) | `1abe93f` |
| SPR-006 GAP-6 | Restore accepts only the canonical form, everywhere | `Agenda::restore`, `CommandQueues::from_state`, `EventLog::from_state`, epochs and inputs checked; `Engine::restore` validates names against `Registered`; 11 refused cases in `agenda_events.rs` | `1abe93f` |
| SPR-006 event hash | `rh-core/events.meta` carries the running event-log hash (SPR-006 G4), so a converged run that emitted different events still changes `state_hash` | Test `a_converged_run_that_emitted_a_different_event_hashes_differently`; a mutant dropping the running hash fails it | `2ef968e` |
| VER-SPR-007-TEST round 2 GAP-6 | `rh-core/events.held` was in `state_hash`, so the hash depended on when the host drained (seed 13: never vs every instant differed at 782 instants) | Ruling replaced (T-02 plan ruling 1 "and the hash"): held events are a carried table, in the keyframe and outside `state_hash`; `state_hash` covers authoritative state only. Tests: drain never / every instant / monthly / random give one ladder; a keyframe keeps held events, hash unchanged. Synthetic and stub-world goldens regenerated (C3), Python cross-check at months 0, 59, 119 | `ee7d6ff` |
| VER-SPR-007-TEST round 2 DEF-3 | A keyframe with a valid envelope but a corrupt Arrow body crashed the loader (7,890 bit flips: 719 panics, 41 allocation aborts) | Ruling: keep Arrow (ADR-0007), harden in four layers. (1) `ipc_guard::check` validates schema, offsets, row counts and declared sizes before Arrow decodes. (2) `catch_unwind` maps a panic to `KeyframeError::Panicked`. (3) Read-back on write. (4) A bit-flip sweep, PR tier and nightly tier. The full sweep over 93,648 flips gives 0 panics, 0 aborts and 0 other states accepted, with layer 1 alone | `b9c3ede` |
| VER-SPR-007-TEST round 2 DEF-4 | `write_keyframe` stamped the caller's history tuple; seed 99 and salt 5 written over a seed-13 engine loaded and diverged at instant 9 | `Engine::world_seed`, `branch_salt` and `provenance`; `write_keyframe(engine, journal)` takes the tuple from the engine. Test `def_4_the_history_tuple_comes_from_the_engine` | (this commit) |
| VER-SPR-007-TEST round 2 GAP-7 | The keyframe's journal anchor had no verifier | `Replay::check_anchor`: count at least the anchor's, chain after it equal to the head. A keyframe's count must equal the applied inputs (hashed `engine.meta`), checked on write and on load. Tests: truncate-and-reforge, re-grown journal, edited header (999, 0, 2), anchor 99 over 1 applied input | (this commit) |
| VER-SPR-007-TEST round 2 GAP-8 | Only `world_seed` of the five history fields was pinned by a test | `gap_8_every_history_field_is_checked_on_load`: all five, each refused under its own name | (this commit) |
| VER-SPR-007-TEST round 2 GAP-9 | `Engine::restore` accepted an agenda entry dated before the cursor | `RestoreError::TimerTime`, refused at restore as queued inputs are. A timer exactly at the cursor still restores | (this commit) |
| Found by DEF-3 layer 3 | The engine-table decoder returned tombstones in row (key) order, not id order. A keyframe whose tombstoned timers fire in another order than they were inserted (seed 99, day 50) decoded to a state that hashed differently, and `Engine::restore` refused it | The decoder sorts tombstones. Test `tombstones_decode_in_id_order_whatever_their_key_order` | (this commit) |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-007` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class.
