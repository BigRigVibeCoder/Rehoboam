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
version: 1.0.0
---

> **BLUF (round 1: T-01 hashes, T-03 journal, T-04 guard; T-02 keyframes comes in round 2 after SPR-006):** The plane hashes are correct against a layout implementation I wrote from the `05.1` §6 item 6 text alone, on the C xxHash library: it reproduces all 553 golden lines and all 120 monthly hashes of the SPR-005 stub world (the declared C3 regeneration), and the results are identical in seven builds. The journal survives 60 kill -9 cycles over two timelines, and my own implementation of the row digest and chain agrees with the Developer's file. I found **two defects, both fixed and re-verified** (DEF-1: the Merkle cache returned another world's hash for a clone; DEF-2: `mark_rows` hung on a huge range) and **five gaps: GAP-1, GAP-2, GAP-3 and GAP-5 fixed and re-verified, GAP-4 open until T-02**. `bin/check_all.sh` is green on the final head. No open defect.

Tested commits: first pass `bb82bceab347fa16ca90257738c2652b52d7c495` (PR #25, stacked on SPR-005 `fc635dc`); re-test on the rebased head `2bb679d817470baf711bf0bc2bc8712c87aedfb8` (onto main `b0cf3e2` after SPR-005 merged) on `766bdb901dfa997541852c2e7d79711a37e3c351` (`MerkleCache::update` free of caller-supplied versions), and on the final head `867bf954df2d2876f0ca06aa69488e48def5145f` (GAP-5 tests). Probes ran on `git archive` copies with a fresh `CARGO_TARGET_DIR` per copy; the repository's working tree was only read. The probe plan was written from the contracts (SPR-007 v1.2.0 with its six rulings, ADR-0006, ADR-0024, ADR-0007, `05.1` §6 items 4, 6, 7, 8, `05.9` IF-2, `09` V-DET-05, V-DET-10, V-DET-11) before the branch was opened; the Architect's later rulings on layout documentation, forced CFLAGS and transitive crates were applied. Reference code: `hashref.py` (the layout, from the text, with XXH3 from python-xxhash 4.0.1 on xxHash 0.8.3), `check_golden.py`, `check_stub.py`, `jv2.py` (row digest and chain, plus 22 tamper cases), `fptable.sh`, `crash.py` and `crash2.py`, and Rust probes added to throwaway copies. They are in the session scratchpad `ver/` directory. Tester model: set by the Architect in GOV-005 §4.2; not recorded here by instruction.

## 1. Verdicts (round 1)

| Task | Verdict | Basis |
|---|---|---|
| T-01 plane hashes | **PASS** (DEF-1, DEF-2 fixed) | Independent reference reproduces every golden line and the stub-world ladder, before and after the rebase; one-row change alters exactly one chunk; dirty tracking equals full recomputation over 6,000 random edits; seven builds identical; the DEF-1 fix survives 400 random steps over cloned worlds. |
| T-03 journal | **PASS** (GAP-3 and GAP-5 fixed) | IF-2 fields round-trip including extremes; `payload_hash` equals the C library's XXH3-128; STRICT schema with no `REAL`; replay reads only the file; row digest and chain reproduced independently; 22 of 22 tamper cases detected (one accepted by design); 120 kill -9 rounds clean on v1 and v2. |
| T-04 unsynced guard | **PASS** | Guard trips on an injected write in debug, is compiled out in release, and both are in `check_all.sh`; the guard mutants are red. See GAP-4 for coverage until T-02. |
| Gates (`lint_arch`, `lint_forbidden`, fp-contract, inventory) | **PASS** (GAP-1, GAP-2 fixed) | 11 of 11 first-pass gate mutations red; the GAP-1 snippets and eight evasion attempts are refused; the GAP-2 variables are flagged; every deletion of a new gate line turns the inventory test red. |
| `bin/check_all.sh` | **PASS** | 152 of 152 on `bb82bce`; 160 of 160 on `2bb679d` and `766bdb9`; 160 of 160 on `867bf95` (clean archives, fresh target directories). |
| T-02 keyframes, V-DET-05 | **not run** | Waits for SPR-006 (ruling 6). |

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
| GAP-4 (informational): guard and hash cover `WorldTables` only | **open until T-02** | The agenda, queues, epochs, inputs and the event counter join as named tables in T-02; I will re-run the guard and ladder probes there. Until then every read-only entry point must call `read_only` by convention; nothing enforces it. |
| GAP-5 (low, test coverage): the suites did not pin some of what the code does | **fixed** (`867bf95`) | On `2bb679d`, mutants that drop `job_id`, `weights_hash` or `prompt_hash` from the row digest, and one whose chain does not mix in the previous link (the head would then commit only to the last row), left every test green; the shipped code was right, as my tamper set showed. On `867bf95` those four and the `model_id` and `provider_version` twins are all red. |

## 5. Observations
- OBS-1 (closed): `Replay::expect` reported an extra live input before the journal's next input as `Mismatched`; it is now `Extra`.
- OBS-2: `put_u32` and `put_bytes` saturate at `u32::MAX`, so a row, name or field over 4 GiB would collide; the Developer documents it and no engine value approaches it.
- OBS-3: the C build is `-O0` in debug and optimised in release, with the same flag; SQLite's floating point never reaches plane A (STRICT, no `REAL`).
- OBS-4: `CPPFLAGS`, `CPP` and `LDFLAGS` are not scanned by the fp-contract guard; the cc crate reads none of them.

## 6. Not run (round 1)
- T-02: keyframe write, load, advance 10 years, pending timers, corruption and truncation refusal, the keyframe's journal-position anchor, V-DET-05 cadence and the yearly cache.
- Windows and macOS ARM (nightly level 2).
- Throughput at 1M rows.
- A power-loss test of `synchronous = FULL` (SIGKILL cannot lose data the OS already holds).
- The first-pass probes were not all repeated on `766bdb9` and `867bf95`: there I re-ran the DEF-1 and DEF-2 attacks, the golden reproductions, the GAP-5 mutants and `check_all.sh`. Those commits change only `MerkleCache::update`'s signature, one journal doc comment and tests.
