---
id: SPR-026
title: "World Stream v1: the live edge over a local socket, viewer cursors and rh-protocol (P1.17, part a)"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-002, PRJ-002, REV-SPR-026, SPR-013, SPR-014, SPR-020, SPR-027, SPR-030, ADR-0024, ADR-0030, ADR-0031, ADR-0035, VER-SPR-026]
created: 2026-10-17
updated: 2026-10-18
version: 1.0.0
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

> **BLUF:** Builds the live edge of IF-0 (`specs/stream/wire_v1.md`, PR #60). This document calls it World Stream v1. The sprint delivers:
> - **`rh-protocol`:** the frame codec, the JSON-RPC control plane and the `Catchup` viewer state machine.
> - **`rh-stream`'s `LiveEdge`:** one `ArcSwap` snapshot, so the writer never waits.
> - **The Unix-socket host in `crates/rh-cli/src/serve.rs`:** it serves `A.sock` for `rh-cli run … --socket` and `C.sock` for the sifter.
> - **`rh-cli tail`'s live mode and `--to-seq`.**
>
> No hash and no channel-A byte changes.
>
> - **Developers:** AGT-002-TOOLS (`rh-protocol`, the host, the viewers), with AGT-002-CORE for `rh-stream`'s `LiveEdge`. One Developer session may hold both roles.
> - **Testers:** AGT-003-IF and AGT-003-DET. One Tester session may hold both roles, on a different model from the Developer's.
> - **Size:** about 3 agent-days.
> - **Class: C2** (the IF-0 wire, a new tool crate; channel A unchanged).
> - **Plan approved by the director on 2026-10-17 (D48; GOV-005 §2).** D42 (the window, option (b)) and D43 (canonical lines on the live edge) were approved on 2026-10-17, as the spec recommended.

# SPR-026 — World Stream v1: the live edge (WP P1.17, part a)

- **Roadmap item:** `docs/11` P1.17 (a) · **Phase / rung:** P1a, rungs 0–2
- **Depends on:**
  - **SPR-013 merged** (REV-SPR-026 R1: this code cannot start before SPR-013's). SPR-026 builds on four things SPR-013 delivers:
    - `rh-stream`'s writer and its sealed segments;
    - `rh-cli tail` (T-10) and `rh-cli sift` (T-11);
    - the canonical printer and parser;
    - T-09's `kill -9` line: a resume re-emits from the first sim-year not on disk, byte for byte. That is where a resumed host's live edge starts (`wire_v1.md` §6.2, §11; R18).
  - **Independent of SPR-014.** SPR-020's record content passes through unparsed: the wire reads only the frozen prefix (`canonical_v0.md` §1).
- **Binding contracts:**
  - `specs/stream/wire_v1.md` §0–§10, with `specs/stream/selftest_wire_v1.py` and `specs/stream/golden/wire_v1.txt`;
  - `specs/stream/canonical_v0.md` §1 (frozen prefix) and §4 (digest);
  - `specs/stream/channel_a_v0.sql` R3 and R5; `specs/stream/channel_c_v0.sql` R7;
  - `05.10` §5 and §7, and `05.9` IF-0 (v1.4; D43 approved);
  - ADR-0024, ADR-0030, ADR-0031 (§1 item 4 amended by D43), ADR-0035 §6;
  - `docs/09` V-STR-01, V-STR-03, V-STR-05, V-PERF-09 (ingredients only), V-IF-02 (style), V-DET-19, V-INSP-03, V-INSP-05, V-PLN-01, V-PLN-04.
- **System spec:** n/a.
- **Rulings (binding):**
  - REV-SPR-026 R1–R24;
  - SPR-013 T-09's resume line (R18);
  - BCK-002 A-42, its stream-host part (R12).
- **Director items:** D42, D43 and D47, all approved on 2026-10-17.
- **Requirements covered:**
  - STR-1 (stream determinism under viewers);
  - STR-3 (cursors, catch-up with no gap and no duplicate);
  - STR-5 (no pace command on a stream socket);
  - PERF-5 (the ingredients of V-PERF-09; the gate is SPR-030's);
  - EVO-4 (`rh-protocol` is an engine crate under the deny-list).

## Specs this sprint needs

| Spec | Exists? | Gap | Writer | Needed by |
|---|---|---|---|---|
| `wire_v1.md`, its reference and golden file | Exists (PR #60, `a53cac2`; `REV-SPR-026` closed). §2.2 is pending D43 and §6.1 pending D42. The two stale SPR-020 sentences (§2.2, §5) were corrected by the Architect at this plan (2026-10-17) | — | — | T-01 |
| The `rh-cli tail` flag for the live edge | **Ruled** (Architect, 7/10): `--follow <person>` stays the person filter (SPR-013 T-10, `05.10`); the socket mode is `tail --live`. `wire_v1.md` §1, §7 and §9 say `--live` now; `sift --follow` keeps its name | — | — | T-05 |
| Where a viewer's socket code lives in `rh-cli` | Silent. `NETWORK_FILES` allows sockets in `src/serve.rs` alone | **Ruled** (Architect, 7/10): the client side (`tail --live`) also lives in `serve.rs`, with no lint change | — | T-05 |
| Whether SPR-013's seal runs on the host thread | Silent in SPR-013 T-09. `wire_v1.md` §6.1 requires an off-thread seal for option (b) | If SPR-013 seals on the host thread, T-02 moves sealing to a seal thread under (b) and keeps it synchronous under (a) | — | T-02 |
| Whether `LiveEdge` exists in a headless run | Silent | **Ruled** (Architect, 7/10): `LiveEdge` is built only when the host serves (`--socket`). Headless runs, the golden ladders and the allocation and instruction baselines pay nothing | — | T-02 |
| Property-test tooling for `rh-protocol` | R7 allows `proptest` as a dev-dependency, but it is not in `Cargo.lock`. The repository's practice is fixed-seed SplitMix64 cases (`crates/rh-core/tests/time_properties.rs`) | **Ruled** (Architect): SplitMix64 property tests and no new crate. `proptest`, or `loom` (§6.2, recommended), only after the Architect's crate review | — | T-01, T-02 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C2:** IF-0's wire (a new interface and a new crate, `rh-protocol`). `state_hash`, channel A and channel C bytes are unchanged on every world |
| Planes touched (ADR-0024) | **A:** `rh-stream`'s `LiveEdge`, which holds no simulation state and only copies lines the writer already rendered. **C:** the sifter's serving of `C.sock`; channel C's bytes are unchanged. **Tool:** `rh-protocol` (pinned tool/tool), `rh-cli` (`serve.rs`, `run --socket`, `tail --live --to-seq`, `sift --follow --socket`). **B:** none |
| Owned columns written | None |
| Reads (via `rh-contracts`) | None |
| RNG purposes added | None. `run_nonce` comes from the wall clock in the tool-plane host and is never in plane A (§3.2; R21) |
| Golden-hash effect | **None:** the ladder with viewers is the ladder without (§9, probe (c)). `worlds/*/baselines/` are unchanged |
| Fixed-point quantities | None. The wire carries integers only: no float anywhere, and a float is refused (§3.1; R24) |
| Determinism lints (GOV-003 §2, §6) | Must pass. The rules already exist (PR #60): `network`, `network-reexport`, `NETWORK_FILES` in `bin/lint_forbidden.py`, and `protocol-leaf`, `PROTOCOL_EXTERNAL_DEPS`, the `rh-protocol` pin, `ENGINE_CRATES` and `arc-swap` on `PLANE_A_EXTERNAL_DEPS` for `rh-stream` alone (R22) in `bin/lint_arch.py`. The sprint makes them hold over real code. `arc-swap` gets an exact pin at a version at least two weeks old, with default features off |
| Tests that gate this sprint | V-STR-01, V-STR-03 (channel A unchanged; throughput reported), V-STR-05, V-IF-02 (style: the host's refusals under fuzz), V-DET-19, V-INSP-03, V-INSP-05, V-PLN-01, V-PLN-04, V-GOLD-01 (unchanged), V-PERF-04 and V-PERF-11 (unchanged headless), the read-only probe, the seal-race probe and probe (b) of §9, plus `bin/check_all.sh` |
| Docs updated in the same sprint | `bin/check_all.sh` and `python/tests/test_check_all_inventory.py` (same commit); `CLAUDE.md` checks; `05.0` §6 (`rh-protocol` built; `LiveEdge` in `rh-stream`); `docs/09` V-STR-01, V-STR-03 and V-STR-05 method lines with the test names; BCK-002 A-42 (its stream-host part recorded) |

**Rules carried from P0–SPR-013 (every task):**
- Property tests vary sizes and seeds: frame sizes from 0 to the 1 MiB limit, viewer counts, seal delays.
- **A positive control fails for the right reason;** its `check_all` line matches the message (`I1 broken`, `STALE:`, the named error code).
- **Every probe exercises its path:**
  - the V-STR-05 sleeper must actually be sent behind;
  - the resume test must actually see `future_seq`;
  - the stalled viewer must actually receive `stream.behind`.

  Each test asserts that it happened.
- **Tests make their run directories under a short temporary path** they create themselves, so the 107-byte socket limit never depends on the runner's working directory. The refusal itself is tested with a planted long path.
- **Python tests read no CI environment variable.**
- **No statistical test here.** Wall-clock figures are reported, never gated, except probe (b)'s liveness deadline. That deadline is generous: it detects a hang, not a slowdown.
- **CI time:** each CI part stays at or under 12 minutes (15 is the hard limit), and the times are reported.
- **Merge `origin/main` when main moves.**
- **Every new gate is in exactly one `check_all.sh` part,** with the inventory test updated in the same commit.
- **No new third-party crate besides `arc-swap`,** which is pre-approved (R22).
- **One commit per task.**

## Tasks

#### T-01: `rh-protocol`: frames, control messages, the dispatcher and `Catchup` (§2–§5, §7; R6, R7, R8, R9, R11, R15, R21, R24)
- **Deliverable:** crate `crates/rh-protocol`, labelled (tool, tool) and pinned. It depends on `serde` and `serde_json` only. Its modules follow §7:
  - **`wire`:** the 12-byte header, the 1 MiB and 4 KiB limits, `decode` returning the bytes consumed or `Incomplete`.
  - **`records`:** §2.2's sub-header and its prefix checks only. The kind-to-channel table is a `const` copied from `canonical_v0.py`'s `CHANNEL` and checked against the canonical golden file.
  - **`control`:**
    - canonical JSON;
    - `deny_unknown_fields` on envelopes and request parameters, and not on results;
    - a float refused as `invalid_params`;
    - nesting depth at most 4;
    - `dispatch`, whose method table holds exactly the three reads, with §4.4's refusals.
  - **`cursor`:** `Catchup`, with §7's events, actions and states, and the reconnect-pending flag.
- **Acceptance:**
  - **The golden file.** `crates/rh-protocol/tests/golden.rs` against `specs/stream/golden/wire_v1.txt`:
    - every `frame` line is encoded byte for byte and decoded back;
    - every `refuse` line is refused with its named reason;
    - every `dispatch` line is answered byte for byte on a fresh connection;
    - the `scenario catchup` transcript is reproduced line for line, with R15's four orderings in it.
  - **Round trips.** A property test over random frames and control messages: SplitMix64 with fixed seeds, payloads from 0 bytes to the limit, at least 10⁴ cases.
  - **The method table** is exactly `{stream.status, stream.subscribe, stream.unsubscribe}`, each a read (§4.5's invariant).
  - **Planted controls, each failing by name:**
    - a dispatcher with a fourth method (`stream.pace`) fails the method-table test;
    - a decoder that accepts a nonzero flag fails its `refuse` line.
  - **The existing lint gates hold over the real crate:**
    - `protocol-leaf` and the plane-A closure are green on the workspace;
    - their three fixtures stay red.
- **Status:** [x] done (branch `sprint/SPR-026-stream-v1`; awaiting the Tester)

#### T-02: `rh-stream`'s `LiveEdge`: the writer never waits (§6.1, §6.2; R2, R3, R4, R16, R17, R18, R22; A-42)
- **Deliverable:** `LiveEdge` in `rh-stream`.
  - **The snapshot.** One `ArcSwap` (`arc-swap`) holds an immutable snapshot of `(w0, next_seq, s, last_year, chunk list, seal log)`.
  - **The API:**
    - `snapshot()`;
    - `read(from, max) -> Result<Lines, Behind { s, last_year, w0 }>`, which copies from the snapshot's own chunks only;
    - `bytes()`;
    - the seal log.
  - **The writer's publication order** (I4): a chunk is appended; a seal lands and `s` is published; a year is dropped and `w0` moves.
  - **Window policies.** Option (b), the year-end drop with a late seal dropping its year when it lands, is the default (D42, approved). Option (a) is a host setting.
  - **Sealing:**
    - Under (b), sealing runs off the host thread. If SPR-013's writer seals on the host thread, this task moves it.
    - The latency of every seal is logged against its sim-year's wall time.
  - **Resume and abort:**
    - A resumed host starts with an empty window at the first seq of the first year not on disk, with `s` and `last_year` read from disk (R18). This builds on SPR-013 T-09.
    - A poisoned host never seals a partial year (A-42).
  - **`LiveEdge` is built only under `--socket`.**
- **Acceptance:**
  - **Probe (b) of §9: deterministic, pass or fail.**
    - The setup: a serving thread is frozen inside `LiveEdge::read` on a test barrier across a year-end drop, and a second connection is never read.
    - The writer must append and seal 10 sim-years before the deadline, with `bytes()` within the policy's bound.
    - Released, the frozen reader returns exactly `[c, c + n)`, or `Behind`.
  - **The seal race, reproduced in Rust** from the reference's check: every interleaving of a reader's loads with one seal's steps and one drop's steps, enumerated exhaustively. None shows `s + 1 < w0` through the snapshot API. A drop between a snapshot and its copy leaves the copy carrying exactly the seqs it claims.
  - **Bounds:**

    | Policy | Peak window |
    |---|---|
    | (b), every seal within a sim-year | At most 2 sim-years |
    | (b), a planted seal k sim-years late | 2 + ⌈k⌉ sim-years: the reference's figures, at least at k = 1.5 and 2.5 |
    | (a) | 1 sim-year |

  - **The `window-gap` control:** a writer that moves `w0` before the dropped year is on disk fails the I1 check with `I1 broken`.
  - **Headless runs are unchanged:** V-PERF-04 and V-PERF-11 stay green on every rung, and no `LiveEdge` is built.
  - **`loom`** is recommended, not required (R3, R22), and needs the Architect's crate review before it is added.
- **Status:** [x] done (branch `sprint/SPR-026-stream-v1`; awaiting the Tester)

#### T-03: The host: `serve.rs` and `A.sock` (§1, §4, §6.2; R11, R12, R13, R14, R20, R21, R24)
- **Deliverable:** `rh-cli run <world> --socket`, with every socket in `crates/rh-cli/src/serve.rs`:
  - **Files and locking:**
    - the run directory is mode `0700`;
    - `stream/t<id>/A.sock` is mode `0600`;
    - `A.lock` is held with `flock` for the host's life, through `std::fs::File::lock` (stable on the pinned 1.97 toolchain; no new crate);
    - a stale socket is removed under the lock;
    - a socket path over 107 bytes is refused, naming the path.
  - **Connections:**
    - the cap is 64 (`too_many_viewers`, retriable);
    - each connection gets its own serving thread;
    - control frames are dispatched through `rh-protocol`;
    - RECORDS frames are byte ranges of `LiveEdge`, at most 1 MiB each.
  - **Notifications are pulled from the seal log:**
    - `stream.sealed`, including the empty-year form;
    - `stream.behind`;
    - `stream.end` with `finished` or `aborted`.
  - **`run_nonce`** comes from wall-clock nanoseconds: never a pid alone, never in plane A.
  - **Waiting** is a poll with a short sleep, so the writer never blocks.
  - **`stream.status`** answers §4.1's fields from one snapshot: pace band `max`, and state `running` or `finished`.
- **Acceptance:**
  - **Probe (c): the ladder with viewers attached equals the ladder without,** on `solo` and `hamlet`, in the observer-invariance style of `rh-snapshot --test observer`.
  - **Read-only (§9):**
    - `player.act`, `timeline.fork`, `timeline.run`, `oracle.whisper` and `stream.pace` sent on `A.sock` are each answered `method_not_found`, with the read-only message;
    - the journal's row count and chain head are unchanged;
    - the ladder is unchanged.
  - **The V-IF-02-style fuzz.** The corrupt frames are seeded from the reference's `refuse` and `dispatch` lines:
    - a wrong magic, version 2, type 3, flag 1;
    - more than 4 KiB, a truncated frame;
    - a RECORDS frame from a viewer;
    - a bad prefix;
    - a JSON batch, a float seq;
    - depth 5, an unknown member;
    - 100,000 nested arrays.

    Each gets its named error, and the host closes the connection. The host does not panic, the run continues, and the ladder is unchanged.
  - **Locking:** a second host on the same run directory is refused by the lock, and a stale socket left by a killed host is removed by the next one.
  - **Abort:** a planted poisoned engine ends every subscription with `stream.end{aborted}`, and the unsealed year stays unsealed.
  - **Lints:** `network` and `network-reexport` are green on the workspace and red on their existing fixtures.
- **Status:** [x] done (branch `sprint/SPR-026-stream-v1`; awaiting the Tester)

#### T-04: Channel C live (§6.4; R5, R13, R20)
- **Deliverable:** `rh-cli sift <run-dir> --follow --socket`.
  - The sifter keeps sifting as channel-A segments seal.
  - It serves `C.sock`, holding `C.lock`, through the same `serve.rs`.
  - An empty year is sent as `stream.sealed{last_seq: null}`.
  - `stream.status` carries `sifted_through`.
- **Acceptance** (§9 "Channel C live"):
  - **The test viewer:** subscribed on `C.sock` from seq 0, reading to `stream.end` through sim-years with no match.
  - **Content:**
    - its lines equal `rh-cli tail --canonical` over the sealed channel-C segments;
    - every empty year arrives with `last_seq: null` and the digest of the empty string;
    - `sifted_through` never exceeds channel A's `sealed.last_seq`.
  - **Isolation:**
    - the state ladder is unchanged (V-PLN-04);
    - while only `run --socket` serves, no `C.sock` exists: the engine process never serves channel C.
- **Status:** [x] done (branch `sprint/SPR-026-stream-v1`; awaiting the Tester)

#### T-05: Viewers: `rh-cli tail --live` and `--to-seq`, and the V-STR probes (§6.3, §9; R8, R15, R21, R23)
- **Deliverable:**
  - **`rh-cli tail <run-dir> --live [--channel A|C]`** drives `rh_protocol::Catchup` over the socket. Its client code is in `serve.rs` (see the specs table). It reads sealed ranges with `rh-stream`'s reader.
  - **`rh-cli tail --to-seq N`** prints exactly the canonical lines of `[from, to]`. This is the sealed-read path for stdlib viewers (R23).
  - **A Rust test harness of viewers,** in `tests/`:

    | Viewer | Behaviour |
    |---|---|
    | Edge follower | Follows the live edge |
    | Playback | Reads at a scripted 100× playback |
    | Rewinder | Re-subscribes from old seqs |
    | Stalled | Connected, never reads |
    | Sleeper | Stops reading until a second connection's `stream.status` shows two more sealed years |
    | Jumper | Jumps to live from 10 years behind |

- **Acceptance:**
  - **V-STR-01.** `solo` and `hamlet` run headless, then with the first four viewers attached. Every sealed segment's `canonical_digest` is equal byte for byte, and the ladder equals the baseline.
  - **V-STR-03.** In the same run, channel A is unchanged. Wall time with and without viewers is reported; the gate is SPR-030's V-PERF-09.
  - **V-STR-05, catch-up:**
    - the sleeper is sent behind, and a probe asserts it received `stream.behind` at least once;
    - it then reads the sealed years through `tail --canonical --from-seq --to-seq`, resubscribes and continues to `stream.end`;
    - its per-year XXH3-128 over the bodies it received equals every `stream.sealed` digest and every `segment.canonical_digest`;
    - its seqs are exactly `0..next_seq`.
  - **V-STR-05, jump to live:** `first_seq = live.next_seq`, with nothing duplicated after it.
  - **V-STR-05, resume.** The host is killed with `kill -9` mid-year while a seal is in flight, then resumed from the 10-year keyframe (SPR-013 T-09). The viewer:
    - receives `future_seq`, backs off and retries;
    - verifies the resumed year against its `stream.sealed`;
    - ends with the same seqs and digests.

    `Catchup` reports no protocol error.
  - **The stalled viewer's first read after the run is `stream.behind`.**
  - **Ingredients for SPR-030's V-PERF-09,** printed term by term:
    - the stalled consumer;
    - `LiveEdge::bytes()`;
    - the seal-latency distribution;
    - the per-viewer frame cap;
    - the host's live bytes, counted with SPR-009's counting allocator.

    No gate is applied here (R23).
- **Status:** [x] done (branch `sprint/SPR-026-stream-v1`; awaiting the Tester)

#### T-06: Gates, docs and CI time
- **Deliverable:**
  - **Part `core`:**
    - `rh-protocol`'s golden, method-table and round-trip tests;
    - `LiveEdge` probe (b), the seal-race tests and the window-bound tests;
    - the host's read-only and fuzz tests.
  - **Part `golden`:** the V-STR-01 and V-STR-05 runs on `solo`. `hamlet` joins them if the part stays at 12 minutes or less; otherwise `hamlet` runs nightly, by the Architect's ruling recorded in the audit.
  - **The inventory test,** in the same commit.
  - **The docs** of the compliance table.
- **Acceptance:**
  - Every gate is green, and each new gate is in exactly one part.
  - The `check_report.md` of both parts and the CI times are in the PR: each part at 12 minutes or less, 15 at most.
  - If `hamlet` moved to the nightly tier, the nightly runs it on the PR head before the audit.
- **Status:** [x] done (branch `sprint/SPR-026-stream-v1`; awaiting the Tester)

## Director items, and what changes under each alternative

| Item | Built (the spec's recommendation) | Under the alternative |
|---|---|---|
| **D42**, the live-edge window and PERF-5's wording (75%) | Option (b), the year-end drop with an off-thread seal: a window of the current and the previous sim-year, plus the lateness of a seal slower than a sim-year, logged. Option (a) is built too, behind a host setting (§6.1) | **(a):** the setting becomes the default. The seal runs on the host thread once per sim-year, and the window is one sim-year. Probe (b) and the seal-race tests stand. The V-PERF-09 RSS term becomes one year. Code: one default and the bound asserted in T-02. **(c), a record-count tail:** the reference does not simulate it (§6.1). That needs a spec round (reference, golden, I1 at a partial year) before any code, so the sprint ships (b) and (c) becomes a later C2 change |
| **D43**, canonical lines on the live edge, amending ADR-0031 §1 item 4 (75%) | RECORDS frames carry canonical lines (§2.2) | **Arrow IPC with LZ4:** §2.2, the reference and `golden/wire_v1.txt` must be rewritten first (a C2 spec round). `rh-protocol`'s `records` module and the `protocol-leaf` allow-list change: `arrow-ipc` and an LZ4 crate, after crate review (R7). The window holds pre-encoded IPC batches, so serving threads still copy bytes. V-STR-05's per-year digest check needs a batch-to-lines decode on the viewer. stdlib Python viewers lose the live edge and need `pyarrow`. T-01 to T-04's control plane, `Catchup`, `LiveEdge`'s snapshot rule and the host are unaffected. **Recommendation:** have D43 ruled at this plan's approval. If it is not ruled, T-01's `records` module and T-05's digest checks wait, and the rest proceeds |
| **D47**, a `docs/11` text fix: tip and scheduled inputs belong to P2 (80%) | Nothing: this sprint journals no input and serves no `control.sock` (§8; R12) | Leaving `docs/11` as it is changes nothing here; SPR-027's scope would say otherwise |

## Out of scope (explicit)
- **SPR-027 (P1.17 b):** the materializer (V-STR-02), the pace command and `control.sock`, `tail` filters beyond the person filter SPR-013 built, the regional digest, and the Textual TUI stub.
- **The V-PERF-09 gate** on the 10k dynastic world (SPR-030; D34). This sprint supplies its ingredients only.
- **Inputs.** Tip and scheduled inputs and any journaled input on a socket (P2; BCK-002 A-36; R12).
- **Other transports.** Windows and macOS, named pipes, TCP (§1).
- **`rh-server` and the WebSocket control plane** (P2; R13).
- **Server-side filters** (v1 has none: §4.2).
- **Frame-level checksums** (§1: the transport is reliable).
- **A-42's poison rule for consumers other than the stream host** (`snapshot` and `state_hash` on a poisoned engine).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | T-01, T-03 and T-04, written from `wire_v1.md` §1–§5, §7 and §8 and from the golden file, before reading the code:<ul><li>the golden conformance, with a codec of their own;</li><li>the fuzz of corrupt frames, extended beyond the golden lines;</li><li>the read-only invariant and the method table;</li><li>socket modes and the lock;</li><li>channel C live with empty years;</li><li>a stdlib Python viewer of their own, built on the reference's `Viewer`, against the real host: `tail --to-seq` for the sealed reads, and lines equal to the Rust viewer's</li></ul> | `VER-SPR-026-TEST` |
| AGT-003-DET | T-02 and T-05, written from §6, ADR-0030 and `05.10` §7:<ul><li>probe (b);</li><li>the seal race, with their own interleavings;</li><li>the window bounds under slow seals;</li><li>V-STR-01, V-STR-03 and V-STR-05, with the `kill -9` resume;</li><li>the ladder unchanged under viewers;</li><li>headless allocation and instruction counts unchanged</li></ul> | As above |

## Merge route

- **Class and conditions.** C2, merged by the Architect after the Tester pass and the audit (GOV-005 §6). No golden change, so nothing is announced beyond the merge summary.
- **Not bound by the one-C3 rule.** The sprint can merge between SPR-014 and SPR-020 or after both. GOV-005 §4.1's "one merge at a time" still applies: the Architect updates the next waiting branch from `main` and lets CI re-run.
- **Shared files.** SPR-026 and SPR-020 both edit `rh-stream`: this sprint adds `LiveEdge` beside the writer, and SPR-020 changes what the writer puts in the envelope tables. They also share `check_all.sh`, the inventory test and `CLAUDE.md`. Whichever merges second merges `origin/main` and resolves the textual conflicts.
- **If SPR-020 lands first,** this sprint's V-STR-01 runs against SPR-020's new baselines. `golden/wire_v1.txt` needs no regeneration, because its lines come from `golden/canonical_v0.txt`, already amended by PR #61.
- **Recommended order:** after SPR-014, and before or after SPR-020 by readiness (7/10).

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-013 not merged | Open |
| 2 | D43 (the RECORDS payload) | **Closed:** approved 2026-10-17 |
| 3 | D42 (the window bound) | **Closed:** option (b) approved 2026-10-17 |
| 4 | The `tail --live` name, the client's file, and `LiveEdge` only under `--socket` (specs table) | **Closed:** ruled by the Architect (specs table) |
| 5 | The director's plan approval (GOV-005 §2) | **Closed:** approved 2026-10-17 (D48) |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-026` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] The merge route matches the CI-detected change class (C2), and no file under `worlds/*/baselines/` changed.
- [ ] Rung checkpoint (GOV-007 §3.5): when `tail --live` first follows `hamlet` live, the director may watch it for 15–30 minutes. The Architect records the notes as `EVO-` or `DEF-` items.
