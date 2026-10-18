---
id: SPR-024
title: "Minds v0, part b, in two stages: event-driven IAUS with the typed considerations and response curves and same-instant agenda semantics (024a); marry, seek office, press a claim and feud, and M7 and T1 in the build (024b) (P3.2a, part b)"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint, simulation]
related: [BCK-002, PRJ-002, REV-MINDS, REV-WAVE5, REV-WAVE6, REV-WAVE7, REV-P1a, REV-SPR-017, REV-M7-membership, EVO-001, EVO-002, SPR-014, SPR-015, SPR-017, SPR-019, SPR-020, SPR-021, SPR-022, SPR-023, SPR-025, SPR-028, SPR-029, SPR-030, SPR-038, ADR-0006, ADR-0012, ADR-0024, ADR-0025, ADR-0033, ADR-0034, ADR-0035]
created: 2026-10-18
updated: 2026-10-18
version: 0.3.0
---

> **BLUF:** Builds the decision half of `T1.minds` in `rh-minds`, in **two stages** (X8; REV-MINDS N2). Each stage is its own PR, Tester pass and audit.
>
> **SPR-024a** (C2) promotes the **T1-I/S** spec (X7) and builds three things.
> - **The Registry's typed `[decision_types]`, `[considerations]` and `[response_curves]`** (REV-P1a MAJ-6). The considerations are a closed set of inputs implemented in code.
> - **Event-driven IAUS** (`05.3` §3).
>   - Polls are anchored at the first poll plus k periods, and every timer kind has its own purpose.
>   - Interrupts come at t + 1 day or later, under the daily cap.
>   - The score is the `Q16_16` curve product with compensation, plus an idle score and a keyed top-k pick.
> - **Same-instant semantics** (`05.1` §4.3) through a `due` hand-off table. They are proven on a test-only decision type, which gates **V-COG-03** and **V-COG-04** (both gate G1a).
>
> **SPR-024b** (C3, `BUILT` only) builds three more.
> - **Four decision types:**
>   - `marry`: a `MARRY_DUE` timer, revalidated when it fires, submits `Marry`, and `MARRIED_TO` is written once M2 has applied it;
>   - `seek_office` (`PetitionSeat`) and `press_claim` (`PressClaim`);
>   - `feud`: declare, attack (through `attack_outcome` and a `DEATH_DUE` timer) and reconcile.
>   Each comes with its `motivated_by` edges and the `stress(actor)` snapshot.
> - **The first real M7 + T1 integration** (X5), after SPR-022a's dormancy pass.
> - **The one `BUILT` move for M7 and T1 together** (X6).
>
> **One golden effect, headers only (024b).** No golden world names M7 or T1, so every root of `smoke`, `solo`, `household` and `hamlet` is unchanged. Adding both systems to `BUILT` does two things. It changes `engine_build` in the three manifest worlds' `tuple` lines (`crates/rh-golden/src/world.rs:256–260`, `:366–367`). It also changes their channel-A segment headers (`engine_build`, `registry_version` and `manifest_hash`, `crates/rh-stream/src/segment.rs:108–110`), because `registry_version` is bumped here. `bin/check_baseline.py`'s C3 rule (`:10–12`) therefore makes 024b's PR **C3**, re-recorded once by `bin/regen_baseline.sh` and announced to the director.
>
> - **Developer:** AGT-002-CORE, with AGT-002-TOOLS for the Registry loader, the reference port checks and the gates.
> - **Testers:** AGT-003-DET and AGT-003-IF (one session may hold both, on a different model from the Developer's), and AGT-003-STAT for V-COG-04.
> - **Size and class:**
>   - **024a** is about **3.0–3.25 agent-days** (T-01 1.0–1.25, T-02 2.0), **C2**.
>   - **024b** is about **3.25–3.75** (T-03 1.5, T-04 0.75, T-05 1.0–1.5), **C3 for `BUILT`**.
>   - The total is **6.25–7** (X8). Each stage has a hard checkpoint: after T-02's IAUS core (before V-COG-04's harness lands), and after T-03.
>   - The T1-I/S spec PR is counted separately (PRJ-002 §6).
> - **Status:** PLANNING, revision 2. REV-MINDS round 1 (N1–N7, Q1–Q3, finding 13), X1–X13 and the round-2 rulings R4, R5, R7, R8, R10, R11 and R14 are applied. It needs the director's plan approval (D59).

# SPR-024 — Minds v0, part b: IAUS and same-instant semantics (024a); four decisions and the build (024b) (WP P3.2a, part b)

- **Roadmap item:** `docs/11` P3.2a (`docs/11_roadmap.md:183`), part b under PRJ-002 ruling M-3 · **Phase / rung:** P1a; the T1 fixture court (024a) and the dynastic fixture worlds `manor` and `twin_vales` (024b). Neither world is golden before SPR-030.
- **Depends on (X9):**
  - **024a:** SPR-021b (M7 0.6.0 frozen; `rh-contracts::m7`; EVO-002's genealogy index and the instant log's events, X1(c), X2); SPR-023; **the T1-I/S spec PR**, which merges after T-00a **and after SPR-023 T-01 promotes T1 0.4.0**, as a new `next/T1` 0.5.0 draft (`next/` holds one draft per system; round-2 ruling R7). **It sits on 024a's critical path:** SPR-023 T-01, then T1-I/S's review, then 024a T-01; SPR-017 (relations in `rh-registry`, `specs/registry/effect_language_v0.md:240–270`); SPR-020 (the envelope's checked `emit`).
  - **024b:** 024a and **SPR-022a** (dormancy: M7 sends `Promote` at t = 0, X4, X5). T-05's probes on `manor` and `twin_vales` also need **SPR-019b** (the dynastic back-fill: both manifests' dynasties are back-filled, `specs/manifest/examples/manor.yaml:7`).
- **Binding contracts:**
  - **`specs/systems/T1.minds.odd.toml` 0.5.0** (T1-I/S, promoted by 024a T-01). It covers:
    - the `decide` submodel (0.3.3 `:901–904`, as amended);
    - `[[emits]]` `MURDER_ATTEMPT`, `FEUD_DECLARED` and `FEUD_ENDED` (`:423–457`), and `[[commands]]` `Marry`, `ScheduleDeath`, `PetitionSeat` and `PressClaim` (`:460–488`);
    - `[[rng]]` `poll_jitter`, `iaus_select`, `action_order`, `attack_outcome` and `outcome_delay` (`:753–785`), plus T1-I/S's timer purposes;
    - INV-T1-07, 08 and 09 (`:939–950`), the Day/Vital `outcome_due` step (`:731`), the I-steps, the `due.*` table, `[[observes]]`, and the `person.vocation` read.
  - **The Registry:** `[decision_types]` (`specs/manifest/registry_v0.toml:339–350`), `[considerations]` (`:352–375`), `[response_curves]` (`:377–417`), `[opinion_modifiers]` (`:312–322`), and the T1 verbs (`:608–614`).
  - **The engine:**
    - `05.1` §4.3 (`docs/05_architecture/05.1_simulation_core.md:175–195`; the same-instant items `:186–193`) and §2 rules 2 and 4, with EVO-002's text (X1(c), X2);
    - `05.3` §3 (`05.3_cognition_epistemics_oracle.md:62–72`);
    - `specs/chronicle/envelope_v0.md` §3 (`:85–88`) and §5 (`:107–129`); `specs/registry/effect_language_v0.md` §5.3, §6 and §9;
    - M2's N7 (`specs/systems/next/M2.demography.odd.toml:643`, frozen in 0.3.x).
  - **ADRs:** ADR-0006, ADR-0012, ADR-0024, ADR-0025 (outcomes are timers), ADR-0033 and ADR-0035; **ADR-0034** (a rule-based IAUS; `ADR-0034:31`).
  - **Tests (`docs/09`):** V-COG-03 (`:283`), V-COG-04 (`:284`; the agenda-fairness band at `:90`), V-COG-10 (`:290`), V-INV-03 (its P1a variant), V-INV-04, V-INV-12 (024b's probes), V-INV-15, V-DET-01, V-DET-02, V-DET-05, V-DET-12, V-DET-14, V-GOLD-01.
- **System spec:** 024a's T-01 promotes T1 from frozen 0.4.0 to **frozen 0.5.0** (T1-I/S). An erratum found later bumps it in place (0.5.x).
- **Rulings (binding):**
  - REV-P1a MAJ-4, MAJ-6 and MAJ-7;
  - EVO-001 R9 and R12;
  - REV-SPR-017's rulings on commands without a date and on Q8 (`CODEX/05_PROJECT/SPR-017_EffectLanguage.md:69`, `:78`);
  - REV-WAVE5 W2, W3 and W42;
  - REV-WAVE6 X1, X4, X5, V1, V2, V3 and Z1;
  - **REV-MINDS round 1** and **the cross-plan rulings X1–X12**. As in SPR-023, a bare "X" id below is a cross-plan ruling.
- **Director items:** D59. The class rule X12 is D58's.
- **Requirements covered:** COG-3 v0 (V-COG-03, V-COG-10); SIM-7 (V-COG-03, V-COG-04); DET-13 (outcomes as timers); DET-2 (keyed RNG only).

## Specs this sprint needs

| Spec | Exists? | Gap | Architect ruling | Needed by |
|---|---|---|---|---|
| **1. T1-I/S** | Missing (X7) | The I- and S-steps depend on SPR-017, SPR-020 and SPR-021 (T-00a, EVO-002) | **Ruled (X7, R7; 70–75%):** a new `next/T1` 0.5.0 draft, written over the frozen 0.4.0 **after SPR-023 T-01 promotes it** and after T-00a merges (`next/` holds one draft per system), reviewed as its own PR and promoted here by 024a T-01; on 024a's critical path. Its contents: <ul><li>the I- and S-steps;</li><li>**`default`s for T1's sixteen parameters** on `[[params]]` and in `specs/systems/next/registry_delta.toml` under `[systems."T1.minds"]` (M2's mechanism; R8), so `manor.yaml` compiles with T1 built and is not edited;</li><li>`[[observes]]` of `MARRIAGE` and `DEATH` (X1(c));</li><li>`break_timer` as a flag (SPR-025's row 9) and V-SAGA-16b's gate fixture (SPR-025 row 11);</li><li>rows 2–8, 10, 11 and 15 below;</li><li>the reference's I- and S-cases with every mutant of SPR-024 and SPR-025 shown on them;</li><li>proposed (row 16): a `manor` case in `specs/worldgen/selftest_fixture_worlds.py`, whose golden `named_at_start` line 024b's probe reads.</li></ul> It has its own `check_all` line. The Architect commissions it | T-01 |
| **2. The considerations are prose** | Inputs are text (`registry_v0.toml:355`, `:357`) | No free-string semantics (ADR-0010); `target_reachable` reads columns that T1 does not declare (`:375`; P1b) | **Proposed (7.5/10):** a closed, code-implemented input per id. The effect language's `needs_code` model for relations applies (`effect_language_v0.md:234`). The loader types `domain`, `unit` and `reads`. `lint_odd.py` checks that the `reads` are declared by T1 for every type whose `requires` names only frozen specs, and warns for the others (`pursue_goal`) | T-01 |
| **3. The curves and types, typed** | `registry_v0.toml:377–379`, `:341–342` | No loader check | **Proposed (8.5/10):** refused by name: knots not ascending; y outside [0, 1]; a value not exact in `Q16_16`; a missing or orphan curve; `poll_months` outside 1..12; `idle_score` outside [0, 1]; an unknown `requires`. Interpolation is linear, clamped at the ends and rounded down. The product and the compensation are rounded down at each multiply, in the I-steps' order | T-01 |
| **4. Polls cannot be decided inside the timer pass** | `fire` runs once per timer, in key order, on one view (SPR-014 branch `engine.rs:1262–1264`), before every binding (`:925–929`); T1's Day/Agenda order (`T1.minds.odd.toml:736`) | Deciding inside `fire` applies decisions in entity-id order, which V-COG-04 forbids. The spec's step order is unrealisable | **Proposed (8/10; the review: feasible):** a T1-owned `due.person`, `due.type` and `due.interrupt` table. `fire` appends a row; `decide`, in the binding, scores, applies and empties it. The table is empty at every instant's end (an invariant). The order is restated as it runs: the timer pass, then `apply_commands`, `promote`, `decide`, `threshold` | T-02 |
| **5. The court of an actor with no seat or office** | `T1.minds.odd.toml:798`, `:903`; relation rows exist only for holders (`:205`) | Most named members have no court and no marriage candidates, while M2 excludes the named from its own marriages (`next/M2.demography.odd.toml:365–371`; `next/M7_membership.md:82`, `:105`) | **Proposed (6.5/10):** it is the court of the highest-tier holder of its house (ties by ascending `TitleId`), else none. The house's title holders come from **the M7-owned house → title-holders derived index, exposed by `rh-contracts::m7` (SPR-021a; round-2 ruling R11)**, not from `rh-core` and not from a scan. SPR-030 measures the member share. Hypergamy stays P1b (`M7_membership.md:69`) | T-03 |
| **6. `marry`'s candidates and `MARRY_DUE` at fire** (REV-MINDS N5) | T1 excludes "parent, child or sibling" (`:903`); N7 drops a pair within `kin(a, 2)` or in holy orders (`next/M2:643`); `MARRY_DUE` fires a day after the decision | A pair can become invalid in that day | **Ruled (85%):** the candidate predicate is N7's, with a lagged identity read of `person.vocation` (T1-I/S). **The fire revalidates:** both alive, both unmarried, not `related(a, b, 2)`, neither in holy orders. Otherwise the timer is dropped with its reason (`dead`, or `ended` in SPR-038b's generic meaning, "the premise no longer holds"; R14), counted | T-03 |
| **7. `FEUD_WITH` and `MARRIED_TO` have no writer** | `registry_v0.toml:317–318`; no spec writes them | Two P1a opinion kinds never occur | **Ruled (N5; 85%):** T1 writes its own rows. `FEUD_WITH` is written both ways when `FEUD_DECLARED` is applied. **`MARRIED_TO` is written both ways after M2 applies the marriage:** T1's Day/Vital step, after M2's drain, lists the instant's merged `MARRIAGE` events through EVO-002's **`events_of(MARRIAGE)`**, in merge order (X1(c); `[[observes]]`; round-2 ruling R10), confirms `person.spouse`, and writes the rows for a named party. This is O(marriages this instant) | T-03 |
| **8. `MURDER_ATTEMPT`'s outcome path and payload** (REV-MINDS N4) | `effect_language_v0.md:437` dates a command (EVO-001 R9); the language cannot schedule a timer (`:294`) | A `[verbs] payload` field would be a third payload convention | **Ruled (85%):** no `effects` program. The payload goes in **F-8's `when`-clause convention**: T1-I/S amends `emits.MURDER_ATTEMPT.when` with `Payload: [success]` (as M2's and M7's kinds do; SPR-021 F-8), and adds no Registry field. On success, a `DEATH_DUE` timer is set with the attempt as its `cause`. §9's sample is marked superseded by an Architect erratum | T-01, T-03 |
| **9. Q8: the seven `rh-minds` verbs** | `registry_v0.toml:584–597` | A rename fails `lint_odd.py` and changes channel A's verb dictionary (`emitted_by`, `crates/rh-golden/src/stream.rs:273`) | **Ruled (the Architect, REV-MINDS finding 13; 75%): no rename in P1a.** The Architect files the BCK-002 item at close | — |
| **10. V-COG-10's edges** | `docs/09:290`; `envelope_v0.md:116`, `:122`, `:124`; no T1 event for `marry`, `seek_office` or `press_claim` (`T1.minds.odd.toml:417–457`) | Three types have nowhere to write their edges; trait contributors have no cause | **Accepted (6/10):** edges go on T1's own decision events. There is one per top-3 contributor that has a cause. `weight` = curve value − 0.5 (`Q8_8`); `evidence` = the margin, min(1, chosen − runner-up). The command-only types send `cause = None`, and decision events for them are a BCK-002 item for P1b. V-COG-10 is not a G1a gate (`docs/09:705`) | T-04 |
| **11. Timer details** (REV-MINDS N6) | "draw mod (poll_months in days)" (`T1.minds.odd.toml:756`); `schedule` refuses a tick at or before now; interrupts and refractory ends name no purpose | (a) A first poll can fall on the promotion instant. (b) `StepCtx::schedule` needs a purpose per timer. (c) Re-arming from each fire drifts | **Ruled (85%):** <ul><li>(a) The first poll is **strictly after** the promotion instant: the first day start after `promotion + (draw mod period)`.</li><li>(b) Each timer kind has its kind code and a named purpose in T1-I/S. Polls use `poll_jitter`. Interrupts, refractory ends, `MARRY_DUE` and `DEATH_DUE` use no-draw purposes (as `outcome_delay`, `:782–785`). Breaks use `break_delay`.</li><li>(c) Polls are anchored at first + k × period, where period = `poll_months × minutes_per_year / 12`, each at its day start.</li></ul> | T-02 |
| **12. The genealogy index** | Built by **SPR-021 T-02** under EVO-002 (X2) | — | **Ruled (X2; 70%): a consumer here.** `marry`'s `related(a, b, 2)` and `kin_distance_to_granter` read the core index through SPR-017's host `Indexes`. A consumer test compares them with a brute-force walk on three sizes and eight seeds. No private copy | T-01 |
| **13. V-COG-04 as a gate** | `docs/09:284`, `:90`; G1a gates it (`:705`) | No sample size | **Proposed (8/10; the review checked it):** 4,000 fixture instants × 8 contenders for one vacant office on the probe type; n = 32,000; **SE 0.0057**; TOST at α = 0.05 per side with δ = 0.05. The seeds are fixed, so the gate is deterministic. The by-id mutant gives ρ = −0.577 (`minds_review/work/cogs04_se.py`). AGT-003-STAT states the power | T-02 |
| **14. `rh-contracts::m7`** | SPR-021 T-02's (REV-MINDS Q1); SPR-023 read M7's columns by name (SPR-023 row 14) | — | **Ruled:** 024a switches T1's twelve M7 reads to `rh-contracts::m7`, with a consumer test | T-01 |
| **15. Re-sampling T1's timers** | A material input asks `resample_timer` (SPR-014 branch `engine.rs:33–40`); the default refuses (`system.rs:1093–1105`); only `outcome_delay` has a rule (`T1.minds.odd.toml:784`); V-DET-17's input targets a named sovereign (`specs/determinism/crn_pairing_v0.md:5`, `:33`) | Without a rule a run stops | **Proposed (7/10; the review: legal):** polls, interrupts, refractory ends, `MARRY_DUE` and `DEATH_DUE` keep their tick. A break is re-drawn by `break_delay` (SPR-025's row). This is stated in T1-I/S | T-02 |
| **16. The first real integration** (REV-MINDS N1) | After SPR-021 alone, M7 sends no `Promote` (dormancy is SPR-022's) | No plan owned the M7 + T1 integration | **Ruled (X5; 80%): 024b owns it, after SPR-022a.** 024a integrates against the stub with a scripted `Promote` set. **024b's probes (round-2 ruling R4):** on `twin_vales`, the minds at t = 0 equal the golden's `named_at_start full` count for that world and seed, read from `specs/worldgen/golden/fixture_worlds_v0.txt` by the test (never typed; seeds 1–3 give 33, 35 and 26 today, `:16`, `:62`, `:106`), and are at least 20; on `manor`, they equal the golden's figure for `manor` and seed, with no floor; plus at least one marriage and one pressed claim. (Revision 1's "at least 50 minds" was false for a correct build.) **No golden holds a `manor` figure today:** the file has `twin_vales` and `kingdom_10k` only (`:13`, `:59`, `:103`, `:156`, `:266`), and its reference knows only those two (`WORLDS`, `specs/worldgen/selftest_fixture_worlds.py:170`; `SEEDS`, `:95–98`). **Proposed (7/10), for the Architect:** the T1-I/S spec PR adds a `manor` case at seeds 1 and 2 (the seeds SPR-019's golden pins, `specs/worldgen/golden/population_synthesis_v0.txt:70`, `:107`) to that reference, and renders its `named_at_start` line by `--write`, never by hand. Until it lands, the `manor` probe has no figure and 024b T-05 is blocked (blockers row 2) | T-05 |
| **17. The class and the build** | PRJ-002 classed SPR-024 C3 | — | **Ruled (X6, X12; 80%):** **024a is C2** (no `BUILT` move; its PR shows the `systems` lines and `BUILT` unchanged; its rates are reported). **024b is C3 for `BUILT` only:** M7 and T1 join together in its `[C3]` commit. SPR-021, SPR-022, SPR-023 and 024a test with their own built list | T-05 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **024a: C2** (the T1 0.5.0 freeze, the typed tables, `due.*`, `rh-contracts` consumers). **024b: C3** by the baseline rule, headers only: `BUILT` gains `M7.dynasty` and `T1.minds`, and `registry_version` is bumped. PR titles `SPR-024a: … [C2]`, `SPR-024b: … [C3]` |
| Planes touched (ADR-0024) | **A:** `rh-minds`, `rh-registry` (typed tables). **Tool:** `rh-manifest`, `rh-golden` (`BUILT`, 024b), `specs/`, `bin/check_all.sh`. **B, C:** none |
| Owned columns written | `T1.minds`: `due.*` (024a); `feud.*`, `mind.interrupt_day`, `interrupts_today`, `opinion.*` (`FEUD_WITH`, `MARRIED_TO`) (024b). Marriages, petitions, claims and deaths only through commands (INV-T1-08) |
| Reads | SPR-023's reads, with M7's switched to `rh-contracts::m7` (024a); `person.vocation` (M7, lagged, identity); the core genealogy index (X2); the instant log's `MARRIAGE` and `DEATH` events (`[[observes]]`, X1(c)). `pursue_goal` and `swear_vow` are disabled without S6 and S7 (`registry_v0.toml:341–342`) |
| RNG purposes | Drawn: `poll_jitter`, `iaus_select`, `action_order`, `attack_outcome`. No draw: `outcome_delay` and T1-I/S's timer purposes (row 11 (b)). None is re-sampled on a fork (row 15) |
| Golden-hash effect | **Every root is unchanged** on all four worlds, in both stages. **024b's headers:** the `tuple` lines of `solo`, `household` and `hamlet` (`engine_build`'s built list, `registry_version`, `manifest_hash` with the re-pinned manifests) and their segment headers (`engine_build`, `registry_version`, `manifest_hash`). `smoke` byte-identical. Re-recorded by `bin/regen_baseline.sh` and announced |
| Fixed-point quantities | Curves, scores and compensation in `Q16_16`, rounded down at each step; the edges' `weight` and `evidence` in `Q8_8`; `attempt_success` as `Prob32`. No float |
| Determinism lints | Must pass. Candidates and score ties in ascending target id (`T1.minds.odd.toml:760`); `action_order` ties in ascending `PersonId` (`:764`); no `HashMap`; no interior mutability (W31) |
| Per-instant cost (REV-WAVE6 X1) | `decide` is O(due) and the `MARRIED_TO` step O(marriages this instant). Candidate sets are bounded by the actor's situation (`:713`). The feud death path reads `exited()` (X1(b)). Idle bindings are O(1) |
| Tests that gate this sprint | **024a:** V-COG-03 and V-COG-04 on the probe type (both gate G1a), V-INV-04, V-DET-01, 02, 05, 12 and 14, V-GOLD-01 (unchanged). **024b:** V-COG-03 on the real types, V-COG-10 (P1a scope), V-INV-03 (subset), V-INV-12 (the probes), V-INV-15 with the real M7, V-GOLD-01 (three headers re-recorded) |
| Docs updated in the same sprint | `bin/check_all.sh`, the inventory test, `CLAUDE.md`; the `docs/09` method lines for V-COG-03, V-COG-04 and V-COG-10; `effect_language_v0.md` §9's erratum (the Architect's); `05.0` §6; `worlds/*/README.md` (024b's header move); PRJ-002's row; BCK-002 (Q8; the decision events of row 10) |

**Rules carried (every task):**
- Everything in SPR-023's list.
- **Baselines** are regenerated only by `bin/regen_baseline.sh`, in 024b's final commit, whose subject ends in `[C3]`.
- **The annals fixtures and goldens** are regenerated by their tools when `registry_version` moves (W1 (d)).
- **Tail snapshots** are rewritten locally under `RH_UPDATE_SNAPSHOTS` and reviewed in the diff.
- **One statistical gate:** V-COG-04, with its power stated.

## Tasks

**Stage 024a** is T-01 and T-02, with a hard checkpoint after T-02's IAUS core. **Stage 024b** is T-03 to T-05, with a hard checkpoint after T-03.

#### T-01 (024a): The promotion of T1 0.5.0, the typed decision tables, the consumers (C2; rows 1–3, 8, 12, 14)
- **Deliverable:**
  - **The promotion of T1-I/S**, with SPR-023 T-01's checklist: (a) the `clean` control and its pinned line re-derived; (b) the frozen and `--next` lints; (c) S6's and S7's reads; (d) the annals lints; (e) the skeleton; (f) the reference's I- and S-case gate.
  - **The typed tables:** `[decision_types]`, `[considerations]` and `[response_curves]` typed and compiled into `rh-registry`, with the rows 2–3 refusals and `lint_odd.py`'s reads check.
  - **Consumer tests:** the genealogy index (row 12), `rh-contracts::m7` (row 14) and its house → title-holders index (row 5; R11).
  - **T1's defaults (R8):** T1-I/S's sixteen `default`s move from `registry_delta.toml` into `registry_v0.toml` (`[systems."T1.minds"] defaults`), and the delta's T1 section is removed. SPR-023's missing-parameter control is retired (every T1 parameter now has a default); a planted default outside its Registry bound fails `lint_odd.py` by name instead.
- **Acceptance:**
  - Each refusal, planted once, fails by name in both the loader and the lint.
  - The compiled tables equal the reference's exact decimals.
  - The index's consumer test and the contract's consumer tests pass.
  - `manor.yaml`, unedited, compiles with `M7.dynasty` and `T1.minds` in the built list against the defaults; SPR-019's `manor` goldens are unchanged (R8).
  - The four ladders are unchanged.
- **Status:** [ ] not started

#### T-02 (024a): The IAUS core, the same-instant semantics, V-COG-03 and V-COG-04 on the probe type (the checkpoint; rows 4, 11, 13, 15; INV-T1-07)
- **Deliverable:**
  - **Timers (row 11):**
    - polls are anchored and re-armed at first + k × period;
    - interrupts come at t + 1 day or later on a new `PASSED_OVER` (the P1a trigger), under the daily cap;
    - a dead actor's poll is dropped at its fire;
    - `resample_timer` follows row 15.
  - **The `due` hand-off** (row 4).
  - **Scoring:** the curves, the product, the compensation, `idle_score`, effective traits through thoughts, and the top-k pick by `iaus_select`.
  - **Same-instant semantics:** the snapshot at the binding's start, the preconditions and read and write keys from the I-steps' table, `action_order`, invalidation, one re-score, then deferral.
  - **The probe type** (test-only: one consideration, one office).
  - **The harnesses:** the V-COG-03 observer (100 years at the PR tier, 1,000 nightly) and the V-COG-04 harness (row 13).
  - **Mutants:** `mutant-action-order-by-id`, `mutant-no-rescore`, `mutant-curve-round-up`, `mutant-interrupt-same-day`.
- **Acceptance:**
  - **Bit-equality** with the I-cases for scores, picks, the permutation, a conflict instant and the timer anchors.
  - **Mutants:**
    - `mutant-action-order-by-id` fails the conflict line, and fails V-COG-04 (ρ about −0.58);
    - `mutant-no-rescore` fails the second-choice line;
    - `mutant-curve-round-up` fails every odd-numerator score line;
    - `mutant-interrupt-same-day` fails the V-COG-03 observer by name.
  - **INV-T1-07 and V-COG-03:** zero violations over 100 years of the court.
  - **V-COG-04:** TOST passes, with the stated power.
  - **Re-sampling:** a scope-only input on a named actor that holds each timer kind matches the reference.
  - **Idle cost:** an idle Day/Agenda binding reads only the `due` table's length.
  - **Checkpoint:** the Tester's interim pass on the IAUS core is recorded before the V-COG-04 harness lands.
- **Status:** [ ] not started

#### T-03 (024b): The four decision types and their acts (the checkpoint; rows 5–8; MAJ-4; EVO-001 R9)
- **Deliverable:**
  - **`marry`:**
    - candidates come from row 5's court under row 6's predicate;
    - `MARRY_DUE` is scheduled at (d + 1 day start, Vital) and revalidated when it fires (row 6);
    - the fire submits `Marry`;
    - `MARRIED_TO` is written after M2 applies the marriage (row 7).
  - **`seek_office`:** `PetitionSeat`. **`press_claim`:** `PressClaim`.
  - **`feud`:**
    - declare: a `feud` row and `FEUD_DECLARED`, with `FEUD_WITH` both ways;
    - attack: `attack_outcome`, `MURDER_ATTEMPT` with its payload clause, and `DEATH_DUE` on success (row 8);
    - reconcile: `FEUD_ENDED`;
    - on a party's death, `FEUD_ENDED` at the death instant through `exited()`.
  - **Mutants:** `mutant-marry-kin1`, `mutant-attack-no-timer`.
- **Acceptance:**
  - **Bit-equality** with the reference's cases for each type.
  - **The death path (V-INV-03 subset):**
    - a successful attack kills at the next day start through M2's drain, and the `DEATH` carries a `triggered` edge to the attempt;
    - if the target dies naturally first, the `DEATH_DUE` is dropped (`dropped{T1.minds, DEATH_DUE, dead}`).
  - **Mutants:**
    - `mutant-marry-kin1` submits a grandparent–grandchild `Marry`, which N7 drops, so the kin case fails;
    - `mutant-attack-no-timer` fails the race case's drop line. `rh-core` does not refuse an outcome command submitted outside its sender's timer (SPR-014 branch `system.rs:879–950`), so this case is the only kill.
  - **Revalidation:** a planted `MARRY_DUE` whose partner married in between is dropped, not submitted.
  - **No dropped marriage** for kinship or vocation over 200 years of the court.
  - **Commands:** `PetitionSeat` and `PressClaim` reach M7's Day/Agenda drain the next day (`next/M7:820`); the queue-age invariant holds.
  - **Checkpoint:** the Tester's interim pass is recorded before T-04 lands.
- **Status:** [ ] not started

#### T-04 (024b): The envelope and V-COG-10 (row 10)
- **Deliverable:**
  - **Edges:** `motivated_by` edges by row 10's rule.
  - **The snapshot:** `stress(actor)` on `FEUD_DECLARED`, `FEUD_ENDED` and `MURDER_ATTEMPT`, the pre-act value (0 until SPR-025).
  - **The feud attack's motive:** `MURDER_ATTEMPT` carries `RETALIATION_FOR` or `AMBITION_FOR` to the feud's cause (`envelope_v0.md:86`).
- **Acceptance:**
  - Each T1 decision event carries its edges, with `evidence` set to the margin.
  - A planted edge to a later event is refused (`ForwardCause`).
  - The annals lints pass.
  - V-COG-03's observer re-runs on the real types with zero violations.
- **Status:** [ ] not started

#### T-05 (024b): The first real integration, M7 and T1 in the build, `registry_version`, CI (rows 16, 17; X5, X6, X10; W2, W3; N3)
- **Deliverable:**
  - **The integration (X5)** on `manor` and `twin_vales`, with M2, M7 (0.6.0, with SPR-022a's dormancy) and T1:
    - 50 years at the PR tier, 300 years nightly;
    - INV-T1-01 to 08 checked by observers, plus V-INV-15 and thread invariance;
    - **the probes (row 16; R4):** on `twin_vales`, minds at t = 0 equal to the golden's `named_at_start full` count for the world and seed, read from `fixture_worlds_v0.txt`, and at least 20; on `manor`, equal to the golden's `manor` figure for the seed (the `manor` case that row 16 proposes T1-I/S adds); at least one marriage and one pressed claim;
    - `manor` is compiled with T1 built against the Registry defaults (R8);
    - the stub remains only in the unit tests.
  - **The final commit, `[C3]`:**
    - `BUILT` gains `M7.dynasty` and `T1.minds` (X6);
    - `registry_version` is bumped to the next free number, for everything since the last bump (M7's defaults included);
    - the manifests' pins and the worlds' SHA-256s follow;
    - the annals fixtures and goldens are regenerated by their tools;
    - `solo`, `household` and `hamlet` are regenerated by `bin/regen_baseline.sh`;
    - tail snapshots that print the header are rewritten locally.
    The `registry_version` bump is measured alone first.
  - **The divergence record** (in the PR body and `VER-SPR-024b`). SPR-024a's record holds its C2 evidence.
  - **The gates, by role and part** (X10; estimates on one runner):

    | Stage | Gate | Role | Part | Profile, horizon | Est. |
    |---|---|---|---|---|---|
    | 024a | Typed-table tests, refusals, consumer tests | by-name tests | core | debug | 4 s |
    | 024a | Reference I- and S-cases (`selftest_minds_v0.py`) | spec reference | core | Python | 4 s |
    | 024a | IAUS unit and I-case tests, one mutant rebuilt | by-name tests and mutants | core | debug | 25 s |
    | 024a | V-COG-03 at 100 years; V-COG-04 harness | release system gates | golden | release | 4 s + 6 s |
    | 024b | Decision tests, the kin and race cases, one mutant | by-name tests and mutants | core | debug | 25 s |
    | 024b | Integration on `manor` and `twin_vales` at 50 years, 1 and N threads | release system gates | golden | release | 15 s |
    | 024b | The three ladders and their `tail` and `replay` gates | golden runs; stream | golden; stream | release | unchanged |
    | both | Other mutants, 600 actors, V-COG-03 at 1,000 years, integration at 300 years | soak | nightly | release | — |

- **Acceptance:**
  - **Robust form (true for a correct build):**
    - `smoke` is byte-identical, header included.
    - On `solo`, `household` and `hamlet`, **every root is byte-identical** at every rung.
    - The `tuple` line differs only in `engine_build`'s built list (`M7.dynasty` and `T1.minds` added), `registry_version` and `manifest_hash`; the `systems` line is unchanged.
    - Anything else that differs is a defect.
  - **Channel A (N3):**
    - every record is byte-identical;
    - the segment header differs only in `engine_build`, `registry_version` and `manifest_hash` (`crates/rh-stream/src/segment.rs:108–110`);
    - the verb dictionary is unchanged (row 9; no payload field is added, row 8).
  - **Ladders** equal the new baselines at 1 and N threads; the nightly monthly ladders are equal on the head; V-PERF-11's budgets are unchanged.
  - **CI:**
    - 024a adds about **+33 s core** and **+10 s golden** by gate sums; 024b about **+25 s core** and **+15 s golden**; stream is unchanged.
    - Budgets ×1.5: **+50 / +15 s** and **+40 / +25 s**.
    - **Nightly candidates:** V-COG-04's harness (with a PR-tier 500-instant smoke that asserts nothing statistical) and the N-thread integration repeat.
- **Status:** [ ] not started

## Director items, and what changes under the alternative

| Item | Planned | Under the alternative |
|---|---|---|
| **Two stages, 6.25–7 agent-days** (X8; D59 item 1; 65%) | 024a (C2) and 024b (C3 for `BUILT`), each with a checkpoint | One sprint: one PR above GOV-005's ~3-day rule by more than twice, with the real integration waiting on SPR-022a either way |
| **M7 and T1 join `BUILT` together, in 024b** (X6; D59 item 3; 60%) | One header rebaseline instead of up to three; SPR-021, SPR-022, SPR-023 and 024a stay C2 | Each system moves the headers in its own sprint: three announcements and three C3 slots |
| **V-COG-10 scoped to T1's own events in P1a** (row 10; 6/10) | The command-only types carry no edge; a BCK-002 item for P1b | A decision verb per type now: three Registry verbs, three annals labels, and a dictionary move on every world |

## Out of scope (explicit)
- **Stress, grief, breaks, dissonance** (SPR-025). `stress(actor)` is snapshotted, and is 0 until then.
- **`swear_vow`, `pursue_goal`, `respond_to_threat`** (P1b; `T1.minds.odd.toml:72`). Knowledge rows and V-SAGA-18.
- **Hypergamy** (P1b) and **decision events** for the command-only types.
- **The engine mechanisms:** the instant log (SPR-038b; SPR-021 T-02 for its events) and the genealogy index (SPR-021 T-02). This plan consumes them.
- **The CRN ensemble** (SPR-030), **V-SF-22 for T1** (SPR-028), and **`twin_vales` as a golden world** (SPR-030).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-DET | T-02, T-03 and T-05, written before reading the code from the I-steps, `05.1` §4.3 and EVO-001 R9:<ul><li>**the 024a checkpoint:** their own scorer, permutation and timer anchors, checked against the engine;</li><li>conflict instants of their own;</li><li>**the 024b checkpoint:** the four types on the reference's cases, plus the `MARRY_DUE` revalidation;</li><li>the six mutants, plus three of their own;</li><li>re-sampling and keyframes;</li><li>the probes and the divergence record</li></ul> | `VER-SPR-024a-TEST`, `VER-SPR-024b-TEST` |
| AGT-003-IF | T-01, T-03 and T-04, from the Registry, T1 0.5.0's `[[commands]]` and `[[observes]]`, M7's and M2's `[[applies]]`, and `envelope_v0.md` §3 and §5:<ul><li>the promotion checklist on a copy;</li><li>refusals of their own;</li><li>registration with the real M7;</li><li>the consumers of the index and of `rh-contracts::m7`;</li><li>the edges, checked through DuckDB</li></ul> | As above |
| AGT-003-STAT | T-02's V-COG-04, from `docs/09:284`, `:90`: the harness's independence, their own TOST, the power, and the mutant's ρ | `VER-SPR-024a-TEST` |

## Merge route

- **Class and conditions.**
  - **024a (C2):** merged after the Tester's pass and the audit; its rates are reported (X12).
  - **024b (C3, headers only):** merged after the Tester's pass, the audit and the announcement of the three header moves; the director may veto. No root moves, so the CRN ensemble has nothing to compare against.
- **Order (X9; 65%):**

```mermaid
flowchart LR
  S021b[SPR-021b] --> S024a[SPR-024a C2<br/>IAUS core]
  S023[SPR-023 C2] --> S024a
  S023 -.T-01 promotes T1 0.4.0.-> T1IS[T1-I/S spec PR<br/>next/T1 0.5.0, after T-00a]
  T1IS --> S024a
  S024a --> S024b[SPR-024b C3<br/>M7 and T1 in BUILT]
  S022a[SPR-022a<br/>dormancy] --> S024b
  S019b[SPR-019b<br/>back-fill] -.manor, twin_vales probes.-> S024b
  S024b --> S025[SPR-025 C2]
```

- **Shared files:**
  - `rh-golden/src/world.rs` (`BUILT`, 024b only);
  - the Registry and `rh-manifest` (with SPR-021 and SPR-022);
  - `check_all.sh`, the inventory test and `CLAUDE.md`.
- **Rung checkpoint** (GOV-007 §3.5): when `twin_vales` first runs green with T1 deciding, the director may read a 50-year `rh-cli tail --follow` of one house.

## Risks

| # | Risk | Likelihood / impact | Mitigation |
|---|---|---|---|
| 1 | The named core's thin marriage market (row 5) | High / thin family histories | Row 5's court rule; SPR-030 measures; hypergamy is the P1b lever |
| 2 | V-COG-04 fails on a correct build | Low / a false red | STAT reviews the harness; the seeds are fixed |
| 3 | SPR-019b is late (it waits on A-62), so 024b's probes have no world | Medium / 024b waits | The probes are 024b's only dependency on it; 024a does not wait |
| 4 | Same-instant read and write keys need more than the I-steps list | Medium / an erratum (0.5.1) | 024a's checkpoint tests the core on the probe type |

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | SPR-023, SPR-021b, the T1-I/S spec PR after SPR-023 T-01 (024a) | Open |
| 2 | SPR-022a; SPR-019b for the probes; a `manor` case in `selftest_fixture_worlds.py` with its golden line in `fixture_worlds_v0.txt`, rendered by `--write` (row 16; proposed for T1-I/S), before T-05 (024b; R4, R5) | Open |
| 3 | The director's plan approval (D59) | Open |

## Completion

- [ ] 024a: T-01 and T-02 meet their acceptance criteria; the checkpoint was held; `check_all` passes; DET, IF and STAT PASS; `VER-SPR-024a` PASS; class C2; T1 0.5.0 frozen; V-COG-03 and V-COG-04 gate in CI.
- [ ] 024b: T-03 to T-05 meet their acceptance criteria; the checkpoint was held; `check_all` passes; DET and IF PASS; `VER-SPR-024b` PASS, with the divergence record and the probes; the three header moves were announced; `M7.dynasty` and `T1.minds` are in `BUILT`; `registry_version` moved in the `[C3]` commit.
