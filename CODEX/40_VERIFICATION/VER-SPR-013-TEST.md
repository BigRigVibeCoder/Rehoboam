---
id: VER-SPR-013-TEST
title: "Determinism and contracts tester evidence for SPR-013"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism, contracts]
related: [SPR-013, SPR-011, VER-SPR-012-TEST, AGT-003-DET, AGT-003-IF, ADR-0024, ADR-0025, ADR-0026]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF: T-09, T-10, T-11, T-12 and T-13 meet their acceptance criteria and I found no blocking defect. One low-severity defect (DEF-1) and eight contract or test-strength gaps.** `bin/check_all.sh` is **463 of 463** on a clean clone of `a64bc59` (445 of 445 on `0c33aa8`), and pytest passes (659) under the `pull_request`, `push` and `workflow_dispatch` shapes. My own probes, written from the contracts before I read the code, agree with the implementation everywhere I looked: a DuckDB reader of every sealed table, a `life_ends` oracle with my own MatchId and chain, a Rust-against-Python differential fuzz of the canonical parser (about 600,000 cases, 0 disagreements), an independent model of M2's death instants on all three golden worlds over 1,000 years, 100 `kill -9` runs, 27 journal and `run.toml` tampers, and 12 thread-count and repeat runs of each rung at 1,000 years that all gave one hash.

Tested heads (both on `sprint/SPR-013-stream`, PR #64):
- `0c33aa851831d4781cad54246b1b0f6e42d2380d` (T-09 to T-12, A-51 to A-53), the head the Architect sent first.
- `a64bc59a64190c41c8395e14100b5ab2217e5f6b` (adds T-13 rungs 1 and 2, the `household` and `hamlet` baselines, per-rung allocations, `--expect-serial`; includes a merge of `main` at `92cefab`), the branch tip when I started T-13. The Architect has since named `a64bc59` as the final head (it adds only merged `main` content to `eccdb77`), so every T-13 result here is on the final head.

Method: every probe ran on a `git archive` copy or a clean clone of the SHA (with `origin/main` set to the merge base for the baseline-integrity gate), each with its own absolute `CARGO_TARGET_DIR`. The Developer's tree was never touched. The probe plan was written from `SPR-013`, `SPR-011` rulings 11 to 19, `05.1` §6 and §9, `05.10` §4 to §8, `05.9` IF-0 to IF-2, `channel_a_v0.sql`, `channel_c_v0.sql`, `canonical_v0.md` and the `docs/09` tests before I read any implementation. My oracles use DuckDB, the standard library, and the repository's independent references (`canonical_v0.py`, `gen_hash_golden.py`, `gen_rng_golden.py`, `gen_m2_draws_reference.py`), never the Rust code under test as its own oracle. GitHub Actions had no runner, so nothing here is a CI result.

## 1. Verdicts

| Task | Verdict | Basis |
|---|---|---|
| T-09 `rh-stream`, sealed segments | **PASS with GAP-1, GAP-7** | Schema, seq, order, digests, bytes, atomic sealing and the parser all hold (§2.1) |
| T-10 `rh-cli tail` | **PASS with GAP-2** | Exactly once from any cursor, mid-run equals after-run, never writes, a stopped tail costs nothing (§2.2) |
| T-11 `rh-sifter`, `life_ends`, the twin | **PASS with GAP-4** | 10,000 of 10,000 matches equal my oracle; sifting twice is byte-identical; channel A untouched (§2.3) |
| T-12 journal replay | **PASS with DEF-1, GAP-3** | Replay equal on 3 worlds at 200 and 1,000 years; every tamper refused; V-DET-14 on `hamlet` (§2.4) |
| T-13 rungs 1 and 2 | **PASS with GAP-5, GAP-6** | V-SCALE-01 at N = 1, 5, 60 at 1,000 years; V-SCALE-04; baselines reproduce; `--expect-serial` works (§2.5) |
| A-51 file I/O out of `rh-registry` | **PASS** | Planted file reads fail in 7 of 7 places they must; `seal.rs` alone is allowed (§2.6) |
| A-52 mutant builds refused | **PASS** | A mutant build refuses `run`, `replay` and `build-info` (§2.6) |
| A-53 test strength | **PASS** | The enlarged draws golden kills a one-day error in q′; the other items are present (§2.6) |
| `bin/check_all.sh` whole | **PASS** | 463 of 463 on `a64bc59`; 445 of 445 on `0c33aa8` |
| pytest under CI's variables | **PASS** | 659 passed under all three event shapes |
| `smoke` and `solo` unchanged | **PASS** | No change to `worlds/smoke` or `worlds/solo` baselines against `main` (`92cefab`); `solo` and `smoke` golden gates pass |

## 2. Results

### 2.1 T-09: the stream and the sealed chronicle (AGT-003-IF)

Runs: `solo` (1 person), `household` (5), `hamlet` (60) and the 10,000-founder fixture, 150 to 1,000 years.

| Probe | Result |
|---|---|
| Every table of every segment against `channel_a_v0.sql` read with DuckDB (`probes/p9_schema.py`): 16 files per segment, column names, order and types equal the DDL built from the file itself; no `FLOAT`/`DOUBLE`/`DECIMAL`; no temporary file | 0 failures on 1,000 segments (`solo`), 200 each for the others |
| `seq` contiguous from 0 across segments; tick never decreases; `first_seq`, `last_seq`, `record_count`, tick range and `sim_year` of each `segment` row equal the data; `order_violation` empty (R4) | 0 failures |
| Semantics (`p94_sem.py`): `DEATH` events = `LIFE_EXIT` deaths = `LIFE_ENTRY` count = step-digest deaths; each exit has its `DEATH` at the same tick; one role (person, target), no actor; one `EXTINCTION` caused by the last `DEATH`; every death on a day start; `living` in each digest equals founders minus exits; entries in `PersonId` order; `DEATH` payload equals the exit's `death_cause` (60 of 60) | 0 failures; `solo`, `household`, `hamlet`, the fixture; 1,000 years |
| Tick 0 order (R4): two inputs at tick 0 and one at tick 1 on `solo` | INPUT, INPUT, ENTERS, DIGEST, then the tick-1 INPUT |
| Canonical digest: `tail --canonical --from-seq F --to-seq L` of each segment, piped through the reference `canonical_v0.py --digest`, against the stored `canonical_digest` | equal on 200 of 200 segments |
| Parser differential fuzz (`p95_fuzz.py`): the Rust `parse` and `render` against the reference `parse` and `render`. Seeds: the golden lines plus about 8,000 real lines from four runs; mutations: bit flips, deletions, insertions, token swaps and duplicates, integer boundaries (`2^63`, `2^64`, 20 and 21 characters, `-0`, leading zero), case flips of hex, escape forms, truncation, CR, NUL, high bytes | 600,000 mutated lines (about 120,000 accepted): **0 disagreements** on accept or reject, and every accepted line re-renders to itself |
| Byte identity: all stream and keyframe files of the same run at 1, 3 and 4 threads; yearly against monthly ladder; release, debug and `target-cpu=native` builds | one tree hash per world in every case |
| 12 seeds (including `2^32` and `2^64 - 1`) x 3 worlds at 1 and 4 threads | 0 divergences |
| Corruption: 60 single-bit flips in random tables of random segments; `tail --canonical` against the clean output | every flip that changed any printed byte was refused; the rest left output identical (padding, metadata) |
| `kill -9` (`p97_kill.sh`): 100 runs (`hamlet` 40, `solo` 40, `household` 20), killed at random instants from 20 ms to 1.9 s, then `tail` on the half-written directory, then a resume from the last keyframe into the same directory | 0 incomplete visible segments; a hidden `.partial` directory was left in 72 runs (mid-seal kills) and is invisible to `tail`; every resumed tree equals the uninterrupted run (100 of 100). A rerun into the same directory without `--resume` is refused ("already holds sealed segment 0") |
| DDL mutations (`check_segments.py` against a changed `channel_a_v0.sql`): a changed column type, a renamed column, an added column, an added `DOUBLE`, a narrowed `phase`, two changed `CHECK`s | 7 of 7 fail (a `CHECK` that the data already satisfies is not a mutant) |

### 2.2 T-10: `rh-cli tail` (AGT-003-IF)

| Probe | Result |
|---|---|
| Every record exactly once, in order, from `--from-seq` 0, 1, 7, 100, last, last + 1, last + 5, and the first and last `seq` of every one of 200 segments | equal to the full output's tail in every case |
| A tail started 1.5 s into a 15 s run (`--wait`), canonical and display, against a tail after the run | equal (the `# STATUS` lines differ by design); it exits when the run ends |
| Never writes: sizes, names, mtimes and modes of the run directory before and after; `strace` of `tail` and `sift` for write-mode opens, `mkdir`, `rename`, `unlink` | unchanged; `tail` writes nothing, `sift` writes only under `C/` |
| A tail stopped with `SIGSTOP` and a live tail attached during a 15 s run of the 10,000-founder fixture | 15.40 s headless, 15.48 s with both: no slowdown |
| `--follow` of a cast name and a person id, an unknown name, `--kind DEATH`, a cursor beyond the end, a missing directory, a pipe closed by `head -1` | right lines; exit 2 for an unknown person or directory; exit 0 and no panic on a closed pipe |
| Corrupt segment (a trailing byte, a flipped byte) | exit 2 naming the file; never skipped |
| Display snapshots | `RH_UPDATE_SNAPSHOTS` under `CI=1` is refused; a missing snapshot fails (check_all controls pass) |

### 2.3 T-11: the sifter and its twin (AGT-003-IF)

| Probe | Result |
|---|---|
| `life_ends` from my own DuckDB oracle straight from channel A (`p11_oracle.py`): one match per `DEATH` target, `MatchId` from `canonical_v0.md` §6 recomputed with the standard-library XXH3, binding `A -> (1, id)`, `e1`, semantics, `closed_by_seq`, channel-C `seq`, and the R7 chain `a_chain` | equal on `solo`, `household`, `hamlet` (200 and 1,000 years) and the fixture: **10,000 of 10,000 matches** |
| The repository's own twin (`twin_check.py`) on the same runs | equal |
| Sifting twice; sifting after deleting channel C; sifting a run with a gap in channel A; with a corrupt segment; with years 120 to 199 missing, then added | byte-identical C directories; the gap and the corrupt file are refused (exit 2); sifting after the missing years return equals the full sift |
| Channel A, keyframes, journal, ladder and `run.toml` before and after `sift` | unchanged (hash); `strace`: writes only under `C/` |
| V-SIFT-05 (`lint_patterns.py`): a bare read, an anchor on an unbound clause, an undeclared attribute, an undeclared selection, a missing `selection`, `on_complete` or `firing_budget`, an unknown verb, an undeclared role variable | all refused; an anchored read passes |
| V-PLN-01: `cargo tree -i rh-sifter`; every plane-A crate's normal and build tree | only `rh-golden`, `rh-bench` and `rh-cli` depend on it; no plane-A crate does |
| Mutation of the chain function (arguments swapped) | passes the Rust tests, **killed by `twin_check.py`** (CHAIN), which is the gate that has to catch it |

### 2.4 T-12: replay (AGT-003-DET)

| Probe | Result |
|---|---|
| `rh-cli replay` of `solo`, `household`, `hamlet` at 200 years (monthly ladder) and at 1,000 years (4 threads) | REPLAY EQUAL: ladder, every segment and every keyframe byte-identical |
| Clean directory holding only `run.toml` and `journal.sqlite` | "REPLAY REBUILT ... nothing to compare" (rc 0): rebuilds from the journal alone, as V-DET-18 needs |
| Journal audit: every `inputs` row has `tick`, `seq`, `class`, `scope`, `scope_id`, `producer`, `payload_hash`, `row_digest`, `chain` | present on all rows |
| 27 tampers (`p123_tamper.sh`): delete the row; delete and zero the count; edit tick, scope id, producer, payload, kind, seq, count, head chain, seed, build, registry or manifest hash; insert an extra row; edit `run.toml`'s seed, head or input count; swap its world; truncate, empty or randomise the journal; remove the journal or `run.toml`; change the calendar | 25 refused with rc 2 and a message naming the cause; two edits (`run.toml`'s `years` and `manifest_hash` lines) are accepted (GAP-3) |
| Re-chained edit (a genuine journal from a run with the input one minute later, spliced in with its `run.toml` head) | REPLAY DIFFERS, rc 1 (STREAM digest and KEYFRAME bytes differ at year 0) |
| `--from-keyframe` at the last minute of year 10, 100, 190 | REPLAY EQUAL on all three worlds. A tick that is not a year's last minute, negative, or past the last keyframe is refused (rc 2). **At the last keyframe (year 200): REPLAY DIFFERS, rc 1 (DEF-1)** |
| V-DET-14 on `hamlet`: one input on each of 5 founders (ticks 100 to 30,000,000); per-person death ticks against the run with no input | exactly one person differs each time (the scoped one, matched to the journal's `scope_id`); 4 of 5 deaths move, 1 does not |
| Independent model of M2's draws (`p123_model.py`), built on `gen_m2_draws_reference.py`, fed only the births and the journal from the run: `solo`, `household`, `hamlet` at 1,000 years | **all 1, 6 and 60 death instants equal the engine's**; each committed input takes the `pending` path and misses, as the input script says |
| Crash: `kill -9` at random points, then a resume from the last keyframe (§2.1) | equal to the continuous run in 100 of 100 |
| Input script entries: a tick on a dead person (year 57, at year 150) is journaled and replays; a tick at the horizon or beyond | dropped, not journaled, no message (GAP-3) |

### 2.5 T-13: rungs 1 and 2 on one binary (AGT-003-DET), `a64bc59`

| Probe | Result |
|---|---|
| One `rh-cli` build runs all three rungs; only `world.toml` and the manifest differ | yes; each `world.toml` pins its manifest's SHA-256 |
| V-SCALE-01: `solo`, `household`, `hamlet`, 1,000 years monthly, `--expect-serial`, 1, 2, 4 and 8 threads x 3 repeats (12 runs each): stream, keyframes and ladder | **one distinct hash per world**; replay at 4 threads REPLAY EQUAL (12,000 rungs, 1,000 segments, 100 keyframes) |
| V-SCALE-04, 1,000 years: one `EXTINCTION` per world (years 0057 `solo`, 0061 `household`, 0078 `hamlet`); `living` 0 afterwards; the chronicle past extinction (§2.1 checks), `tail` canonical digest, `sift` (60 of 60 matches for `hamlet`), the twin and replay all consistent. Segments continue to be sealed after extinction (each costs about 13.5 KB, 16 files) | PASS |
| The committed `household` and `hamlet` baselines against my recomputation at 200 years, 1 thread, serial | byte-equal (with `solo`) |
| Debug, release and `target-cpu=native` builds, 200 years, 4 threads | one hash per world |
| V-INV-02 soak: debug build, 1,000 years yearly, with the input scripts | rc 0, ladders equal to the release ones |
| `--expect-serial`: `rh-cli run smoke ... --expect-serial` | refused: "NOT SERIAL: 14600 of 206475 levels held two or more jobs", rc 1; the same run without the flag is rc 0 |
| `smoke` and `solo` baselines | unchanged against `main`; `3f938a0` carries `[C3] [perf-baseline]` as its trailing tag group |
| V-PERF-11 budgets recorded | `solo` 1,068 allocations over 12 ticks, `household` 1,179 over 13, `hamlet` 1,502 over 14, 1 thread |
| Keyframe sizes (200 years, 20 keyframes each) | `solo` and `household`: 11,010 to 16,986 bytes (total 250,080 and 256,056); `hamlet`: 14,594 to 22,746 (total 340,688). The stream is 2.7 MB per world; journal 20 KB |
| V-WC-04 for both manifests, V-GOLD-01, the nightly script's commands | in `check_all` (all PASS); the 1,000-year forms I ran by hand as above |
| Mutants of the extinction rule and the serial check (matrix, §4) | all killed by a named gate |

### 2.6 The backlog items (AGT-003-IF)

| Item | Probe | Result |
|---|---|---|
| A-51 | `grep` of `crates/rh-registry/src` for `fs`, `File`, `read_to_string`: none. Planted in a copy: `std::fs::read` in `rh-stream/src/segment.rs`, in `writer.rs`, in `rh-core`, in `rh-worldgen`, an aliased `use std as s; s::fs::read` in `rh-registry`, `File::open` through a `use`, a file named `seal.rs` in a subdirectory (`rh-stream/src/x/seal.rs`) and a `seal_helper.rs` | each fails `lint_forbidden.py` (`V-DET-19/file-io`); the same call inside `rh-stream/src/seal.rs` passes (the ruling), and `std::process::Command` in `seal.rs` still fails. The `spec-loader` rule is gone |
| A-52 | `rh-cli` built with `mutant-resample-day-k0`: `build-info`, `run solo`, `run smoke`, `run hamlet`, `replay`, `sift`, `tail` | `build-info` prints `mutants: mutant-resample-day-k0` (rc 1); `run` and `replay` stop with `MUTANT BUILD` (rc 2) and write nothing; `sift` and `tail` still work (they run no engine). `regen_baseline.sh` builds its own binary and then checks `build-info` |
| A-53 | `draws_golden` now has 530 re-samples in 7 worlds; I planted a one-day error in q′ (`interval_q(.., end - 1440)`); whitespace-only `pass:`/`fail:` text; the stale feature name; the `spec-loader` exemption | the one-day mutant **fails** (`w7: death 3 differs`); the harness controls for blank text exist and pass; the feature names are current; the exemption rule retired with A-51 |

### 2.7 The Architect's six T-13 points, and CI (`a64bc59`)

1. **Baselines:** `check_baseline` passes in `check_all`; the `household` and `hamlet` ladders equal my own 200-year recomputation at 1 and 4 threads and from debug and `native` builds; `smoke` and `solo` are unchanged. `0c33aa8` is still an ancestor of `a64bc59` and unchanged.
2. **`--expect-serial`:** passes on `household` and `hamlet` (1,000 years, 1 to 8 threads); the `smoke` run is refused with its message.
3. **V-PERF-11:** `perf_gate.py alloc-record` on a clean clone of `a64bc59` wrote `allocations_v1.txt` **byte-identical** to the committed file (smoke 819,401 allocations and 281 at most per tick at 1 thread, and the three `rung` lines); `alloc-gate` passes in `check_all`.
4. **Input scripts:** an independent model fed only the births and the journal shows the input on Oswin (`household`, tick 2,377,807) and on Job (`hamlet`, tick 17,772,847) each landing on a pending death (`pending`, re-sample misses), and the engine's death instants equal the model's (§2.4).
5. **V-SCALE-04 / `rungs`:** consistent past extinction at 1,000 years (§2.5), and the `rungs` mutants are killed (§4).
6. **Nightly entries by hand:** V-DET-01/02 (12 runs per world, 1,000 years), V-SCALE-01/04 (replay at 4 threads, one EXTINCTION, sift, twin, `check_segments`), V-INV-02 (debug 1,000-year soak): all pass. The script itself I did not run.

CI (run 39025299299 on `a64bc59`, PR #64): success. `--part core` 12 min 04 s inside `bin/check_all.sh` (job 12 min 41 s); `--part golden` 9 min 18 s (job 10 min 22 s); wall 13.2 min for the run. The earlier run on `eccdb77` (39025105444) was cancelled after 2.5 minutes by the newer push.

## 3. Defects and gaps

### DEF-1 (low): `replay --from-keyframe` at the run's last keyframe reports a false failure

Contract: T-12 and V-DET-04, a replay from a keyframe equals the continuous run.
Repro (on `0c33aa8` and `a64bc59`):
```
rh-cli run solo --years 200 --ladder monthly --run-dir D --inputs worlds/solo/inputs.toml
rh-cli replay D --into E --from-keyframe 103679999
```
Actual: `REPLAY DIFFERS ... LADDER: the resumed replay's 0 rungs are not the run's last 0`, rc 1. Expected: REPLAY EQUAL (nothing remains to replay; the stream and keyframes are the run's). The same happens on `household` and `hamlet`. Years 10, 100 and 190 are equal. It is an edge of an unreachable use, so low severity, but a gate that fails on a correct run teaches people to ignore it.

### Contract and test-strength gaps (none is a defect)

| # | Gap | Recommendation | Confidence |
|---|---|---|---|
| GAP-1 | Test strength in `rh-stream`. Seven mutants survive the repository's `rh-stream` tests (§4): a duplicate role, cause or event id (`>=` to `>`) is not planted in the Rust tests, though both parsers refuse it (my fuzz sees it); a CR or non-ASCII byte inside a `#` comment line of a canonical file passes `digest_file`; `StreamWriter` accepts a record whose R4 key equals the last one; `seal()` does not check that `segment.parquet` is last; re-sealing and stale-`.partial` guards are untested | Add planted cases for each (the reference's `REFUSED` list has the first three); make `digest_file` refuse CR and non-ASCII in comments too, or say they are free | 75% |
| GAP-2 | `tail --kind NOPE` (an unknown kind name) prints nothing and exits 0, while `--follow` of an unknown person exits 2 | Refuse an unknown kind against the segment's `verb` dictionary and the record kinds | 70% |
| GAP-3 | Replay and the input script are lenient in three places: an edited `manifest_hash` line in `run.toml` is accepted (the journal's own history tuple still binds the rebuild); an edited `years` is accepted (it only shortens the replay); an input past the horizon is silently dropped from the journal, so a script that never takes effect looks the same as one that does | Check `run.toml` against the manifest it rebuilds from; warn or refuse an input past the run's end | 60% |
| GAP-4 | `lint_patterns.py` accepts `life_ends` with its only role binding deleted (a declared role `A` that no clause binds): the pattern would match every `DEATH` with an empty binding | Refuse a declared role that no clause binds | 60% |
| GAP-5 | V-DET-01/02 on `household` and `hamlet` cannot fail: dispatch holds one job per level (`--expect-serial` enforces it; `solo` reports 0 of 2,401 levels with two jobs), and within-system parallelism needs more than 65,536 rows. Thread-count invariance is exercised only by `smoke`. The ruling (`--expect-serial`) makes the vacuity explicit and trips when a second system arrives, which is the right guard | State in `docs/09` V-SCALE-01 that for M2-only rungs the thread sweep is a regression guard; add a rung with a second system as soon as one exists | 75% |
| GAP-6 | Baselines for `household` and `hamlet` equal my recomputation, but I did not re-derive their genesis table roots independently as I did for `solo` in `VER-SPR-012-TEST` (§5). The death instants are independently confirmed (§2.4) | Re-derive the 19 world-column roots of the new baselines from the plane-hash reference, in the next sprint that touches worldgen | 65% |
| GAP-7 | Volume: each sealed year is 16 Parquet files, about 13.5 KB, even for an empty year, so a 60-person world is 2.7 MB per 200 years and 13 MB per 1,000 (about 1 KB per record against `05.10` §5's 10.5 bytes per event). Immaterial at rung 2; at 1,000 persons x 100 years it is dominated by real events | Record the fixed per-segment cost in `05.10` §5 so the 10.5 bytes per event is not read as the whole figure | 55% |

`tail`'s file-read cost is about 7 ms per segment (1.5 s for 200 segments), so a cursor near the end of a long run still reads every segment's footer. Not a contract item at this size.

## 4. Mutation matrix

K = killed; S = survived; where a mutant survived the Rust tests I say which other gate killed it, if one did. Tests are the repository's (`cargo nextest`) unless stated; the mutants are in a scratch copy and nothing was committed.

| ID | Mutant | Result |
|---|---|---|
| c01 | integer leading zeros accepted | K (`canonical` planted lines, mutated lines) |
| c02 | no 20-character limit on integer tokens | S, equivalent: the range check on `i64`/`u64` rejects the same tokens |
| c03 | uppercase hex accepted in a hash | K |
| c04 | lowercase hex accepted in an escape | K |
| c05 | escaped safe byte accepted | K |
| c06 | empty string token (not `%`) accepted | K |
| c07 | unescaped byte outside the safe set accepted | K |
| c08 | digest without the final LF | K (golden, `parquet_props`) |
| c09 | CR in a file accepted | S: only a CR inside a `#` comment line differs (GAP-1) |
| c10 | seq gap in a canonical file accepted | K |
| c11 | tick going back accepted | K |
| c12 | duplicate role accepted | S: **real gap** (GAP-1); the Rust parser refuses the line in the baseline |
| c13 | duplicate cause accepted | S: real gap (GAP-1) |
| c14 | duplicate binding role accepted | K |
| c15 | duplicate event id in a match accepted | S: real gap (GAP-1) |
| c16 | `MATCH` closing event not among its events accepted | K |
| c17 | `MATCH` opened after it closed accepted | K |
| c18 | non-ASCII byte in a file accepted | S: only in a comment line (GAP-1) |
| c19 | file without a final LF accepted | K |
| w01 | in-instant order: class rank reversed | K |
| w02 | in-instant order: id descending | K |
| w03 | writer accepts a record whose key equals the last | S: real gap (GAP-1) |
| w04 | resume at a tick that is not a year's last minute | K |
| w05 | resume leaves stale `.partial` directories | K |
| s01 | `seal()` does not check `segment.parquet` is last | S: real gap (GAP-1) |
| s02 | `seal()` overwrites an existing sealed directory | S: the `rename` onto a non-empty directory fails anyway (equivalent in effect) |
| s03 | stale `.partial` not removed before sealing | S: `create_dir` then fails (equivalent in effect); the resume path removes it (w05 K) |
| s04 | segment file names with path characters accepted | S: real gap (GAP-1), a defensive check only |
| t01 | `tail --from-seq` exclusive | K |
| t02 | `tail --to-seq` exclusive | K |
| x01 | channel-C chain arguments swapped | S in Rust tests; **K by `twin_check.py`** (CHAIN) |
| x02 | sifter does not de-duplicate bindings | S, equivalent for `life_ends` (one binding per target) |
| x03 | sifter ignores the clause's verb | K |
| x04 | channel-C matches written in the wrong order | K |
| r01 | `EXTINCTION` when any person is dead | K (`rungs`) |
| r02 | `EXTINCTION` never | K (`rungs`) |
| r03 | `EXTINCTION` without its cause edge | K (`m2_system`) |
| r04 | `DEATH` payload cause + 1 | S in `rungs` and `m2_system`; K by the `tail` display snapshot (`the_display_form_matches_its_snapshots`). No gate compares the payload with the exit's `death_cause` by value |
| r05 | `check_serial` always passes | S in nextest; killed by the `check_all` control (`smoke --expect-serial` must fail), by construction (I did not build it) |
| r06 | `check_serial` needs 3 jobs | same as r05 |
| a53 | one-day error in q′ (A-53 GAP-8) | K (`draws_golden`: 530 re-samples) |
| g01 | delete a `check_all` gate line | K (`test_check_all_inventory`) |
| d01–d07 | `channel_a_v0.sql`: a type, a name, an added column, a `DOUBLE`, a width, two `CHECK`s the data violates | 7 of 7 K by `check_segments.py` (a `CHECK` the data already satisfies, `record_count > 5`, is equivalent) |
| f01 | a planted `std::fs` use in 7 places, an alias and a look-alike `seal.rs` | K in all (§2.6) |

Of the 40 Rust-code mutants (c, w, s, t, x, r), 24 are killed by the Rust tests, 4 more by another named gate (x01, r04, r05, r06), 4 are equivalent in effect (c02, s02, s03, x02), and 8 survive every gate: c12, c13, c15, w03 and s01 are real test-strength gaps; c09, c18 and s04 are corners (comment lines, a defensive check). a53 is killed as well.

## 5. What was not run

- **Power-loss behaviour of `fsync`:** I killed processes, I did not cut power. The order (file, `fsync`, rename, directory `fsync`) is as the contract says; nothing here proves the disk honoured it.
- **A second platform or architecture:** one x86-64 Linux machine; debug, release and `target-cpu=native` builds only.
- **GitHub Actions:** no runner; `bin/check_all.sh` and pytest ran locally on clean clones.
- **`bin/golden_nightly.sh` whole:** I ran its commands by hand (1,000-year thread sweeps, the debug soak, replay, twin) but not the script, the V-DET-11 bisect x 64, or `exp_ln_v0.py --exhaustive`.
- **Pace:** the CLI has no pace control, so V-STR-01's "MAX against 1 yr/s" is not exercised; I compared yearly against monthly ladder cadence instead.
- **Genesis roots of the `household` and `hamlet` baselines from an independent reference (GAP-6).**
- **The 10,000-founder fixture at 1,000 years, or any world above 60 persons for a long run.**
- **A killed `sift`:** I killed `run`, not `sift` (its sealing code is shared and the sifter is re-runnable).
