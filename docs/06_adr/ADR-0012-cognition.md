# ADR-0012 — Generic psych core + overlays; event-driven IAUS + HTN

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15) \
**Amended by ADR-0034 (2026-10-16, D27):** deliberation consults and pull inputs retired.

## Context

The draft's PsychState mixed timescales, and its vocabulary only made sense in one setting. GOAP at 10–30 Hz for 5k actors costs an estimated 7–75 core-seconds per simulated second.

## Decision

- **Core state.** HEXACO traits, Schwartz values, PAD mood plus up to 8 decaying OCC emotions, needs, opinion-modifier lists, RimWorld-style Thoughts, a belief store, and ACT-R memory activation.
- **Overlays.** Setting-specific stats (Paranoia, Filial Piety and so on) are derived overlays declared in the Registry.
- **Decisions.** Goals come from event-driven IAUS scoring; plans come from bounded HTN; places use Sims-style advertisements.
- **Cadence.** CK3-style polling plus interrupts.
- **Explainability.** The top contributors and margin of each decision are logged as `motivated_by` edges.

## Alternatives considered

| Option | Why not |
|---|---|
| GOAP everywhere | Too costly. |
| LLM agents (Concordia, Smallville) | Cost is about 2×10¹¹ tokens over 1,000 years, and they are non-deterministic. |
| A fixed five-vector psych model | Not generic across settings. |

## Consequences

- Calibration needs sensitivity sweeps.
- Light minds for unnamed persons are seeded from pop distributions.

## Revisit when

- Model-chosen actions may return only as a research experiment after G3b, outside the roadmap, through a new ADR (ADR-0034).

## Evidence

R04 §1–2, §4.1; R03 §6.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C1, RT-M1, RT-M2). Sources of truth: ADR-0024, ADR-0025, 05.1 §4.3, 05.3 §1, §3, §8.*

**Corrected inline:**
- "pivotal-consult" → **deliberation consult** (Revisit when). Since retired by ADR-0034.

**Changes:**
1. **Deliberation consults: retired by ADR-0034 (D27).** Named actors decide only through IAUS and HTN. No LLM chooses an action, and no decision waits on a model.
2. **Same-instant semantics: simultaneous deliberation, ordered action** (RT-M1).
   - All due decisions are scored in parallel against a snapshot taken at the start of the `Agenda` phase.
   - **Response curves are fixed-point lookup tables**, so threshold comparisons are platform-independent.
   - Actions apply in a keyed permutation order. An action whose precondition fails after an earlier write re-scores once, then defers to the next poll.
3. **Interrupts** schedule re-evaluation at the next instant at the earliest (t + 1 day by default), with a per-actor daily cap.
4. **Selection** is top-k weighted with keyed RNG. The `motivated_by` edges record the top contributors and the **margin over the runner-up**.
5. **T1 membership** comes only from simulation criteria or a journaled input. Being observed or pinned never promotes anyone (ADR-0024). Great-house members far from any title, claim or office become **dormant** (SIM-9).
6. **Light minds are pure functions of plane A**: `H`, pop belief shares and the life-course log. Whether a person holds a belief is a keyed Bernoulli draw, stable across visits. What a conversation showed you goes to the plane-B facts ledger. Promotion needs a journaled material input, for example you hire or accuse the person, or they witness a material event you caused.
7. **Speech.** The simulation decides the `SpeechPlan` (IAUS over speech acts). The LLM only realises the prose. Bubble NPCs never originate material events (ADR-0024 rule 6).
8. **Hive scale.** Hive members are a ledger flag plus squad aggregates. Only office-holding or investigated nodes are T1.
