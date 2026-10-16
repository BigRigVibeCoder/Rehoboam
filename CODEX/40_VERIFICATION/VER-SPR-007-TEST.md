---
id: VER-SPR-007-TEST
title: "Determinism tester evidence for SPR-007"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-007, VER-SPR-007, VER-SPR-003-TEST, AGT-003-DET, ADR-0006, ADR-0024]
created: 2026-10-16
updated: 2026-10-16
version: 2.0.0
---

> **BLUF (rounds 1 and 2: all four tasks, T-01 to T-04):** Round 1 found two defects and five gaps, all fixed and re-verified. Round 2 (T-02 keyframes) passes V-DET-05 at 47 cut points and in every variant I could build (1, 2, 4 and 8 threads, inputs submitted after the load, chained keyframes, a write in one process and a load in another, debug and release), and my own implementation of the eight engine tables reproduces `state_hash` on 13 states and the 120-month stub ladder. **Two defects are open:** DEF-3, a keyframe with a valid envelope and a corrupt Arrow body panics or aborts the loader (338 panics and 19 aborts in 3,946 forged files at the final head); DEF-4, `write_keyframe` stamps the caller's history tuple, and a keyframe stamped with the wrong seed and salt restores an engine that diverges after 9 instants while hashing equal at load. **Open gaps:** GAP-7 (the journal anchor in a keyframe has no verifier), GAP-8 (history fields other than the seed are not pinned by a test), GAP-9 (`Engine::restore` accepts an agenda entry dated before the cursor). GAP-6, that `state_hash` depended on when the host drained events (782 of 782 instants differed), was fixed by the Developer during the round and is re-verified. `bin/check_all.sh` is green, 165 of 165, on the final head.

Tested commits: first pass `bb82bceab347fa16ca90257738c2652b52d7c495` (PR #25, stacked on SPR-005 `fc635dc`); re-test on the rebased head `2bb679d817470baf711bf0bc2bc8712c87aedfb8` (onto main `b0cf3e2` after SPR-005 merged) on `766bdb901dfa997541852c2e7d79711a37e3c351` (`MerkleCache::update` free of caller-supplied versions), and on the round-1 final head `867bf954df2d2876f0ca06aa69488e48def5145f` (GAP-5 tests). Round 2 ran on `2ef968eb5f3606fe866c01c38530f2f1e6951b3a` (PR #25 rebased onto main with SPR-006) and was re-run on the final head `ee7d6ff4e4f83762f615f2e683829819e1ca487c` (held events carried outside `state_hash`). The round-2 probes are Rust tests and examples added to throwaway copies (`tester_kf.rs`, in the scratchpad `ver/` directory with `forge.py`, `enc8.py` and `crash2.py`); the repository's working tree was only read. Probes ran on `git archive` copies with a fresh `CARGO_TARGET_DIR` per copy; the repository's working tree was only read. The probe plan was written from the contracts (SPR-007 v1.2.0 with its six rulings, ADR-0006, ADR-0024, ADR-0007, `05.1` §6 items 4, 6, 7, 8, `05.9` IF-2, `09` V-DET-05, V-DET-10, V-DET-11) before the branch was opened; the Architect's later rulings on layout documentation, forced CFLAGS and transitive crates were applied. Reference code: `hashref.py` (the layout, from the text, with XXH3 from python-xxhash 4.0.1 on xxHash 0.8.3), `check_golden.py`, `check_stub.py`, `jv2.py` (row digest and chain, plus 22 tamper cases), `fptable.sh`, `crash.py` and `crash2.py`, and Rust probes added to throwaway copies. They are in the session scratchpad `ver/` directory. Tester model: set by the Architect in GOV-005 §4.2; not recorded here by instruction.

## 1. Verdicts (round 1)

| Task | Verdict | Basis |
|---|---|---|
| T-01 plane hashes | **PASS** (DEF-1, DEF-2 fixed) | Independent reference reproduces every golden line and the stub-world ladder, before and after the rebase; one-row change alters exactly one chunk; dirty tracking equals full recomputation over 6,000 random edits; seven builds identical; the DEF-1 fix survives 400 random steps over cloned worlds. |
| T-03 journal | **PASS** (GAP-3 and GAP-5 fixed) | IF-2 fields round-trip including extremes; `payload_hash` equals the C library's XXH3-128; STRICT schema with no `REAL`; replay reads only the file; row digest and chain reproduced independently; 22 of 22 tamper cases detected (one accepted by design); 120 kill -9 rounds clean on v1 and v2. |
| T-04 unsynced guard | **PASS** | Guard trips on an injected write in debug, is compiled out in release, and both are in `check_all.sh`; the guard mutants are red. Since T-02 it also covers engine-held state (GAP-4 closed, section 8). |
| Gates (`lint_arch`, `lint_forbidden`, fp-contract, inventory) | **PASS** (GAP-1, GAP-2 fixed) | 11 of 11 first-pass gate mutations red; the GAP-1 snippets and eight evasion attempts are refused; the GAP-2 variables are flagged; every deletion of a new gate line turns the inventory test red. |
| `bin/check_all.sh` | **PASS** | 152 of 152 on `bb82bce`; 160 of 160 on `2bb679d`, `766bdb9` and `867bf95`; 165 of 165 on `2ef968e` and `ee7d6ff` (clean archives, fresh target directories). |
| T-02 keyframes, V-DET-05 | **PASS with DEF-3, DEF-4 open** (round 2, section 7) | V-DET-05 holds at 47 cuts and every variant; engine-table encoding reproduced independently; all accidental corruption refused; but a re-sealed corrupt body crashes the loader, and the history tuple is trusted from the caller. |

## 2. Results

### 2.1 Hashes (T-01)

| Probe | Expected | Actual | Verdict |
|---|---|---|---|
| My implementation of layout v1 vs `rh_snapshot_hash_v1.txt` (268 XXH3-64, 268 XXH3-128, 7 column, 4 bytes-row, 3 empty-plane and 3 populated-plane lines) | equal | 553 of 553, on `bb82bce` and again on `2bb679d` | PASS |
| Stub world, 120 monthly instants (seed 0x5EED): I dumped the eight columns from the engine and hashed them myself, and compared with the engine's hash and with `stub_world_hashes.txt` | three-way equal | 120 of 120 on `bb82bce` and on the rebased tree; the dumped tables are byte-identical across the two | PASS (the declared C3 golden regeneration is correct; the moved file has the same 120 values) |
| One-row change at rows 0, 1, 1023, 1024, 1025, 2047, 2048, 2999 of a 3,000-row table | exactly chunk ⌊r/1024⌋ differs; root and plane change | as expected | PASS |
| `MerkleCache` against full recomputation: 200 random tables × 30 steps of row edit with `mark_rows`, append, truncate, range edit | equal at every step | equal | PASS |
| Name order (prefix names `a`/`ab`, empty name, non-ASCII), duplicate names, empty vs absent table, row framing (`["ab","c"]` vs `["a","bc"]`, two empty rows vs one), plane letter, row order, table name | each pair differs; unsorted or duplicate names refused | as expected | PASS |
| Divergence report: perturb one row of a 5,000-row table and one of a small one | first table by name, chunk 4, first row 4096 | as expected | PASS |
| Builds: dev, release (opt-level 3, s, 1 without debug assertions), `+fma,+avx2`, `target-cpu=native`, fat LTO with one codegen unit | identical results | the golden test, the hash tests and all stub-world determinism tests pass in all | PASS |
| DEF-1 attacks on the fix: 400 random steps over a growing pool of worlds (clone chains, replace by clone, column edits, truncation) with `update_world` compared to `state_hash` and `verify()`; `Default`, `mem::swap`, `mem::take`; two caches on one world; 160,000 clones over 8 threads | equal; identities unique | equal; all 160,000 identities unique | PASS |
| DEF-2 attacks on the fix: `mark_rows` over `0..usize::MAX`, `usize::MAX-5..usize::MAX`, a reversed range, an unknown table, then a resize and an edit past the range | returns; digest equals a full recompute | as expected | PASS |

### 2.2 Journal (T-03)

| Probe | Result | Verdict |
|---|---|---|
| Round trip of every kind, scope and producer variant; ticks `i64::MIN`, `i64::MAX`, −5; ids `u64::MAX`, 2^63; a 100,000-byte, an empty and a binary payload; strings with a quote, `;`, NUL and an emoji; two timelines | all equal, `seq` per timeline from 0 | PASS |
| Read back through Python's `sqlite3`: `payload_hash` against the C library's XXH3-128 canonical (big-endian) form | equal on every row | PASS |
| Row digest and chain (v2): my implementation from the `row_digest` documentation (domain tags, little-endian integers, `u32`-prefixed fields, `u8` presence flags, genesis from the timeline id) against the digest and chain columns of the Developer's file | 7 of 7 rows, both timelines and both heads equal | PASS |
| Schema: all tables STRICT; column types INTEGER, TEXT and BLOB only; `typeof` of tick, seq and scope_id is integer; `journal_mode` is `wal` | as ruled | PASS |
| Writer killed with SIGKILL at random moments: 60 rounds, 12,972 appends (v1), then 60 rounds, 13,185 appends over two timelines (v2) | `integrity_check` ok every round; `seq` contiguous; count = rows; v2: a full replay with digest and chain verification passes on both timelines every round | PASS |
| Two `Journal` handles appending alternately to one file | no duplicate or gap in `seq` | PASS |
| Tamper set on a v2 file, 22 cases: tick, scope, scope id, kind, kind arg, producer, job id, weights hash, prompt hash, provider version, payload (with and without a zeroed payload hash), last row dropped with the count lowered, last row dropped only, middle row dropped, two rows' `seq` swapped, row digest, chain value, head, count raised, a row moved between timelines, history seed | each replay hard-fails with the right kind (`Mismatched`, `Missing` or `Extra`) naming the row | PASS |
| Tail truncation with the count and the head both rewritten from the stored chain value | replays as a shorter journal | accepted by design: it needs no hashing, and only an external anchor (the keyframe's journal position, T-02) can see it. The module documentation says it is integrity against accident and naive edits, not a signature. Round 2 checks the anchor. |
| A tampered row of timeline 1 while replaying timeline 0 | timeline 0 replays | PASS (isolated by design; a replay verifies its own timeline) |
| Copy of the main file without its `-wal` while a writer is open | refused, not a short journal | PASS |
| `Replay::expect` with an extra live input before the journal's next input | `Extra` | PASS (source and test; OBS-1 closed) |
| `Journal::durability()`, the PRAGMA read-back, `:memory:` refused | present in the source and covered by the Developer's tests; the mutant `synchronous = OFF` is red | PASS |

### 2.3 Unsynced guard (T-04)

Two mutants (`never trips`, `hash before only`) turn `guard` red on both heads. The Developer's V-DET-10 fuzz on the stub world (now in `rh-snapshot/tests/unsynced_guard.rs`), the positive control and the release-absence test all pass in the gate. The guard hashes only `WorldTables` today; see GAP-4.

### 2.4 Gates

| Probe | Expected | Actual |
|---|---|---|
| `lint_arch`: twox-hash with default features; rusqlite with `bundled-full`; rusqlite unpinned; twox-hash with `xxhash64`; an extra crate (itoa) in rh-snapshot; twox-hash in rh-core | each a `plane-A-external` violation | 6 of 6 red, each with its own message |
| `float-dep` rule (GAP-1): `ValueRef::Real(x) => x.to_bits()`, `x < y` on two reals, `x.max(y).floor().to_bits()`; `r#Real`; a comment between `::` and `Real`; an aliased import; `as_f64` called directly and through `ValueRef as V`; `Type::Real` | each refused by `lint_forbidden` (`as_f64` also by clippy's disallowed method) | 9 of 9 refused; the control `v.data_type() == Type::Integer` passes |
| fp-contract guard truth table (65 cases: CFLAGS values, `CFLAGS_<target>`, `CC_ENCODED_CFLAGS`, four RUSTFLAGS forms, config forms: forced, unforced, string, table, duplicate key, nested, legacy file, commented) plus `CXXFLAGS`, `CC`, `CXX`, `HOST_*`, `TARGET_*` and `_<target>` variants (GAP-2) | accepts only `-ffp-contract=off` forced in CFLAGS | as specified; `CPPFLAGS`, `CPP` and `LDFLAGS` are not scanned, and the cc crate reads none of them; one case of mine, a TOML value split across lines, fails closed |
| Config mutations: `force` dropped, config removed, an extra flag next to `off`, CFLAGS moved to `CFLAGS_<target>` | red | 4 of 4 red (`c_fp_contract_check` catches removal and extra flags) |
| Real compiler line (a logging `CC`): with `CFLAGS_x86_64_unknown_linux_gnu="-O2 -mfma"` set | forced flag still present | the cc crate concatenates the target-specific and generic flags; `-ffp-contract=off` stays on the gcc command line |
| Delete each new `run` line of `check_all.sh` | inventory test red | 10 of 10 red |
| Implementation mutants against the Developer's suites: 23 on `bb82bce` (layout bytes, dirty tracking, replay checks, journal pragmas, guard) and 25 on `2bb679d` (row digest fields, chain, head check, identity, clamp, expect) | red | 21 of 23, then 21 of 25 (the four GAP-5 survivors); on `867bf95` the six digest and chain mutants are red; the only survivor is one equivalent mutant (the `synchronous` read-back removed, whose `OFF` twin is red) |

## 3. Defects (both fixed)

### DEF-1 (medium, latent; fixed in `6409856`, narrowed in `766bdb9`): the Merkle cache trusted per-instance column versions
`WorldTables::insert` took a version from a counter that belonged to the instance, and `MerkleCache::update` treated an equal version as "unchanged". Two clones that each replaced a column got the same version, and a cache used across them returned the other world's hash; `verify()` compared the cache with itself and would not notice.
```
let mut w1 = WorldTables::new(); w1.insert("a", vec![1, 2, 3]);
let mut cache = MerkleCache::new(Plane::A); cache.update_world(&w1);
let mut w2 = w1.clone();
w1.insert("a", vec![10, 20, 30]);   // version 1
w2.insert("a", vec![7, 8, 9]);      // version 1
cache.update_world(&w1);            // == state_hash(&w1)
cache.update_world(&w2);            // returned w1's hash, not state_hash(&w2)
```
Fix: a process-wide identity per `WorldTables` instance, fresh at construction and at every clone; `update_world` re-hashes in full when the identity changes; no public path takes a caller-supplied version. Verified by the attacks in section 2.1 and by the mutants "cache ignores identity", "clone keeps identity" and "identity counter constant", each red. Identity and versions never enter a hash or an equality (tested by the Developer; my golden and stub reproductions agree).

### DEF-2 (low; fixed in `3970a67`): `mark_rows` with a huge range hung
It extended a `BTreeSet` with every chunk index of the range; `0..usize::MAX` did not return in 20 s with memory growing under a 4 GB limit. Fix: the range is clamped to the cached row count. Verified in section 2.1; the mutant with the clamp removed is red.

## 4. Contract gaps

| Gap | Status | Basis |
|---|---|---|
| GAP-1 (medium): a float reached plane A through `rusqlite` with every gate green | **fixed** (`99c8c0b`) | Rule `float-dep` with `FLOAT_YIELDING_APIS` (`Real`, `as_f64`, `as_f64_or_null`) and clippy disallowed methods; my three snippets and six evasion attempts are refused (section 2.4). Arrow's float types come with T-02. |
| GAP-2 (low): the fp-contract guard scanned only variables named `*CFLAGS*` | **fixed** (`5c1aa48`) | `CXXFLAGS`, `CC`, `CXX` and their `HOST_`, `TARGET_` and `_<target>` forms are flagged. Add zstd-sys to `C_BUILD_CRATES` when T-02 brings it. |
| GAP-3 (low-medium): no integrity check beyond `payload_hash` | **fixed** (`46b7291`) | Row digest and chain per timeline (journal v2); independent reproduction and the 22 tamper cases in section 2.2. One accepted limit: a forged head after tail truncation needs an external anchor, which round 2 checks in the keyframe; `867bf95` now says so in the module documentation. |
| GAP-4 (informational): guard and hash covered `WorldTables` only | **fixed** (`16bef5b`) | In round 2 the guard hashes the engine's snapshot (section 8). Every read-only entry point must still call `read_only` by convention; nothing enforces it. |
| GAP-5 (low, test coverage): the suites did not pin some of what the code does | **fixed** (`867bf95`) | On `2bb679d`, mutants that drop `job_id`, `weights_hash` or `prompt_hash` from the row digest, and one whose chain does not mix in the previous link (the head would then commit only to the last row), left every test green; the shipped code was right, as my tamper set showed. On `867bf95` those four and the `model_id` and `provider_version` twins are all red. |

## 5. Observations
- OBS-1 (closed): `Replay::expect` reported an extra live input before the journal's next input as `Mismatched`; it is now `Extra`.
- OBS-2: `put_u32` and `put_bytes` saturate at `u32::MAX`, so a row, name or field over 4 GiB would collide; the Developer documents it and no engine value approaches it.
- OBS-3: the C build is `-O0` in debug and optimised in release, with the same flag; SQLite's floating point never reaches plane A (STRICT, no `REAL`).
- OBS-4: `CPPFLAGS`, `CPP` and `LDFLAGS` are not scanned by the fp-contract guard; the cc crate reads none of them.

## 6. Not run (round 1, now superseded by section 12)
- T-02: keyframe write, load, advance 10 years, pending timers, corruption and truncation refusal, the keyframe's journal-position anchor, V-DET-05 cadence and the yearly cache.
- Windows and macOS ARM (nightly level 2).
- Throughput at 1M rows.
- A power-loss test of `synchronous = FULL` (SIGKILL cannot lose data the OS already holds).
- The first-pass probes were not all repeated on `766bdb9` and `867bf95`: there I re-ran the DEF-1 and DEF-2 attacks, the golden reproductions, the GAP-5 mutants and `check_all.sh`. Those commits change only `MerkleCache::update`'s signature, one journal doc comment and tests.

# Round 2: T-02 keyframes (`2ef968e`, re-run on `ee7d6ff`)

Round 2 follows the Architect's list. The brief for it named the cut points, the thread and process variations, the tamper cases, the journal anchor, the event-hash question from the SPR-008 Tester, and my own implementation of the eight engine tables. The probes and their harness are in the scratchpad; I read the Rust encoder (`state.rs`) before writing my own, so my implementation is independent in code and in method, not in having seen the layout.

## 7. Round-2 verdicts and results

### 7.1 V-DET-05 and restore equivalence

| Probe | Result | Verdict |
|---|---|---|
| 47 cut points of the life world (seed 13, 48 rows, four queued material inputs): minutes 0, 1, 3 days, 45 days + 2, +3, +4 (around a queued input at minute 3 of day 45), 400 days + 3, 16 random minutes within 3 years, and 8 instants each at −1, 0, +1 minute. Each: write a keyframe, load, advance 10 years, compare every instant's (minute, `state_hash`, drained events) with the cut engine run on | all equal; the restored hash equals the cut engine's at each cut | PASS |
| The loaded engine at 1, 2, 4 and 8 threads, with four further inputs submitted after the load: at the cursor, cursor + 1, cursor + 500 and day 3000 (the continuous run gets the same) | all equal | PASS |
| Chained keyframes: write and reload every 400 days over 6 years; concatenated ladders against one continuous run | equal | PASS |
| A keyframe written by a debug and by a release process, loaded by each at 1 and 4 threads, advanced 6 years in a separate process | all 8 combinations give identical output (written hashes, folded ladders, instant counts) | PASS |
| `KeyframeStore` against my own model: 300 rounds × 120 random admit, touch, holds and `nearest_at_or_before` operations, cadence 1 to 12, capacity 0 to 5, negative years | equal at every step | PASS |
| Mutants of the store (capacity off by one, cadence, touch not refreshing) | red | PASS |

### 7.2 Engine-table encoding

| Probe | Result | Verdict |
|---|---|---|
| My implementation of the eight tables (`rh-core/agenda.meta`, `agenda.timers`, `engine.commands`, `engine.epochs`, `engine.inputs`, `engine.meta`, `events.held`, `events.meta`) and the plane hash, fed by field dumps of `Engine::snapshot()`: 11 states of the life world (cuts from minute 0 to year 6, including day 45 minute 2 and 3 and state with 124 held events and 36 epochs) | `state_hash` equal on 11 of 11 | PASS |
| Two hand-built states: three queued commands (with and without both kinds of cause, `i64::MIN` and `i64::MAX` arguments, an empty kind, a negative tick), a tombstoned timer; a `None` cursor with `u64::MAX` counters | equal on 2 of 2 | PASS |
| On `ee7d6ff` (held events outside the hash): the same 13 states with `events.held` left out, and the 120 monthly engine states of the SPR-005 stub world against the committed `stub_world_hashes.txt` | 13 of 13 and 120 of 120 | PASS |
| `rh_snapshot_hash_v1.txt` against my plane implementation | 553 of 553 lines (the synthetic `engine` lines of the new golden are covered by my own states above) | PASS |

### 7.3 Event hash (the SPR-008 Tester's question)

With every event drained, equal tables and a one-year run, each of the following changes `state_hash` through `rh-core/events.meta`: two events in swapped order, the same events one month later, a subject added, a different kind, one event fewer, a `caused_by` edge, a payload of another length. Draining or not does not change the running hash. So a perturbation within a year that leaves the tables equal is still seen.

### 7.4 Tampering, structure and forgery

| Probe | Result | Verdict |
|---|---|---|
| Every byte of an 11,858-byte keyframe flipped, bits 0 and 7 (23,716 files) | refused, no panic | PASS |
| Every truncation length; 1, 7 and 4,096 trailing bytes | refused, no panic | PASS |
| Forged envelope: single bits of the Arrow body flipped, the envelope digest recomputed (my envelope matches the Developer's byte for byte). `2ef968e`: 7,890 files; `ee7d6ff`: 3,946 files (every 6th byte, bits 0 and 7), each loaded in a subprocess | refused 4,220 and 1,986; equal state (padding) 2,910 and 1,603; **panic (exit 101) 719 and 338; SIGABRT 41 and 19**; accepted with a different state 0 and 0 | **DEF-3** |
| Valid envelope, consistent structure: table rows duplicated, rows reversed, two rows of a table swapped, an unknown extra table, a nullable schema, an extra `Float64` column, the table column as `Int64`, zero batches, the header's `format` key renamed, `hash_layout` changed, `world_seed` changed, `state_hash` zeroed, a world row altered with the hash not fixed | each refused with its own message, except the nullable schema with the same data, which loads | PASS |
| `journal_inputs` in the header changed to 999 (envelope re-sealed) | loads: the anchor is outside `state_hash` | **GAP-7** |
| Carried events (`ee7d6ff`): oldest or all held events dropped with the header count lowered and the envelope re-sealed | loads, `state_hash` unchanged (they are outside the hash by design) | OBS-6 |

### 7.5 History, anchor and restore

| Probe | Result | Verdict |
|---|---|---|
| History tuple: each of build, registry version, manifest hash, world seed, branch salt differing at load | refused naming the field, five of five | PASS (but see GAP-8: only the seed is pinned by a test) |
| `write_keyframe` with a history that is not the engine's (seed 99, salt 5 for an engine with seed 13, salt 0), loaded under the same wrong history, then advanced 3 years | written and loaded without complaint; restored hash equals the original; ladders diverge at instant 9 of 1,128 | **DEF-4** |
| Journal anchor: a real 5-input journal, a keyframe with its true position, then the journal truncated with a forged count and head | `Replay::open` succeeds with position (4, head₃); the keyframe records (5, head₅); no function compares them; `load_engine` takes no journal; a keyframe with anchor (99, `0x07…`) over an engine that applied 1 input loads | **GAP-7** |
| `Engine::restore` with 19 altered states | refused at restore: queued input before the cursor, held id at or past the counter, reversed held events, `events.last` behind a held event, reversed or duplicated epochs, a tombstone naming no timer, `agenda.next_seq` below an entry, `None` cursor with pending inputs, a missing world column (10); accepted and then rejected by the engine at runtime: cursor moved back before the held events, a world column of the wrong length (a caught panic in the owning system), a value out of domain (3); accepted: four counters with no check available (`events.running`, `commands.next_seq`, `applied_inputs`, the timeline id); **a pending timer dated before the cursor and one far in the future** | GAP-9 for the past-dated timer (the runtime no-back-dating invariant refuses it when it fires) |

### 7.6 Drain independence (GAP-6)

On `2ef968e`: two identical 2-year runs, one never draining events and one draining after every instant, hashed differently at 782 of 782 instants. On `ee7d6ff`, where the held events are carried in a keyframe and left out of `state_hash`: 0 of 782 differ, and a keyframe still restores the held events. Deleting `rh-core/events.held` from `CARRIED_TABLES` turns four tests red (golden, stub ladder, and the two new keyframe tests).

## 8. Gates and the guard in round 2

| Probe | Result | Verdict |
|---|---|---|
| Guard on engine-held state (GAP-4): a "read-only" call that submits an input, advances the engine, or drains events, behind a `RefCell` | on `2ef968e` all three trip the guard in debug; on `ee7d6ff` the first two trip and the drain does not (the held events left the hash, OBS-8); an honest `snapshot()` passes; in release the guard is compiled out | PASS (GAP-4 closed) |
| `lint_arch` on the Arrow entries: `lz4` feature on `arrow-ipc`, default features on `arrow-array`, `ffi` on `arrow-array`, `arrow-schema` unpinned, `arrow-ipc` in rh-core, an unlisted `arrow-cast`; and `arrow-ipc` without `zstd` | the first six are violations with their own messages; the last is a permitted subset | PASS |
| Arrow float tokens: `Float64Array`, `Float64Type` through an alias, `DataType::Float32`, a spaced `arrow_schema::DataType :: Float16`, `typed_data::<f32>()` | refused; `is_floating()` and `as_primitive::<Int64Type>()` allowed | PASS |
| C builds: every real compiler line of `zstd-sys` and bundled SQLite through a logging compiler | 32 of 32 carry `-ffp-contract=off`; the 4 invocations without it are `-E` and `-?` probes. `c_fp_contract_check` finds both crates' build output | PASS |
| Delete each of the four new `check_all.sh` gate lines (float-dep Arrow control, `rh-snapshot` suites, V-DET-05, the C check) | the inventory test is red each time | PASS |
| T-02 implementation mutants against the Developer's suites: 46 (every field of every engine table, the envelope check, the stored-hash check, the five history fields, the poisoned refusal, the timeline check, three store mutants) | 41 red; 5 survive: the four untested history fields (GAP-8) and the redundant restored-state hash check (equivalent, since `restore(x).snapshot() == x`) | PASS with GAP-8 |

## 9. Defects (open)

### DEF-3 (medium): a keyframe with a valid envelope and a corrupt Arrow body panics or aborts the loader
The envelope digest is an XXH3-128 of the Arrow bytes, so a body written wrongly with a correct digest passes it, and Arrow 60's reader panics (`arrow-buffer-60.0.0/src/buffer/immutable.rs:299`) or requests an absurd allocation ("memory allocation of 2240417436949248509 bytes failed", an abort that `catch_unwind` cannot catch). The envelope exists for this, but it only stops accidental corruption: all 23,716 single-bit flips of a whole file and every truncation are refused cleanly. The state-hash check does catch every semantic edit (0 of 11,836 forged files loaded with a different state).

Reproduction (`forge.py`): write a day-50 life-world keyframe, flip one bit of the Arrow body at byte 387 (bit 0) or byte 384 (bit 7), recompute the 16-byte digest after the 8-byte magic, call `load_engine`. Recommendation: treat the Arrow body as untrusted: bound the declared buffer and decompressed sizes by the file length before `FileReader` sees them, or decode the fixed two-column schema with a small reader; wrap the read in `catch_unwind` as a second line, and add a regression test that re-seals corrupted bodies. Confidence 85% on the finding, 70% on the fix choice.

### DEF-4 (medium): `write_keyframe` trusts the caller's history tuple, and the engine cannot be asked for its own
`write_keyframe(&engine, &History { world_seed: 99, branch_salt: 5, .. }, position)` succeeds for an engine with seed 13 and salt 0, and `load_engine` accepts the file under the same history. `state_hash` does not cover seed or salt, so the restored state hashes equal; the restored engine, built with seed 99 and salt 5, diverges from the original at the 9th instant after the load. `Engine` has no accessor for its seed, salt or provenance, so the writer cannot check them.

Reproduction: `k11_writer_trusts_the_callers_history` in the scratchpad copy. Recommendation: add `Engine::world_seed()`, `branch_salt()` and `provenance()`, and take the tuple from the engine or refuse a mismatch in `write_keyframe`. Confidence 90%.

## 10. Contract gaps (round 2)

| Gap | Status | Basis |
|---|---|---|
| GAP-6 (medium): `state_hash` depended on when the host called `drain_events` | **fixed** (`ee7d6ff`) | Held events are a carried table: in the keyframe, outside the hash; `events.meta` (counter and running hash) still commits all history, and I checked that it is order, tick, subject, kind, cause, payload and count sensitive and drain independent (7.3, 7.6). |
| GAP-7 (medium): the keyframe's journal anchor has no verifier, no test, and is outside `state_hash` | **open** | Section 7.5. Recommendation: `Replay::check_anchor(&JournalPosition)` requiring a count of at least `inputs` and `chain_after(inputs) == head`, a test that the truncation above fails it, and a ruling on whether `load_engine` should take it. Confidence 85%. |
| GAP-8 (low, test coverage): only `world_seed` of the five history fields is pinned by a test | **open** | The shipped code refuses all five; four mutants survive. Recommendation: one parameterised test. |
| GAP-9 (low): `Engine::restore` accepts an agenda entry dated before the cursor | **open** | Refused only when it fires. Recommendation: refuse `time < cursor` at restore, as for queued inputs. Confidence 75%. |

## 11. Observations (round 2)
- OBS-5 (from round 1, now closed by the Architect's decision to document it): a table truncated and regrown to the same row count needs `mark_rows` for the regrown rows, or `MerkleCache::update` returns a stale root (3000 rows cached, `truncate(1500)`, 1500 new rows, no marks). 400 rounds of honest marking never went stale.
- OBS-6: after GAP-6's fix the carried events are covered by the envelope digest only. A re-sealed keyframe can drop them (the oldest, or all) and still load with an equal `state_hash`; pending output is lost, history is not. This follows from "output, not history" and costs nothing in determinism.
- OBS-8: since GAP-6's fix the unsynced guard no longer sees a read-only call that drains the held events (they are outside `state_hash`); the loss would be pending output, not history. Guarding the carried tables would need a separate hash.
- OBS-7: a keyframe with a nullable Arrow schema and the same data loads; the reader checks the column types, not their nullability.

## 12. Not run (rounds 1 and 2)
- Windows and macOS ARM (nightly level 2); throughput at 1M rows and keyframe sizes at scale.
- A power-loss test of `synchronous = FULL` (SIGKILL cannot lose data the OS already holds).
- The first-pass round-1 probes were not all repeated on the round-2 heads: there I re-ran the DEF-1 and DEF-2 attacks on `867bf95`, and in round 2 the guard, golden, ladder and gate probes on `2ef968e` and `ee7d6ff`.
- The synthetic `engine` lines of the new golden file (their state is defined only in `bin/gen_hash_golden.py`); my own states cover the same encodings.
- DEF-3's forged files were run at a stride (every 3rd byte on `2ef968e`, every 6th on `ee7d6ff`), not exhaustively.
