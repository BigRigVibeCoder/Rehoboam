# RT-2026-10-16 — External red-team review: dispositions

*Status: Closed (2026-10-16). Reviewer: an external red-team agent from a different model family, run by the director. Disposition by the Architect (AGT-001); folding of the adopted items approved by the director on 2026-10-16. The review itself is the director's copy and is not reproduced here; this record answers it point by point.*

## Summary

The review raised four "fault lines": coupled-system instability, the 1-to-1M scale range, shallow sifted drama, and code decay in AI-written Rust.

- **Already covered (8 claims).** About two-thirds of it restates risks the plan already tracks, with mitigations already specified: R-01 and R-05, ADR-0028 scale profiles, ADR-0031 encounters, `05.4` significance and suspense, and the G2 diversity guard.
- **Adopted (5 items).** They sharpen the plan and are now folded into the specs, the tests and two sprints.
- **Declined (4 items).** Each would make the engine worse in a way the review itself warns against.
- **Corrected (5 factual points).**

| Disposition | Count |
|---|---|
| Already covered | 8 |
| Adopted | 5 |
| Declined | 4 |
| Corrected (factual) | 5 |

## 1. Already covered

| # | Claim | Where the plan already handles it |
|---|---|---|
| C-1 | Coupled systems will run away to extinction or sink into stagnation | R-05 (runaway, extinction or static history) and R-01 (history is boring, score 20, the top risk). Defences: density-dependent demography; the liveness gate (200 y × 8 seeds); 32-seed stylized-fact bands; and the G2 thesis test against a coupling-ablated control (`09` §3.4.1). Extinction is a valid, journaled end state (ADR-0028 §1) |
| C-2 | Calibration of 50+ parameters is hopeless by grid search | No grid search is planned. The plan uses pattern-oriented modelling with several patterns at once (Grimm et al. 2005; `09` §3.1); Morris and Sobol screening (V-SF-13); Latin-hypercube filtering within Registry bounds; Bayesian optimisation or ABC for repair (`05.5` §4.2); submodel tournaments (`09` §3.8); and held-out bands |
| C-3 | One engine cannot serve both a household and an empire | ADR-0028 §2 scale profiles. In micro worlds (≤ 1k) **everyone** is a named mind, the economy is household holdings, and the default pattern packs are personal (`trial_of_faith`, `betrayal_by_friends`). One engine, configured by profile |
| C-4 | Virtual telemetry is a non-interacting "ghost town" | ADR-0031 resolves encounters in a second phase, after positions are fixed. At `blocks` and `minutes` resolution, consequential encounters (ties, infections, crimes) become authoritative events. The prototype does not compute position as a hash of person and time |
| C-5 | The sifter will extract thousands of identical stories | Rarity is scored conditioned on actor traits (`05.4` §3). G2's diversity guard fails if one pattern family exceeds 40% of highly rated items (`09` §3.4.1) |
| C-6 | Raw causal centrality ranks a trivial cause above a personal tragedy | `05.4` §3: a tellability prior per act class, plus consequence as PageRank on the reversed causal DAG, taken as a percentile within act class and era. This is the review's own proposed fix |
| C-7 | Suspense needs O'Neill & Riedl's goal-threat and escape-route model | Built in batch 5: `05.4` §2.3, item 8 (Dramatis, O'Neill & Riedl 2014; R12 §20), with `threatened_role`, escape routes and suspense = stakes × the probability of completion |
| C-8 | State must be Structure-of-Arrays, not per-entity heap structs | The design already stores state as columns, each owned by one system (`05.1` §2; ADR-0002) |

## 2. Adopted (folded in on 2026-10-16)

| # | Item | What changed |
|---|---|---|
| A-1 | **Test each system alone under synthetic forcing, then ramp the couplings** | New V-SF-22 (isolation harness: a gate before a system's in-couplings are switched on) and V-SF-23 (coupling-strength sweep α ∈ {0, 0.25, 0.5, 0.75, 1}; α = 0 is the G2 control), both in `09`. `lint_odd.py` requires an `[isolation]` section with a forcing for every rate or mode read. GOV-002 tier 13; R-05; the G1b gate |
| A-2 | **Every cross-system influence declares its form and bounds** (the review proposed Hill functions everywhere; adopted in a different form, see D-1) | ODD-R reads declare `use` (identity, mode or rate). A rate read needs an in-coupling with `form`, `neutral`, `output_bounds`, `data_range`, `beyond_range` and `source`. Warnings on draft specs, errors on frozen ones. ADR-0033 amended; template and the S5 and M2 specs updated |
| A-3 | **No heap allocation in the tick hot path** (adopted as a measured budget, not an absolute) | New V-PERF-11; SPR-009 T-05: a counting allocator in bench builds, a per-bench budget (default 0 for P0 machinery), gated like instruction counts; buffer growth on capacity doubling counted separately |
| A-4 | **Statistical RNG stream independence** | New V-DET-20; SPR-002 T-06: correlation, byte chi-square and runs tests across distinct keys and sibling salts, on top of the known-answer vectors |
| A-5 | **Micro-world versions of M3 and M4**, and M4's behaviour beyond its data | BCK-001 §3: at the micro profile, the M3 harvest is the household's own fields and M4 food is its own stores and holdings. M4's price rule declares its `data_range` (Davenant's schedule covers shortfalls up to about 50%) and a sourced `beyond_range` with bounded output |

## 3. Declined

| # | Proposal | Why not |
|---|---|---|
| D-1 | Hill-function saturation on **every** cross-system interaction | Applied everywhere, it damps the model toward the over-stable attractor the review warns about, and it replaces evidence-based shapes (Davenant, Brass, SDT) with an arbitrary one. A-2 keeps the goal (bounded, declared influence) without the cost |
| D-2 | Rate limiters on slow variables (singular-perturbation separation) | Real history has abrupt collapses: the Black Death removed 30–50% within two years, and V-SF-03 and V-SF-11c require shocks of that size. Bands and the isolation harness judge stability; limiters would hide it. The engine is already multi-rate (pulses, phases and timers; `05.1` §4) |
| D-3 | A hard dual-mode switch at N ≤ 50, with separate code paths | It doubles the code under test and breaks "one engine". ADR-0028's profiles are configuration defaults, and every rung of the complexity ladder runs the same tests |
| D-4 | Information asymmetry as a **mandatory** gate on every sifter pattern | Sieges, famines and succession crises are drama without dramatic irony. Epistemic patterns (tragic irony, betrayal, the covert mimic) are welcome as a pattern family that G2 measures, not as a filter on all of them |

## 4. Corrections to the review

1. **It audited a stray branch.** `claude/project-thread-2kxrgv` at `45283fb` came from a deleted project thread, and its "SPR-010 approved" note was not the director's ruling. The director approved SPR-010 later the same day; that approval is recorded in `decisions-log.md`.
2. **"Every seed goes extinct within 25 years"** is an assertion, not a measurement: no coupled system exists yet. The liveness gate, the ladder and V-SF-23 exist to measure it.
3. **`bin/lint_forbidden.py` is a token-level scanner, not an AST linter.** It has positive controls and is backed by clippy's disallowed types and methods.
4. **SPR-003 is the samplers, not the data layout.** Columns are `rh-core` and system-sprint work (SPR-005 onward, P1).
5. **The Davenant price rule is a fitted schedule over its data range,** not an unbounded power law in the specs. Its behaviour beyond that range was unspecified, which A-5 fixes.

## 5. Follow-ups

- The first frozen system specs (P1a: M2 with entitlement, M3, M4, S3) must pass the coupling rules as errors, not warnings.
- V-SF-23 is first reported at G1b. If an α between 0 and 1 shows a sharp transition into extinction or flatline, it becomes a named risk with its own mitigation.
