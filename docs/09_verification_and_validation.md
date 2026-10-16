# 09 — Verification & Validation Plan

*Status: Draft v1.1 (2026-10-15). **Verification** asks "did we build it right?" against the requirements in `03`. **Validation** asks "did we build the right thing?" against the success criteria and MOEs in `01`. Every gate below is automated unless marked **[H]**, which means a human judgement by you.*
*v1.6 (2026-10-16): V-SCALE-04's end event is in channel A, not the journal, which holds inputs only.*
*v1.5 (2026-10-16): V-INSP-03 lists `rh-sample`, as `bin/lint_arch.py` allows; V-INSP-05 uses the one engine-crate list of `05.0` §6.*
*v1.4 (2026-10-16): ADR-0035, the saga engine. New V-SAGA suite (§2.20, V-SAGA-01 to V-SAGA-21), V-INSP-05 (engine purity) and V-HUM-13 (the G3 season); V-R3D-02, 03, 05 and 09 withdrawn; band gate status under ADR-0035 (§3.1): economic-history bands become report-only; gates G1a–G5 re-aimed (§5); the matrix follows the new phases (§6).*
*v1.3 (2026-10-16): D27 (ADR-0034): V-DET-15, V-COG-09 and V-COG-11 withdrawn; V-DET-04 records push inputs only; new V-LLM-11 for LLM-9.*
*v1.2 changes (R12 batches 1–3): era-conditioned and literature-sourced bands; band tags for kind, role (calibration / held-out), level and discrimination; CI-based gate with no held-out failures; non-ergodicity and typicality reporting; submodel tournaments (§3.8); V-ERA (§3.6) and V-SCALE (§3.7) suites.*
*v1.1 changes: test IDs for every suite and a full requirement → test matrix (§6). New suites V-PLN, V-DATA, V-COG, V-ORA, V-TM, V-WC, V-R3D and V-AUD. DET-6 console fuzz from P2. TOST equivalence replaces KS tests (§1.2). Certification statistics with a red-noise null, Clauset tail fits and censoring (§3.1). Foresight validity tests replace per-decile calibration (§3.3). A pre-registered G2 protocol (§3.4.1).*

---

## 1. Test architecture

```
                         ┌───────────────────────────────────────────────┐
     Validation [H]      │  G2 pre-registered protocol · rating sessions │  phase gates
                         │  blind A/B · MOE-1..6                         │
                         ├───────────────────────────────────────────────┤
     System V&V          │  Stylized-fact certification · couplings ·    │  nightly / labelled PRs
                         │  Foresight validity · persuasion bands ·      │
                         │  adversarial player bot · liveness gate       │
                         ├───────────────────────────────────────────────┤
     Integration         │  Golden runs · plane isolation · console      │  every PR
                         │  fuzz · sifter twins · Playwright · protocol  │
                         │  conformance · hydration tests                │
                         ├───────────────────────────────────────────────┤
     Component           │  Property tests (invariants) · sampler golden │  every PR
                         │  vectors · unit tests · benches · lints       │
                         └───────────────────────────────────────────────┘
```

### 1.1 Suite index

Test IDs have the form `V-<suite>-<nn>`. Work packages cite them in their acceptance gates (`13` §2).

| Suite | Section | Covers | Cadence |
|---|---|---|---|
| **V-DET** Determinism | §2.1 | DET-* | Every PR; cross-platform nightly |
| **V-STR** World Stream | §2.19 | STR-*, PERF-5 | Every PR from P1.0 (01); P1b (02, 03, 05); P2 (04) |
| **V-INV** Invariants and properties | §2.2 | SIM-*, EVO-1, EVO-2 | Every PR; soak nightly |
| **V-GOLD** Golden worlds and save compatibility | §2.3 | DET-5, DET-10, EVO-1 | Every PR; full length nightly; every release |
| **V-SIFT** Sifter | §2.4 | SIFT-*, TM-2 | Every PR; budgets nightly |
| **V-HYD** Hydration and multi-resolution consistency | §2.5 | R3D-4, EVO-3, DET-6 | PRs touching the bubble or fidelity levels; nightly |
| **V-IF** Protocol and interface conformance | §2.6 | SIM-11, R3D-1, IF-1–IF-9 | Every PR |
| **V-PERF** Performance | §2.7 | PERF-* | Every PR; reference desktop nightly |
| **V-LLM** LLM-facing verification | §2.8 | LLM-*, SEC-1 | PRs touching prompts or models; quarterly |
| **V-NAR** Narrative QA | §2.9 | NAR-* | Per artifact; NOVEL-20 on model or prompt change |
| **V-PLN** Plane isolation | §2.10 | DET-6, DET-12 | Every PR |
| **V-DATA** Chronicle and data | §2.11 | DATA-* | Every PR; volume runs nightly |
| **V-COG** Cognition and epistemics | §2.12 | COG-*, SIM-7 | Every PR |
| **V-ORA** Oracle and persuasion | §2.13 | ORA-*, SEC-1 | Every PR; sweeps and player bot nightly |
| **V-TM** Time-Machine console (Playwright) | §2.14 | TM-*, DET-6, DET-14 | PRs touching the console or server |
| **V-WC** World compiler | §2.15 | WC-*, SIM-10 | PRs touching the compiler or Registry; certification per compiled world |
| **V-R3D** 2D client (3D tests withdrawn, ADR-0035) | §2.16 | R3D-* | P5 PRs |
| **V-AUD** Audio | §2.17 | AUD-* | Per export; latency tests only if street-level is taken up |
| **V-INSP** Inspection and licences | §2.18 | LIC-1, OBS-1, EVO-1 | Every PR (licences); review |
| **V-SF** Stylized-fact harness and certification | §3.1 | SIM-3, SIM-12, EVO-3, MOP-5 | Nightly; labelled PRs; per compiled world |
| **V-COUP** Drama-coupling scenarios | §3.2 | SIM-3, SIM-12 | Nightly; phase gates |
| **V-FOR** Foresight validity | §3.3 | ORA-5, ORA-9 | Nightly on the cluster |
| **V-HUM** Human validation [H] | §3.4, §3.5 | MOE-1–MOE-6, TM-6 | Phase gates |
| **V-ERA** Era transitions | §3.6 | SIM-13, SIM-14, SIM-10 | Nightly from P1b (V-ERA-01, 02); P4 (03–05); each era pack (06, 07) |
| **V-SCALE** Scale range | §3.7 | SIM-1, SIM-15, SIM-16 | Nightly from P1.0 (01–04); P1b (05, 06; mythic catalysts, ADR-0035) |
| **V-SAGA** Sagas and the chronicle pipeline | §2.20 | SAGA-*, MOE-2 | Every PR (01–04, 11, 14, 15, 17, 19, 21); nightly ensembles (05, 09, 10, 13, 16, 18, 20); per export (07, 08, 12); G2 (06) |

### 1.2 Statistical conventions

1. **Equivalence is shown, not assumed.** When a test must show that two things agree, it uses **TOST** (two one-sided tests). For a statistic θ and a margin δ fixed in advance, the two null hypotheses θ_A − θ_B ≤ −δ and θ_A − θ_B ≥ +δ are each rejected at α = 0.05. Equivalently, the 90% confidence interval of the difference lies inside (−δ, +δ). A "fail to reject" test (KS or χ² with p > 0.05) never counts as evidence of agreement: with few samples it passes broken models, and with many it fails good ones.
2. **Margins are stated before the run**, in the system's `AggregateSpec`, the pattern spec or the table below. They are never changed after seeing results; a change is logged in `docs/decisions-log.md`.
3. **Several statistics.** Equivalence across several statistics is claimed only if every TOST passes (intersection–union), so no multiplicity correction is needed.
4. **Pairing.** Two configurations are compared on the same seeds and salts (common random numbers, CRN) wherever possible, and the paired differences are tested.
5. **Units.** For ensembles, the unit of analysis is the seed. Intervals come from t-intervals over seed-level statistics or from a seed-level bootstrap (10,000 resamples).
6. **Proportions.** Foresight probabilities use Wilson intervals. Small-count rates (bad-run rate, coupling completion) use Clopper–Pearson intervals.
7. **Model-fit tests are different.** A goodness-of-fit test against a fitted model, such as the Clauset–Shalizi–Newman plausibility bootstrap (§3.1), asks whether a model is tenable. It does not claim that two systems agree, and it is used only as defined there.

**Default equivalence margins**

| Comparison | Statistic | Margin δ | Test |
|---|---|---|---|
| Presented crowds vs plane-A schedules | Share of persons per (cell, hour, activity) bucket and per destination type | ±0.02 absolute | V-HYD-05 |
| Python prototype vs Rust port | Each declared aggregate, seed means over 32 seeds | Declared tolerance; default ±2% relative | V-INV-07 |
| F(n) vs F(n+1) | Declared targets (conserved quantities are compared exactly) | Declared tolerance; default ±5% relative | V-HYD-10 |
| Model change (change class) | Each stylized-fact statistic, CRN-paired over 32 seeds | 10% of the band width | V-GOLD-03 |
| Should-system dynamics vs analytic solution | Shares, adoption curves, flows on fixture worlds | Declared tolerance; default ±5% relative | V-INV-14 |
| Reduced-cost vs full-cost Foresight | P(predicate), CRN-paired | ±0.03 absolute | V-FOR-02 |
| Oracle-mode Foresight leak | P(predicate) in two worlds that differ only in a secret the Oracle lacks | ±0.03 absolute | V-ORA-10 |
| Agenda fairness | Spearman ρ between entity-ID rank and first-applied rate | ±0.05 | V-COG-04 |
| Malthusian income | Long-run real income change after a technology shock | ±5% relative | V-SF-10 |

## 2. Verification suites

### 2.1 Determinism (V-DET)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-DET-01 | Thread-count invariance | Run reference worlds at 1, 2, 4 and N threads for 50 sim-years; compare monthly hashes | All equal |
| V-DET-02 | Repeat invariance | Same binary, 3 runs | All equal |
| V-DET-03 | Cross-platform (level 2 goal) | Nightly runs on Linux x86-64, Windows x86-64, macOS ARM; compare yearly hashes | Report. Becomes a gate once achieved |
| V-DET-04 | Replay | Record a session with inputs (Oracle actions, forks, speech acts), then replay from the journal alone | Final and monthly hashes equal; no gateway call during replay |
| V-DET-05 | Keyframe round-trip | Write a keyframe, load it, advance 10 years; compare against a continuous run. Check the permanent 10-year cadence and the yearly LRU cache. P0 (SPR-007 T-02): `crates/rh-snapshot/tests/keyframe.rs` on the SPR-006 life world, cut at day 50 (timers pending, events held, inputs queued) and at year 1, 120 monthly `state_hash` values compared; the cadence and LRU through `rh_snapshot::store::KeyframeStore`, with a seek from the nearest held year; every wrong keyframe (lost timer, altered row, wrong count, corrupt or truncated bytes, other history) refused; held (undrained) events kept by a load, outside `state_hash`, whose monthly ladder is identical whether the host drains never, every instant, monthly or at random instants | Equal |
| V-DET-06 | **Console fuzz (DET-6, from P2)** | Run A headless. Run B on the same seed while a fuzzer drives random sequences of non-material console and JSON-RPC calls throughout: `entity.*` inspections (which write facts and pins), `chronicle.*` and `sift.*` queries, ratings, scrub, `timeline.seek` (drop-in, then return to the head), pause and resume at random ticks, speed-band changes, Foresight jobs. Journaled inputs are excluded | `state_hash` equal every month; plane-A Parquet segments byte-identical; EventIds identical. `observation_hash` must change (positive control) |
| V-DET-07 | Observer invariance with passive crowds (P5) | Run B with the Diorama attached: camera director, T2c crowds, Spectacle | As V-DET-06 |
| V-DET-08 | Observer invariance with live hydration (Later, if street-level is taken up) | Run B with a scripted avatar that hydrates, inspects and opens conversations, with no material inputs | As V-DET-06 |
| V-DET-09 | Sampler golden vectors | Each vendored sampler (`rh-sample`: binomial, Poisson, normal, gamma, beta) × its methods and edge parameters × fixed keys: the first samples and the draws consumed (`crates/rh-sample/tests/golden/samplers_v1.txt`), checked at opt-level 0 and 3 in the PR tier and on GitHub's runner (SPR-003 T-05). Golden vectors only freeze outputs; correctness is a separate test against the exact distributions from `bin/gen_sampler_reference.py` (Python `fractions`/`decimal`): χ² for binomial and Poisson, mean and variance z-tests for Poisson, gamma, beta and normal, and normal tail counts at 1–4 σ; the PR tier draws 2¹⁶ per case (2²⁰ for tails), the nightly run 64 times more. A C3 change to a sampler or its reference generator merges only with the nightly tier green on its head (GOV-005 §6) | Bit-equal across releases, machines and opt-levels; every statistic inside its bound (χ² below the 99.9% quantile, \|z\| < 4.5) |
| V-DET-10 | Unsynced guard (DET-11) | Fuzz the read-only APIs (frame builders, Python `frame()`, LLM context builders) with the debug hash check on. P0 (SPR-007): `rh_snapshot::guard::read_only`; the engine's read-only APIs fuzzed on the stub world (`crates/rh-snapshot/tests/unsynced_guard.rs`), the injected-write control in `crates/rh-snapshot/tests/guard.rs` | No hash change; the guard trips on an injected write (positive control) |
| V-DET-11 | Divergence bisect (DET-4) | Inject a one-row perturbation at a random month and table | The tool reports that month, table and row |
| V-DET-12 | No back-dating (DET-13) | Property test over random small worlds | Every emitted event's tick ≥ the current instant; EventIds strictly increase in emission order `(tick, phase, seq)`; sampled outcomes sit in the agenda as timers until they fire |
| V-DET-13 | Acting in the past (DET-14) | Seek to t < head and apply a material input | `apply_input` returns `PastInput`; the Timeline Manager forks at t; the canonical timeline's journal and hashes are unchanged; the fork replays to the same hashes |
| V-DET-14 | Material-input scope (DET-14) | Apply a scoped input; compare pending timers with a run without it | Only timers inside the scope are re-sampled, keyed by `epoch`; timers outside keep their ticks; at equal ticks the input applies first |
| V-DET-15 | **Withdrawn (ADR-0034).** Was: pull inputs (DET-15) | — | — |
| V-DET-16 | Branch salts (DET-2) | Property test over random fork trees | No two rollouts share a salt; `resample_on_fork` purposes re-draw after a fork and others are shared; a what-if fork without overrides matches its parent's hashes |
| V-DET-17 | CRN survival (ADR-0026) | Paired intervention rollouts over 10 years | Correlation of per-person death times across the pair ≥ 0.9 for unaffected lineages |
| V-DET-18 | Journal completeness (DET-1, DET-7) | Rebuild a session from `(engine_build, registry_version, manifest_hash, seed, input_journal)` only, on a clean machine; audit every journal record | Hashes equal; every record has tick, seq, class, scope and producer |
| V-DET-19 | Forbidden APIs (DET-2, DET-3) | Clippy lints and a grep gate on authoritative crates: `HashMap` iteration, `thread_rng`, `StdRng`, `mul_add`, float types in threshold positions. Floats: `bin/lint_forbidden.py` reports every `f32`/`f64` type, float conversion and float literal outside a crate's `tests/`, `benches/` and `examples/`; only a declared float field (an escape naming a column a system spec types `f32_field`) is excused (SPR-010). The float rule exempts one crate by its pinned name, `rh-sample`, the samplers' float zone (SPR-003 Architect ruling); every other rule applies to it, positive controls show the exemption reaches no other crate (`rh-sampler` is still reported) and that `rh-sample` cannot be relabelled out of plane A; the rule `float-api` keeps `f32`, `f64` and `libm` out of `rh-sample`'s public surface (`pub` signatures, fields, consts, statics, aliases, re-exports, item headers and `impl` headers; no escape; a planted control per form); and `clippy.toml` disallows the `std` float transcendentals (`libm` only), with a positive control; a test pins all 44 entries; `bin/lint_arch.py` (rule `lints`) requires every plane-A crate to take exactly `[lints] workspace = true`, so no crate can switch the clippy half off in its manifest, and (rule `clippy-config`) the root `clippy.toml` is the only clippy configuration in the repository, with `CLIPPY_CONF_DIR` unset; and an `allow`/`expect` of `clippy::disallowed_methods` or `disallowed_types` (or their groups) in plane A is itself a forbidden token (rule `lint-allow`), so silencing clippy's half needs a counted escape | Zero hits |
| V-DET-20 | RNG stream independence (statistical; RT-2026-10-16) | Streams for distinct (system, purpose, entity, t, k) keys and for sibling branch salts: pairwise correlation, chi-square on bytes, and a runs test, over at least 10³ key pairs including adjacent entities and adjacent t. The PR tier draws 2¹⁶ per stream; the nightly run (`.github/workflows/nightly.yml`) does the full sweep of 10⁶ draws per stream | Every statistic inside its 99.9% band, with the expected number of exceedances across all pairs (no more than a binomial 99.9% allows) |

### 2.2 Invariants and properties (V-INV)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-INV-01 | Pulse invariants | **proptest** strategies generate small random worlds and manifests within bounds. After every pulse, assert the `05.1` §9 invariants: population conservation reconciled with the life-course log, conserved compartments, non-negativity, genealogy acyclicity and minimum parental age, life-course consistency, exclusive relations, valid IDs, plane isolation | Zero violations |
| V-INV-02 | Long-run soak | 1,000 years with invariants enabled in debug builds, nightly | Zero violations |
| V-INV-03 | Single death path (EVO-1) | Kill persons through each cause: `M2.demography` hazard, `M5.contagion` infection, `M8.war` battle, agenda action | Each produces the same consequence set through the lifecycle queue: ledger removal, life-course entry, succession trigger, inheritance, death event |
| V-INV-04 | Ownership and scheduler (EVO-1, SIM-4) | Start-up checks on mutated `AccessSpec`s: two writers for one column, an undeclared read, a write outside owned columns, a cycle without a declared lag | Each refused at start-up with a named error; the derived order is stable (topological, ties by system ID) |
| V-INV-05 | Boundary adapters (SIM-8) | Off-map F0 compartments exchange people with the ledger through the materialisation adapter | People conserved across the boundary; immigrant IDs follow `H64(seed, "immigrant", origin, t, k)` |
| V-INV-06 | Crate checklist (EVO-2) | A script audits each system crate | Property tests, a calibration notebook and golden fixtures present |
| V-INV-07 | Prototype port (`05.1` §10) | Rust port against the Python prototype on the fixture world, 32 seeds | TOST equivalence of every declared aggregate (§1.2) |
| V-INV-08 | Vital-event draws (SIM-5) | Unit and property tests; grep gate | Binomial draws only; hazards respond to age, density and entitlement as declared; no Poisson death draws |
| V-INV-09 | Capacity (SIM-1) | A synthetic 2M-person world runs 10 years with invariants on | No ID or column overflow; zero violations |
| V-INV-10 | World generation (SIM-2) | Same seed and generator version on two machines | Identical raster, region graph and routes; resolutions follow the manifest |
| V-INV-11 | Clocks (SIM-6) | Pulse-trace audit; founders born before the epoch | `Day` pulses only for regions, armies and sieges marked active at the preceding coarser pulse; negative `SimTime` round-trips through keyframes |
| V-INV-12 | Great houses (SIM-9) | Run 300 years on `val_khor` | Ruling houses and active great-house members are T1 from world start; dormancy and re-promotion follow simulation criteria only; any person's genealogy reconstructs from the life-course log |
| V-INV-13 | Ecology bound (SIM-12) | Long runs with `S4.ecology` and dragons | Predator standing biomass never exceeds the energetics bound; dragon counts change only through individual life-history events |
| V-INV-14 | Should-system dynamics (SIM-12) | `S1.culture` shares, `S2.technology` adoption and `S3.migration` flows on fixture worlds with known analytic solutions | TOST equivalence within declared margins (§1.2) |

### 2.3 Golden worlds and save compatibility (V-GOLD)

- `worlds/` holds reference manifests: `val_khor` (feudal plus dragons), `aethelgard` (modern hive, 10 years), `delta_republic` (merchant republic), and a minimal `smoke` world. The G2 held-out manifest (§3.4.1) is stored sealed and joins the golden set only after G2.
- Golden baselines cover plane A only. Pattern, significance and compiler changes never require a new baseline (V-PLN-04).

| ID | Test | Method | Gate |
|---|---|---|---|
| V-GOLD-01 | Hash ladders (DET-5) | Yearly hash ladders per engine version: `smoke` for 1,000 years and the other worlds for 200 years on every PR; all worlds at full length nightly | Equal to the approved baseline |
| V-GOLD-02 | Declared divergence | A PR that changes a ladder declares its change class, names the affected systems and attaches the CRN-paired 32-seed stylized-fact comparison | Reviewer approval; director approval if out of tolerance |
| V-GOLD-03 | Change classes (EVO-1; `13` §4.2) | CI detects each PR's class (C0–C3) independently of the declared one. C0–C2: hash comparison. C3: TOST on each stylized-fact statistic of the 32-seed CRN-paired ensemble, margin 10% of the band width | A C0–C2 PR with any `state_hash` change is reclassified C3. A C3 PR is auto-approved only on equivalence; otherwise, or if it changes the determinism machinery, it goes to you |
| V-GOLD-04 | Save-compatibility corpus (DET-10, MOE-4) | Every release opens the saved worlds of all previous releases | History before each migration keyframe is reproduced hash-equal by its archived build, or read from the chronicle. The head migrates through a logged migration keyframe and then runs deterministically (repeat-run hashes equal) |
| V-GOLD-05 | Archived builds (DET-10) | For every segment in the corpus, fetch its engine build by content hash and replay one decade; audit keyframe thinning | Build present and replay hash-equal; no keyframe gap exceeds what an archived build can regenerate |
| V-GOLD-06 | No re-simulation on upgrade (DET-10) | Open a corpus world in the new build and scrub its whole history | The new build re-simulates no archived segment (server counters) |

### 2.4 Sifter (V-SIFT)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-SIFT-01 | Twin equivalence (SIFT-3, SIFT-8) | For each pattern, run the live automaton and the DuckDB SQL twin on recorded chronicles and clustered fixtures, under every declared combination of `selection`, `on_complete` and `dedupe` | Identical match sets, including bindings and event IDs |
| V-SIFT-02 | Clustered fixtures | A generator with actors that have about 20 interaction partners each, symmetric ties (A acts on B and B on A) and repeated interactions over decades. Adversarial cases: `unless` interleavings, an event that both completes a match and matches an `unless` (it completes), deaths of tolerant and non-tolerant roles mid-pattern, window boundaries, several events in one tick. Uniform random fixtures are not accepted | Fixtures used by V-SIFT-01, V-SIFT-03 and V-SIFT-04 |
| V-SIFT-03 | Firing-rate budget (SIFT-8) | Count each pattern's completions per century per 100k persons on the reference ensembles | ≤ 3× the declared budget |
| V-SIFT-04 | precision@K (SIFT-7, SIFT-8) | Precision of each pattern's top-K matches per era against your labelled story-worthy set | Reported per pattern and per pack at each phase gate. An added or LLM-proposed pattern must not lower its pack's precision@K |
| V-SIFT-05 | Anchoring lint (SIFT-1) | `specs/sifting/lint_patterns.py` and the pattern compiler: closed key set, declared typed attributes, every state read anchored | Bare or undeclared reads rejected |
| V-SIFT-06 | Composition and ID space (SIFT-4) | Higher-order patterns consume `match_event`s; edit a pattern and re-run | Bindings imported correctly; simulation EventIds and plane-A hashes unchanged (V-PLN-04) |
| V-SIFT-07 | Incremental matching and auto-pause (SIFT-2, TM-2) | Live sifter at MAX; then with induced lag on the worker thread | Auto-pause at the completing tick; suspense pauses within the rate limit; a lagging sifter pauses and seeks back to the match tick |
| V-SIFT-08 | Significance and retention (SIFT-5, SIFT-6) | Recompute per era twice from the same events and versions; audit the retention walks | Identical components (`narrative_hash` equal). Walks follow only `contingent` edges plus the single top `motivated_by` edge, stop at root kinds or depth 12, and never touch plane A |
| V-SIFT-09 | No LLM in detection (SIFT-7) | Dependency check on `rh-sifter` and the SQL twins | No gateway dependency |
| V-SIFT-10 | Throughput | `bench/sifter_throughput` (V-PERF) | No regression |
| V-SIFT-11 | Mode equivalence under state drift | Anchored attributes change between stages and after completion; the live sifter and the SQL twin run on the same chronicle. Positive control: a harness-only unanchored read (never a DSL feature) | Identical match sets; the unanchored control differs (Winnow's incremental and compiled modes diverge in exactly this way, R12 §19) |
| V-SIFT-12 | Multi-stage conformance | Winnow's published three-stage trace (Kreminski et al. 2021, Fig. 1), re-authored as a fixture, under `all` + `keep` | Exactly the published match; the death or departure of the guest kills every open partial bound to them; the twin agrees |
| V-SIFT-13 | Partial-match bound | Peak live partials per pattern on the clustered 10M-event chronicle | Within the `08` bound; the pool plateaus (no monotone growth over the last half of the run). Winnow's windowless pool grew to 21,269 with no plateau |
| V-SIFT-14 | Why-not | For each expected-but-missing fixture match, query the per-clause twin views | The first failing clause is named (after Felt's `whyNot`) |

### 2.5 Hydration and multi-resolution consistency (V-HYD; `05.2` §3.6)

| ID | Criterion | Test | Gate |
|---|---|---|---|
| V-HYD-01 | Identity stability | Hydrate the same person in different orders, at different times and on different machines | Traits identical |
| V-HYD-02 | Conservation | With the bubble active, compare macro counts with the sum of living rows at every pulse | Equal |
| V-HYD-03 | Non-contradiction | V-DATA-11, run inside the bubble | 0 contradictions with `facts_ledger` |
| V-HYD-04 | Temporal | Run the same seed watched (bubble over the affected settlement) and unwatched | Every scheduled outcome fires at the same tick with the same EventId |
| V-HYD-05 | Distributional fidelity | Activity and destination shares of presented T2c crowds against plane-A schedules, per (cell, hour) bucket | TOST equivalence, δ = ±0.02 absolute share (§1.2) |
| V-HYD-06 | Observer independence | V-DET-06 to V-DET-08 | Hashes equal |
| V-HYD-07 | Rings and budget (R3D-4) | Scripted camera paths: approach, retreat, orbit, speed-band changes | Hydrate at 80 m; dehydrate at 130 m plus ≥ 10 s out of view; never dehydrate in view; ≤ 10 hydrations/s sustained; no T3 above Live speed |
| V-HYD-08 | No NPC-originated material events | Street-level session (Later) with conversations but no material speech acts | Plane-A event set identical to the headless run |
| V-HYD-09 | Light minds | Recompute a light mind for the same person and tick on two machines; converse without material input | Identical; no T1 promotion |
| V-HYD-10 | Fidelity ladder (EVO-3) | Each level against V-SF on its own; F(n) against F(n+1) on 32-seed CRN-paired ensembles | Each level passes its V-SF bands. Conserved quantities equal; declared targets TOST-equivalent within declared margins. Every combination in the supported-combinations matrix tested nightly |
| V-HYD-11 | Boundary adapters (EVO-3) | Lift and project across F0/F1 boundaries | Conservation exact; projected distributions TOST-equivalent within declared margins |

### 2.6 Protocol and interface conformance (V-IF)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-IF-01 | JSON-RPC | A schema conformance suite runs against the server; the console's mock server is checked against the same suite | Pass |
| V-IF-02 | Binary frames (SIM-11, IF-4) | Encode/decode round-trip, delta-against-baseline correctness, fuzzing of corrupt frames | Pass; no crash |
| V-IF-03 | Python bindings | Results from `rh-py` against the CLI | Equal hashes and tables |
| V-IF-04 | Host parity (SIM-11) | The same world through the CLI, wheel, server and (P5+) GDExtension host; dependency check on `rh-core` | Equal hashes; `rh-core` has no I/O dependency |
| V-IF-05 | Viewer process (PERF-5; was R3D-1) | Kill and restart a viewer (`rh-tui`, or the P5 diorama) mid-run | The simulation is unaffected; the client resyncs from a full frame |
| V-IF-06 | Versioning (IF-9) | Contract tests per interface across server, wheel and gdext | Breaking changes only with a major version and a migration |

### 2.7 Performance (V-PERF)

The benchmarks listed in `08` §9 run under criterion in CI on fixed runner hardware, plus a nightly run on the reference desktop.

| ID | Test | Method | Gate |
|---|---|---|---|
| V-PERF-01 | MOP-1 (PERF-1) | `bench/run_1000y_kingdom` (≤ 50k persons, ≤ 2,000 named actors); `bench/run_1000y_100k` and the 1M variant with the optional scale-up (P3.10) | ≤ 2 min for the kingdom (G1b); ≤ 2 min at 100k and ≤ 15 min at 1M with the scale-up (provisional, `08` §1.4) |
| V-PERF-02 | Macro step (PERF-2) | `bench/ledger_month_1M`, `bench/timers_lifecycle_1M`, `bench/serial_fraction`, `bench/raster_8x512`, `bench/agenda_5k`, `bench/plane_hashes`; end-to-end normal and crisis months at 1M | The PERF-2 value set at G1b (provisional: ≤ 60 ms per normal month and ≤ 130 ms per crisis month at 1M); serial share ≤ 20% |
| V-PERF-03 | Sifter overhead (PERF-3) | MAX-speed runs with and without the live sifter; `bench/sifter_throughput` | ≤ 10% on the G2 worlds (G2); ≤ 20% at 1M with the scale-up (Later) |
| V-PERF-04 | Regression gate (PERF-4) | Every `08` §9 benchmark against its baseline | A slowdown of more than 10% fails the build. A speed-up never re-baselines automatically |
| V-PERF-05 | Timeline operations | `bench/keyframe_write_read`, `bench/dropin`, fork | `08` §2 targets |
| V-PERF-06 | Foresight throughput | `bench/foresight_256x25y_100k`; `bench/foresight_64x50y_1M` nightly on a cluster node | `08` §3 targets; batch Foresight at 1M (N = 64 × 50 years) ≤ 60 min on the desktop (with the scale-up, Later) |
| V-PERF-07 | Compiler path | `bench/liveness_gate_100k`; certification wall time | Liveness gate ≤ 90 s wall at 100k on 8 cores (target 60 s); certification ≤ 30 min wall (target 20 min) |
| V-PERF-08 | Belief stores | `bench/belief_store_5k` | Memory and update cost within `08` §5; ≤ 600 MB at 5k actors (G4) |
| V-PERF-09 | Harness isolation (PERF-5; ADR-0030) | `bench/harness_load_100k`: MAX-speed run headless, then the same seed with 4 harnesses attached (`rh-tui` subscribed to frames and the chronicle feed, a live hydration bubble, a control-plane query fuzzer at the maximum rate, and one consumer that stops reading) | Throughput ≥ 95% of headless; engine RSS growth ≤ the in-memory live-edge segment; `state_hash` ladder identical; the stalled consumer later resumes from sealed segments with nothing missing, and never back-pressures the engine |
| V-PERF-10 | High-fidelity daily life (PERF-6) | `bench/solo_80y_minutes`; `bench/minutes_1M_day` with the scale-up (Later) | `solo`: 80 years ≤ 1 min. 1M: ≤ 15 s per sim-day (provisional; recorded with the agent richness used) |
| V-PERF-11 | Allocation budget per tick (RT-2026-10-16) | A counting global allocator in bench builds; steady-state ticks of the `smoke` world from P0, the ladder worlds from P1a. Buffer growth on capacity doubling is counted separately and amortised | Steady-state heap allocations per tick ≤ the budget recorded per bench (default 0 for P0 machinery); more fails the PR like an instruction-count regression |

### 2.8 LLM-facing verification (V-LLM)

| ID | Eval | Method | Gate |
|---|---|---|---|
| V-LLM-01 | RP-200 (LLM-4, LLM-5) | Role-play over our belief store | Leak = 0 on concealment items; unsupported claims < 2%; TTFT p95 < 0.5 s |
| V-LLM-02 | ARG-300 (LLM-6, ORA-2) | Whisper parsing; 60 adversarial items | Field F1 ≥ 0.85; meta-instruction recall ≥ 0.95 |
| V-LLM-03 | Red-team corpus (LLM-7, SEC-1) | Injections, master-key tokens, persuasion taxonomy | No state delta beyond the documented caps; all meta-instructions penalised |
| V-LLM-04 | Schema robustness (LLM-3) | Truncation, refusal and enum-case drift inputs | Handled without crashes; ≤ 3 repair rounds, then the fallback |
| V-LLM-05 | Gateway logging (LLM-1) | Audit every job record | Model ID, weights hash or provider version, prompt hash, latency, tokens and cost present |
| V-LLM-06 | Replay without models (LLM-2) | Replay a recorded session with the network and gateway disabled | Hashes equal (V-DET-04) |
| V-LLM-07 | Registry enums (LLM-3) | Change the Registry and regenerate schemas | Decoding enums follow the live Registry; IDs cannot be invented |
| V-LLM-08 | No numeric deltas (LLM-6) | Schema audit of parser outputs | No numeric state fields |
| V-LLM-09 | Claim check (LLM-5) | Inject unsupported claims into realised lines | Regenerated at most twice, then the template fallback |
| V-LLM-10 | Offline play (LLM-8) | A full play test with the network disabled | Every job kind falls back locally; the world stays usable with degraded narration |
| V-LLM-11 | LLMs never decide (LLM-9; ADR-0034) | Static check of the gateway's job registry against IF-2: every job kind is registered as translator, narrator or QA (`05.6` §0), and only translator outputs map to a journaled input kind. Then replay a recorded session with the gateway disabled | Every job kind has exactly one role; no narrator or QA output has a path into plane A; no input kind carries an action, outcome or parameter chosen by a model; hashes equal (V-DET-04) |

### 2.9 Narrative QA (V-NAR)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-NAR-01 | Per scene (NAR-3) | Graph check, then NLI | 0 CT0/CT1 contradictions; 0 CT3 inventions |
| V-NAR-02 | Per book (NAR-3) | Backbone coverage, causal order, epistemic leaks | Coverage ≥ 95%; every causal parent shown or referenced before its child; 0 leaks |
| V-NAR-03 | Prose | Style metrics, slop and repetition density, cross-book n-gram similarity | Within the style card's bands; below thresholds |
| V-NAR-04 | Exporters | Fountain parse round-trip, page-count band, TTS manifest | Pass; complete voice mappings |
| V-NAR-05 | Provenance (NAR-5) | Sidecar audit | Every sentence maps to event IDs |
| V-NAR-06 | Pipeline artifacts (NAR-2) | Stage outputs | Stages run in order; each artifact versioned with provenance |
| V-NAR-07 | Forms and gate (NAR-1) | Digests that pass and fail the story-shape gate | Failing digests route to Saga or Chronicle; Annals and the templated Chronicle need no LLM |
| V-NAR-08 | NOVEL-20 (NAR-3) | 20 fixed epochs × configurations | 0 CT0/CT1 contradictions |
| V-NAR-09 | Style cards (NAR-4) | Inspection | No living or estate-protected author names; exemplars are US public domain |

### 2.10 Plane isolation (V-PLN; ADR-0024)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-PLN-01 | No B/C reads from plane A | Dependency-graph lint: no crate on the plane-A path (`rh-core`, `rh-contracts`, `rh-sys-*`, `rh-minds`) depends on a plane-B or plane-C crate or type. Compile-fail tests: a test system that tries to read each plane-B and plane-C table must not compile (since SPR-001: `cargo check` fixture workspaces for plane B and plane C; `trybuild` per table once plane-B and plane-C tables exist) | Any edge or compiling read fails CI |
| V-PLN-02 | Plane tags | Every table, column and Registry attribute carries a plane tag. Schema check: no plane-A table holds a plane-B or plane-C reference (`MatchId`, pin or rating IDs) | Zero violations (`05.1` §9) |
| V-PLN-03 | Observation-only APIs | Call every observation API (pins, facts-ledger writes, `bubble.*`, `entity.*`, `sift.rate`) many times, in random order, between pulses | `state_hash` unchanged; `observation_hash` changes |
| V-PLN-04 | Re-sift | Re-sift a recorded timeline with another pattern-library version and recompute significance | `state_hash` ladder and plane-A segments unchanged; `narrative_hash` changes and is reproducible from (events, versions) |
| V-PLN-05 | Pins never promote | Pin 1,000 random commoners through the console during a run | T1 set, agenda contents, event rows and EventIds equal those of an unpinned run |
| V-PLN-06 | Oracle-mode views | See V-ORA-09 | 0 leaks |

### 2.11 Chronicle and data (V-DATA)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-DATA-01 | No physical compaction (DATA-4) | 1,000-year run with per-era significance recomputation and visibility updates; seal segments; fork twice; recompute again | Sealed plane-A segment hashes never change; no named-actor event row is moved or deleted; visibility changes touch only the plane-C sidecar; forks read identical shared bytes |
| V-DATA-02 | Base rates (DATA-4) | Recount per (pattern, era) from the full chronicle after visibility tiering and cold archiving | Equal to the stored base-rate counts |
| V-DATA-03 | Hard-deletion guard (DATA-4) | Dry-run the disk-pressure deletion on a corpus with hot, warm, cold and plane-A-referenced data | Acts only on opt-in cold archives; refuses anything plane A reads or the archived build cannot regenerate |
| V-DATA-04 | Life-course completeness (DATA-8) | After 1,000 years, audit every `PersonId` ever emitted (births, founders, immigrants) | Exactly one life-course entry each; parents, or founder/immigrant provenance; a death record if and only if absent from the live ledger; residence and occupation intervals tile each lifetime with no gaps or overlaps; counts reconcile with births, deaths and migration per region and month |
| V-DATA-05 | Genealogical IDs (DATA-8) | Property tests on the derivation rule; forced-collision injection; two branches that diverge away from a given mother | Every ID matches its rule; collisions resolve by the same deterministic rehash on every run; no ID reused; that mother's n-th child has the same ID in both branches |
| V-DATA-06 | Knowledge fronts (DATA-9) | On small worlds, compare `knows(a, e, t)` from fronts with a brute-force per-person propagation reference | Exact agreement; arrival ticks respect channel latency and never decrease along a route |
| V-DATA-07 | Event-row criteria (DATA-5) | Audit every event row's admission reason; repeat with heavy pinning | Every row has a plane-A reason (tellable act class, T1 participant, rarity, material input); pins create none; regional aggregates and `aggregate_event` counts reconcile with the life-course log |
| V-DATA-08 | Anchored snapshots and intervals (DATA-3) | For every pattern-readable attribute, compare `event_snapshot` with the value at emission; check `state_interval` | Values equal; intervals per (entity, attribute) never overlap |
| V-DATA-09 | Causal edges (DATA-2) | Schema and emission audit | Only the eight kinds; `contingent` edges only from explicit unlocking references |
| V-DATA-10 | Store schema and formats (DATA-1, DATA-6) | Conformance with `specs/chronicle/schema.sql`; delete the DuckDB caches and rebuild | Every table in its declared plane; Parquet and Arrow schemas versioned; rebuilt caches give identical query results |
| V-DATA-11 | Facts ledger (DATA-7, MOE-3) | Display facts, dehydrate, advance a century, display again (the MOE-3 re-open probe) | Every display writes facts with source event IDs; no later presentation contradicts `facts_ledger` |
| V-DATA-12 | Volume | Synthetic 100-year world at 100k and 1M (`08` §5–6) | Measured sizes reported against `05.2` §8. At G4, at the largest P4 world (at 1M with the scale-up): ≤ 3 GB per century; a compressed keyframe ≤ 0.5 GB. More than 2× an estimate opens a risk item (`10`) |

### 2.12 Cognition and epistemics (V-COG)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-COG-01 | Psychological core (COG-1) | Unit and property tests | Fixed-point ranges hold; ≤ 8 OCC slots; emotions decay by half-life; mood returns to its trait baseline; opinion modifiers expire |
| V-COG-02 | Overlays (COG-2) | Overlay values against their effect-language derivations on fixture minds | Equal; no stored overlay state |
| V-COG-03 | Cadence (COG-3, SIM-7) | Agenda instrumentation over 100 years at MAX | Decisions only at polls or interrupts; interrupts at ≥ t + 1 day within the daily cap; zero A*/GOAP calls in fast-forward |
| V-COG-04 | Same-instant semantics (SIM-7) | Fixture instants with conflicting actions; ensemble over seeds | Invalidated actors re-score once, then defer. First-applied rate is not biased by entity ID: TOST on Spearman ρ, δ = ±0.05 |
| V-COG-05 | Beliefs and distortion (COG-4) | Update-rule unit tests; trace rumour variants | Records carry every declared field; each `variant_id` chain reconstructs to its original through `lineage_ptr` |
| V-COG-06 | Channels (COG-5) | Fixture social graphs, one per channel | Latency, fidelity, trust and reach as declared; the hive channel has latency 0, fidelity 1 and full reach |
| V-COG-07 | Pop beliefs (COG-6) | Long run with proposition templates | Shares stay in [0, 1] per pop; templates below ε for longer than τ are collected |
| V-COG-08 | Hive and mimics (COG-7) | Scenario fixtures on `aethelgard` | Every use of hive-only knowledge in action emits an impossible-knowledge event; mimic cover behaviour responds to local risk |
| V-COG-09 | **Withdrawn (ADR-0034).** Was: deliberation consults (COG-8) | — | — |
| V-COG-10 | Motivation edges | Audit utility decisions | Each records its top contributors and margin as `motivated_by` edges |
| V-COG-11 | **Withdrawn (ADR-0034).** Was: the consult A/B [H] (WP P3.12) | — | — |

### 2.13 Oracle and persuasion (V-ORA)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-ORA-01 | Embodiment (ORA-1) | Move the Oracle across the route graph; attempt verbs remotely and without Foresight | Travel time follows route costs; verbs refused unless co-located and funded; mortality hazards apply |
| V-ORA-02 | Whisper pipeline (ORA-2) | Fixed `ArgumentFrame`s through ground, score and apply | Same frame and state give the same Δ; no LLM call in scoring; a meta-instruction gives Δ = 0, raises Paranoia and adds the "uncanny voice" memory; a request against the target's top values flips the sign (reactance) |
| V-ORA-03 | Grounding (ORA-2, ORA-9) | Whispers citing facts that are true in the world but absent from the Oracle's store | Graded as bluffs. Only stored facts count as verified evidence |
| V-ORA-04 | Pierce (ORA-3) | Pierce a hidden mimic | A `vision` belief appears in the Oracle's store only; no NPC belief store changes |
| V-ORA-05 | Omen (ORA-4) | Repeated and failed omens on fixture pops | Credibility decays with frequency and with failed portents; morale and fear move through compartment flows |
| V-ORA-06 | Ward (ORA-8) | Ward under a fantasy and a non-fantasy Registry | Field effect at the coordinate for the declared sim-time duration, journaled; rejected at validation without the Ward |
| V-ORA-07 | Leverage bands (SEC-1) | Parameter sweeps; CRN-paired ΔP per whisper per unit of Foresight over the ARG-300 frames | Strong, verified whispers on receptive targets: 5–15 points. Weak or bluffed: < 3 |
| V-ORA-08 | Adversarial player bot (SEC-1) | Nightly CI gate: a search over red-team texts and argument strategies that maximises ΔP per Foresight | No strategy exceeds the cap envelope |
| V-ORA-09 | Oracle-mode leaks (ORA-9, DET-12) | Plant canary secrets: hidden plots, hive membership, private events. Fuzz every Oracle-mode view and API: map, inspector, chronicle, person pages, `entity.*`, `sift.*`, Foresight outputs and failure clusters, the whisper parse display | 0 returned facts, events or attributes absent from the Oracle's belief store at that tick |
| V-ORA-10 | Oracle-mode Foresight (ORA-9) | Two worlds identical except for one secret the Oracle does not know; run the same prophecy in Oracle mode | P(predicate) TOST-equivalent between the worlds, δ = ±0.03. In Cultivator mode the difference may appear (positive control) |
| V-ORA-11 | Cultivator mode (ORA-9) | Switch modes; enable verbs in Cultivator | Mode switches journaled; verbs disabled by default; enabled verbs journaled `omniscient`; such timelines excluded from Oracle-mode statistics |
| V-ORA-12 | Journaling (ORA-6) | Every verb type in a recorded session | Each appears in the journal and as `player` causal edges |
| V-ORA-13 | **Withdrawn (ADR-0035).** Was: Oracle-3D parity (ORA-7, P7) | Issue the same verbs from the console and from the Godot client on the same seed | Identical journal records and `state_hash` ladders |

### 2.14 Time-Machine console (V-TM)

Playwright drives the console against the real `rh-server` and against the mock server (V-IF-01).

| ID | Test | Method | Gate |
|---|---|---|---|
| V-TM-01 | Speed bands and run-until (TM-1) | Each band; run-until on a date, a predicate and `next_match(salience ≥ θ)` | Stops at the right tick; bands never change hashes |
| V-TM-02 | Auto-pause (TM-2) | Thresholds set in the UI | Pauses on completion and on suspense; the rate limiter holds |
| V-TM-03 | Scrub (TM-3, MOP-3) | Scrub 1,000 years under a performance trace | < 100 ms per update (p95); zero `advance_until` calls |
| V-TM-04 | Drop-in (TM-3, MOP-4, DET-14) | Drop in at random past ticks | ≤ 10 s worst case at 100k; no journal append; head `state_hash` unchanged |
| V-TM-05 | Acting in the past (DET-14) | Seek into the past and issue a material verb from the UI | A new branch appears in the lanes; the canonical timeline is unchanged |
| V-TM-06 | Causal inspector (TM-4) | For random events, compare the rendered graph with `chronicle.causes` and `chronicle.effects` | Same ancestors, descendants, anchored values and significance components |
| V-TM-07 | Map (TM-5) | At random ticks, compare layers with frame data | Equal; LoD aggregation applied |
| V-TM-08 | Rating tool (TM-6) | Rate pauses and chronicle excerpts | Ratings persist in plane C, round-trip and export |
| V-TM-09 | Person and place pages (TM-7) | Living and dead persons | Life-course record, genealogy, life events and facts shown; writes go to plane B only |
| V-TM-10 | UI fuzz (DET-6) | Random walk over screens and controls, excluding material verbs | Server `state_hash` ladder equal to the headless run (V-DET-06) |
| V-TM-11 | Mode display (ORA-9) | Switch modes | Mode shown in the header; Oracle mode omits unknown events (absent, not greyed) |
| V-TM-12 | Provenance | DOM audit | Every number and sentence links to its source events |

### 2.15 World compiler (V-WC)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-WC-01 | INTENT-100 (WC-3) | 100 world prompts → expected template, key overrides and mapping table | Template match ≥ 90%; validity after repair ≥ 98% |
| V-WC-02 | INTENT adversarial subset (WC-4) | 30 of the 100 prompts ask for things the Registry cannot express, phrased to invite a near-miss mapping | **Gap recall ≥ 0.9; silent approximation (an approximate mapping marked `exact`) ≤ 5%** |
| V-WC-03 | Mapping checker (WC-4) | Deterministic checker on every compile | Every Registry ID in the table exists; every ID used by the Intent appears in the table; every approximate or unsupported row files a gap report |
| V-WC-04 | Validators (WC-3, SIM-10) | A passing and a failing fixture for each `05.5` §5 validator, plus the `CLAUDE.md` spec checks: `val_khor` and `aethelgard` validate, `invalid_draft_style` fails. The Rust loader (`rh-cli manifest validate`, SPR-004) gives the same verdict and the same diagnostics as `manifest_v0.py` on every example and on `tests/manifest_parity/` (`bin/manifest_parity.py`). Diagnostics means every `[ERROR]`/`[WARNING]` line, in order, and for schema errors the `(loc, type)` pairs. A ruled divergence (`05.5` §3) is pinned by a `# parity-divergence:` header to exactly its two verdicts and Rust's diagnostic | Every validator behaves as declared; no parity mismatch |
| V-WC-05 | Liveness gate (WC-3) | 200 y × 8 seeds, headless, at 100k | No extinction or explosion; vital-rate guard; ≥ 2 completed arcs per century; **bad-run rate ≤ 1/8**; no invariant violations; wall time per V-PERF-07 |
| V-WC-06 | Calibration repair (WC-3) | Force a failing gate | Every changed parameter stays inside its Registry bounds and is logged before and after; ≤ 30 gate runs; no LLM job after semantic validation (gateway log) |
| V-WC-07 | Certification (WC-3) | 1,000 y × 32 seeds in the background, running V-SF; wall time per V-PERF-07 | The world is labelled *certified* or *uncertified* with its failing bands; nothing is rolled back |
| V-WC-08 | Flavour isolation (WC-2) | CI lint: no `rh-core` or `rh-sys-*` crate depends on flavour asset types. Change only flavour text and re-run | Lint passes; `state_hash` unchanged |
| V-WC-09 | Catalysts (WC-5) | Schema inspection | No forced-outcome fields; trigger→consequence rules rejected |
| V-WC-10 | Effect language (WC-1, EVO-1) | Evaluator tests: termination within declared bounds, fixed point only, reads checked against scheduler read sets, effects only through declared commands, keyed RNG only. Land the `sortition_citizenry` fixture | All hold. The data-only extension merges with a Registry edit plus tests and no Rust change. Golden tests pin lowered expressions |
| V-WC-11 | Registry closure (WC-1) | Generate the JSON Schema from the Registry; feed manifests with unknown IDs | Unknown IDs rejected; Registry semver bumped on change |
| V-WC-12 | Hand authoring (WC-2) | A hand-written YAML world and compiler output for the same world | Same schema; both validate and run |
| V-WC-13 | Gap-report loop (WC-4, G4) | Convert gap reports into Registry entries | ≥ 3 merged, at least 1 of them `data_only` through the effect language |

### 2.16 3D client (V-R3D)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-R3D-01 | Street-from-seed (SPK-2; R3D-2) | The same burg on two machines; grow it from 2k to 5k people; reload a saved world after a generator update | Identical town; > 90% of buildings unchanged; no silent re-roll |
| V-R3D-02 | **Withdrawn (ADR-0035).** Was: crowds spike SPK-1 (R3D-3) | 50 rigged + 3,000 VAT + impostors at 1440p on a 3060-class card | ≥ 60 FPS |
| V-R3D-03 | **Withdrawn (ADR-0035).** Was: wallpaper power spike SPK-3 (R3D-5) | Windows 11 24H2 host; covered, fullscreen and battery cases | ≤ 30 FPS; < 10% GPU; < 5 W extra package power; 0 FPS and stopped audio streams when covered; photograph mode on battery |
| V-R3D-04 | Overrides (R3D-2) | A fire destroys a block; reload and revisit | Override present; the rest of the baseline unchanged |
| V-R3D-05 | **Withdrawn (ADR-0035).** Was: 3D LoD tiers (R3D-3) | Diorama session at each speed band | T3 only at Live; Spectacle visible to draw distance at every band |
| V-R3D-06 | Camera director (R3D-7) [D] | 30-minute wallpaper session; scoring unit tests | Vantage changes on the slow cadence with dissolves; the same inputs give the same choice |
| V-R3D-07 | Replay mode (R3D-8) | Render a recorded history | Zero `advance_until` calls on the host; `state_hash` unaffected; frames match the recorded history |
| V-R3D-08 | Investigation templates (R3D-9) [D] | Play each template end to end; inspect its definition | A clear purpose; outcomes come from simulation state and material inputs only (no scripted-outcome fields) |
| V-R3D-09 | **Withdrawn (ADR-0035).** Was: the 3D art bible (R3D-6) | Asset audit | Palette atlases and LUTs present; every imported asset re-shaded and provenance-logged |
| V-R3D-10 | Glyph performance (R3D-10) | Browser on the reference laptop iGPU; 10k cells with panning and a 1 Hz frame stream; wallpaper covered and uncovered | ≥ 60 FPS uncovered; ≤ 15 FPS cap in wallpaper mode; ≈ 0 rendering work when covered |
| V-R3D-11 | The glyph client decides nothing (R3D-10) | Lint for FOV, pathing and RNG APIs in client code; feed two clients the same recorded server stream | Lint clean; byte-identical rendered cell buffers |
| V-R3D-12 | Glyph-pack completeness (R3D-10) | CI lint over every Registry presentation key and each shipped pack | Every key mapped; every asset's licence on the allow-list with attribution recorded |

### 2.17 Audio (V-AUD)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-AUD-01 | Systemic ambience (AUD-1) | Telemetry trace and mix audit in Diorama sessions | `AudioTelemetry` at ~10 Hz, aggregated per tile; one-shot caps and aggregation rules hold (e.g. ≤ 3 sirens); no neural-audio dependency at runtime |
| V-AUD-02 | Paused audio (R3D-10, wallpaper mode) | Pause and cover the wallpaper | Streams stopped, not muted; the audio device is released |
| V-AUD-03 | Loudness (AUD-3) | ITU-R BS.1770 integrated-loudness and true-peak meter on every episode | −16 LUFS ± 1 LU integrated; true peak ≤ −1 dBTP |
| V-AUD-04 | TTS intelligibility (AUD-3) | ASR transcript of every rendered line against its script, on normalised text (WER); speaker similarity against the voice-bible clip | Lines above the WER gate re-render (best of 3); per-episode WER reported; similarity above its threshold |
| V-AUD-05 | Voice latency (AUD-4) | ≥ 200 scripted spoken turns on the reference machine with the glyph view rendering (Later, if street-level is taken up); measure from VAD end-of-turn to the first NPC audio sample | p95 ≤ 2 s |
| V-AUD-06 | Voices and licences (AUD-2) | Inspection and binary symbol scan | Allow-listed TTS only; designed voices only; no espeak-ng linked |
| V-AUD-07 | Export (AUD-3, SAGA-5) | Validate the MP3 chapters, transcripts and ID3 tags the offline exporter writes | CHAP/CTOC chapters, a VTT transcript and AI-disclosure text present in every file |

### 2.18 Inspection and licences (V-INSP)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-INSP-01 | Licences (LIC-1) | cargo-deny, pip-licenses and license-checker on every PR; provenance check for weights and assets | Personal-use licence mode (D19; ADR-0021 as amended): the licence ledger (`artifacts/licence_ledger.txt`) lists every dependency and never fails a build. Fails only on an item whose licence forbids the use itself, or on copied code that carries no licence |
| V-INSP-02 | ADR conformance | Reviewer-agent checklist on every PR (`13` §7) | Signed off |
| V-INSP-03 | Crate boundaries (EVO-1) | Dependency-graph lint (`bin/lint_arch.py`): system crates depend only on `rh-core`, `rh-ids`, `rh-sample`, `rh-events`, `rh-registry` and contract crates (`rh-sample` by the SPR-003 Architect ruling) | Zero violations |
| V-INSP-04 | Observability (OBS-1) [D] | Headless run with Rerun and the TUI | Per-system timing, invariant counters and sampled agents on the `sim_tick` timeline |
| V-INSP-05 | Engine purity (EVO-4; ADR-0035 §6) | Dependency-graph lint over the engine crates (`rh-core`, `rh-ids`, `rh-sample`, `rh-events`, `rh-registry`, `rh-manifest`, `rh-contracts`, `rh-snapshot`, `rh-cli`, `rh-sys-*` (planned), `rh-sifter` (planned), `rh-server` (planned), `rh-minds` (planned), `rh-worldgen` (planned); `05.0` §6) against one deny-list of media, HTTP-UI and LLM-client crates (e.g. `image`, `rodio`, `cpal`, `symphonia`, `reqwest`, `axum`, `tower-http`, `async-openai`, `tokenizers`), kept in one place, with a positive control | Zero violations. Planned: BCK-002 A-14 adds the rule to `bin/lint_arch.py` in P1a |

### 2.19 World Stream (V-STR; ADR-0030, `05.10`)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-STR-01 | Stream determinism (STR-1) | Run the same seed and journal twice, at MAX and at 1 yr/s, and with 1 and 8 threads; hash channel A segment by segment | Byte-identical channel-A streams |
| V-STR-02 | Fidelity and no hidden state (STR-2) | At 50 random ticks per run, rebuild the state with the materializer (nearest keyframe + replay) and compare `state_hash`; plus a lint that every plane-A write goes through the event API or the replayed step functions | Equal hashes at every sampled tick; lint clean |
| V-STR-03 | Cursor independence (STR-3) | Viewers at 1×, at 100×, rewinding, and one stalled for 10 minutes, all attached during a MAX run | Channel A unchanged (V-STR-01 hash); throughput per V-PERF-09 |
| V-STR-04 | Tip semantics (STR-4) | A scripted viewer 10 sim-years behind issues an action, a scheduled action for a future tick, and an action aimed at a past tick | The first lands at the then-current tip and is journaled with that tick; the second lands exactly on its tick; the third is refused unless issued as a fork; replay reproduces all |
| V-STR-05 | Live edge and catch-up (STR-3) | A viewer falls behind the in-memory edge, reads sealed segments, then jumps to live | `seq` contiguous throughout: no gaps, no duplicates |
| V-STR-06 | Viewer equals engine in focus regions (STR-6; ADR-0031) | In a `blocks` focus region, compare the activity a viewer computes for 10k random person-blocks with the positions the engine used for encounter resolution | Identical |
| V-STR-07 | Virtual telemetry without replay (STR-6) | Query random person-days across a 1,000-year run from the life-course log and seed only; compare with a stateful engine loop on a sample | Identical; < 1 ms per person-day; no keyframe touched (prototype: `specs/stream/selftest_virtual_telemetry.py`) |
| V-STR-08 | Resolution changes count, not looks (STR-7) | Run a region at `life_course` and again at `blocks` from the same keyframe | Displayed activity identical up to the first encounter event; differences only downstream of encounter events |

### 2.20 Sagas and the chronicle pipeline (V-SAGA; ADR-0035)

| ID | Test | Method | Gate |
|---|---|---|---|
| V-SAGA-01 | Vow lifecycle conservation (SAGA-2) | Invariant check after every pulse in debug and CI builds, and over the 1,000-year soak: vows counted by status | sworn = active + fulfilled + broken + void at every tick boundary; fulfilled, broken and void never change; no active vow has a dead vower; 0 violations |
| V-SAGA-02 | Artifact singularity (SAGA-1) | Invariant check after every pulse; transfers through every path (inheritance, theft, gift, loss, finding, forging, destruction), with the bearer's death through each death cause (V-INV-03) | Every artifact not destroyed has exactly one living bearer or one location; a destroyed one has neither; no duplicate ids; 0 violations |
| V-SAGA-03 | Influence bounds (SAGA-1) | Property tests over the in-coupling form for H ∈ [−100, 100]; the S6 isolation run (V-SF-22) | Monthly influence in [0, 2 × `influence_base`]; exposure monotone while borne and 0 after a transfer; `BEARER_CORRUPTED` at most once per bearer-artifact pair |
| V-SAGA-04 | Catalysts are journaled material inputs (SAGA-3, SIM-16) | Schema lint over every catalyst kind; a world with an awakening at year 100 replayed against the same world without it | No catalyst field sets an outcome; every scheduled catalyst is in the journal with its tick and scope; plane-A hashes equal up to that tick; replay exact |
| V-SAGA-05 | Saga-pattern budgets (SAGA-8) | The V-SIFT-01 and V-SIFT-03 machinery on the saga packs, over the reference ensembles and clustered fixtures | Twin equivalence; every saga pattern within 3× its declared budget; precision@K reported |
| V-SAGA-06 | Saga-mechanics ablation (report; MOE-2) | At G2, a third, report-only arm: the same seeds with `S6.artifacts` and `S7.vows` off (WP P2.14), rated in the same blind sessions (§3.4.1) | Reported, not gated: the paired difference treatment − saga-ablated with its 80% t-interval; MOE-1 and arc diversity per arm |
| V-SAGA-07 | Audio chronicle: TTS with an ASR round trip (SAGA-5, AUD-3) | Every exported line is transcribed by ASR and compared with its script (normalised WER; proper nouns checked against the cast list) | A line above the WER gate (default 10%, a design value) re-renders, best of 3, then falls back to text; per-chapter WER and fallback count reported; names recognised in ≥ 95% of lines |
| V-SAGA-08 | Graphic-chronicle panel consistency (report first; SAGA-6, SAGA-9) | Per chapter: embedding similarity of each actor's figure in every panel against their panel-0 character sheet; the share of panels whose depicted cast matches the storyboard's | Reported in P3; it becomes a gate once a threshold is set from the evaluation (R-32) |
| V-SAGA-09 | Vows never remove survival responses (SAGA-2) | CRN-paired fixture: vowers and non-vowers under a forced lethal threat (an armed attacker, a burning hall); a lint over the decision types a vow's weight may enter | Survival-response rates TOST-equivalent between the arms (δ = ±0.05); no vow weight in a survival decision |
| V-SAGA-10 | Sagas form, possible but not forced (SAGA-1, SAGA-2) | Kingdom ensemble, 32 seeds × 200 years after spin-up | G1b defaults (`11`): ≥ 75% of seeds have ≥ 1 vow fulfilled and ≥ 1 broken; every vow kind sworn ≥ 10 times ends in ≥ 2 terminal states; relics change hands in ≥ 75% of seeds through ≥ 2 transfer kinds; ≥ 1 betrayal per seed-century in ≥ 50% of seeds |
| V-SAGA-11 | Secrets v0 and the information horizon (SAGA-4) | Fixture of hidden acts with scripted accomplices and witnesses, at `blocks` (S5 co-presence) and at `life_course` (keyed witness hazard); trace every knowledge-record change | Holders start as exactly {actor, accomplices, witnesses}; every later holder was added by a scheduled outcome (discovery timer, alarm, eyewitness flight, news along the route graph at travel speed, testimony, confession) that fired at its own tick; the discovery hazard rises with evidence and patrol presence as declared; no knowledge arrives faster than travel speed allows |
| V-SAGA-12 | Offline audio export and the local reader (SAGA-5, SAGA-7) | Export a season; open the reader from a local folder with networking off; play chapters; follow 50 random sentence and panel links | Every chapter has valid MP3 chapter marks, a transcript and provenance; the reader works offline and never contacts the engine; every link resolves to its events |
| V-SAGA-13 | Micro behaviour (SAGA-1, SAGA-2) | CRN-paired fixtures: (a) a father murdered by a known culprit, before a son who is a named actor; (b) a relic with rumour reach R, with ambitious named actors inside and outside its reach | (a) the `AVENGE_KIN` swear rate lies inside the range the `T1.minds` spec declares; (b) the seek rate inside the reach exceeds the rate outside (lower 95% bound of the difference > 0) |
| V-SAGA-14 | Visual DNA (SAGA-9) | Recompute the tokens of 10k named actors twice, on two machines and in a fork; compare siblings with unrelated pairs; run with the visual-DNA tool on and off | Identical tokens; tokens only from the Registry vocabulary of the actor's culture and era; sibling token overlap above unrelated pairs (lower 95% bound > 0); `state_hash` and every plane-A RNG counter unchanged by computing it |
| V-SAGA-15 | Cinematic event envelope (SAGA-10) | Schema lint over event emission; stored bytes measured on a 100-year kingdom run; the plane-C derivations run twice | Channel A holds only simulated envelope fields (roles, place, tick, cell where tracked, weather and emotion snapshots where simulated, instrument, contaminant flags, motive edges); no perceptual field in channel A; mean stored overhead ≤ 40 bytes per event (≤ 48 where cells and contaminants are tracked) against the ≈ 35 / ≈ 43-byte budget; every motive edge points to an earlier event; derived fields reproducible; inferred emotion labelled inferred |
| V-SAGA-16 | Stress conservation and break frequency (SAGA-11) | Exact fixed-point accounting per named actor over a 1,000-year soak; break counts on the kingdom ensemble | stress(t) = stress(t0) + accrual − decay − break relief exactly, within [0, 100]; each threshold crossing schedules exactly one break timer, which fires at its own tick, and its act is in the Registry break table; breaks per 100 named-actor-years inside the band the `T1.minds` spec declares (default 0.5–5, a design value) |
| V-SAGA-16b | Cascade scenario: no runaway grief chain (SAGA-11; RT-2026-10-16 item 1) | The `T1.minds` isolation fixture court (60 named actors, 3 houses), 32 seeds: the sovereign is murdered in front of the court at year 50; trace every break, the deaths it causes (`MURDER_ATTEMPT`) and the grief those deaths add | Over the fixture, the chain branching ratio (breaks caused per break, through attempt deaths and grief) < 1, upper 95% bound included; the court's break rate in the year after the death inside V-SAGA-16's band in the ensemble mean; no actor breaks twice within `break_refractory_days`; damping by mechanism only (relationship-scaled grief, per-instant saturation, refractory period), with no lock and no fiat cap |
| V-SAGA-17 | No spontaneous props (SAGA-12) | Audit of every event's items over a 1,000-year soak: artifacts in roles and envelopes, and mundane props drawn from stocks | No event uses an item before its origin event; every artifact's bearer chain runs unbroken from its origin to now, and every bearer change is a chain event with its cause (trade, theft, gift, inheritance, loss); every mundane prop points to an acquisition event and the stock it came from; 0 violations |
| V-SAGA-18 | No reaction before knowledge (SAGA-4) | Audit of every decision, grudge, accusation, alarm and riot that responds to an event, over the soak and the kingdom ensemble | Each reaction references a knowledge-acquisition event of its actor at or before its tick; no riot precedes the discovery it responds to; 0 violations |
| V-SAGA-19 | Immediate succession (SAGA-13) | Kill every titleholder through each death cause (V-INV-03); fixtures with a clear heir, rival claims, a distant heir and an empty treasury | Every titleholder death is followed in the same instant by a succession event or an interregnum-crisis event; no title is vacant across a pulse without a declared interregnum; arbitration weighs heir, claims, military proximity and treasury as declared |
| V-SAGA-20 | Fugitive movement is continuous on the route graph (SAGA-14) | Trace every fugitive's positions on the kingdom ensemble; tile paths in a `blocks` focus region | Every position change is a scheduled move along an M1 route edge (a tile path inside focus regions); consecutive positions are adjacent (no teleporting); while fugitive, a next move or an arrival, capture or death is always scheduled (no standing still); interception hazards per segment as declared |
| V-SAGA-21 | Provenance of change (SAGA-15) | Audit of every damage or transformation state (ruins, scars and maimings, captured or cursed artifacts, burned fields); the plane-B chronicle index queried for 1,000 random places and cells | Every such state carries a cause EventId that resolves to an existing, earlier event; no per-tile last-mutating-event column exists; the index answers "what last happened here?" from the chronicle |

## 3. Validation (fit for purpose)

### 3.1 Stylized-fact harness and certification (V-SF; MOP-5; R09 §11)

The bands are used by two ensembles:
- The **liveness gate** (200 y × 8 seeds; V-WC-05) blocks play. It checks only liveness: no extinction or explosion, vital rates, ≥ 2 completed arcs per century, bad-run rate ≤ 1/8, no invariant violations. A 200-year run cannot see secular cycles, polity lifespans or tail exponents.
- **Certification** (1,000 y × 32 seeds; V-WC-07, and nightly per reference world) evaluates every band below. MOP-5 is measured on certification ensembles.

Each band carries four tags (R12 §14; pattern-oriented modelling, Grimm et al. 2005; Fagiolo et al. 2007):
- **Kind.**
  - *Identity:* the fact follows directly from a model equation or a calibrated input. A failure is an implementation bug and blocks, whatever the overall pass rate.
  - *Emergent:* the fact needs several systems to interact. Emergent bands are the evidence that history happens, and their pass rate is reported separately.
- **Role.**
  - *Calibration:* the band may be used when tuning Registry bounds and defaults during development.
  - *Held-out:* the band is sealed before P1b tuning starts and is never used to tune. It is the independent prediction that gives a pass its credibility. Held-out bands: **V-SF-03c, V-SF-03d, V-SF-03e, V-SF-04b, V-SF-05, V-SF-06c, V-SF-07, V-SF-09b, V-SF-11b, V-SF-11c, V-SF-14, V-ERA-08, V-ERA-09**. All the others are calibration bands.
- **Level:** individual, household, polity or system. Patterns at several levels are what constrain structure; one level alone does not.
- **Discriminates:** the submodel candidates the band can tell apart (§3.8), or none.

**Bounds are set by filtering, not hand-tuning.** Registry bounds and defaults are set as follows:
1. Sample parameters by Latin hypercube within the Registry bounds.
2. Keep the samples that pass the *calibration* bands.
3. Narrow the bounds to the survivors.
4. Report how much sensitivity dropped (Grimm et al. 2005's inverse-modelling filter).

Held-out bands are evaluated only after this.

**Welfare-ratio conventions** (Allen 2001, R12 §18). These are measurement conventions for comparison with the sources, not mechanics:
- **WR_A** (region, year) = 250 × labourer day wage ÷ (3.15 × C), where C is the cost of the Registry basket `baskets.allen2001` at regional prices, with the climate-zone variant. The 3.15 is three baskets for a family of four plus 5% rent. Compute it yearly and then average per window. It needs a posted wage, so it is INSUFFICIENT in economies without a wage market.
- **WR_h** (household) = income from the M4 income account, in kind included and valued at regional prices, ÷ ((1 + rent share) × adult-male equivalents × C).
- **WR_kcal** (household) = calories available, bought plus produced, ÷ (adult-male equivalents × 1,941 × 365). It is used where households do not buy food.
- **Allen's WR = 1 line is a "respectability" poverty line, not subsistence.** The bread-only floor costs about 0.44 of it. Identity checks therefore use WR_h or WR_kcal, never WR_A. The IPF's α (V-SF-16) uses the subsistence floor (`subsistence_kg_grain` valued at grain prices), never Allen's line.

| ID | Fact | Band (scope stated per row; era-conditioned bands are keyed to the era label of the window tested, ADR-0027; sources in `research/R12_literature_verification.md`) | Kind | Statistic and test |
|---|---|---|---|---|
| V-SF-01 | Long-run growth without technology shocks | mean r ∈ [−0.1, +0.3]%/yr; population ∈ [0.3, 5]·K | Identity | Seed-mean r with t-interval; population range per seed |
| V-SF-02 | Child mortality to age 15 (agrarian) | 35–55%. At β = 1 this is equivalent to Brass α ∈ [0.20, 0.61] and e₀ ∈ [23.4, 36.6] (R12 §Brass; `specs/demography/selftest_brass.py`). An agrarian manifest with e₀ ≈ 45 implies q(15) ≈ 25% unless β < 1 | Identity | q(15) from the life-course log, per seed. INSUFFICIENT below about 400 births (a ±5-point band at 2σ) |
| V-SF-02b | Adult mortality shape | l(65)/l(15) inside the range the manifest's (α, β) implies on the Brass General Standard ± 0.03; β ∈ [0.64, 1.64] unless the manifest declares extrapolation | Identity | From the life-course log; pins β, which V-SF-02 alone does not |
| V-SF-03 | Famine mortality crises (agrarian windows; R12 §17). The old band ("≥ 1 event of ≥ 10% loss per century") was a plague band cited to Black Death pages; no famine source supports it | **Detector** (Del Panta & Livi Bacci, as used by Alfani et al. 2017, p. 28): per settlement, normal deaths = the mean of years t−6…t−2 with the maximum and minimum dropped; a crisis is a year with deaths ≥ 1.5× normal; small 1.5–2×, medium 2–4×, major ≥ 4×. A *famine* crisis falls in a regional food-shortage window (staple price ≥ 1.5× its 7-year trimmed normal) with plague deaths < 50% of the excess. **Pressured windows** (N/K in the top third of the run, no low-correlation second staple): famine crises per settlement-century: any class 2–5, medium or worse 1–3, major 0.1–0.5 (Italy, 17th century: 3.6 / 2.1 / 0.27, derived). In the worst famine of a 300-year window, 50–90% of settlements are in crisis (1590–93: 79%; 1648–49: 73%). Plague crises are counted and reported separately; their band waits for a plague source | Emergent | Settlement level. Units ≥ 1,000 persons for the any-class count, ≥ 300 for medium or worse, ≥ 100 for major, so that Poisson false positives stay near 1% a year; smaller settlements are pooled into parish clusters. Fewer than 10 units in a region-window → INSUFFICIENT. Rate per unit-century with a seed-level bootstrap CI. Needs annual deaths per settlement and cause (`05.1` §7) |
| V-SF-03a | Harvest shortfalls (temperate zones; Campbell & Ó Gráda 2011, Tables 2–3) | Composite staple yield (½ winter, ½ spring crops), detrended. **Pre-improvement:** gross CV of a single winter staple 0.13–0.19 (0.165); composite 0.09–0.13 (0.111). P(composite net yield < 0.9 × trend) 0.15–0.30 a year; P(< 0.8) 0.07–0.16. Back-to-back composite < 0.9: 0.04–0.10 a year; < 0.8: 0.01–0.035. Three in a row < 0.8: ≤ 0.01 a year ("once in two or three centuries"). Lag-1 autocorrelation of detrended yield \|ρ\| ≤ 0.15. **Improved husbandry:** single staple CV 0.10–0.15, composite 0.08–0.12. **Industrial:** composite 0.03–0.07 | Identity | Region level. Realised statistics must also match the manifest's analytic values, P(shortfall ≥ d) = Φ(−d/CV). Clopper–Pearson CIs; regions pooled only by block bootstrap. Minimum 200 region-years (1,000 for three in a row). HP filter λ = 100, with 6.25 reported (the source does not state λ) |
| V-SF-03b | Price response and famine price amplitude — **report only (ADR-0035)** | (a) Mean absolute year-on-year staple price change: 0.20–0.30 with low integration and dear storage (≈ 25%, 13th–14th centuries); 0.10–0.16 when integrated (13%, 17th–19th centuries). Reduced-form demand elasticity, low integration: σ-ratio method 0.45–0.80; first-difference method 0.40–0.75. (b) Peak-year price ÷ 7-year trimmed normal, in the worst famine of each 300-year pressured window: median across markets ×2.0–3.5, maximum ×3–5 (Modena 1527–29 ×4.0; Naples and Bari 1648–49 ×3.4). In buffered windows no famine has a median above ×1.8. **"Two bad harvests send grain ×3–5" is the extreme tail; typical shortfall years give ×1.5–2** | Emergent | Region level; windows of ≥ 100 years; seed-level bootstrap. Both elasticity methods are computed, because storage and trade move them in opposite directions. Not evaluated where the price is an off-map input |
| V-SF-03c | What triggers famine | ≥ 70% of famine windows that put ≥ 25% of settlements in crisis follow ≥ 2 consecutive composite shortfalls of ≥ 10%, or coincide with a siege or requisition. Isolated single-year shortfalls of 10–20% do so in < 10% of cases (the thresholds are ours; the pattern is the sources': "hardship … hardly classic famine conditions", Campbell & Ó Gráda p. 867) | Emergent | Region level; Clopper–Pearson CIs on both shares |
| V-SF-03d | Entitlement, not stock alone | (i) When a ≥ 30% mortality shock coincides with ≥ 2 consecutive shortfalls of ≥ 25%, real wages rise and the famine-crisis share falls below that of the CRN-paired run without the epidemic (1349–51: famine "forestalled by plague", pp. 871–872). (ii) At equal shortfall depth, the crisis share rises with the pre-shock share of households below the subsistence floor (30–40% in 1315 against < 20% in 1437) | Emergent | Household → region. (i) CRN-paired difference with CI (from P3, once M5 exists); (ii) partial Spearman ρ > 0, CI excluding 0. INSUFFICIENT below 100 households per region |
| V-SF-03e | General famines per polity-century (soft until the Alfani & Ó Gráda 2017 Introduction is read) | A general famine puts ≥ 50% of settlements in crisis in ≥ 3 regions. Pressured agrarian windows: 2–5 per century (Italy, Centre-North: five large-scale famines 1250–1350; four in the 16th century; four or five in the 17th), median onset gap 15–40 years. After a demographic collapse (low N/K): 0–1. Buffered: 0–2 (Italy, 18th century: two, milder; England after 1742: none) | Emergent | Polity level; Poisson CI per class; needs ≥ 3 regions with ≥ 10 units each |
| V-SF-04 | Secular cycles (agrarian windows: agricultural share > 50%) — **gated from G4 (needs M6 F1; ADR-0035)** | Measured on N/K (population over carrying capacity), decadal. **(a)** Cycle length between turning points: ensemble median 200–330 years, with ≥ 80% of cycles in 150–450 (Ibn Khaldun-type polities with polygynous elites and nomad counter-elites: 80–150). **(b)** Growth (integrative) phases 100–220 years; decline (disintegrative) phases 50–170, usually no longer than the growth phase. **(c)** N/K peak-to-trough ≥ 1.5. **(d)** The elite peak lags the population peak by about a generation; elite numbers fall by ½–¾ from peak to trough (Turchin & Nefedov 2009: 12 cycles, 160–415 years, median ≈ 270; R12 §12) | Emergent | Turning points of smoothed N/K per seed; cycle and phase lengths pooled over seeds with bootstrap CIs; INSUFFICIENT if fewer than 30 completed cycles in the ensemble. A spectral peak is reported only (cycles are not periodic, p. 21), and only on runs of ≥ 2,000 years. Not applied in industrial windows (ADR-0027) |
| V-SF-04b | Instability contrast between phases — **gated from G4 (needs M6 F1; ADR-0035)** | Instability events per decade, decline phase ÷ growth phase: > 1 in every completed cycle; ensemble median 3–10, target ≈ 6 (Table 10.2: 0.6 vs 3.8; China 3.4 vs 11.3) | Emergent | Per seed, Wilcoxon test of decadal counts by phase, p < 0.01; ratio pooled with bootstrap CI. Phase boundaries come from N/K turning points, not from instability, to avoid circularity |
| V-SF-05 | Fathers-and-sons violence waves — **gated from G4 (needs M6 F1; ADR-0035)** | Intervals between internal-violence peaks within decline phases: median 40–60 years, interquartile range inside 30–75 (Turchin & Nefedov 2009: 50 ± 10 years in all 8 cases). Waves may also appear, milder, in growth phases | Emergent | Peaks detected per seed on decadal internal-violence counts; intervals pooled across seeds. Not a spectral test: a decline phase holds only 2–3 peaks. Interstate war onsets are excluded (no periodicity: Clauset 2018) |
| V-SF-05b | Structural-demographic lead–lag (soft) | The relative-wage downturn leads the rise in relative elite numbers by 10–30 years; the rise of Ψ̄ leads major internal violence by 10–30 years (Turchin 2013: ≈ 20 and ≈ 15–25 years, one country) | Emergent | Cross-correlation lag of detrended series per seed; ensemble median lag with bootstrap CI. Soft until a second source confirms it |
| V-SF-06 | Ruler murder (era-conditioned) | Murders per 1,000 ruler-years by institutional state: weak or contested succession (like 600–1099) 6–25; consolidating (like 1100–1399) 7–12; institutionalised hereditary monarchy (like 1400–1799) 1–5. Pooled 5–15 only for mixed medieval scenarios. Violent share of ruler *deaths* (not reigns) 15–30% (Eisner 2011: 22.4%) | Identity | Poisson rate per institutional-state window with exact CI (counts are small late; report them); violent share with Clopper–Pearson CI |
| V-SF-06b | Deposition by succession law | Share of reigns ending in domestic deposition: primogeniture 8–16%, elective 21–49%, agnatic seniority 43–57%; primogeniture vs elective hazard ratio inside [0.17, 0.63] (Kokkonen & Sundell 2014) | Identity | Cox model on simulated reigns stratified by polity, natural death and foreign conquest censored; HR with CI |
| V-SF-06c | Reign length follows institutions | Mean completed reign: unconstrained rulers (salaried or slave army, no assembly) 8–15 years; constrained (landed military elite plus assembly) 17–22 years; the gap appears within 1–2 entry centuries of the institutional shift and is +6 to +12 years within the same era (Blaydes & Chaney 2013) | Emergent | Kaplan–Meier restricted mean (τ = 60 years) by institutional state; difference with bootstrap CI. Shorter reigns go with more deposition (rank correlation < 0) |
| V-SF-06d | Regicide contagion (soft) | A murdered predecessor raises the odds of the next ruler's murder: odds ratio 1.5–4 (Eisner 2011: 2.98) | Identity (the odds ratio is a Registry parameter, `predecessor_killed_or`) | Logistic regression on consecutive reigns; OR with CI |
| V-SF-07 | Polity lifespans | **(a) Shape, all top-level polities:** Weibull shape k ∈ [0.8, 1.7], i.e. neither strong early mortality nor strong ageing. Arbesman's own 41 empires give k̂ ≈ 1.26 (bootstrap 90% CI 1.11–1.62) and reject a pure exponential at p ≈ 0.01 (R12 §16), so the old test (k equivalent to 1 within [0.75, 1.33]) would have failed real history. **(b) Scale, empire-class polities only** (top-level polities ruling ≥ 3 subordinate polities): mean lifespan 150–320 years (41 empires, 3000 BCE–600 CE: mean 215, exact 95% CI 162–300). Those empires had millions of subjects, so most worlds below 1M persons report INSUFFICIENT here, which is correct. **(c) Other polities:** mean reported, not gated, until a source on small-polity survival is read | Emergent | **Censoring-aware.** Polities alive at the horizon are right-censored. Kaplan–Meier survival curve; censored Weibull MLE, with the shape CI by bootstrap over seeds. (a) passes when the 90% CI of k̂ lies inside the band. (b) mean = total exposure ÷ ends, 90% CI inside the band; INSUFFICIENT below 20 completed spells pooled. Also report log-normal vs Weibull ΔAIC. A lifespan runs from founding to absorption, partition or fragmentation; a regime change inside the same polity does not end it (ADR-0027). Align with Taagepera's duration definition once read. Dynasty tenures get the same Kaplan–Meier treatment and are reported |
| V-SF-08 | City sizes (soft until n suffices) | Settlement-size tail exponent α̂ (pdf) ∈ [1.8, 2.6] (Zipf corresponds to α = 2; US cities α = 2.37 ± 0.08: Clauset, Shalizi & Newman 2009) | Emergent | The V-SF-09 procedure on settlement populations at snapshots ≥ 200 years apart. **Not** OLS on log–log ranks, which CSN 2009 shows is biased. Soft while the pooled tail has n < 100 (likely at 100k persons) |
| V-SF-09 | War severity (era-invariant shape) | Heavy tail, α̂ (pdf) ∈ [1.3, 1.9]. This is Rehoboam's widening of Clauset (2018)'s 95% CI [1.37, 1.76] for interstate wars 1823–2003 and CSN (2009)'s war intensity α = 1.7 ± 0.2. Cederman's model gives 1.49–1.64 and its ablations ≈ 2.3, so the band separates them. Absolute severity *and* intensity (deaths ÷ belligerents' population) are both logged; severity is the pre-registered band statistic | Emergent | **CSN 2009 recipe, deterministic** (R12 §CSN): exact discrete MLE for counts; x_min minimising the KS distance among candidates leaving n_tail ≥ 50; plausibility by semi-parametric bootstrap, B = 2,500 with a full refit each, **pass iff p > 0.1** (unrounded); the exponential must be rejected (normalised LR > 0, p < 0.1); lognormal, stretched exponential and cutoff (Wilks χ²₁) reported. α̂ pooled over seeds with a 10,000-resample seed-level bootstrap CI. Plausibility is also run **per seed** where n_tail ≥ 50 (≥ 75% of those seeds must pass), because pooled tests reject finite-population cutoffs. A power law with a population-scaled upper cutoff is accepted (pre-registered). Pooled n_tail < 100 → **INSUFFICIENT**, counted as a fail and shown separately. Sort keys and RNG keys as in R12 |
| V-SF-09b | War onsets are memoryless | Gaps between interstate war onsets are geometric-plausible; the dispersion index of yearly onset counts lies in [0.7, 1.5] (Clauset 2018: geometric q̂ = 0.43, p = 0.13) | Emergent | Per era window: CSN-style bootstrap plausibility of the geometric fit (≥ 75% of seeds p > 0.1); dispersion index with bootstrap CI. There is no system-wide war cooldown, by design |
| V-SF-09c | The war tail is produced, not fitted | Freezing power-projection technology, or turning off war contagion, raises α̂ by ≥ 0.3 or makes the tail implausible (Cederman 2003: ≈ 1.55 → ≈ 2.3 under ablation) | Emergent | CRN-paired ablation ensembles (32 seeds each); difference in α̂ with bootstrap CI. Fails if the ablated model still produces the tail |
| V-SF-10 | Malthusian test (agrarian windows) — **report only (ADR-0035)** | A technology shock raises density, not long-run income. Consistent with Allen (2001): England's population grew about sevenfold over 1500–1850 with flat real wages (p. 433) | Identity | CRN-paired shock and no-shock ensembles. Density increase with lower 95% bound > 0; long-run real income (means over windows of ≥ 50 years, since half-century swings of −18% to +11% occur in the data) TOST-equivalent to no change, δ = ±5% |
| V-SF-10b | Malthusian wage pressure and its escape (agrarian and early-modern windows) — **report only (ADR-0035)** | (a) With technology frozen, induced population growth lowers the labourer welfare ratio WR_A (lower 95% bound of the decline > 0). Allen 2001: over 1500–1750, craftsman WR fell 39–59% and labourer WR 28–52% in eight continental cities (Kraków labourers −9%, from an already low level). (b) When productivity grows as fast as population, 50-year means of WR_A stay within ±20% of the start (London −18% to +11% while population rose sevenfold) | Emergent | CRN-paired ensembles: technology frozen vs a forced productivity schedule, with the same induced population growth in both. Discriminates M4 wage formation (§3.8): the null fails (a) |
| V-SF-11 | Post-mortality-shock real wage (agrarian windows) — **report only (ADR-0035)** | After a regional population loss of ≥ 20% within ≤ 5 years, mean WR_A over years +10 to +50 exceeds its mean over years −25 to −1. Years 0–2 are excluded, because wages dip during the shock itself (London masons c. 1348–50; Allen 2001, Fig. 10), and the rise to the plateau took 50–70 years. The ≥ 20% threshold is ours; Allen gives no mortality figure | Identity | Sign per qualifying event; sign test over events pooled across seeds, p < 0.05; INSUFFICIENT below 10 events |
| V-SF-11c | Black-Death-scale wage magnitude (soft: one historical event, baseline read from a figure) — **report only (ADR-0035)** | For population losses of 30–60%: WR_A over years +40 to +60 divided by WR_A over years −25 to −1 lies in [1.2, 2.5]; over years +60 to +150, in [1.3, 2.5]. Allen 2001: labourers rose from about 0.75 before the plague (p. 430) to 1.07 (London) and 1.24 (northern Italy) in 1350–99, then 1.3–1.65 in the 15th century (×1.4–1.65, then ×2.0–2.15) | Emergent | Per event; ensemble median with bootstrap CI; typicality (§3.1): Allen's London ratio inside the 5–95% range of per-event values |
| V-SF-11b | Post-plague levelling (soft until Alfani 2021 is read) — **report only (ADR-0035)** | After a mortality shock of ≥ 20%, the top-10% wealth share falls below its pre-event level within 50 years; a CRN-paired ablation with impartible or entailed inheritance plus regressive taxes levels less | Emergent | Sign test over qualifying events across seeds, p < 0.05; ablation difference with CI |
| V-SF-12 | Hive / testing (modern) | At prevalence < 1% with se/sp = 0.99, more than 50% of detentions are false positives | Identity | Detention outcomes on `aethelgard`, with Clopper–Pearson CI |
| V-SF-13 | Sensitivity screening | — | — | Morris and Sobol screening on labelled PRs flags parameters with outsized influence |
| V-SF-14 | Demographic-structural crises (agrarian era; soft until *Secular Cycles* is read) — **report only (ADR-0035)** | State breakdowns follow 35–100% population growth over 2–5 generations, with grain prices ×3–7 and real wages about −50% (trough decades against the prior peak, deflated by a basket CPI, not grain: half-century means fall less, by 17–18% in London and 39–59% for craftsmen in continental cities over 1500–1750; Allen 2001, R12 §18); growth phases of ≤ 25% stay calm; breakdown usually follows elevation of all three Ψ strains, and two produce a crisis without breakdown (Goldstone 1991); this is a tendency, not a rule: Rome broke down without a prior fiscal crisis (Turchin & Nefedov 2009, pp. 306, 314). In growth phases population rises from +80% to ×3, prices ×4–6 and real wages −50–75% (the −75% end is a Phelps Brown–Hopkins reading; the largest fall on Allen's basis is about −66%) | Emergent | Per seed: classify windows by population growth; breakdown frequency by growth class with Clopper–Pearson CIs; count of elevated components at each breakdown |
| V-SF-15 | Industrial damping (from era pack E3) | Once fiscal instruments, food trade and industrial labour absorption are present, population growth no longer produces breakdown; polities without them still do (Goldstone 1991, pp. 288–338, 460–461) | Emergent | CRN-paired E3 ensembles with and without the damping technologies; breakdown hazard ratio with CI < 1 |
| V-SF-16 | Income inequality (agrarian windows, α ≤ 7) — **report only (ADR-0035)** | Person-weighted Gini of per-capita household income (in kind included), as Gini2 over ≥ 8 class or occupation groups: every seed-window in [24, 64]; ensemble median in [33, 58]. **Extraction ratio** ER = Gini ÷ IPF, where the inequality possibility frontier IPF = (α − 1)/α and α = mean income ÷ subsistence: ensemble median 55–100%, per seed 45–115%. ER > 100% for over a decade must coincide with raised hunger mortality. ER falls as α rises (Spearman ρ < 0). Modern windows: ER 25–65% (Milanovic, Lindert & Williamson 2011; R12 §13) | Emergent (ER > 100% rule: identity) | Per region-window; INSUFFICIENT below 100 households |
| V-SF-17 | Wealth inequality (soft until Alfani 2021 is read) — **report only (ADR-0035)** | Wealth Gini and top-10% share over households from valued holdings (with and without zero-wealth households): wealth Gini > income Gini; the top-10% share rises over centuries without a ≥ 20% mortality shock | Emergent | Sign tests; INSUFFICIENT below 200 households (Gini) or 400 (top-10% share); n/(n−1) correction |
| V-SF-18 | Household structure (soft until sources are read: Laslett; Hajnal) | Mean household size and the share of complex (multi-generation or multi-couple) households fall inside the manifest culture's declared range; the age at first marriage for women follows the culture's declared marriage pattern | Emergent | Household level; per region-window; INSUFFICIENT below 100 households |
| V-SF-19 | Polity consolidation (soft) | With rising power-projection technology the number of independent polities falls over centuries; with technology frozen it does not (Cederman 2003: 200 → ≈ 35 states) | Emergent | System level; CRN-paired technology-on vs technology-frozen ensembles; sign test on polity-count trend |
| V-SF-20 | Agrarian welfare-ratio levels (agrarian and early-modern windows) — **report only (ADR-0035)** | Labourer WR_A per region and half-century: every window in [0.35, 1.7]; ensemble median in [0.61, 1.36] (Allen 2001's 10th–90th percentiles; median 0.89; 65% of city-periods below 1). Craftsman WR_A: every window in [0.65, 2.9]; median in [0.90, 2.06]. Skill premium (craftsman ÷ labourer day wage): ensemble median in [1.3, 2.0] (Allen's median 1.57, partly imposed by assumption) | Emergent | Per region-window; INSUFFICIENT where no wage was paid in the window (e.g. a pastoral household economy) |
| V-SF-20b | Subsistence floor coupling | A household whose calorie-entitlement ratio WR_kcal stays below 0.44 (the bread-only floor) for more than a year has a raised hunger-mortality hazard in M2. This is consistent with V-SF-16's ER > 100% rule | Identity | Per household-year at any N, N = 1 included; a violation is an invariant breach |
| V-SF-21 | Regional price dispersion (soft) — **report only (ADR-0035)** | In agrarian and early-modern windows with ≥ 9 market regions, the ratio of the dearest to the cheapest regional basket cost lies in [1.8, 4.0] (Allen 2001 Table 4: 2.0–3.5). Dispersion falls as `market_integration` rises, and does not fall with trade volume alone (pp. 424, 432) | Emergent | Per window; Spearman ρ(dispersion, `market_integration`) < 0 |
| V-SF-22 | Isolation harness: each system alone (RT-2026-10-16) | Before a system is coupled, it runs under the synthetic forcing its spec declares in `[isolation]` (constant, step, sine, ramp, and historical series where available) for the spec's horizon, its other inputs held at their neutral values. It passes when its own bands and invariants hold and its behaviour matches the spec's `expect`: bounded, no runaway, no extinction unless the forcing demands it | Gate | Required for each system before its in-couplings are switched on (P1a, P1b); `lint_odd.py` requires the forcing for every rate or mode read |
| V-SF-23 | Coupling-strength sweep (RT-2026-10-16) | The coupled engine at coupling strength α ∈ {0, 0.25, 0.5, 0.75, 1}, where α scales each in-coupling's deviation from its declared `neutral` value; 8 seeds × 200 years per α (the liveness-gate ensemble). α = 0 is the G2 coupling-ablated control | Report | Per α: liveness, the V-SF pass count, event rates by act class and V-COUP chain counts. Flags any α at which every seed goes extinct or no V-COUP chain completes (the runaway and flatline attractors). Reported at G1b and G2 |

**Gate (MOP-5).**
- **Per band.** A band **passes** when the 90% interval of its ensemble statistic lies inside the band. It **fails definitively** when the interval lies wholly outside. Anything else is **marginal** and is reported as such.
- **Gate rule.**
  - At least 90% of the non-soft bands pass.
  - Every identity band passes.
  - **No held-out band fails definitively.**
- **Phasing.** A band is evaluated once the systems it needs exist: V-SF-01, V-SF-02 and V-SF-02b from G1a; the ADR-0035 gate bands below from G1b; V-SF-03d, V-SF-04, V-SF-04b, V-SF-05, V-SF-09, V-SF-09b, V-SF-09c and V-SF-12 from G4 (`11`).

**Gate status under ADR-0035 (the saga engine).** The new gates need demography, dynasty duration, famine mortality and succession, not economic history. Every band is kept and computed; the economic-history bands are **reported, not gated**: they do not count toward MOP-5, an identity band among them does not have to pass, and a held-out band among them cannot fail a gate (its result is still reported). The tags in the table above mark them.

| Status | Bands |
|---|---|
| **Gate from G1a** | Demography: V-SF-01, V-SF-02, V-SF-02b. Isolation: V-SF-22 (M2, M7, `T1.minds`) |
| **Gate from G1b** | Demography (above); dynasty and succession: V-SF-06, V-SF-06b, V-SF-06c, V-SF-07; famine mortality: V-SF-03, V-SF-03a, V-SF-03c, V-SF-20b; isolation V-SF-22 for every P1 system |
| **Gate from G4** (the systems arrive in P4) | V-SF-03d (plague forestalls famine), V-SF-04, V-SF-04b, V-SF-05 (secular cycles and violence waves; M6 F1), V-SF-09, V-SF-09b, V-SF-09c (war; M8), V-SF-12 (hive; M5) |
| **Report only (ADR-0035)** | Economic history: V-SF-03b (prices), V-SF-10, V-SF-10b (Malthusian wages), V-SF-11, V-SF-11b, V-SF-11c (post-shock wages and levelling), V-SF-14 (price and wage crises), V-SF-16, V-SF-17 (inequality), V-SF-20 (welfare ratios), V-SF-21 (price dispersion) |
| **Soft, unchanged** | V-SF-03e, V-SF-05b, V-SF-06d, V-SF-08, V-SF-18, V-SF-19 |
| **Report, unchanged** | V-SF-13 (screening), V-SF-23 (coupling sweep); V-SF-15 waits for era pack E3 |
- **Non-ergodicity reporting.** Seeds can diverge into different regimes, so for every band the report also gives:
  - the **share of seeds** whose own statistic lies inside the band;
  - between-seed vs within-seed variance (first vs second half of the run);
  - regime clusters, if the per-seed values are multimodal.

  A seed-mean inside a band that no individual seed reaches is flagged.
- **Typicality.** Where a band is anchored on a single real history, such as Clauset's 1823–2003 war tail, the real value must also lie within the 5–95% range of the per-seed values. A real history is one draw, not the expected value.

### 3.2 Drama-coupling scenarios (V-COUP; `05.1` §8)

For each designed coupling, a 32-seed ensemble checks two properties. The unit is a seed-century (320 units in a 32-seed × 1,000-year ensemble), or a seed-run for the 10-year modern world (`11`).
- **Possible.** The full chain completes in ≥ x% of units.
- **Not forced.** The chain is absent in ≥ y% of units.

x and y are set numerically per coupling before the phase starts (defaults in `11`) and logged in the decisions log. The gates use point estimates; Clopper–Pearson 95% intervals are reported.

| ID | Coupling | Phase |
|---|---|---|
| V-COUP-01 | Famine → unrest → succession crisis (truncated at regency or civil-war onset until `M8.war` lands) | P1b; full chain P4 |
| V-COUP-02 | Plague → paranoia → purge → labour revolt | P4 |
| V-COUP-03 | Hive detection spiral (modern) | P4 |
| V-COUP-04 | Elite overproduction → violence wave (needs M6 F1; ADR-0035) | P4 |
| V-COUP-05 | Dragon raids, with dragon numbers inside the energetics bound | P4 |

### 3.3 Foresight validity (V-FOR; ORA-5, ORA-9)

Per-decile reliability with about 20 prophecies per bucket cannot pass even for a perfect forecaster (RT-M5). v1.0's "p ± 0.05 per decile" gate is therefore replaced by the tests below, which run nightly on the cluster.

**Back-test set.** Recorded timelines of the reference worlds, forked at ≥ 20 ticks each. At each fork, N = 256 rollouts run with fresh salt lineages. The recorded canonical continuation is the held-out outcome. The prophecy panel holds ≥ 200 binary predicates over 5–50-year horizons (succession, war, famine, regicide, hive detection).

| ID | Test | Method | Gate |
|---|---|---|---|
| V-FOR-01 | Exchangeability | Rank histogram of the canonical continuation among the rollouts, for continuous quantities at the horizon (population, treasury, named deaths), and randomised PIT for the binary predicates; 10 bins | Uniform: χ² p ≥ 0.01, with ≥ 400 continuations per quantity so that the test has ≥ 0.8 power against outer deciles holding 15% of the mass each (under-dispersion) |
| V-FOR-02 | Reduced-cost bias | Reduced-cost rollouts (coarser cadence and richness) against full-cost rollouts, CRN-paired, per predicate | TOST: the 90% CI of ΔP lies inside ±0.03 for every predicate in the panel |
| V-FOR-03 | Scoring | Brier and log score with bootstrap CIs, against the climatological base rate. Spiegelhalter's Z = Σ(o − p)(1 − 2p) ÷ √(Σ(1 − 2p)² p(1 − p)) | Brier skill score > 0 (lower 95% bound); \|Z\| ≤ 1.96 |
| V-FOR-04 | Reliability diagram | Pooled reliability diagram with Wilson intervals per bin | Reported (a diagnostic, not a gate) |
| V-FOR-05 | Oracle-mode information loss (ORA-9) | V-FOR-03 in Oracle mode (belief-resampled forks) against Cultivator mode on the same panel | Reported. Belief-resampled rollouts are not held to the V-FOR-02 bound, because they differ from true-state rollouts by design (`05.3` §7) |
| V-FOR-06 | CRN variance reduction | Paired against independent intervention ensembles | Reported against the `08` §3 estimate |

### 3.4 Human validation [H] (V-HUM; MOE-1, MOE-2, MOE-3, MOE-6)

| ID | Activity | Protocol | Gate |
|---|---|---|---|
| V-HUM-01 | **Story-yield sessions** (MOE-1) | At G2, run under the pre-registered protocol (§3.4.1). At G4, the same protocol with fresh held-out seeds | Pause budget ≤ 10 per century. P2: median ≥ 3 events rated ≥ 4 per century, precision ≥ 0.5. P4: ≥ 6, precision ≥ 0.7 |
| V-HUM-02 | **Chronicle A/B** (MOE-2) | ≥ 40 blind pairs (4 per seed) on the G2 seeds: the chronicle of a run against the chronicle of the coupling-ablated run (§3.4.1) with the same engine, manifest, seed and sifter. Both chronicles come from the frozen template pipeline (no LLM text), and their lengths are within ±20% (the longer is cut at an arc boundary) | **Proposed rule (Q21):** the lower bound of the two-sided 80% Clopper–Pearson interval is > 0.5 (≥ 25 of 40); the point estimate is reported against the `01` target of 80%. Under the old rule (≥ 16 of 20), a true 70% preference passes only 24% of the time, and a true 80% only 63% (R12 §20) |
| V-HUM-03 | **Bad-run audit** (MOE-6) | Automated: seeds with fewer than N = 2 completed arcs per century (complete matches from ≥ 2 pattern packs), 32 seeds per manifest | < 10% (point estimate). The Clopper–Pearson 95% interval is reported: with 32 seeds, even 0 bad runs gives an upper bound of about 11%. A borderline result is resolved by adding seeds |
| V-HUM-04 | **Interrogation sessions** (MOE-3; P2 text; voice only if street-level is taken up) | Interrogate 20 random people per session; log contradictions | 0 hard, < 1 soft per 20 |
| V-HUM-05 | **Narrative read** (P3) | Blind: Rehoboam saga against an ungrounded LLM saga on the same premise; rate on HANNA criteria plus "would keep reading" | 10 pairs. Rehoboam preferred on coherence and on surprise in ≥ 7 of 10 each, and on engagement in ≥ 5 of 10 (no regression) |
| V-HUM-06 | **Diorama believability** (P5) | 30-minute ambient sessions; note "fake" moments: pops, loops, impossible errands | ≤ 2 "fake" moments per session, averaged over the last 3 sessions, with a falling trend |
| V-HUM-07 | **Oracle play** (P4) | 3 Oracle-mode sessions of about 1 h | You rate "whispers feel consequential but not controlling" ≥ 4/5 in ≥ 2 of 3 |
| V-HUM-08 | **Full-vision acceptance** (Later, if old P6 and P7 are taken up) | Scenarios OS-1 to OS-11 (`04`), in their ADR-0035 form | Accepted |
| V-HUM-09 | **G2 thesis gate** | §3.4.1 | §3.4.1 decision rule |
| V-HUM-10 | **Personal preference model** | §3.5 | Spearman ρ ≥ 0.4 by G4 |
| V-HUM-11 | **Random-window control** (selection check; reported, not gating) | Per G2 seed, 6 control items: windows with the same simulated span and event count as a randomly chosen treatment pause, drawn from the same seed and century with `H(preregistration_hash, "g2-rand", seed, i)`, rendered by the same templates and interleaved blind (the control design of Behrooz et al. 2015) | Per-seed difference, treatment-pause mean − control mean, with the lower bound of the 80% t-interval (detects about 0.37 points). A non-positive bound points the iterate cycle at patterns and significance rather than couplings |
| V-HUM-12 | **Story × narration factorial** (P3, before any claim that the Narrative Compiler adds value) | 2 × 2 within rater: a top-significance arc vs a random arc of matched span from the same seed, × the template Chronicle vs a Narrative Compiler excerpt of matched length. 20 items per cell in Latin-square order, rated on the G2 item plus HANNA engagement | Reports the story effect, the narration effect and their interaction. Lessard et al. (2026) found narration explained more of the ratings than the story did (R12 §20) |
| V-HUM-13 | **A season of the saga** (G3; SAGA-5–SAGA-7) | One season (a simulated century of one saga world, 6–10 episodes) in the local saga reader: audio drama, prose and graphic chronicle, in story order. Not blind: this rates the product, not a selection. Each episode rated 1–5 on "I want the next episode", with notes | Median episode rating ≥ 4, and ≥ 70% of episodes rated ≥ 3 (defaults; fixed before P3) |

#### 3.4.1 G2 thesis gate: pre-registered protocol (V-HUM-09)

G2 tests the core thesis (S1): unforgiving, coupled systems, with vows and relics in plane A, produce sagas you want to follow. **The rated items are sagas** (ADR-0035). If it fails, no illustrated-saga work (P3) starts (`11`). The evaluator is also the person who tuned the engine, so the protocol is **pre-registered**.

**Pre-registration**
1. The protocol is filled in and frozen before the first evaluation run: engine build, Registry version, held-out manifest hash, seed list, pause thresholds, preference-model hash, **rendering version** (hash of the annals and rating-card templates), presentation-order seed and analysis-script hash. The filled-in instance is committed and its hash recorded in `docs/decisions-log.md`. It is not edited afterwards. Any deviation is logged and reported with the result.
2. **Held-out material.** Seeds are derived as `H(preregistration_hash, "g2", i)` after freezing, so none can have been used in tuning. Certification (§3.1) is also run on the held-out manifest, as an independent prediction test. One **held-out manifest**, hand-authored during P1 and sealed (hash committed, never run), is opened only for G2.
3. **Seeds.** 10 seeds: 7 on the tuned saga reference manifests at kingdom scale (WP P1.11, e.g. `kingdom` and `delta_republic`) and 3 on the held-out manifest.
4. **Personal preference model** (§3.5). Frozen before the run, with its weights hash in the pre-registration. Results are reported both **without** it (default pause thresholds; the primary analysis) and **with** it.

**Arms**
- **Treatment:** the engine as built.
- **Baseline: coupling-ablated simulation** (the MOE-2 baseline, `01`). The same build, manifests, seeds and sifter, with the cross-system couplings of `05.1` §8 disabled. Implementation: every read a system makes of another system's owned columns (through its contract-crate accessors) is served from an independent seed's recorded run of the same manifest. Each system keeps its own dynamics and marginal behaviour, but causation between systems is removed. Significance and pause selection are unchanged. The ablation switch is a test-harness feature, never available in play, and needs a P2 work package.
- **Report-only arm: saga mechanics ablated** (V-SAGA-06; ADR-0035). The same build, manifests, seeds and sifter, with `S6.artifacts` and `S7.vows` switched off. Its items are interleaved blind with the other arms in the same sessions. Its result is reported beside the decision and never changes it. It adds about a third to the rating time; if the window is tight it runs on 5 of the 10 seeds, decided before freezing.

**Session design**
- **Horizon.** 2 simulated centuries per seed and arm, after the standard spin-up.
- **Pause budget.** ≤ 10 auto-pauses per simulated century, per arm and per selection rule. The candidate pool (completed matches and suspense partials) is ranked by default salience ("without") and by the frozen model ("with"). The union is rated.
- **Rating.** You rate each item 1–5 on "I want to know what happens next" in the rating tool (TM-6), with the causal inspector and the previous decade's annals available. Treatment and baseline items from the same seed are interleaved in keyed random order with arm labels hidden. Ratings are locked before any arm is revealed.
- **Rendering held constant.** Every rated item, control and MOE-2 chronicle uses the frozen template pipeline only, with item text capped at a fixed length. LLM polish and LLM match explanations are off. The rating view hides significance and suspense scores. Narration alone can move identical events from 3rd to 12th of 12 (Lessard et al. 2026, R12 §20), so a comparison whose rendering differs between arms measures narration, not history.
- **Random-window control** (V-HUM-11; reported, not gating): 6 control items per seed, interleaved blind. The coupling ablation tests whether coupled systems make better material; this control tests whether the sifter picks well.
- **Whose taste.** G2 measures **your** taste (S1: stories *you* want to read). Designers rate their own worlds higher than naive readers do (Lessard et al. 2026), so the absolute bar (≥ 2.5) does not generalise to other readers. The paired difference is robust to it. An external reader panel is optional and comes later (Q22).
- **Effort.** About 40–60 items, or 3–4 hours of rating, per seed, plus about 6 control items (+3–6 hours in total); about 35–45 hours in total, spread over several sessions.
- **Intra-rater reliability.** A random 10% of items are re-presented, blind, at least 7 days later. Agreement is measured with linearly weighted Cohen's κ.
- **No interim looks.** The analysis runs once, after all ratings are locked.

**Analysis** (primary: default pause selection; every quantity is also reported for the "with" selection)
- The unit of analysis is the seed: one mean rating per seed and arm.
- **Mean rating:** lower bound of the two-sided 80% t-interval over the 10 per-seed treatment means (9 df: mean − 1.383·s/√10).
- **Treatment vs baseline:** per-seed paired difference d = treatment mean − baseline mean; lower bound of the two-sided 80% t-interval of mean d. A Wilcoxon signed-rank test is reported alongside.
- **Pause count and precision:** auto-pauses per century, and the fraction rated ≥ 3, per arm.
- **Arc-signature diversity:** the share of treatment items rated ≥ 4 whose match belongs to each pattern family (Registry pattern pack), pooled over seeds.
- **By pause type:** MOE-1 precision and mean rating are reported separately for suspense partials and completed matches, because the rating item is forward-looking.
- **Power statement** (frozen with the protocol): with 10 seeds and one-sided α = 0.10, 80% power needs a standardised seed-level effect of ≈ 0.71. With item SD ≈ 1, ≈ 25 items per arm per seed and between-seed heterogeneity ≈ 0.25 (all assumptions; recompute from the rating-tool pilot before freezing), that is ≈ 0.27 rating points. Smaller true effects will tend to read as "iterate".
- **MOE-1:** items rated ≥ 4 per century, median over seeds. **MOE-2:** V-HUM-02 on the same seeds. **MOE-6:** completed arcs per century per seed, and the bad-run rate with a Clopper–Pearson 95% interval, on the 10 rated seeds and on the 32-seed automated ensemble.

**Decision rule**

| Condition | Requirement |
|---|---|
| **Pass criterion** | The lower 80% bound of the mean treatment rating is ≥ 2.5 on the 1–5 scale, **and** the lower 80% bound of the paired difference (treatment − coupling-ablated) is > 0. Both under the primary analysis |
| **Diversity guard** | No single pattern family accounts for more than 40% of treatment items rated ≥ 4 |
| **Validity** | κ ≥ 0.4; blinding intact (after each session you guess the arm of 20 random rated items, and the exact 95% CI of your accuracy must include 0.5); no deviation from the frozen protocol that touches seeds, manifests, thresholds or analysis. If validity fails, the run is void and repeated: it is neither a pass nor a fail |

G2 is **go** when all of the following hold:
1. the pass criterion;
2. the diversity guard;
3. the P2 targets of `01`: MOE-1 (≥ 3 per century rated ≥ 4, precision ≥ 0.5, within the pause budget), MOE-2 (the V-HUM-02 rule) and MOE-6 (< 10%, point estimate);
4. the verification gates binding at P2 (§5, §6).

Otherwise G2 is **iterate** (`11` P2: couplings, contingent unlocking, patterns). After 3 failed cycles the `11` pivot criterion applies.

### 3.5 Personal preference model (V-HUM-10)

Ratings train a small model (logistic regression over the significance components) that personalises auto-pause thresholds.
- **Validate by hold-out.** On each new session, its ranking of pauses should correlate with your ratings at Spearman ρ ≥ 0.4 by G4.
- **Freeze before gates.** For any gate evaluation it is frozen beforehand, and results are reported with and without it (§3.4.1).

### 3.6 Era transitions (V-ERA; SIM-13, SIM-14; ADR-0027)

Worlds must be able to pass through eras in one run. These tests check that era change comes from mechanisms, not from dates or labels.

| ID | Test | Method | Pass |
|---|---|---|---|
| V-ERA-01 | Mechanics never read the era label | Static lint over `rh-sys-*` and the Registry expressions; a mutation test that flips the label mid-run | Lint clean; `state_hash` identical with the label flipped |
| V-ERA-02 | No forced modernity | Agrarian reference manifest, technology invention disabled, 1,000 years × 32 seeds | V-SF-01 holds; era label stays `agrarian` in every seed |
| V-ERA-03 | Hazards follow institutions, not dates | Ensembles with `succession_institutionalisation` and `exec_constraint` swept independently of time | Murder and deposition rates fall monotonically with institutionalisation (Spearman ρ < 0, CI excludes 0); a calendar term added to the regression has a CI that includes 0 |
| V-ERA-04 | Institutional divergence | Two polities from the same start, one gaining a landed military elite and an assembly, one a salaried army | V-SF-06c gap (+6 to +12 years) appears within 1–2 simulated centuries of the shift |
| V-ERA-05 | Primogeniture adoption | Polities that adopt primogeniture endogenously (one clean turnover required) | Within-polity deposition HR after adoption inside [0.17, 0.63]; short-run foreign-conquest risk not lower in the first two centuries (Kokkonen & Sundell 2014) |
| V-ERA-06 | Transition hindcast (from era pack E3) | Agrarian start with a forced technology schedule (sanitation, medicine, fossil energy) | Mortality decline precedes fertility decline, with the lag inside the band set from Lee (2003) when pulled; output per head sustains growth. Bands fixed before the pack starts |
| V-ERA-07 | Engine frontier | A world driven to the last supported era | It keeps running, files exactly one frontier gap report per technology, and continues under a newer build through a migration keyframe with pre-migration seeks reproducing archived hashes |
| V-ERA-08 | Early-modern divergence hindcast (era pack E2) | Two or more regions from the same agrarian start (labourer WR_A ≈ 1.0–1.6), all with population growth. The leader gets rising productivity and `market_integration` on a forced schedule; the laggard does not. The era label is never read (V-ERA-01) | Over 250 years: (a) the laggard's WR_A falls 25–60%; (b) the leader's stays within ±20%; (c) the leader-to-laggard WR_A ratio rises from ≤ 1.4 to ≥ 1.6 (Allen 2001: 1.34 → 1.93); (d) the leader's nominal wage gap exceeds its real gap, because its basket is dearer; (e) capital regions with large fiscal spending may hold their WR_A (Madrid, Paris, Warsaw): reported, not gated |
| V-ERA-09 | Famine fades with buffering, not with dates | CRN-paired ensembles sweeping, independently of time: (a) adoption of a low-correlation second staple; (b) `market_integration` and storage cost; (c) relief or provisioning capacity | Famine-crisis rates fall: buffered ÷ pressured ≤ 0.6 for any class, ≤ 0.5 for medium or worse, ≤ 0.4 for major (Italy, 17th → 18th century: 0.33 / 0.20 / 0.12, derived). The price → mortality slope falls toward 0 as relief rises, while the food-riot hazard per price spike does not fall and rises with the share of households that grow no food (Campbell & Ó Gráda p. 881). A calendar term added to the regression has a CI that includes 0 |

### 3.7 Scale range (V-SCALE; SIM-1, SIM-15, SIM-16; ADR-0028)

| ID | Test | Method | Pass |
|---|---|---|---|
| V-SCALE-01 | Invariants and determinism across the range | The same build at N ∈ {1, 10, 60 (`uz_job`), 1k, 10k, 100k, 1M}; 100 years each (1,000 at N ≤ 10k) | 0 invariant violations; identical plane hashes at 1/2/4/N threads; journal replay exact |
| V-SCALE-02 | No false statistics at small N | Run the certification harness on N = 1, 10 and 60 worlds | Every band reports INSUFFICIENT or a valid result; none passes or fails below its declared minimum |
| V-SCALE-03 | Cost scales with work | Per-tick wall time against N and active timers | Fixed per-tick overhead ≤ 1 ms; a 1-person world runs 1,000 years at daily cadence in ≤ 10 s |
| V-SCALE-04 | Extinction is a valid end | Worlds driven to zero population | An end event in channel A (the journal holds inputs only); chronicle, console and replay stay consistent |
| V-SCALE-05 | Agentive catalysts are bounded | `uz_job` with the Adversary under deterministic and scripted-player policies; 64 seeds | No verb outside the stage scope; `spare_life` never violated; the protagonist's renounce/hold-fast outcome varies across seeds (both outcomes occur at least once), which shows it is not forced |
| V-SCALE-06 | Personal-arc liveness (micro) | `uz_job` 64 seeds × 10 years | ≥ 1 completed personal arc (trial, lament, counsel dispute or restoration) per seed-decade in ≥ 90% of seeds |

### 3.8 Submodel tournaments (pattern-oriented model selection)

Where the sources support rival mechanisms, the choice is made by **strong inference against patterns**, not by taste. The method follows Grimm et al. (2005); the guard against selecting by small likelihood margins follows Fagiolo et al. (2007, p. 211).
1. **Pre-register** in `docs/decisions-log.md`:
   - the candidates, always including a **null**;
   - the discriminating patterns;
   - the calibration budget.
2. **Calibrate every candidate equally,** with the same budget, within Registry bounds, on calibration bands only, with CRN seeds.
3. **Reject** a candidate on a definitive fail of any held-out band. **Never** choose between survivors by small differences in fit.
4. **If two or more survive,** log "not discriminated". Keep them as Registry variants, and look for a sharper pattern.

**First tournaments:**

| Mechanism | Candidates (with null) | Discriminating patterns |
|---|---|---|
| M6 Ψ | Turchin's multiplicative form with a geometric-mean threshold; Goldstone's additive mobilisation term with three-strain gating; **null:** a constant instability hazard | V-SF-04b, V-SF-05, V-SF-05b, V-SF-07 (shape: the null gives k ≈ 1, the source k̂ ≈ 1.26), V-SF-14 |
| M8 attrition exponent W | Derived from force composition; one fixed value per world; **null:** 0.5 | V-SF-09, V-SF-09c, campaign-loss distributions (battle data cannot separate them, R12 §11) |
| M7 ruler hazards | Institution-driven (R12 §1–3); **null:** a constant hazard per ruler-year | V-SF-06, V-SF-06b, V-SF-06c, V-ERA-03 |
| M4 wage formation | Marginal product of labour on land, with food relative prices; an institutional wage that follows it with lag τ = `wage_stickiness_years`; **null:** a real wage independent of labour supply | V-SF-10b, V-SF-11, V-SF-11c, V-SF-20 |
| M3 yield process | AR(1) with ρ fitted; AR(1) with ρ ≤ 0.2 plus rare multi-year anomaly regimes (2–7 years; 1315–21, 1349–51); **null:** iid Gaussian | V-SF-03a: deep shortfalls cluster far more than shallow ones (back-to-back ratio to independence 1.1–1.35 at < 90% of trend but 2.4–6.7 at < 70%), which one AR(1) cannot produce |
| M4 famine mortality | Entitlement (household income ÷ price against the subsistence basket); availability (regional stock per head); both; **null:** a price-independent famine hazard | V-SF-03c, V-SF-03d, V-ERA-09 |

## 4. Observability for V&V

- **Rerun.** Per-system timing, invariant counters, sampled agents, the hydration bubble and AOI cells on a `sim_tick` timeline.
- **Textual TUI.** Ticks/s, memory, pattern hits and logs for headless runs.
- **Run report** (auto-generated per run): plane hashes, stylized-fact table with identity/emergent tags, liveness or certification status, bad-run metrics, top arcs, firing rates against budgets, timings, costs.
- **Gate report** (per phase gate): suite results by test ID, the requirement → test matrix with status, and for G2 the pre-registration hash and the §3.4.1 analysis.

## 5. Phase gates (summary; see `11_roadmap.md`)

Each phase ends with:
1. All verification suites green for the requirements binding at that phase (§6).
2. The validation activities scheduled for that phase completed.
3. A **go / iterate / pivot** decision recorded in `docs/decisions-log.md`, with evidence links.

| Gate | Gate-specific evidence (numbers in `11`) |
|---|---|
| G0 | V-DET-01, V-DET-02, V-DET-05, V-DET-09, V-DET-11, V-WC-04, V-PERF-04 |
| G1a | Demography bands V-SF-01, V-SF-02, V-SF-02b; V-SF-22 for M2, M7 and `T1.minds`; V-DET-17 (CRN pairing); V-INV-02, V-INV-03, V-INV-12; V-COG-03, V-COG-04; V-STR-01–V-STR-03, V-STR-05; V-SAGA-15, V-SAGA-16, V-SAGA-19; [H] you read the family history of 3 seeds |
| G1b | V-SAGA-01–V-SAGA-04, V-SAGA-09–V-SAGA-11, V-SAGA-13, V-SAGA-17, V-SAGA-18, V-SAGA-20, V-SAGA-21; the G1b gate bands of §3.1 (MOP-5); V-SF-22 for every P1 system; V-SF-23 reported; V-COUP-01 (truncated); V-PERF-01, V-PERF-07; [H] you read the run reports and saga digests of 3 seeds |
| G2 | The pre-registered protocol (§3.4.1): V-HUM-01, V-HUM-02, V-HUM-03, V-HUM-09; V-SAGA-06 reported; MOP-3 and MOP-4 (V-TM-03, V-TM-04); V-PERF-03 on the G2 worlds; V-SIFT-01, V-SIFT-03, V-SIFT-04, V-SAGA-05; V-DET-06; V-GOLD-04; V-SAGA-07, V-SAGA-12; V-LLM-11 |
| G3 | V-HUM-13 (the season); V-NAR-01, V-NAR-02, V-NAR-08; V-HUM-05; V-HUM-12; V-SAGA-07, V-SAGA-14, V-SAGA-08 reported; V-AUD-03, V-AUD-04, V-AUD-07; V-LLM-10 |
| G4 | V-COUP-01 (full chain) to V-COUP-05; V-SF on all gate bands, including those gated from G4; V-COG-01–V-COG-08; V-DATA-06, V-DATA-12, V-PERF-08; MOE-1 ≥ 6 under the G2 protocol (V-HUM-01); V-HUM-07; V-FOR-01–V-FOR-03; V-ORA-07–V-ORA-10; V-LLM-02, V-LLM-03, V-LLM-11; V-WC-01, V-WC-02, V-WC-10, V-WC-13; V-HUM-10 |
| G5 | V-R3D-01 (SPK-2), V-R3D-10, V-R3D-11, V-R3D-12; V-DET-07; V-HUM-06 |
| Later (if old P6 and P7 are taken up) | V-DET-08; V-HYD-01, V-HYD-02, V-HYD-05; V-LLM-01; V-HUM-04 (voice); V-AUD-05; V-HUM-08 |

## 6. Requirement → test matrix

Every requirement in `03` maps to at least one test ID, except withdrawn ones. Phases are those of `03`.

### 6.1 System requirements

| Req. | Subject | Phase | Tests |
|---|---|---|---|
| SIM-1 | Ledger rows, genealogical IDs, ≥ 2M persons | P1a | V-INV-09, V-DATA-05, V-DATA-12, V-PERF-02 |
| SIM-2 | Region graph and raster | P1a | V-INV-10 |
| SIM-3 | Must-have systems | P1a/P1b/P4 | V-SF-01–V-SF-12, V-SF-22, V-SF-23, V-COUP-01–V-COUP-04, V-INV-01 |
| SIM-4 | `SystemModel` trait | P0 | V-INV-04, V-INSP-02 |
| SIM-5 | Hazards and binomial draws | P1a | V-INV-08, V-SF-02, V-DET-12 |
| SIM-6 | Clocks, `i64` sim-time, adaptive daily steps | P1a | V-INV-11, V-DET-01 |
| SIM-7 | Agenda and same-instant semantics | P1a | V-COG-03, V-COG-04, V-PERF-02 |
| SIM-8 | Conservation | P1a | V-INV-01, V-INV-02, V-INV-05, V-HYD-02 |
| SIM-9 | Great houses and dormancy | P1a | V-INV-12 |
| SIM-10 | Horizon declaration | P0 | V-WC-04 |
| SIM-11 | Library and hosts | P0–P2 | V-IF-01, V-IF-02, V-IF-03, V-IF-04, V-IF-06 |
| SIM-12 | Should-have systems | P4 | V-INV-13, V-INV-14, V-COUP-02, V-COUP-05, V-SF-01–V-SF-12 |
| SIM-13 | Era-generic mechanisms | P1b | V-ERA-01, V-ERA-02, V-ERA-03 |
| SIM-14 | Endogenous institutions and era packs | P4 / packs | V-ERA-04, V-ERA-05, V-ERA-06, V-ERA-07, V-SF-06b, V-SF-06c |
| SIM-15 | No minimum-N assumptions; scale profiles | P1a | V-SCALE-01, V-SCALE-02, V-SCALE-03, V-SCALE-04 |
| SIM-16 | Agentive catalysts | P1b | V-SCALE-05, V-SCALE-06, V-SAGA-04 |
| DET-1 | History function | P0 | V-DET-04, V-DET-18, V-GOLD-01 |
| DET-2 | Keyed RNG and branch salts | P0 | V-DET-09, V-DET-16, V-DET-17, V-DET-19, V-DET-20 |
| DET-3 | Fixed-point thresholds | P0 | V-DET-03, V-DET-19 |
| DET-4 | State hash and divergence bisect | P0 | V-DET-01, V-DET-02, V-DET-11 |
| DET-5 | Golden-run CI | P0 | V-GOLD-01, V-GOLD-02 |
| DET-6 | Observer invariance | P2 | V-DET-06, V-DET-07, V-DET-08, V-TM-10, V-PLN-03, V-HYD-06, V-HYD-08 |
| DET-7 | Input journal | P0 | V-DET-04, V-DET-18 |
| DET-8 | Keyframes | P2 | V-DET-05, V-PERF-05 |
| DET-9 | Forking | P2 | V-DET-13, V-DET-16, V-DATA-01 |
| DET-10 | Frozen past, migrated future | P2 | V-GOLD-04, V-GOLD-05, V-GOLD-06 |
| DET-11 | Immutable frames, unsynced guard | P1b | V-DET-10 |
| DET-12 | Three planes | P0/P2 | V-PLN-01–V-PLN-06, V-ORA-09, V-HYD-09 |
| DET-13 | No back-dating | P1a | V-DET-12, V-HYD-04, V-SIFT-01 |
| DET-14 | Acting in the past forks; input scope | P2 | V-DET-13, V-DET-14, V-TM-04, V-TM-05 |
| DET-15 | Pull inputs. **Withdrawn (ADR-0034)** | — | None (V-DET-15, V-COG-09 withdrawn) |
| DATA-1 | Typed store and plane assignment | P1a–P4 | V-DATA-10, V-PLN-02 |
| DATA-2 | Causal edge kinds | P1a | V-DATA-09, V-SAGA-15 |
| DATA-3 | Anchored snapshots and intervals | P1a | V-DATA-08, V-SAGA-15 |
| DATA-4 | No physical compaction; base rates | P2 | V-DATA-01, V-DATA-02, V-DATA-03 |
| DATA-5 | Event-row criteria | P1a | V-DATA-07, V-DATA-12, V-PLN-05 |
| DATA-6 | Durable formats | P0 | V-DATA-10 |
| DATA-7 | Facts ledger | P2 | V-DATA-11, V-HYD-03 |
| DATA-8 | Life-course log and genealogical IDs | P1a | V-DATA-04, V-DATA-05, V-DET-17, V-HYD-01 |
| DATA-9 | Knowledge fronts | P4 | V-DATA-06 |
| SIFT-1 | Pattern DSL and anchoring | P2 | V-SIFT-05, V-SIFT-01 |
| SIFT-2 | Incremental matching | P2 | V-SIFT-07 |
| SIFT-3 | SQL twins | P2 | V-SIFT-01, V-SIFT-02 |
| SIFT-4 | Match events, `MatchId` space | P2 | V-SIFT-06, V-PLN-04 |
| SIFT-5 | Significance | P2 | V-SIFT-08 |
| SIFT-6 | Hindsight retention | P2 | V-SIFT-08, V-DATA-01 |
| SIFT-7 | No LLM in detection | P2 | V-SIFT-09, V-SIFT-04 |
| SIFT-8 | Match semantics and budgets | P2 | V-SIFT-01, V-SIFT-02, V-SIFT-03, V-SIFT-04 |
| COG-1 | Psychological core | P1a/P4 | V-COG-01 |
| COG-2 | Derived overlays | P1b/P4 | V-COG-02, V-WC-10, V-SAGA-03 |
| COG-3 | IAUS + HTN cadence | P1a/P4 | V-COG-03, V-COG-10, V-PERF-02 |
| COG-4 | Belief records | P4 | V-COG-05, V-PERF-08 |
| COG-5 | Channels | P4 | V-COG-06, V-DATA-06 |
| COG-6 | Pop-level beliefs | P4 | V-COG-07 |
| COG-7 | Hive and mimics | P4 | V-COG-08, V-COUP-03 |
| COG-8 | Deliberation consult. **Withdrawn (ADR-0034)** | — | None (V-COG-09, V-COG-11, V-DET-15 withdrawn) |
| LLM-1 | Gateway | P2 | V-LLM-05 |
| LLM-2 | Journaled outputs, no re-query | P4 | V-LLM-06, V-DET-04 |
| LLM-3 | Constrained decoding | P3 | V-LLM-04, V-LLM-07 |
| LLM-4 | TTFT | Later | V-LLM-01 |
| LLM-5 | Plan-then-realise, claim check | Later | V-LLM-01, V-LLM-09 |
| LLM-6 | Closed-schema parsing | P4 | V-LLM-02, V-LLM-08 |
| LLM-7 | Red-team corpus | P4 | V-LLM-03 |
| LLM-8 | Offline use | P3 | V-LLM-10 |
| LLM-9 | LLMs never decide; the engine never waits on a model | P2 | V-LLM-11, V-LLM-06 |
| WC-1 | Mechanics Registry | P0 | V-WC-11, V-WC-10 |
| WC-2 | Hand-authored manifests, flavour asset | P1b | V-WC-12, V-WC-08, V-WC-04 |
| WC-3 | Compile pipeline, liveness, certification | P4 | V-WC-01, V-WC-04, V-WC-05, V-WC-06, V-WC-07, V-SF-13, V-PERF-07 |
| WC-4 | Mapping table and gap reports | P4 | V-WC-02, V-WC-03, V-WC-13 |
| WC-5 | Catalysts | P1b | V-WC-09 |
| TM-1 | Speed bands and run-until | P2 | V-TM-01 |
| TM-2 | Auto-pause | P2 | V-TM-02, V-SIFT-07 |
| TM-3 | Scrub and drop-in | P2 | V-TM-03, V-TM-04, V-PERF-05 |
| TM-4 | Causal inspector | P2 | V-TM-06, V-TM-11, V-TM-12 |
| TM-5 | Map | P2 | V-TM-07 |
| TM-6 | Rating tool | P2 | V-TM-08, V-HUM-01 |
| TM-7 | Person inspection | P2 | V-TM-09, V-DATA-11 |
| ORA-1 | Embodied Oracle | P4 | V-ORA-01 |
| ORA-2 | Whisper Counsel | P4 | V-ORA-02, V-ORA-03, V-LLM-02 |
| ORA-3 | Pierce the Veil | P4 | V-ORA-04 |
| ORA-4 | Plant Omen | P4 | V-ORA-05 |
| ORA-5 | Foresight | P4 | V-FOR-01–V-FOR-04, V-FOR-06, V-PERF-06 |
| ORA-6 | Oracle journaling | P4 | V-ORA-12 |
| ORA-7 | Oracle-3D. **Withdrawn (ADR-0035)** | — | None (V-ORA-13 withdrawn with it; the 2D travel map is WP P7.1) |
| ORA-8 | Forbidden Ward | P4 | V-ORA-06 |
| ORA-9 | Oracle knowledge and console modes | P4 | V-ORA-09, V-ORA-10, V-ORA-11, V-FOR-05, V-TM-11 |
| NAR-1 | Forms | P1a/P2/P3 | V-NAR-07, V-NAR-04 |
| NAR-2 | Pipeline | P3 | V-NAR-06 |
| NAR-3 | Canon tiers | P3 | V-NAR-01, V-NAR-02, V-NAR-08 |
| NAR-4 | Style cards | P3 | V-NAR-09, V-NAR-03 |
| NAR-5 | Sentence provenance | P3 | V-NAR-05 |
| R3D-1 | Separate renderer process. **Withdrawn (ADR-0035)** | — | None (V-IF-02 now traces to SIM-11, V-IF-05 to PERF-5) |
| R3D-2 | Street-from-seed content | P5 | V-R3D-01, V-R3D-04 |
| R3D-3 | Crowd LoD and Spectacle. **Withdrawn (ADR-0035)** | — | None (V-R3D-02, V-R3D-05 withdrawn; V-HYD-05 stays with DET-6) |
| R3D-4 | Hydration rings | Later | V-HYD-07 |
| R3D-5 | Wallpaper power policy. **Withdrawn (ADR-0035)** | — | None (V-R3D-03 withdrawn; V-AUD-02 now traces to R3D-10) |
| R3D-6 | Art bible. **Withdrawn (ADR-0035)** | — | None (V-R3D-09 withdrawn) |
| R3D-7 | Camera director | P5 | V-R3D-06 |
| R3D-8 | Replay mode | P5 | V-R3D-07 |
| R3D-9 | Investigation templates | Later | V-R3D-08 |
| R3D-10 | Lo-fi glyph client first | P5 | V-R3D-10, V-R3D-11, V-R3D-12, V-DET-07, V-AUD-02 |
| AUD-1 | Systemic ambience | P5 | V-AUD-01 |
| AUD-2 | Licence-cleared TTS, designed voices | Later | V-AUD-06 |
| AUD-3 | Audio-drama rendering | P2/P3 | V-AUD-03, V-AUD-04, V-AUD-07, V-SAGA-07 |
| AUD-4 | Voice latency | Later | V-AUD-05 |
| SAGA-1 | Artifacts | P1b | V-SAGA-02, V-SAGA-03, V-SAGA-10, V-SAGA-13 |
| SAGA-2 | Vows, grudges and quests | P1b | V-SAGA-01, V-SAGA-09, V-SAGA-10, V-SAGA-13 |
| SAGA-3 | Mythic catalysts | P1b | V-SAGA-04, V-SCALE-05 |
| SAGA-4 | Secrets v0 and the information horizon | P1b | V-SAGA-11, V-SAGA-18 |
| SAGA-5 | Audio chronicle (offline exporter) | P2/P3 | V-SAGA-07, V-AUD-03, V-AUD-07 |
| SAGA-6 | Graphic chronicle | P3 | V-SAGA-08, V-NAR-05 |
| SAGA-7 | The local saga reader | P2/P3 | V-SAGA-12, V-HUM-13 |
| SAGA-8 | Saga pattern packs | P2 | V-SAGA-05, V-SIFT-01 |
| SAGA-9 | Visual DNA | P3 | V-SAGA-14 |
| SAGA-10 | Cinematic event envelope | P1a | V-SAGA-15, V-DATA-08 |
| SAGA-11 | Stress and mental breaks | P1a | V-SAGA-16, V-SAGA-16b |
| SAGA-12 | No spontaneous props | P1b | V-SAGA-17 |
| SAGA-13 | Immediate succession | P1a | V-SAGA-19, V-INV-03 |
| SAGA-14 | Fugitives | P1b | V-SAGA-20 |
| SAGA-15 | Provenance of change | P1b | V-SAGA-21 |
| PERF-1 | MOP-1 run time | P1b/Later | V-PERF-01 |
| PERF-2 | Macro-step budget | Later | V-PERF-02 |
| PERF-3 | Sifter overhead | P2/Later | V-PERF-03, V-SIFT-10 |
| PERF-4 | Benchmark regression gate | P0 | V-PERF-04, V-PERF-11 |
| PERF-5 | Harness isolation | P2 | V-PERF-09, V-IF-05 |
| PERF-6 | High-fidelity daily life | P1a/Later | V-PERF-10 |
| STR-1 | World Stream, deterministic | P1.0/P1a | V-STR-01 |
| STR-2 | Stream fidelity, no hidden state | P1a | V-STR-02 |
| STR-3 | Viewing is a buffer event | P1a | V-STR-03, V-STR-05, V-PERF-09 |
| STR-4 | Interaction is a tip event | P2 | V-STR-04, V-DET-13 |
| STR-5 | Pace is the director's setting | P2 | V-STR-03, V-TM-03 |
| STR-6 | Virtual telemetry | P1a | V-STR-06, V-STR-07 |
| STR-7 | Activity resolution is a world setting | P4 | V-STR-08, V-DET-06 |
| EVO-1 | Ownership and change scope | P0 | V-INV-03, V-INV-04, V-INSP-03, V-GOLD-03, V-WC-10 |
| EVO-2 | Crate deliverables | P1a | V-INV-06, V-INV-07 |
| EVO-3 | Fidelity levels | P1b/P4 | V-HYD-10, V-HYD-11, V-SF-01–V-SF-12 |
| EVO-4 | The engine only engines | P1a | V-INSP-05, V-INSP-03 |
| OBS-1 | Observability | P1a | V-INSP-04 |
| SEC-1 | Bounded LLM worst case, leverage bands | P4 | V-ORA-07, V-ORA-08, V-LLM-03 |
| LIC-1 | Licence allow-list | P0 | V-INSP-01, V-AUD-06 |

### 6.2 Stakeholder needs (validated, not verified)

| Need | Validated by |
|---|---|
| SN-1 | MOE-1 (V-HUM-01, V-HUM-09); preference model (V-HUM-10) |
| SN-2 | V-TM-06; G2 items are rated with the causal inspector open (§3.4.1) |
| SN-3 | MOE-1, MOE-2, MOE-6 (V-HUM-01–V-HUM-03); arc-signature diversity (§3.4.1) |
| SN-4 | MOE-4 (V-GOLD-04); V-DET-04 |
| SN-5 | MOE-3 (V-HUM-04, V-DATA-11) |
| SN-6 | V-HUM-07; V-ORA-07 |
| SN-7 | V-HUM-05; MOP-6 (V-NAR-08); V-HUM-13 |
| SN-8 | V-HUM-06 (the diorama); V-HUM-04 and V-HUM-08 only if street-level is taken up |
| SN-9 | V-WC-01, V-WC-02 (OS-10) |
| SN-10 | MOE-5: work-package cycle times (`13`) |
| SN-11 | V-PERF-06; V-FOR-06 |
| SN-12 | V-HUM-13 (a season listened to and read); V-SAGA-12; MOE-1 on sagas (V-HUM-01) |
