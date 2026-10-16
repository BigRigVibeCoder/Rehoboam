---
id: SPR-011
title: "Walking skeleton: one loop from manifest to replay, on solo, household and hamlet"
type: how-to
status: APPROVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, BCK-002, PRJ-001, PRJ-002, SPR-007, SPR-008, ADR-0025, ADR-0026, ADR-0030, ADR-0031, ADR-0035]
created: 2026-10-16
updated: 2026-10-16
version: 0.4.0
---

> **BLUF:** Close the engine's loop end to end at the lowest fidelity: manifest → `rh-worldgen` stub → ledger (born, ages, dies) → scheduled death timers → events → the Parquet chronicle and World Stream v0 (channel A) → `rh-cli tail` → a trivial sifter pattern → journal replay. It runs on `solo` first, then on `household` and `hamlet`, on one binary, with plane-hash ladders in CI from the first commit. Developer: AGT-002-CORE (AGT-002-TOOLS for the twin and its check). Testers: AGT-003-DET, AGT-003-IF, AGT-003-STAT. **APPROVED by the director at G0** (C3; D30, 2026-10-16; `docs/decisions_pending.md` D30 to D32); the rulings below are binding. The work is about 9 agent-days, three times the GOV-005 bound, so ruling 1 runs it as three sprints: SPR-011 (increment A), SPR-012 (increment B) and SPR-013 (increment C).

# SPR-011 — Walking skeleton (WP P1.0)

- **Roadmap item:** `docs/11` P1.0 · **Phase / rung:** P1a, rungs 0–2
- **Depends on:** G0 passed (`VER-G0`); SPR-007 merged (2026-10-16); SPR-008 merged; SPR-009 merged for the V-PERF-11 counts in T-13 · **Plan approved by the director 2026-10-16** (C3, D30; GOV-005 §2)
- **Binding contracts:** `05.1` §2, §4, §6, §9; `05.2` §0, §1, §4, §6; `05.5` §3.2; `05.10` §4, §5, §7, §7a; `05.9` IF-0, IF-1, IF-2; `specs/chronicle/schema.sql`; ADR-0006, ADR-0024, ADR-0025, ADR-0026, ADR-0030, ADR-0031, ADR-0035 §6; `docs/01` §5 principle 8
- **System spec:** `specs/systems/M2.demography.odd.toml`, rung-0 slice. It is a draft with gaps (Blockers 2 and 3) and must be completed, pass `lint_odd.py` and be marked `frozen` before increment B starts.
- **Requirements covered:** STR-1 (v0), DET-1, DET-4, DET-5, DET-7 and DET-13 on the new worlds, DATA-1 and DATA-6 (subset), DATA-8 (founders and deaths), SIM-1 (N = 1 to 60), SIM-5 (death draws), SIM-6 (pre-epoch founders), SIM-15, SIFT-3 (one pattern), EVO-4

## Architect rulings (binding since the plan's approval, D30)

Each ruling is needed because a contract is silent or two contracts disagree. Confidence is the Architect's estimate that the recommendation is right. Rulings 1, 2 and 19 change the roadmap or an earlier statement, so the director decided them: ruling 1 by D30, ruling 2 by D31 and ruling 19 by D32, all approved 2026-10-16. The others are the Architect's. Ruling 5 also serves `T1.minds` and `M7.dynasty` (PR #32): the dead stay readable until `Bookkeeping` of their death instant.

1. **Size: three sprints, one plan.** P1.0 is about 9 agent-days (increment A ≈ 3, B ≈ 3, C ≈ 3.5). GOV-005 §3 and GOV-007 §3 cap a sprint at about 3. Recommendation: the director approves this one plan; at approval the Architect issues increment A as SPR-011, B as SPR-012 and C as SPR-013, moving tasks and rulings verbatim, each with its own branch, PR, Tester pass and audit. The alternative, one 9-day sprint by exception, gives one huge C3 PR that is hard to audit. **70%.** **Approved (D30):** SPR-011 is increment A, the engine prerequisites (the purity lint, the 32-bit probability type, the life-course log plumbing); SPR-012 is increment B, `solo` lives and dies; SPR-013 is increment C, World Stream v0 and the chronicle.
2. **What P1.0 simulates on each rung.** Ageing and death only; every person is a founder created at world creation; no births. `household` and `hamlet` run the same slice and die out within about a century; births, partnering and kinship come with P1.3. The ladder's rung-0 entries "a household's food" (M3, M4: P1b) and "daily life at `minutes`" (S5 F2: Later, `05.1` §3) are not reachable in P1a. Record this in `docs/11` and PRJ-001 §3 rather than let rung 0 look incomplete. **75%.**
3. **Systems a manifest names that the build lacks.** `solo`'s template lists M1, M3, M4 and S5, and its `fidelity_profile` names M4 F1 and S5 F2; none is built. The run header (ladder header and the first channel-A segment) lists each named system as `built` or `not_built`. A `not_built` system is skipped, never stubbed with behaviour. `rh-cli run --strict` refuses such a manifest. The built set is part of `engine_build`, so history stays a function of the tuple (DET-1). **70%.**
4. **A finer probability type.** `rh_core::fixed` has only `Q16_16` probabilities (step 1/65,536). A monthly death probability at e₀ = 70 for a ten-year-old is about 4 × 10⁻⁵, under 3 steps: a 30% rounding error. Add an unsigned probability with 32 fraction bits (working name `Prob32`), with `draw_hits`/`chance` mapped exactly as SPR-010 did (hits iff `draw < bits · 2^32`), golden vectors from `bin/gen_fixed_vectors.py`, and implement BCK-002 A-21 against it. C3 machinery, no hash change. **70%.**
5. **Ledger rows and the life-course log.** The ledger exists when `person.id` is registered. `rh-core` keeps every `person.*` column aligned and sorted by `person.id`. Dead rows leave at once by a core compaction pass at the start of `Bookkeeping` of the instant of death, across every `person.*` column whatever its owner, before invariants and hashes. `life_course.*` columns are append-only; the exit fields of a row are written once. The smoke world has no `person.id`, so its baseline does not move. Alternative: a step API for row removal, which spreads the rule over every owner. **65%.**
6. **The death path in P1.0 sends no lifecycle command.** A firing M2 death timer is M2's own scheduled outcome. M2 applies it through one function, `apply_death` (alive flag, death tick, life-course exit, `DEATH` event), which is also the function that will drain `Death` commands from other systems. Today a command reaches its owner only when the owner next runs (SPR-006), up to a month later for M2, which breaks "dead rows leave at once". Same-instant delivery to a later phase, which M7's immediate succession also needs (V-SAGA-19), goes to the P1.13 sprint with BCK-002 A-30 and A-31. **70%.**
7. **The P1.0 mortality algorithm, with no transcendental in plane A.** At β = 1 Brass is l(x) = 1 / (1 + K·O(x)), with O(x) = (1 − lₛ(x)) / lₛ(x) exact from the four-digit standard and K = e^{2α}. K is solved from e₀ by fixed-point bisection over a fixed iteration count (e₀ falls as K rises; trapezoid on the standard grid, as `selftest_brass.py` does). Monthly death probability = 1 − l(x + Δ) / l(x), with l linear between grid ages and Δ the month's length in years; l = 0 at 100 gives certain death (the old-age tail is P1.3). The M2 spec states every width and rounding step; an exact `fractions` reference reproduces it bit for bit. A manifest that sets `brass_alpha`, or β ≠ 1, is refused until P1.3 adds fixed-point `exp` and `ln`. **Time of death (amended 2026-10-16, REV-P1a BLK-1):** a death fires at the start of a keyed day (the Day-pulse instant), never at a keyed minute. Bindings run only at their pulse instants (`05.1` §4.3), and M7's immediate succession and `T1.minds`' grief must run in the death's instant. k = 1 draws the day within the month, never the Month-pulse instant itself. Minute-level times wait for BCK-002 A-35. **65%.**
8. **Parameter defaults live in the Registry.** The Registry gives bounds, not defaults, and `solo` sets no M2 parameter. Add `defaults = { … }` beside `params` in `[systems."<id>"]` (a typed-section change, C2), mirrored as `default` in the spec's `[[params]]` and checked by `lint_odd.py`. M2: e₀ = 30, β = 1 (design values inside V-SF-02's band, e₀ ∈ [23.441086, 36.569855] at β = 1, derived exactly from q(15) ∈ [0.35, 0.55] by the M2 reference; `uz_job` already uses 30). **60%.**
9. **The authored-number boundary.** `05.5` §3.2 binds the first sprint that feeds manifest numbers into plane A: this one. `rh_manifest::compile(manifest, registry)` builds a plane-A `rh_registry::WorldSpec`: seed, calendar, horizon, the built and not-built systems, M2 parameters as fixed point, and founders (sex, age) in manifest order. Each number is the exact decimal of its literal, rounded half to even, never through `f64`; a value that does not fit is a load error. Names and ids stay on the tool side (the flavour rule, `05.5` §3): `WorldSpec` holds no string with meaning. **75%.**
10. **The worldgen stub.** New crate `rh-worldgen`, pinned (A, machinery). It writes founders only: no raster, regions, settlements or households (no spec owns them yet; P1.1a). Order: cast first, then household children and servants, then the rest of `population`. IDs are `founder(seed, k)` with the deterministic rehash. Sex as given, else a fair keyed draw. Age as given; else children uniform in [0, 18) years, servants in [15, 60), everyone else in [0, 60): design values, replaced by P1.2. Birth tick = −(age × minutes per year) − a keyed offset inside the year. Draws use the keyed RNG under the reserved id `rh-worldgen`, with purposes in its `rng_purposes.rs`; registration refuses a system id that collides with it. Genesis writes only columns whose owner's `[initialization]` names the manifest section. **60%.**
11. **Verbs, verb ids and roles.** M2 emits `DEATH`, which is not in the Registry's `[verbs]`; `lint_odd.py` does not check `[[emits]]` against `[verbs]`. Add `DEATH` and `EXTINCTION` (act class `death`), both `emitted_by = "M2.demography"`, with a new `tellability` field (0–255; design values 128 and 255). `lint_odd.py` gains the check: an error for a frozen spec, a warning for a draft (S5's `ENCOUNTER` is not a verb yet), and the smoke overlay declares its own kinds. `verb_id` is the verb's index in `[verbs]` sorted by name (TOML tables carry no order), and every sealed segment carries its `registry_version` and a `verb` dictionary table, so segments stay readable after the Registry grows. The deceased fills role `target`; a natural death has no `actor`. SPR-006's untyped `causes` are written as `triggered` edges until P1.9 types them. Every death at rungs 0–2 is admitted under DATA-5 as a tellable act class. **55%.**
12. **EventId layout follows the code** (Architect; SPR-006 already ruled the counter, and the document fix lands before G0). `05.2` §1, `05.9` IF-1 and `schema.sql` say `EventId = (day_index << 24) | seq_in_day`. SPR-006 shipped one counter per timeline, which is hashed and in every keyframe. Keep the counter (the day is derived from `tick`) and amend the three documents and `specs/chronicle/selftest.py`. Changing the code instead is a C3 change to P0 machinery for no gain. **75%.**
13. **World Stream v0 and the chronicle: one writer, `rh-stream`.**
    - A new crate `rh-stream`, pinned (A, machinery), so the forbidden-API gate scans it. It holds the record types, the canonical text form and its parser, the segment writer and the reader. `parquet` (=60.0.0, matching SPR-007's arrow-rs pins; default features off, `zstd` only) is reviewed into `PLANE_A_EXTERNAL_DEPS`: it can carry float columns, and `rh-stream` declares none. The alternative is to put the writer in `rh-snapshot`.
    - Channel A in P1.0 carries `EVENT`, `INPUT`, `STEP_DIGEST` (monthly: living count, deaths in the month, the three plane hashes) and `KEYFRAME`. It also carries two new kinds, `LIFE_ENTRY` and `LIFE_EXIT`, because the fidelity promise (`05.10` §4) needs every birth and death in channel A while DATA-5 keeps commoner deaths out of `event` rows at scale. `05.10` §4 and `05.9` IF-0 are amended.
    - Order within an instant: inputs, events in `EventId` order, life records in `PersonId` order, the digest, the keyframe marker. Genesis writes the founders' `LIFE_ENTRY` records at tick 0 in `PersonId` order.
    - A sealed segment is one directory per (timeline, channel, sim-year) holding one Parquet file per table: `event`, `event_role`, `causal_edge`, `life_course_entry`, `life_course_exit`, `input`, `step_digest`, `keyframe_marker`, `verb`. Every record row carries its `seq`. A DuckDB view joins entry and exit into `schema.sql`'s `life_course`.
    - Sealing is atomic: temporary file, `fsync`, rename. There is no live socket (P1.17); `tail` follows sealed segments.
    - V-STR-01 compares the segment files byte for byte within one build, plus a canonical digest of the records in `seq` order that survives a Parquet library upgrade.

    **60%.**
14. **The sifter in P1.0 runs downstream.** A new crate `rh-sifter` v0, pinned (C, machinery), matches single-clause patterns only (act or verb, roles, declared semantics). `rh-cli sift <run-dir>` reads sealed channel-A segments in `seq` order and writes channel-C segments (`sift_match`). It never runs inside the engine process. The live sifter on a worker thread is P2.1. The pattern is `life_ends` (pack `solitary_life`, which the Registry already lists): one clause, verb `DEATH`, target `A`, `selection = "latest"`, `on_complete = "clear_binding"`, and a firing budget equal to the crude death rate at e₀ = 30. Its DuckDB twin reads the Parquet segments. **65%.**
15. **Material inputs in P1.0.** No player verb exists in P1a. The journal is exercised with scope-only re-sample inputs from a committed script (`rh-cli run --inputs`), journaled through SPR-007, as SPR-008's smoke world does. Each golden world's committed script holds one input, so epoch-keyed draws (BCK-002 A-29) are inside the golden run. **65%.**
16. **Golden worlds and CI tiers.**
    - `worlds/<id>/world.toml` points at the manifest in `specs/manifest/examples/` and pins its SHA-256: one copy, no drift. It also holds the input script and the baselines.
    - PR tier: `solo` end to end (run, tail, sift, twin, replay) with a 200-year yearly ladder at 1 and N threads; `household` and `hamlet` 200-year yearly ladders at 1 thread. This satisfies both GOV-002 tier 9 ("`solo` on every PR") and `docs/13` §4.3 ("200 for the others").
    - Nightly: all three rungs end to end, monthly for 1,000 years at 1, 2, 4 and N threads × 3 repeats (G1a, V-DET-01/02).
    - A run may pass the manifest's horizon; the header records it.

    **65%.**
17. **Extinction.** When `apply_death` empties the ledger, M2 emits `EXTINCTION` once, caused by that `DEATH` (a draft-local cause). The run continues to its requested end, taking hashes as usual (V-SCALE-04). `09`'s phrase "a journaled end event" is read as a channel-A event, not a journal row: the journal holds inputs only. **65%.**
18. **The rung-1 and rung-2 manifests (BCK-002 A-06).** `household.yaml`: one household, a couple and three children, template `pastoral_household`, M2 defaults. `hamlet.yaml`: `uz_job`'s people (population 60, the cast, Job's household with children and servants, e₀ = 30) without the agent, the wager catalyst, the off-map raiders and the overlays, none of which P1a simulates. `uz_job` stays the P1b micro-catalyst world (V-SCALE-05). **70%.**
19. **Scale claims for P1.0.** ADR-0028 ("P1.0 runs at N = 1 and N = 100k from its first week") and `docs/12` Q12 ("1, 60 and 100k") predate ADR-0035's meso default and P1.2's population synthesis. Withdraw the 100k claim. V-SCALE-01 in P1.0 covers N ∈ {1, 5, 60}; 10k comes with P1.14. **80%.**

## P0 carry-forwards this sprint inherits (BCK-002, open items)

| Item | What | Due (BCK-002) | Here |
|---|---|---|---|
| A-14 | Engine-purity lint (V-INSP-05) | P1a | T-00 |
| A-21 | `UniformDraw` for `rh_ids::RngStream` | First sprint that draws against a probability: this one | T-01 (ruling 4) |
| A-29 | `StepCtx` draws keyed by the entity's epoch | Before the first material input that touches a drawing system: this one (ruling 15) | T-02 |
| A-30 | Drop a command for a person who died earlier in the same application | With the first death command | Not due here: P1.0 sends no lifecycle command (ruling 6). Goes to the P1.13 sprint |
| A-31 | Invariant on each command queue's age or length | With the first real lifecycle command | As A-30 |
| A-28 | Exact `start_of_day` and `previous_pulse` lags | Before S5, S6 or S7 runs | Not due: M2 reads nothing. Goes to the P1.18 sprint |
| A-32 | Per-tick allocations driven to the `08` budget | Before benches 1 and 2 | Not due. T-13 records V-PERF-11 counts for the rungs as their budgets; driving them down is P1.14 |
| A-22 | `f32_field` threshold rule | First sprint with an `f32_field` | Not due: no raster here (ruling 10) |
| A-24 | Type the saga vocabulary tables | Before S6, S7, T1 | Not due |
| A-23 | YAML flow-continuation indentation | When a hand-written manifest hits it | Possible in A-06's manifests; fix the manifest, not the loader |
| A-06 | Rung 1–3 manifests (Architect) | P1a | Blocker 4, before increment C |
| A-15 | Coverage gates, owned by no sprint | Since SPR-002 | Still unowned. Recommendation in the P1a sprint map |
| SPR-007, carried | `KeyframeStore` LRU capacity defaults to 8 "until P1 measures keyframe sizes" | P1 | T-13 reports keyframe sizes per rung |
| SPR-006 ruling | "Replay refuses a run with material inputs until the journal is wired in" | SPR-007 wired it | T-12 replays through `rh_snapshot::resume` |

## Specs this sprint needs

| Spec | Exists? | Gap | Writer | Needed by |
|---|---|---|---|---|
| `specs/systems/M2.demography.odd.toml` | Draft (rung-0 slice) | Owns no `person.id` or `life_course.*`; no `EXTINCTION`; no defaults; the mortality algorithm is one sentence (ruling 7 states it); `INV-M2-01` cites V-DET-01 where V-INV-01 is meant; no `resample_on_fork` flag on `death_time` | Architect | T-05, T-07 |
| `specs/manifest/registry_v0.toml` | Exists | No `DEATH` or `EXTINCTION` verb; verbs have no `tellability`; systems have no `defaults` | Architect (C2) | T-04, T-07 |
| `specs/manifest/examples/solo.yaml` | Exists | None (ruling 3 covers its unbuilt systems) | — | T-08 |
| `household.yaml`, `hamlet.yaml` | Missing | BCK-002 A-06 (ruling 18) | Architect | T-13 |
| `specs/chronicle/schema.sql` | Exists | EventId comment (ruling 12); `life_course` entry/exit split (ruling 13); `concrete_role` codes (initiator, target, partner, bystander, instrument) differ from the Registry's `[event_roles]` (actor, target, witness, instrument); `regional_aggregate.value` is `DOUBLE` in a plane-A table | Architect | T-09 |
| `specs/stream/channel_a_v0.sql`: segment DDL, record header, kinds, in-instant order | Missing (`05.10` §5 and `05.9` IF-0 are prose) | Ruling 13 | Architect | T-09 |
| `specs/sifting/patterns.solitary_life.toml` (`life_ends`) | Missing | Ruling 14 | Architect | T-11 |
| `docs/05.5` §3.2 | Exists | — | — | T-04 |
| Probability type | Missing | Ruling 4 | Architect | T-01 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C3.** The first real system crate (`rh-sys-demography`) and three new golden worlds; determinism machinery (probability type, epoch-keyed draws, ledger compaction, the channel-A record format). Contains C2 changes: the frozen M2 spec, Registry verbs and defaults, the segment schema, four new crates, one new plane-A dependency |
| Planes touched (ADR-0024) | A (`rh-core`, `rh-registry`, `rh-worldgen`, `rh-sys-demography`, `rh-stream`); C (`rh-sifter`, channel-C segments); tool (`rh-manifest`, `rh-cli`, `bin/`, `specs/`). B: none |
| Owned columns written | `M2.demography`: `person.id`, `person.birth_tick`, `person.sex`, `person.alive`, `person.death_tick`, `life_course.*` (after the spec freeze). `rh-core`: row compaction of `person.*` (ruling 5). `rh-worldgen`: initial values only (ruling 10) |
| Reads (via `rh-contracts`) | None: the rung-0 slice reads no other system |
| RNG purposes added | M2 `death_time` (person, month; k = 0 the monthly Bernoulli, k = 1 the minute), flagged `resample_on_fork` (a vital timer, `05.1` §6 item 9). `rh-worldgen`: `founder_sex`, `founder_age`, `founder_birthday` (founder, genesis). Re-sampled draws at the entity's epoch (T-02) |
| Golden-hash effect | New baselines for `solo`, `household`, `hamlet`; `smoke` regenerated by T-02 (A-29) with its bisect report. All announced to the director before merge |
| Fixed-point quantities | Probabilities (`Prob32`, ruling 4); K, O(x) and l(x) at the widths the M2 spec states; e₀ and β compiled from the manifest (ruling 9) |
| Determinism lints (GOV-003 §2, §6) | Must pass. New: the engine-purity rule (T-00); pinned labels for `rh-worldgen`, `rh-stream`, `rh-sifter`, `rh-sys-demography`; `parquet` in `PLANE_A_EXTERNAL_DEPS` |
| Tests that gate this sprint | V-INSP-05, V-DET-01, V-DET-02, V-DET-04, V-DET-05, V-DET-12, V-DET-14, V-DET-18 (subset), V-DET-19, V-INV-01 (subset), V-INV-02 (nightly), V-INV-08, V-INV-11, V-DATA-04 (subset), V-DATA-05, V-DATA-10 (subset), V-STR-01, V-SIFT-01, V-SIFT-05, V-PLN-01, V-SCALE-01, V-SCALE-04, V-GOLD-01, V-WC-04 (new manifests), plus `bin/check_all.sh` |
| Docs updated in the same sprint | `bin/check_all.sh` and `CLAUDE.md` checks; `05.0` §6 (new crates); `05.2` §1, `05.9` IF-0 and IF-1, `05.10` §4–5, `schema.sql` (rulings 12, 13); `docs/09` rows (V-SCALE-01's N set, V-STR cadence); `docs/11` rung 0 (ruling 2); the M2 brief |

## Tasks

### Increment A: engine prerequisites (SPR-011 under ruling 1; about 3 agent-days)

#### T-00: Engine-purity lint (BCK-002 A-14)
- **Deliverable:** Rule `engine-purity` in `bin/lint_arch.py`. One deny-list constant (media, HTTP-UI and LLM-client crates, as `docs/13` §3 lists), checked over the transitive normal and build dependencies of every engine crate. The engine set is the union of the lists in `docs/13` §3, `05.0` §6 and ADR-0035 §6, plus `rh-sys-*` and the new crates. Planted fixtures in `tests/lint_fixtures/`; Python tests; the gate inventory test updated.
- **Acceptance:** V-INSP-05: zero violations on the workspace. A planted direct dependency on a deny-listed crate FAILS; a planted transitive one FAILS; a dev-dependency passes. A test proves the deny-list is defined once.
- **Status:** [ ] not started

#### T-01: A finer probability type and keyed uniform draws (BCK-002 A-21; ruling 4)
- **Deliverable:** `Prob32` in `rh_core::fixed`, with `draw_hits` and `chance` on it; `UniformDraw` for `rh_ids::RngStream`; a `StepCtx` helper that draws one keyed `u64` against a probability. `bin/gen_fixed_vectors.py` extended (exact rational reference) and its golden file regenerated.
- **Acceptance:** Golden vectors bit-equal (`gen_fixed_vectors.py --check`). P(hit) = bits / 2³² exactly, by the reference. One draw is consumed per call whatever p is (CRN). Compile-fail controls: no implicit mix with `Q16_16`; no float conversion. V-DET-19 clean. Every existing ladder unchanged.
- **Status:** [ ] not started

#### T-02: Draws keyed by the entity's epoch (BCK-002 A-29)
- **Deliverable:** `StepCtx::rng` and `StepCtx::stream` key every draw by the drawn entity's current epoch (ADR-0025; `05.1` §6 item 2). `ResampleCtx` is unchanged. The smoke baseline is regenerated with SPR-008's tools; the bisect report goes in the PR.
- **Acceptance:** V-DET-14: after a scoped input, only entities in the scope draw differently, from the input's instant on; entities outside keep bit-equal draws (property test over random scopes). An entity at epoch 0 draws exactly as in P0. The bisect report puts smoke's first divergence at or after its first synthetic input, never earlier.
- **Status:** [ ] not started

#### T-03: Ledger rows and the life-course log (ruling 5)
- **Deliverable:** In `rh-core`: the ledger exists when `person.id` is registered. All `person.*` columns stay aligned and sorted by `person.id`. A compaction pass at the start of `Bookkeeping` removes every row with `person.alive = 0` from all `person.*` columns. `life_course.*` columns are append-only, and a row's exit fields are written once (a core check refuses anything else). Core invariants: alignment, order, and one open life-course entry per ledger row. Compaction removes rows from `person.*` columns that other systems own (`person.named` for T1, `person.house` and `person.vocation` for M7): this is a core-only exemption from single-writer ownership, and no `SystemModel` can obtain it. The life-course log keeps `mother` and `father` for everyone who ever entered, so kinship stays readable through the dead (REV-P1a NEW-1).
- **Acceptance:** V-INV-01 subset as property tests on random small worlds: alignment; order; the living count equals the open entries; a closed entry is absent from the ledger. A death applied in `Vital` is gone before `Bookkeeping` hashes the same instant. V-DET-05: keyframe round-trip with a compaction inside the window. V-DET-01/02 on a stub world with deaths. The smoke ladder is unchanged. V-INV-04 positive control (REV-P1a MIN-9): a test system that removes a ledger row, or writes a `person.*` column it does not own, is refused at registration or at the step with a named error, while the core compaction of the same rows passes.
- **Status:** [ ] not started

#### T-04: Authored numbers into plane A (`05.5` §3.2; rulings 8, 9)
- **Deliverable:** `rh_registry::WorldSpec` (plane-A types only) and `rh_manifest::compile(manifest, registry) -> WorldSpec`. Registry `defaults` are read (ruling 8). Exact decimal to fixed point, rounded half to even, never through `f64`; a value that does not fit is a load error. `rh-manifest` gains its edge to `rh-registry`; no plane-A crate depends on `rh-manifest`.
- **Acceptance:** A property test against a Python `decimal` reference over random literals, halfway cases included, gives the same bits. Out-of-range and non-fitting literals are refused by name. The plane-A-closure lint is clean. `solo` and `uz_job` compile. V-WC-04 parity is unchanged.
- **Status:** [ ] not started

### Increment B: `solo` lives and dies (SPR-012 under ruling 1; about 3 agent-days)

#### T-05: The M2 rung-0 schedule in fixed point (ruling 7)
- **Deliverable:** In `rh-sys-demography`: the Brass General Standard as exact decimals, K from e₀ by bisection, l on the monthly grid, monthly death probabilities as `Prob32`. `bin/gen_m2_reference.py` (Python `fractions`, the same algorithm and rounding) with `--check`, and golden vectors in `crates/rh-sys-demography/tests/golden/m2_schedule_v1.txt`.
- **Acceptance:** Bit-equal to the reference at e₀ ∈ {20, 23.441086, 30, 36.569855, 43.45915, 60, 85}. At the standard's own e₀ (43.45915, the trapezoid on the standard grid; corrected from 43.44 by the skeleton-spec drafter), K = 1 within one unit. At the e₀ values that Brass Table 19's α rows imply, l(x) matches Table 19 within 1 per 10,000 (the `selftest_brass.py` tolerance). The e₀ recomputed from the fixed-point schedule is within 0.06 years of the input. q(15) ∈ [0.35, 0.55] exactly when e₀ ∈ [23.441086, 36.569855] (V-SF-02's identity at the schedule level).
- **Status:** [ ] not started

#### T-06: The `rh-worldgen` stub (ruling 10)
- **Deliverable:** Crate `rh-worldgen`, pinned (A, machinery), with `genesis(&WorldSpec) -> WorldTables`: founders into M2's ledger columns and life-course entries (origin founder, pre-epoch birth ticks), keyed draws, `rng_purposes.rs`. No raster, regions, settlements or households.
- **Acceptance:** V-DATA-05: every founder id equals `founder(seed, k)` or its deterministic rehash, in ruling-10 order; a forced collision resolves the same way on every run. The same `WorldSpec` gives bit-equal tables twice and on 1 and N threads. V-INV-11: negative birth ticks round-trip through a keyframe. `solo` gives Ada: female, aged 25 at tick 0. A planted write to a column whose owner does not initialise from the manifest is refused.
- **Status:** [ ] not started

#### T-07: `rh-sys-demography`, the rung-0 slice
- **Deliverable:** System `M2.demography`, with the skeleton from `lint_odd.py --rust` brought in by `include!`, bound to Month/Vital. For each living person: one Bernoulli on the month's probability (k = 0); on a hit, a death timer at the start of a keyed day inside the month (k = 1), never at the Month-pulse instant (ruling 7, amended). `fire` calls `apply_death` (ruling 6): alive flag, death tick, life-course exit with cause `natural`, `DEATH` with the deceased as `target`. `EXTINCTION` once when the ledger empties (ruling 17). `resample_timer` re-draws a pending death at the new epoch. Invariants `INV-M2-01` (conservation) and life-course consistency.
- **Acceptance:** V-DET-12: every event's tick ≥ its instant, ids strictly increase, a sampled death sits in the agenda as a timer until it fires. V-INV-01 subset after every pulse on random small worlds. V-INV-08: Bernoulli draws only (a grep gate: no Poisson death draw). Registration rule `spec` passes against the frozen spec. M2 refuses `brass_alpha` or β ≠ 1 at construction with a named error (ruling 7). A fixture `WorldSpec` of 10,000 founders aged 0 gives, over 32 seeds, an observed q(15) inside the schedule's value ± its binomial 99.9% band; the band harness itself (V-SF-02, V-SCALE-02) is P1.10. V-DATA-04 subset on the same fixture over 1,000 years: every `PersonId` ever emitted has exactly one life-course entry with founder provenance, and a death record if and only if it is absent from the ledger (residence and occupation intervals wait for S3 and M4).
- **Status:** [ ] not started

#### T-08: `rh-cli run <world>` and the `solo` golden ladder (rulings 3, 16)
- **Deliverable:** SPR-008's runner generalised to real worlds: compile, genesis, register the built systems against `specs/systems/`, run with SPR-007's journal and keyframe store (10-year cadence), write the ladder. The run header carries the history tuple and the built and not-built systems. `worlds/solo/` with `world.toml`, the input script and baselines. PR and nightly jobs. SPR-008's baseline-integrity gate covers `solo`.
- **Acceptance:** V-DET-01/02 on `solo`: monthly at 1, 2, 4 and N threads × 3, nightly; yearly at 1 and N threads on PRs. V-GOLD-01: the PR ladder equals the committed baseline. SPR-008 ruling 6 controls: a hand-edited `solo` baseline FAILS; a regenerated one in a PR not titled `[C3]` FAILS. `--strict` refuses `solo` (ruling 3). V-SCALE-03: the 1,000-year wall time of `solo` is reported. V-PERF-04: the PR tier stays within 15 minutes.
- **Status:** [ ] not started

### Increment C: the stream, the sifter, replay and the ladder (SPR-013 under ruling 1; about 3.5 agent-days)

#### T-09: `rh-stream`: channel A and the sealed chronicle (rulings 11, 12, 13)
- **Deliverable:** Crate `rh-stream`, pinned (A, machinery), with `parquet` reviewed in. The record header and the P1.0 kinds; the in-instant order; per-sim-year sealed segments matching `schema.sql` and `specs/stream/channel_a_v0.sql`; atomic sealing; the per-segment canonical digest; the lossless one-line canonical text form and its parser. The host writes after each instant, from `Engine::drain_events`, the journal and the life-course delta.
- **Acceptance:** V-STR-01 on `solo` and a 60-person fixture: segments byte-identical across two runs and across 1 and N threads, and canonical digests equal. V-DATA-10 subset: a Python test reads every sealed table with DuckDB, with `schema.sql`'s types. `seq` is contiguous from 0 across segments. The canonical form round-trips every record (property test; the parser joins the nightly fuzz tier). A `kill -9` during sealing leaves no partial segment, and the resumed run reseals the same bytes (GOV-002 tier 15). No channel-A table has a float column.
- **Status:** [ ] not started

#### T-10: `rh-cli tail`
- **Deliverable:** `rh-cli tail <run-dir> [--from-seq N] [--follow <person>] [--kind K …] [--canonical] [--wait]`. It reads sealed segments in `seq` order and follows new ones as they seal. It prints the display line (date in the world calendar, kind, roles, causes; cast names from the manifest, tool side) or the canonical line, and the status line of `05.10` §2: last sealed tick and `seq`, the cursor, the lag. Channel-C matches appear as annotations when a channel-C segment exists.
- **Acceptance:** On a `solo` run, every channel-A record prints exactly once, in `seq` order, from any `--from-seq`. A tail started mid-run and one started after the run print the same lines (V-STR-05 for sealed segments). Tail never writes to the run directory (a read-only test). A stopped tail does not slow `rh-cli run` (wall time against headless, reported). `insta` snapshots of the display form.
- **Status:** [ ] not started

#### T-11: The trivial sifter pattern and its twin (ruling 14)
- **Deliverable:** `life_ends` in `specs/sifting/patterns.solitary_life.toml` (Architect). Crate `rh-sifter` v0, pinned (C, machinery). `rh-cli sift <run-dir>` writes channel-C segments. The twin in `specs/sifting/twins/solitary_life.sql`, checked by `python -I specs/sifting/twin_check.py <run-dir>` (AGT-002-TOOLS).
- **Acceptance:** V-SIFT-05: `lint_patterns.py` passes `life_ends` and rejects a planted unanchored variant. V-SIFT-01: `rh-sifter`'s matches equal the twin's (match ids, bindings, event ids) on `solo`, `household`, `hamlet` and the 10,000-founder fixture. V-PLN-04 subset: sifting twice gives the same channel-C digest and leaves channel A and the state ladder unchanged. V-PLN-01: no plane-A crate depends on `rh-sifter`.
- **Status:** [ ] not started

#### T-12: Journal replay, end to end (ruling 15)
- **Deliverable:** `rh-cli run --inputs <file>` journals scope-only re-sample inputs through SPR-007's journal. `rh-cli replay <run-dir>` rebuilds from the history tuple and the journal alone. `rh-cli replay --from-keyframe <tick>` resumes through `rh_snapshot::resume`. `worlds/solo/inputs.toml` holds one input on Ada.
- **Acceptance:** V-DET-04: the replayed ladder and channel-A digests equal the original, every month. V-DET-18 subset: a replay in a clean directory holding only (build, Registry, manifest, seed, journal) is hash-equal, and every journal row has tick, seq, class, scope and producer. V-DET-05 on `solo` and `hamlet`: resume from the year-10 keyframe, advance 10 years, equal to the continuous run. A missing, extra or edited journal row hard-fails replay. V-DET-14 on `solo`: the input re-samples only Ada, keyed by her new epoch.
- **Status:** [ ] not started

#### T-13: Rungs 1 and 2 on the same binary (rulings 16–19)
- **Deliverable:** `worlds/household/` and `worlds/hamlet/`, pointing at A-06's manifests, with input scripts and baselines. The PR-tier and nightly jobs of ruling 16. Nightly V-SCALE jobs. The keyframe-size report.
- **Acceptance:** One `rh-cli` build runs all three rungs; only the manifest changes. V-SCALE-01 at N ∈ {1, 5, 60}: 0 invariant violations, equal hashes across threads, exact replay. V-SCALE-04: each world reaches zero population, emits one `EXTINCTION`, and its chronicle, tail and replay stay consistent after it. V-WC-04: both manifests validate in Python and in Rust (`manifest_parity.py`). V-DET-01/02 nightly on all three rungs. V-INV-02: a 1,000-year debug soak with invariants on, 0 violations. V-PERF-11: steady-state allocations per tick recorded for each rung as its budget (needs SPR-009; otherwise Blocker 1 stays open for this line). Keyframe sizes per rung reported.
- **Status:** [ ] not started

## Out of scope (explicit)
- Births, partnering, households and kinship (P1.3); fixed-point `exp`/`ln` and Brass at any (α, β) (P1.3).
- Regions, settlements, routes and residence (P1.1a, `S3.migration`); place on events stays empty.
- The event envelope: anchored snapshots, typed and motive edges, the reconciled role codes (P1.9).
- The live socket, cursors as a protocol, `stream.subscribe`, the materializer, pace commands, the TUI stub (P1.17).
- The live in-process sifter and multi-clause patterns (P2.1).
- Same-instant command delivery, BCK-002 A-30 and A-31 (P1.13); exact lags, A-28 (P1.18).
- Food (P1b); daily life (P1.18, then Later for `minutes`); worlds above 60 persons (P1.14).

## Director approvals (given at G0, 2026-10-16)
1. **The plan, as C3** (GOV-005 §2): **approved (D30)**, with the C2 changes inside it: the frozen M2 spec (ownership of `person.id` and `life_course.*`), the Registry additions (`DEATH`, `EXTINCTION`, verb `tellability`, system `defaults`), the channel-A segment schema and the `LIFE_ENTRY`/`LIFE_EXIT` kinds, four new crates (`rh-worldgen`, `rh-sys-demography`, `rh-stream`, `rh-sifter`) and one new plane-A dependency (`parquet`).
2. **The golden-corpus change**, still announced before each merge with its evidence, with a veto: `smoke` regenerated (A-29, increment A); `solo` (increment B); `household` and `hamlet` (increment C).
3. **Rulings that change the roadmap or an earlier statement:** 1 (three sprints; approved, D30), 2 (rung 0 in P1a without food or minute-level daily life; approved, D31), 19 (the 100k claims of ADR-0028 and `docs/12` withdrawn; approved, D32). Ruling 12 is the Architect's: the documents are corrected to match SPR-006's audited code.

The other rulings are the Architect's.

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | T-01–T-03, T-06–T-08, T-12, T-13: V-DET-01/02/04/05/12/14/18, V-STR-01, V-SCALE-01/04, epoch-keyed draws, ledger compaction. Written from `05.1` §6, ADR-0025, ADR-0026 and `05.10` §7 before reading the implementation | `VER-SPR-011` (and `-012`, `-013` under ruling 1) |
| AGT-003-IF | T-00, T-04, T-09–T-11: the purity lint's controls; the authored-number boundary (`05.5` §3.2); segment conformance with `schema.sql` and the segment DDL; the canonical form and its parser (fuzz); `tail`; the twin (V-SIFT-01, V-SIFT-05); registration against the frozen M2 spec | As above |
| AGT-003-STAT | T-05, T-07: the fixed-point schedule against the exact reference and Brass Table 19; the 10,000-founder q(15) check against the schedule. Written from `specs/demography/` and R12 §9 | As above |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| 1 | G0 not passed: `VER-G0` is a draft; SPR-007 merged 2026-10-16; SPR-008 and SPR-009 in progress | `VER-G0` | Open |
| 2 | M2 spec incomplete and not frozen (see "Specs this sprint needs") | Spec work, Architect | Open |
| 3 | Registry lacks `DEATH`, `EXTINCTION`, verb `tellability` and system `defaults`; `lint_odd.py` does not check `[[emits]]` against `[verbs]` | Spec work, Architect (C2) | Open |
| 4 | `household.yaml` and `hamlet.yaml` do not exist | BCK-002 A-06 | Open (before increment C) |
| 5 | `life_ends` and the channel-A segment DDL do not exist | Spec work, Architect | Open (before increment C) |
| 6 | Contract documents disagree with the code on `EventId` and with the Registry on role codes | Ruling 12 (Architect's consistency fix before G0); P1.9 for roles | Open |
| 7 | Director approval of the plan and of rulings 1, 2, 19 (D30 to D32) | This document | Closed: approved 2026-10-16 |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-011` is PASS (and `-012`, `-013` under ruling 1).
- [ ] No open DEF- against this sprint.
- [ ] Merge route matches the CI-detected change class; each golden-corpus change was announced to the director first.
- [ ] Rung checkpoints (GOV-007 §3.5): when `solo`, then `household`, then `hamlet` first runs green, the director watches its stream through `rh-cli tail` for 15–30 minutes. The Architect records the notes as `EVO-` or `DEF-` items.
