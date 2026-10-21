---
id: SPR-013
title: "Walking skeleton C: the stream, the sifter, replay and rungs 1-2 (P1.0, increment C)"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, BCK-002, PRJ-002, SPR-011, SPR-012, SPR-007, SPR-009, ADR-0006, ADR-0024, ADR-0030, ADR-0035]
created: 2026-10-16
updated: 2026-10-17
version: 1.1.0
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

> **BLUF:** Increment C of the walking skeleton (WP P1.0): World Stream v0 and the sealed Parquet chronicle (`rh-stream`), `rh-cli tail`, the `life_ends` pattern with its DuckDB twin (`rh-sifter` v0), journal replay end to end, and rungs 1 and 2 (`household`, `hamlet`) on the same binary. Developers: AGT-002-CORE (AGT-002-TOOLS for the twin and its check). Testers: AGT-003-DET and AGT-003-IF (one Tester session may hold both roles, on a different model from the Developer's). About 3.5 agent-days. **APPROVED by the director at G0** (C3; D30, 2026-10-16). SPR-011's rulings 1–19 bind this sprint and are not repeated here.

# SPR-013 — Walking skeleton C: the stream, the sifter, replay and rungs 1–2 (WP P1.0)

- **Roadmap item:** `docs/11` P1.0 (increment C) · **Phase / rung:** P1a, rungs 0–2
- **Depends on:** SPR-012 merged; SPR-009 merged (V-PERF-11 counts in T-13) · **Plan approved by the director 2026-10-16** (C3, D30; GOV-005 §2)
- **Binding contracts:** `05.1` §6, §9; `05.2` §1; `05.10` §4, §5, §7, §7a; `05.9` IF-0, IF-1, IF-2; `specs/chronicle/schema.sql`; `specs/stream/channel_a_v0.sql`; ADR-0006, ADR-0024, ADR-0030, ADR-0035 §6
- **Rulings (SPR-011, binding):** 11 (verbs, verb ids, roles), 12 (EventId layout), 13 (World Stream v0 and the chronicle: one writer, `rh-stream`), 14 (the sifter runs downstream), 15 (material inputs), 16 (golden worlds and CI tiers), 17 (extinction), 18 (the rung-1 and rung-2 manifests), 19 (scale claims: N ∈ {1, 5, 60})
- **Requirements covered:** STR-1 (v0), DET-4, DET-5, DET-7 on all three rungs, DATA-1 and DATA-6 (subset), SIM-1 (N = 1 to 60), SIFT-3 (one pattern), EVO-4

## Specs this sprint needs

| Spec | Exists? | Gap | Writer | Needed by |
|---|---|---|---|---|
| `specs/chronicle/schema.sql` | Exists | Reconciled with `[event_roles]` by the envelope spec (PR #61, `specs/chronicle/envelope_v0.md` §1): `event_role` is channel A's child table column for column; ruling 11 uses `target` for the deceased | — | T-09 |
| `specs/stream/channel_a_v0.sql` | Exists (PR #37; `selftest_channel_a.py`), **amended in place by the envelope spec (PR #61) before any build wrote a segment** (REV-SPR-020 R6, the Architect approves this amendment at that merge): three child tables (`event_role`, `event_snapshot`, `causal_edge` with `motive`, `weight`, `evidence`, `cause_told`), six dictionaries (`verb`, `act_class`, `record_kind`, `event_role_kind`, `motive_edge_kind`, `attribute`), 16 files per segment, most of them empty on `solo`, `household` and `hamlet` (no snapshot, no motive edge in P1.0); `entity_kind` 0–5. T-09 implements this shape, which is final for P1a: SPR-020 moves `rh-core/events.meta`, not a segment | — | T-09 |
| `specs/sifting/patterns.solitary_life.toml` (`life_ends`) | Exists (PR #37) | — | — | T-11 |
| `specs/stream/channel_c_v0.sql` (`sift_match`) | In review: PR #45 (draft; `selftest_channel_c.py`, with a `life_ends` SQL twin) | PR #37 Q10 | Architect | T-11 |
| `household.yaml`, `hamlet.yaml` | Exist (PR #37) | — | — | T-13 |
| DATA-5 tellability (PR #37 Q8) and the canonical text form with `segment.canonical_digest` (Q11) | In review: PR #45 (draft): `05.2` §4.0 (the admission rule; its DATA-5 amendment awaits D38, and P1.0 admits every draft) and `specs/stream/canonical_v0.md` with `canonical_v0.py` and `golden/canonical_v0.txt` | — | Architect | T-09 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C3**: the channel-A record format and two new golden worlds. Contains C2: the segment schema, the `LIFE_ENTRY`/`LIFE_EXIT` kinds, two new crates (`rh-stream`, `rh-sifter`) and one new plane-A dependency (`parquet`) |
| Planes touched (ADR-0024) | A (`rh-stream`); C (`rh-sifter`, channel-C segments); tool (`rh-cli`, `bin/`, `specs/`). B: none |
| Owned columns written | None new |
| RNG purposes added | None |
| Golden-hash effect | New baselines for `household` and `hamlet`, announced to the director before merge (SPR-011, director approval 2). `solo` and `smoke` unchanged |
| Determinism lints (GOV-003 §2, §6) | Must pass. Pinned labels for `rh-stream` and `rh-sifter`; `parquet` in `PLANE_A_EXTERNAL_DEPS` |
| Tests that gate this sprint | V-DET-01, V-DET-02, V-DET-04, V-DET-05, V-DET-14, V-DET-18 (subset), V-INV-02 (nightly), V-DATA-10 (subset), V-STR-01, V-STR-05 (sealed), V-SIFT-01, V-SIFT-05, V-PLN-01, V-PLN-04 (subset), V-SCALE-01, V-SCALE-04, V-GOLD-01, V-WC-04 (new manifests), V-PERF-11 (recorded), plus `bin/check_all.sh` |
| Docs updated in the same sprint | `bin/check_all.sh` and `CLAUDE.md` checks; `05.0` §6 (new crates); `05.9` IF-0 and `05.10` §4–5 (ruling 13) |

## Tasks

Fixed in SPR-013's plan before it starts (PR #37's carried questions): what makes an act class "tellable" for DATA-5 (Q8; ruling 11 already admits every death at rungs 0–2 as tellable); the canonical text form and `segment.canonical_digest`, XXH3-128 over the canonical lines in `seq` order (Q11).

#### T-09: `rh-stream`: channel A and the sealed chronicle (rulings 11, 12, 13)
- **Deliverable:** Crate `rh-stream`, pinned (A, machinery), with `parquet` reviewed in. The record header and the P1.0 kinds; the in-instant order; per-sim-year sealed segments matching `schema.sql` and `specs/stream/channel_a_v0.sql` as amended by the envelope spec (every child table and dictionary written, empty or not: 16 files per segment; the `EVENT` canonical line with its `snapshots` and six-part `causes`; `cause_told` true on every P1.0 edge); atomic sealing; the per-segment canonical digest; the lossless one-line canonical text form and its parser. The host writes after each instant, from `Engine::drain_events`, the journal and the life-course delta.
- **Acceptance:** V-STR-01 on `solo` and a 60-person fixture: segments byte-identical across two runs and across 1 and N threads, and canonical digests equal. V-DATA-10 subset: a Python test reads every sealed table with DuckDB, with `schema.sql`'s types. `seq` is contiguous from 0 across segments. The canonical form round-trips every record (property test; the parser joins the nightly fuzz tier). A mid-year `kill -9` of `rh-cli run`, followed by a resume from the 10-year keyframe, re-emits no seq below the first sim-year not on disk. That year's lines, `STEP_DIGEST` included, are byte-identical to the unbroken run's. A seal in flight at the kill is lost, and the year is resealed to the same bytes. SPR-026's host starts its live edge from that same point (`specs/stream/wire_v1.md` §6.2 and §11; REV-SPR-026 R18). A `kill -9` during sealing leaves no partial segment, and the resumed run reseals the same bytes (GOV-002 tier 15). No channel-A table has a float column.
- **Status:** [x] done (`72be0e9`, `d927077`; envelope shape `0c33aa8`; review round 1 fixes MAJ-1, MAJ-3, MIN-3, MIN-4, MIN-10)

#### T-10: `rh-cli tail`
- **Deliverable:** `rh-cli tail <run-dir> [--from-seq N] [--follow <person>] [--kind K …] [--canonical] [--wait]`. It reads sealed segments in `seq` order and follows new ones as they seal. It prints the display line (date in the world calendar, kind, roles, causes; cast names from the manifest, tool side) or the canonical line, and the status line of `05.10` §2: last sealed tick and `seq`, the cursor, the lag. Channel-C matches appear as annotations when a channel-C segment exists.
- **Acceptance:** On a `solo` run, every channel-A record prints exactly once, in `seq` order, from any `--from-seq`. A tail started mid-run and one started after the run print the same lines (V-STR-05 for sealed segments). Tail never writes to the run directory (a read-only test). A stopped tail does not slow `rh-cli run` (wall time against headless, reported). File snapshots of the display form: committed files, updated only with `RH_UPDATE_SNAPSHOTS=1`, where a missing file fails and a planted control must FAIL (the Architect's ruling at T-10; `insta` was rejected; text amended by `VER-SPR-013`).
- **Status:** [x] done (`d927077`, `2c925f2`; file snapshots per the Architect's ruling, `0c33aa8`; review round 1 MIN-9, MIN-10)

#### T-11: The trivial sifter pattern and its twin (ruling 14)
- **Deliverable:** `life_ends` in `specs/sifting/patterns.solitary_life.toml` (Architect). Crate `rh-sifter` v0, pinned (C, machinery). `rh-cli sift <run-dir>` writes channel-C segments. The twin in `specs/sifting/twins/solitary_life.sql`, checked by `python -I specs/sifting/twin_check.py <run-dir>` (AGT-002-TOOLS).
- **Acceptance:** V-SIFT-05: `lint_patterns.py` passes `life_ends` and rejects a planted unanchored variant. V-SIFT-01: `rh-sifter`'s matches equal the twin's (match ids, bindings, event ids) on `solo`, `household`, `hamlet` and the 10,000-founder fixture. V-PLN-04 subset: sifting twice gives the same channel-C digest and leaves channel A and the state ladder unchanged. V-PLN-01: no plane-A crate depends on `rh-sifter`.
- **Status:** [x] done (`b019caa`, `a2dba6a`; review round 1 MIN-1, MIN-2)

#### T-12: Journal replay, end to end (ruling 15)
- **Deliverable:** `rh-cli run --inputs <file>` journals scope-only re-sample inputs through SPR-007's journal. `rh-cli replay <run-dir>` rebuilds from the history tuple and the journal alone. `rh-cli replay --from-keyframe <tick>` resumes through `rh_snapshot::resume`. `worlds/solo/inputs.toml` holds one input on Ada.
- **Acceptance:** V-DET-04: the replayed ladder and channel-A digests equal the original, every month. V-DET-18 subset: a replay in a clean directory holding only (build, Registry, manifest, seed, journal) is hash-equal, and every journal row has tick, seq, class, scope and producer. V-DET-05 on `solo` and `hamlet`: resume from the year-10 keyframe, advance 10 years, equal to the continuous run. A missing, extra or edited journal row hard-fails replay. V-DET-14 on `solo`: the input re-samples only Ada, keyed by her new epoch.
- **Status:** [x] done (`252b539`; review round 1 MAJ-2, MAJ-3, MIN-5 to MIN-8, VER-SPR-013-TEST DEF-1)

#### T-13: Rungs 1 and 2 on the same binary (rulings 16–19)
- **Deliverable:** `worlds/household/` and `worlds/hamlet/`, pointing at A-06's manifests, with input scripts and baselines. The PR-tier and nightly jobs of ruling 16. Nightly V-SCALE jobs. The keyframe-size report.
- **Acceptance:** One `rh-cli` build runs all three rungs; only the manifest changes. V-SCALE-01 at N ∈ {1, 5, 60}: 0 invariant violations, equal hashes across threads, exact replay. V-SCALE-04: each world reaches zero population, emits one `EXTINCTION`, and its chronicle, tail and replay stay consistent after it. V-WC-04: both manifests validate in Python and in Rust (`manifest_parity.py`). V-DET-01/02 nightly on all three rungs. V-INV-02: a 1,000-year debug soak with invariants on, 0 violations. V-PERF-11: steady-state allocations per tick recorded for each rung as its budget (SPR-009's counting allocator). Keyframe sizes per rung reported. The `household` and `hamlet` golden runs carry `--expect-serial`: M2 is their only system, so no scheduler level can hold two jobs and `--require-parallel` could never pass. That first text was the Architect's error, and this ruling corrects it. `--expect-serial` passes only while every level holds one job, and fails, naming the floor to set, once a second system arrives (`VER-SPR-012` GAP-2, GAP-7; text amended by `VER-SPR-013`).
- **Status:** [x] done (`322ff7f`, `0e3addc`, `b6f3157`; baselines `3f938a0` [C3] [perf-baseline])

## Out of scope (explicit)
- Births, partnering, households and kinship (P1.3); fixed-point `exp`/`ln` and Brass at any (α, β) (P1.3).
- Regions, settlements, routes and residence (P1.1a, `S3.migration`); place on events stays empty.
- The event envelope: anchored snapshots, typed and motive edges, the reconciled role codes (P1.9).
- The live socket, cursors as a protocol, `stream.subscribe`, the materializer, pace commands, the TUI stub (P1.17).
- The live in-process sifter and multi-clause patterns (P2.1).
- Same-instant command delivery, BCK-002 A-30 and A-31 (P1.13); exact lags, A-28 (P1.18).
- Food (P1b, D31); daily life (P1.18, in P1b since D33; Later for `minutes`, D31); worlds above 60 persons (P1.14).
- The infant sub-schedule (P1.3): rung 0's hazard stays linear through year 0 until then (PR #37 Q13, accepted).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | T-12, T-13: V-DET-01/02/04/05/14/18, V-STR-01, V-SCALE-01/04, replay from the journal alone and from a keyframe. Written from `05.1` §6, ADR-0025, ADR-0026 and `05.10` §7 before reading the implementation | `VER-SPR-013-TEST` |
| AGT-003-IF | T-09–T-11: segment conformance with `schema.sql` and `channel_a_v0.sql`; the canonical form and its parser (fuzz); atomic sealing under `kill -9`; `tail`; the twin (V-SIFT-01, V-SIFT-05) | As above |

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-012 not merged | Closed: SPR-012 merged (`df4a7f4`) |
| 2 | `specs/stream/channel_c_v0.sql` does not exist (PR #37 Q10) | Closed: PR #45 (reviewed) |
| 3 | The canonical line grammar and the general DATA-5 tellability rule are not written (PR #37 Q8, Q11) | Closed: PR #45 (reviewed) for the grammar. The DATA-5 amendment (`05.2` §4.0) awaits the director (D38); P1.0 admits every draft, so T-09 does not wait on it |

## Completion

- [x] All tasks meet their acceptance criteria.
- [x] `bin/check_all.sh` passes; the report is attached.
- [x] Tester verdicts are PASS.
- [x] Architect audit `VER-SPR-013` is PASS.
- [x] No open DEF- against this sprint.
- [x] Merge route matches the CI-detected change class; each golden-corpus change was announced to the director first.
- [ ] **Open after the merge, a director activity.** Rung checkpoints (GOV-007 §3.5): when `solo`, then `household`, then `hamlet` first runs green, the director watches its stream through `rh-cli tail` for 15–30 minutes. The Architect records the notes as `EVO-` or `DEF-` items.
