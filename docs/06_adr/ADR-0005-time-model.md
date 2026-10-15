# ADR-0005 — Integer sim-time, pulses, event-driven agenda, speed bands

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

The draft's speed multipliers were off by about five orders of magnitude. At 10,000×, 1,000 years takes 36.5 days. Its agents ran at 10–30 Hz even in fast-forward, and a single `TimeScale` was set per world.

## Decision

- `SimTime = i64` minutes from the world epoch, with a calendar defined in the manifest. Negative values are allowed (ADR-0026).
- Pulses run at Year, Season, Month, Day and Hour; inside the 3D bubble there is also a presentation-only real-time pulse.
- Systems bind to pulses. Adaptive daily stepping happens only for active epidemics, campaigns and sieges.
- Named actors run on a discrete-event agenda keyed by `(time, phase, entity_id, seq)`, with polling intervals in the style of CK3 plus interrupts.
- A fixed phase order within each instant (05.1 §4.2).
- Controls are bands of *simulated time per wall-clock second*, plus **MAX** and **run-until**.
- **Speed never changes the simulation.**

## Alternatives considered

| Option | Why not |
|---|---|
| A fixed global tick | Wastes compute on idle systems and can't reach millennial speed. |
| Float time | Ordering and determinism hazards. |

## Consequences

- Decisions finer than a month exist only for named actors and active daily systems, at every speed. Scheduled outcomes still carry intra-pulse ticks for everyone (ADR-0025). The sifter must accept monthly decision granularity.
- The bubble at Live speed is a plane-B view. It displays scheduled outcomes and never takes authority from the simulation (ADR-0025).

## Revisit when

- Profiling shows pulse overhead dominates. Unlikely.

## Evidence

R03 §5; R01 §R5.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C2, RT-M1). Sources of truth: ADR-0024, ADR-0025, ADR-0026, 05.1 §4.*

**Corrected inline:**
- "`SimTime = u64`" → `i64` minutes from the world epoch. Founders need pre-epoch birth times, which an unsigned type cannot hold.
- "exchanges decrees with the macro layer" → the bubble is a view. The lease and decree reconciliation of R01 §R5 is superseded by scheduled outcomes (ADR-0025).
- "At MAX speed, drama finer than a month …" implied that granularity depends on speed. It does not, and the sentence now says so.
- The bubble's real-time pulse is marked presentation-only.

**Changes:**
1. **Signed time.** Negative `SimTime` values are pre-epoch. There is no millennial overflow. The ledger stores `birth_day` as `i32`.
2. **Calendar.** 12 months. `days_per_year` is 360 or 365. Seasons are 2 or 4.
3. **Scheduled outcomes (ADR-0025).** At each pulse, systems sample the intra-pulse times of births, deaths, illness onset, migrations and marriages for everyone, watched or not. Each becomes a **timer** in the agenda and fires at its own tick in every mode. The agenda holds **decisions and timers**, keyed by `(time, phase, entity_id, seq)`.
4. **No back-dating.** No event is emitted with a tick earlier than the current instant. `EventId` order = emission order = `(tick, phase, seq)`.
5. **Phase order** (05.1 §4.2). `Inputs` comes first: journaled push inputs and due pull inputs apply, and timers in each input's dependency scope are cancelled and re-sampled. `Vital` fires due timers. `Agenda` runs named-actor decisions. `Bookkeeping` takes the plane hashes.
6. **Agenda semantics (RT-M1).**
   - An interrupt schedules re-evaluation at the **next instant at the earliest** (t + 1 day by default), with a per-actor daily cap.
   - **Simultaneous deliberation, ordered action.** Due decisions are scored in parallel against a snapshot taken at the start of the `Agenda` phase. Actions apply in a keyed permutation order `H(seed, salt, t, entity_id)`. An invalidated action re-scores once, then defers to the next poll.
7. **Bubble only at Live.** T3/T4 hydration exists only at Live speed (street clock 20–60:1). Faster bands collapse the bubble. Speed bands change only the publish cadence and how far `advance_until` goes per wall interval.
8. **Run-until** stops on a date, a predicate over aggregates, or `next_match(salience ≥ θ)`.
9. **Speed and deliberation consults.** Speed never changes the simulation *for a given journal*. With consults enabled, a missing answer **stalls** the run at t + D. On timeout the fallback applies and is journaled, so replay is exact. MAX-speed runs and Foresight rollouts use the declared fallback policy and are labelled (05.3 §8).
