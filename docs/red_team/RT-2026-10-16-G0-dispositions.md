# RT-2026-10-16 — Red-team batches 01 to 04: G0 dispositions

*Status: RULED (2026-10-16). Disposition by the Architect (AGT-001); the director ruled on items 9 and 15 the same day (§2, §3; `docs/decisions-log.md`). Input: `docs/red_team/inbox.md` items 1 to 15, the Architect's intake notes N1 to N12, the director's batch 03 rulings (decisions log, 2026-10-16) and the red team's response to batch 04. The red team's prototypes are evidence for a proposal, not measurements of Rehoboam.*

## Summary

Most of the fifteen items restate something the plan already has, in a stricter or more general form. The red team's strongest new material is in three places:
- the vow-collision rule (item 5);
- in-world demotion for the nobility (item 12, rebuilt prototype 7);
- Priority-Flood for river generation (item 11).

Two proposals are declined as stated: the summed −log p saga ranking (item 7) and the global court lock (part of item 1). Both would put a regulator or a rejected method where the plan has a mechanism. Nothing here changes an ADR. The director ruled on the two items that needed it:
- Laws 2 and 3 are adopted, narrowly, as principles 8 and 9 of `docs/01` §5.
- Item 15 goes into GOV-003 §6 as a failure-modes table, with no `CLAUDE.md` change.

| Disposition | Items |
|---|---|
| Already covered (no change; corrections noted) | 4, 13, 14 |
| Adopt (into a named spec, at its sprint) | 5, 11, 12 |
| Adapt (the failure mode is real; the fix changes shape) | 1, 2, 3, 6, 8, 10 |
| Decline as proposed | 7 |
| Director ruled (2026-10-16) | 9 (Laws 2 and 3 adopted narrowly), 15 (GOV-003 table) |

Facts that apply to several items:
- `specs/systems/` holds M2, S5, S6 and S7 only. There is no `M14.court_politics` anywhere; court politics belongs to M7 (dynasty) and the minds spec `T1.minds`, which is unwritten (ADR-0035).
- `S2` is `S2.technology`, not agents.
- Siege is M8 (warfare, P4), not M9.

## 1. Dispositions

| # | Item | Disposition | What enters the plan | Owner (phase) | Confidence |
|---|---|---|---|---|---|
| 1 | Court circuit breakers | **Adapt.** The cascade is real. The fix must be mechanics, not a lock: a global `violence_locked` flag or a fiat stress cap at 80 is a regulative governor, which 05.4 and ADR-0009 forbid ("no hidden director") | (a) A cascade scenario test: one court death must keep break rates inside V-SAGA-16's band (0.5 to 5 per 100 named-actor-years). (b) Damping in the minds spec: grief stress per death scales with the relationship and saturates per instant; a per-actor refractory period after a break; all thresholds Registry values in fixed point. (c) The inquest is an institution, so it waits for M6-lite; regency is already in M7 | `T1.minds` spec and M7 (P1a); inquest with M6-lite | 80% |
| 2 | Causal motif grammar | **Adapt.** Most of it exists: ordered clauses with increasing EventId, `within_days`, `unless`, `require_edge` on motive edges, `match_event`, and anchored emotion reads (`05.4` §2). Missing: a minimum delay | Add a `min_days` key (the grudge latency) to the pattern language and to `lint_patterns.py`'s closed key set. Plane C (ruling N1) | P2.1 (P2) | 85% |
| 3 | Belief stores ("ghost king") | **Adapt.** Agents already act on beliefs, not world truth (`05.3`), and secrets v0 travel only along the route graph at travel speed (P1.22). Correction: beliefs are plane-A state, not separate from plane A. A false belief needs distortion, which comes with full epistemics | The "ghost king" (a false death report that arrives before the correction) becomes a scenario test for Epistemics v1 | P3.3 (P4); horizon P1.22 (P1b) | 85% |
| 4 | 2D settlement view as a pure receiver | **Already covered.** P5.11 is the settlement view; presentation never calls back into the engine (ADR-0035 §6); V-DET-07 checks that watching never changes history. `FAMINE_RIOT` and `GATE_BREACHED` are not Registry verbs yet | None | P5 | 95% |
| 5 | Vow arbitration | **Adopt the collision rule; decline the hard tiers.** S7 has priorities (`kind_weight`), break-on-strain and a `renounce` act class, but no rule for two vows that demand opposite acts | S7: when two active vows of one actor demand contradictory acts, the higher priority wins (ties: Registry kind order, then `VowId`); the loser breaks through the existing break path (`broken_vow_opinion`, the curse thought). Costs come from the Registry and reach minds through its in-coupling. New test: an actor with colliding vows never oscillates. Declined: hard-coded tiers (`kind_weight` is the tier), an honour column, and a string reputation (ADR-0010) | P1.20 (P1b) | 80% |
| 6 | Reflection tree (Park et al. 2023) | **Adapt, later.** Memory is a set of event references with ACT-R activation (`05.3`); consolidation fits there. Constraints: importance is symbolic and from the Registry (ADR-0034), sums are fixed point, events are never compacted (ADR-0007; the mind only drops its references), and a reflection that becomes a vow goes through `SwearVow` | Consolidation of low-activation memories into opinion modifiers, specified with memory | P3.2b (P4) | 75% |
| 7 | Unexpectedness, Σ −log₂ p weighted by elapsed time | **Decline as proposed.** `05.4` §3 already scores surprise with StU as a percentile within (pattern, era), and rejects raw −log p over open-ended chains: in Lessard et al. (2026) it selected ever longer chains and inexplicable outliers. Plane C, not A or B | None | P2.5 | 90% |
| 8 | Relic corruption attractor | **Adapt.** S6 F1 is already a fixed-point pull on one overlay, with exposure in `q16_16` and `BEARER_CORRUPTED` at a threshold. Corrections: HEXACO traits are static; S6 never writes minds columns; κ and Ψ would be fixed point. Prototype 10 (§5) shows the dynamics: a deterministic pull, vow dissonance raising stress, and a break into a broken vow | Multi-channel influence stays an S6 F2 option, applied through minds' in-coupling, never a write to traits. Vow dissonance as a stress source belongs to the minds spec (`T1.minds`), reading S7's vow-conflict signal | P1.19 (P1b); `T1.minds` (P1a) | 85% |
| 9 | Red-team charter | Approved as internal working rules (director, 2026-10-16). **Laws 2 and 3: adopted narrowly by the director at G0** (§2) | Principles 8 and 9 of `docs/01` §5 | G0 | — |
| 10a | Social practices (Versu) | **Adapt, evaluate.** New to the plan. A practice is a scheduled situation with roles and a reduced action set; it is a natural home for item 1's inquest and for bug 5's bounded action scans | Evaluate when the minds spec is written; candidate owner M6-lite or S5 | P1a spec work | 65% |
| 10b | Mythic domain tokens | **Adopt in the ruled form (N2).** Symbolic tokens in the Registry; adjectives, styles and prompt text in plane C | Token table at the visual-DNA work | P3 | 85% |
| 10c | Four-beat comic grammar | See 14 | — | — | — |
| 10d | Nobility thermostat | See 12 | — | — | — |
| 11 | Priority-Flood hydrology | **Adopt.** Barnes, Lehman and Mulla (2014) is the standard method. `rh-worldgen` today plans flux accumulation without depression filling | Depression filling before flow routing in worldgen, implemented from the paper: integer (u16) elevations and a total-order tie-break (elevation, then cell index) so the heap order is deterministic. Add the paper and RichDEM (licence check) to `docs/14`. Rivers as polity borders: a cost term for M5 polity growth, evaluated there | Full P1.1 worldgen (P4); earlier if P1.1a needs rivers | 85% |
| 12 | Nobility thermostat → in-world demotion | **Adopt the rebuilt mechanism.** SIM-9 already makes distant great-house members dormant. Prototype 7 (rebuilt) adds what SIM-9 lacks: finite seats, re-grant of vacant fiefs, vocations for later children, and demotion when a noble holds no seat, office, claim or active vow | M7 spec: seats are in-world objects; vocations (holy orders, mercenary companies) are careers of people who stay in the ledger; the named-actor count is a **test expectation** that scales with population (`08`), never enforced; no history is compacted (ADR-0007). The 150 to 350 band is not adopted as a number until it is measured | P1.7 (P1a) | 85% |
| 13 | Recursive causal-chain extraction | **Already covered.** `specs/chronicle/schema.sql` has the DuckDB `causal_ancestors` macro (`WITH RECURSIVE` over the many-to-many `causal_edge` table). Prototype 8 measures extraction, not search (N8) | A search benchmark with realistic causal density and no planted answer, in the V-SIFT rows | P2 | 90% |
| 14 | Four-beat comic compiler | **Already covered.** ADR-0035 decided four panels per scene (staging, confrontation, climax, consequence) plus panel 0, the character sheet. InstantID and similar tools are P3 candidates; "100% consistency" is prompt-level only (N9) | None | P3.16, P3.17 | 90% |
| 15 | Ten bug invariants into `CLAUDE.md` and GOV-008 | **Director ruled: as recommended** (§3). No `CLAUDE.md` change | GOV-003 §6, the failure-modes table, binding in review | G0 | — |

## 2. Laws 2 and 3 (director ruled, 2026-10-16)

The director adopted both, each worded narrowly. The Architect recommended the same, with slightly different wording (kept below for the record). `docs/01` §5 carries the director's wording, with the Architect's reading of how it applies. The director may overturn that reading.

| Law | Director's ruling | Architect's reading (`docs/01` §5) | Recommendation it replaces |
|---|---|---|---|
| **Law 2:** zero spontaneous reality (principle 8) | Every plane-A entity and mutation originates from an explicit, simulated in-world event. No god-mode spawning and no global count controllers | World creation is the root of the causal graph and seeds the initial state. "A simulated in-world event" means a system's mechanics, a timer that fires, or a journaled material input, so a routine column update needs no event record of its own. An ordinary prop drawn from aggregate stock is such a draw (the gap-1 ruling, D28). A count (nobles, population) is an outcome that tests check, never a target the engine enforces (item 12) | "Every plane-A entity is created by a journaled event or by world creation; nothing is spawned to steer a story." (85%) |
| **Law 3:** tragic, not slapstick (principle 9) | Mental breaks are constrained to mythic, consequential acts (usurpation, exile, feuds, broken vows); no slapstick or meme breaks | This is a rule for authoring. The Registry's break acts and stress sources are written for the saga register, and plane C tells events in that register. No rule in plane A filters, suppresses or rerolls an outcome for its tone (principle 4, ADR-0009). The listed acts are examples; each break act fits the actor's station | "The Registry's stress sources and break acts are authored for the saga register; plane C tells events in that register. No rule in plane A suppresses, rerolls or filters an outcome for its tone." (80%) |

## 3. Item 15: the ten bugs (director ruled, 2026-10-16: as recommended)

Ruling: **no change to the `CLAUDE.md` non-negotiables.** Most of the ten bugs are already guarded by a test or a requirement, and the rest are design choices with costs. The table below goes into GOV-003 §6 (coding standard) and binds every review: a reviewer cites its row, and a change that reintroduces a failure mode without its guard is refused. The Architect had recommended this at 80%.

| Bug | Its guard in the plan | Status |
|---|---|---|
| 1. Repath storm | Static route graph at M1 F0; unreachable targets fail with a backoff timer | Add the backoff to the movement spec (fugitives, P1b) |
| 2. Item clutter | SAGA-12 and V-SAGA-17: an artifact has exactly one bearer or location; goods are aggregate stocks (M4) | Covered |
| 3. Per-tick temperature | M3 F0 is an annual anomaly; each system has its own fidelity ladder (`05.1` §3) | Covered |
| 4. Paw-licking cascade | The abstraction horizon is the fidelity ladder; plane A has no hygiene model | Covered |
| 5. Full-map job scans | Candidate actions come from the agenda and the actor's situation | Check when the minds spec is written; see 10a |
| 6. Allocation stutter | V-PERF-11: an allocation budget per bench (default 0 for P0 machinery), failing like an instruction-count regression | Covered; a budget, not a blanket ban |
| 7. Nested containers | S6: one bearer or location; no containers in the schema | Add "no nested containers" to S6 explicitly |
| 8. Reality-bubble seams | The world runs whole, watched or not (ADR-0025); V-DET-07, observer invariance | Covered |
| 9. Rivers flowing uphill | Item 11 | Adopted |
| 10. Subgraph explosion | Ordered, anchored patterns with budgets (`05.4`); item 13 | Covered |

## 4. Corrections to the batches (factual)

| Claim | Correction |
|---|---|
| `M14.court_politics` | No such system. Court politics is M7 plus `T1.minds` |
| "S2.agents" | `S2` is `S2.technology` |
| Siege in M9 | Siege is M8 (P4); M9 is the validation harness |
| Item 13: "0.197 ms sifting" | One five-hop backward walk from a known event (N8); the Architect's runs took 0.312 ms and 0.433 ms |
| Item 12: "N bounded to [150, 350]" | Rebuilt prototype 7 ended at 98 active nobles with no count rule; the band is not yet measured |
| Item 15, bug 6: "under `CLAUDE.md` … zero heap allocation" | Neither `CLAUDE.md` nor GOV-008 says this; V-PERF-11 sets a budget |

## 5. Material received after the draft

| Commit (on `arch/red-team-batch-04`) | What | Disposition |
|---|---|---|
| `c3fa703` | Prototype 10: the relic corruption attractor, with vow dissonance as a stress source (`RT-ACT-013`) | **Taken in** as evidence for item 8. Three notes: (1) it drifts the bearer's traits, while in Rehoboam traits are static and S6 never writes minds columns, so the pull acts on an S6 exposure overlay and reaches minds through in-coupling; (2) it computes in floats, while the engine would use fixed point; (3) its 2-to-5-year window is a hand-set check with no source. It runs and passes (`run_all.sh`, now 10 prototypes) |
| `28b44ac` | A science dossier on pre-industrial demography, inequality and empire lifespans (Alfani 2021; Coale and Demeny 1983; Taagepera 1978, 1979) | **Held for the director** (the intake rule: the Architect believes parts are mistaken). The sources are on the "still wanted" list, so the figures cannot be checked against them. The Coale–Demeny level labels do not match the published levels (female West level 6 is e₀ = 32.5, not 30). The Weibull parameters and the 85% fracture risk have no source. "Gini clamped to [0.65, 0.85]" is a global controller, which Law 2 forbids: a band is a test (`V-SF`), never enforced |
