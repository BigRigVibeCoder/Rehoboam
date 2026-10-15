# ADR-0015 — Foresight = Monte-Carlo forked rollouts with common random numbers

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

The user has unbounded compute and wants the engine to scale with it. The fictional Rehoboam predicts futures. Determinism makes both forking and seeding new futures cheap.

## Decision

1. Foresight forks the canonical timeline at the current tick. Each rollout extends the fork's **salt lineage** with a unique step: `branch_salt(t) = H(lineage path covering t)`.
2. N rollouts run on stateless workers that scale horizontally.
3. The output is P ± a Wilson confidence interval for a player-defined prophecy predicate, together with clusters of the paths that fail it.
4. Interventions are evaluated with **common random numbers**: the same salts are run with and without the intervention, and the results are compared pairwise.
5. Validity is enforced by nightly tests on the cluster: exchangeability (PIT), reduced-cost bias, and proper scores. The reliability diagram is a display, not a CI gate.
6. The canonical timeline is never altered.

## Alternatives considered

| Option | Why not |
|---|---|
| Heuristic or analytic forecasts | Not grounded in the simulation. |
| Independent ensembles for intervention A/B | Much higher variance. |

## Consequences

- This is the main horizontal-scaling workload.
- It is also a core game mechanic, paid for in Foresight.
- Rollouts can run at reduced cost where the predicate cannot be affected, through cadence and richness, never by switching representation.

## Revisit when

- Rollout cost limits interactivity. Then add surrogate models: learned emulators trained on rollouts.

## Evidence

R11 §H; R03 §4 (common random numbers implied by keyed RNG); R10 §R4.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C3, RT-M4, RT-M5, RT-M13). Sources of truth: ADR-0026, 05.1 §3, §6.9, 05.3 §7, 05.9 IF-1.*

**Corrected inline:**
- "`branch_salt = k`" → lineage salts. A plain `k` collides across Foresight jobs and nested forks.
- "reliability diagram in CI" → validity tests. Per-decile reliability with about 20 prophecies per bucket cannot pass even for a perfect forecaster.
- "reduced fidelity" → reduced cost through cadence and richness. Fidelity switches happen only through a migration keyframe, never mid-run in a rollout.

**Changes:**
1. **Lineage salts.** The canonical path is empty. Every Foresight job and every rollout extends the path uniquely, so salts never collide (`SaltLineage`, IF-1).
2. **`resample_on_fork`.** Pre-sampled timers and polls whose purpose carries this Registry flag (vital timers, latent periods) are re-drawn after the fork. Without this, rollouts would share pre-drawn outcomes and be under-dispersed.
3. **What-if A/B keeps the parent salt** (common random numbers). An intervention runs on the same salts as its baseline, and ΔP is computed pairwise. "Reroll fate" extends the salt instead.
4. **CRN pairing survives across branches** because `PersonId`s are genealogical (ADR-0026). Test: per-person death times in a pair correlate ≥ 0.9 at 10 years for unaffected lineages.
5. **Oracle-mode rollouts fork from a belief-resampled state.** Hidden attributes the Oracle does not know (secret plots, hive membership) are re-drawn per rollout from the Oracle's posterior, keyed by salt. Foresight never leaks hidden truth. **Cultivator** mode forks from the true state.
6. **Validity tests** (nightly, on the cluster):
   - **(a) Exchangeability.** A rank histogram or PIT of the canonical continuation among the rollouts. It must be uniform (χ² test).
   - **(b) Reduced-cost bias.** CRN-paired comparison of reduced-cost against full-cost rollouts: |ΔP| ≤ 0.03.
   - **(c) Scoring.** Brier and log score with bootstrap CIs, plus a Spiegelhalter calibration test (Z). In Oracle mode these also measure the information lost to belief resampling.
   - The **reliability diagram** stays on the Foresight screen as a display.
7. **Scale.** Default N = 256, truncated at the predicate's horizon. At 1M persons Foresight is a batch job (minutes on a cluster node). Interactive Foresight targets 100k-class worlds.
8. **Deliberation consults.** Rollouts use the declared fallback policy and are labelled as such (ADR-0011 v1.1).
