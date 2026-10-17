---
id: VER-SPR-007
title: "Audit of SPR-007"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-007, VER-SPR-007-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** PASS. SPR-007 makes every state reproducible and checkable:
> - plane hashes are a versioned, domain-separated XXH3 Merkle layout, pinned by an independent Python reference. They cover all plane-A state, including what the engine holds outside its tables: the agenda, the command queues, the epochs, the queued inputs and the running event-log hash.
> - keyframes are Arrow IPC with zstd inside an integrity envelope, and a load that reaches the 10-year mark equals a continuous run (V-DET-05).
> - the input journal is SQLite with a per-row digest and a chain hash, and replay hard-fails on any mismatch.
> - an unsynced guard proves that read-only APIs change nothing (V-DET-10).
>
> The Tester's round 1 found 2 defects and 5 gaps, among them a float that could reach plane A through a listed crate without naming a float type. Round 2 tested the keyframes and found:
> - 4 more defects, one of them high (DEF-5: keyframes of any world above about 100 rows could not be loaded);
> - 6 gaps, one of them my own ruling (GAP-6: undrained events in the hash) and one the journal position itself (GAP-10).
>
> My review of the DEF-3 fix found one more (DEF-3b). All are fixed. Change class C3. The Architect merges (GOV-005 §6).

# VER-SPR-007 — Audit of SPR-007

Commit SHAs in this record are as reported during the sprint and in `VER-SPR-007-TEST`. The branch was rebased onto `main` twice (`3f42e4d`, then `bd10d90` for the merge) and is rebase-merged, so the SHAs on `main` differ. Each commit keeps its subject line, so `git log --grep` finds it.

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` | PASS (166 checks) | Developer at every fix commit; Tester at `8755af4` and `5103b13` (clean archives); Architect at `2ef968e`, `10b0f4e` (its own DEF-3b check) and the audit commit |
| CI on GitHub-hosted runners | PASS at `2ef968e` (run 38903972844) at `8755af4` (run 38912354485) and at `5103b13` (run 38918429148) and at `ff87a0d` (run 38924124132) | PR #25 |
| Change class | C3: defines the plane hashes, keyframes and journal; the stub golden was regenerated twice as declared C3 changes, each cross-checked independently | — |

## 2. Tester verdicts
| Round | Head | Result |
|---|---|---|
| 1 | `bb82bce` → `867bf95` (rebased onto main `b0cf3e2`; report `31567d9`) | T-01, T-03 and T-04 **PASS**. The Tester's independent XXH3 Merkle implementation, written from the `05.1` text, reproduces all 553 layout lines and all 120 stub-world hashes. Journal: 60 kill-9 rounds with integrity intact; 22 tamper cases detected; STRICT integer-only storage. Results are identical across 7 build profiles. DEF-1 (the Merkle cache trusted per-instance versions) and DEF-2 (`mark_rows` hung on a huge range) are fixed. GAP-1 (a float reaching plane A through a listed crate), GAP-2, GAP-3 (the journal chain) and GAP-5 (test coverage) are fixed |
| 2 | `2ef968e`/`5eebedb` → `ee7d6ff` (report `2dd9fcf`) | T-02 keyframes: V-DET-05 at 47 cuts; the Tester's encoder reproduces `state_hash` on 13 states and the 120-month ladder. **DEF-3** (a forged Arrow body crashed the loader: 338 panics and 19 aborts in 3,946 files) and **DEF-4** (a caller-supplied history) found. GAP-6 (the drain schedule changed the hash) is fixed in `ee7d6ff`: 0 of 782 instants differ. GAP-7 to GAP-9 opened |
| 2b, 2c | `8755af4` and `5103b13` (reports `71a90d0`, `76e474f`) | **PASS, no defect open.** Forge corpus: 103,787 files on `8755af4` and 53,534 on `5103b13` across three keyframes (up to 20,000 rows, 1.3 MB), in 1-bit, 2-bit, byte-replacement and field-level classes, each loaded in a child process. Panics, aborts and other-state loads are 0 in every class; on `ee7d6ff` the field class gave 909 panics and 107 aborts. **DEF-5**, found here: a dense sweep of 24 seeds × 3 years with a keyframe at every instant (29,926 keyframes, 2,426 with out-of-order tombstones) gives 0 errors after the fix; 33 world sizes up to 20,000 rows give 0 errors. Every history field is refused when it differs. `check_anchor` refuses every forgery tried. **GAP-10**, found here: the journal anchor could accept a forged applied input; after the fix, 40 random out-of-order journals gave 160 resumes and 0 differences. Through `resume`: 7 targeted cuts of a fixed schedule are equal, while re-submitting from the applied count diverges or is refused at every cut. A crash with two queued inputs replays only with exactly its 4 journal rows. A multi-entity input submitted outside the journal makes the run unresumable, which supports the one-input-per-row rule. Mutants: 58 on `8755af4`, 52 red; 8 on `5103b13`, 6 red. All survivors are equivalent, redundant layers, or GAP-11 |

## 3. Rulings (Architect)
| # | Question | Ruling |
|---|---|---|
| Pre-sprint 1–6 | The hash, keyframes, the journal, C flags, the unsynced guard, order | SPR-007 v1.2.0 |
| 10:12 | Domain separation; forced `CFLAGS`; transitive crates | Separate tags for chunk, table and plane; the plane letter and the table name and row count go into the hash. `CFLAGS` with `force = true`. `PLANE_A_EXTERNAL_DEPS` covers direct dependencies, with review notes |
| Round 1 | The dev cycle; DEF-1; GAP-1 | No rh-core dev-dependency on rh-snapshot. A per-instance `WorldTables` identity, and no public path that takes a caller-supplied version. Float-yielding API tokens per dependency, enforced by `lint_forbidden` (rule `float-dep`) |
| T-02 | The keyframe plan; four deviations | Undrained events are kept. Crash artifact format 2 (`journal_head`, `last_keyframe`). `Scope` maps onto `EntityKind`. `KeyframeStore` is a 10-year cadence plus a yearly LRU of 8. Engine tables carry the `rh-core/` prefix (plus `engine.meta` and `events.held`). A reserved header table and one canonical-bytes schema. An XXH3-128 envelope ahead of Arrow (Arrow 60 panics on some corrupt input) |
| Event hash | Transient divergences | The running event-log hash is in `events.meta`, so a divergence that emitted different events shows at the next rung even when the tables reconverge |
| Round 2 | GAP-6 | My T-02 ruling 1 was wrong on the hash. Undrained events are carried in keyframes but stay outside `state_hash`; corrected in the decisions log |
| Round 2 | DEF-3 | ADR-0007 keeps Arrow IPC with zstd, inside four layers: a metadata guard ahead of Arrow, `catch_unwind` (later around the guard too, after DEF-3b), read-back on write, and forge sweeps in the PR and nightly tiers. No planted positive control, because it would need a switch in production code; the pre-fix abort is the evidence |
| Round 2 | DEF-4, GAP-7 to GAP-9 | The history comes from the engine. `Replay::check_anchor` is the journal verifier, and `load_engine` stays journal-free. Restore refuses a timer before the cursor; a timer at the cursor in any phase is legal, because keyframes are taken between advances |
| Round 2 | GAP-10 | The journal position is the count of accepted inputs, for keyframes and crash artifacts. The host checks, journals, then submits. `rh_snapshot::resume` is the single host loop for replay from a keyframe. Non-material inputs are counted the same way when they arrive (BCK-002 A-33) |
| Audit | One row, one input | A journal row's scope is one entity, and a journaled run submits exactly `material_input(row)` per row. Added to `05.1` §6; the Tester showed that an input submitted outside the journal makes a run unresumable |

## 4. Defects and gaps: disposition
| Item | Disposition |
|---|---|
| DEF-1 the Merkle cache trusted per-instance column versions across clones | Fixed (`6409856`): an instance identity; full recompute on change |
| DEF-2 `mark_rows` over a huge range hung | Fixed (`3970a67`): clamped to the cached rows |
| GAP-1 a float through a listed crate, no float token named | Fixed (`99c8c0b`): `FLOAT_YIELDING_APIS` plus the `float-dep` rule plus clippy disallowed methods; the Tester's evasions are refused |
| GAP-2 the fp-contract scan missed CXXFLAGS, CC and CXX | Fixed (`5c1aa48`) |
| GAP-3 the journal had no integrity check beyond the payload | Fixed (`46b7291`): a row digest plus a chain; tail truncation is caught against the keyframe anchor |
| GAP-4 the guard and hash covered `WorldTables` only | Fixed in T-02 (`16bef5b`) |
| GAP-5 the digest fields and chain link were untested | Fixed (`867bf95`) |
| OBS-1 Extra reported as Mismatched | Fixed (`d8b71c4`) |
| OBS-5 shrink then regrow to the same count | A doc line: `update` sees only the count at call time |
| SPR-006 DEF-6, GAP-6 (carried) | Fixed in T-02 (`1abe93f`): the artifact cap test; canonical restore everywhere |
| GAP-6 undrained events in `state_hash` made the hash depend on the host's drain schedule (the Architect's T-02 ruling 1) | Fixed (`ee7d6ff`): `rh-core/events.held` is carried in keyframes but outside `state_hash`. The ruling is corrected in the decisions log. The Tester found 0 of 782 instants differing between draining and non-draining runs |
| DEF-3 a valid envelope around a corrupt Arrow body crashed the loader (338 panics and 19 aborts in 3,946 forged files) | Fixed (`b9c3ede`), in four layers. (1) `ipc_guard` checks the IPC metadata before decoding: the exact schema, every block and buffer inside the body, and capped uncompressed sizes. (2) `catch_unwind` around the decode. (3) Every write is read back. (4) A forge sweep: every bit of the body, 93,648 cases, gives 0 panics and 0 aborts (nightly), plus a strided PR tier |
| DEF-3b (found in the Architect's review of `b9c3ede`) the guard itself panicked, outside `catch_unwind`, on a block with a negative `bodyLength` that is a multiple of 8. A single bit flip cannot produce that value, so neither sweep reached it | Fixed (`10b0f4e`): signs are checked before alignment, `split_at_checked` is used, and the guard runs inside the `catch_unwind` too. A PR-tier test forges 56 located fields (every footer block's offset and lengths; the first batch's length, node lengths and null counts, and buffer offsets and lengths) to 10 boundary values: 557 cases, 0 panics (`8755af4`; the old guard: 39 panics on 51 fields). The Tester's field sweep found the same panics and nothing else |
| DEF-4 `write_keyframe` stamped whatever history the caller passed | Fixed (`24d2262`): `Engine::world_seed()`, `branch_salt()` and `provenance()`; `write_keyframe` takes no history, so a mismatched one cannot be written |
| GAP-7 the journal anchor had no verifier | Fixed (`24d2262`): `Replay::check_anchor` (count and chain head; truncate-and-reforge is Missing, regrown is Mismatched). A keyframe whose journal count differs from the engine's journal position (the accepted count since GAP-10) is refused on write and on load. The first host to replay from a keyframe (SPR-008 bisect) must call it |
| GAP-8 four of the five history fields were unpinned | Fixed (`24d2262`): one test over all five fields, each refused under its own name |
| GAP-9 `Engine::restore` accepted a timer dated before the cursor | Fixed (`24d2262`): `RestoreError::TimerTime` before the cursor; a timer at the cursor still restores |
| DEF-5 (high; found independently by the Tester and by the DEF-3 read-back, layer 3) the decoder returned agenda tombstones in key order, not id order, so a keyframe whose cancelled timers were due out of insertion order could not be loaded. Every life world of 150 rows or more failed by day 100; the tests used 48 rows | Fixed (`24d2262`): sorted on decode, and `Agenda::restore` refuses a non-increasing set. A targeted test, plus a property test over row counts and seeds (`8755af4`: rows 48 to 5,000 × 3 seeds, loaded at five instants up to day 200, about 7 s in the PR tier; with the sort removed it fails). The live `state_hash` was never affected |
| GAP-10 (Tester, widened by the Architect) the journal position counted *applied* inputs. With inputs journaled out of time order, the anchor committed to an unapplied input and accepted a forged applied one. Even in order, replay from a keyframe would have re-submitted queued inputs, and the crash artifact's position (SPR-006) had the same flaw | Fixed (`5103b13`): the position is the count of *accepted* inputs (`Engine::journal_position()`; applied is derived). Keyframes and crash artifacts record it. A host checks, journals, then submits. `rh_snapshot::resume` is the host loop: it loads the keyframe, checks the anchor, and submits the rows after the position. Tests: the forged-journal repro; a resumed run equal to the continuous run for more than two years, with out-of-order queued inputs (the applied-count control diverges in 23 months); a crash replay with a queued input |
| GAP-11 (test coverage, low) the guard's uncompressed-size bounds and restore's "queued ≤ accepted" check were unpinned by any in-repo test; only the Tester's subprocess corpus killed the bound mutant | Fixed (`ff87a0d`, test-only): a PR-tier test forges every prefixed buffer's uncompressed length to five values over its bounds, and all 30 cases are refused as `Ipc`. The Architect checked that with the bounds switched off the test fails. A restore case now pins queued ≤ accepted |
| OBS-6, OBS-8 | Accepted. Carried events are covered by the envelope digest only, and the guard does not see a read-only drain. Both follow from the GAP-6 correction: what they leave uncovered is pending output, not history |

**Escapes found later:** none so far.

## 5. Acceptance criteria
| Task | Criterion | Met |
|---|---|---|
| T-01 | Hash stable across runs; a one-row change alters exactly one chunk | Yes (an independent implementation; 553 + 120 vectors; identical across 7 builds) |
| T-02 | V-DET-05: load, advance 10 years, equal to a continuous run | Yes (47 cut points; world sizes from 48 to 20,000 rows; a keyframe at every instant of 24 seeds × 3 years; and through the journal with `resume`, 160 resumes) |
| T-03 | Round-trip; a replay reads only the journal | Yes (journal copied alone; 22 tamper cases; 60 kill-9 rounds) |
| T-04 | V-DET-10: an injected write trips the guard | Yes (debug build; compiled out of release; a 3-year fuzz) |

## 6. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** |
| Merge route | The Architect merges (GOV-005 §6) |
| Carried forward | BCK-002 A-33 (count non-material inputs when they reach the engine). Typed keyframe batches when tables gain typed columns. LRU capacity measured in P1. Not run by the Tester: Windows and macOS ARM, performance at scale, a power-loss test, larger flips on the large keyframe, forgeries of the schema message, keyframes with queued lifecycle commands at an instant boundary |
| Escapes found later | — |
