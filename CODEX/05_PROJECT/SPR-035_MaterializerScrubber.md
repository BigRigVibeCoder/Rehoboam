---
id: SPR-035
title: "The materializer hookup and the timeline scrubber: unwatched jumps through SPR-027's materializer (the run's own rh-cli, locally or on adjutant over SSH), the watched-history cache (D62), rewind equal to forward play (V-R3D-13) and the scrub latency reported (V-VIEW-05)"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint, observability, stream]
related: [PRJ-003, ADR-0024, ADR-0025, ADR-0030, ADR-0031, ADR-0036, SPR-013, SPR-027, SPR-030, SPR-033, SPR-034, SPR-038, D51, D62]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF:**
> - **What.** The director can scrub through a run's history.
>   - **The timeline scrubber:** a bar of sim-years with the keyframes, the watched ranges and the live tip, and three modes:
>     - **live** follows the tip;
>     - **history** holds a past tick while the run goes on;
>     - **play** advances through history at a chosen rate.
>
>     Nothing the scrubber does reaches the run.
>   - **The watched-history cache** (D62): the view state at every tick the viewer has received, as checkpoints and a delta log, plus a content-addressed store of hydrated chunks and meshes. Scrubbing back within watched history needs no materializer and no new I/O. It is plane B, never authoritative, in RAM by default, and may spill to `/dev/shm`.
>   - **Unwatched jumps through SPR-027's materializer.** The viewer runs the run's own `rh-cli materialize` as a subprocess, on the laptop or on `adjutant` over the SSH connection SPR-034 shares. It verifies the dump's plane digest against the published `STEP_DIGEST` hash itself, then folds forward to the exact tick. While the bar is dragged through unwatched history, previews snap to keyframes (no replay); the exact tick is materialized on release.
>   - **V-R3D-13's last clause:** rewind through the materializer, forward play, and scrubbing back through the cache give the same content digest at fixed ticks.
>   - **V-VIEW-05, as PRJ-003 v1.3.0 restates it:** within watched history, ≤ 250 ms from the cache (the red team's aim is one frame); into unwatched history, reported against world size.
> - **Honest expectation.**
>   - Within watched history: one or two frames on a cache hit, and well under 250 ms when a settlement must be re-hydrated.
>   - Unwatched jumps: sub-second on the rungs, and **seconds at 10,000 people**. A jump there is a keyframe plus up to 10 sim-years (about 3,600 instants) of replay: 4–15 s at today's per-instant cost, and an order of magnitude less if SPR-038's targets hold. 250 ms is not reachable for them by any cache the plan allows; keyframe prefetch and keyframe-snapped previews are the mitigation.
> - **Who.** Developer: AGT-002-TOOLS. Testers: AGT-003-DET and AGT-003-IF, one session, on a different model from the Developer's. The Red Team audits.
> - **Size and class.** About 3 agent-days. **C1.**
> - **Status:** PLANNING. It needs the Architect's approval as a C1 plan (GOV-005 §2), and goes to the director in D63.

# SPR-035 — The materializer hookup and the timeline scrubber

- **Roadmap item:** `PRJ-003` §4 (SPR-035); `docs/11` viewer lane V.1 · **Phase:** viewer lane (priority 2, D62; tool plane)
- **Developer:** `AGT-002-TOOLS`
- **Testers:** `AGT-003-DET` and `AGT-003-IF` (one session, on a different model from the Developer's). The Red Team audits.
- **Depends on:**
  - **SPR-034 merged:** run sources (local and remote), catch-up, `view_state_v0`, grammar v0, hydration, the digests.
  - **SPR-027 merged**, specifically T-02:
    - `rh_golden::materialize(run_dir, tick)` and `rh-cli materialize <run-dir> --tick T | --at-digest <seq> [--dump <file>] [--hash-only]`;
    - the state after Bookkeeping of `T` (W37);
    - the refusal of a tick with no published hash;
    - the rule that a materialize writes nothing;
    - `rh_golden::dump::Dump` with `parse` and `digest`;
    - the reported seek time at the farthest tick from a keyframe, which is this sprint's baseline.
  - **Measured on, not required:** SPR-038 (per-instant cost) and SPR-030 (`twin_vales` and `kingdom_10k` runnable). Without them the 10k figure is a projection, and the PR says so.
- **Binding contracts:**
  - ADR-0036 §1–§4; ADR-0030 (items 3 and 5: the materializer and the buffer); ADR-0025 (scrubbing never acts; an act in the past forks, and the viewer has none);
  - `05.10` §1, §4 (the fidelity promise, the materializer); PRJ-003 §2, §3 (the watched-history cache row), §5 (V-VIEW-05);
  - D62 (the cache; `/dev/shm` allowed for it, authoritative runs keep disk and fsync);
  - `docs/09` V-R3D-13, V-STR-02 (the materializer's equality, which this sprint relies on and does not re-prove);
  - `rehoboam-dump v1` (`crates/rh-golden/src/dump.rs`).
- **Requirements covered:**
  - R3D-11 (the same content whether the tick is reached by rewind or by forward play);
  - STR-2 (the exact state at any tick, consumed);
  - V-VIEW-02 and V-VIEW-05 (PRJ-003 §5).

## Specs this sprint needs

| Spec | Exists? | Gap | Proposed (for the Architect's ruling) | Needed by |
|---|---|---|---|---|
| **Where the replay runs** | PRJ-003 §2 lets the viewer link `rh-golden` and call `rh_golden::materialize` in process (D-7) | `engine_build` is `rh-golden <version>` plus the built systems (`crates/rh-golden/src/world.rs`), so it does not name a commit: the viewer cannot tell from the history tuple whether its own linked engine replays the run's history. A remote run's directory is on `adjutant`, not on the laptop | **Default:** the run's own `rh-cli materialize` as a subprocess, on the machine that holds the run (local, or over SSH on `adjutant`'s eight cores). The viewer recomputes `Dump::digest()` and compares it with the published hash. **`--materialize in-process`:** `rh_golden::materialize` on a local run, allowed only after the viewer's linked engine has reproduced the `state_hash` of the run's nearest keyframe, and refused by name otherwise (F1) | T-01 |
| **The dump on standard output** | SPR-027 T-02: `--dump <file>` | A remote materialize needs the dump on stdout and the report elsewhere | Ask SPR-027 T-02 for `--dump -`, which writes only the dump to stdout, with the report on stderr. Until then the viewer runs `rh-cli materialize … --dump "$tmp" && cat "$tmp" && rm "$tmp"` through SSH (F2) | T-01 |
| **What the cache holds** | D62: "a cache of the history it has shown", plane B, `/dev/shm` allowed | Key, bound, eviction, memory budget; materialized state or hydrated content | The section "The watched-history cache" below: view-state checkpoints and a delta log, and hydrated content addressed by its inputs. Neither dumps nor per-tick voxels (F3) | T-02 |
| **Households at materialized ticks** | A dump holds `person.household` (after SPR-015) and M7's columns (after SPR-021); the stream does not (SPR-034 F1) | Whether the scrubber may show them | Not in grammar v0. A materialized tick may show a household panel in the HUD, labelled "from the materialized state at <tick>", never in the content digest (F4) | T-03 |
| **The scrub latency's measurement** | PRJ-003 §5 V-VIEW-05; SPR-027 T-02 reports a seek time | How it is measured and reported | `--bench scrub` (T-04): from the input event to the first frame whose bubble is fully meshed and uploaded, by class, with the world's living count and where the replay ran | T-04 |

## The watched-history cache (D62)

**What it holds, and why.** Three things could be cached:

| Option | For | Against |
|---|---|---|
| (a) Materialized states (`Dump`s) at watched ticks | Exact, every table | Several MB per tick at 10,000 people (the text form about 7 MB). Caching every watched tick is impossible, so scrubbing between cached ticks needs a fold anyway |
| (b) Hydrated voxel content per tick | Display is immediate | About 60 MB of raw voxels per bubble per tick. It is tied to one bubble and one camera, and it is wasteful because content changes rarely |
| **(c) The view state, as checkpoints and a delta log, plus hydrated content addressed by its inputs** | **Small** (KB to tens of MB). **Exact** for every watched tick. **Independent of the camera.** Repeated configurations of a settlement hydrate once | A content miss costs a re-hydration (tens of ms per settlement) |

**Recommendation: (c)** (75%). A dump enters the cache only as the source of a checkpoint: the viewer converts it to `view_state_v0` at once and drops the dump.

**The two layers.**

1. **The history layer.**
   - **Key:** the run's history digest, `XXH3-128(engine_build, registry_version, manifest_hash, world_seed)`, with `timeline_id` and `view_state_v0`'s version.
   - **Entries:**
     - **checkpoints** `V(t)`: every keyframe tick (each 10 sim-years) of every watched decade, plus yearly checkpoints for the 20 sim-years either side of the last scrub position;
     - **the delta log:** the records the view state reads (`LIFE_ENTRY`, `LIFE_EXIT`, `BIRTH`'s place), per sim-year, as packed integers;
     - **the coverage:** the seq ranges the viewer has received and verified against their sealed digests ("watched").
   - **Lookup:** the nearest checkpoint at or before `t`, plus at most 10 sim-years of deltas. At 10,000 people that is about 8,000 deltas, well under a millisecond.
   - Every checkpoint records its origin: a verified fold over seq ranges with digests, or a materialized state with its `state_hash`.
2. **The content layer.**
   - **Key:** `(terrain_v0, grammar_v0_e1, settlement id, XXH3-128 of the grammar's effective inputs)`. Grammar v0 reads residents only through the dwelling count `ceil(residents / 5)` and the church flag, so the inputs hashed are the seat, the routes, that count and that flag. A settlement has as many entries as its distinct village forms (at most 14 for 60 residents: 13 dwelling counts, one of them with and without the church).
   - **Entries:** the settlement's chunks (run-length encoded `u16` voxels) and their meshes, per level.

**Budgets and eviction.**

| Layer | Default budget | Eviction |
|---|---|---|
| History | 256 MiB of RAM | Yearly checkpoints farthest from the cursor first; then decadal checkpoints, oldest first. Deltas go last: when they alone pass half the budget, the oldest sim-years outside the visible part of the bar are dropped and marked unwatched, so a jump there goes through the materializer |
| Content | 512 MiB of RAM | Least recently drawn first; the current bubble's entries are pinned |

**The memory it needs** (`view_state_v0` is about 40 bytes per living person, a delta about 16–41 bytes):

| World | Living | Watched span | History layer | Content layer (all populations of the focus settlement) |
|---|---|---|---|---|
| `hamlet` | 60 | 200 years | under 1 MB | about 20–40 MB (one entry per distinct dwelling count) |
| `twin_vales` | about 1,500 | 300 years | about 3 MB | about 50 MB per settlement |
| `kingdom_10k` | 10,000 | 1,000 years | about 80 MB: 100 decadal checkpoints at 0.4 MB, 40 yearly, deltas about 22 MB | bounded by the budget |

The P51 has 32 GB, Bevy uses 1–2 GB, and the bubble's meshes live on the GPU's 4 GB.

**`/dev/shm` (opt-in, D62).**
- **Turning it on:** `--cache-dir /dev/shm/rh-view-$UID` lets evicted checkpoints and content entries spill to tmpfs, up to `--cache-shm-max` (default 2 GiB). Ubuntu sizes `/dev/shm` at half of RAM, 16 GB on the P51.
- **Each file** carries a header: format, history digest, generator versions, and the payload's XXH3-128. A file whose header or payload does not match is deleted and recomputed.
- **Cleanup:** the directory is `0700`, and it is removed at exit unless `--keep-cache`. A kept cache is reused only by a viewer of the same history.
- **What it buys:** a viewer restart keeps the watched history.
- **What it never is:** authoritative. Every entry can be re-derived by a fold or a materialize. Nothing in it is read by plane A or written into the run directory, and authoritative runs keep their disk and fsync (D62).

**Prefetch (on by default; `--prefetch none` turns it off).** When the viewer is idle, it materializes keyframe ticks outward from the cursor, nearest first, one at a time, and stores each as a decadal checkpoint. It stops at the history budget. Remote prefetch runs on `adjutant`'s CPU with `nice`. After a pass, previews while dragging through unwatched history are immediate.

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C1.** `viewer/` and its job. One additive request to SPR-027's CLI (F2), which is SPR-027's to make |
| Planes touched (ADR-0024) | **Tool:** `rh-view-core` (`materialize`, `cache`, `timeline`), `rh-view` (the scrubber), the job. **B:** the cache, in RAM and optionally `/dev/shm`. **A:** read through `rh-cli materialize` (a subprocess that writes nothing, SPR-027 T-02) or, opt-in, `rh_golden::materialize`; never written. **C:** none |
| Owned columns written | None |
| Reads (via `rh-contracts`) | None. The dump's tables `person.*` and `life_course.*` (and `person.residence` after SPR-018), read through `rh_golden::dump::Dump` |
| RNG purposes added | None |
| Golden-hash effect | **None** on any engine ladder. `viewer/golden/hydrate_v0.txt` is unchanged by this sprint: the same ticks give the same digests by every path, which is the point |
| Fixed-point quantities | None |
| Determinism lints | SPR-033's. The cache uses `BTreeMap` only. Wall-clock reads (latency measurement, eviction by last draw) live in `rh-view`'s timing module, with use-site allows; no clock value enters content or a cache key |
| Tests that gate this sprint | V-R3D-13's rewind clause by the materializer and the cache (T-04); the cache's integrity tests (T-02); the materializer client's digest check and refusals (T-01); the planted controls; `bin/check_viewer.sh`. **Reported:** V-VIEW-05 |
| Docs updated in the same sprint | `CLAUDE.md`; `docs/09` V-R3D-13's method line (rewind); PRJ-003 (status; §3's cache row with the budgets as built); `viewer/README.md` (the scrubber, the cache, `/dev/shm`, the remote materializer); RUN-001 (the scrub protocol) |

**Rules carried from SPR-033 and SPR-034 (every task):** a positive control fails by name; a probe asserts its path (the jump really materialized, from the nearest keyframe; the scrub really hit the cache, and no subprocess ran); no wall-clock gate; one commit per task; merge `origin/main` when it moves.

## Tasks

#### T-01: The materializer client (0.7 day)
- **Deliverable (`rh-view-core::materialize`):**
  - **`Materializer::Subprocess` (the default):**
    - **Local:** `<rh-cli> materialize <run-dir> --at-digest <seq> --dump -`. **Remote:** the same through SSH (`ssh <host> …` with SPR-034's `ControlMaster`).
    - **The binary:** `<rh-cli>` is `--rh-cli PATH` locally and `--remote-rh-cli PATH` remotely. Before its first use, `rh-cli build-info` must print `mutants: none`, or the viewer refuses it by name.
    - **The target tick.** For a requested tick `T` the viewer:
      1. reads the sealed year that holds `T` through its source;
      2. takes the last `STEP_DIGEST` at or before `T` (seq `d`, tick `m`, hash `h`), or genesis (`--tick 0`) when there is none;
      3. materializes at `d`;
      4. parses the dump with `Dump::parse`;
      5. checks that `Dump::digest()` equals `h`, refusing by name if not, because a transport, toolchain or build difference has shown up;
      6. converts it to `view_state_v0` (`from_dump`: the living rows of `person.*`, their `life_course` fields, `person.residence` when present, otherwise the genesis export's settlement for founders and nothing for the rest);
      7. folds the records of `(m, T]` from the same sealed year.
    - **Keyframe previews:** `--tick K` at a `KEYFRAME` marker's tick, with no replay.
    - **Cancellation:** a request is cancelled when the cursor moves on (the subprocess, or its remote command, is killed). At most one materialize runs at a time.
  - **`Materializer::InProcess` (`--materialize in-process`, local runs only):**
    - calls `rh_golden::materialize` (`rh-golden` joins as a path dependency, PRJ-003 §2);
    - is refused by name unless a check succeeds first: materializing the run's nearest keyframe tick in process must give the `KEYFRAME` record's `state_hash` (F1).
  - **The genesis cross-check:** once per run, the materialized tick-0 state (`--tick 0`) gives the founders' settlements; when `person.residence` exists (SPR-018 on `main`), they must equal the genesis export's. This is the engine checked against the reference the viewer has shown since SPR-033.
- **Acceptance** (in the viewer job, on the debug `rh-cli` and a 30-year `hamlet` run directory):
  - **Equality:** at 20 SplitMix64-chosen ticks, `from_dump` plus the fold equals SPR-034's fold from genesis, as structures and as content digests.
  - **The path is real:** a probe asserts the replay started from the nearest keyframe (SPR-027's report) and that at least one tick lies 9 or more sim-years past its keyframe.
  - **The digest check:** a planted `mutant-dump-flip` (one byte of the received dump flipped) is refused by name.
  - **The binary check:** a planted `rh-cli` whose `build-info` reports a mutant is refused by name.
  - **The remote path,** through SPR-034's `ssh` stub, gives the same view states.
  - **The in-process path** gives the same view states as the subprocess on the same run. Its refusal is tested with a planted mismatch (`mutant-inprocess-stale`).
  - **Read-only:** the run directory's tree and SHA-256s are unchanged after 20 materializations (SPR-013 T-10's form).
  - **Reported:** the materialize time by keyframe distance (0, 2, 5 and 9 sim-years) on `solo`, `household` and `hamlet`, and the dump's size and parse time.
- **Status:** [ ] not started

#### T-02: The watched-history cache (0.8 day)
- **Deliverable (`rh-view-core::cache`):** the two layers, keys, budgets, eviction, `/dev/shm` spill and prefetch of the section above; the coverage set of watched, verified seq ranges; `--cache-budget-history`, `--cache-budget-content`, `--cache-dir`, `--cache-shm-max`, `--keep-cache`, `--prefetch`.
- **Acceptance:**
  - **Exactness:** for 200 SplitMix64 ticks in watched history, the cached `V(t)` equals a fresh fold, and the content from the cache equals a fresh hydration (digests).
  - **No subprocess on a hit:** a probe counts materialize calls during 100 watched scrubs, and the count must be 0.
  - **Eviction under a small budget** (1 MiB history, 4 MiB content):
    - the order is as specified;
    - an evicted watched year becomes unwatched, and a scrub there materializes (the probe asserts it);
    - the result is still exact.
  - **`/dev/shm`** (run against a temporary tmpfs-like directory in CI):
    - a file with a changed byte, a wrong history digest or a wrong generator version is deleted and recomputed, never used (each asserted);
    - the directory is `0700`, and it is removed at exit unless `--keep-cache`;
    - a kept cache is reused only by the same history.
  - **Planted controls, each failing by name:**
    - `mutant-cache-neighbour` returns the checkpoint of the following sim-year; it fails exactness;
    - `mutant-shm-trust` skips the header check; it fails the tampered-file test.
  - **Reported:** the memory per layer after `hamlet`'s 200 years, and a projection for `kingdom_10k` from the per-person sizes.
- **Status:** [ ] not started

#### T-03: The timeline scrubber (0.7 day)
- **Deliverable (`rh-view`; `rh-view-core::timeline`):**
  - **The bar,** along the bottom of the window:
    - sim-years from genesis to the tip;
    - keyframe ticks;
    - watched ranges shaded, and prefetched keyframes dotted;
    - the cursor and the live tip;
    - a hover showing the date.
  - **Modes:**

    | Mode | Behaviour |
    |---|---|
    | **Live** | The cursor follows the tip on the live edge |
    | **History** | The cursor holds a past tick; the HUD shows "viewing <date>, tip <date>". The run continues |
    | **Play** | The cursor advances from a past tick at 1 sim-month, 1 sim-year or 10 sim-years per second, folding forward. It reads the cache in watched history and sealed records beyond, never the materializer after its starting point |

  - **Keys:** `←`/`→` one sim-month; `Shift` one sim-year; `PgUp`/`PgDn` one keyframe (10 sim-years); `Home` genesis; `End` or `L` live; `Space` play or pause the playback (not the run).
  - **Dragging into unwatched history:**
    - previews snap to keyframe ticks (prefetched ones are immediate);
    - on release the exact tick is materialized, with a progress line and an estimate from the last materialize's time per sim-year;
    - a new position cancels the pending one.
  - **The household panel (F4):** at a materialized tick, the HUD can list the focus settlement's households from the dump, labelled with that tick. It is not drawn into the content.
  - **Read-only:** no scrub, play or pause sends anything to the run. Pacing the run stays `rh-cli pace` over SSH (SPR-027).
- **Acceptance:**
  - **The mode machine** is tested without a GPU, on scripted inputs: live → history → play → live, with the cursor's ticks asserted at each step, and the run's stream unaffected (the client's request set is unchanged).
  - **Keyframe snapping:** during a scripted drag only keyframe ticks are requested; on release exactly one exact materialize runs.
  - **Cancellation:** a second release before the first completes kills the first (the probe asserts the subprocess ended).
  - **The headless app test** (SPR-033's) drives the bar through a scripted scrub and asserts that the displayed chunks' digests equal the view state's hydration at each stop.
- **Status:** [ ] not started

#### T-04: Rewind equals forward play (V-R3D-13), and the scrub latency (V-VIEW-05) (0.6 day)
- **Deliverable:**
  - **The V-R3D-13 test, on `hamlet`'s 30-year run directory:** at the end of sim-years 1, 5, 10, 20 and 30, and at 10 SplitMix64 ticks between them, the bubble's content digest by five paths:

    | Path | How the tick is reached |
    |---|---|
    | **F**, forward | SPR-034's live fold from seq 0, digest recorded as the tick passes |
    | **S**, re-read | SPR-034's batch fold through the sealed segments |
    | **R**, rewind by the materializer | Materialize at the last `STEP_DIGEST` at or before the tick, then fold forward |
    | **K**, rewind from the keyframe | Materialize at the `KEYFRAME` at or before the tick, then fold forward |
    | **C**, scrub back through the cache | Scrub from the tip back to the tick |

    The paths are run at 1 and 4 hydration threads and in debug and release builds of the test.
  - **`--bench scrub`, the scrub-latency protocol (V-VIEW-05):**
    - **The scenario:** against a finished run, of which the bench watches only the last third (`--watch-from`, by default two thirds of the way through), so the first two thirds are unwatched: 100 scrubs to SplitMix64 ticks in watched history, and 20 jumps into unwatched history at keyframe distances 0, 2, 5 and 9 sim-years, with prefetch off.
    - **The measure:** from the input event to the first frame whose bubble is fully meshed and uploaded.
    - **The report:** per class (cache hit; content miss; unwatched at each distance), the p50, p95 and maximum; the world's living count; where the replay ran; and, for the unwatched class, its parts (`rh-cli`'s time, transfer, parse and digest, the fold, hydration, upload). The report is written to `--out`.
- **Acceptance:**
  - **Equality:** `F = S = R = K = C` at every tick, thread count and build. The planted controls fail by name:
    - `mutant-materialize-early`, which materializes at the `STEP_DIGEST` before the right one, fails R;
    - `mutant-fold-from-keyframe-twice`, which folds the keyframe's own instant again, fails K.
  - **The latency report** is in the PR, from the viewer job (headless: the data path without upload, reported) and from the director's P51 (the full measure).
  - **V-VIEW-05's within-watched figure** is reported against 250 ms. A p95 above 250 ms is reported with its parts, never hidden; it is not a gate (D62).
- **Status:** [ ] not started

#### T-05: The job, the protocol and the docs (0.2 day)
- **Deliverable:**
  - **`bin/check_viewer.sh`** gains T-01 to T-04's tests and controls, and runs `hamlet`'s materialize probes with the debug `rh-cli`.
  - **The workflow:** the `main` trigger list is unchanged from SPR-034's (`crates/**` already covers `rh-golden`).
  - **The docs** of the compliance table, with the commands below.
- **Acceptance:** the viewer job is green; its times are reported against SPR-034's.
- **Status:** [ ] not started

**Size.** T-01 0.7, T-02 0.8, T-03 0.7, T-04 0.6, T-05 0.2: 3.0 agent-days. **Cut order** if it runs long:
1. `/dev/shm` spill (0.15 day): RAM only, which D62 permits.
2. Prefetch (0.15 day): previews then snap to keyframes at their materialize cost.
3. The in-process materializer (0.1 day).
4. The household panel (0.05 day).

The five-path equality and the cache's exactness tests are never cut.

## Unwatched jumps: the latency to expect (projections; T-01 and T-04 measure the runnable worlds)

| World | Living | A jump's replay | Projected materialize-and-show | Basis |
|---|---|---|---|---|
| `solo`, `household` | 1–5 | ≤ 10 sim-years (about 3,600 instants) | well under 0.5 s | SPR-027 T-02 reports the figure; release `solo` runs 200 years in under 1 s |
| `hamlet` | 60 at genesis | ≤ 10 sim-years | under 0.5 s | as above |
| `twin_vales` | about 1,500 | ≤ 10 sim-years | about 1–3 s | interpolated; runnable after SPR-030's genesis |
| `kingdom_10k` | 10,000 | ≤ 10 sim-years | **4–15 s today; about an order of magnitude less if SPR-038's targets hold** | SPR-038: a quiet instant at N = 10⁴ takes 1.12 ms at L = 10⁴ and 3.99 ms at L = 1.1 × 10⁵, and 3,600 instants make 4–14 s. Its target is ≤ 20,000 instructions per quiet instant. Add about 0.3–1 s for the keyframe, the dump (about 7 MB of text, compressed by SSH) and its parse |

**What this means for V-VIEW-05.** The 250 ms aim holds within watched history (the cache), and for unwatched jumps on the rungs. At 10,000 people it does not hold for unwatched jumps: the replay alone is seconds. Prefetched keyframes make dragging immediate at keyframe granularity, and the exact tick follows in seconds. If that is too slow for the director, the remedy is ADR-0030's "revisit when": delta records for hot columns, or keyframes every sim-year for viewed runs. That is an engine decision with storage and C3 consequences, not a viewer one.

## How the director runs it (as planned)

**A local run:** SPR-033's two terminals. The viewer now finds `rh-cli` beside the run:
```bash
target/viewer/release/rh-view --world hamlet --run-dir ~/rh-runs/hamlet-1 --rh-cli target/release/rh-cli
```

**A run on `adjutant`:** SPR-033's terminal 1 (the `ssh -t` session that holds the run and the tunnel), then:
```bash
target/viewer/release/rh-view --world hamlet \
    --remote adjutant --remote-run-dir "$REMOTE_HOME/rh-runs/hamlet-1" \
    --remote-rh-cli "$REMOTE_HOME/Rehoboam/target/release/rh-cli" \
    --socket "$XDG_RUNTIME_DIR/rh/hamlet-A.sock" \
    --cache-dir "/dev/shm/rh-view-$(id -u)"        # optional: keeps watched history across viewer restarts
```

**The scrub-latency measurement (V-VIEW-05; the director, on the P51, about 10 minutes):** with a finished `hamlet` run (and later `twin_vales` and `kingdom_10k`, after SPR-030):
```bash
target/viewer/release/rh-view --world hamlet --run-dir ~/rh-runs/hamlet-1 --rh-cli target/release/rh-cli \
    --bench scrub --present immediate --fullscreen --out ~/rh-scrub-hamlet.txt
```
The same against a run on `adjutant` (with the `--remote` options) gives the remote figures. Send the files to the Architect, or paste them into the PR.

## Out of scope (explicit)
- **Delta records for hot columns, or denser keyframes** (ADR-0030's "revisit when"): engine work, and only if the reported latency is unacceptable.
- **Households, houses and titles in the content** (SPR-034 F1). The HUD panel at materialized ticks is the most this sprint shows.
- **Pace commands from the viewer.** `rh-cli pace`, `pause` and `resume` over SSH (SPR-027).
- **Forks and acting in the past** (P2.7, V.3; ADR-0025). The scrubber only reads.
- **The camera director and channel C** (SPR-036).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | Written from R3D-11, `docs/09` V-R3D-13, ADR-0036 §2.4, D62 and this plan's cache section, before reading the code:<ul><li>the five paths at ticks of their own, including a tick on a keyframe, a tick on a Month pulse and a tick one minute after one;</li><li>the cache's exactness under budgets of their own and a forced eviction storm;</li><li>tampered `/dev/shm` files of their own (truncated, wrong version, another run's history);</li><li>the digest check with a dump of their own altered at a table boundary;</li><li>two mutants of their own (a checkpoint keyed without the generator version; a delta log that drops a same-instant birth and death pair)</li></ul> | `VER-SPR-035-TEST` |
| AGT-003-IF | Written from SPR-027 T-02, `control_v1.md` §7 (nothing here is a controller), `05.10` §4 and this plan's specs table:<ul><li>the subprocess protocol (arguments, stdout and stderr, exit codes, cancellation that leaves no orphan process, locally and through the `ssh` stub);</li><li>the refusals (a mutant `rh-cli`, a mismatched in-process build, a dump that fails its digest);</li><li>the read-only check after materializations;</li><li>the mode machine's inputs;</li><li>the latency report's method against its definition</li></ul> | As above |
| Red Team (audit) | Scrubbing and play on `hamlet` and, when runnable, `twin_vales`: the scrub latency as felt, the timeline's legibility. A finding becomes a check (ADR-0036 §4) | Red Team notes |

## Merge route

- **Class and conditions.** C1: green CI, the Tester's pass and the Architect's review. Nothing is announced beyond the merge summary.
- **Contention (D62).** Priority 1 wins every merge slot, review hour and CI minute.
- **The SPR-027 request (F2)** is a line in SPR-027 T-02 if SPR-027 has not started, or a C1 follow-up on `rh-cli` otherwise. It is not a precondition: the temporary-file fallback works.

## Risks

| # | Risk | Likelihood / effect | Mitigation |
|---|---|---|---|
| 1 | Unwatched jumps at 10,000 people take seconds | Certain until SPR-038 and SPR-030 measure; high after | Stated in the plan and D63. Prefetch, keyframe previews, and the engine's own remedies, which are the Architect's to rule on |
| 2 | The viewer's `Dump` parser and the engine's renderer drift | Low / medium | The parser is `rh-golden`'s own, and its digest check fails on any drift |
| 3 | The in-process materializer replays different code from the run's | Medium if used / high | Off by default; refused unless it reproduces a keyframe's hash; `engine_build` does not name a commit (F1) |
| 4 | `/dev/shm` holds stale or foreign entries | Low / medium | The header check, the history key, removal at exit |
| 5 | A remote materialize leaves a process on `adjutant` after cancellation | Medium / low (CPU) | The viewer kills the remote command through the shared SSH connection, and the Tester checks for orphans |
| 6 | The cache budget is too small for `kingdom_10k` over 1,000 years | Low / low | About 80 MB projected, against 256 MiB; eviction to the materializer keeps it correct |

## Findings for the Architect

| # | Finding | Recommendation | Confidence |
|---|---|---|---|
| F1 | **`engine_build` does not name a commit.** It is `rh-golden <version>` plus the built systems, so the viewer cannot tell from the history tuple whether its own linked engine replays the run. A run on `adjutant` is not on the laptop's disk anyway | Replay with the run's own `rh-cli` as a subprocess, where the run is, and verify the dump's digest in the viewer. In process only when it first reproduces a keyframe's hash. File a BCK-002 item to add the builder's commit to `run.toml` (tool plane, never in the ladder or the history tuple), so a viewer can name the right binary | 75% |
| F2 | **`rh-cli materialize --dump <file>` cannot stream a dump over SSH cleanly** | Add `--dump -` to SPR-027 T-02: the dump alone on stdout, the report on stderr. Until then, use a temporary file on the remote | 80% |
| F3 | **What the cache holds** (D62 left it open) | View-state checkpoints and deltas, plus content addressed by the grammar's inputs; not dumps, not per-tick voxels. RAM by default, with `/dev/shm` opt-in for persistence across viewer restarts | 75% |
| F4 | **Households and M7 data exist only at materialized ticks** (SPR-034 F1) | A HUD panel labelled with the materialized tick, never in the content digest. The content gains them only when channel A carries them | 70% |
| F5 | **V-VIEW-05's 250 ms cannot hold for unwatched jumps at 10,000 people:** 4–15 s of replay today | Keep it a reported target (D62) with the parts broken down. If the director needs faster jumps, the choice is the engine's: denser keyframes for viewed runs, or ADR-0030's delta records | 80% |
| F6 | **SPR-027's seek figures are measured on the rungs only** | SPR-030 reports the materialize time at the farthest tick from a keyframe on `twin_vales` and `kingdom_10k` with its G1a run, as this sprint's 10k figure | 70% |
| F7 | **Prefetch spends `adjutant`'s CPU** while the director watches | On by default, one materialize at a time, `nice`d, stopping at the history budget, and `--prefetch none` turns it off | 65% |
