---
id: VER-SPR-026-TEST
title: "Determinism and contracts tester evidence for SPR-026"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism, stream]
related: [SPR-026, REV-SPR-026, VER-SPR-013-TEST, AGT-003-DET, AGT-003-IF, ADR-0024, ADR-0030, ADR-0031, BCK-002]
created: 2026-10-18
updated: 2026-10-18
version: 1.2.0
---

> **BLUF: T-01 to T-06 and the A-58 items meet their acceptance criteria. I found no blocking defect, one test defect (DEF-1, a flaky V-STR-05 test under CPU load) and seven gaps, the largest being GAP-1: at full speed the D42 (b) window is not two sim-years.** `bin/check_all.sh` is **528 of 528** (both parts, one run) on a clean clone of `1d7f9a1`. My probes use my own codec, viewers and oracles, never the code under test, and agree with the implementation everywhere except the items below. Tested head: `1d7f9a1d5b628ae823ccefbc78fdc6ea208fbc02` on `sprint/SPR-026-stream-v1` (PR #82; base `772478f`). Tester model: Claude Sonnet 5.5 (the Developer's differs).

Method: a `git clone` of the SHA with its own `CARGO_TARGET_DIR`; the Developer's tree was never touched. The probe plan was written from `SPR-026`, `wire_v1.md` §0–§11, REV-SPR-026 R1–R24, `docs/09` and BCK-002 A-58 before I read any implementation. Oracles: my own stdlib wire codec (calibrated first against the golden file: all 26 golden frames reproduced byte for byte), my own raw-socket viewer (§6.3 from the prose; sealed reads through `rh-cli tail`), DuckDB readers of the sealed Parquet, the repository's independent canonical and XXH3 references, and `strace` for kill points. Scratch was `/dev/shm` (short paths, socket limit). Probe sources are in my scratchpad, not the repository. Where a probe failed on a first run, the cause was my own test code in every case (listed in §4) and I fixed the probe, not the expectation, only after checking the golden file or the spec.

## 1. Verdicts

| Task | Verdict | Basis |
|---|---|---|
| T-01 `rh-protocol` | **PASS** | Golden by my own codec; 86-case conversation fuzz against the live host; method table (§2.1) |
| T-02 `LiveEdge` | **PASS with GAP-1** | Seal race, copy race, window bounds, abort, resume (§2.2); the bound fails at full speed (GAP-1) |
| T-03 host, `A.sock` | **PASS with GAP-2, GAP-5** | Modes, lock, stale socket, 107/108 bytes, cap 64, read-only, poison, C16 (§2.3) |
| T-04 channel C live | **PASS** | 11 of 11 checks, 44,337 `sifted_through` polls clean (§2.4) |
| T-05 viewers, V-STR-01/03/05 | **PASS with DEF-1, GAP-6** | Sleeper, stalled, jumper, kill -9 + resume, invariance matrix (§2.5) |
| T-06 gates and docs | **PASS** | 528 of 528; new gates in exactly one part each; baselines untouched (§2.6) |
| A-58 (DEF-3, GAP-8, GAP-10, GAP-11, GAP-12, wording) | **PASS with GAP-3, GAP-4** | 370 of 370 kill points recovered; GAP-10 catches 77 of 88 plants (§2.7) |
| `--test-poison` and the hook guard | **PASS** | Five checks of the Architect's ruling (§2.3) |
| C16 per-class close rule | **PASS** | Closed iff `id` is null, over 86 cases (§2.1) |
| Flake hunt (scratch `/dev/shm` and `/tmp`, idle and loaded) | **DEF-1** | 2 failures in 12 runs, both under load (§3) |

## 2. Results

### 2.1 T-01 and the dispatcher (AGT-003-IF)

| Probe | Result |
|---|---|
| My codec against `golden/wire_v1.txt` | 24 control and 2 records frames reproduced byte for byte, canonical JSON included |
| Golden `dispatch` lines replayed on fresh connections to the real host | 13 of 15 byte-identical. The other 2 (`status`, `subscribe_future`) differ only because the golden assumes an empty host with a fixed history; their structure is right. 7 close and 8 stay open, as the Developer said |
| 86-case conversation fuzz (`p_codec.py`): wrong magic, versions 0/2/65535, types 0/3, flags 1/128, length 4095/4096/4097/1 MiB/1 MiB+1/u32 max, a RECORDS frame from a viewer, empty payload, BOM, batch, scalar, floats in seq/id/unknown params, negative and string and 2^64 seq, depth 4/5/2040/4090, 100,000 nested arrays (answered `invalid_frame` on the header alone), unknown envelope member, duplicate key, non-ASCII key and value, `\u` escape, id types, notification and response from a viewer, whitespace, pipelined, 1-byte writes, valid frame then garbage, truncation | 86 of 86. **Per class: the host closes after every answer whose `id` is null and keeps the connection open after `method_not_found`, `invalid_params` with the id and `not_subscribed`, in all 86.** A float is answered `invalid_params` with a null id and a close, which is what the golden `dispatch_subscribe_float` says |
| Method table: 60 act-like names (`player.*`, `timeline.*`, `oracle.*`, `stream.pace`, `control.*`, `journal.append`, …) and 11 near-misses (case, trailing space, `stream.`) | All `method_not_found`, one identical message, equal to the §4.4 sentence byte for byte |

A truncated frame (EOF mid-payload) gets no answer and the connection closes; the Developer listed this as a deviation and I agree it is the only possible behaviour.

### 2.2 T-02 `LiveEdge` and the live-edge rule (AGT-DET)

| Probe | Result |
|---|---|
| Black-box seal-race stress (`p_stress.py`): 6 pollers and 6 subscribers against a fast `solo`, every `stream.status`, `behind_live_edge`, `stream.behind` and frame checked for I1, monotone `s`/`w0`/`w1`, `tip = live.next_seq`, exact `first_seq + i` and the canonical prefix | **0 violations** in 85,840 status answers, 15,138 subscriptions, 8,224 `behind_live_edge` errors and 20,996 frames; the run's tree equals the headless run's |
| Window bound over the wire, seals keeping up | (b): peak 2 sim-years. (a): peak 1 |
| Policy (a) vs (b) at full speed, `solo` 1,500 years | **See GAP-1** |
| Abort (`--test-poison`) | §2.3 |
| Resume | A resumed host starts with an empty window at the first year not on disk (`future_seq` seen by every viewer); the tree equals the unbroken run (§2.5) |

I did not run the Rust-side interleaving enumerations of probe (b) and the seal race myself; they pass inside `check_all` (rows 269 and 270, including the `window-gap` control) and my black-box stress covers the same invariant on the real host.

### 2.3 T-03: host, read-only, poison (AGT-IF and AGT-DET)

| Probe | Result |
|---|---|
| Run directory `0700`, `A.sock` `0600`, `A.lock` present; no `C.sock`, `C.lock` or `control.sock` while only `run --socket` serves | yes |
| Second host on the same directory | refused, message names the lock; the first host keeps serving |
| `kill -9`, then a resume: stale socket removed, new `run_nonce` | yes |
| `run_nonce` over 12 host starts | 12 distinct, all about 1.79e18 (wall-clock nanoseconds), none pid-like |
| Cap | 64 connections served; the 65th gets `too_many_viewers` (`id` null, `limit` 64) and a close; after one closes, a new one is served; threads 67 at the cap, 3 after all close |
| Socket path 107 bytes / 108 bytes | accepted / refused naming the path, nothing written |
| Read-only: 60 act-like methods with the journal's row count and chain head sampled before and after | unchanged (1 row, same head); no `control.sock` |
| `--test-poison TICK` (poison at year 15, `solo`) | `stream.end{aborted}` with `next_seq` 183 equal to the records the viewer holds; year 14 sealed, year 15 not, no visible partial; only years ≤ 14 were announced by `stream.sealed`; the abort and the host's exit name the hook; a resume then completes and the tree equals the unbroken run |
| `--test-poison` with `--vitals` | refused: "plants its own observer and is refused with --vitals" |
| Flag absent, no observer | the observer is attached only when `args.poison` is set (source read; the allocation gates pass unchanged) |
| `check_test_hooks.py` | clean tree passes; `--test-pace-ms 1`, `--test-stall mid:5` and `--test-poison 5` planted in `regen_baseline.sh` each fail with the file and line |

### 2.4 T-04: channel C live (AGT-IF)

`hamlet`, 600 years, sifter beside the host. No `C.sock` until the sifter starts, then `C.sock` `0600` under its own `C.lock`. The viewer on `C.sock` from seq 0 reads to `stream.end{finished}`: 556 of 600 years announced empty (`last_seq` null, the empty-string digest `99aa06d3…`, `first_seq` equal to the next record's seq), 44 non-empty years whose digests equal my digest over the received bodies, 60 lines equal to `tail --canonical --channel C`, and equal to the batch-sifted channel C of an independent headless run. `sifted_through` never exceeded channel A's sealed values in 44,337 paired polls. Channel A's tree and ladder equal the headless run's; `twin_check.py` passes on the live-sifted directory.

### 2.5 T-05: V-STR-01, V-STR-03, V-STR-05 (AGT-DET)

| Probe | Result |
|---|---|
| V-STR-05 catch-up (`solo` 1,200 years, host paced): edge follower, sleeper asleep until 900 sim-years sealed, stalled, all on my viewer | Edge: seqs exactly `0..14525`, 1,200 of 1,200 digests equal the DuckDB `segment.canonical_digest`. Sleeper: 1 `stream.behind`, 5 `behind_live_edge`, 6 sealed reads through `tail --from-seq --to-seq`, the same seqs and 1,200 of 1,200 digests. Stalled, read during the host's linger: 250 buffered frames, then `stream.behind` (`next_seq` 251, `live_first_seq` 14525) |
| Jump to live from 7,854 records behind (full-speed host) | `unsubscribe.next_seq` equals the cursor after the in-flight frames; `first_seq` (9448) lies between the tips sampled before (9432) and after (9460) the answer; the first frame starts at `first_seq`; nothing duplicated |
| `tail --to-seq` (`solo` 200 years, 200 segments) | 1,000 ranges at every segment boundary (`f..l`, `l..l`, `f-1..l`, `f..l+1`, `l..l+1`): lines equal the full output's slice, the header printed exactly once. Edges: `0..0`, `from > to` (no lines), beyond the end, `to = 2^64-1`, `to` past the end |
| `kill -9` with a viewer attached throughout, then a resume (`p_kill.py`): 18 runs, 5 with a second kill of the resumed host | 18 of 18: the kill landed mid-seal (a `.partial` directory present) in 18; `future_seq` seen in 18; a new `run_nonce` in 18; resumes from the year-10 keyframe in 4, from genesis in the rest (seals lag, so no keyframe yet); the viewer ended `finished` with lines equal to the unbroken run's canonical lines; the tree equals the unbroken run's in 18 |
| V-STR-01 matrix: `solo` and `hamlet`, 300 years, monthly ladder, threads 1, 2, 4; edge, stalled, rewinding and garbage-sending viewers attached | 6 of 6: the whole tree (stream, keyframes, ladder, journal) equals the headless run's |
| `--socket` against headless, 6,000 years, 3 repeats each (never gated) | median wall **29.84 s** headless, **29.91 s** `--socket` with no viewer (+0.2%), **29.99 s** with two edge and two stalled viewers (+0.5%) |
| No file under `worlds/` changed against `main`; V-PERF-04 and V-PERF-11 pass unchanged | yes (`git diff 772478f..1d7f9a1 -- worlds` is empty; rows 426 to 448) |

### 2.6 T-06

528 of 528 on `1d7f9a1`, about 29 minutes on a 4-core machine. The new gates are present once each: rows 40–44 (lints and the hook guard with its control), 265–270, 375–377, 483–488, 493–495. I did not audit the prose of the doc updates.

### 2.7 BCK-002 A-58 (AGT-IF)

| Item | Probe | Result |
|---|---|---|
| DEF-3 | `strace -e inject=…:signal=KILL:when=N` at every mutating syscall of run-directory creation, N = 1 to 170 (headless) and 1 to 200 (`--socket`, stale socket and lock included) | 370 of 370 recovered: 102 accepted as a new run, 268 by `--resume genesis`; every final tree equals the unbroken run's; none refused |
| GAP-8 | monthly run killed, resumed with no `--ladder`; with `--ladder yearly`; with `--ladder monthly`; a yearly run resumed bare | takes the rung from `run.toml`; the conflicting flag is refused naming both rungs, and the refused resume wrote nothing; all results equal the unbroken run |
| GAP-12 | run.ladder truncated by 3 bytes, by 16 bytes, one byte flipped, emptied | all refused (rc 2, names the keyframe tick). 7 junk bytes appended after the last rung: accepted, the tail is discarded, tree equals the unbroken run, which is right. The refusal changes `journal.sqlite`, its WAL files and the lock (GAP-4) |
| Wording | `replay` of a directory with a running host, a running headless writer, and a killed run | "the run's writer is still running (it holds …/.writer.lock)" twice; "the run was killed and not resumed" for the killed one |
| GAP-10 | 88 one-value plants, one per column of every channel-C table (a DuckDB rewrite of the real files); the control rewrites 1,200 files with no edit | control: rc 0. 77 caught. 3 plants were no-ops (`NULL + 1` in an empty segment). **5 columns escape**: `record_kind.kind`, `.name`, `.class` (also missed by `check_segments.py` and `tail`) and `sift_match_event.clause`, `.ordinal` (also missed by `check_segments.py`) — GAP-3 |
| GAP-11 (watch) | 370 creation kills and 18 mid-run kills with immediate resumes | "another run is writing" never appeared |

### 2.8 Mutants and lint plants

| ID | Mutant (scratch copy, one edit) | Killed by |
|---|---|---|
| M5 | `n < live_first_seq` to `<=` (subscribe at `w0` refused) | stress; host_misc |
| M6 | `n > next_seq` to `>=` (`future_seq` at exactly the tip) | stress (a seq at or below a sampled tip answered `future_seq`); host_misc (after I tightened it, see §4) |
| M12 | `droppable` ignores the sealed bound (drop before the seal lands) | stress (`I1 broken`); host_misc |
| M17 | year-end bound `completed - 1` to `completed - 3` (window too long) | window-bound probe (peak 4 years); stress |

The clean binary survives all four probes. I built four of the planned 32 mutants; the rest of the plan's mutants were not run (§5).

Lint plants in an extracted copy: `std::os::unix::net as nn` in another `rh-cli` file, and a `UnixStream` in `rh-stream`, fail `lint_forbidden`; `arc-swap` in `rh-core` and `rh-protocol -> rh-core` fail `lint_arch`. **A socket type in `rh-golden`, `rh-protocol` or `rh-sifter`, and `std::process::Command` in `rh-cli`, pass both lints (GAP-2).**

## 3. Defects

### DEF-1 (low for the product, must fix): `v_str_05_a_viewer_survives_a_kill_and_resume_with_a_seal_in_flight` fails under CPU load

Contract: SPR-026 T-05 and the rule that a probe asserts its path happened; AGT-003-DET §3 (a flaky test is a leak, never quarantined).
Repro: `RH_TEST_SCRATCH=<dir> cargo nextest run -p rh-cli --locked --release --test serve --test channel_c --test run_dir --test vstr --no-fail-fast`, three rounds of four conditions (scratch `/dev/shm` and `/tmp`, idle and with four busy loops on four cores).

| Condition | Runs | Failures |
|---|---|---|
| idle, `/dev/shm` and `/tmp` | 6 | 0 |
| loaded, `/dev/shm` | 3 | 1 |
| loaded, `/tmp` | 3 | 1 |

Both failures: `crates/rh-cli/tests/vstr.rs:478: no re-emitted year verified against its stream.sealed`; every assertion before it passed (no `Catchup` error, `Finished`, reconnects, new nonce, `future_seq`, retry). Cause, from the code: the viewer verifies a year only if it received the whole year live (`support/viewer.rs` `verify`) and was subscribed when the resumed host announced it. The test kills after a fixed 300 ms sleep and relies on a 5 ms-a-stop resumed host to wait for the viewer; under starvation the viewer reconnects after the resumed host has re-emitted and sealed the lost years, so it never hears their `stream.sealed`. The product behaved correctly (my own kill probe, which checks the digest chain differently, passed 18 of 18); the test's precondition is lost. Fix: make the resumed host wait for the viewer (hold at the first re-emitted instant until a viewer is connected, as `--test-stall` does) or kill after the viewer's cursor is observed at the edge, not after a sleep. CI runs the two parts as parallel jobs, so this can fire there.

## 4. Gaps (none is a defect)

| # | Gap | Recommendation | Confidence |
|---|---|---|---|
| GAP-1 | **The D42 (b) bound is not two sim-years at full speed.** `solo`, 1,500 years, `--seal-log`: 1,499 of 1,500 seals landed later than their sim-year (seal latency p50 5.78 s against a sim-year of about 9 ms), so the window held the whole run (peak `LiveEdge::bytes()` 1.84 MB). The host's peak memory (`VmHWM`) rose from 16.0 MB headless to 25.0 MB, about 5 times the counted window, because the seal queue holds structured records that `bytes()` does not count. Policy (a) on the same run: 19.3 MB, window 2.5 KB, 72 of 1,500 seals slower than their year. Wall time was the same in all three (13.6, 14.1, 13.5 s). The spec accepts the overrun case and logs it (§6.1), and the Developer measured 163–188 unsealed years. But on the 10k world at MAX the window then grows with the run length, and V-PERF-09's term "live bytes ≤ `bytes()` + V × 1 MiB + allowance" is likely to fail by this factor | SPR-030 should measure with the seal queue included (count it in `bytes()` or add a term), and the Architect should decide whether the seal thread needs a bound or a queue-depth warning | 75% |
| GAP-2 | The `network` lint does not cover `rh-golden` (which runs the tick loop), `rh-protocol` or `rh-sifter`, and `std::process::Command` is allowed in `rh-cli` outside `serve.rs`. A socket write could be added on the tick path without a lint failing (the MAJ-2 (c) concern of REV-SPR-026). Probe (b) would still catch a blocking write | Extend `NETWORK_FILES` to the tool crates that run the engine or sift, or state that probe (b) is the only guard | 65% |
| GAP-3 | `twin_check.py` does not compare `record_kind` (3 columns) or `sift_match_event.clause` and `.ordinal`; `check_segments.py` and `tail` miss `record_kind` | Compare the dictionary tables with the DDL's seed rows, and the event rows' clause and ordinal with the twin | 70% |
| GAP-4 | A refused torn-rung resume (GAP-12) first opens the journal, so SQLite folds the WAL and `journal.sqlite*` and `.writer.lock` change. The conflicting-`--ladder` refusal writes nothing, as the message says. Idempotent and harmless to history | Check the keyframe's rung before opening the journal, or state the exception | 55% |
| GAP-5 | A viewer that sends part of a frame header and stops holds its serving thread and one of the 64 slots until it closes (no read timeout). A hostile local user could fill the cap; the socket is `0600` | A per-frame read deadline, or document the local-trust assumption | 50% |
| GAP-6 | Vacuity: below about 600 sim-years a `solo` stream (about 2,400 records for 200 years) fits in the kernel socket buffers, so no sleeper or stalled viewer can fall behind. The Developer's tests use 600 years; I used 1,200. Also, "the stalled viewer's first read after the run is `stream.behind`" holds only during the host's 10 s linger; after it, the connection is closed with no notice | State the minimum run length in `wire_v1.md` §9 and the linger in the V-STR-05 row | 60% |
| GAP-7 | `--test-pace-ms` is per step, not per year (3 ms gave about 90 ms a sim-year), which makes pace figures in test names misleading | Rename or document | 40% |

My own test mistakes, fixed before reporting (none is a finding): three golden-contradicting float expectations (the golden says a float is answered with a null `id` and a close); a nesting test of 8,000 bytes that the 4 KiB cap refused first; a fixed 50-year run below the world's input horizon; a `--seal-log` pipe I did not drain; a `/dev/shm` I filled with my own run directories (`ENOSPC`); a "clean" binary that I had overwritten with a mutant build (rebuilt and re-run, §2.8); an end-of-run socket removal that my stress threads counted as a violation; and a jump test that was not far enough behind to mean anything (redone with a full-speed host).

## 5. Not done

- 28 of the 32 planned mutants (frame codec, `Catchup`, notification order, nonce, modes, lock, cap, abort, resume window, `--to-seq`) were not built; their areas are covered by the probes above and by the Developer's `check_all` controls.
- The window bound under a planted late seal (k = 0.5 to 4) and the Rust interleaving enumeration were not reproduced; I relied on `check_all` rows 269 and 270.
- No `loom` model (not required). No V-DET-01/02 sweep with 12 repeats at 8 threads; I ran threads 1, 2 and 4 once each on 300 years.
- The 300-cycle kill and resume soak for GAP-11 was replaced by 388 kills (370 creation, 18 mid-run).
- The Python stdlib viewer built on the reference's `Viewer` was not used; my own viewer plays that part.
- I did not run the nightly script, V-PERF-09 on the 10k world (SPR-030's), or any run on `adjutant`.

## 6. Recommendation

Merge-ready from the Tester side once DEF-1 is fixed or the Architect accepts it as a known flake. GAP-1 needs an Architect decision before SPR-030 writes V-PERF-09's terms. GAP-2 and GAP-3 are small and can go to BCK-002.

## 7. Phase 3: DEF-1 and GAP-1 re-checked (heads `53ecbe9` and `433e42d`)

The Developer's batch is `8b6385f` (DEF-1) and `53ecbe9` (GAP-1). `433e42d` merges `main` on top; it changes nothing under `crates/`, `Cargo.*` or the toolchain (`git diff 53ecbe9 433e42d` is empty there), and I ran both probes on it as well. Method as before: a clean checkout, a fresh release build, my own probes (`flake3.sh`, `p_mem3.py`).

| Item | Verdict | Basis |
|---|---|---|
| DEF-1 | **CLOSED** | 12 of 12 loaded runs pass (was 2 of 6) |
| GAP-1 | **MET, with a caveat on the allowance** | The bound `VmHWM growth <= 2 x peak bytes() + 2 MiB` holds in every run (16 of 16); the ratio without the allowance is 1.9 to 2.5 |
| Deviation in how the queue is measured | **Meets the purpose, not the letter of "encoded size"** | See below |

### 7.1 DEF-1

The fix is `--test-wait-viewer` (the host emits nothing until a viewer's first request is answered, so the viewer receives every year live; the flag is refused by `sift` and by a run without `--socket`), a wait on the viewer's progress in place of the 300 ms sleep, and a 2 ms backoff poll in the harness. The Developer's trace (the viewer's 64-poll backoff of 1.28 s outlasting a 300 ms window, so the years were re-read from disk and none was verified) is consistent with my reading of the failure.

My hunt: the same four test binaries as phase 2 (`serve`, `channel_c`, `run_dir`, `vstr`, 15 tests), release build, four busy loops on four cores, six rounds on `/dev/shm` and six on `/tmp`.

| Condition | Runs | Failures (before the fix) | Failures (after) |
|---|---|---|---|
| loaded, `/dev/shm` | 6 | 1 of 3 | 0 |
| loaded, `/tmp` | 6 | 1 of 3 | 0 |

12 of 12 passed (each 15 of 15; the slowest run took 151 s with the build). On `433e42d` the three `vstr` tests pass once more (45 s). The new flag is in `check_test_hooks.py`'s list: a `--test-wait-viewer` planted in `regen_baseline.sh` fails the guard (line 36), and the clean tree passes.

One design note, not a finding: the probe is now less adversarial. It no longer covers a viewer that returns after the resumed host has re-emitted the lost years. That path is correct by the spec (the viewer reads the sealed segments and never hears that year's `stream.sealed`), and my phase-2 kill probe, which has no handshake, covered it (18 of 18).

### 7.2 GAP-1

The measure: `bytes()` is now the window plus `queued_bytes()`, where a completed year queued for the seal thread counts its records as allocated (`record::allocated`: capacity in records times `size_of::<Record>()`, plus each record's strings and vectors) from the enqueue until the job has run. The ruling said "encoded size at enqueue". The Developer counts memory as allocated instead, because the canonical-text count left the host at 2.75 to 3.4 times `bytes()`. I read the commit and the code (`live.rs`, `record.rs`, `writer.rs`): the count is added at enqueue and removed after the seal or verify job (or when a failed seal drops the queue), as intended. The allocator's own overhead is not counted, which the comment says. This follows the purpose of the ruling (what the live edge holds) better than the letter, and I accept it; it is a deviation from the wording.

My probe: `solo`, full speed, policy (b), headless `VmHWM` measured separately in the same session, the host's own `peak LiveEdge::bytes()`. The bound is the Architect's: growth over the headless median `<= 2 x peak bytes() + 2 MiB`.

| Years | Runs | Headless `VmHWM` (MB) | Growth (MB) | Peak `bytes()` (MB) | Growth / `bytes()` vs median | Margin to the bound (MB) |
|---|---|---|---|---|---|---|
| 600 | 3 | 14.9, 14.9, 15.0 | 4.4 to 5.3 | 2.34 to 2.48 | 1.87 to 2.15 | 1.6 to 2.3 |
| 1,500 | 3 | 16.7, 17.4, 19.6 | 9.0 to 9.4 | 3.76 to 3.93 | 2.28 to 2.49 | **0.15** to 0.9 |
| 1,500 (5 runs on `53ecbe9`, earlier) | 5 | 16.4 to 18.3 | 8.4 to 9.7 | 3.69 to 4.13 | 2.04 to 2.44 | 0.4 to 1.9 |
| 1,500 (on `433e42d`) | 2 | 17.3, 18.0 | 8.8, 9.2 | 3.93, 4.04 | 2.24, 2.29 | 0.8, 1.1 |
| 3,000 | 3 | 23.8 to 25.5 | 10.0 to 10.2 | 4.32 to 4.61 | 2.19 to 2.31 | 0.7 to 1.2 |

All 16 runs are within the bound. Policy (a) on the same runs: `bytes()` 2.5 KB, growth about 0, as before. Wall time did not change.

Does the 2 MiB allowance hide a real gap? **Partly.**
- The allowance covers real noise: identical headless runs differ by 1.9 MB at 1,500 years (16.7 to 18.3) and by 2.9 MB in one batch (16.7 to 19.6), more than 2 MiB. Measured against the headless maximum instead of the median, the ratio is 1.7 to 2.2.
- But the count is still about 2.2 times too small. The uncounted part, growth minus `bytes()`, is 2.0 to 2.9 MB at 600 years and 4.9 to 5.7 MB at 1,500 and 3,000 years, which is 0.9 to 1.5 times `bytes()`. It does not grow without limit: the queue plateaus (peak `bytes()` about 4.5 MB at 3,000 years), so I withdraw the worry I had that the gap grows with run length. It is allocator residency and fragmentation of tens of thousands of small allocations, plus the seal thread's working set (Parquet buffers), none of which `allocated` counts.
- It is the factor 2, not the allowance, that absorbs this under-count, and it leaves a thin margin: 0.15 MB in two of three runs at 1,500 years on `53ecbe9`. The Developer's own gate (`tests/memory.rs`, part golden) measured 1.76 to 1.88 times on its machine. On mine the same bound is nearly tight, so the gate could fail on a slower or noisier runner.

| # | Gap | Recommendation | Confidence |
|---|---|---|---|
| GAP-8 | The V-PERF-09 term "live bytes <= `bytes()` + V x 1 MiB + allowance" is met with `bytes()` that understates the host by about 2.2 times at the median baseline (1.7 to 2.5 across baselines); the 2 MiB allowance in `tests/memory.rs` is smaller than headless noise (up to 2.9 MB) and the margin was 0.15 MB in two of three of my 1,500-year runs | Take the headless baseline as the maximum of three runs, or widen the allowance to 4 MiB, or add a stated per-allocation overhead to `allocated`; carry the factor into SPR-030's V-PERF-09 terms (BCK-002 A-67) | 65% |

### 7.3 Verdicts

DEF-1 is closed. GAP-1 is met as ruled (the bound holds in 16 of 16 runs), with GAP-8 recording that its margin is thin on my machine. The previous gaps GAP-2 to GAP-7 are unchanged and go to BCK-002 at the Architect's audit. No new defect.

## 8. Phase 4: GAP-8 and the merge by gate name (head `762bd84`)

`762bd84` merges `main` `99a5b3d` (PR #89, the third CI job; #95) into the branch. Since my phase-3 report (`f610915`) it adds `ba017f4` (GAP-8: the memory gate's baseline is the highest of three headless runs) and main's CI work. Method as before: a clean checkout, a fresh release build, the repository's own test and my own mutants. I did not run `bin/check_all.sh` whole again; CI is green on all three parts (the Architect's run 39095148268).

| Item | Verdict | Basis |
|---|---|---|
| GAP-8: `tests/memory.rs`, 3 runs at 1,500 years | **PASS** | Margins 4.05, 2.65 and 2.58 MiB; rise 1.51 to 1.85 times `bytes()` (§8.1) |
| GAP-8: "the old canonical count must still fail" | **NOT MET: DEF-2** | The window-only count fails; the canonical-text count fails in 3 of 4 runs and passes in 1 (§8.2) |
| The merge by gate name | **PASS** | 503 gates, each once, both ways; nothing lost from `main`; nothing moved against `main` (§8.3) |

### 8.1 GAP-8: the gate, three runs

`cargo nextest run -p rh-cli --locked --release --test memory --no-capture`, scratch on `/dev/shm`, four cores idle. The test runs the three headless processes in parallel (each process has its own `VmHWM`), takes the highest, then runs `--socket` under policy (b). Bound: rise `<= 2 x peak bytes() + 2 MiB`.

| Run | Headless, highest of three (B) | With `--socket` (B) | Rise (B) | Peak `bytes()` (B) | Rise / `bytes()` | Bound (B) | Margin | Test wall time |
|---|---|---|---|---|---|---|---|---|
| 1 | 20,017,152 | 26,710,016 | 6,692,864 | 4,421,954 | 1.51 | 10,941,060 | 4,248,196 (4.05 MiB) | 30.4 s |
| 2 | 19,861,504 | 28,176,384 | 8,314,880 | 4,496,858 | 1.85 | 11,090,868 | 2,775,988 (2.65 MiB) | 30.1 s |
| 3 | 19,922,944 | 27,607,040 | 7,684,096 | 4,145,848 | 1.85 | 10,388,848 | 2,704,752 (2.58 MiB) | 28.8 s |

The thin margin I saw in phase 3 (0.15 MB) is gone: the highest-of-three baseline removes the headless noise (the three headless peaks in one run differ by up to 2.1 MB). These ratios agree with the Developer's 1.76 to 1.88.

### 8.2 The control: the old counts against the new rule

The repository has no planted control for the memory gate, so I built two mutants of `bytes()` (one edit each, reverted afterwards; `git status` is clean) and ran the same test.

| Mutant | Runs | Result | Rise / `bytes()` | Margin |
|---|---|---|---|---|
| MA: the window only (the count before GAP-1) | 1 | **FAIL** | 4.15 | rise 7,913,472 against a bound of 5,913,348 |
| MB: the seal queue counted as canonical text (the count the Developer rejected) | 4 | **FAIL, FAIL, FAIL, PASS** | 2.94, 2.93, 2.83, **2.41** | the passing run: rise 6,430,720 against a bound of 7,434,642, a margin of 1.0 MB |

The window-only count is rejected, as the ruling requires. The canonical-text count is rejected in three of four runs but **passes in one**: with the baseline now the highest of three (about 3 MB higher than the median I used in phase 3), the rise of the canonical-text mutant fell from the 2.75 to 3.4 times the Developer saw to 2.4 to 2.9 times, and the 2 MiB allowance is large next to the gap between the two counts. A control that must fail and passes one run in four is not a control. The clean count passes 3 of 3, so there is room to tighten.

#### DEF-2 (low): the gate does not reliably reject the canonical-text count

Contract: the Architect's GAP-8 ruling ("the bound stays at 2 x peak `bytes()` + 2 MiB, and the old canonical count must still fail it").
Reproduce: replace `crate::record::allocated(&records)` in `StreamWriter`'s enqueue (`writer.rs`) with the sum of the canonical lines' lengths (+1 each), then run the test four times.
Expected: FAIL every time. Actual: 3 FAIL, 1 PASS.
Suggestion, from the data: the smallest clean margin is 2.58 MiB, so an allowance of 1 MiB leaves 1.5 MiB clean margin and would have failed the passing mutant run by 44 KB; 0.5 MiB fails it by 0.57 MB and leaves 2.1 MiB clean. Either is better than 2 MiB now that the baseline absorbs the noise. Add the two mutants as planted controls in `check_all.sh`, each expected to FAIL with the gate's own message (as `mutant-window-gap` does for the window), so the gate's power is itself tested. Confidence 70%.

### 8.3 The merge by gate name

`python -I bin/check_all_selftest.py inventory`: `INVENTORY OK: 503 gates chained and listed, each once, both ways; 2 control loop(s) called; no run line outside the chain and its loops`.

My own count (`gates.py`: the first quoted string of every `run "…"` line, assigned to the part whose `part X &&` marker precedes it), on three versions of `check_all.sh`:

| Version | Gates | core | golden | stream |
|---|---|---|---|---|
| `main` `99a5b3d` | 485 | 381 | 78 | 26 |
| the branch before the merge (`433e42d`) | 485 | 378 | 107 | none (two parts) |
| `762bd84` | **503** | 389 | 78 | 36 |

- No name appears twice. No gate of `main` is missing from `762bd84`. No gate exists in `762bd84` that is in neither side.
- One name of the branch is absent: "CRN pairing v0 … at seed 1 …". `main` reworded the same gate ("the PR tier (the first VALID seed of 1 to 4)"), so the merge took main's text; nothing is lost.
- **Moved against `main`: 0.** The branch adds 18 gates that `main` lacks (8 in `core`, 10 in `stream`). Against the branch's old two-part layout, 37 gates sit in another part, all by the third job's kind rule (PART_OF_PREFIX): 31 `golden` to `stream` (stream, tail, sift, replay, rungs, `check_segments` and twin gates, and this sprint's serve, channel C, run-directory, V-STR, memory and V-PERF-09 gates), 2 `core` to `stream`, 3 `core` to `golden` (V-DET-10 release; the two release M2 gates) and 1 `golden` to `core` (a population-synthesis control). 27 of the 37 are `main`'s own placements; the other 10 are this sprint's gates, placed by the same rule.
- Part `core` keeps `rh-protocol` golden and method_table with their two planted controls (a fourth method; a nonzero flag), and `rh-stream` live with its `window-gap` control. Part `stream` has round_trip, serve, channel_c, run_dir, vstr, memory, the V-PERF-09 bench and the three twin_check controls (seq, pattern, segment column).
- `kill` and `tail_live` run only as release copies in `stream` (the "kill -9 by name" and "tail by name" gates); the workspace-wide debug run in `core` filters every by-name binary out (`BY_NAME_TESTS` includes them, `rh-cli::memory` too).
- No household golden line moved: its five golden gates (baseline, ladders, two runs, keyframe sizes) are in `golden`, and its three stream-side gates (stream part input, V-SIFT-01, V-SCALE-04) are in `stream`, exactly as on `main`.

### 8.4 Verdicts

GAP-8's bound is met (3 of 3 clean runs with margins of 2.6 to 4.1 MiB). Its second requirement is not: **DEF-2**, the canonical-text count passes the gate in one of four runs. The merge is faithful by gate name. GAP-2 to GAP-7 are unchanged.
