---
id: SPR-027
title: "World Stream v1, part b: the materializer, the control endpoint and the pace command, tail filters and the digest, the Textual TUI stub (P1.17, part b)"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint, stream]
related: [BCK-002, PRJ-002, PRJ-003, REV-SPR-026, SPR-013, SPR-018, SPR-020, SPR-026, SPR-030, SPR-033, SPR-035, ADR-0024, ADR-0025, ADR-0030, ADR-0031, ADR-0035, ADR-0036]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF:** Finishes World Stream v1 (`05.10`; `docs/11` P1.17) on top of SPR-026's live edge:
> - **the materializer** (V-STR-02): `rh_golden::materialize(run_dir, tick)`, the nearest permanent keyframe plus deterministic replay, giving the exact `state_hash` and a readable dump at any published tick, exposed as `rh-cli materialize`;
> - **the control endpoint:** `<run-dir>/stream/t<id>/control.sock` with `rehoboam/stream/control/v1` (`control.status`, `control.pace`, `control.pause`, `control.resume`), the pace governor in the host's run loop, the run log; the stream sockets stay read-only;
> - **`rh-cli tail` filters:** `--place`, `--pattern`, `--min-tellability`, and `--digest` (the world digest from `STEP_DIGEST`);
> - **the Textual TUI stub:** a read-only status view over the stream, the first Python package of the uv project, V-PERF-09's third viewer.
>
> No hash and no channel-A byte changes; no baseline moves. Pace never changes history, and the sprint proves it (V-STR-01 at two paces).
>
> - **Developer:** AGT-002-TOOLS (`rh-golden`, `rh-cli`, the Python package, the spec text); AGT-002-CORE is consulted for `rh-snapshot::resume` if the materializer needs a new entry point.
> - **Testers:** AGT-003-IF (the control plane, the filters, the TUI, the hidden-state lint) and AGT-003-DET (V-STR-02's equality, pace invariance, observer invariance). One session may hold both, on a different model from the Developer's.
> - **Size and class:** about 3 agent-days. **C2** (IF-0 gains a socket and a versioned method set; `05.9` IF-0 v1.5) containing C1 (the materializer, the filters, the TUI).
> - **Status:** PLANNING. It needs the Architect's rulings in the specs table and the director's plan approval (D54; GOV-005 §2). **ADR-0036 and SPR-033 start when this sprint lands** (D51); "What SPR-033 consumes" says what they take from it.

# SPR-027 — World Stream v1, part b (WP P1.17, part b)

- **Roadmap item:** `docs/11` P1.17 (b) · **Phase / rung:** P1a, rungs 0–2
- **Depends on:**
  - **SPR-026 merged:** `rh-protocol` (the frame codec, canonical JSON, the dispatcher, `Catchup`), `rh-cli/src/serve.rs` and `run --socket`, `LiveEdge`, `tail --live --to-seq`, the viewer harness, the V-STR-01/03/05 probes this sprint extends.
  - **SPR-013 merged:** keyframes in the run directory with `KEYFRAME` markers, the sealed segments, `tail`, the `kill -9` resume line (T-09).
  - Independent of SPR-014, SPR-015, SPR-016 and SPR-020 beyond `main` having moved: the materializer replays whatever systems the world registers; the filters read the frozen prefix and the fields channel A v0 already has. If SPR-020 is on `main`, `--place` has real `place_id`s on the full-envelope stub world; if not, on a fixture.
- **Binding contracts:**
  - `05.10` §1, §3, §4 (the fidelity promise, the materializer), §5 (transport: `control.sock`), §6 (filters and the digest), §7 items 2, 4, 5, §7a; `05.9` IF-0 (v1.4; this sprint writes v1.5) and IF-3's JSON-RPC shape; `05.1` §4.4 (the speed bands).
  - `specs/stream/wire_v1.md` §2, §3, §4.4, §4.5, §5 (the carrier the control endpoint reuses) and §8 (what is reserved for it); `specs/stream/canonical_v0.md` §1 (the frozen prefix) and §3 (`STEP_DIGEST`, `KEYFRAME`, `EVENT`, `MATCH` fields); `channel_a_v0.sql`, `channel_c_v0.sql`.
  - The new `specs/stream/control_v1.md` (T-01; the Architect approves before T-03).
  - ADR-0024, ADR-0025, ADR-0030 (items 3, 5), ADR-0031 §4, ADR-0035 §6, ADR-0036 §1 and §5 (the consumer).
  - `docs/09` V-STR-01 ("at any pace"), **V-STR-02**, V-STR-03, V-STR-05, V-PERF-09 (ingredients: the TUI stub), V-IF-02 (style), V-DET-19, V-INSP-03, V-INSP-05, V-PLN-01, V-PLN-04, V-GOLD-01 (unchanged).
- **System spec:** n/a.
- **Rulings (binding):** REV-SPR-026 R10 (header filters by a split; the stdlib sealed-read path), R12 (A-36 to P2: neither SPR-026 nor SPR-027 journals an input; pace is run-log only), R13 (sockets in `serve.rs`; no `rh-server` before a consumer), R23 (V-PERF-09 is SPR-030's; this sprint supplies its third viewer); the Architect's text rulings at SPR-026's plan (`tail --live`; the client in `serve.rs`); the Architect's rulings at this plan (the specs table).
- **Director items:** D42, D43, D47 (approved 2026-10-17) and D51 (ADR-0036: SPR-033 after this sprint). This plan raises none; it names two Architect text fixes.
- **Requirements covered:** STR-2 (the materializer rebuilds the exact state); STR-3 (viewing is a buffer event: a third viewer, the TUI); STR-5 (the pace command part: a control command, logged in the run log, never the journal; the Watch/Play auto-pause part is P2); PERF-5 (the ingredients; the gate is SPR-030's); EVO-4 (the TUI is downstream and in Python; the engine only engines).

## Specs this sprint needs

| Spec | Exists? | Gap | Proposed Architect ruling | Needed by |
|---|---|---|---|---|
| **`control_v1.md`: the control endpoint's method set** | `wire_v1.md` §8 reserves the socket, its version string, and says "its methods are SPR-027's and P2's to specify" | Missing: the method table, the band tokens, the state machine, the errors, the run-log line, golden lines | **The sprint writes `specs/stream/control_v1.md` first (T-01), reusing wire v1's framing, canonical JSON, error model and version rule: `control.status` → `{version, pace: {band, state}, tip: {tick, seq}}`; `control.pace {band}` with `band` one of `live`, `1h`, `1d`, `1w`, `1mo`, `1y`, `10y`, `max`; `control.pause`; `control.resume`; every other method `method_not_found`; a float, an unknown band or an extra member `invalid_params`. Each accepted command appends one run-log line `pace <wall_ns> <band> <state>`; nothing is journaled. Golden `dispatch` lines join `selftest_wire_v1.py` as a `control` section. The Architect approves it in the PR (C2)** (7/10) | T-01, T-03 |
| **Where the materializer lives** | `05.10` §1: "Materializer: keyframe + deterministic replay"; `rh_snapshot::resume` exists (plane A); `rh-golden` (tool, tool) builds worlds from manifests, runs, keyframes and dumps; PRJ-003 §2 lets `rh-view` depend by path on `rh-protocol`, `rh-stream`, `rh-worldgen`, `rh-snapshot` | Silent on the crate; the viewer's path list omits the crate that can build a world | **`rh_golden::materialize` (a library module beside `run.rs` and `dump.rs`), exposed as `rh-cli materialize`; PRJ-003 §2 invariant 3's path list gains `rh-golden`** (7/10). Alternative: a new `rh-materialize` tool crate, which would duplicate `rh-golden`'s world builder for one consumer | T-02 |
| **V-STR-02's "random ticks"** | `docs/09` V-STR-02: "at 50 random ticks per run, rebuild the state and compare `state_hash`" | The engine publishes a `state_hash` only at Month-pulse instants (`STEP_DIGEST`) and keyframes | **The ticks are drawn by fixed-seed SplitMix64 from the run's `STEP_DIGEST` records, plus every `KEYFRAME` marker; the reference hash is the record's** (8/10). A daily `state_hash` has no published reference in P1a |
| **V-STR-02's lint clause** | "a lint that every plane-A write goes through the event API or the replayed step functions"; `05.10` §7 item 2 | No such lint is named; `rh-core` already confines writes to `SystemView::write` under the `AccessSpec`, the core insert and compaction, and `apply_inputs` | **Discharged by the existing write path plus one new `bin/lint_forbidden.py` rule, `hidden-state`: no `static mut`, `thread_local!`, `OnceLock`, `OnceCell`, `LazyLock` or `Atomic*` in plane-A `src/` except under a counted `LINT-ALLOW(hidden-state)`; planted controls both ways. The materializer equality is the evidence; the rule closes the one route a write could take around it** (6/10) | T-02 |
| **What a run directory records for a later materialize** | `rh-cli run` writes the ladder, keyframes, `stream/`, the journal; the ladder header carries the history tuple | A materializer needs the world name or manifest path, the seed, the horizon, the thread count and the Registry it ran under | **`rh-cli run` writes `<run-dir>/run.toml`: world, manifest path and SHA-256, `registry_version`, seed, horizon, threads, the `not_built` list; the pace log is `<run-dir>/run.log`. A materialize refuses a directory whose `run.toml` names a manifest whose SHA differs from the file's** (7/10) | T-02, T-03 |
| **The pace governor's placement** | `05.1` §4.4: "speed bands change only the publish cadence and how far `advance_until` goes per wall interval"; `05.10` §7 item 4 | Where the sleeping happens | **In `rh-golden`'s run loop (tool plane): after each instant the host sleeps until the sim-time elapsed since the band was set, at the band's rate, is at or under the wall time elapsed; `max` never sleeps; `paused` holds the loop between instants; `live` is one sim-minute per wall-second. Plane A sees nothing: the clock lives in the host, and `lint_forbidden`'s clock rule stays green** (8/10) | T-03 |
| **`--digest region`** | `05.10` §6: "the world digest (`--digest region`), which prints the per-step population, prices and Ψ per region" | **Gap.** Channel A v0 has one world-level `STEP_DIGEST` (`living`, `births`, `deaths`, the three hashes); regions arrive with SPR-018 (wave 6), prices with M4 and Ψ with M6 (P1b) | **`--digest` prints the world digest table in P1a. `--digest region` is refused with "regions arrive with SPR-018"; the per-region split is derived viewer-side from kind 7 `RESIDENCE_INTERVAL` records and the life records once SPR-018 lands, as a BCK-002 item owned by SPR-018 or the next tools sprint. No new channel-A record kind here** (7/10). `05.10` §6's sentence gets a note |
| **`--sig θ`** | `05.10` §6: "filter: sig ≥ 0.6" | **Gap.** Channel C v0's `MATCH` carries no significance (P2.1, `05.4`) | **`--min-tellability N` on `EVENT.tellability` is the P1a filter; `--sig` waits for channel C's significance** (7/10) | T-04 |
| **`--place <id>`** | `EVENT.place_id` exists and is NULL in every P1a world until SPR-018 | Testable on real worlds only after wave 6 | **Built and tested on a stub world that emits places (SPR-020's full-envelope stub, or a fixture); on the rungs it filters everything out until SPR-018** (8/10) | T-04 |
| **The TUI stub's home and its dependencies** | `docs/11` P1.17: "a read-only status view over the stream"; `docs/07`: Textual, Adopt; `python/pyproject.toml`: a virtual project "until the first Python package lands"; `wire_v1.md` §2.2: the stub stays stdlib for the live edge | Where the code goes, and whether `python -I -m` can run it | **The first Python package of the uv project: `python/rehoboam_tui/`, `package = true` with a build backend, `textual` pinned as a dependency, `uv lock` regenerated and committed (GOV-008 §2). Its protocol client is standard library: a port of the reference's `Catchup` and frame codec, checked against the reference's `scenario catchup` transcript. It never opens `control.sock`** (6/10) | T-05 |
| **The pace-invariance probe's band** | V-STR-01: "at any pace (MAX vs 1 yr/s)" | 1 yr/s on `solo`'s 200 years is 200 wall-seconds | **The PR tier compares `max` with `10y` on `solo` for 50 years (5 wall-seconds); the nightly sweeps every band on 10 sim-years, `live` capped at 60 wall-seconds** (8/10) | T-03 |
| **Seek latency** | ADR-0030 and ADR-0036 "revisit when V-STR-02 shows replay-based reconstruction is too slow for interactive seeking"; PRJ-003 V-VIEW-05 (≤ 250 ms for a 20-year scrub, reported) | No figure exists | **Reported, not gated: the materialize time at the farthest published tick from a keyframe (up to ten years of replay) on `solo` and `hamlet`, printed in the PR and the audit as SPR-035's baseline** (8/10) | T-02 |
| **`docs/03` STR-5's phase** | STR-5 says P2; `wire_v1.md` §8 and PRJ-002 give the pace command to SPR-027 (P1a) | A text inconsistency | **Architect text fix: STR-5's phase reads "P1a (the pace command: SPR-027) / P2 (Watch and Play auto-pause)"** (7/10) | T-06 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C2**: IF-0 gains `control.sock` and the method set `rehoboam/stream/control/v1` (`05.9` IF-0 v1.5), a new interface. Contains C1: the materializer, the filters, the TUI, `run.toml`. `state_hash`, channel A and channel C bytes are unchanged on every world |
| Planes touched (ADR-0024) | **A:** `rh-core` and `rh-snapshot` untouched unless the materializer needs a new `resume` entry point (then an additive one, reviewed by AGT-002-CORE). **Tool:** `rh-golden` (`materialize.rs`, the governor, `run.toml`), `rh-cli` (`serve.rs`: `control.sock`; `materialize`; `tail` filters), `rh-protocol` (the `control` method table, a second dispatcher), `python/rehoboam_tui/`, `specs/stream/control_v1.md`, `selftest_wire_v1.py`, `bin/lint_forbidden.py` (`hidden-state`). **B:** none. **C:** `tail --pattern` reads channel C; nothing written |
| Owned columns written | None |
| Reads (via `rh-contracts`) | None. The materializer reads plane-A tables through `rh-snapshot`'s `EngineState` and `rh-golden`'s `Dump`, read-only |
| RNG purposes added | None. The governor's clock is the host's wall clock, never in plane A |
| Golden-hash effect | **None.** The ladder at any pace, with the TUI attached, and after a materialize equals the ladder without. `worlds/*/baselines/` are unchanged |
| Fixed-point quantities | None. The digest table prints the integers of `STEP_DIGEST` |
| Determinism lints (GOV-003 §2, §6) | Must pass. `network` and `network-reexport` hold: every socket in `serve.rs`; `protocol-leaf` holds (`rh-protocol` gains no dependency); the new `hidden-state` rule is green on the workspace and red on its two planted controls |
| Tests that gate this sprint | **V-STR-02** (new), V-STR-01 (two paces; the TUI attached), V-STR-03 (the TUI as a fifth viewer), V-STR-05 (the TUI's catch-up), V-IF-02 (style: the control endpoint under fuzz), V-PLN-04 (a materialize and a `tail` write nothing), V-DET-19, V-INSP-03, V-INSP-05, V-PLN-01, V-GOLD-01 (unchanged), V-PERF-04 and V-PERF-11 (unchanged headless at `max`), plus `bin/check_all.sh` |
| Docs updated in the same sprint | `bin/check_all.sh` and `python/tests/test_check_all_inventory.py` (same commit); the `CLAUDE.md` checks; `05.10` §4 (the materializer's API and `run.toml`), §5 and §7a (`control.sock` v1), §6 (the filters as built, the digest note); `05.9` IF-0 v1.5; `05.0` §6 (`rh-golden` hosts the materializer; the Python package); `docs/09` V-STR-02 and V-STR-03 method lines with the test names; `docs/03` STR-5 (the phase split); `docs/07` (Textual pinned); PRJ-003 §2 (the path list); BCK-002 (the regional digest and significance items); `python/README.md` |

**Rules carried from P0–SPR-026 (every task):**
- **Property tests vary sizes and seeds:** ticks across the whole run, keyframe distances from 0 to 10 years, control payloads from empty to the 4 KiB limit; fixed-seed SplitMix64; no new Rust crate.
- **A positive control fails for the right reason;** its `check_all` line matches the message.
- **Every probe exercises its path, and asserts it:** the materialize really replayed across the input's instant; the governor really slept; the TUI really received `stream.behind`; the paused host really held the tip.
- **Tests make their run directories under a short temporary path** (the 107-byte socket limit).
- **Python tests read no CI environment variable.**
- **No statistical test here.** Wall-clock figures are reported, never gated, except liveness deadlines generous enough to detect a hang.
- **CI time:** each part at or under 12 minutes (15 hard), reported; T-06 says what moves to the nightly tier.
- **Merge `origin/main` when main moves.**
- **Every new gate is in exactly one `check_all.sh` part,** with the inventory test updated in the same commit.
- **No new third-party Rust crate.** `textual` is a Python dependency of the uv project (`docs/07`: Adopt).
- **One commit per task.**

## Tasks

T-01 comes first (no code before its spec). T-02 and T-04 depend only on SPR-013 and SPR-026. T-03 follows T-01's approval. T-05 follows T-03 (it displays the pace). T-06 comes last.

#### T-01: `specs/stream/control_v1.md` and its reference lines (the specs table's first row; `wire_v1.md` §8; STR-5)
- **Deliverable:** the contract: the socket and its mode, the version string, the four methods with their parameters and results, the band tokens of `05.1` §4.4, the state machine (`running` ⇄ `paused` → `finished`), the errors (wire §4.4's codes), the run-log line, the read-only restatement ("nothing here is journaled; a stream socket never gains a method that changes anything"), and what P2 adds (tip and scheduled inputs, forks; D47). `selftest_wire_v1.py` gains a `control` section: the dispatcher's method table, canonical `dispatch` lines in `golden/wire_v1.txt` (or a sibling `golden/control_v1.txt`), the refusals.
- **Acceptance:** `selftest_wire_v1.py --check` passes with the new lines; a planted fifth method on the control table and a planted `control.pace` on the stream table each fail the method-table checks by name; the Architect approves the text in the PR before T-03 starts (recorded in the PR body).
- **Status:** [ ] not started

#### T-02: The materializer (V-STR-02; `05.10` §4; ADR-0030 item 3)
- **Deliverable:**
  - **`rh-cli run` writes `run.toml`** (the specs table).
  - **`rh_golden::materialize(run_dir, tick) -> Result<Materialized, Error>`:** reads `run.toml`, re-pins the manifest, builds the world as `setup` does (the same systems, the same `not_built` list), finds the nearest permanent keyframe at or before `tick` through the `KEYFRAME` markers (genesis when none), resumes through `rh_snapshot::resume` with the journal, advances to `tick` with no stream writer attached (it emits nothing and seals nothing), and returns `{tick, state_hash, dump: Dump}` with the table roots. It refuses a tick with no published hash unless `--any-tick` is given (then it returns the hash with no reference to compare).
  - **`rh-cli materialize <run-dir> --tick T [--dump <file>] [--hash-only]`**, and `--at-digest <seq>` to name a `STEP_DIGEST` record instead of a tick.
  - **The `hidden-state` lint** in `bin/lint_forbidden.py` with its two planted controls.
- **Acceptance:**
  - **V-STR-02 on `solo` (200 years) and `hamlet` (150 years):** at 50 SplitMix64-chosen `STEP_DIGEST` ticks per run and at every `KEYFRAME` marker, the materialized `state_hash` equals the record's; where the tick is a yearly rung, the dump's table roots equal the ladder's. The probe asserts that at least one chosen tick lies after `solo`'s journaled input (year 53) and at least one is nine or more years past its keyframe.
  - **Resume path.** A run killed with `kill -9` mid-year and resumed (SPR-013 T-09's line) materializes to the same hashes at the same ticks.
  - **Planted controls, each failing by name:** a materializer that skips the journal (`mutant-materialize-no-journal`) fails at the first tick after `solo`'s input; one that starts from genesis for every tick gives equal hashes but trips the probe's "replayed from the nearest keyframe" assertion; a stub system with a `static` counter read in its step fails the `hidden-state` lint, and (in the stub world) the materializer's hash differs from the run's at the first digest after the counter diverges, which is the lint's reason.
  - **Read-only (V-PLN-04).** A materialize writes no segment, no journal row and no keyframe; the run directory's file list and SHA-256s are unchanged after it.
  - **Reported:** the materialize time at the farthest tick from a keyframe on each world, and the keyframe sizes read.
- **Status:** [ ] not started

#### T-03: `control.sock`, the pace governor and the run log (T-01's contract; `05.1` §4.4; `05.10` §3, §7 item 4; REV-SPR-026 R12, R13)
- **Deliverable:**
  - **`serve.rs` serves `control.sock`** (mode `0600`, `control.lock`, the 107-byte refusal, one connection at a time or the cap, as the Architect rules in T-01) through a second `rh-protocol` dispatcher whose method table is exactly the four control methods; a stream method on it, or a control method on `A.sock` or `C.sock`, is `method_not_found` with each socket's message.
  - **The governor** in `rh-golden`'s run loop (the specs table): band and state as a shared cell the serving thread writes and the loop reads between instants; `max` by default; `paused` holds the tip; `stream.status.pace` reports the band and state from the same cell.
  - **The run log** `<run-dir>/run.log`: one line per accepted command with the wall-clock nanoseconds and the resulting band and state; the journal is never touched (a test counts its rows and checks its chain head).
- **Acceptance:**
  - **Pace invariance (V-STR-01, "at any pace"):** `solo` for 50 years at `max`, then at `10y` (about 5 wall-seconds), then with a `pause` of two wall-seconds at year 20 and a `resume`: the three ladders are byte-identical, every sealed segment's `canonical_digest` is equal, and the journal is unchanged. The probe asserts that the `10y` run took at least 4 wall-seconds and that the pause held the tip (the `stream.status` tick did not move during it).
  - **The V-IF-02-style fuzz** of the control endpoint, seeded from the reference's refusal lines: wrong magic and version, a RECORDS frame, a float band, an unknown band, an extra member, depth 5, 100,000 nested arrays, a 5 KiB control frame: each gets its named error, the connection closes, the host does not panic, the run continues and the ladder is unchanged.
  - **Read-only, both ways:** `control.pace` on `A.sock` and `stream.subscribe` on `control.sock` are `method_not_found`; `player.act` on `control.sock` is `method_not_found` ("P2"); the journal is unchanged after every test.
  - **Lints:** `network` and `network-reexport` green (every socket in `serve.rs`); `lint_forbidden`'s clock rule green (the governor's clock is in `rh-golden`, tool plane); `protocol-leaf` green.
  - **Headless cost:** V-PERF-04 and V-PERF-11 unchanged at `max` with no socket (the governor is one branch per instant).
- **Status:** [ ] not started

#### T-04: `rh-cli tail` filters and the digest (`05.10` §6; REV-SPR-026 R10)
- **Deliverable:** `--place <settlement-or-region id>` on `EVENT.place_id`; `--pattern <pattern_id>` on channel-C `MATCH` lines; `--min-tellability N` on `EVENT.tellability`; `--digest` printing one row per `STEP_DIGEST` (the world date, `living`, `births`, `deaths`, `state_hash`), `--digest region` refused with its reason (the specs table). Every filter composes with SPR-013's `--follow <person>`, `--kind`, `--from-seq`, `--to-seq`, `--canonical` and SPR-026's `--live`, and each reads only the frozen prefix plus the one field it needs (a split, never a full parse, except `--canonical`).
- **Acceptance:**
  - On a stub world with places, matches and tellabilities (SPR-020's full-envelope stub, or a fixture that writes canonical lines through `rh-stream`), each filter prints exactly the records that satisfy it, once each, in `seq` order; two filters print the intersection; sealed and live (`--live`) runs print the same lines.
  - On `hamlet`, `--min-tellability 100` prints `DEATH` (128) and, once SPR-015 is on `main`, `MARRIAGE` (112) and never `BIRTH` (96); `--pattern life_ends` prints the sifter's matches as the channel-C segments hold them; `--digest` prints one row per Month pulse with `living` equal to the life-course count, and `insta` snapshots pin the display form.
  - `tail` writes nothing (the SPR-013 T-10 read-only test, repeated with every filter).
  - Measured per-line cost of a filter (a split) is reported beside SPR-026's 0.3 µs.
- **Status:** [ ] not started

#### T-05: The Textual TUI stub (`docs/11` P1.17; V-PERF-09's third viewer; REV-SPR-026 R10, R23)
- **Deliverable:** `python/rehoboam_tui/` (the specs table): a stdlib protocol client (a port of the reference's frame codec and `Catchup`); a Textual app with a status line (the tip's date and tick, pace band and state from `stream.status`, the cursor, the lag, the history tuple), a digest panel (the last 24 `STEP_DIGEST` rows), a sealed-years panel with digests (from `stream.sealed`), an event tail (header split only: tick, kind, verb name; a bounded ring of 200 lines), and a matches panel when `C.sock` exists; `--headless --until-end` and `--headless --seconds N` for CI; a `--stall-after <year>` test switch. It opens `A.sock` and, if present, `C.sock`, and never `control.sock`.
- **Acceptance:**
  - **The client equals the reference:** the reference's `scenario catchup` transcript is reproduced line for line by the port.
  - **Observer invariance (V-STR-01, V-STR-03):** `solo` with the TUI attached headless to `stream.end` gives the baseline ladder; its seqs are contiguous and its per-year XXH3-128 over the bodies equals every `stream.sealed` digest.
  - **Catch-up (V-STR-05):** with `--stall-after 10` the TUI's next read is `stream.behind`, it reads the sealed years through `tail --canonical --from-seq --to-seq` and continues to `stream.end` with the same digests.
  - **Read-only:** a test asserts the set of sockets the process opened; `control.sock` is not among them, and the journal is unchanged.
  - **Cost, reported:** the stub's CPU per line at the G1a pace on one pinned core (SPR-026 measured 0.3 µs for a split and 14.7 µs for a full parse); the stub stays under 1 µs per `EVENT` line.
  - `ruff` and `mypy` clean on the package; `uv lock` committed; `uv sync --frozen` succeeds.
- **Status:** [ ] not started

#### T-06: Gates, docs, CI time, and the handover to SPR-033
- **Deliverable:**
  - **Part `core`:** `selftest_wire_v1.py --check` with the control section and its controls; the control-table tests and fuzz; the `hidden-state` lint and its controls; the filters' snapshots on the stub world; the TUI package's unit tests (the transcript, the sockets opened).
  - **Part `golden`:** V-STR-02 on `solo` (50 ticks); the pace-invariance triple on `solo` for 50 years; the TUI headless attach on `solo`. `hamlet`'s V-STR-02 and its TUI attach join the part if it stays at 12 minutes or less; otherwise they run nightly, by the Architect's ruling recorded in the audit.
  - **Nightly:** V-STR-02 at 500 ticks on all three rungs; the band sweep (every band on 10 sim-years, `live` capped at 60 wall-seconds); the TUI stalled for 10 minutes (V-STR-03's clause).
  - **The inventory test,** in the same commit; the docs of the compliance table; **the "What SPR-033 consumes" section below, kept current in the PR body.**
- **Acceptance:** every gate green, each in exactly one part; the `check_report.md` of both parts and the CI times in the PR, each part at 12 minutes or less (15 at most). Estimate: core +1 min, golden +1.5 min.
- **Status:** [ ] not started

## What SPR-033 (and SPR-035) consume from this sprint

The director's D51 starts the voxel viewer lane when this sprint lands. SPR-033 itself (the Bevy scaffold, the mesher, the procedural demo diorama) reads no engine data; what it takes from here is the schedule gate and the contracts its successors bind to:

| Consumer | What it takes | Where it is fixed |
|---|---|---|
| SPR-033 (the spike) | The landing of this sprint (D51's start condition); the materializer's API shape to design `rh-view`'s state model against; the seek-latency figures as V-VIEW-05's baseline | T-02, T-06; `05.10` §4 |
| SPR-034 (stream ingestion) | Nothing new: `rh-protocol`'s `Catchup` and `rh-stream`'s reader are SPR-026's. The `tail` filters are its oracle for "what happened here" | SPR-026; T-04 |
| SPR-035 (the scrubber) | `rh_golden::materialize(run_dir, tick) -> Materialized { tick, state_hash, dump }`, with typed reads of the materialized tables (`Dump`, or `rh-contracts` accessors over the resumed `SystemView` if the Architect rules so when SPR-035 is planned); the 10-year keyframe cadence and the `KEYFRAME` markers; `run.toml`; the rule that a materialize writes nothing | T-02; `05.10` §4; PRJ-003 §2 (the path list gains `rh-golden`) |
| SPR-037 (the avatar, P2) | `control.sock` as the viewer's one writeable surface: its framing, version rule and method table, which P2 extends with tip and scheduled inputs (D47) | T-01, T-03; `control_v1.md` |

## Director items, and what changes under the alternative

| Item | Planned | Under the alternative |
|---|---|---|
| None for the director. D42, D43, D47 and D51 are decided and built on | — | — |
| Architect text fixes (not director items): `docs/03` STR-5's phase split; `05.10` §6's `--digest region` and `--sig` notes; PRJ-003 §2's path list; `05.1` §4.4 gains the `live` definition (one sim-minute per wall-second) | Made in T-06 | If the Architect prefers `05.10` §6 unchanged, the two filters are built as "accepted and refused with a reason" and the BCK-002 items carry the follow-ups |

## Out of scope (explicit)
- **Tip and scheduled inputs, forks, `control.sock`'s `player.*`, `timeline.*`, `oracle.*`** (P2.7; D47; BCK-002 A-36: no input is journaled here).
- **Watch and Play auto-pause** (STR-5's second half; `05.4` §2.3's suspense: P2).
- **A per-region digest, prices and Ψ in the digest** (SPR-018 and P1b; the specs table), **significance filters** (P2.1).
- **`rh-tui` in Rust** (P2.7, ratatui) and **`rh-server`** (P2; REV-SPR-026 R13).
- **The V-PERF-09 gate** on the 10k dynastic world (SPR-030; D34). This sprint supplies the third viewer and its cost.
- **Delta records for hot columns** (ADR-0030's "revisit when"): only if the reported seek latency is unacceptable to SPR-035.
- **Any engine change:** `rh-core`, `rh-events`, `rh-snapshot` are untouched unless the materializer needs an additive `resume` entry point.
- **The voxel viewer itself** (SPR-033 onward, PRJ-003).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | T-01, T-03, T-04, T-05, written from `control_v1.md`, `wire_v1.md` §2–§5 and §8, `canonical_v0.md` §3 and `05.10` §6 before reading the code:<ul><li>the control method table and the read-only property of all three sockets, with a dispatcher of their own and a fuzz extended beyond the golden lines;</li><li>the run log's lines and the journal's row count and chain head after every command;</li><li>each filter against a DuckDB query over the sealed segments (the twin: the lines `tail` prints are exactly the rows the query returns);</li><li>the TUI's client against the reference's transcript, its socket set, its catch-up;</li><li>the `hidden-state` lint with variants of their own (a `static` behind a macro, a `thread_local!` in a dependency-free helper)</li></ul> | `VER-SPR-027-TEST` |
| AGT-003-DET | T-02, T-03, T-05, written from `05.10` §4 and §7, `05.1` §4.4, ADR-0030 and ADR-0025:<ul><li>V-STR-02 with their own tick selection and an independent comparison of dumps against the ladder's roots;</li><li>the resume path materialized;</li><li>pace invariance across the bands, with their own timing probes;</li><li>observer invariance with the TUI and a materialize running beside the host;</li><li>the headless instruction and allocation counts unchanged;</li><li>the nightly band sweep on the head</li></ul> | As above |

## Merge route

- **Class and conditions.** C2, merged by the Architect after the Tester pass and the audit (GOV-005 §6). No golden change, so nothing is announced beyond the merge summary.
- **Not bound by the one-C3 rule.** The sprint merges in any gap between the wave's C3 merges (SPR-020, SPR-015, SPR-016), after SPR-026. GOV-005 §4.1's "one merge at a time" still applies.
- **Shared files.** With SPR-026 (its successor on the same files): `serve.rs`, `rh-protocol`, `rh-golden/src/run.rs`, `selftest_wire_v1.py` and the golden file; this sprint builds on the merged state. With SPR-015 and SPR-016: `check_all.sh`, the inventory test, `CLAUDE.md` only. With SPR-020: the stub world T-04 uses, if SPR-020 lands first; otherwise a fixture. Whichever merges second resolves the textual conflicts.
- **Recommended order:** after SPR-026 and before SPR-015 if it is ready by then (7/10), so that SPR-033's lane starts as early as D51 allows; otherwise between SPR-015 and SPR-016.
- **What this sprint re-verifies after merging `origin/main`:** both CI parts; V-STR-02 against whatever baselines and segments are then on `main` (its reference hashes are the run's own, so a baseline move elsewhere changes nothing here); the `tail` snapshots if SPR-015's `BIRTH` and `MARRIAGE` lines have arrived.

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-026 not merged | Open |
| 2 | SPR-013 not merged (keyframes in run directories; `tail`) | Open |
| 3 | The Architect's rulings of the specs table, above all the materializer's home and `control_v1.md`'s shape (before T-01 and T-02) | Open |
| 4 | The Architect's approval of `control_v1.md` in the PR (before T-03) | Open |
| 5 | The director's plan approval (D54; GOV-005 §2) | Open |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-027` is PASS, with the seek-latency and viewer-cost figures recorded for SPR-030 and SPR-035.
- [ ] No open DEF- against this sprint.
- [ ] The merge route matches the CI-detected change class (C2); no file under `worlds/*/baselines/` changed.
- [ ] `control_v1.md` is approved; `05.9` IF-0 is v1.5; `docs/11` P1.17 is complete and the viewer lane (SPR-033) may start (D51).
- [ ] Rung checkpoint (GOV-007 §3.5): when the TUI stub first follows `hamlet` live at a chosen pace, the director may watch it for 15–30 minutes; the Architect records the notes as `EVO-` or `DEF-` items.
