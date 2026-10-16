# ADR-0008 — Custom incremental sifter + SQL twins; LLM non-authoritative

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15), v1.2 (2026-10-16, prior art: Felt and Winnow read)

## Context

No maintained, embeddable engine combines Datalog syntax, incremental evaluation and Python access. Most story patterns are bounded-window sequence matches with anti-joins.

- Felt (1 commit, 2020) and Winnow (22 commits, 2021) have no licence in any commit; the Winnow paper's "open source" grants nothing.
- Winnow's reference matcher re-queries every partial match on every event: 13 ms per event at a pool of 10 and 912 ms at 1,000, about 0.9 ms per partial (browser JS, a 5-character world; Kreminski et al. 2021, Table 1).
- A Python streaming matcher reached 2.2M events/s and agreed exactly with the equivalent DuckDB SQL (6,073 = 6,073).
- LLM sifting at scale is infeasible and unevaluated.

## Decision

- A declarative **pattern DSL** (TOML surface) over abstract roles and act classes, with anchored attributes (`attr@e`), `unless` clauses, windows and composable match events.
- Patterns compile into **partial-match automata** in the Rust sim host (`rh-sifter`, plane C), with declarative pruning only.
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
- Sifter state is **outside** the authoritative state hash, and match events use a separate ID space. Patterns that feed back into simulation behaviour are promoted into the Registry rule set and become authoritative.

## Revisit when

- Pattern needs outgrow sequence matching.
- A pattern needs causal ancestry deeper than a bounded walk, or disjunction over guards (v1.2).

## Evidence

R02 §1, §8, §R3; R10 §2, §R3; R12 §19 (Felt and Winnow read in full, with their code checked for licences).

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-C1, RT-C4). Sources of truth: ADR-0024, 05.4 §2–3, 05.9 IF-7, `specs/sifting/patterns.example.toml`, `specs/sifting/lint_patterns.py`, `specs/chronicle/selftest.py`.*

**Corrected inline:**
- "inside the Rust simulation, with pruning" → the sifter runs in the sim host as plane C, outside the simulation core. It prunes only declaratively. *(v1.2: the stated reason was wrong. K-inactivity pruning is a window in global event count and is twin-expressible; it is rejected because it makes matches depend on unrelated event volume.)*

**Changes:**
1. **Sifter output is plane C** (ADR-0024): `sift_match`, `match` events, significance and visibility. `narrative_hash` is a pure function of plane A's event stream plus the pattern-library version. Match events use a separate **`MatchId` space**, never interleaved with `EventId`s, and are **never fed back** into the event bus or plane A.
2. **Feedback curation.** A pattern whose matches may change simulation behaviour is promoted into the **Registry rule set**. It then runs inside `rh-core`, is part of `state_hash`, and follows the golden-baseline process.
3. **Match semantics, declared per pattern (RT-C4).** "All combinations" semantics flood on clustered data. In the repo's clustered fixture, `trust_then_betrayal` under `all` produced 79 matches per 1,000 events, 4.5× the 17.5 under the default `latest` + `clear_binding`. Uniform random fixtures hide the difference.
   - `selection = "latest" | "earliest" | "all"`: which open partial a completing event pairs with. Usually `latest`; it must be declared (v1.2: the linter no longer defaults it).
   - `on_complete = "clear_binding" | "consume" | "keep"`: what happens to open partials after a match. Usually `clear_binding`; it must be declared.
   - `dedupe = { key = [roles], bucket_days = N }`: at most one match per binding per time bucket.
   - All are expressible in the SQL twin (`LAG`, `ROW_NUMBER() … QUALIFY`). *(v1.2: twins exist so far only for `all` + `keep`, `latest` + `clear_binding` and `dedupe` on a two-clause pattern.)*
4. **Declarative pruning only:** windows, `unless`, a death-tolerance flag, and the semantics above. There is no "drop after K inactive events" heuristic.
5. **Per-pattern firing budget.** Each pattern declares an expected firing rate per century per 100k persons. CI fails a pattern that exceeds 3× its budget on the reference ensembles.
6. **Precision@K on clustered fixtures.** Twin and budget tests run on clustered chronicles (repeated interaction among partners), not uniform random ones. CI reports precision@K against your labelled story-worthy set. An LLM-proposed pattern must report precision@K against its SQL twin before review.
7. **Typed anchored attributes.** Reads are `attr(args)@eN`, declared and typed in the Registry `[attributes]`. A bare or undeclared read is a compile error. The pattern linter enforces a closed key set.
8. **Re-sifting history is a feature.** Any timeline can be re-sifted with any pattern-library version without touching plane A. Pattern iteration never churns golden baselines.
9. **Auto-pause.** Suspense auto-pauses are rate-limited (≤ 1 per simulated decade by default, so that all auto-pauses together stay within MOE-1's ≤ 10 per century; `05.4` §2.3). A sifter that lags triggers a pause, then a seek back to the match tick.

## Amendment v1.2 (2026-10-16): prior art read (Felt, Winnow; R12 §19)

1. **Semantics kept.** Winnow's default (fork on every advancing event, keep the parent, no time windows) is our `all` + `keep` without windows. On the repo's clustered fixture it gives 99.3 matches per 1,000 events against 17.5 under `latest` + `clear_binding`, and on the uniform fixture its partial pool grew without a plateau. Its incremental and compiled modes disagree by design, because constraints read end state; our anchored reads remove that divergence. Its syntax embeds raw Datalog rule and builtin calls, which would be free-string semantics (ADR-0010).
2. **Adopted:**
   - kernel order for clauses;
   - `unless` with `bind` and anchored `where`, a default `between` of [first, last], and completion taking precedence over a coincident `unless`;
   - `death_tolerant = [roles]` (the "death-tolerance flag" this ADR named but never specified);
   - `surface_from`, the stage from which a partial is surfaced and can carry suspense;
   - a `killed` match status with the killing event;
   - a content-addressed `MatchId`;
   - per-clause why-not views (after Felt's `whyNot`).
3. **Guarded.** Patterns with more than two clauses must use `all` + `keep` until per-stage semantics are defined and twinned.
4. **Rejected:**
   - K-inactivity pruning;
   - consuming partial matches as events;
   - logging speculative narration into any stream a pattern reads;
   - Winnow's syntax;
   - porting their code (no licence; O(pool) per event).

