# ADR-0025 — Scheduled outcomes for everyone; material-input semantics; no back-dating

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-15 · **Supersedes:** the lease and lookahead-decree mechanism in 05.2 §3.2 (v1.0)

## Context

Baseline v1.0 gave hydrated persons special "decrees", e.g. "dies at d14 h09", while headless runs applied deaths at the month pulse. That had three problems (RT-C2):

- **Divergence or back-dating.** Either the event tick differed between watched and unwatched runs, or headless events had to be back-dated.
- **Twin disagreement.** `EventId = (day << 24) | seq` would then disagree with emission order, so live-vs-SQL twin equivalence broke.
- **Undefined conflicts.** There was no rule for conflicts between decrees and player actions, and none for acting in the past after a drop-in.

## Decision

1. **Scheduled outcomes for everyone.** At each pulse, systems sample the **intra-pulse times** of state-changing outcomes for every affected person, watched or not. These outcomes are births, deaths, illness onset, migrations, marriages and similar. Each one is inserted as a **timer** in the authoritative agenda and fires at its own tick in every mode. The 3D bubble only *displays* outcomes. This costs about 10⁴ timers per month at 1M persons (deaths and births alone are ≈ 6–7k; `08`).
2. **No back-dating.** No event may be emitted with a tick earlier than the current instant. `EventId` order = emission order = `(tick, phase, seq)`.
3. **Material inputs declare a dependency scope** (person, household, settlement or region).
   - In the `Inputs` phase, pending timers inside the scope are **cancelled and re-sampled**.
   - The re-sample key is `H(seed, branch_salt, purpose, entity, t_input, epoch)`, where `epoch` counts the material inputs that have touched that entity.
4. **Precedence.** The earlier tick wins. At equal ticks, `Inputs`-phase effects apply before system phases.
5. **Acting in the past forks.** A material input at a tick before the head of the timeline creates a **fork** at that tick. Passive drop-in (seek, inspect, watch) is read-only.
6. **No leases.** Hydrated persons' positions and activities are *views*: schedule templates plus the scheduled outcomes. Authority never moves to the bubble.

## Consequences

- Observer invariance holds by construction.
- The sifter and SQL twins see one consistent ordering.
- Material player actions have defined, reproducible consequences.
- The agenda carries timers as well as decisions. Timers are cheap: a heap entry plus a closure ID.

## Evidence

RT-C2; R01 §R5 (lease/decree idea, now replaced); R10 §R4 (journal/replay).
