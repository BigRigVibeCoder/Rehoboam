---
id: REV-SPR-015
title: "Independent review of the M2 F1 births spec (PR #50)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-015, SPR-016, EVO-001, REV-EVO-001, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF: CLOSED.** The M2 F1 births spec (PR #50) is ready to freeze as `specs/systems/next/M2.demography.odd.toml` 0.3.0. It took three review rounds, all in sessions on a different model from the drafter's:
>
> | Round | Head | Findings | Architect rulings |
> |---|---|---|---|
> | 1 | `421f2e1` | 1 blocker, 5 majors, 10 minors, 7 nits | R1–R11 (§0) |
> | 2 | `35226c0`, plus the `13fbb70` delta | 0 blockers, 4 majors, 7 minors, 7 nits | R12–R18 (§2.0) |
> | 3 | `f9aca53` | 0 blockers, 0 majors, 3 minors, 6 nits | R19–R22 (§3.0) |
>
> - **The merge head.** The Architect checked the fixes on `d6dbdb9` and `3a87d8f`, then the merge head `5732e3d` (§4).
> - **Commanded outcomes.** Commanded deaths and marriages follow REV-EVO-001 R9, shape (b): M2 holds no pending commanded outcomes.
> - **For the director.** Two questions go as D-items: D40, the `tfr` default of 4.9 with the early marriage pattern; and D41, V-SF-24 and V-SF-25 reported at G1a and gating from G1b.

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


## 2. Round 2 (2026-10-17, `35226c0` and `13fbb70`)

### 2.0 Architect rulings, round 2

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R12 | R5 and the review's shape recommendation | **Shape (b)** (REV-EVO-001 R9). M2 holds no pending commanded outcome: no `pending.*` table, no `COMMANDED_*` kinds, no `commanded` purpose. A `ScheduleDeath` or `Marry` reaches M2 at its outcome's instant, submitted by the sender's timer, and M2 applies it in its `Day/Vital` drain of that instant. INV-M2-13 (pending rows) goes. `tick` leaves the payloads (M2 0.2.1 via EVO-001). Switch your one paragraph | 7.5/10 |
| R13 | MAJ-2 | Changes to frozen S12 go through `[[amends]]` records. A commanded death applied by M2 cancels the victim's own pending `death_time` timer in the same step, and a pending commanded death whose victim died first is dropped by the "living key" rule (REV-EVO-001 R1), counted. There is no double death and no `SimError` | 8/10 |
| R14 | MAJ-3 | Moot under R12. If any pending-state invariant remains, it must hold when a victim dies naturally before the commanded day | — |
| R15 | MAJ-1 | Accepted. `--next` refuses duplicate keys among invariants, `[[emits]]` and `[[applies]]`, and compares frozen `when`, `[initialization]` and step texts line by line, so a prefixed negation is a changed line and needs `[[amends]]`. Each bypass the reviewer planted becomes a probe that must FAIL | 9/10 |
| R16 | MAJ-4 | Accepted. The husband choice goes through `StepCtx`'s keyed RNG under a declared purpose (for example `husband_pick`), so the branch salt and the epoch are in the key. It stays an argmin over the filtered candidates, so it stays CRN-stable. `resample_on_fork` follows `05.1` §6 item 9 | 8/10 |
| R17 | MIN items | (1) A death before a birth in one slot must not emit EXTINCTION while a newborn enters in that slot. (2) The V-SF-24 report values come from the chain with mortality (45–49: 2.02). (3) The isolation expectations report V-SF-24 and V-SF-25 at G1a and do not gate them (Q6 is the director's). (4) An amendment to a computational step bumps that table's version. (5) `household.yaml` dies out in every replica: state extinction as the rung-1 expectation and test the distribution of extinction times, not persistence (7/10). (6) The two-settlement forcing runs for every generation. (7) A marriage commanded by T1 may join two settlements; the household spans both in P1a, which is documented as a P1a limitation that S3 F1's moves (P1b) lift (6/10) | 7/10 |
| R18 | Q5 and Q6 | After round 3, these go to the director as D-items with the final numbers: tfr 4.9, r = +0.114 %/yr; the early marriage pattern and the 50-month birth interval; V-SF-24 and V-SF-25 reported at G1a and gating from G1b | — |

### 2.1 The review (round 2, independent reviewer)

Reviewer: independent and adversarial, on a different model from the drafter's. Copy reviewed: `$R/src`, a git archive of
`35226c0`. The delta was reviewed in `$R/src13` (a git archive of `13fbb70`, §5).
`$R` = `/tmp/claude-0/-home-user-Rehoboam/78f410bc-aa41-55ce-883d-f1b2ebd2cbcd/scratchpad/births_review_r2`.
Line numbers refer to `$R/src`. Only `$R/tools` and `$R/probes` were written; the repository was not touched.

### BLUF

**Not ready to freeze, but close: 0 blockers, 4 majors, 7 minors, 7 nits** (the minors and nits include 2 and 1 from
the `13fbb70` delta).

**What is now right:**
- The arithmetic reproduces independently.
- R2 (pregnancy state), R3 (bands), R7 (widowhood NRR) and R8 (Wulf) are correctly applied. NRR = 1 at tfr ≈ 4.73–4.74,
  and the default 4.9 gives r = +0.114 %/yr. My day-resolution cohort Monte Carlo gives +0.116 %/yr, and three
  1,000-founder micro-simulations give +0.09, +0.15 and +0.14 %/yr.
- V-SF-24's new identity is exact by construction. Measured over 3.9 M simulated conceptions, the ratio to c_i is
  1.000 ± 0.004.

**What still fails as a contract:**
1. **MAJ-1.** `lint_odd --next` can still be bypassed. A duplicate `[[invariants]]`, `[[emits]]` or `[[applies]]` entry
   shadows the frozen one, so a frozen statement or a dropped sender gets through with no `[[amends]]` record (three
   probes PASS). Prefixing a frozen `when` with a negation also passes.
2. **MAJ-2.** The draft again changes a frozen algorithm sentence (S12) from other tables (F10, L2). It also leaves M2's
   own death timer undefined when the person has already died by command. That is a double-death or crash path.
3. **MAJ-3.** INV-M2-13 contradicts L3. It fails, fatally, in a legitimate history: a victim who dies naturally before
   their commanded tick.
4. **MAJ-4.** The argmin husband hash is uniform and CRN-stable, but it is keyed outside the RNG rule: no branch salt,
   no epoch, no declared purpose.

**Shape question for the Architect.** Shape (a) re-samples a commanded death to its own tick. That keeps ADR-0025
item 3's mechanics and defeats its substance, for the only case that actually uses the pending table (M7's background
murders). **Recommendation: (b), 65%.**

Confidence in the overall verdict: 80%.

### 1. Architect rulings R1–R11: status

| # | Status | Evidence / what is missing |
|---|---|---|
| R1 | **PARTLY APPLIED** | The `[[amends]]` mechanism works for its five path kinds. `frozen` is checked byte for byte: probe `amend_frozen_trailing_space` FAILs with "`frozen` is not the frozen text byte for byte", and `amend_record_removed` FAILs. The draft uses it to scope INV-M2-03, S2 (with `[algorithm.parameters_f1]`), EXTINCTION's `when` and `[initialization]`, and the lint prints four AMEND lines. **Missing:** F10 and L2 still change the meaning of frozen S12 from new tables, with no record (MAJ-2), which is exactly the BLK-1 pattern. An amendment can also rewrite a *computational* frozen step without bumping the table's `version` (MIN-4) |
| R2 | **APPLIED** | `person.conceived_tick` (`:155-160`); BIRTH_TIMER payload = t_m (F7 `:595`); F8 derives the due month from the payload and writes nothing (`:596`); INV-M2-10 restated without a tick equality (`:782`). `resample_timer` has no world access: F8, S11 and N6 need only the payload, t_in and the calendar, and L4 returns the timer's own tick. The reference's `resample_birth_day` (`selftest_m2_f1.py:217`) takes only payload-derived bounds. Check 6 passes, including the repeated late re-sample (`end + DAY`) |
| R3 | **APPLIED** | F0 `:588` reads i = floor((floor(x/Y) − 15)/5). The reference's `bands_cover` (`:285-291`) pins 0..6 over [15Y, 50Y) on both calendars |
| R4 | **PARTLY APPLIED** | The identity is now conceptions per susceptible woman-month = c_i by band at conception (docs/09 V-SF-24). It is **exact**: my cohort Monte Carlo gives 0.993–1.004 per band over 3.86 M conceptions; the micro-sim gives 0.94–1.03 (small 45–49 counts). **Missing:** the "exact per-band values" of the report come from a no-mortality chain banded by month index, not from the quantity measured (MIN-2). The observable is not stated: conceptions are not in channel A. The elite multiplier's mixture is not stratified (MIN-2) |
| R5 | **PARTLY APPLIED** | Binding parts done: a commanded outcome is never under a `resample_on_fork = true` purpose (`commanded`, `:522-527`). Cause, culprit, `cause_event` and sender live in `pending.*` (`:244-299`): plane A, so in `state_hash` and keyframes. Determinism holds (seq = the queue's merge counter, hashed in `rh-core/engine.meta`). **Missing:** INV-M2-13 is unsound (MAJ-3); M2's own DEATH_TIMER after a commanded death is unspecified (MAJ-2); the shape still awaits EVO-001 R6 (§4) |
| R6 | **PARTLY APPLIED** | `from` ⊇ frozen and frozen-level strictness are applied: probe `r1_from` FAILs twice. The four round-1 silent-change probes now FAIL (`r1_emits`, `r1_init`, `r1_sched`, `r1_from`). **Missing:** duplicate-key shadowing and prefix/suffix contradictions pass (MAJ-1). There is no self-check probe for frozen schedule steps or duplicates, and the `[initialization]` probe passes only through the `to` fallback (`lint_odd.py:672`, `:700`; NIT-1). Frozen `[[schedule]] when`, `[[rng]] use` and `[[owns]] meaning` are unchecked (MIN-4) |
| R7 | **APPLIED** | Check 5 has the 42-state widowhood chain: NRR 1.0341, r = +0.114 %/yr, NRR = 1 at 4.741. Independent reproductions: round 1's `chain_widow.py` re-run gives 4.9 / gap 3: NRR 1.0352, +0.117 %/yr, with the root at ≈ 4.736. My `cohort_mc.py` (10^6 women, day resolution) gives NRR 1.0349, +0.116 %/yr, and NRR 1.0016 at 4.741, so the root is ≈ 4.733. The micro-sim (1,000 founders, 1,000 y, seeds 1–3) gives +0.092, +0.152 and +0.135 %/yr, with an emergent husband–wife gap of 3.7–3.8 y and the population at 2.1–3.6× its start. The no-widowhood claim "+0.31 %" reproduces (+0.304 %), but the reference does not compute it (NIT-2). The default moved to 4.9 ahead of Q5: acceptable as a proposal |
| R8 | **APPLIED** | `household.yaml` validates (0 errors). Wulf (19, own household) makes a marriage possible in 81 % of 5,000 replicas (`household_mc.py`). But P(alive) is 0.39 at 100 y, 0.033 at 200 y and **0/5,000 at 1,000 y**. "Persists or dies out by chance" is true only on the decade scale (MIN-5) |
| R9 | **PARTLY APPLIED** | MIN-4, MIN-5, MIN-6, MIN-7, MIN-10 and NIT-1 to NIT-3, NIT-6 and NIT-7 are done. **MIN-3 is partial:** if the death fires before the birth (possible after a re-sample), the amended text still emits EXTINCTION before the birth (MIN-1). **MIN-8:** the argmin is uniform and CRN-stable but unsalted (MAJ-4). **NIT-4:** the `constants` tables are read, but the prose numbers are unchecked (NIT-2). **NIT-5:** applied (birth_sex by child; NIT-4 below on wording) |
| R10 | **APPLIED** | The G1b list is reverted and both rows are marked "proposed (Q6)". But `[isolation].expect` (2)–(3) gates V-SF-24/25 through V-SF-22 at G1a (MIN-3) |
| R11 | **APPLIED** | Q1 is cited-pending (F1 `:589`), Q2 follows SPR-014, Q3 is H1 `household/v1`, and Q5/Q6/Q7 are routed. Stale: `:208` still calls the marriage LIFE_INTERVAL format "SPR-015's format decision (Q4)", which R11 deferred to P1.11 (NIT-5) |

### 2. Re-run of round 1's reproductions

- **Silent-change probes**, re-planted on the round-2 text (`$R/tools/probes.py`, `$R/probes/r1_*`): `from`, `emits`
  (DEATH roles/payload), `init` ("sex female for everyone") and `sched` (the mortality step dropped) all **FAIL** now,
  each with its own diagnostic.
- **INV-M2-03:** amended by record (`:870-874`). The `frozen` text matches the 0.2.0 file byte for byte, and `to`
  matches `:753`. Its new clause "a COMMANDED_* timer at its sender's tick … not Month pulse instants" is not enforced
  (MAJ-3).
- **`resample_timer` without world access:** confirmed (R2 row).
- **Band formula:** confirmed (R3 row).
- **V-SF-24.** The identity is exact (R4 row). Round 1's `chain.py` at 4.9 gives births per married woman-year / M n
  of 0.821, 0.959, 1.004, 1.009, 1.027, 1.131, 1.971; the spec pins 0.85 … 1.87 (MIN-2).
- **Widowhood NRR:** confirmed (R7 row).
- **`household.yaml`:** a marriage is possible (R8 row).

### 3. New findings

#### MAJ-1. `lint_odd --next` can still be bypassed without an `[[amends]]` record (confidence 90%)
- **Where:** `specs/systems/lint_odd.py:273` (`invs = {id: …}`), `:283` (`applies = {command: …}`), `:300` (`whens =
  {kind: …}`), `:305` and `:308` (substring tests), `:335` (`lint_applies` also keeps the last duplicate).
- **Evidence** (`$R/tools/probes.py`, `probes_shadow.py`; each probe is the round-2 spec with one edit, run as
  `lint_odd.py --next <dir> --overlay registry_delta.toml`):

  | Probe | Edit | Result |
  |---|---|---|
  | `inv02_shadow` | a second `INV-M2-02` with a changed statement, placed before the frozen one | **PASS** |
  | `emits_death_shadow` | a changed `DEATH` `[[emits]]` placed before the frozen one (EXTINCTION first, so the self-check's index-0 plant still trips) | **PASS** |
  | `applies_sd_shadow` | a `ScheduleDeath` `[[applies]]` without `M7.dynasty`, with `drained_by = "Month/Polity"`, placed before the frozen-`from` copy | **PASS** |
  | `emits_prefix_negation` | `when = "NEVER (the text below is void): " + frozen text` | **PASS** |
  | `emits_suffix_contradiction` | frozen text + ". F1 override: the deceased is actor and the payload is [age]" | **PASS** |
  | `sched_superset_contradiction` | a second Month/Vital `apply_death:` step "deaths fire at the Month pulse instant" | **PASS** |

  Dicts keep the last duplicate, so a changed copy placed first is invisible to both `lint_next` and `lint_applies`.
  The same probes with the changed copy at index 0 crash the self-check with an `AssertionError`. That is a FAIL, but
  only by accident.
- **Fix.**
  - Reject duplicate invariant ids, emits kinds, applies commands, owned columns, param names and RNG purposes in
    every spec, in both modes.
  - Now that `[[amends]]` exists, compare a frozen `when` and `[initialization]` for **equality or amendment**. If the
    superset rule stays, require the frozen text as a prefix.
  - Add self-check probes for a duplicate and for a dropped schedule step.

#### MAJ-2. Frozen S12 is changed from other tables, and M2's own DEATH_TIMER after a commanded death is undefined (85%)
- **Where:**
  - S12 `:563` (frozen: "emit DEATH (target = person; no actor …); then, if no ledger row has alive = 1, emit EXTINCTION").
  - F10 `:598` ("S12's EXTINCTION test … include[s] the newborns of the slot").
  - L2 `:664` ("the DEATH event's actor = the culprit … its cause edge = cause_event").
  - H4(c) `:652`; L3 `:665`.
- **Problem 1: the R1 rule is broken again.** F10 and L2 change what frozen S12 does, with no `[[amends]]` record for
  S12. The lint cannot see it.
- **Problem 2: a crash path.** F9 drops a birth whose mother is absent, N7 drops a marriage, and L3 drops a commanded
  death whose victim is dead. Nothing says what S12 does when **M2's own DEATH_TIMER** fires for a person already dead
  by command:
  - **On an earlier day:** the row is compacted, so S12 writes to an absent row.
  - **In the same slot after a re-sample** (`agenda.rs` re-inserts with a new seq): the row has alive = 0, and S12's
    second life-course exit write is refused by rh-core (ruling 5). That is a `SimError`.
- **The case is reachable.** A T1 command dated a Month pulse instant is applied in the Day/Vital drain *after*
  Month/Vital has drawn the victim's month. An M7 murder on day d can follow a natural death timer drawn for a later
  day. The probability is about q_month (0.2–0.4 % at adult ages) per commanded death: rare, but certain across a
  nightly ensemble.
- **Fix.** One `[[amends]]` of S12 that states:
  1. a death timer whose person is absent, or has alive = 0, is dropped and counted (or apply_death cancels the
     person's pending DEATH_TIMER through `StepCtx::own_timers`, `system.rs:579`, and `cancel`);
  2. a commanded death's actor and cause edge;
  3. the newborn-aware EXTINCTION test.

#### MAJ-3. INV-M2-13 is unsound (85%)
- **Where:** INV-M2-13 `:794`; L1–L3 `:663-665`; INV-M2-03 as amended `:753`.
- **Problem 1: the dead-key-person clause.** INV-M2-13 says "no row names a key person already dead at Bookkeeping
  (the fire or the drain removed it)". But L3 keeps a pending row until its timer fires, and drops the command only
  then ("a victim already dead at the fire (a natural death came first)"). Take an M7 murder held for day 20 whose
  ruler dies naturally on day 9: the row names a dead person at Bookkeeping of day 9. INV-M2-13 fails, and invariant
  failures are fatal.
- **Problem 2: unenforced tick claims.** "pending.tick is a day start, not a Month pulse instant" is asserted, and
  repeated in the INV-M2-03 amendment. L1 and L2 never check it. T1 dates its commands "the next day start", which is
  a Month pulse instant on the 1st. Those commands are applied at the drain today, never held, but any future sender
  that dates a held command on the 1st, or off a day start, triggers a fatal invariant.
- **Fix.**
  - H4 gains (e): a dead key person's pending rows are removed, their COMMANDED timers cancelled, and the drop
    counted. Alternatively, delete the clause.
  - L1 drops and reports a tick that is not a day start (INV-M2-05's rule).
  - L2 either drops a held tick that is a Month pulse instant, or accepts it and removes the claim from INV-M2-13 and
    from the INV-M2-03 amendment. Firing at a pulse instant is harmless: the timer pass precedes the pulse steps.

#### MAJ-4. The husband hash is keyed outside the RNG rule (80%)
- **Where:** N4 `:638`. The husband is h = argmin of `h64_words(tag(husband/v1), [world_seed, w, h, t_m])`.
- **What is sound** (`$R/tools/argmin_hash.py`, a Python port of rh-ids' SplitMix64):
  - **Uniform:** 200k draws over 8 candidates give χ² = 5.0 with ids 1..8 and 3.8 with ids k·2^40 (7 df; the 99.9 %
    critical value is 24.3).
  - **CRN-stable:** 0 of 50,000 non-winner removals changed the match.
  - The ruling's MIN-8 intent is met.
- **What is not.** The CLAUDE.md non-negotiable is H(world_seed, branch_salt(t), system_id, epoch, purpose, entity, t,
  k). This hash has no branch salt, no epoch and no declared `[[rng]]` purpose, so registration cannot check it
  (05.1 §6 item 9). Consequences:
  - "reroll fate" forks and Foresight rollouts re-draw the k = 0 hit and the day, but rank the husbands identically in
    every branch;
  - a material input that scopes the bride (epoch + 1) cannot change her ranking.
- **Fix.** Rank candidates by draw k = h of a keyed stream: a new purpose `husband`, appended after `commanded` so no
  discriminant moves; key (person, month); entity w, t = t_m, her epoch. `RngStream::draw(t, k)` takes a u64 k, and
  `LightStream` is the declared scalar option (`rng.rs:8`). CRN stability is unchanged, because the ranking is still
  per candidate.

#### MIN-1. MIN-3 is only half fixed: death-before-birth in one slot still gives EXTINCTION, then a child (75%)
- **Where:** EXTINCTION `when` `:441` and its amendment `:883`; F9 `:597`; F10 `:598`.
- **Problem.** F10 admits that after a re-sample the death timer can fire first. S12's test then sees no living row
  and no newborn yet, and emits EXTINCTION. F9 then bears the child, because the mother is "dead in this very slot".
  The amended text's reason, "a pending birth of a dead mother is dropped", is false for a same-slot death. So the
  world is extinct and inhabited, and a second EXTINCTION follows later.
- **A smaller contradiction.** F9's re-opening of a dissolved household rewrites `dissolved_tick`, against INV-M2-09's
  "written once".
- **Fix.** Count pending BIRTH_TIMERs of this slot whose mother's row is present in the EXTINCTION test, or test
  EXTINCTION once at the end of M2's timer pass. Restate INV-M2-09 to allow F9's same-instant re-open.

#### MIN-2. V-SF-24's report values are not the measured quantity, and its observable is unstated (80%)
- **Where:** `report_ratios` `:603`; facts `:617-620`; `[isolation].expect` (2) `:861`; reference check 7
  (`selftest_m2_f1.py:484-501`); `docs/09` V-SF-24.
- **Evidence.** Births per married woman-year by mother's age at birth / M n_i, at tfr 4.9:

  | Source | 15–19 | 20–24 | 25–29 | 30–34 | 35–39 | 40–44 | 45–49 |
  |---|---|---|---|---|---|---|---|
  | spec (reference chain, no mortality, banded by month index) | 0.85 | 0.97 | 1.00 | 1.01 | 1.02 | 1.12 | 1.87 |
  | round 1 `chain.py` (age at birth = pulse age + ½ month) | 0.821 | 0.959 | 1.004 | 1.009 | 1.027 | 1.131 | 1.971 |
  | my `cohort_mc.py`, no mortality, exact age at the birth instant | 0.832 | 0.963 | 1.004 | 1.009 | 1.026 | 1.122 | 1.923 |
  | my `cohort_mc.py`, e0 = 30, widowhood (gap 3) — what `[isolation]` (1)–(2) measures | 0.837 | 0.965 | 1.010 | 1.015 | 1.035 | 1.149 | 2.019 |
  | micro-sim, 1,000 founders, seeds 1–3 | 0.83–0.85 | 0.95–0.97 | 1.01 | 1.02 | 1.02–1.06 | 1.16–1.17 | 1.92–2.04 |

  The 40–44 and 45–49 values are off by about +2.6 % and +8 % in the population that is actually measured. With a
  few hundred births per seed in 45–49, single seeds mostly pass. Pooled over 32 seeds, both bands fall outside a
  99.9 % interval.
- **Observable.** Conceptions are not in channel A. A pregnancy lost with its mother leaves no trace, so a log-based
  count is biased: about 1 % fewer conceptions, and spurious "susceptible" months. Under the isolation's
  10 %-at-×1.3 elite forcing, the per-band rate is a mixture of c_i and c_i^e.
- **Fix.**
  - Generate the report values with mortality and widowhood at day resolution, or report them with a tolerance
    (±5 %; ±15 % at 45–49).
  - Delete "within their binomial intervals" from expect (2).
  - State that V-SF-24 is measured by a plane-A observer of the fertility step (or of `conceived_tick` per pulse),
    stratified by table.

#### MIN-3. `[isolation]` gates V-SF-24/25 at G1a, ahead of Q6 (70%)
- **Where:** `[isolation].expect` (2) and (3) `:861`; `docs/09` V-SF-22 "Gate from G1a … Isolation: V-SF-22 (M2, …)".
- **Problem.** V-SF-22 passes only "when … its behaviour matches the spec's expect". M2's expect contains V-SF-24's
  identity and V-SF-25's bands. So they are gated at G1a through V-SF-22, while their docs/09 rows say "reported at
  G1a; gating from G1b is the Q6 proposal".
- **Fix.** Mark (2) and (3) "reported, not gated, until Q6", or say in Q6 that the director's ruling also covers this.

#### MIN-4. `[[amends]]` can rewrite arithmetic under an unchanged version, and some frozen texts are never compared (75%)
- **Problem 1: arithmetic under the same version.** Probe `amend_s9_computational` replaces S9's multiply-shift day
  draw with `d1 mod (n − 1)` through a valid record. The lint **PASSes** with one more AMEND line, and
  `[algorithm.mortality_rung0] version` stays `m2-rung0-v1`. A frozen algorithm can thus change its arithmetic, and
  move goldens, under the same name.
- **Problem 2: unchecked frozen texts.** These pass silently:
  - probe `sched_when`: a frozen binding's `when` set to "never (planted)";
  - probe `rng_use`: a frozen `[[rng]] use` rewritten;
  - probe `owns_meaning`: a frozen `[[owns]] meaning` rewritten. This draft changes seven frozen meanings this way
    (person.id, birth_tick, sex, mother, father, spouse, life_course.origin), allowed by `next/README.md`.
- **Problem 3: the amendment list is not pinned.** `check_all.sh:800` checks only the self-check line, so a fifth
  amendment added later changes no gate outcome.
- **Fix.**
  - An amended `[algorithm.*]` step requires a new table `version`, checked by the lint.
  - Add `owns.<col>.meaning` and `rng.<purpose>.use` as amendable paths (or print a MEANING note), and compare frozen
    `when`s.
  - Pin the expected AMEND paths in `check_all`.

#### MIN-5. `household.yaml` dies out almost surely within the isolation horizon (90%)
- **Evidence** (`$R/tools/household_mc.py`, 5,000 replicas, the F1 rules at e0 = 30, tfr 4.9):

  | Alive at | 50 y | 100 y | 200 y | 1,000 y |
  |---|---|---|---|---|
  | Share of replicas | 0.961 | 0.393 | 0.033 | 0 |

  At least one non-genesis marriage happens in 81 % of replicas. The median extinction year is 88.
- **Problem.** Expect (5)'s "persists or dies out by chance" is literally true. Over V-SF-22's 1,000-year horizon it
  means "dies out": fewer than 0.06 % of replicas survive.
- **Fix.** State the numbers in expect (5) and companion Q5. If persistence is the lesson, a second unrelated couple
  is needed, and a 6-person world will still die out on 1,000-year scales.

#### MIN-6 and MIN-7 are from the `13fbb70` delta (§5).

#### NITs
- **NIT-1.** The self-check (`lint_odd.py:672`, `:700`): the `[initialization]` probe is satisfied only through the
  "`to` is not what…" fallback, because that path is amended. There are no probes for schedule steps, duplicates or
  `lint_applies` strictness.
- **NIT-2.** The reference:
  - check 5's docstring says "the no-widowhood figure is printed", but the code prints a same-age-husband *widowhood*
    chain (NRR 1.0370);
  - the spec's "+0.31 % without widowhood" is not computed (my `chain.py`: +0.304 %);
  - check 7's docstring says conceptions per susceptible month = c_i is checked, but nothing is computed;
  - the prose numbers (F1's 0.411…, N1's 18…, F2's 105/205) are not compared with `constants`. Probe `ref/prose.toml`,
    with F1's prose 0.411 → 0.511, PASSes;
  - `constants` edits are caught: `natural[0] = 412` and `gestation_months = 10` both FAIL.
- **NIT-3.** `person.unborn_father` is correct and consistent with INV-M2-06 and INV-M2-10, but it is derivable from
  `conceived_tick` and the marriage log's row open at t_m. Keep it as a stated cache or drop it.
- **NIT-4.** `birth_sex`'s key "branch_salt(t) … t = 0" (`:517`) reads as the canonical salt; the engine's `SystemKey`
  uses the current branch salt. Say "the current branch salt". Keying by the child alone is otherwise sound: a new
  id has epoch 0, there is one draw per id, and it is not re-sampled.
- **NIT-5.** Stale text:
  - `:208` (Q4 is deferred to P1.11);
  - `[algorithm.parameters_f1]` P1 `:626` says "S2 unchanged there", although S2 is amended;
  - the lifecycle header `:659-660` says (a) "extends ADR-0025 item 3" to queued commands, but nothing in L1–L5 does:
    `apply_inputs` never visits the command queue (`engine.rs:922-945`).
- **NIT-6.** The `pending` entity is keyed by seq, not by an entity id, and is the first M2 table whose rows the owner
  deletes. Say how its rows are removed: the owner rewrites its columns.
- **NIT-7** is from the `13fbb70` delta (§5).

### 4. R5, ADR-0025 item 3 and DET-13: analysis and recommendation

**Who re-draws a commanded death under shape (a) when a material input scopes the victim? Nobody.**
- `Engine::apply_inputs` (`engine.rs:922-945`) cancels every pending timer about the scoped entity and calls its
  owner's `resample_timer`. For a COMMANDED_DEATH the owner is M2, and L4 returns the timer's own tick. The engine
  re-inserts it with a new agenda seq; the payload (the pending seq) and the tick are unchanged.
- The sender (M7) holds no timer about the victim and is never asked. M2 has nothing to draw from.
- So once submitted, the outcome is fixed. A Foresight "reroll fate" fork keeps it too (`resample_on_fork = false`),
  while natural deaths re-draw.

**Does an identity re-sample satisfy ADR-0025 item 3? In mechanics yes, in substance no.**
- It satisfies the mechanics: cancel, call the owner, re-insert, no back-dating.
- It contradicts the substance: item 3 makes a material input re-draw the sampled outcomes in its scope under the new
  epoch, so the input can change them.
- In P1a the only commands that are *held* are M7's background murders. T1 dates its commands at the drain instant,
  so they are applied at once. An M7 murder is a hazard draw at the month's first day, with no emitted cause, which is
  exactly what item 3 says to re-draw.
- Keeping T1's outcomes would be defensible (they follow an already-emitted successful attempt), but they never reach
  the pending table.
- So (a) contradicts item 3 in every case it actually serves. The drafter's own header (`:659-660`) concedes that
  this needs a director decision.

**Is the outcome a plane-A timer while it waits in the queue (DET-13)? No.**
- Between submit and drain, M7 at (t_m, Polity) → M2 at (t_m + 1 d, Vital), and T1 at (d, Agenda) → (d + 1, Vital),
  the outcome is an entry in `rh-core/engine.commands`.
- That is plane A, inside `state_hash` and stored in keyframes (`rh-snapshot/src/state.rs:30`, `:66`), and it is
  deterministic.
- But it is not a timer, and `apply_inputs` never visits the queue.
- This window, at most one day (A-31), exists in **both** shapes: T1 always goes through it. DET-13's letter needs the
  director's reading that a hashed queue entry counts.
- Shape (a) adds a second gap: an M2 timer that cannot be re-drawn, held for up to a month.

**Shape (b) satisfies both rules as written, at a cost.**
- In (b), M7 owns its murder timer, as it already owns its deposition timer (`exit_timer`).
- A scoped input re-draws it through M7's `resample_timer` with the new epoch, and the queue then carries an outcome
  whose timer has already fired.
- The cost is an M7 next version: a murder timer kind and a resample rule.
- There is also a placement convention. `StepCtx::schedule` refuses a timer in a phase where the system is not bound
  (`system.rs:533-551`). So M7 fires at (d − 1, Polity) and sends `{tick = d}`, which M2's drain applies on time. The
  alternative is a Vital binding for M7.
- T1 needs no change: its tick is already the drain instant.
- M2 loses 8 columns, a purpose, two kinds, INV-M2-13 and L2–L4, which removes MAJ-3 and half of MAJ-2.

**Recommendation: (b), confidence 65%.**
- (a) needs an amendment of ADR-0025 item 3 ("a commanded outcome keeps its tick on a re-sample") on top of DET-13's
  queue reading, and it fixes M7's murders against player inputs.
- (b) keeps R5's own principle, that re-sampling belongs to the system that sampled the outcome.
- If the Architect keeps (a) because it matches M7's frozen text:
  - fix MAJ-2 and MAJ-3;
  - put the item-3 amendment to the director explicitly. Today's draft claims an extension (`:659-660`) that L1–L5 do
    not implement.

### 5. The `13fbb70` delta (REV-SPR-018 R8: the husband draw stays in the bride's settlement)

`lint_odd` passes in both modes; the only new warning is "reads person.residence from S3.migration, which has no spec
yet". `selftest_m2_f1.py` PASSes.

- **Is the lag legal and deterministic? Yes.**
  - The engine snapshots every lagged column at the start of the instant (`engine.rs:815-821`), so the read is a pure
    function of the previous instant's state.
  - `lagged` is also *required*. S3's `residence_at_birth` reads M2's new rows in the same Vital slot, so a
    same-instant read by M2 would close an M2↔S3 cycle.
  - Row alignment between the lagged snapshot and the live ledger holds at a Month pulse instant. Nothing is inserted
    before M2's Month/Vital step there: births never fire at pulse instants (F6, F8, INV-M2-10), and compaction runs
    only at Bookkeeping.
  - With S3 registered, every living row has residence ≠ 0 by the next instant (S3 INV-S3-02; M1
    `settlement.id` is 1..K, and 0 is reserved).
- **Does the filter keep the argmin CRN-stable? Yes.**
  - The filter is a per-candidate predicate on residences that never change in P1a (ruling M-6).
  - An argmin over a filtered set still changes only when the added or removed man is the winner.
  - The gap fallback keeps the settlement condition (N4), so both tiers stay inside the settlement. The salt and
    epoch gap of MAJ-4 is unchanged.
- **Does the neutral reproduce the unfiltered pool? Yes, exactly.** With residence 0 for everyone, "both not 0" never
  holds. The candidate list, the hash inputs and therefore the argmin are those of `35226c0`, bit for bit. A
  registered S3 with a single settlement (the rung worlds) is also an identity filter.
- **Does a world with settlements but no S3 behave as round 2 did? Yes.** SPR-014's registration rule drops the read
  and supplies the neutral 0.

New findings from the delta:
- **MIN-6. The two-settlement isolation forcing stops testing R8 after one generation (75%).**
  - **Where:** `[isolation].forcing` `person.residence` and expect (3) (`src13` `:878`, `:881`).
  - **Problem.** The step series splits only the *founders*. Without S3, newborns keep the row insert's 0, which
    means "no restriction". From the first born generation on, a man descended from settlement 1 marries a settlement-2
    bride freely, so "no marriage pairs two settlements" becomes vacuous.
  - **Fix.** The forcing gives each newborn its mother's residence, emulating S3 F0's R4, or the check runs with S3 F0
    registered.
- **MIN-7. T1's Marry commands can still pair two settlements (65%).**
  - **Problem.** R8 states the goal "marriage must not create a household that spans two settlements". N7 has no
    settlement precondition, so a named actor's cross-settlement Marry still creates such a household through H3.
  - **Fix.** Add the N7 precondition residence(a) = residence(b) when both are non-zero, dropped and reported, or have
    the Architect accept the exception (S3 F1's co-residence).
- **NIT-7.** The coupling's `output_bounds = [0, 65535]` should match S3's `u32` SettlementId. Its `data_range` should
  read "SettlementId (M1 settlement.id, 1..K; 0 = none)", not "RegionId or SettlementId".

### 6. Checks run, and what I did not run

**Run on `35226c0`, all passing unless noted:**
- `lint_odd.py`: PASS (6 specs).
- `lint_odd.py --next … --overlay registry_delta.toml`: PASS, with 4 AMEND lines and "10 planted changes rejected,
  the clean next version kept".
- The three rendered `odd_next` controls (algorithm, rng, bound) FAIL with their own diagnostics.
- `selftest_m2_f1.py`: PASS. Its planted control (`tfr1_bits = 615615`) FAILs with "tfr1_bits: spec 615615 !=
  reference 615614".
- Manifests: all 7 examples validate as expected, and `invalid_draft_style` is INVALID.
- `gen_manifest --check` and `gen_skeletons --check` pass. `--next --rust` keeps `DeathTime = 1` and appends
  `Commanded = 5`.
- Repository-wide ruff and mypy pass.
- `test_check_all_inventory.py` (venv): 12 passed.

**My tools, all in `$R/tools`** (floats, tool side, none of them the engine):
- `probes.py` and `probes_shadow.py` (20 lint probes in `$R/probes`, plus 5 reference probes in `$R/probes/ref`);
- `cohort_mc.py` (10^6 women, day resolution);
- `household_mc.py`;
- `argmin_hash.py`;
- round 1's `chain.py`, `chain_widow.py` and `microsim.py`, re-run at 4.9. Outputs are in `mc_*.txt`,
  `microsim_49_s*.txt` and `household_mc.txt`.

**Not run:**
- The engine, any Rust build or test: no M2 crate exists, and neither commit changes Rust.
- `rh-cli` manifest parity, which needs a build.
- The full `bin/check_all.sh`, which needs valgrind and iai-callgrind-runner.
- A 365-day calendar.
- The hamlet world, and T1 or M7 coupled.
- A multi-settlement world under the `13fbb70` filter: kin exhaustion and no-candidate months in small settlements are
  unquantified.
- I could not check Coale & Trussell (1974) Table 1, which is still cited-pending.


## 3. Round 3 (2026-10-17, `f9aca53`)

### 3.0 Architect rulings, round 3
**Ready to freeze after three small fixes, R19 to R22 below.** No blockers and no majors.

| # | Finding | Ruling |
|---|---|---|
| R19 | MIN-1 | Pin the amendment paths, not the count: a committed list of each amended path with a hash of its new text, compared as an exact set. The lint refuses a new `[algorithm.*]` table that names a frozen step id unless an `[[amends]]` record covers it; if that cannot be done mechanically, it becomes a review rule in `next/README.md` |
| R20 | MIN-2 | The 45–49 report value is 2.03. State the banding exactly and recompute every band; Q6 carries the corrected values |
| R21 | MIN-3 | The settlement rule gates: an invariant that no marriage M2 draws pairs two non-zero settlements. T1's commanded marriages are exempt (the documented P1a limitation) |
| R22 | Nits | As the reviewer lists them, including the stale step text at `:424` |

### 3.1 The review (round 3, independent reviewer)

Reviewer: independent and adversarial, on a different model from the drafter's. Copy reviewed: `$R/src`, a git archive
of `f9aca53` (delta from `13fbb70`: 7 files, +404/−219). Line numbers refer to `$R/src`.
`$R` = `/tmp/claude-0/-home-user-Rehoboam/78f410bc-aa41-55ce-883d-f1b2ebd2cbcd/scratchpad/births_review_r3`.
Only `$R/tools`, `$R/probes` and this file were written; the repository was not touched.

### BLUF

**Ready to freeze after three small fixes: 0 blockers, 0 majors, 3 minors, 6 nits.**

- R12, R13, R15, R16 and R18 are applied. R17 is partly applied: R17.4 leaves a gap (MIN-1).
- Every gate passes, and every control fails as designed.
- Every R15 bypass from round 2 now fails. Of my own new attempts, two pass:
  - **an amendment swap.** S9's arithmetic is changed through a valid record, and one meaning amendment is withdrawn so
    the pinned count stays at 12. It passes both the lint and the `check_all` gate string (MIN-1).
  - **a new table that overrides a frozen step.** No syntactic lint can catch this; only review can.
- The numbers reproduce: NRR 1.035 and r = +0.114 to +0.117 %/yr at tfr 4.9, NRR = 1 at tfr 4.736 to 4.741, and the c_i
  bits match exactly.
- **Q6 needs a correction.** The age-shape report values are biased by how the chain bands births. Banded by exact age
  at birth, 45–49 is **2.03**, not 1.98 (MIN-2).
- **Foreign reads comply with the Architect's rule.** All four are `mode`, each has an explicit neutral, and none is
  `identity`.

Confidence in the verdict: 80%.

### 1. Rulings R12–R18

| # | Status | Evidence |
|---|---|---|
| R12 | **APPLIED** | The `pending.*` columns, `COMMANDED_*` kinds, the `commanded` purpose and INV-M2-13 are all gone. A grep of the spec finds none of them. The generated `RngPurpose` is DeathTime=1 … BirthSex=4, HusbandPick=5, with no Commanded. `[[applies]]` has `drained_by = "Day/Vital"` (`:376`, `:382`). L1–L2 (`:622-623`) apply every ScheduleDeath and Marry in M2's Day/Vital drain, after the slot's timer pass. `lifecycle_f1` is now `m2-f1-lifecycle-v2`, and L4 records shape (a) as not chosen. The frozen `tick` field stays in the payloads; L1 counts a mismatched tick and never holds the command (per the ruling, 0.2.1 removes it). Residue: stale step text at `:424` (NIT-1) |
| R13 | **APPLIED** | The S12 `[[amends]]` record (`:859-862`) passes the lint byte for byte, and `[algorithm.mortality_rung0] version` is now `m2-rung0-v1.1` (`:500`). An absent or `alive = 0` row is dropped and counted, so there is no double death. A commanded death cancels the victim's DEATH_TIMER through `own_timers` and `cancel`, and rh-core supports this (`system.rs:561-589`): the drain's level runs after the timer pass, and after Month/Vital at a pulse instant, so a timer drawn at the same instant is visible and cancellable. A second command for the same victim meets `alive = 0` before it can cancel, so no "cancelled twice" `SimError` occurs. L1 relies on the living-key drop, with S12 as a second guard. I found no path to a double death or a `SimError` |
| R14 | moot (confirmed) | No pending-state invariant remains |
| R15 | **APPLIED** | `lint_duplicate_keys` runs on every spec in both modes (`lint_odd.py:245`, `:442`). Frozen `when`, `[initialization]`, schedule steps, binding `when`, column `meaning` and purpose `use` are now compared by equality. The self-check has 19 probes, and `check_all.sh:800` pins its output line. **All of round 2's bypass probes, re-planted on this text, FAIL:** six duplicate shadows, the prefix negation, the suffix contradiction and the superset step, along with r1_*, sched_when, rng_use, owns_meaning, amend_owns, amend_frozen_trailing_space and amend_record_removed. Three index-0 duplicate probes fail through an `AssertionError` that names the duplicate (NIT-4). The exception is `amend_s9_computational`: it passes the lint and fails only through the count pin (MIN-1). **My new attempts:** see §2.1. The R15 rules hold; the remaining gaps lie outside them |
| R16 | **APPLIED** | `husband_pick` is the 5th purpose, appended after the frozen ones (`:471-478`). Its key is (woman, month), k = the candidate id, and `resample_on_fork = false`: no timer carries it, and the marriage timer keeps the husband (N6). N4 (`:594`) takes the least draw. **The argmin stays CRN-stable over the filtered set.** Every filter is a per-candidate predicate. The tier fallback switches only when the gap tier is empty, and then the added man is the winner. The per-step matched set cascades only through winners. Measured with the repository's ChaCha8 reference (`$R/tools/argmin_chacha.py`): χ² = 7.2 with ids 1..8 and 10.6 with random u64 ids (7 df; the 99.9 % critical value is 24.3). Removing a non-winner changed the pick in 0 of 1,600 trials, and another branch salt kept the same pick in 182 and 216 of 1,600 (≈ 1/8, so a fork re-ranks). Caveat: StepCtx keys every pulse draw at epoch 0 until BCK-002 A-29 lands (NIT-2) |
| R17 | **PARTLY APPLIED** | (1) Applied: the S12 and EXTINCTION `when` amendments count the slot's due births, and INV-M2-09 allows F9's same-slot re-open. NIT-3 covers how to implement the test. (2) Applied: `report_ratios` come from the chain with mortality and widowhood, but carry a banding bias (MIN-2). (3) Applied, but over-applied to the settlement check (MIN-3). (4) **Partly:** the lint refuses an amended step under the frozen version, and v1.1 is set. But a later amendment under v1.1 passes, and the gate pins the amendment count, not the paths (MIN-1). (5) Applied in expect (5); the comment in `household.yaml` is stale (NIT-1). (6) Applied: the forcing gives every newborn its mother's residence (`:825`). (7) Applied: N7 records a T1 cross-settlement marriage as a P1a limitation (`:597`) |
| R18 | **APPLIED (numbers confirmed)** | See §3. Q5's figures reproduce. Q6's c_i reproduce exactly, but the age shape at 45–49 should read 2.03 (MIN-2) |

### 2. New findings (round 3)

#### MIN-1. An amendment can rewrite a frozen algorithm's arithmetic under the already-bumped version, and `check_all` pins a count, not paths (80%)
- **Where:** `specs/systems/lint_odd.py:300-312` (the version rule compares against the frozen version only);
  `bin/check_all.sh:800` (expects "12 amendments recorded").
- **Probe `r3_amend_swap_s9`** (`$R/tools/probes_r3.py`). The edit:
  - replaces S9's multiply-shift day draw with `j = 1 + (d1 mod (n − 1))`, through a valid `[[amends]]` record;
  - withdraws the `owns.life_course.origin.meaning` amendment and restores the frozen meaning.
- **Result: the lint passes, and so does the `check_all` gate string.** The count is still 12 and the table is still
  `m2-rung0-v1.1`, whose comment claims "S0-S11 … unchanged". The golden vectors could move under an unchanged name.
  Round 2's plain `amend_s9_computational` (13 amendments) is caught only by the count.
- **Fix (pick one):**
  - The lint prints a digest of the sorted AMEND paths, and `check_all` pins that digest.
  - Each amended table declares `amended_steps = [...]`; the lint checks the list against the records, and requires the
    version to move whenever the list changes.

#### MIN-2. V-SF-24's `report_ratios` carry a half-month banding bias; Q6 should read 2.03 at 45–49, not 1.98 (85%)
- **Where:** `report_ratios` `:555`; `selftest_m2_f1.py:496-504` (`band_ratios`); docs/09 V-SF-24 `:498`; Q6.
- **Cause.** The chain bands month a's births by month index. A mother born mid-month gives birth at a mean age half a
  month later, so half of each boundary month's births belong to the next band.
- **Test** (`$R/tools/shape_shift.py`): the drafter's own Chain, with each month's births split half and half across the
  boundary, gives the "exact age" row below. It matches my day-resolution cohort Monte Carlo, which bands by exact age
  at the birth instant, to within 0.006.

  | Source | 15–19 | 20–24 | 25–29 | 30–34 | 35–39 | 40–44 | 45–49 |
  |---|---|---|---|---|---|---|---|
  | spec (banded by month index) | 0.85 | 0.97 | 1.01 | 1.02 | 1.03 | 1.14 | 1.98 |
  | drafter's chain, banded by exact age at birth | 0.837 | 0.965 | 1.009 | 1.017 | 1.036 | 1.148 | 2.034 |
  | my cohort MC, 10^6 women, gap 3, seed 7 (seed 1) | 0.837 | 0.966 | 1.009 | 1.017 | 1.036 | 1.148 | 2.040 (2.019) |

- **Impact.** Nothing fails, because the gaps are inside the 5 % and 15 % tolerances. But the numbers the director sees
  are biased: +1.5 % at 15–19 and −2.8 % at 45–49.
- **Fix:** split each month's births in `band_ratios`, then pin 0.84, 0.97, 1.01, 1.02, 1.04, 1.15, 2.03.

#### MIN-3. R17.3 is over-applied: the settlement rule is in the "REPORTED, not gated" bucket (70%)
- **Where:** `[isolation].expect` (3) `:828`.
- **Problem.** "Under the two-settlement forcing no M2-drawn marriage pairs two settlements" is a hard rule of N4
  (REV-SPR-018 R8), not a Q6 stylized fact. In the "not gated" bucket, a bug in the settlement filter cannot fail G1a.
- **Fix:** move it to a gated expect item, or add an invariant: every open M2-drawn marriage row pairs equal residences,
  or a residence of 0 (P1a has no moves).

#### NITs
- **NIT-1. Stale text.**
  - `:424`: the Day/Vital `apply_death` step still says "due death timers (M2's own and those made from ScheduleDeath
    commands)". That is shape (a); under R12, M2 makes no timer from a command.
  - `:690`: the `apply_marry` summary says "commanded marriages and T1's Marry commands", which names one thing twice.
  - `:328-331`: the comment says "All three are flags … none enters a rate". There are four reads, and
    `house_member.person` selects c_i^e, so it does enter the conception rate when `elite_fertility_mult` ≠ 1.
  - `specs/manifest/examples/household.yaml:4-5` still says "persists or dies out by chance", against expect (5), which
    says it dies out in every replica (R17.5).
  - Optional: frozen DEATH `when` reads "when a death timer fires", but a commanded death is applied in the drain. A
    one-line `[[amends]]` would make it exact.
- **NIT-2. The epoch is not yet in the key (BCK-002 A-29, open).** `StepCtx::rng` keys every draw at epoch 0
  (`system.rs:429-431`, `engine.rs:1057`). So "the woman's epoch … a scoped input re-ranks" (`:474`, N4 `:594`) holds
  only once A-29 lands. The fork part already holds through the branch salt.
  - Cite A-29 in the text.
  - `RngStream::draw(t, k)` does not exist. Name one of these instead: `StepCtx::rng(husband_pick, w, h)`,
    `RngStream::draw(k)` (`rng.rs:292`) or `StreamKey::draw(t, k)` (`hash.rs:187`).
- **NIT-3. S12's "BIRTH_TIMER due in this slot" test has no direct accessor.** rh-core pops due timers before the timer
  pass (`agenda.rs:231`, `engine.rs:834`), and `own_timers` excludes them (`system.rs:577`). The test can still be
  derived: a woman with `conceived_tick ≠ i64::MIN` and no pending own BIRTH_TIMER has her birth due in this slot. Say
  so in S12 or F10.
- **NIT-4. Self-check robustness.**
  - A duplicate at index 0 of the next version itself aborts on the `assert` (`lint_odd.py:801`) before any ERROR line
    prints. The run still fails, and the assertion names the duplicate, but the diagnostic is buried.
  - The self-check has no probe for `rng.<purpose>.use` and none for an unknown amend path. The lint catches both;
    only the self-check is missing them.
- **NIT-5. The preserved slice omits some frozen content.** Three probes pass silently: `r3_verification_drop` (a
  frozen `[verification]` test dropped), `r3_coupling_drop` (a frozen out-coupling dropped) and `r3_param_meaning` (a
  frozen parameter meaning rewritten). Cheap fixes: require frozen tests ⊆ next tests, and keep frozen couplings by
  (system, direction).
- **NIT-6.** The `[[reads]] house_member.person` neutral is `0` (`:351`), but the forcing says `constant:absent` and
  the coupling says "no row". Use "absent".

#### 2.1 Bypass attempts (round 3), `$R/tools/probes_r3.py`, `$R/probes/r3_*`

| Probe | Edit | lint | `check_all` gate |
|---|---|---|---|
| `r3_amend_swap_s9` | S9 changed through a valid record; one meaning amendment withdrawn | **PASS** | **PASS** (MIN-1) |
| `r3_override_table` | a new `[algorithm.mortality_override_f1]` that "replaces S9's day draw"; the frozen table kept | **PASS** | **PASS**. Inherent: review only. The README should name it as the reviewer's duty |
| `r3_added_step_contradiction` | a new Month/Vital step `mortality_f1` declaring the frozen `mortality` step void | **PASS** | **PASS**. The same class: additions are allowed |
| `r3_verification_drop`, `r3_coupling_drop`, `r3_param_meaning` | see NIT-5 | PASS | PASS |
| `r3_amends_duplicate` | the same amendment path recorded twice | FAIL | FAIL |
| `r3_stepname_space` | `"mortality : …"` added beside the frozen `mortality` | FAIL (duplicate step name) | FAIL |

### 3. Numbers (R18)

- **The widowhood chain**, re-run with round 1's `chain_widow.py` (`$R/tools/chain_widow_r3.txt`):

  | tfr | husband older by | NRR | r |
  |---|---|---|---|
  | 4.9 | 3 y | 1.0352 | +0.117 %/yr |
  | 4.9 | 4 y | 1.0342 | +0.114 %/yr |
  | 4.741 | 3 y | 1.0011 | |
  | 4.741 | 4 y | 1.0001 | |
  | 4.7 | 3 y | 0.9923 | −0.026 %/yr |

  - The root (NRR = 1) is 4.736 with a 3-year gap and 4.741 with a 4-year gap.
  - The cohort Monte Carlo (10^6 women, day resolution, seed 7) gives NRR 1.0351 and r = +0.117 %/yr (round 2's seed 1:
    1.0349 and +0.116).
  - The drafter's figures are NRR 1.0341, r = +0.114 %/yr and a root at 4.741. They agree to within 0.003 %/yr.
- **The c_i values.** An independent exact recomputation (`$R/tools/ci_check.py`) gives TFR1 = 9.393524 (bits
  615614), m = 34186, and c bits [119397409, 143118858, 128689234, 112318286, 83490541, 36474834, 4576314], that is,
  0.0278, 0.03332, 0.02996, 0.02615, 0.01944, 0.00849 and 0.00107. These equal the spec's `check_values`, and
  12c/(1 + 20c) = M n_i holds exactly.
- **The age shape.** See MIN-2: 0.84, 0.97, 1.01, 1.02, 1.04, 1.15, **2.03**.
- **The interval facts** check out: 1/c at 20–24 is 30 months, and the long-run rate is 0.240 a year, a birth every
  50 months.

### 4. Foreign reads (`specs/systems/next/M2.demography.odd.toml`)

| Read (line) | Owner | Lag | `use` | `[[reads]]` neutral | In-coupling neutral (line) | Isolation forcing | Verdict |
|---|---|---|---|---|---|---|---|
| `person.named` (`:333`) | T1.minds | lagged | mode | `false` | "person.named = 0 for everyone" (`:768`) | constant:0 | OK |
| `person.vocation` (`:340`) | M7.dynasty | lagged | mode | `"none"` | "person.vocation = none for everyone" (`:785`) | constant:none | OK |
| `house_member.person` (`:347`) | M7.dynasty | lagged | mode | `0` | "no house_member row for anyone" (`:797`) | constant:absent, plus a 10 % step | OK. It does enter a rate when the multiplier is ≠ 1, but it already has a full in-coupling with bounds (NIT-1, NIT-6) |
| `person.residence` (`:355`) | S3.migration | lagged | mode | `0` | "person.residence = 0 for everyone" (`:814`) | constant:0, plus a two-settlement step | OK |

**Verdict: compliant.** There is no `identity` read. Every foreign read is `mode`, with an explicit neutral on both the
`[[reads]]` and the in-coupling, and with an isolation forcing. I also checked the algorithm text for undeclared reads:
F5, N3, N4 and N7 read only these four columns from other owners. So M2 registers without M7, S3 or T1.

### 5. Checks run

All of these ran on `f9aca53` in `$R/src`:
- `lint_odd.py` passes: 6 specs, 8 warnings.
- `lint_odd.py --next … --overlay registry_delta.toml` passes, with 12 AMEND lines and "19 planted changes rejected, 12
  amendments recorded, the clean next version kept".
- The three `odd_next_control` cases (algorithm, rng, bound) FAIL, each with its own diagnostic.
- All 19 lint_odd-related `check_all` gates pass (`$R/tools/odd_gates.sh`): the smoke specs, the skeletons, their
  controls, and the rung-0 and F1 references.
- `selftest_m2_f1.py` passes all 8 checks. Its control (`tfr1_bits = 615615`) FAILs with "tfr1_bits: spec 615615 !=
  reference 615614".
- `selftest_m2_rung0.py` passes.
- `gen_manifest --check` passes, and the household, hamlet and solo manifests are VALID.
- The venv's `test_check_all_inventory.py` gives 12 passed. Repository-wide ruff and mypy are clean.

**Not run:**
- the engine, or any Rust build or test (this delta changes no Rust);
- the full `check_all.sh`;
- a 365-day calendar;
- T1, M7 or S3 coupled.

## 4. Closing: the Architect's check of the merge head (2026-10-17, `5732e3d`)

`5732e3d` merges `origin/main` at `cdf7d86` (EVO-001, SPR-018) into `3a87d8f`, the head that carries R19–R22.

### What was run
All of it ran on a worktree of `5732e3d`:

| Check | Result |
|---|---|
| `lint_odd.py` (frozen mode) | PASS: 8 system specs, 14 warnings, the self-check rejects the broken spec |
| `lint_odd.py --next specs/systems/next --overlay registry_delta.toml` | PASS. The self-check reads "29 planted changes rejected (3 of them by the amendments lock), 18 amendments recorded and locked" |
| `--write-amendments-lock` | Regenerates `amendments.lock.toml` byte for byte (no diff) |
| `selftest_m2_f1.py` | All 8 checks PASS |
| `gen_manifest --check`; `gen_skeletons --check` | PASS |
| `household.yaml`, `hamlet.yaml` | VALID |
| pytest, the whole suite in a git worktree | 619 passed |
| `cargo nextest run -p rh-manifest --test compile` | 10/10 |

The drafter's `bin/check_all.sh` on the committed tree, both parts, gives PASS with 344 checks.

### The merge decisions
Each is accepted:
- **The frozen 0.2.1 `[[applies]]` entries** come byte for byte, with the typed payloads and no `tick`.
- **No `[[amends]]` record for `DEATH`'s `when`.** The frozen 0.2.1 text already names the Day/Vital drain, and the lint refuses a record that amends nothing.
- **`rh-manifest`'s compile test** now expects three cast founders in `household.yaml`, the third being the unrelated founder Wulf (R8). No engine code changes.

### What stays open
These are not blockers:
- **Q1.** The Coale–Trussell n(a) values, the 105 : 100 sex ratio and Barrett's 11 months are cited-pending. A research batch confirms them before SPR-015 is planned, and a corrected value moves every check value.
- **Folding mortality into `next/`.** `specs/demography/mortality_f1.toml` (SPR-016) moves into the next M2 as a new `[algorithm.*]` table beside the frozen one, by the SPR-016 drafter, after this merge.
- **D40 and D41** go to the director (`docs/decisions_pending.md`).

