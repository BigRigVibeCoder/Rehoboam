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
All of them, fail-fast, with a report: `bin/check_all.sh` (CI runs its two parts, `--part core` and `--part golden`, as parallel jobs; no flag runs both). It uses `python/.venv` (create it with `uv sync --frozen --project python`; `PYTHON` overrides) and needs uv, cargo-nextest, cargo-deny, valgrind and iai-callgrind-runner (RUN-001). Each gate also runs on a planted violation that must FAIL (`tests/lint_fixtures/`). Individually:
```bash
# Rust: workspace in crates/, toolchain pinned in rust-toolchain.toml
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny --locked check bans sources            # crates.io only, no wildcards (licences: ledger only, ADR-0021 amended)
cargo nextest run --workspace --locked
cargo test --doc --workspace --locked             # doctests: rh-ids compile-fail cases, counted per file by check_all (SPR-002)
cargo nextest run -p rh-ids --run-ignored only --test stream_independence   # V-DET-20 full sweep, 10^6 draws (nightly)
# Architecture lints: stdlib Python
python -I bin/lint_arch.py        # crate boundaries (V-INSP-03) and plane isolation (V-PLN-01)
python -I bin/lint_forbidden.py   # forbidden APIs in plane-A crates (V-DET-19), floats outside declared f32_field columns included
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
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml  # must FAIL
python -I bin/gen_rng_golden.py --check  # rh-ids golden vectors match the independent reference (SPR-002)
python -I bin/gen_hash_golden.py --check  # rh-snapshot plane-hash and XXH3 golden vectors match the independent stdlib reference (SPR-007)
python -I specs/chronicle/selftest.py
python -I bin/gen_fixed_vectors.py --check   # rh-core::fixed golden vectors match the exact rational reference (SPR-010)
python -I bin/gen_sampler_reference.py --check   # rh-sample's exact reference distributions are current (SPR-003)
cargo nextest run -p rh-sample --locked --release --test golden   # V-DET-09 sampler golden vectors at opt-level 3 (SPR-003 T-05)
cargo nextest run -p rh-sample --run-ignored only --test stats   # sampler statistics, nightly tier (64x the PR samples)
python -I specs/demography/selftest_brass.py
python -I specs/demography/selftest_m2_rung0.py   # M2 rung-0 mortality: the exact reference of [algorithm.mortality_rung0] (SPR-011)
python -I specs/stream/selftest_channel_a.py      # World Stream v0 channel-A segment DDL (SPR-011)
python -I specs/stream/selftest_channel_c.py      # World Stream v0 channel-C segment DDL: sift matches, with a life_ends twin (SPR-013)
python -I specs/stream/canonical_v0.py --check    # the canonical text form v0 and segment.canonical_digest: round trip and golden lines (SPR-013)
python -I specs/economy/selftest_economy.py
python -I specs/stream/selftest_virtual_telemetry.py
python -I specs/systems/lint_odd.py
python -I bin/gen_skeletons.py --check     # committed rh-core skeletons = lint_odd.py --rust (SPR-005)
python -I specs/systems/lint_odd.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml   # the smoke world's dummy specs, outside the real Registry (SPR-008)
python -I bin/gen_skeletons.py --specs worlds/smoke/systems --overlay worlds/smoke/registry_overlay.toml --out crates/rh-sys-smoke/src/generated --check
cargo nextest run -p rh-sys-smoke --locked --test smoke   # smoke world: registration against its specs, coverage, small-scale V-DET-01/02 (SPR-008 T-01)
cargo build --release -p rh-cli && target/release/rh-cli run smoke --years 1000 --ladder yearly --require-parallel 5 --out target/golden/smoke.ladder   # the golden run: 1,000 sim-years, plane-A hash ladder, at least 5% parallel levels (SPR-008 T-02, DEF-2)
python -I bin/check_baseline.py --baseline worlds/smoke/baselines/smoke_yearly_1000y.ladder --recomputed target/golden/smoke.ladder   # baseline = recomputation; changed only in a [C3] PR (T-04)
cargo nextest run -p rh-golden --release --test golden   # resume = continuous, thread invariance, V-DET-11 bisect (SPR-008 T-02, T-03)
python -I bin/check_all_selftest.py verdict --or-true   # must FAIL: check_all's own verdict fails a gate hidden by || true (VER-SPR-008-TEST GAP-3)
bin/golden_nightly.sh            # nightly: monthly ladders at 1/2/4/N threads x 3 (V-DET-01/02), the yearly cut = the baseline (GAP-6), V-DET-11 x 64
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
python -I bin/perf_gate.py gate --out artifacts/perf/instructions.txt   # V-PERF-04: benches 7 and 8 within 10% of the baseline
python -I bin/perf_gate.py integrity --measured artifacts/perf/instructions.txt   # ruling 3: a changed baseline = the tool's output, [perf-baseline] PR
python -I bin/perf_gate.py record   # regenerate the baseline (never by hand); only in a [perf-baseline] PR with a perf-baseline-reason: line
cargo nextest run -p rh-snapshot --locked --test observer   # observer invariance: a host observer changes no hash (SPR-009 ruling 4)
RH_WALL_ROWS=100000,1000000 cargo bench -p rh-bench --bench wall && python -I bin/wall_trend.py summarize --machine <adjutant|laptop> --out artifacts/perf/wall.txt   # T-02b wall-clock trends (gate only on adjutant)
python -I bin/perf_gate.py alloc-gate --out artifacts/perf/allocations.txt   # V-PERF-11: smoke-world allocations per steady-state tick within the recorded budget (SPR-009 T-05)
python -I bin/perf_gate.py alloc-record   # regenerate the allocation baseline (exact counts; never by hand)
target/release/rh-cli run smoke --years 1000 --threads 4 --vitals --out target/golden/smoke_yearly_tN.ladder   # T-03 vitals line, on the N-thread golden run; its ladder equals the golden baseline
python -I specs/sifting/lint_patterns.py specs/sifting/patterns.solitary_life.toml specs/manifest/registry_v0.toml   # life_ends (SPR-011)
```

Not a check: `bin/regen_baseline.sh` regenerates the smoke golden baseline, only in a C3 PR's final commit, whose subject ends with `[C3]` (never type a baseline; SPR-008 T-04, `worlds/smoke/README.md`).

## Conventions
- Docs are Markdown, and diagrams are Mermaid.
- Requirements use IDs from `docs/03`. ADRs are numbered `ADR-NNNN`.
- Cite research as `R0x §n`.
- Record director rulings and gate outcomes in `docs/decisions-log.md`.
- Every crate declares its plane and kind in `[package.metadata.rehoboam]` (`bin/lint_arch.py` checks it).
- A forbidden API in plane A needs `// LINT-ALLOW(forbidden-api): <reason>` on its line; the gate counts and prints every escape.
