---
id: SPR-034
title: "Stream ingestion and plane-B procedural hydration: the IF-0 reader with catch-up from sealed segments (local, or over SSH from adjutant), the stream-derived view state, building grammar v0 from the settlement seed, and V-R3D-13 across runs, threads, builds and rewind by re-reading against forward play"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint, observability, stream]
related: [PRJ-003, ADR-0024, ADR-0030, ADR-0031, ADR-0035, ADR-0036, SPR-015, SPR-018, SPR-020, SPR-026, SPR-033, SPR-035, D51, D53, D57, D62]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF:**
> - **What.** The viewer starts reading the world's history, not just its tip.
>   - **The full IF-0 reader:** `rh-protocol`'s `Catchup` with the sealed-segment reader of `rh-stream`, so a viewer sent behind reads the sealed years and rejoins with no gap and no duplicate. It works on a local run directory, or on a run on `adjutant`: the live edge through an SSH tunnel to `A.sock`, the sealed years through `rh-cli tail --canonical` over the same SSH connection.
>   - **The view state** `view_state_v0`: the plane-B projection that hydration reads. It holds the settlements, the living persons with sex, birth, parents and settlement, and the residents per settlement. It is folded from the genesis export and channel-A records, and checked against every `STEP_DIGEST`.
>   - **Building grammar v0** (E1, 1000–1500): dwellings on a plot grid along the seat's routes, a green and a well, a stone church past a population threshold, earth roads. The grammar is integer and keyed through `rh-ids` under plane-B purposes, from the settlement seed, and growth-stable.
>   - **V-R3D-13:** the bubble's content digest is equal across two runs, 1 and 4 hydration threads, debug and release builds, a run at 1 and 4 engine threads, and **rewind by re-reading** (re-folding from genesis through the sealed segments) against forward play on the live edge. Rewind through the materializer is SPR-035's.
> - **What it cannot show yet, plainly.** Channel A carries no household membership (`MARRIAGE` has an empty payload) and, until S3 F1 in P1b, no residence record (kind 7 was deferred, D57). Births have a settlement only once SPR-018 and SPR-020 give events their place, and births exist only once SPR-015 lands. Grammar v0 therefore reads residents per settlement, not households. Houses (M7) and titles wait for their records (F1, F2).
> - **Boundaries.** Tool plane, read-only. Procedural assets only (D53): no texture, model or sound file.
> - **Who.** Developer: AGT-002-TOOLS. Testers: AGT-003-IF and AGT-003-DET, one session, on a different model from the Developer's. The Red Team audits.
> - **Size and class.** About 3 agent-days. **C1.**
> - **Status:** PLANNING. It needs the Architect's approval as a C1 plan (GOV-005 §2); it goes to the director in D63 with SPR-033 and SPR-035.

# SPR-034 — Stream ingestion and plane-B procedural hydration

- **Roadmap item:** `PRJ-003` §4 (SPR-034); `docs/11` viewer lane V.1 · **Phase:** viewer lane (priority 2, D62; tool plane)
- **Developer:** `AGT-002-TOOLS`
- **Testers:** `AGT-003-IF` and `AGT-003-DET` (one session, on a different model from the Developer's). The Red Team audits.
- **Depends on:**
  - **Hard:**
    - **SPR-033 merged:** the workspace, chunks, mesher, terrain, export, client, cameras and job;
    - **SPR-026 merged:** `rh-protocol`'s `Catchup`, the host, `rh-cli tail --canonical --from-seq --to-seq` (REV-SPR-026 R23's sealed-read path) and `rh-stream`'s `LiveEdge`.
  - **Soft, for content only.** The sprint builds and gates without these; what they add appears when they are on `main`:

    | Sprint | What it adds |
    |---|---|
    | SPR-015 | Births, marriages, households in plane A |
    | SPR-018 | M1 F0, S3 F0 and events' `place` in the engine |
    | SPR-020 | `Event.place` and the envelope |

  - **Not** SPR-027.
- **Binding contracts:**
  - ADR-0036 §1, §2 (items 2, 4, 5), §4, §5;
  - R3D-2 and R3D-11 (`docs/03`); `docs/09` V-R3D-13, V-R3D-14;
  - `specs/stream/wire_v1.md` §1–§6, §9 (the viewer's side), with `golden/wire_v1.txt`'s `scenario catchup`;
  - `specs/stream/canonical_v0.md` §1, §3, §4; `channel_a_v0.sql` (record kinds 2–6);
  - `05.10` §5–§7;
  - `specs/systems/S3.migration.odd.toml` `[algorithm.residence_f0]` (genesis only);
  - `05.1` §6 item 2 (the keyed hash `H`), as ADR-0036 §2.4 applies it to plane B;
  - D53.
- **Requirements covered:**
  - R3D-11 (reads channel A; content a keyed function of the state at the tick and the generator version);
  - R3D-2 (part: versioned generators from a seed hierarchy and the snapshot);
  - STR-3 (a viewer cursor with catch-up);
  - V-VIEW-02 (PRJ-003 §5).

## Specs this sprint needs

| Spec | Exists? | Gap | Proposed (for the Architect's ruling) | Needed by |
|---|---|---|---|---|
| **The view state** | ADR-0036 §2.4: content is a function of "the state at the tick" | Which part of the state, and from which records, so that forward play and rewind give the same thing | `view_state_v0` (T-02): the settlements; the living persons `{id, sex, birth_tick, mother, father, settlement?}`; residents per settlement; the count of living persons with no known settlement. It holds only what the genesis export and channel A determine exactly. **Nothing is inferred:** the viewer never applies a plane-A rule, such as S3's "newborn takes the mother's settlement" | T-02 |
| **Households, houses and titles in the stream** | `M2.demography` (next 0.3.0): `MARRIAGE` roles husband and wife, payload empty; household membership is a column, not a record. Kind 7 `RESIDENCE_INTERVAL` is deferred to S3 F1 (D57, BCK-002 A-61) | A hydration that reads households cannot be equal under rewind and forward play: the stream does not carry them | Grammar v0 reads residents per settlement, not households (F1). File a BCK-002 item for a channel-A record of household membership changes beside S3 F1's kind 7 in P1b | T-02, T-03 |
| **The settlement seed and plane-B purposes** | ADR-0036 §2.4: `H(world_seed, purpose, entity, tick-bucket, k)` "with plane-B purposes registered like any other" | No plane-B registry exists, and PRJ-003 §2's path list has no `rh-ids` | `rh-ids`' keyed hash under the system id `rh-view`, branch salt 0, epoch 0, tick 0. Purposes are listed in `rh-view-core/src/purposes.rs`: `settlement_plan`, `plot_order`, `building_form`, `building_dims`, `church_site`. A test refuses a name used twice, or a system id equal to a reserved or Registry one. `rh-ids` joins PRJ-003 §2's list; it is already in the viewer's graph through `rh-stream` (F3) | T-03 |
| **Remote sealed reads** | `wire_v1.md` §6.3: a viewer sent behind reads the sealed years from disk | On the laptop, `adjutant`'s run directory is not on disk | `ssh <host> <rh-cli> tail <run-dir> --canonical --from-seq a --to-seq b` (R23's stdlib path, canonical lines). One SSH connection is shared through `ControlMaster`, so a read costs a round trip, not a handshake. Each year is checked against its `stream.sealed` digest or segment row | T-01 |
| **Founders' residence at genesis** | S3 F0's placement is in the golden's `residence` line for the rung worlds; the engine's `person.residence` exists only after SPR-018; the stream has no residence record in P1a | Where `view_state_v0` gets founders' settlements | SPR-033's export, which carries each founder's `PersonId` (the reference's `founder(world_seed, k)`; the engine rehashes only on a collision, which the reference's check 3 rules out for these worlds) with its settlement, for worlds whose population the manifest gives in full. For a synthesised population (`twin_vales`, SPR-019), it is unknown until SPR-035's materializer reads `person.residence` | T-02 |
| **Newborns' settlement** | After SPR-018 and SPR-020, `BIRTH` carries `place_id` (PRJ-002's SPR-018 row) | Before then, none | The `BIRTH` event's `place_id`, whose target role is the child. Before SPR-018 the child's settlement is unknown and counted as such; it is never inferred from the mother | T-02 |
| **The engine binary in the viewer job** | SPR-033's job builds only `viewer/` | Catch-up, V-R3D-13 across engine threads and the ladder probe need real runs | The job builds `rh-cli` in debug from the root workspace and runs `hamlet` for 30 sim-years per case, plus one 200-year run for the ladder probe. No build profile enters history (ADR-0006's tuple has none), and that probe checks it: the debug run's ladder must equal the release-recorded baseline | T-01, T-04, T-05 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C1.** `viewer/` and its job only. No engine crate changes |
| Planes touched (ADR-0024) | **Tool:** `rh-view-core` (`source`, `catchup`, `view_state`, `purposes`, `grammar`, `hydrate`), `rh-view` (figures, HUD), the viewer job, `bin/check_viewer.sh`. **B:** the view state and the hydrated content, in the viewer's memory only. **A:** read through `A.sock`, the sealed segments (`rh-stream`'s reader) and `rh-cli tail`; never written. **C:** none |
| Owned columns written | None |
| Reads (via `rh-contracts`) | None. Channel-A records: `LIFE_ENTRY`, `LIFE_EXIT`, `EVENT` (`BIRTH`'s `place_id` and roles), `STEP_DIGEST`, `KEYFRAME` |
| RNG purposes added | **Plane B only**, in `rh-view-core/src/purposes.rs`: `settlement_plan`, `plot_order`, `building_form`, `building_dims`, `church_site`. They are keyed by `rh-ids` under the system id `rh-view`, never registered in plane A's Registry, and never drawn by plane A |
| Golden-hash effect | **None** on any engine ladder. The viewer's `viewer/golden/hydrate_v0.txt` is new, plane B, regenerated only by its tool |
| Fixed-point quantities | None; content is integer. `rh-view-core` keeps `clippy::float_arithmetic` denied |
| Determinism lints | The engine's are unchanged. In `viewer/`: SPR-033's lints. No `HashMap` or `HashSet` in `rh-view-core` (the root `clippy.toml`'s ban applies). Hydration results are ordered by `(settlement, chunk)`, never by completion |
| Assets | Procedural only (D53). SPR-033's assets check (no image, model or sound file under `viewer/`, except what its F11 ruling allows) keeps running; the grammar adds palette entries, never files |
| Tests that gate this sprint | V-R3D-13 (all five clauses of T-04); the catch-up probes (V-STR-05's method, with the viewer as the client); the view state against every `STEP_DIGEST`; growth stability; the planted controls; V-VIEW-01's ladder clause, now in the job; `bin/check_viewer.sh` and `bin/check_all.sh` part core (`viewer-isolation`) |
| Docs updated in the same sprint | `CLAUDE.md` (the new viewer checks); `docs/09` V-R3D-13's method line; PRJ-003 (status, F1–F3 as ruled); BCK-002 (F1's channel-A item for P1b); `viewer/README.md` (sources, remote mode, the view state's limits) |

**Rules carried from SPR-033 (every task):**
- A positive control fails by name.
- A probe asserts its path: the sleeper really went behind, the sealed read really ran over the SSH stub, the hydration really ran on four threads.
- Generated files only through their tools.
- No wall-clock gate.
- One commit per task.
- Merge `origin/main` when it moves.

## Tasks

#### T-01: The run source and catch-up, local and remote (0.8 day)
- **Deliverable (`rh-view-core::source`, `::catchup`):**
  - **`RunSource`:**
    - **`Local { run_dir }`:** the live edge at `run_dir/stream/t0000000000/A.sock`. Sealed reads through `rh-stream`'s `read_channel_a` and `canonical::render`, so both sources hand the fold canonical lines.
    - **`Remote { host, run_dir, rh_cli }`:**
      - The live edge through a forwarded socket: `--socket PATH` (the director's tunnel, SPR-033), or, if absent, a tunnel the viewer spawns, `ssh -N -o ExitOnForwardFailure=yes -o StreamLocalBindUnlink=yes -L $XDG_RUNTIME_DIR/rh-view/<run>/A.sock:<run_dir>/stream/t0000000000/A.sock <host>`, killed on exit and on drop.
      - Sealed reads by `ssh <host> <rh_cli> tail <run_dir> --canonical --from-seq a --to-seq b`.
      - Every `ssh` runs with `-o ControlMaster=auto -o ControlPath=$XDG_RUNTIME_DIR/rh-view/cm-%C -o ControlPersist=60 -o BatchMode=yes`.
      - The HUD warns that a tunnel the viewer spawns does not keep `adjutant` awake (SPR-033 F9).
    - `std::process::Command` lives in `source.rs` alone, with a use-site `allow` and its reason (the root `clippy.toml`).
  - **Catch-up:** `rh_protocol::cursor::Catchup` drives every connection.
    - Sent behind, the client reads `[cursor, sealed_last_seq]` from the source and subscribes again from `sealed_last_seq + 1`.
    - A resumed host (`future_seq` while not finished) is retried with the machine's backoff.
    - A changed `history` stops the viewer with an error naming both tuples.
    - Every sealed year read, and every year received live, is checked against its `stream.sealed` digest, or its segment row's `canonical_digest`, by XXH3-128 over the bodies (`rh-stream::canonical::digest`). A mismatch stops the viewer by name.
  - **The command line:** `--run-dir DIR` (local), or `--remote HOST --remote-run-dir DIR [--remote-rh-cli PATH] [--socket PATH]`.
  - `rh-stream` joins `rh-view-core` as a path dependency (PRJ-003 §2).
- **Acceptance** (in the viewer job, on runs of the debug `rh-cli` the job builds):
  - **The sleeper (V-STR-05's method):**
    - a client stops reading until two more years have sealed (asserted through a second connection's `stream.status`);
    - it is sent behind (it asserts it received `stream.behind`), reads the sealed years, rejoins and reads to `stream.end`;
    - its seqs are exactly `0..next_seq`, and every year's digest matches.
  - **Jump to live:** `first_seq = live.next_seq`, nothing duplicated.
  - **Kill and resume:** the host killed with `kill -9` mid-year and resumed from its 10-year keyframe (SPR-013 T-09). The client backs off on `future_seq`, verifies the re-emitted year against its `stream.sealed`, and ends with the same seqs and digests.
  - **The remote path, on an `ssh` stub** first on `PATH`. The stub:
    - runs the remote command locally;
    - for `-L a:b`, links `a` to `b`, then waits to be killed;
    - records its arguments.

    The remote source then gives the same lines as the local one. The test asserts the stub saw the tunnel and the `tail` invocations, with `ControlMaster`.
  - **Read-only:**
    - the client's request set is exactly the three stream methods;
    - the run directory's tree and SHA-256s are unchanged after a sealed read;
    - the run's ladder equals `worlds/hamlet/baselines/hamlet_yearly_200y.ladder` with the viewer's client attached (V-VIEW-01's ladder clause, now a gate in the job).
- **Status:** [ ] not started

#### T-02: The view state and its fold (0.6 day)
- **Deliverable (`rh-view-core::view_state`, version `view_state_v0`):**
  - **The state:**

    | Part | Contents |
    |---|---|
    | Settlements | `id` (1-based, worldgen-lite's), region, seat cell, routes |
    | Persons | the living, keyed by `PersonId`: `sex`, `birth_tick`, `mother`, `father`, `settlement: Option<u32>` |
    | Derived | residents per settlement; the unplaced count |

  - **Its sources, in order:**
    1. **The genesis export** (SPR-033's format: each founder's `PersonId` and settlement, for worlds whose population the manifest gives in full).
    2. **Every channel-A record with `tick ≤ T`, in `seq` order:**

       | Record | Effect |
       |---|---|
       | `LIFE_ENTRY` | adds a person; a founder takes the export's settlement |
       | `EVENT BIRTH` | sets the child's settlement from `place_id`, when present |
       | `LIFE_EXIT` | removes the person |

  - **The tick rule:** "state at T" means after Bookkeeping of T, which is exactly the materializer's meaning (SPR-027 W37) and the last record of T's instant.
  - **An incremental form for forward play:** apply one record, report which settlements' inputs changed.
  - **`STEP_DIGEST` cross-check:** at every Month pulse the fold's living count equals `STEP_DIGEST.living`, and its births and deaths since the previous digest equal `births` and `deaths`. A mismatch stops the viewer by name: the fold has missed a record.
- **Acceptance:**
  - **The cross-check holds at every Month pulse** of `solo`, `household` and `hamlet`: 30 sim-years each in the PR job, 200 in `check_viewer.sh --full` (the Tester's run, attached to the PR).
  - **The incremental and batch forms** give the same state at 50 SplitMix64-chosen ticks.
  - **Planted controls, each failing by name:**
    - `mutant-fold-skip-exit`, which ignores one `LIFE_EXIT`, fails the cross-check;
    - `mutant-fold-infer-mother`, which gives a newborn its mother's settlement when `place_id` is absent, fails the unit test that unplaced births stay unplaced.
  - **Unit tests** on a fixture of canonical lines: a founder, a birth with and without a place, a death, a same-instant birth and death.
- **Status:** [ ] not started

#### T-03: Building grammar v0 (0.9 day)
- **Deliverable (`rh-view-core::grammar`, version `grammar_v0_e1`; `purposes.rs`):**
  - **Inputs, and only these:**
    - the settlement's id, seat cell, region and the bearings of its routes;
    - the terrain (`terrain_v0`) under its plots;
    - `residents(s, T)` from the view state;
    - the generator version.

    No tick enters except through `residents`, so the same population gives the same village.
  - **The plan (`settlement_plan`, keyed on the settlement):**
    - a main street from the seat along the bearing of its longest route, or a keyed bearing for a settlement with no route;
    - lanes perpendicular every 24–36 m (keyed);
    - an infinite, fixed sequence of candidate plots along them, ordered by `plot_order`: plots of 12–20 m by 20–30 m, each with its frontage on a street, skipping water and slopes over 1 in 3;
    - the green and a well at the seat.
  - **Dwellings:**
    - `n = ceil(residents / 5)` (a design value: about five per dwelling in E1);
    - dwelling `k` stands on plot `k` for `k < n`, so growth adds the next plot and decline leaves the highest plots empty;
    - **exact growth stability:** from `n` to `n + 1` exactly one plot changes.
  - **Forms (E1, 1000–1500; F4):**
    - **The dwellings,** keyed by plot: a cruck-framed longhouse (single storey, 10–14 m by 5–6 m) or a box-framed cottage (7–9 m by 4–5 m). Both have wattle-and-daub walls on a stone sill, a thatched roof pitched 45–55°, a door on the street side and an opening for smoke, with dimensions keyed in 0.35 m steps within the stated bounds.
    - **A stone church** once residents reach 40 (a design value; plane-B staging), on a `church_site` plot beside the green, with a nave, chancel and west tower.
    - **No Tudor jettying or brick chimneys**, which belong to E2 (1500–1750).
  - **Roads:** the main street and lanes as packed earth, 4 voxels wide, draped on the terrain.
  - **Determinism:** every draw is `rh-ids`' keyed hash with system id `rh-view`, branch salt 0, epoch 0, tick 0, the purpose, the settlement or plot as entity, and a fixed `k` per choice (fixed draw counts, no rejection loops). All coordinates are integer voxel units.
  - **The palette** gains timber, daub, thatch, sill stone, church stone and earth.
  - **Procedural only:** no texture or model; colour is per material, with SPR-033's render-only jitter.
- **Acceptance:**
  - **Unit tests per form:**
    - the footprint lies inside its plot;
    - the door faces the street;
    - the roof closes over the walls (no hole visible from above);
    - the church appears exactly when residents ≥ 40;
    - no building on water.
  - **Growth stability:** for `n` from 0 to 60, the content outside plot `n` is identical between `n` and `n + 1`, compared as voxel arrays over the settlement's chunks.
  - **Purposes:** names unique; `rh-view` not among `rh_core::register::RESERVED_SYSTEM_IDS` nor the Registry's system ids (`rh-core` as a dev-dependency).
  - **SPR-033's assets check** passes with the grammar in place.
  - **Reported:** the time to generate and mesh one settlement plan at 0.35 m, against the red team's 35 ms.
- **Status:** [ ] not started

#### T-04: Hydration and V-R3D-13 (0.5 day)
- **Deliverable (`rh-view-core::hydrate`; `rh-view`):**
  - **The bubble's content:** `terrain_v0` plus `grammar_v0_e1` at `view_state_v0(T)`, at L0 inside the bubble and reduced for L1 and L2.
    - Hydration runs per settlement and per chunk on N threads.
    - Results are merged in `(settlement, chunk)` order; a voxel two buildings claim takes the lower plot index, by a rule, never by timing.
  - **Forward play** re-hydrates only the settlements whose inputs changed.
  - **The digest:** SPR-033's content digest now covers the buildings, with every generator version.
  - **Residents** are drawn as standing figures at their dwelling's door, up to one per resident, with no movement. They show who lives there, not where anyone is: daily movement is P1b's (ADR-0036 §5).
  - **The HUD:** per settlement, residents, dwellings, the church, the unplaced count; the generator versions; the source (local or remote).
  - **`viewer/golden/hydrate_v0.txt`:** the generator's golden, independent of any run: the digests of `hamlet`'s settlement-1 bubble for fixed view states built from the genesis export with 0, 5, 20, 40 and 60 residents. Written only by its tool's `--write`; a changed digest under an unchanged generator version fails by name. An engine change that moves `hamlet`'s history never touches it.
- **Acceptance (V-R3D-13), at the end of sim-years 1, 5, 10, 20 and 30 of the job's `hamlet` run:**
  1. **Two runs** of the viewer give equal digests.
  2. **1 and 4 hydration threads** give equal digests.
  3. **Debug and release builds** of the digest test give equal digests.
  4. **The run at `--threads 1` and `--threads 4`:** identical streams (V-STR-01), so equal digests.
  5. **Rewind by re-reading against forward play:**
     - forward: the client follows the live edge from seq 0, folding and hydrating incrementally, and records each digest as it passes the tick;
     - rewind: after `stream.end`, the viewer rebuilds the state at each tick by a batch fold from genesis through the sealed segments, and hydrates from scratch;
     - the digests are equal. (Rewind through the materializer is SPR-035's.)

  **Planted controls, each failing by name:**
  - `mutant-reverse-merge`: overlapping voxels go to the higher plot index; it fails clause 2's companion test, which compares against the rule;
  - `mutant-incremental-stale`: forward play skips re-hydrating a settlement whose residents changed; it fails clause 5.
- **Status:** [ ] not started

#### T-05: The job, the commands and the docs (0.2 day)
- **Deliverable:**
  - **`bin/check_viewer.sh`** gains:
    - the debug build of `rh-cli` from the root workspace;
    - the `hamlet` runs: 30 sim-years at 1 and 4 threads with `--socket`, and the 200-year ladder probe;
    - T-01's probes, T-02's cross-check and T-04's five clauses;
    - the new controls;
    - `--full` for the 200-year cross-checks.
  - **The workflow:**
    - pull-request triggers unchanged (the viewer's own paths);
    - **the `main` list widens** to the crates the job now builds: `crates/**` (the `rh-cli` closure), `specs/stream/**` and `Cargo.lock`;
    - `rust-cache` also caches the root workspace's `target/` for the debug engine, saved on `main` only.
  - **Docs:** the compliance table's list; `viewer/README.md` with the commands below.
- **Acceptance:**
  - The viewer job is green; its cold and warm times are reported against the estimate below.
  - The `--full` run's report is attached to the PR.
- **Status:** [ ] not started

**Size.** T-01 0.8, T-02 0.6, T-03 0.9, T-04 0.5, T-05 0.2: 3.0 agent-days. **Cut order** if it runs long:
1. The church and its tower (0.15 day), to the next viewer sprint.
2. The viewer-spawned tunnel: `--socket` stays required for remote runs (0.1 day).
3. The incremental re-hydration: forward play re-hydrates the whole bubble at each changed sim-month (0.1 day; clause 5 then compares two batch paths, and the plan says so).

## How the director runs it (as planned, in addition to SPR-033's)

**A run on `adjutant`, with history:** start the run and the tunnel exactly as in SPR-033 (terminal 1, the `ssh -t` session that keeps the VM awake). Then:
```bash
target/viewer/release/rh-view --world hamlet \
    --remote adjutant --remote-run-dir "$REMOTE_HOME/rh-runs/hamlet-1" \
    --remote-rh-cli "$REMOTE_HOME/Rehoboam/target/release/rh-cli" \
    --socket "$XDG_RUNTIME_DIR/rh/hamlet-A.sock"
```
The viewer reads the years it missed through `rh-cli tail` over the shared SSH connection, then follows the live edge.

**A finished run, local or remote:** the same command after `stream.end` folds the whole history from the sealed years, and the cameras roam the end state.

## The CI job's cost, after this sprint (estimate; T-05 reports the measured figures)

| Item | Added | Basis |
|---|---|---|
| `rh-stream` and the engine crates it reads, in the viewer's build | about 40 s cold, nothing warm | Measured 2026-10-18: 71 crates, 40 s on the 4-core sandbox |
| The debug `rh-cli` from the root workspace | about 1.5–2 min cold, under 30 s warm | The subset above plus `rh-golden`, `rh-manifest`, `rh-sifter` and the system crates |
| The `hamlet` runs and probes (30 sim-years each, and one 200-year ladder probe, in debug) | about 2 min | Release `hamlet` takes about 1 s per 200 years; debug is assumed 30–60× slower |
| **A cold run / a warm run** | **about 11–14 min / 4–6 min (4 vCPU)** | — |
| **Minutes per active month** | **about 300–400 of 2,000** (15–20%), of which about 100–150 are `main` pushes from engine merges | about 45 PR runs, 20–30 `main` runs, mostly warm |
| Actions cache | about +0.4 GB for the debug engine target | Saved on `main` only |

## Out of scope (explicit)
- **The materializer, the watched-history cache, the scrubber,** and rewind through the materializer (SPR-035).
- **Households as houses, dynastic houses, titles, halls and towers at title seats.** They wait for channel-A records (F1) or the materializer (SPR-035, at materialized ticks only, F1).
- **People moving, fields, mills** (V.2, with P1b's systems).
- **`rh-worldgen` as a source.** The export is exact for the golden worlds, and SPR-035's materializer checks the engine's genesis against it (F5).
- **Channel C** (SPR-036). **The pace command** (`rh-cli pace` over SSH).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | Written from `wire_v1.md` §6, REV-SPR-026 R15 and R23, `canonical_v0.md` and this plan's specs table, before reading the code:<ul><li>catch-up with a sleeper, a jumper and a kill-and-resume of their own, with R15's four orderings forced by their own host;</li><li>the remote path with their own `ssh` stub, including a stub that fails mid-read (the viewer must stop by name, never show a partial year);</li><li>the view state against a DuckDB query of their own over the sealed segments (living per settlement at chosen ticks);</li><li>the `STEP_DIGEST` cross-check with a dropped record of their own;</li><li>the assets test with a planted `.png`</li></ul> | `VER-SPR-034-TEST` |
| AGT-003-DET | Written from ADR-0036 §2.4, `docs/09` V-R3D-13 and this plan:<ul><li>the five V-R3D-13 clauses at ticks of their own choosing;</li><li>growth stability with their own `n` sequence, including decline;</li><li>the keyed draws: a fixed draw count per choice, and the same plan for the same residents at different ticks;</li><li>two mutants of their own (a `BTreeMap` iterated by a key that includes a thread index; a grammar choice keyed by tick)</li></ul> | As above |
| Red Team (audit) | The villages: plausibility for E1, the street and plot logic, growth. Each finding becomes a check (ADR-0036 §4) | Red Team notes |

## Merge route

- **Class and conditions.** C1: green CI, the Tester's pass and the Architect's review. Nothing is announced beyond the merge summary.
- **Contention (D62).** Priority 1 wins every merge slot, review hour and CI minute.
- **Shared files.** Only `viewer/`, `bin/check_viewer.sh`, the workflow, `CLAUDE.md` and BCK-002.
- **After SPR-015, SPR-018 or SPR-020 merge:** the viewer job's `main` run shows the new content (births, places). The view state's cross-check and the five V-R3D-13 clauses compare paths with each other, so they need no regeneration; `viewer/golden/hydrate_v0.txt` depends on the generator only. If a new record kind breaks the fold, the fix is a priority-2 follow-up, never a condition on the engine PR.

## Risks

| # | Risk | Likelihood / effect | Mitigation |
|---|---|---|---|
| 1 | The villages look thin, because residents are the only input and households are not in the stream | High / medium (the red team's audit) | F1's backlog item; at materialized ticks SPR-035 can show households, labelled as such. The HUD states what is unknown |
| 2 | SSH sealed reads are slow over IAP | Medium / low | `ControlMaster`; reads per sealed year in one call; the read cost reported |
| 3 | An engine change moves `hamlet`'s history | High over P1a / none for the gates | Every run-based check compares two paths of the same run; the hydration golden depends on the generator alone |
| 4 | The fold misses a record kind a later engine sprint adds | Medium / medium | The `STEP_DIGEST` cross-check stops the viewer by name; unknown kinds are skipped only after the frozen prefix is checked |
| 5 | The grammar's thresholds look wrong to the red team | Medium / low | Design values in one table, versioned; a change bumps `grammar_v0_e1` and regenerates the golden |
| 6 | `twin_vales` shows no founders' settlements until SPR-035 | Certain / low | Stated in the HUD; SPR-035's materializer supplies `person.residence` |

## Findings for the Architect

| # | Finding | Recommendation | Confidence |
|---|---|---|---|
| F1 | **Channel A cannot carry households, M7 houses or titles to the viewer as it stands.** `MARRIAGE` has an empty payload, household membership is a column, and kind 7 `RESIDENCE_INTERVAL` is deferred to S3 F1 (D57). A hydration that read them could not be equal under rewind and forward play, as R3D-11 requires | Grammar v0 reads residents per settlement. File a BCK-002 item for P1b: a channel-A record of household membership changes (and M7's tenure changes, if its events do not carry them), beside kind 7. SPR-035 may show households at materialized ticks only, labelled "at the last materialized tick" | 75% |
| F2 | **Residence before SPR-018 is genesis-only, and births before SPR-015 do not exist.** `PRJ-003` §6 had no edge for this | Soft edges SPR-015, SPR-018 and SPR-020 → SPR-034 in PRJ-003 §6 (done in this plan's PR). The sprint builds and gates without them | 80% |
| F3 | **ADR-0036 §2.4's plane-B purposes have no registry, and PRJ-003 §2's path list lacks `rh-ids`** | Purposes in `rh-view-core/src/purposes.rs` under the system id `rh-view`, using `rh-ids`' keyed hash. Add `rh-ids` to PRJ-003 §2's list; it is already in the viewer's graph through `rh-stream` | 70% |
| F4 | **PRJ-003 §4's "Tudor houses"** are 1485–1603, at the very end of E1 (1000–1500) | Grammar v0 builds cruck and box-framed houses with wattle and daub and thatch, and a stone church; Tudor forms wait for E2's pack | 60% |
| F5 | **`rh-worldgen` is not needed in SPR-034.** The export is exact for the golden worlds, and SPR-018's sprint is gated to the same golden | Keep `rh-worldgen` off the viewer's dependencies until a world without a golden case needs the engine's genesis; SPR-035 checks the engine's genesis against the export through the materializer | 70% |
| F6 | **The viewer job's engine build makes its `main` trigger cover nearly all of `crates/`** | Accept it on `main` only (SPR-033 F7), and use a debug engine in the job. No build profile enters history, and the job's ladder probe checks the debug run against the release-recorded baseline | 75% |
| F7 | **R23's `tail --canonical --from-seq --to-seq` is a public interface** for the viewer, not only the TUI | Name it in `wire_v1.md` §6.3 as the sealed-read path for remote viewers, so a change to it is reviewed as one | 65% |
