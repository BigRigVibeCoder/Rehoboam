# 10 — Risk Register

*Status: Draft v1.0 (2026-10-15). Scale: Likelihood (L) and Impact (I) are each rated 1–5, and Score = L × I. Review the register at every phase gate. Each risk carries an **early-warning indicator** (EWI) so it can surface before it bites.*

---

| ID | Risk | L | I | Score | Mitigation | Early-warning indicator | Phase |
|---|---|---|---|---|---|---|---|
| **R-01** | **Emergent history is boring.** The simulation runs but produces logs, not stories (the emergent-narrative problem). | 4 | 5 | **20** | Build the sifter, rating tool and couplings *with* the first simulation. Contingent unlocking in the Registry. Knowledge asymmetry (irony). Bad-run rate as a metric. Gate P2 on MOE-1 before any 3D work. | MOE-1 below 3/century after two iteration cycles. Bad-run rate above 20%. | P1–P2 |
| **R-02** | **Scope creep across five modalities**, so nothing reaches depth. | 4 | 5 | **20** | Sequenced modalities with phase gates (`01` §6). Explicit "not now" list. Every new idea goes into the backlog, not the current phase. | Work packages opened outside the current phase. Phase durations slipping more than 50%. | All |
| **R-03** | **Determinism leaks** (float, iteration order, dependency upgrades) break replay, branching and Foresight. | 4 | 4 | 16 | Determinism contract (ADR-0006). Vendored samplers. Golden runs on every PR. Thread-count tests. Unsynced guard. | Any hash mismatch in CI. Nightly cross-platform drift. | P0+ |
| **R-04** | **Agent-built code degrades architecture over time**: cross-cutting hacks, test erosion. | 4 | 4 | 16 | Spec-first work packages. Crate boundaries enforced (`cargo-deny` bans plus a dependency-graph lint). ADR changes need your approval. Coverage of invariants and property tests. Periodic refactor work packages. | Rising PR size. Crates gaining unexpected dependencies. Flaky tests. | All |
| **R-05** | **Macro model mis-calibrated**: runaway growth, extinction or static history. | 3 | 4 | 12 | Density-dependent demography (binomial draws). SDT overshoot. Stylized-fact harness on 32 seeds. Sensitivity screening. Burn-in gate in the compiler. | Bands failing. Spectral peak missing. | P1 |
| **R-06** | **Sifter does not scale** to millions of events and dozens of patterns. | 3 | 4 | 12 | Trigger indexing, pruning and partial-match expiry. Benchmarks. Retrospective SQL for heavy patterns. dbsp/differential-dataflow as an escalation path. | Sifter more than 10% of run time. Partial-match memory growth. | P2 |
| **R-07** | **Small local LLMs leak knowledge or break character** in dialogue. | 4 | 3 | 12 | Plan-then-realise. Belief-scoped context. Claim check. Template fallback. RP-200 gate. Option to use larger or cluster models (compute is unbounded). | RP-200 leak rate above 0 on concealment items. | P5–P6 |
| **R-08** | **Prompt injection or gaming of persuasion.** | 3 | 3 | 9 | Parse, don't inject. Deterministic scoring. Caps. Penalties in-world. Red-team corpus in CI. | Red-team regressions. Players reaching the cap trivially. | P3 |
| **R-09** | **3D crowd performance in Godot** misses its targets. | 3 | 4 | 12 | VAT and impostors are mandatory. Spike 1 runs before P5 commits. Hot paths in gdext. Fallback: fewer embodied characters, more impostors. | Spike 1 below 60 FPS on reference hardware. | P5 |
| **R-10** | **Street-level content disagrees with the simulation**: rivers, roads, wealth, re-rolled towns. | 4 | 3 | 12 | A shared `rh-worldgen` crate. Rivers carved from the macro graph. Incremental growth. Versioned generators. Overrides for events. Spike 2 runs before P5. | Spike 2 stability below 90%. User-noted "fake" moments. | P5 |
| **R-11** | **Belief and memory state explodes** memory or keyframe size at 1M scale. | 3 | 3 | 9 | Beliefs only for notable propositions and named actors. Crowd beliefs as pop shares. Semantic roll-ups. Measure in P3. | Keyframe above 0.5 GB at 1M. | P3 |
| **R-12** | **Dependency churn** (Bevy, gdext, DuckDB 2.0, Polars 2.0, model APIs). | 4 | 2 | 8 | Pin versions. Facades. Parquet/Arrow as the system of record. Upgrade work packages, each gated by golden runs. | Upstream breaking-change notes. | All |
| **R-13** | **Licence contamination** from GPL/AGPL/NC code, weights or assets. | 2 | 4 | 8 | ADR-0021 allow-list. cargo-deny and pip-licenses in CI. Provenance log. Reimplement rather than copy. | CI licence failure. Unknown-licence asset. | All |
| **R-14** | **LLM provider volatility**: deprecations, price changes, refusals on grim content. | 3 | 2 | 6 | Provider-agnostic gateway. Cross-family fallbacks. Local and cluster paths. Refusals logged and rerouted. | Rising refusal rate. Deprecation notices. | P2+ |
| **R-15** | **Narrative prose quality disappoints** ("chronicle-itis", slop, sameness). | 4 | 3 | 12 | Story-shape gate. Best-of-N plans. Style cards. Slop and repetition gates. In-world documents. Saga form as the default. Honest expectations (R08). | Your blind ratings below the ungrounded baseline on engagement. | P4 |
| **R-16** | **Foresight is too slow or miscalibrated.** | 2 | 3 | 6 | Single-threaded rollouts on many cores. Reduced fidelity outside the predicate's scope. Common random numbers. Reliability diagram. Surrogate emulators later. | p95 rollout batch above 2 min at 100k. Calibration off by more than 0.05. | P3 |
| **R-17** | **Wallpaper drains power**, or a Windows update breaks hosting. | 3 | 2 | 6 | Self-throttle. Stop audio streams when paused. Replay mode. Windowed fallback. Lively package. | Battery or sleep complaints. A Windows update breaks WorkerW. | P5 |
| **R-18** | **Hydration inconsistencies** (popping, contradictions, chain disaggregation). | 3 | 3 | 9 | Ledger rows, keyed traits, leases and decrees, pins, stub depth caps, hysteresis, tests in `09` §2.5. | Interrogation-session contradictions. | P5–P6 |
| **R-19** | **Calibrating the fantastic** (dragons, hive, magic) has no ground truth. | 3 | 2 | 6 | Bound by energetics. Burn-in liveness checks. Treat catalysts as probabilities. | Degenerate dragon or hive outcomes across seeds. | P3 |
| **R-20** | **Disk growth** from keyframes and chronicles across many worlds and branches. | 3 | 2 | 6 | Copy-on-write sharing for forks. Thinning cold keyframes. Regeneration from the journal. | Above 100 GB per world. | P2+ |
| **R-21** | **Sensitive emergent content** (pogroms, purges, witch-hunts) is rendered gratuitously in prose. | 2 | 2 | 4 | Narration framing as history. Style-card restraint. Refusal handling. Your content settings. | Refusal spikes. Discomfort in ratings. | P4 |
| **R-22** | **Name and brand collision** ("Rehoboam" and Westworld), relevant only if published. | 2 | 2 | 4 | Engine and Oracle codename. Trademark clearance before any release. | Decision to publish. | Later |

## Top-5 focus (current)
1. **R-01**: prove story yield in P2 before investing in 3D.
2. **R-02**: hold the sequencing line.
3. **R-03**: build the determinism harness first, in P0.
4. **R-04**: agent hygiene: specs, contracts, gates.
5. **R-05 / R-06**: calibration and sifter scale, both measured early.
