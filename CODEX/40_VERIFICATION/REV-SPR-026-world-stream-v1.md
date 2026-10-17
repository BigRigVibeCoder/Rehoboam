---
id: REV-SPR-026
title: "Independent review of the World Stream v1 wire format and rh-protocol spec (PR #60)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-026, SPR-027, SPR-013, SPR-020, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF: round 1 on `5e19bc2` found 1 blocker, 5 majors, 8 minors and 7 nits. Not mergeable yet.**
> - **What holds.** Re-derived independently, the live-edge rule (I1–I4) is correct under an atomic snapshot. In the reviewer's own simulation: 0 failures in 640 runs, across 8 viewer types, retention 0 and 1, overlapping seals and empty years.
> - **The blocker.** Canonical lines on the live edge overturn ADR-0031 §1 item 4 ("Arrow IPC with LZ4"). That is the director's call, so it goes to the director as D43.
> - **The majors:**
>   - the window bound when a seal overlaps the next year;
>   - a lock the writer could share with readers;
>   - races between a seal and a subscribe;
>   - empty channel-C years;
>   - coupling to the SPR-020 envelope.
> - **Rulings:** R1–R14 in §0. The review ran in a session on a different model from the drafter's.

# REV-SPR-026: independent review of the World Stream v1 wire format

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | BLK-1 | **Accepted.** Replacing Arrow IPC with LZ4 by canonical lines on the live edge amends ADR-0031 §1 item 4, so the director decides it.<ul><li>Draft it as **D43** in the PR body, recommending approval (75%). The case for it: one spelling with a reference; live delivery checked line by line against `segment.canonical_digest`; a stdlib TUI; bytes are no constraint on a local socket. Arrow IPC can be a frame type later.</li><li>Until D43 is ruled, mark `wire_v1.md` §2.2, `05.10` §5 v1.4 and `05.9` IF-0 v1.4 "pending D43". The spec may merge with those marks: SPR-026's code cannot start before SPR-013's</li></ul> | 9/10 |
| R2 | MAJ-1 | **Accepted.** Drop the previous sealed year when the current year *completes*, not at the next seal; the bound is "the current sim-year and the previous one". Reframe D-wire-1 as **D42**, with three options:<ul><li>(a) PERF-5 literal: retention 0 and a synchronous seal on the host thread, against §6.2;</li><li>(b) the current and the previous sim-year with the year-end drop (recommended, 75%);</li><li>(c) a record-count tail.</li></ul>Remove the amended wording from §6.1 and §9 until D42 is ruled. The spec states the bound per option, and the default parameter follows the recommendation, marked "pending D42" | 8/10 |
| R3 | MAJ-2 | **Accepted. The property is normative; the mechanism is recommended.** The writer takes no lock a reader or serving thread can hold, touches no per-connection state, and never waits on a serving thread.<ul><li>Readers pull `stream.sealed` from a shared seal log; the writer never pushes it.</li><li>The recommended mechanism is append-only chunks behind an `AtomicU64` (Release/Acquire), plus one atomically published snapshot of `(w0, s, last_year)`. A std-only seqlock-style snapshot is preferred; `arc-swap` needs the Architect's crate review (name, version, licence, why).</li><li>Replace probe (b) with a deterministic one: freeze a serving thread inside `LiveEdge::read` with a test barrier and stall a socket; the writer must still append and seal 10 sim-years before a deadline.</li><li>A `loom` model of `LiveEdge` is recommended for SPR-026, not required</li></ul> | 8/10 |
| R4 | MAJ-3 | **Accepted.** `LiveEdge::read(from, max) -> Result<Lines, Behind { s, last_year, w0 }>` is evaluated against one snapshot. Every answer that carries `(s, w0)` takes them from one snapshot. The writer publishes `s` before it moves `w0` (I4). The reference models the reader's loads and the writer's steps interleaved, and asserts I1 against the atomic read | 9/10 |
| R5 | MAJ-4 | **Accepted.**<ul><li>`sealed.last_seq` is nullable: the last seq of the last non-empty sealed segment.</li><li>`stream.sealed` for an empty year carries `last_seq: null` and `first_seq` = the next seq, and is sent at once.</li><li>The simulation allows empty years on channel C.</li><li>`check_status` accepts null</li></ul> | 9/10 |
| R6 | MAJ-5 | **Accepted.**<ul><li>The reference builds its record lines from the canonical reference (`canonical_v0.py`'s generator or golden lines), never hand-typed.</li><li>The wire relies only on the line prefix `<KIND> <tl> <seq> <tick> `, which `canonical_v0.md` states as frozen across canonical versions; coordinate that sentence with the SPR-020 drafter.</li><li>"MATCH only" becomes "a kind the canonical format assigns to that channel".</li><li>Correct `:68`.</li><li>Merge order is free: whichever of PR #60 and the SPR-020 PR merges second merges `main` and regenerates `golden/wire_v1.txt` with the tool</li></ul> | 9/10 |
| R7 | MIN-1 | **Accepted.**<ul><li>Add `\bnet::(?:Unix\|Tcp\|Udp)\w*` to the `network` rule, with the three parent-module bypasses as planted controls.</li><li>Give `rh-protocol` an exact third-party allow-list: `serde`, `serde_json`, and `proptest` as a dev-dependency only.</li><li>Sockets in `rh-cli` are allowed only in `serve.rs`, as a file allow-list (MAJ-2 c)</li></ul> | 9/10 |
| R8 | MIN-2 | **Accepted.**<ul><li>An error moves `Catchup` to `Failed`, which accepts only `Closed`, then a resubscribe at the cursor.</li><li>A `Retry` with backoff handles `future_seq` and `too_many_viewers`.</li><li>`stream.behind.next_seq` is checked.</li><li>A host `run_nonce` in the status is checked on reconnect.</li><li>A `Jump` input goes through the machine</li></ul> | 8/10 |
| R9 | MIN-3 | **Accepted.**<ul><li>Frames from a viewer are capped at 4 KiB, with nesting depth at most 4.</li><li>Unknown members are refused.</li><li>Decode errors are `WireError`.</li><li>A host dispatcher in the reference, with golden refusals including `method_not_found`</li></ul> | 8/10 |
| R10 | MIN-4, MIN-5, MIN-7 | **Accepted.**<ul><li>Lines are validated once, at append, or not at all; serving threads copy bytes.</li><li>Name the stdlib sealed-read path (a `rh-cli tail --canonical` subprocess).</li><li>Viewers filter on the header by a split.</li><li>V-PERF-09 pins the viewers' cores.</li><li>The RSS bound is `LiveEdge::bytes()` + V × the frame cap (records frames capped at 1 MiB) + a stated allowance</li></ul> | 8/10 |
| R11 | MIN-6 | **Accepted.**<ul><li>An invariant for every version: the stream sockets accept exactly the `stream.*` read methods.</li><li>A conformance test enumerates the method table.</li><li>The run directory is 0700, and `control.sock` is 0600 with its own version string</li></ul> | 8/10 |
| R12 | MIN-8; open question 3 | **Accepted.**<ul><li>`stream.end { reason: finished \| aborted }` now.</li><li>A poisoned host never seals a partial year.</li><li>BCK-002 A-42 stays with SPR-026's host.</li><li>A-36 moves to P2, the first journaled input on `control.sock`; neither SPR-026 nor SPR-027 journals an input</li></ul> | 7/10 |
| R13 | Open questions 1, 2 | **SPR-026 serves `C.sock`,** with the same `serve.rs`, once R5 holds (65%). **The host code lives in `rh-cli/src/serve.rs`;** no `rh-server` crate before it has a consumer (75%) | 7/10 |
| R14 | NIT-1 to NIT-7 | All accepted:<ul><li>a real digest in the golden `notify_sealed`;</li><li>the control never rewrites the golden file, and plants the stated failure;</li><li>`05.9`'s version order;</li><li>`docs/14` §K records why `jsonrpsee` is not used;</li><li>socket paths `stream/t<id>/{A,C}.sock`, plus a run-directory `flock`;</li><li>IF-0 = {A.sock, C.sock, control.sock} stated against ADR-0035 §6.</li></ul>NIT-6 (`docs/11` P1.17 still lists tip and scheduled inputs in P1a) goes to the director with D42 and D43 as a roadmap text fix | 9/10 |

**Director items from this PR:** D42 (the live-edge window, PERF-5 and V-PERF-09) and D43 (ADR-0031 §1 item 4, the live-edge container). The drafter drafts both in the PR body; the Architect files them in `docs/decisions_pending.md` when the spec is ready to merge.

## 1. The review (round 1, independent reviewer)

Reviewer: independent and adversarial, on a different model from the drafter's. Copy reviewed: `$R/src`, a git archive of `5e19bc2`. `$R` = the Architect's scratchpad `wire_review/`. Its scripts (`my_live_edge_sim.py`, `policy_b.py`, `empty_year.py`) stay in that scratchpad.

Reviewer: independent (Claude Opus 5.5), 2026-10-17. Copy: `wire_review/src` (git archive of `5e19bc2`). My scripts are in `wire_review/scripts/`.

## BLUF

**Not mergeable as is: 1 BLK, 5 MAJ, 8 MIN, 7 NIT.**

- The protocol core is sound. With an atomic host snapshot, my own simulation (not the drafter's) delivers seqs `0..N-1` with no gap and no duplicate. It covers 8 viewer types, retention 0 and 1, synchronous and overlapping seals, and empty years: 0 failures in 640 runs. The frame codec, the canonical JSON and the read-only method table are well specified, and every gate passes.
- **The blocker is governance.** ADR-0031 §1 item 4, which the director accepted, says "Live edge: Arrow IPC with LZ4". The PR replaces it and amends only `05.10` and `05.9`. Under `docs/13` §4.2, a change to an ADR needs the director's decision first.
- **The MAJ findings:**
  - the window bound behind D-wire-1 is wrong whenever a seal overlaps the next year's appends, which §4.3 says it does;
  - "the engine never waits" is asserted, not specified: the writer may take a lock that a reader holds, and the seal pushes into each connection's state;
  - the host may read `(s, w0)` from two moments, which breaks I1 as the viewer sees it;
  - empty channel-C years cannot be expressed on the wire;
  - SPR-020 breaks this PR's reference and golden file on merge.

Recommendation: fix the five MAJs, add D-wire-2 (amend ADR-0031 §1.4; approve, 80%), and reframe D-wire-1. Then this is a good contract.

## Findings

### BLK-1: the live-edge container contradicts an Accepted ADR, with no director item
- **Where:** `specs/stream/wire_v1.md:70`, `docs/05_architecture/05.10_world_stream.md:80`, `05.9_interfaces_icd.md:22`.
- **Problem:** ADR-0031 §1 item 4 (`docs/06_adr/ADR-0031-…md:40`, Accepted by the director on 2026-10-16) says **"Live edge: Arrow IPC with LZ4."** The PR changes the container to canonical lines but amends only the architecture prose. Two things contradict it:
  - `docs/13` §4.2 (C2 row): "A change to an ADR needs your decision first."
  - The spec's own justification cites "ADR-0031 §4 layer 3" for the at-rest container and never mentions §1.4.
- **Fix:**
  - Add **D-wire-2**: amend ADR-0031 §1 item 4 to "Live edge: frames of canonical lines (`wire_v1.md`); Arrow IPC may be added later as a frame type".
  - Recommend approval (80%). The bandwidth case holds (MIN-5) and the digest-verifiability argument is real.
  - Until the director rules, mark §2.2, `05.10` v1.4 and `05.9` v1.4 "pending D-wire-2". Confidence 95% that this needs the director.

### MAJ-1: "two sim-years at most" and D-wire-1's bound are false when sealing overlaps
- **Where:** `wire_v1.md:194` (and :148, :266).
- **Problem:** §4.3 (:148) says "sealing year *y* and writing year *y + 1*'s first records overlap". §6.1 drops a retained year "only at a seal". During the overlap the window therefore holds three years with retention 1: *y−1* (retained), *y* (sealing) and *y+1* (being written). With retention 0 it holds two, while the spec says retention 0 "keeps the bound at one segment".
- **Why the reference does not show it:** the reference seals synchronously, inside the tick and before any record of *y+1* (`selftest_wire_v1.py:584`, called at :951). It therefore never models what §4.3 says.
- **The trap:** if an implementation makes the seal synchronous to meet the bound, Parquet encoding and `fsync` run on the host thread. That contradicts §6.2: "[appending] is the only thing on the engine host's tick path".
- **Evidence:** `scripts/my_live_edge_sim.py` with an atomic host, 40 seeds per configuration:

  | Retention | Synchronous seal (delay 0) | Overlapping seal (delay 1, 5, 11 ticks) |
  |---|---|---|
  | 1 | peak 2 years | peak **3** years |
  | 0 | peak 1 year | peak **2** years |

- **D-wire-1's framing is not neutral:**
  - §6.1 ("is read as this window") and §9's pass criterion already apply the amended wording before the director rules.
  - It omits the alternative that needs no ruling: retention 0 with a synchronous seal keeps PERF-5 literally.
- **Fix:**
  - Drop the previous sealed year when the current year **completes**, not at the next seal. `scripts/policy_b.py` shows a peak of 2 years at every seal delay, with 0 failures in 240 runs.
  - State the bound as "the current sim-year and the previous one".
  - Reframe D-wire-1 with three options: (a) PERF-5 unchanged, retention 0, synchronous seal; (b) "current and previous sim-year" with the year-end drop (recommended, 75%); (c) a record-count tail.
  - Default `retain_sealed_years` to 0 until the ruling.
- **Confidence:** 90%.

### MAJ-2: "the engine never waits" is not specified at the writer/server boundary, and the probe cannot see a regression
- **Where:** `wire_v1.md:206-210`, :251-252, :267; `selftest_wire_v1.py:604`, :720.
- **Problems:**
  - **(a) The writer may share a lock with readers.** §6.2 forbids only a "lock held across a socket write". It permits an `RwLock` or condvar mutex that the writer takes while a serving thread holds it (copying, or preempted). That is weaker than the existing harness rule, `05.9` :142: "Reads never lock the tick … `Arc<SnapshotFrame>`".
  - **(b) The seal pushes into viewer state.** "A seal sends `stream.sealed` to every subscription", and the host keeps "the sealed notifications it owes" per connection. The reference does exactly this: `Writer.on_seal` → `Host._sealed` appends to every `conn.sealed_pending`. In a threaded host, a per-connection mutex held across a write that a stalled viewer blocks would freeze the writer at the next seal.
  - **(c) The `network` lint does not cover the tick loop.** The rule covers plane A only, but the tick loop and `serve.rs` are both in `rh-cli`, which is tool plane. "Never waits as a lint, not a promise" (:252) therefore does not hold for the code that calls `append`.
  - **(d) Probe (b) is statistical.** It compares a sampled per-append maximum against the no-viewer run's maximum. A microsecond-scale lock wait sits far under the maximum set by allocation and page faults, and scheduler noise can make the probe flaky. The stalled consumer does catch the gross case, a blocking socket write on the tick path.
- **Fix:**
  - Specify that the writer takes no lock and touches no connection state.
  - Publish `LiveEdge` with append-only chunks behind an `AtomicU64 next_seq` (Release/Acquire), plus one `Arc`-swapped snapshot of `(w0, s, last_year)` and the seal log.
  - Serving threads poll (sleep), or wake through a primitive the writer cannot block on.
  - Serving threads *pull* `stream.sealed` from the shared seal log.
  - Add a deterministic probe: freeze a serving thread inside `LiveEdge::read` (test barrier) and stall a socket. The writer must still append and seal 10 years before a deadline (pass/fail). Add a `loom` model of `LiveEdge`.
  - Restrict sockets in `rh-cli` to `serve.rs` (MIN-1's lint, with a file allow-list).
- **Confidence:** 85%.

### MAJ-3: races between a seal and a subscribe or a serve
- **Where:** `wire_v1.md:127`, :197, :208, :211, :251 (`first_seq`, `next_seq` and `lines(from, max)` as separate calls).
- **Problem:** the invariants are said to hold "at every instant a viewer can observe", but nothing says how a threaded host reads them.
  - If the host reads `s` and then `w0` in two loads while a seal completes between them, then with retention 0 a `behind_live_edge` or `stream.behind` carries `s + 1 < w0`. The viewer's I1 assertion then turns it into a protocol error.
  - Check-then-copy (`c ≥ w0`, then `lines(c)`) can copy after `w0` has moved.
- **Evidence** (my simulation, writer micro-operations interleaved between the reader's two loads):

  | Read order | Retention | Overlapping seal | Seeds that fail | Failure |
  |---|---|---|---|---|
  | `s` then `w0` | 0 | delay 1 | 40/40 | "I1 visible: s=137 w0=162" |
  | `w0` then `s` | 0 | any | 0 | no I1 failure |
  | either order, non-atomic check-then-copy | 0 and 1 | any | up to 40/40 | "TOCTOU: copy from c after w0 moved" |

  The atomic snapshot fails 0 times.
- **Fix:**
  - `LiveEdge::read(from, max) -> Result<Lines, Behind{s, last_year, w0}>`, evaluated against one snapshot.
  - Every answer carrying `(s, w0)` takes them from one snapshot. If loads are separate, `w0` is read before `s`, and the writer publishes `s` before it moves `w0` (I4).
- **Confidence:** 90%.

### MAJ-4: empty channel-C years cannot be expressed on the wire
- **Where:** `wire_v1.md:123`, :148, :92 (`last_seq` is a `u64`); `selftest_wire_v1.py:948`.
- **Problem:** `channel_c_v0.sql` R7 seals a sim-year with no match (`record_count = 0`, `last_seq` NULL). The wire, however:
  - types `sealed.last_seq` and `stream.sealed.last_seq` as `u64`;
  - sends `stream.sealed` "after the frame that carries `last_seq`", a frame that does not exist for an empty year.
- **Evidence** (`scripts/empty_year.py`):
  - The reference emits `stream.sealed{… "last_seq":-1 …}`.
  - Its own `check_status` refuses its own status: "sealed.last_seq is not a u64".
  - The simulation never covers the case: `randint(1 if last_of_year …)` forces at least one record per year.
- **Fix:**
  - `sealed.last_seq` is the last seq of the last non-empty sealed segment, or null.
  - `stream.sealed` for an empty year carries `last_seq: null` and `first_seq` = the next seq, and is sent at once.
  - The simulation allows empty years on channel C.
- **Confidence:** 90%.

### MAJ-5: SPR-020 breaks the reference and the golden file; the agnosticism claim is wrong
- **Where:** `wire_v1.md:68`, :185; `selftest_wire_v1.py:1028-1036`.
- **Problem:** the spec says the envelope "changes the canonical form and its version string; it changes nothing here". In fact `arch/SPR-020-envelope` amends canonical **v0 in place** (the version stays v0, and `EVENT` gains `snapshots`). The wire reference, however, hand-types P1.0-shaped lines (:1035). Those lines are embedded in `golden/wire_v1.txt`: the records frames and 8 of the refusal lines.
- **Evidence:**
  - `git merge-tree` of `5e19bc2` with SPR-020 is textually clean.
  - On the merged tree, `selftest_wire_v1.py --check` crashes with a traceback: `WireError: seq 10: not a canonical line: EVENT: 16 fields, expected 17`.
- **A second coupling:** "anything but MATCH on channel C" (:64) is a protocol error. Channel C's future kinds (`SIGNIFICANCE`, `SUSPENSE`, `VISIBILITY`) would therefore need wire v2.
- **Fix:**
  - Build `_records_a` and `_records_c` from the canonical reference's own golden lines or generator.
  - State the one line prefix the wire relies on, `<KIND> <tl> <seq> <tick> `, as frozen across canonical versions.
  - Replace "MATCH only" with "a kind the canonical format assigns to the other channel".
  - Correct the sentence at :68.
  - Merge SPR-020 first, then regenerate `wire_v1.txt` here.
- **Confidence:** 95%.

### MIN-1: the `network` lint is bypassed through the parent module; `protocol-leaf` leaves third-party crates open
- **Where:** `bin/lint_forbidden.py:378-379`; `bin/lint_arch.py:736`.
- **Bypasses:** each of these compiles with `rustc` and is NOT CAUGHT:
  - `use std::os::unix as u; u::net::UnixListener::bind(p)`
  - `use std::os::unix::*; net::UnixListener::…`
  - `use std::os::unix::{self as x}; x::net::UnixDatagram`
- **Caught:** `use std::os::unix::net as n`, fully qualified and raw-identifier paths, groups, `use std::os as o`, and macro paths.
- **`tokio::net` and `mio`:** not seen by `lint_forbidden`, but refused for plane A by `PLANE_A_EXTERNAL_DEPS`.
- **`protocol-leaf`:** checks workspace members only. `rh-protocol` could add `tokio`, `mio` or `socket2` despite §7's "Nothing else".
- **Fix:**
  - Add `|\bnet::(?:Unix|Tcp|Udp)\w*` to the rule. Verified: it catches all 4 bypasses, the workspace stays at 0 violations, and the planted fixture still reports exactly 1 finding.
  - Give `rh-protocol` an exact third-party allow-list: `serde`, `serde_json`, and `proptest` as a dev-dependency.
- **Confidence:** 95%.

### MIN-2: `Catchup` keeps going after a protocol error and has no retry path
- **Where:** `golden/wire_v1.txt:109-112`; `selftest_wire_v1.py:760`, :835, :897.
- **Problems:**
  - After "first_seq 402 is not the cursor 401" the machine delivers 401..403 on the same subscription. §2 says to stop reading and close, and §2.2 says "never as something to skip".
  - There is no event for `future_seq` or `too_many_viewers`. A viewer that reconnects to a resumed host after a `kill -9` (SPR-013 T-09) asks for a cursor above the new tip, and the reference records "subscribe refused: future_seq" and stops.
  - The `next_seq` of `stream.behind` is never checked.
  - A reconnect never re-checks `status.history`.
  - "Jump to live" bypasses the machine (`loop.state = "live"`).
- **Fix:**
  - An error moves the machine to `Failed`, which accepts only `Closed`, then resubscribes at the cursor.
  - Add `Retry` with backoff for `future_seq` and `too_many_viewers`.
  - Check `next_seq`.
  - Add a host `run_nonce` to the status, checked on reconnect.
  - Add a `Jump` input.
- **Confidence:** 85%.

### MIN-3: control frames are unbounded and the host's refusals are not pinned
- **Problems:**
  - The 16 MiB limit also applies to frames from the viewer: 64 connections × 16 MiB is 1 GiB of request buffers.
  - Nesting depth has no limit. The reference crashes with `RecursionError`, while `serde_json` stops at depth 128.
  - The reference accepts unknown top-level members and `stream.status` params `{x:1}`, against §5's "a host never ignores".
  - A non-ASCII byte in a RECORDS body crashes the reference with `UnicodeDecodeError`, which is not a `WireError` (:239).
  - There is no host dispatcher, so neither `method_not_found` nor the parameter refusals are in the golden file.
- **Fix:**
  - A 4 KiB limit for frames from the viewer, and a nesting depth of at most 4.
  - Refuse unknown members.
  - Raise `WireError` for decode errors.
  - Add a dispatcher, and golden refusals for its answers.
- **Confidence:** 85%.

### MIN-4: the host parses every line for every viewer
- **Where:** `wire_v1.md:250`; `selftest_wire_v1.py:211`.
- **Problem:** "the line check uses `rh-stream`'s parser in the host and the viewer", and in the reference `Host.serve` → `encode_records` parses each line. That contradicts "a host does no work per viewer but copying bytes" (§4.2).
- **Fix:** validate once at append, or not at all, since the writer rendered the line. Serving threads copy bytes.
- **Confidence:** 80%.

### MIN-5: format reason (2) is overstated; the viewer's CPU cost is not budgeted
- **Overstated claim:** any `Catchup` viewer must read sealed Parquet for `[c, s]`, so a standard-library Python stub needs `pyarrow` or DuckDB, or a `rh-cli tail --canonical` subprocess. The spec names neither.
- **Bytes are fine:**
  - A 10k world makes about 3–7k records per sim-year at 80–120 bytes (after SPR-020).
  - At the G1a pace of at least 16.7 sim-years/s, that is about 4–14 MB/s, or 50–120k lines/s.
  - At 1M persons it is about 28 MB/s. A Unix socket carries GB/s.
- **CPU is not:** full parsing in Python measured **14.7 µs/line**, or about 68k lines/s on one core. The TUI stub would saturate one core of the 8-core V-PERF-09 machine, which threatens the ≥ 95% throughput criterion.
- **Fix:**
  - Name the standard-library sealed-read path.
  - Viewers filter on the header with a split (measured 0.3 µs/line).
  - V-PERF-09 pins the viewers' cores.
- **Confidence:** 75%.

### MIN-6: the reservation of `control.sock` is not version-proof
- **Where:** `wire_v1.md:172-176`, :183-184, :257.
- **Problem:**
  - "Read-only" is a property of v1 only. §5 does not classify a new *request* method, so nothing stops v2 or SPR-027 from adding one to `A.sock` or `C.sock`.
  - The socket file modes are unspecified, so "a property of a file" is not yet true.
  - `control.sock`'s own version string is undefined.
- **Fix:**
  - State as an invariant for every version: the stream sockets accept exactly the `stream.*` read methods.
  - A conformance test enumerates the method table.
  - `control.sock` is mode 0600.
  - `control.sock`'s method set carries its own version string.
- **Confidence:** 80%.

### MIN-7: V-PERF-09's RSS measure cannot pass as written
- **Where:** `wire_v1.md:266`.
- **Problem:** "the sum of the two segments' line bytes" leaves out three costs:
  - per-connection frame buffers, up to 16 MiB each;
  - thread stacks;
  - `String`/`Vec` overhead, about 1.5–2× for 60–100-byte lines.
- **Scale:** at 10k persons the window is about 0.5–1.6 MB, which is inside RSS noise.
- **Fix:** bound = `LiveEdge::bytes()` as allocated + V × a frame cap (cap frames at 1 MiB) + a fixed allowance.
- **Confidence:** 80%.

### MIN-8: a run that ends on error has no terminal message
- **Problem:** `stream.end` means "finished, last segment sealed", and nothing covers a poisoned engine (BCK-002 A-42).
- **Fix:**
  - Add `stream.end{reason: finished|aborted}` now.
  - A poisoned host never seals a partial year.
- **Confidence:** 70%.

### NITs
- **NIT-1:** the golden `notify_sealed` (`selftest_wire_v1.py:1074`) carries `99aa06d3…`, the XXH3-128 of an *empty* body, for a 300-record year.
- **NIT-2:**
  - `--control window-gap` without `--check` rewrites the committed golden file if the control ever passes (:1432).
  - The control plants a short sealed segment, not "memory dropped before the seal is on disk": with retention 1 it fails through the sealed-read count, and through I1 only with retention 0.
- **NIT-3:** `05.9`: the v1.4 status line sits above v1.3.
- **NIT-4:** `docs/14` §K lists jsonrpsee as DEPEND ("saves hand-rolled RPC framing") for `rh-protocol`. Record why it is not used: it has no Unix-socket transport and needs an async runtime.
- **NIT-5:** the socket path `stream/A.sock` has no timeline component, while `channel_a_v0.sql` R5 lays out `stream/t<id>/…`, several timelines per run directory. Use `stream/t<id>/A.sock`, plus a run-directory `flock`, so that a second host cannot unlink a live socket as "stale".
- **NIT-6:** `docs/11` P1.17 lists "tip and scheduled inputs" in P1a; the spec, PRJ-002 and STR-4 put them in P2. This was already inconsistent before the PR; flag it to the director.
- **NIT-7:** ADR-0035 §6 says "the one local control interface … a control and stream endpoint". State that IF-0 is `{A.sock, C.sock, control.sock}`.

## Answers to the brief

- **Q1 (the live-edge rule):** I re-derived I1–I4. They are correct under an atomic snapshot. Seal–subscribe races: MAJ-3. A cursor inside a segment being sealed is served from memory, and that is fine. A cursor ahead of the writer: `future_seq`, which `Catchup` does not handle (MIN-2). What the reference does that the text does not say: it seals synchronously, pushes sealed notifications into each connection, parses lines for each viewer, continues after an error, forces non-empty years, and ignores `behind.next_seq`.
- **Q2 (the engine never waits):** the thread-per-connection model and the socket buffers are fine; only the serving thread blocks. The shared lock and the seal fan-out: MAJ-2. "Only copying bytes": no (MIN-4). V-PERF-09: the stalled consumer catches the gross case; probe (b) does not catch the subtle one (MAJ-2).
- **Q3 (planes):**
  - No method or frame on `A.sock` or `C.sock` reaches plane A: `method_not_found` answers everything else, and records frames from a viewer are `invalid_frame`.
  - Files in the run directory are outside the protocol, the same user's.
  - `control.sock` and the lint gaps: MIN-6 and MIN-1.
  - Each fixture fails for its own rule. Like the existing fixtures, both arch fixtures also print the "no Cargo.lock" line.
- **Q4 (format):** sound on bytes; governance in BLK-1; CPU in MIN-5. The canonical JSON is well specified except for the limits in MIN-3. Versioning is enough for SPR-027 once MIN-6 is fixed. P2: the spec does not yet say how RECORDS frames map onto IF-3's WebSocket text frames; leave that to P2.
- **Q5 (SPR-020):** the protocol is agnostic; the reference, the golden file and the "MATCH only" rule are not (MAJ-5).
- **Q6 (governance):**
  - The `05.10` §5 and `05.9` IF-0 text is the Architect's (C2), except where it overturns ADR-0031 §1.4 (BLK-1).
  - D-wire-1 is the director's, and its framing is not neutral (MAJ-1).
  - `docs/09` V-PERF-09 is covered by D-wire-1.
  - The `05.0` §6 crate list is the Architect's.
- **Q7 (the drafter's open questions):**
  - **`C.sock` in SPR-026:** yes, with the same `serve.rs`, once MAJ-4 is fixed (65%).
  - **Where the host goes:** `rh-cli/src/serve.rs` now, with sockets allowed only in that file; no `rh-server` before a consumer exists (75%).
  - **A-36:** moves to P2, the first journaled input on `control.sock`. Neither SPR-026 nor SPR-027 journals an input; pace goes to the run log (80%).
  - **A-42:** stays with SPR-026's host, decided as in MIN-8 (65%).
  - **Shared files:** this PR and SPR-020 both edit `CLAUDE.md`, PRJ-002, `check_all.sh`, the inventory test and the lint-fixtures README. The merge is textually clean, but the golden file breaks (MAJ-5). Merge SPR-020 first.
- **Q8 (gates):**
  - All pass in the copy: `selftest_wire_v1 --check` (OK, 116 lines), its window-gap control (FAIL as required), the stale-golden control (STALE:), `lint_arch` and its two fixtures, `lint_forbidden` and `net_socket`, `gen_manifest --check`, ruff, mypy (43 files), and pytest (632 passed).
  - One test, `test_the_deny_list_is_defined_once`, needs `git ls-files`, so I ran `git init` in the copy first.
  - Each new gate is in the core part only (`check_all.sh` :744-745 and :929-931, between `part core` at :702 and `part golden` at :933).

## Sound (checked)
- **Framing:** a 12-byte header whose magic and version are frozen across versions; the 22 refusals; Incomplete handling; contiguous seqs within one subscription; a records body equal to the digest input, so a live year can be verified against its sealed digest.
- **Canonical JSON:** ranges, escapes, key order, repeated keys (refused at every depth), and floats, NaN and 1e400 (refused).
- **Read-only method table and its message;** error codes in IF-3's validation range.
- **The live-edge rule** under an atomic snapshot, at both retentions, with synchronous and overlapping seals: my simulation.
- **Catch-up termination:** the window moves once per sim-year.
- **Plane-A isolation:** `tokio` and `mio` are refused for plane A by the reviewed-dependency list; the plane-A closure refuses `rh-stream` → `rh-protocol`; `protocol-leaf` catches normal, build and dev edges, renames and target tables.
- **Channel C:** served by the sifter, which keeps the engine unaware of it (V-PLN-04).

## Not run
- No Rust: `rh-protocol`, `serve.rs` and `LiveEdge` do not exist yet, and nothing in the PR builds.
- The full `bin/check_all.sh`, which needs valgrind and the other tools.
- V-PERF-09 and the V-STR rows, which need an implementation.
- A real threaded host and `loom`; I modelled the interleavings in Python only.
- CPU and RSS figures are estimates from `docs/08`, `docs/11` and generated lines, except the Python parse speed, which I measured.
