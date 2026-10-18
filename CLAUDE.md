# CLAUDE.md: guidance for AI agents working on Rehoboam

## Project state
P0 (foundations) is under way. SPR-001 created the Cargo workspace (stub crates in `crates/`), the Python tooling (`python/`), the architecture lints and CI. The plan lives in `docs/`, the evidence in `research/`, and executable contracts in `specs/`.

## Operating model (ADR-0032)
Day-to-day work runs through `CODEX/`: roles (`CODEX/80_AGENTS/`), sprints and backlogs (`CODEX/05_PROJECT/`), governance (`CODEX/10_GOVERNANCE/`), and the generated index (`CODEX/00_INDEX/MANIFEST.yaml`: run `python -I bin/gen_manifest.py` to create it; it is not committed). Start each session with `AGENTS.md`. Run `bin/check_all.sh` before asking for an audit; no report, no pass. Be direct: no flattery, no softened bad news; bring the director decisions with a recommendation and a confidence rating, not open questions.

## Read before any work
1. `docs/00_executive_summary.md`
2. `docs/01_vision_and_intent.md` (success criteria S1–S4, principles)
3. The ADRs relevant to your task (`docs/06_adr/`)
4. `docs/13_agentic_development_playbook.md` (work-package template, Definition of Done, change classes)
5. `docs/14_reuse_inventory.md` before writing anything from scratch: check whether a permissive project already solves it (PORT or DEPEND)

## Non-negotiables (from ADRs; changing them needs director approval)
- **The simulation decides; LLMs describe.** LLM output enters state only as a journaled, schema-validated input (ADR-0006, ADR-0011, ADR-0034); LLMs never choose actions and the engine never waits on a model.
- **Determinism.**
  - History = f(engine_build, registry_version, manifest_hash, seed, input_journal).
  - Use keyed counter-based RNG only: `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)`.
  - Threshold quantities must be fixed-point.
  - No HashMap iteration and no float-order-dependent reductions in authoritative code.
- **Three planes (ADR-0024).** Plane A (authoritative) never reads plane B (observation: pins, facts ledger, hydrated views) or plane C (narrative: sift matches, significance, visibility). The only way into plane A is a journaled material input.
- **Scheduled outcomes, no back-dating (ADR-0025).** Outcomes are timers that fire for everyone, watched or not. EventId order is emission order. Acting in the past forks.
- **Single-writer column ownership.** Each column has one owning system; others read it through contract crates and request changes through lifecycle command queues (`05.1` §2).
- **No physical compaction of named-actor history.** Hide low-salience events with the visibility sidecar instead (ADR-0007).
- **Sifting patterns must anchor every state read** (`attr@eN`) and declare their match semantics and firing budget. Use `specs/sifting/lint_patterns.py`.
- **Mechanics come only from the Registry.** No free-string semantics in manifests (ADR-0010).
- **Floats in plane A live in `rh-sample` only** (SPR-003 Architect ruling): the samplers compute in `f64` with `libm` (never `std` transcendentals, never `mul_add`) and return integers or `rh_core::fixed` values.
- **Licences: personal-use mode** (ADR-0021 as amended, D19, 2026-10-16). Rehoboam is a private experiment that is never distributed, so any licence may be used. Two limits remain: no licence whose terms forbid the use itself, and no copying of unlicensed code. The repository stays private. `bin/check_all.sh` keeps a licence ledger (`artifacts/licence_ledger.txt`) that never fails a build; crates come from crates.io only, with no wildcard versions.

## Checks to run after any change
All of them, fail-fast, with a report: `bin/check_all.sh` (CI runs its three parts, `--part core`, `--part golden` and `--part stream`, as parallel jobs; no flag runs all three). It uses `python/.venv` (create it with `uv sync --frozen --project python`; `PYTHON` overrides) and needs uv, cargo-nextest, cargo-deny, valgrind and iai-callgrind-runner (RUN-001). Each gate also runs on a planted violation that must FAIL (`tests/lint_fixtures/`). Individually:
```bash
# Rust: workspace in crates/, toolchain pinned in rust-toolchain.toml
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny --locked check bans sources            # crates.io only, no wildcards (licences: ledger only, ADR-0021 amended)
cargo nextest run --workspace --locked
cargo test --doc --workspace --locked             # doctests: rh-ids compile-fail cases, counted per file by check_all (SPR-002)
cargo nextest run -p rh-ids --run-ignored only --test stream_independence   # V-DET-20 full sweep, 10^6 draws (nightly)
# Architecture lints: stdlib Python
python -I bin/lint_arch.py        # crate boundaries (V-INSP-03), plane isolation (V-PLN-01), engine purity (V-INSP-05, SPR-011)
python -I bin/lint_forbidden.py   # forbidden APIs in plane-A crates (V-DET-19), floats outside declared f32_field columns, file clocks, std::process::Command and file I/O outside FILE_IO_CRATES included (A-44)
# Python: configuration in python/pyproject.toml, run from the repository root
python -m ruff check --config python/pyproject.toml .
python -m mypy --config-file python/pyproject.toml
python -m pytest -c python/pyproject.toml --rootdir=.
# Specs
python -I bin/gen_manifest.py --check
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/solo.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/aethelgard.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/uz_job.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/household.yaml   # rung 1 (BCK-002 A-06)
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/hamlet.yaml      # rung 2 (BCK-002 A-06)
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/manor.yaml       # the SPR-019 fixture: a dynastic manor between rung 2 and SPR-030's fixture world
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/twin_vales.yaml  # the SPR-030 dynastic fixture world: 1,500 persons, two polities, five great houses, 300 years (specs/worldgen/fixture_worlds.md)
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/kingdom_10k.yaml # the SPR-030 G1a performance world: 10,000 persons, three polities, twelve houses, 1,000 years
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml  # must FAIL
python -I bin/gen_rng_golden.py --check  # rh-ids golden vectors match the independent reference (SPR-002)
python -I bin/gen_hash_golden.py --check  # rh-snapshot plane-hash and XXH3 golden vectors match the independent stdlib reference (SPR-007)
python -I specs/chronicle/selftest.py
python -I specs/chronicle/selftest_envelope.py   # the event envelope v0: anchored snapshots, typed edges, cause_told, the plane-B index, the byte budget (SPR-020; specs/chronicle/envelope_v0.md); --control snapshot-drift, motive-on-triggered, cause-later, budget-bloat and origin-lost must FAIL
python -I specs/annals/fixtures/gen_fixtures.py --check   # the annals fixture chronicles equal the scripted history derived from M2's and M7's rules (SPR-029; never hand-edited)
python -I specs/annals/selftest_annals_v0.py --check   # family annals v0 (SPR-029; specs/annals/annals_v0.md): templates over the envelope, calendar dates, designation by name or kinship, grouping by house, line and year, the provenance sidecar re-derived by a whole-group re-render (V-NAR-05), no model (V-NAR-07), a possible history under M7's rules, determinism in two processes, a canonical extract digest, golden annals and manifests of the fixture chronicles with no orphan golden file; --write regenerates the goldens (never by hand); --control no-event-id, row-drops-own, body-as-prologue, missing-kf-source, unrelated-event, rows-swapped, prologue-altered, foreign-root, forged-source, text-truncated, undeclared-slot, state-read, network-import, shared-name, fixture-deposed-day-one and fixture-claim-untriggered must FAIL
python -I specs/annals/lint_templates.py   # every annals slot resolves to a role, snapshot, payload slot, motive or edge the emitting [[emits]] declares; every in-scope kind has a template whose last variant alone is unconditional; every out-of-scope kind a label; enum ids equal the emitter's codes; each prologue template keeps to its own vocabulary and known id; no control characters; seventeen controls rendered by check_all from the real set, one diagnostic each (SPR-029)
python -I bin/gen_fixed_vectors.py --check   # rh-core::fixed golden vectors match the exact rational reference (SPR-010)
python -I specs/registry/effect_ref.py --check   # effect language v0: the reference reproduces every golden case (SPR-017 contract)
python -I bin/gen_decimal_vectors.py --check   # authored numbers: rh-manifest exact decimals -> fixed point match Python decimal (SPR-011 T-04)
python -I specs/numerics/exp_ln_v0.py --check   # fixed-point exp and ln v0 (Q32_32): constants, convergence, faithful rounding, golden vectors (05.1 §6 item 4 v1.10; SPR-016); --exhaustive nightly
python -I bin/gen_sampler_reference.py --check   # rh-sample's exact reference distributions are current (SPR-003)
cargo nextest run -p rh-sample --locked --release --test golden   # V-DET-09 sampler golden vectors at opt-level 3 (SPR-003 T-05)
cargo nextest run -p rh-sample --run-ignored only --test stats   # sampler statistics, nightly tier (64x the PR samples)
python -I specs/demography/selftest_brass.py
python -I specs/demography/selftest_m2_rung0.py   # M2 rung-0 mortality: the exact reference of [algorithm.mortality_rung0] (SPR-011)
python -I bin/gen_m2_reference.py --check   # rh-sys-demography's rung-0 schedule vectors = the exact reference (SPR-012 T-05)
cargo nextest run -p rh-sys-demography --locked --test schedule_golden   # the fixed-point schedule: bit-equal, Table 19, e0 bounds, the q(15) band (SPR-012 T-05)
python -I bin/gen_m2_draws_reference.py --check   # M2's death instants on fixture worlds from an independent model of S8-S12 on the RNG and schedule references, every re-sample case covered (SPR-012)
cargo nextest run -p rh-sys-demography --locked --test draws_golden   # the engine's death instants = that model's; --features mutant-resample-hit-k1 or mutant-resample-day-k0 must FAIL (SPR-012)
cargo nextest run -p rh-worldgen --locked --test genesis   # worldgen stub: V-DATA-05 founder ids, bit-equal genesis, refused writes (SPR-012 T-06)
cargo nextest run -p rh-worldgen --locked --test keyframe --test m2_system   # V-INV-11 birth ticks through a keyframe; M2: V-DET-12, V-INV-01, EXTINCTION, re-sampling (SPR-012 T-06, T-07)
cargo nextest run -p rh-worldgen --locked --release --test m2_system   # also the 10,000-founder q(15) band over 32 seeds and V-DATA-04 over 1,000 years (release only)
python -I specs/demography/selftest_m2_f1.py      # M2 F1 fertility and partnering: the exact reference of [algorithm.fertility_f1] and [algorithm.partnering_f1] in the next-version spec (SPR-015)
python -I specs/systems/lint_odd.py --next specs/systems/next --overlay specs/systems/next/registry_delta.toml   # the next version of a frozen spec (M2 F1 draft 0.3.0) supersedes it for the run and must preserve the frozen slice (SPR-015; specs/systems/next/README.md); its [[emits]] roles and motives may only grow, and snapshots, default and cause_column stay equal unless an [[amends]] record changes them (SPR-020; REV-SPR-020 R15); deny by default (PR #79's review R3): every other field of the frozen slice (submodel summaries, command why, couplings, params source, fidelity ladder, purpose, isolation, design, scales, the prototype, the requirement ids) is equal or amended on its generic path, with seventeen controls rendered by check_all from the M7 draft that must each FAIL (one per path class, the reviewer's q5_summary, q5_why and q5_frozen_onbirth, and round 2's enum code renamed, drained_by moved, from_manifest edited and a read's lag changed; the exemption list is in specs/systems/next/README.md)
python -I specs/worldgen/selftest_worldgen_lite.py   # worldgen-lite v0 and S3 F0 residence: the exact reference of M1's [algorithm.worldgen_lite] and S3's [algorithm.residence_f0], golden outputs in specs/worldgen/golden/ (SPR-018)
python -I specs/worldgen/selftest_fixture_worlds.py --tier pr   # SPR-030's fixture worlds at genesis (twin_vales seeds 1-3 and kingdom_10k seed 1 in the PR tier; every case nightly), built by the SPR-019 reference: counts, houses with their members' founder indices, title and tenure lines, each title's line with its heir and second, the named actors at start under M7's rule in the narrow and the full reading, in specs/worldgen/golden/fixture_worlds_v0.txt; --write regenerates; --altered-golden PATH must report STALE in the same run; --control dormancy-one must FAIL (REV-SPR-030 R6, R10, R11)
python -I specs/worldgen/project_population.py --check   # the fixture worlds' population projection with the house-member mechanics (elite_fertility_mult, vocation_holy_orders) under the three membership rules of BCK-002 A-56 (cognatic, agnatic, bounded), in M7's isolation regime and the T1-registered regime, with the levers of PR #79's review (later-children vocations, the elite read on the couple, hypergamy): member share, members and population over each horizon, uncapped and capped (REV-SPR-030 R1; specs/worldgen/fixture_worlds.md §3.3; specs/systems/next/M7_membership.md); --derive-caps recomputes the named core from the fixture reference
python -I specs/determinism/selftest_crn_pairing.py --seeds 1 --controls   # V-DET-17 CRN pairing v0 (specs/determinism/crn_pairing_v0.md): paired 10-year rollouts of twin_vales differing by one scope-only input inside the month of the sovereign's run-A death; the target chosen per seed from run A (the primogeniture sovereign, else the tanistry one, else the seed is pinned as skipped), the pair must be VALID with a five-year window and 100 informative pairs, every unaffected death instant identical, rho reported against 0.9; five mutants on the same run A must each FAIL for their own reason; seeds 1-3 nightly; in specs/determinism/golden/crn_pairing_v0.txt
python -I specs/worldgen/selftest_population_synthesis.py   # population synthesis v0 and the dynastic back-fill: the exact reference of specs/worldgen/population_synthesis.toml [algorithm.population_synthesis] and [algorithm.dynastic_backfill], golden outputs in specs/worldgen/golden/ (SPR-019); --control spacing, --control rounding-cells and --control rounding-table must FAIL, and --spec on a copy with a stale check value must FAIL; with --effect-ref specs/registry it also runs the differential gate against effect_ref.py and, on the same genesis, its reversed-line control, which must differ (REV-SPR-019 R3; REV-SPR-030 R27; in check_all since PR #41 merged)
python -I specs/demography/selftest_m2_f1_mortality.py --check   # M2 F1 mortality: the exact reference of the M2 spec's [algorithm.mortality_f1] (specs/systems/next while it is a draft; SPR-016); --control beta-one-power, --control infant-discontinuous and --control newborn-uncovered must FAIL (each isolates one check)
python -I specs/stream/selftest_channel_a.py      # World Stream v0 channel-A segment DDL (SPR-011)
python -I specs/stream/selftest_channel_c.py      # World Stream v0 channel-C segment DDL: sift matches, with a life_ends twin (SPR-013)
python -I specs/stream/canonical_v0.py --check    # the canonical text form v0 and segment.canonical_digest: round trip and golden lines (SPR-013)
cargo nextest run -p rh-stream --locked --test canonical --test writer --test parquet_props   # rh-stream: the canonical form = the reference's golden lines; byte-identical sealing; the pinned Parquet properties and golden segment bytes (SPR-013 T-09); the parser fuzz is nightly (--run-ignored)
CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true cargo nextest run -p rh-cli --locked --release --test kill   # part stream, release with overflow checks (BCK-002 A-63): kill -9 during sealing and mid-year, then resume from the 10-year keyframe: the same bytes, no seq re-emitted below the first unsealed year (SPR-013 T-09)
target/release/rh-cli run solo --years 200 --run-dir target/golden/solo_run   # a run directory: run.toml, journal.sqlite, run.ladder, keyframes/ (each with its ladder sidecar), stream/ (SPR-013 T-09); --out, --journal and --inject are refused with it
python -I specs/stream/check_segments.py target/golden/solo_run   # V-DATA-10 subset: DuckDB reads every sealed table with the DDLs' types; the reference recomputes every canonical_digest; --plant float-column and --plant digest must FAIL
cargo nextest run -p rh-golden --release --test stream   # V-STR-01 on solo and the 60-person hamlet: byte-identical segments over repeats and threads; resume verification (SPR-013 T-09)
target/release/rh-cli tail target/golden/solo_run [--from-seq N] [--to-seq M] [--follow PERSON] [--kind K] [--canonical] [--wait]   # the sealed stream as display or canonical lines (SPR-013 T-10)
cargo nextest run -p rh-golden --release --test tail   # tail: every record once from any cursor, canonical digests, read-only, display snapshots (RH_UPDATE_SNAPSHOTS=1 rewrites them; a stale copy must FAIL)
CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true cargo nextest run -p rh-cli --locked --release --test tail_live   # part stream, release with overflow checks: a tail started mid-run prints the after-run tail's lines (SPR-013 T-10)
target/release/rh-cli sift target/golden/solo_run   # channel C (life_ends matches) from sealed channel A, downstream of the engine (SPR-013 T-11)
python -I specs/sifting/twin_check.py target/golden/solo_run   # V-SIFT-01: the SQL twin (specs/sifting/twins/) = rh-sifter's channel C; --control drop-last must FAIL
cargo nextest run -p rh-sifter --locked --test sift   # rh-sifter v0: the golden matches and MatchIds; refusals (SPR-013 T-11)
cargo nextest run -p rh-golden --release --test sift   # V-PLN-04 subset: sifting twice is byte-identical; channel A and the ladder unchanged (SPR-013 T-11)
python -I specs/sifting/lint_patterns.py tests/lint_fixtures/sifting/life_ends_unanchored.toml specs/manifest/registry_v0.toml   # V-SIFT-05: must FAIL (an unanchored read)
python -I specs/stream/selftest_wire_v1.py --check   # World Stream wire format v1 (IF-0 live edge): framing, control messages, refusals, the live-edge rule simulated with slow, stalled and reconnecting viewers, golden frames (SPR-026); --control window-gap must FAIL
python -I specs/stream/selftest_control_v1.py --check   # World Stream control channel v1 (IF-0 control.sock, the pace command; SPR-027 T-01; specs/stream/control_v1.md): the method table, every request and response, refusals, the governor's rule, pace invariance on a simulated engine with a fake monotonic clock (commands applied at the engine's decision points, landing mid-instant, floods of no-change commands), the run log, golden frames; --control pause-writes-record, pace-in-journal, band-in-hash, input-accepted, step-overshoot, pace-on-stream, paused-time-owed and extra-method must FAIL
python -I specs/economy/selftest_economy.py
python -I specs/stream/selftest_virtual_telemetry.py
python -I specs/systems/lint_odd.py   # system specs; [[emits]] roles, motives and snapshots against the Registry and the spec's columns, cause_column on a state column (SPR-020; planted in tests/lint_fixtures/odd_envelope/); a foreign [[reads]] entry says whether its owner is `required`, with a typed `neutral` on rate and mode reads of absent owners; [[applies]] payloads are typed and name their drain binding (EVO-001); planted both ways in tests/lint_fixtures/odd_neutral/ and odd_applies/
python -I bin/gen_skeletons.py --check     # committed rh-core skeletons = lint_odd.py --rust (SPR-005)
python -I specs/systems/lint_odd.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml   # the smoke world's dummy specs, outside the real Registry (SPR-008)
python -I bin/gen_skeletons.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml --out crates/rh-sys-smoke/src/generated --check
cargo nextest run -p rh-sys-smoke --locked --test smoke   # smoke world: registration against its specs, coverage, small-scale V-DET-01/02 (SPR-008 T-01)
cargo build --release -p rh-cli && target/release/rh-cli run smoke --years 1000 --ladder yearly --require-parallel 5 --out target/golden/smoke.ladder   # the golden run: 1,000 sim-years, plane-A hash ladder, at least 5% parallel levels (SPR-008 T-02, DEF-2)
python -I bin/check_baseline.py --baseline worlds/smoke/baselines/smoke_yearly_1000y.ladder --recomputed target/golden/smoke.ladder   # baseline = recomputation; changed only in a [C3] PR (T-04)
target/release/rh-cli run solo --years 200 --ladder yearly --out target/golden/solo.ladder   # solo, rung 0: compile, genesis, M2; the header carries the history tuple and the built and not-built systems (SPR-012 T-08); --strict refuses it
python -I bin/check_baseline.py --baseline worlds/solo/baselines/solo_yearly_200y.ladder --recomputed target/golden/solo.ladder   # solo's baseline = recomputation; changed only in a [C3] PR
cargo nextest run -p rh-golden --release --test solo   # solo: resume through 10-year keyframes = continuous, thread invariance, the manifest pin, Ada, the input re-sampling her pending death (SPR-012 T-08)
cargo nextest run -p rh-golden --release --test golden   # resume = continuous, thread invariance, V-DET-11 bisect (SPR-008 T-02, T-03)
python -I bin/check_all_selftest.py verdict --or-true   # must FAIL: check_all's own verdict fails a gate hidden by || true (VER-SPR-008-TEST GAP-3)
python -I bin/check_all_selftest.py inventory   # the gate inventory both ways: every chained gate listed, every listed gate chained, every chain call a listed loop, no run line outside them; --plant-unlisted and --plant-dropped must FAIL (BCK-002 A-65)
python -I bin/check_all_selftest.py baseline-controls   # every Golden baseline control keeps its own reason on a stale baseline; --plant-committed must FAIL (BCK-002 A-66)
python -I bin/ci_scope.py trace population-synthesis -- specs/worldgen/selftest_population_synthesis.py   # a path-scoped gate under the audit hook: an undeclared read or listing, or a started process, is SCOPE INCOMPLETE (BCK-002 A-64; check_all runs both scoped gates this way, `--gate KEY` decides whether they run in CI)
bin/golden_nightly.sh            # nightly: monthly ladders at 1/2/4/N threads x 3 (V-DET-01/02) for smoke, solo, household and hamlet, the yearly cut = the baseline (GAP-6), V-SCALE-01/04 at 1,000 years, the V-INV-02 debug soak, V-DET-11 x 64
cargo nextest run -p rh-core -p rh-registry --test determinism --test registration --test real_specs --test phase_table --test time_properties --test system_id_golden --test engine_contract   # V-DET-01/02 stub world, AccessSpec conflicts, skeletons, RNG purposes, phase table, calendar, system_id golden, engine contract (SPR-005)
cargo nextest run -p rh-core -p rh-events --test agenda --test agenda_events --test invariants --lib   # agenda, timers, lifecycle queues, event bus, V-DET-12, invariants, crash replay (SPR-006)
cargo nextest run -p rh-snapshot --test hash --test golden --test journal --test keyframe --test guard --test unsynced_guard --test stub_world   # plane hashes, input journal, V-DET-05 keyframes, V-DET-10 unsynced guard, stub-world hash golden (SPR-007)
cargo nextest run -p rh-snapshot --release --test guard   # the unsynced guard is compiled out of release builds (SPR-007)
cargo nextest run -p rh-snapshot --run-ignored only --test keyframe   # DEF-3 full sweep: every bit of a keyframe's Arrow body flipped, nightly tier (SPR-007)
cargo build -p rh-cli && python -I bin/manifest_parity.py specs/manifest/examples/*.yaml tests/manifest_parity/*.yaml  # V-WC-04: Rust loader = manifest_v0.py
target/debug/rh-cli ownership    # single-writer column ownership from specs/systems (SPR-004)
target/debug/rh-cli manifest schema --diff specs/manifest/manifest_v0.schema.json  # Rust types = reference schema (SPR-004)
python -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml
# Performance (SPR-009): instruction counts under valgrind (needs valgrind and iai-callgrind-runner 0.16.1)
python -I bin/perf_gate.py gate --out artifacts/perf/instructions.txt   # V-PERF-04: benches 7 and 8, and the draw-cost bench (SPR-011 A-37), within 10% of the baseline
python -I bin/perf_gate.py integrity --measured artifacts/perf/instructions.txt   # ruling 3: a changed baseline = the tool's output, [perf-baseline] PR
python -I bin/perf_gate.py record   # regenerate the baseline (never by hand); only in a [perf-baseline] PR with a perf-baseline-reason: line
cargo nextest run -p rh-snapshot --locked --test observer   # observer invariance: a host observer changes no hash (SPR-009 ruling 4)
RH_WALL_ROWS=100000,1000000 cargo bench -p rh-bench --bench wall && python -I bin/wall_trend.py summarize --machine <adjutant|laptop> --out artifacts/perf/wall.txt   # T-02b wall-clock trends (gate only on adjutant)
python -I bin/perf_gate.py alloc-gate --out artifacts/perf/allocations.txt   # V-PERF-11: smoke-world allocations per steady-state tick within the recorded budget (SPR-009 T-05), and each rung's (solo, household, hamlet; SPR-013 T-13)
python -I bin/perf_gate.py alloc-record   # regenerate the allocation baseline (exact counts; never by hand)
target/release/rh-cli run smoke --years 1000 --threads 4 --vitals --out target/golden/smoke_yearly_tN.ladder   # T-03 vitals line, on the N-thread golden run; its ladder equals the golden baseline
python -I specs/sifting/lint_patterns.py specs/sifting/patterns.solitary_life.toml specs/manifest/registry_v0.toml   # life_ends (SPR-011)
target/release/rh-cli replay <run-dir> --into <dir> [--from-keyframe <tick>]   # V-DET-04/05: the run rebuilt from run.toml and its journal alone; REPLAY EQUAL, or a missing, extra or edited journal row hard-fails (SPR-013 T-12); `--into` must be absent, empty or a previous replay's output (it holds `.rh-replay`), never the run, inside it or containing it (DEF-2)
cargo nextest run -p rh-golden --release --test replay   # V-DET-04/18 (run.toml and the journal alone), V-DET-05 from keyframes, V-DET-14, the tampered-journal controls (SPR-013 T-12)
target/release/rh-cli run household --years 200 --threads 1 --ladder yearly --expect-serial --run-dir target/golden/household_run   # rung 1's golden run (rung 2: hamlet); --expect-serial passes only while no level holds two jobs, and the multi-system smoke world must FAIL it (SPR-013 T-13)
python -I bin/check_baseline.py --baseline worlds/household/baselines/household_yearly_200y.ladder --recomputed target/golden/household_run/run.ladder   # household's and hamlet's baselines = recomputation; changed only in a [C3] PR
cargo nextest run -p rh-golden --release --test rungs   # V-SCALE-04 PR subset: household and hamlet past their extinction, one EXTINCTION, tail, sift and replay consistent; one build runs rungs 0-2 (SPR-013 T-13)
```

Not a check: `bin/regen_baseline.sh [smoke|solo|household|hamlet]` regenerates the golden baselines (all four without an argument), only in a C3 PR's final commit, whose subject ends with `[C3]` (never type a baseline; SPR-008 T-04, `worlds/smoke/README.md`). Nor is `lint_odd.py --next … --write-amendments-lock`, which rewrites `specs/systems/next/amendments.lock.toml` (every amendment of a next-version spec, by path and text digest) after the Architect has read the new `AMEND` lines; the lock is never typed (SPR-015, REV-SPR-015 round 3 MIN-1).

## Conventions
- Docs are Markdown, and diagrams are Mermaid.
- Requirements use IDs from `docs/03`. ADRs are numbered `ADR-NNNN`.
- Cite research as `R0x §n`.
- Record director rulings and gate outcomes in `docs/decisions-log.md`.
- Every crate declares its plane and kind in `[package.metadata.rehoboam]` (`bin/lint_arch.py` checks it).
- A forbidden API in plane A needs `// LINT-ALLOW(forbidden-api): <reason>` on its line; the gate counts and prints every escape.
