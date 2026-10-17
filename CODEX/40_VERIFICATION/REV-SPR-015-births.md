---
id: REV-SPR-015
title: "Independent review of the M2 F1 births spec (PR #50)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-015, SPR-016, EVO-001, REV-EVO-001, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF:** Round 1 on PR #50 at `421f2e1`: **not ready to freeze. 1 blocker, 5 majors, 10 minors, 7 nits.** The arithmetic is right: every check value was reproduced independently. The spec fails as a contract, for four reasons. (1) The `--next` preservation lint cannot carry an amendment the spec needs (INV-M2-03), and it misses silent changes. (2) Two mechanisms do not fit rh-core's frozen trait and timer layout. (3) F0's band formula is wrong. (4) V-SF-24 fails by construction. The Architect's rulings are in §0. The drafter revises, and a round 2 re-checks. The review ran in a session on a different model from the drafter's. The reviewer's scripts and probes were kept in the Architect's scratchpad. The findings below give each reproduction.

# REV-SPR-015: independent review of the M2 F1 births spec

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | BLK-1 | Accepted. A next version may amend a frozen statement only through an explicit, lint-checked record: an `amends` entry carrying the frozen text byte for byte, the new text and a reason. The lint checks the old text against the frozen file and reports every amendment. Use it to scope INV-M2-03 to death (and marriage) timers, and for S2 through a normative `[algorithm.parameters_f1]` that names the S2 sentences it replaces | 8/10 |
| R2 | MAJ-1 | Accepted. Pregnancy state that a re-sample never changes: `person.conceived_tick`, the conception pulse t_m, recommended over deriving it from `own_timers`. F5 reads "pregnant" from it, and F8 computes the due month as the month of t_conc + G. INV-M2-10 is restated without a tick equality. `resample_timer` writes nothing, as the frozen trait requires | 8/10 |
| R3 | MAJ-2 | Accepted: correct F0's band formula, and add a reference check that pins the 0..6 range | 9/10 |
| R4 | MAJ-3 | Accepted. V-SF-24's identity becomes conceptions per susceptible woman-month, by band at conception, inside the binomial interval of c_i. The Coale–Trussell shape is reported against the exact per-band values the reference computes from the chain, not against M·n_i | 8/10 |
| R5 | MAJ-4 | Accepted in principle; the shape depends on EVO-001 R6 (scheduled outcomes held in queues, PR #49 round 2). Binding now: a commanded death or marriage is never filed under a purpose with `resample_on_fork = true`, because re-sampling belongs to the system that sampled the outcome, and the command's cause, culprit and `cause_event` are never lost. If R6 lands on (a), M2 owns distinct kinds (`COMMANDED_DEATH`, `COMMANDED_MARRIAGE`) whose payload indexes an M2-owned plane-A pending table, hashed and in keyframes, and which keep their fixed tick on a re-sample. If R6 lands on (b), the sender owns the timer and M2 applies at its drain binding. The births and EVO-001 drafters settle this together | 7/10 |
| R6 | MAJ-5 | Accepted. In `--next` mode the superseding spec gets frozen-level strictness on `[[applies]]` and `[[emits]]`, and `from` must be a superset of the frozen `from`. `[[emits]].when`, `[initialization]` and the frozen `[[schedule]]` steps are compared as supersets of the frozen lines, or changed through R1's amendment record. Each case gets a self-check probe | 9/10 |
| R7 | MIN-1 | Accepted. Check 5 gains widowhood (the 42-state chain). The facts are restated: NRR = 1 at tfr ≈ 4.74; mid-band (+0.1 %/yr) at ≈ 4.9. Q5 goes to the director with these numbers and MIN-10's birth-interval trade-off | 8/10 |
| R8 | MIN-2 | Give `household.yaml` an unrelated founder so a marriage is possible, or state that its extinction is the rung-1 expectation. Recommended: the founder, since a rung that cannot persist teaches nothing | 7/10 |
| R9 | MIN-3 to MIN-8, MIN-10, nits | Accepted, with the reviewer's fixes. MIN-5: M2's minimum ages apply to M2's own draws; a command follows its sender's rule, and holy orders exclude a person from the husband draw. MIN-6: servants are members (ruling 10). MIN-7: the infant sub-schedule's exposure in the month of birth belongs to SPR-016, and the mortality drafter is told. MIN-8: the husband is `argmin` of a keyed hash over the candidates (CRN-stable) | 8/10 |
| R10 | MIN-9 | Accepted. The G1b gate-list edit is reverted until the director rules on Q6. The rows stay, marked "proposed (Q6)" | 9/10 |
| R11 | Drafter's Q1–Q4 | Q1: the transcription is cited-pending, and a research batch confirms n(a), the sex ratio and Barrett's 11 months before freeze (8/10). Q2: SPR-014's row-insert contract as assumed, to be confirmed against EVO-001 round 2 (7/10). Q3: the `household/v1` H64 layout in `rh-ids` is accepted as additive (8/10). Q4: marriage spells in channel A are deferred to P1.11 (7/10). Q5 and Q6 go to the director as D-items after round 2, with R7's numbers | — |

## 1. The review (round 1, independent reviewer)

Reviewer: independent (adversarial). Copy reviewed: `$R/src` (git archive of 421f2e1). Line numbers are in that copy.
`$R` = `/tmp/claude-0/-home-user-Rehoboam/78f410bc-aa41-55ce-883d-f1b2ebd2cbcd/scratchpad/births_review`. Reviewer's
own scripts: `$R/tools/recompute.py`, `chain.py`, `chain_widow.py`, `microsim.py` (tool side, floats allowed; none of
them is the engine).

## BLUF

**Not ready to freeze. 1 blocker, 5 majors, 10 minors, 7 nits.** The arithmetic is right: every check value reproduces
independently, the gates are wired and pass, and the RNG keying, forward-only timers, id derivation and kinship columns
are sound. What fails is the spec as a contract:

- **The preservation mechanism cannot express this spec.** The frozen INV-M2-03 is now false, because birth timers are
  M2 timers that stay pending across Month pulses. `lint_next` forbids the amendment the spec needs, and it lets other
  frozen behaviour change with no error at all.
- **Two mechanisms do not fit rh-core's frozen trait and timer layout.** Writing `due_tick` during a re-sample, and
  commanded deaths as kind-1 timers, both have nowhere to go.
- **The F0 band formula is wrong as written.**
- **V-SF-24 would fail by construction** in three of seven bands.

The tfr default is also justified with the wrong number: r is about −0.03 to −0.04 %/yr, not +0.165 %/yr. Most fixes
are a few lines each; MAJ-1 and MAJ-4 need a design choice. Confidence in the overall verdict: 85%.

## Findings

### BLK-1. Frozen INV-M2-03 is contradicted by F1, and the `--next` rule forbids fixing it (confidence 85%)
- **Where:** `specs/systems/next/M2.demography.odd.toml:634-636` (INV-M2-03, frozen text kept verbatim); F6/F7
  `:509-510`; INV-M2-10 `:663-665`; `specs/systems/lint_odd.py:205-210`.
- **Problem.** INV-M2-03 reads: "every pending M2 timer drawn at the Month pulse t_m … lies on a day start in
  [t_m + 1440, t_(m+1) − 1440], so none is pending at a Month pulse instant". F7 makes a BIRTH_TIMER, which is an M2
  timer drawn at t_m, due in month m + 9 and pending across eight Month pulses. Implemented as written, INV-M2-03 fails
  on every pregnancy. The header (`:14-15`) says "the rung-0 semantics are therefore unchanged". That is true of the
  bytes, not of the meaning.
- **Why it blocks.** The only correct fix is to narrow the statement to death and marriage timers. `lint_next` rejects
  any change to a frozen invariant's statement (`lint_odd.py:210`), so its first use shows the mechanism cannot carry
  a needed amendment. The same pattern appears at `:499`: the frozen S2 sentence ("tfr … not read … not compiled") is
  "superseded" by a comment in a new table, which the lint cannot see.
- **Fix.** Let a next version amend a frozen invariant or algorithm sentence only through an explicit, lint-checked
  record, for example `[[invariants]] amends = "<frozen statement>"` with `reason`. The lint then requires the old
  statement to match the frozen one byte for byte and reports the amendment. Use it to scope INV-M2-03 to
  DEATH_TIMER (and MARRIAGE_TIMER). Do the same for S2 with a normative `[algorithm.parameters_f1]` that declares the
  S2 sentences it replaces.

### MAJ-1. F8 writes `person.due_tick` inside a re-sample, which has no world access; INV-M2-10 then breaks (90%)
- **Where:** `next/M2…toml:150-154` (due_tick "re-written by a re-sample (F8)"); F8 `:511` ("due_tick := tau'");
  INV-M2-10 `:663-665`; `crates/rh-core/src/system.rs:759-771`; `crates/rh-core/src/engine.rs:948-983`.
- **Evidence.** The frozen trait is `fn resample_timer(&self, timer: &TimerEntry, ctx: &ResampleCtx) ->
  Result<Option<SimTime>, SimError>`. It gets no `SystemView`, and the engine only re-inserts the timer at the
  returned time (`engine.rs:980-981`). The re-sample therefore cannot write due_tick. After any input that re-draws a
  birth, due_tick ≠ the timer's tick, so INV-M2-10 ("pending, at (due_tick, Vital)") fails at that instant's
  Bookkeeping.
- **Second defect.** F8 needs t_(m+G), the conception month. The timer's single i64 payload holds the father, so after
  a late re-sample to t_(m+G+1) + 1440, a second re-sample cannot recover m. Deriving it from the timer's tick gives a
  different rule from F8's text, and the reference (`resample_birth_day` takes `t_due` as an argument) does not settle
  which one is meant.
- **Fix.** Make pregnancy state that a re-sample does not change, such as `person.conceived_tick` (the conception
  pulse t_m). From it, F5 reads "pregnant" and F8 reads m. Alternatively, drop the column and use
  `StepCtx::own_timers(entity)`, which already exists (`system.rs:579`). Restate INV-M2-10 without a tick equality.
  State F8's due month as "the month of t_conc + G".

### MAJ-2. The F0 band formula is wrong (95%)
- **Where:** `next/M2…toml:503`. "A woman's band at t is i = floor(x / Y) − 15 for 15 Y ≤ x < 50 Y (i in 0..6 …)".
- **Evidence.** floor(x / Y) − 15 runs over 0..34, not 0..6. Taken literally, it indexes the seven-entry N_i and mu_i
  tables out of range, or it treats ages 15, 16, … as bands 0, 1, …. My first chain model did exactly that and gave
  TFR 0.54. The reference uses `a // BAND_MONTHS` (five-year bands), so the spec and the reference disagree on a
  normative line.
- **Fix.** i = floor((floor(x / Y) − 15) / 5), or floor(x / (5Y)) − 3.

### MAJ-3. V-SF-24 is unpassable as written: measured marital fertility ≠ M·n(a) in three bands (90%)
- **Where:** `docs/09_verification_and_validation.md:498`; `[isolation].expect` (2) `next/M2…toml:739`; F3, F4
  `:506-507`.
- **Evidence.** The exact month-by-month chain of the spec's own rules (F5's 20 closed pulses, N1 hazards; no
  mortality; `$R/tools/chain.py`) gives these ratios of births per married woman-year, by mother's age at birth, to
  M·n_i:

  | 15–19 | 20–24 | 25–29 | 30–34 | 35–39 | 40–44 | 45–49 |
  |---|---|---|---|---|---|---|
  | 0.817 | 0.957 | 1.004 | 1.009 | 1.027 | 1.132 | 1.972 |

  Four 1,000-year micro-simulation seeds agree: 0.82–0.84, 0.95–0.98, …, 1.14–1.16, 2.03–2.20. The causes are a
  9-month zero-birth lag after marriage, and banding by age at birth while the rate is set by age at conception. At
  about 22,000 births per seed, the 99.9% binomial interval is a few percent wide, so 15–19, 40–44 and 45–49 fail
  definitively.
- **The "method" column does not help.** "woman-months at risk (married, not pregnant, not within 11 months)" gives
  births per susceptible year of 0.23–0.35, which is not M·n_i either. The only exact binomial identity is
  conceptions per susceptible woman-month = c_i (chain and micro-sim: ratio 1.00 ± noise).
- **Fix.** Make V-SF-24's identity "conceptions per susceptible woman-month, by band at conception, inside the
  binomial interval of c_i". Report the Coale–Trussell shape as a soft or emergent check against the exact expected
  per-band values, which the reference can compute from the chain, not against M·n_i.

### MAJ-4. Commanded deaths as "kind 1" timers cannot carry the command and would be re-drawn as natural deaths (80%)
- **Where:** `next/M2…toml:398` (lifecycle step: "ScheduleDeath: kind 1 at (tick, Vital) with the command's cause");
  S10 `:475`; S11 `:476`; `[[rng]] death_time` `resample_on_fork = true` `:407`; `crates/rh-core/src/agenda.rs:76-90`
  (`Timer { owner, purpose, kind: u32, payload: i64 }`).
- **Evidence.** M7 sends `ScheduleDeath` with tick = "the drawn day's start" later in the month (M7 `:350`, coupling
  `:731-734`), so command timers do occur. Kind 1's payload is b, the birth tick that S11 needs. A command needs cause,
  culprit (u64) and cause_event (u64), which do not fit in one i64.
- **Consequence.** If the timer is filed under `death_time`, a material input that scopes the victim re-samples a
  scheduled murder through S11 as a hazard death: usually dropped, or kept with no cause. The same applies to Marry
  timers (kind 3): one payload, the source is lost (N7's report must name it), and N6 can drop them.
- **Fix.** Add distinct kinds, such as COMMANDED_DEATH and COMMANDED_MARRIAGE, whose payload indexes an M2-owned,
  plane-A pending-command table (hashed, in keyframes). File them under a purpose with `resample_on_fork = false`, or
  state their re-sample rule (a fixed tick is kept). Agree it with SPR-014.

### MAJ-5. `lint_next` lets a next version change frozen behaviour silently (90%)
- **Where:** `specs/systems/lint_odd.py:163-230`, `:244`; `specs/systems/next/README.md:21-35`.
- **Evidence** (planted probes in `$R/probes/`, run as `lint_odd.py --next <probe> --overlay registry_delta.toml`):
  - Dropping M7.dynasty from `[[applies]] ScheduleDeath.from` gives only a `WARN … does not list it in from` and a
    PASS. The same edit to the frozen file gives an `ERROR` and a FAIL. `lint_applies` keys strictness on the
    target's status (`:244`), and the next spec is a draft. The README's "(`from` may grow)" is not enforced.
  - Rewriting DEATH's roles and payload in `[[emits]].when` ("the deceased is actor; payload [age, cause]"): PASS.
  - Rewriting the frozen `[initialization]` ("sex female for everyone"): PASS.
  - Frozen `[[schedule]]` step text, invariant meaning (BLK-1) and algorithm text superseded by prose (S2): unchecked.
- **Fix.** In `--next` mode, apply frozen-level strictness to the superseding spec's `[[applies]]` and `[[emits]]`
  checks. Require `from` ⊇ frozen. Compare `[[emits]].when`, `[initialization]` and the frozen steps, either as
  "superset of frozen lines" or through the explicit amendment record of BLK-1. Add a self-check probe for each.

### MIN-1. The tfr default's growth claim omits widowhood: r is about −0.03 %/yr, not +0.165 %/yr (75%)
- **Where:** `next/M2…toml:517-521`, `:576`; `specs/demography/selftest_m2_f1.py:291-328`; companion F1-D4.
- **Evidence.**
  - Reference check 5 (no widowhood): NRR 1.0494. My chain without widowhood agrees: 1.0497, r = +0.163 %/yr. The
    approximation itself is fine.
  - With the husband 3–5 years older dying on the same schedule and widows re-entering at N1 hazards
    (`chain_widow.py`): NRR 0.990–0.992, r = −0.026 to −0.033 %/yr.
  - Micro-simulation (`microsim.py`, 1,000 founders, 1,000 years, seeds 1–4): r = −0.030, −0.036, −0.057,
    −0.040 %/yr; mean husband–wife gap 3.9 years; population 1,000 → 540–750.
- **Consequence.** The default is still inside V-SF-01's band [−0.1, +0.3], but only 0.06 %/yr above its floor, not
  in the middle as stated. NRR = 1 is at tfr ≈ 4.74 and mid-band (+0.1 %/yr) at ≈ 4.9.
- **Fix.** Add widowhood to check 5 (a 42-state chain), restate the facts, and give the director Q5's real numbers.

### MIN-2. `[isolation]` expect (5) is false for `household.yaml`: it dies out by construction (95%)
- **Where:** `next/M2…toml:739`; companion Q5.
- **Evidence.** H2 marries Oswin and Maren and makes the three children theirs. Every other opposite-sex pair is
  parent–child or siblings, so it is inside kin(·, 2) and excluded by N4 and N7. Every future child is again a
  sibling. No non-kin pair can ever form, so extinction is certain whatever tfr is.
- **Fix.** Say so, and either accept it as the rung-1 lesson or give the household world an unrelated founder.

### MIN-3. Same-slot birth-then-death edges (75%)
- **Where:** `next/M2…toml:60-63` (insert "at the merge of M2's level"); S12 `:477`; H4(b) `:547`; F10 `:513`;
  Day/Vital step `:395`.
- **Problem.** Within M2's timer pass, the newborn is not yet a ledger row. If its mother then dies in the same slot
  (the case F10 is designed for):
  - S12 sees no living row when she was the last, so it emits EXTINCTION. A second EXTINCTION follows when the child
    dies, contradicting "at most once per run" (`:360`).
  - H4(b) dissolves her household when she headed it alone (for example a pregnant widow). The child is then inserted
    into a dissolved household, which violates INV-M2-09, a hard invariant.
  - Separately, F10's "the birth timer has the lower seq" fails after a re-sample, because the engine re-inserts with
    a new seq (`agenda.rs:162-170`).
- **Fix.** State that M2's own scans in the same step include the rows it inserted earlier in the step, or that
  EXTINCTION and H4 count pending inserts. Restate F10 as holding absent re-samples.

### MIN-4. INV-M2-06's age bounds are wrong (90%)
- **Where:** `next/M2…toml:647-648`.
- **Evidence.** Conception is allowed at x just under 50Y, and the birth falls at t_(m+9) + 1440·j with j ≤ 29. That
  gives birth_tick(child) − birth_tick(mother) up to 50Y + 9 months + 29 days, plus one more month after a late
  re-sample. That exceeds "< 50 Y + 9 months". Also, "9 months" in minutes is undefined on the 365-day calendar.
- **Fix.** Bound by "< 50Y + (t_(m+G+1) − t_m) + 1440", or state the bound in calendar months.

### MIN-5. Clashes with the frozen T1 and M7 contracts (70%)
- **Where:** N2 and N7 `:531`, `:536`; F5 `:508`; T1 `marriage_min_age` bounds [12, 20] (`registry_v0.toml:134`);
  M7 vocation `:671` ("holy_orders … M2's fertility reads it").
- **Problem.** N7 enforces 18 and 15 "for every path", so T1's frozen parameter is inert below those ages and T1
  proposals are dropped and reported. F5 ignores vocation, yet M7 draws vocations at majority_age (up to 21), after
  many women have married. A married woman in holy orders keeps conceiving, against M7's frozen statement.
- **Fix.** Either apply M2's minimum ages to M2's own timers only and the sender's rule to commands, or put one shared
  minimum in the Registry. Exclude holy_orders from F5.

### MIN-6. H2 contradicts itself about servants (90%)
- **Where:** `next/M2…toml:545`.
- **Problem.** The rule gives the household's "children and servants" person.household = it. The worked example says
  "the 30 servants and the 14 others each their own household".
- **Fix.** Pick one. The rule as written (servants are members) matches ruling 10's manifest semantics.

### MIN-7. SPR-016 seams not left clean (75%)
- **Newborns are not exposed to mortality before their first Month pulse.** A child born on day j is first drawn at
  age (30 − j) days, and the frozen S8 hazard starts at that age. At rung 0 with e0 = 30, that skips 1.2 points of
  q0 = 0.283, about 4% of infant deaths. With SPR-016's neonatal-weighted infant schedule it skips a large share.
  Reserve the rule now (for example a draw at the birth instant for the rest of the birth month, a new purpose), or
  list it in not_yet for SPR-016.
- **One next file per id.** `next/` holds one draft per system id (`lint_odd.py`: "already has a next-version spec"),
  and only a frozen spec takes a next version. SPR-016's mortality draft therefore cannot be linted until 0.3.0
  freezes. Say so in `next/README.md` and in the companion §4.
- **S2 superseded by prose** (BLK-1): SPR-016 will need the same supersession again.

### MIN-8. The husband choice weakens CRN across branches (65%)
- **Where:** N4 `:533`; ADR-0026 consequences; V-DET-17.
- **Problem.** "Uniform index into the candidate list" means one extra or missing man anywhere in the pool reshuffles
  every later woman's husband in that branch. That spreads into widowhood timing and conception eligibility.
- **Fix.** Choose h = argmin over candidates of H(world_seed, w, h, t_m) (or the keyed stream of (w, t_m) applied
  per candidate). Then a match changes only if the changed man was the winner.

### MIN-9. Governance: the G1b gate list is edited before the director's Q6 ruling (70%)
- **Where:** `docs/09_verification_and_validation.md:514`; companion Q6.
- **Problem.** The V-SF-24 and V-SF-25 rows are the Architect's. Moving them into "Gate from G1b" touches the D34
  gate lists, and the companion itself routes that to the director.
- **Fix.** Keep the rows and revert line 514 until the ruling, or mark it "proposed (Q6)".

### MIN-10. Demographic plausibility: M = 0.50 implies birth intervals of about 52 months (60%)
- **Where:** F3 facts `:522-524`; R12 §9 (Barrett: fecundability 0.2, 11 months postpartum).
- **Problem.** Combining the early, near-universal pattern (TFR1 = 9.39) with e0 = 30 forces marital fertility to
  half of natural fertility. Peak-age intervals become about 52 months, where natural-fertility populations show
  roughly 30. Siblings come out widely spaced, which bears on heir spacing.
- **Alternative.** The Hajnal pattern that R09 §2 actually describes gives a lower TFR1, a higher M and more
  plausible intervals.
- **Fix.** Put this trade-off into Q5.

### NIT-1. F4's den_i
`:507` says den_i is u64 but also tests "den_i ≤ 0". The branch is unreachable within the bounds: 20·b ≤ 7.70e8 <
7.86e8. Use i64, or delete the branch.

### NIT-2. Id edge cases
- PersonId 0 is the "none" sentinel but is not reserved in `PersonIdTable` (`crates/rh-ids/src/person.rs:143-160`).
- `RehashExhausted` (after 64 rehashes) is not specified in F9.
- A household-id hash collision is called "a load error" (`:167`), which means nothing for a runtime marriage.

### NIT-3. TFR1 is defined by pointing at the Python
F3 (`:506`) should state the chain: married share at the start of each month, S(a+1) = 1 − Π(1 − mu). The
after-the-draw convention gives 9.4289, which is a different constant.

### NIT-4. Constants and fixtures can drift
- The reference hard-codes N_i, the nuptiality values, G, P and 105/205. A change to the step text alone is not caught.
  Read them from the spec, or carry them as data.
- The two 743-line fixture copies (`tests/lint_fixtures/odd_next/*/`) will drift. Render them with sed in
  `check_all.sh`, as `m2_f1_stale_control` does.

### NIT-5. The sex draw is keyed by the birth instant
`birth_sex` uses t = the birth instant (`:435-441`). A re-sampled birth day changes the child's sex across branches.
Keying it by the child's id alone (t = 0) keeps "the same person" the same sex.

### NIT-6. Citations
- R09 §2 B summarises only the European pattern, not the "non-European pattern" N1 cites it for.
- From memory, the n(a) table usually reproduced (Coale & Trussell 1978) has 0.396 and 0.321 at 30–34 and 35–39,
  where the spec has 0.395 and 0.322. I could not verify this. Q1 should close before freezing, because changing a
  value moves every check value.

### NIT-7. Routing and stale text
- Q5 goes to the director although Registry defaults are the Architect's.
- `docs/09:498` cites the `next/` path, which moves at freeze.
- Manifest comments are stale: hamlet "tfr dropped: no births in P1a"; household "not yet married".
- INV-M2-11's scope over commanded Marry timers is unclear.

## Sound

- **Check values reproduce independently.** Reviewer's own `fractions` code, `$R/tools/recompute.py`: mu bits
  [77309411, 150323855, 103079215, 47244640, 21474836, 8589935, 8589935]; TFR1 = 9.393524 → 615614; T(4.7) = 308019;
  m = 32791; c_i = [111984581, 133644237, 120491498, 105483713, 78832946, 34745617, 4385759]; SEX_M = 2199861298.
- **Widths.** Numerator 58 bits ≤ 60, denominator 30 bits. At tfr 7.5 × 1.6, c(20–24) saturates at 2^32 as stated.
  Monotone in tfr and E.
- **`selftest_m2_f1.py` passes.** Its planted control (`tfr1_bits = 615615`) fails for the right reason
  ("tfr1_bits: spec 615615 != reference 615614"). Apart from MAJ-2 and NIT-3, its arithmetic matches F3, F4, F6, N1,
  N4 and F8 as written.
- **Lint and gates.**
  - `lint_odd.py`: PASS (6 specs). `--next`: PASS with "4 planted changes rejected, the clean next version kept".
  - Fixtures fail for their planted reasons: `algorithm_changed` with "frozen [algorithm.mortality_rung0] changed";
    `rng_prefix` with "must stay at position 0"; `overlay_moves_bound` with "replacement changes or drops parameter tfr
    bounds".
  - All six gates sit in part core (`check_all.sh:777-780`, `:857-858`, between `part core` at 666 and
    `part golden` at 866).
  - `test_check_all_inventory.py`: 12 passed (venv). ruff, mypy, `gen_manifest --check` and `gen_skeletons --check`
    pass. `--next --rust` generates a skeleton with death_time still at discriminant 1.
- **Determinism.**
  - Every probability is a Prob32 with rhe rounding stated. There is exactly one k = 0 draw per eligible person, per
    purpose, per month.
  - Purposes and k are distinct per use. Re-sample draws use `resample_entity_id`, so they cannot collide with
    monthly draws.
  - Every timer is strictly forward. F8's c = 0 fallback satisfies the engine's (time, phase) > (t_in, Inputs) check.
  - `resample_on_fork` is set on all three vital timers. `birth_sex` is correctly not a timer.
  - No floats. No order-dependent iteration: lists are ascending person.id, and fires run in agenda-key order within
    one job (`engine.rs:1083-1087`).
- **Ids.** H64(mother, parity) is `rh_ids::child_base`. The rehash is `PersonIdTable::issue`, the same rule as
  founder() (ruling 10): a BTreeSet over every id ever issued, rebuildable from `life_course.person_id`, so it is
  deterministic. Counting genesis children in parity matches rh-ids' "children the mother bore before".
- **Closed period.**
  - F5's `last_birth_tick < t_(m−11)` gives exactly 20 closed pulses after a conception (checked at day level), so
    12c/(1 + 20c) = M·n_i holds.
  - Micro-sim: conceptions per susceptible month / c_i = 1.00 ± noise.
  - Cohort TFR without mortality = 4.743 for tfr 4.7 (+0.9%), so tfr's meaning is approximately as stated.
  - The 9 + 11 months are plausible (Barrett, R12 §9).
- **Kinship.**
  - Every birth writes life_course person_id, mother, father, sex and birth_tick at entry (REV-P1a NEW-7). The log is
    append-only and written only by M2 in Vital, so M7's and T1's `same_instant` reads are exact (NEW-6).
  - M2's reads of T1 and M7 are `lagged`, which avoids cycles. kin(w, 2) matches the effect language's
    "parent-or-child steps".
  - Rows enter the ledger only through the core insert, and exit fields are written once (ruling 5). The birth path
    names SPR-014's insert, which PRJ-002's SPR-014 row owns.
- **V-SF-25 is testable and passes** in the micro-sim: mean age at first marriage of sim-born women 18.7–18.8; every
  born row has a father.
- **Governance otherwise clean.** No ADR or docs/03 change. Verbs BIRTH and MARRIAGE and the defaults are
  Architect-level. The S6 `household.head` warning is disclosed (F1-D9).

## Not run

- The engine and any Rust build or test: no M2 crate exists yet, and the commit changes no Rust.
- The full `bin/check_all.sh` (it needs valgrind and iai-callgrind-runner); I ran the new gates and the Python checks
  individually.
- The micro-simulation and chains are my own float models of the spec text, not the engine's integer arithmetic. They
  are good to about ±0.02 %/yr on r, with 4 seeds × 1,000 years. I did not run a 365-day calendar, the hamlet world,
  or T1 and M7 coupled.
- I could not check the Coale & Trussell (1974) paper (not in `research/`), the sex-ratio source, or Hajnal (1965).
