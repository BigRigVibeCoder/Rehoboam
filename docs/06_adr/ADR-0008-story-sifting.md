# ADR-0008 — Custom incremental sifter + SQL twins; LLM non-authoritative

**Status:** Proposed · **Date:** 2026-10-15

## Context

No maintained, embeddable engine combines Datalog syntax, incremental evaluation and Python access. Most story patterns are bounded-window sequence matches with anti-joins.

- Felt and Winnow are unlicensed reference designs.
- Winnow's naive incremental matcher cost 912 ms per event.
- A Python streaming matcher reached 2.2M events/s and agreed exactly with the equivalent DuckDB SQL (6,073 = 6,073).
- LLM sifting at scale is infeasible and unevaluated.

## Decision

- A declarative **pattern DSL** (TOML surface) over abstract roles and act classes, with anchored attributes (`attr@e`), `unless` clauses, windows and composable match events.
- Patterns compile into **partial-match automata** inside the Rust simulation, with pruning.
- Every pattern ships with a **DuckDB SQL twin**, and CI checks the two for equivalence.
- LLMs may only re-rank, explain or propose patterns offline, and only for human review.

## Alternatives considered

| Option | Why not |
|---|---|
| Datalog engines (Soufflé, Ascent, crepe) | Batch-only; fine for offline research. |
| Differential-dataflow or dbsp | Kept as an escalation path for recursive or aggregate rules. |
| CozoDB, DDlog | Dead. |
| LLM detection | Non-deterministic, unscalable, unevaluated. |

## Consequences

- We own the sifter, but it is a modest, well-specified component.
- Pattern authoring is designer work and tied to the Registry vocabulary.
- Patterns are versioned per world so replays stay reproducible.

## Revisit when

- Pattern needs outgrow sequence matching.

## Evidence

R02 §1, §8, §R3; R10 §2, §R3.
