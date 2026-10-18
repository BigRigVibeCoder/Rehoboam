---
id: REV-T1A
title: "Independent review of the T1-A spec PR: T1 0.4.0's A-steps ([algorithm.minds_f1] A0-A16), the exact reference specs/minds/selftest_minds_v0.py, the rh-minds pin"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [SPR-023, SPR-024, SPR-025, REV-MINDS, EVO-002, PRJ-002, BCK-002, SPR-038, SPR-021, SPR-022]
created: 2026-10-18
updated: 2026-10-18
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit (2026-10-18).** T1 0.4.0 stays a `next/` draft until SPR-023 T-01 promotes it.
>
> - **Round 1** (`777e330`, one commit on `main` `78fef08`): **AFTER SMALL FIXES**, 0 BLK, 0 MAJ, 8 MIN, 12 NIT (§1). Every touched gate passed. The trait draw, opinion and thought expiry, A7's cancel rule and the relation refresh re-derived independently. 35 of 40 reference mutants were killed; five survived (F5).
> - **Rulings** (§0): every finding accepted. F1 makes `exits` the only step that ends the dead's relation rows (`grief` amended). F2 closes A7's vocabulary. F6 records the trait draw as Irwin–Hall(4) with its reason and the rejected alternative. F7's SPR-023 plan edits are the Architect's (SPR-023 0.3.1, this commit). F8 moves the cast criterion to T-00a.
> - **Revision 1** (`c53911b`, merged with `main` `99a5b3d` as `8c43a85`): every row of §0 is applied. The Architect's verification is §0c.
> - **What SPR-023 inherits:** 26 reads (with `person.id` and `title.id`); 21 `[[amends]]` records, locked; the opinion sum in `i64`; world start with the cast's minds first, then M7's tick-0 `Promote`s; Demote cancels every own timer except `DEATH_DUE` and `MARRY_DUE`; the golden of 253 lines on six scripted worlds; five controls and four mutants, each failing its own named line.

## 0. Architect rulings, round 1 (2026-10-18), on `777e330`

The independent review returned AFTER SMALL FIXES: 0 BLK, 0 MAJ, 8 MIN, 12 NIT. Every gate passes. The trait draw, expiry, A7 and the relation refresh re-derive independently. Every finding is accepted unless a row says otherwise.

| # | Ruling | Conf. |
|---|---|---|
| F1 | **The `exits` step ends the dead's relation rows, and `grief` ends none.** Amend `schedule.Day/Vital.steps.grief` with an `[[amends]]` record: grief reads the rows that `exits` ended at `now` (`ended_at = now`, a or b the dead) together with kin, adds grief stress, and ends the dead's feuds, not relation rows. Add A8's sentence: "grief (T1-I/S) reads the rows ended at `now`; it ends no relation row itself." The step order stays `outcome_due, exits, grief, threshold` | 80% |
| F2 | **(a)** A7 closes the vocabulary: Demote cancels "every own timer of p except `DEATH_DUE` and `MARRY_DUE`", by `cancel_because(id, Ended)`, counted per kind. No kind names are invented here. **(b)** Add a sentence: `own_timers` is the agenda at the level's start, so a timer scheduled earlier in the same step is not cancelled; its fire revalidates `person.named` and drops as `ended` (T1-I/S states the fire-time check) | 85% |
| F3 | A0: "until T1-I/S gives T1's parameters defaults through `registry_delta.toml` (R8), a built T1 missing a parameter is refused by name" | 90% |
| F4 | `thought_gain = 100` goes into `constants`, as a design value. T1-I/S defines its coupling to `[overlay_traits]` | 85% |
| F5 | Add the five scripted operations that kill E2, E7, E26, E30 and E44 (a pulse-day thought command; a promotion on the instant the liege died with a same-instant regrant; a title change on a pulse day; a planted `refractory` with a Demote; planted break and refractory-end timers). Regenerate the golden by `--write` only | 85% |
| F6 | **Accepted, and recorded.** The trait draw is Irwin–Hall(4), the sum of four 16-bit fields scaled to `trait_sd`: bounded at ±3.46 sd, excess kurtosis −0.3. The amendment's `reason` states the distribution, why (one `StepCtx::rng` draw per k avoids `StreamReuse`; an exact Python reference of a ziggurat with `libm` is not feasible), and the rejected alternative (seven `Normal::sample` calls on one stream, legal in effect but against `05.1` §6 item 3's letter). `trait_sd` stays the sd | 75% |
| F7 | **SPR-023's plan edits are the Architect's, at the T1-A close:** A11 sums in i64; 26 reads (with `title.id`); A5's order (the cast's minds first, then M7's tick-0 Promotes, each ascending); T-01 row 4 and checklist (f) (the pin control and the check_all lines are done here); and the promotion checklist gains `minds_stale_control`. **The drafter's part:** `specs/systems/next/README.md` checklist (e) names `minds_stale_control` beside the two M2 stale controls | 85% |
| F8 | **The cast criterion is T-00a's** (M7 0.6.0's criterion (e), ruled 70%). M7 reads the manifest `cast` at `[initialization]`. The marker that lets the dormancy pass keep cast members named is T-00a's, and its reviewer checks it. **In T1-A:** `world_start` no longer scripts M7 Demoting a cast member, because M7 0.6.0 never sends that Demote. A5 states that the cast stays named under M7 0.6.0's criterion (e), and keeps "a Demote received for any named person is applied" as T1's general rule, with no world-start special case | 70% |
| N1–N12 | **Accepted:**<ul><li>N1: the citations, using plan rows 3, 7, 8 and 9 rather than the drafter's own finding numbers.</li><li>N2: A0's claim narrowed to the parameters this part narrows.</li><li>N3: the `[[rng]]` header comment.</li><li>N4: the stand-in fires M2's natural deaths before the owed timers, as `outcome_due` says, and exercises `drop_timer(dead)`.</li><li>N5: INV-T1-12's test id; INV-T1-13's "right after a Demote" clause becomes a step assertion, not an observer one.</li><li>N6: the `clean` control by a version-agnostic bump, if trivial.</li><li>N7: CLAUDE.md's `--next` line names the T1 draft.</li><li>N8: no forward reference in the `break_timer` meaning.</li><li>N9: the reference reads the HEXACO order from `[[owns]]` and A15's step order from `[[schedule]]`. INV-T1-05's key half is recorded at creation by the observer.</li><li>N10: merge main.</li><li>N11: the mutant's printed name.</li><li>N12: the dead text in A12.</li></ul> | 85% |

## 0c. Architect's verification of revision 1 (2026-10-18), on `8c43a85`

Run in a worktree of `arch/T1-0.4-minds-a` at `8c43a85` (revision 1 `c53911b` with `main` `99a5b3d` merged).

| Check | Result |
|---|---|
| `selftest_minds_v0.py --check` | `MINDS V0 (A-STEPS): PASS (6 cases, 253 golden lines)`: 236 lines before, plus F5's five operations |
| The five `--control` runs (`redraw-on-repromote`, `demote-cancels-death-due`, `stale-index`, `trait-key-wrong-t`, `world-start-genesis-names`) | Each exits 1 on its own observer |
| `lint_odd.py --next specs/systems/next --overlay specs/systems/next/registry_delta.toml` | T1: 43 planted changes rejected (2 by the amendments lock), **21 amendments recorded and locked** (19 before, plus F1's grief step and summary), the clean next version kept |
| `lint_odd.py` (frozen) | PASS: 8 system specs linted (12 warnings) |
| F1 | The `grief` summary and `schedule.Day/Vital.steps.grief` are amended: grief reads the rows `exits` ended at `now` and ends the dead's feuds, not relation rows. A8 carries the sentence. The step order is unchanged |
| F2 | A7: Demote cancels "every own timer … except `DEATH_DUE` and `MARRY_DUE`" by `cancel_because(id, DropReason::Ended)`, counted per kind; the same-step sentence is present |
| F3, F4 | A0 reworded to R8's form; `constants.thought_gain = 100`, labelled a design value, cited in A12 |
| F6 | A3 and the `rng.trait_draw.use` amendment's `reason` state Irwin–Hall(4), ±3.46 sd, excess kurtosis −0.3, why, and the rejected alternative |
| F7 | `specs/systems/next/README.md` checklist (e) names `minds_stale_control`. The plan edits are in SPR-023 0.3.1 (this commit) |
| F8 | `world_start` no longer scripts M7 Demoting a cast member. A5 states the cast stays named under M7 0.6.0's criterion (e) and keeps the general Demote rule |
| N1–N12 | Applied. N4: the stand-in fires M2's natural deaths before the owed timers. N6: the `clean` control rewrites `spec_version` to 99.0.0. N7: `CLAUDE.md`'s `--next` line names the T1 draft. The opinion-no-expiry mutant's `check_all` line quotes `decide (stand-in, T1-I/S)` |

**The SPR-023 plan edits (F7), in this commit, SPR-023 0.3.0 → 0.3.1:**
- the opinion sum in `i64` (BLUF, row 10, compliance, T-04);
- 26 reads, with `title.id` (compliance, row 14, T-03; thirteen lagged M7 reads in the cost row);
- world start's order: the cast's minds first (`world_start`), then M7's tick-0 `Promote`s (`promote`), each ascending (row 7, T-03);
- the Demote rule's closed vocabulary (row 8, T-03, T-02's acceptance);
- the trait draw: one `StepCtx::rng` draw per k, Irwin–Hall(4), no `rh-sample` call (compliance);
- T-01: row 4's pin and fixtures, and checklist (f)'s gates, are done in T1-A; the promotion moves `minds_stale_control` and the selftest's spec path to the frozen file; the `clean` control is version-agnostic.

**Left open, and owned:**
- The merge with T-00a: both branches add a row to `specs/systems/next/README.md`'s table. Whichever merges second keeps both rows.
- The cast marker for criterion (e) is T-00a's (F8); its reviewer checks it.
- BCK-002 A-69 (new fields on frozen entries are not tracked by the next-version lint) is filed at the SPR-014 audit, with T-00a's MIN-13.

## 1. The independent review, round 1 (`777e330`)

The review was independent, on a different model from the drafter's. Nothing in the repository was edited, committed or pushed, and the review worktree was removed. The Architect recorded this report from the reviewer's returned text. The evidence is in `t1a_review/work/`: `indep_trait.py`, `gates.sh`, `gates.out`, the control and mutant outputs, and `spec.diff`.

### Verdict

**Verdict: AFTER SMALL FIXES.** 0 BLK, 0 MAJ, 8 MIN, 12 NIT.

| Area | Result |
|---|---|
| Gates | Every touched gate passes, including the 14 new ones (12 minds gates and 2 `lint_arch` gates). pytest 694 passed; ruff and mypy strict clean |
| Rulings | X4, X13f, R3/R8, R9 and R14 are honoured. One A0 wording conflicts with R8 (F3). The 19 amendments are justified and match the draft |
| Re-derivation | The trait draw, opinion and thought expiry, the A7 cancel rule and the relation refresh all re-derive independently |
| Golden strength | 5 of 40 reference mutants survive (F5) |
| Most important fix | F1: `exits` ends relation rows before `grief`, whose frozen text still claims to end them |
| Process | The branch is behind `main` (PRs #93 and #96), with no file overlap |

### 1. Gate results

| Gate | Result |
|---|---|
| `lint_odd.py` (plain) | PASS: 8 specs, 14 pre-existing warnings |
| `lint_odd --next` | PASS. T1: 19 amendments recorded, 19 paths match the lock, 43 planted changes rejected (2 by the lock), the clean next version kept. M2 and M7 unchanged |
| The 41 touched `check_all` gates | PASS in 9.9 s, run through `check_all.sh`'s own `run` helper and control functions |
| `selftest_minds_v0.py --check` | PASS: 6 cases, 236 golden lines, about 0.45 s |
| Five `--control` runs | Each FAILs for its own reason: `redraw-on-repromote` (INV-T1-05 immutable), `demote-cancels-death-due` (A7, twice), `stale-index` (A2), `trait-key-wrong-t` (INV-T1-05 key), `world-start-genesis-names` (INV-T1-11) |
| Four `--mutant` runs | Each changes a named golden line |
| Stale-golden and stale-spec controls | FAIL as intended |
| `lint_arch` and its two fixtures | PASS; each fixture fails for its own reason |
| `gen_manifest --check`, `gen_skeletons --check`, `lint_templates` | PASS |
| ruff, ruff format, mypy strict, pytest | PASS |
| Lock reproducibility | PASS: the tool regenerates it; only the header comment's command path differs |
| Reference determinism | PASS: four `--write` runs give identical files equal to the committed golden |
| Deny-by-default spot checks | PASS |

**CI placement:** the 12 minds gates run in part core, about 5 s in all. No reason to move them.

### 2. Independent re-derivation

- **Trait draw:** derived from A3's text only, through `gen_rng_golden.h`. SIGMA recomputes to 2479700524. All 7 `traits` lines and all 25 `create` lines match. k = 0..6 lie in ChaCha block 0. The width bound holds below 2^47.
- **Opinion and thought arithmetic:** hand-checked. The expiry boundaries are right on both calendars, the Registry magnitude overrides the payload, the sum clamps, the narrowing is right, and the half-open decay window is right.
- **A7:** the entities of `DEATH_DUE` and `MARRY_DUE` were confirmed against the frozen spec. `own_timers` filtered by kind matches the golden. The API names exist on SPR-014's branch.
- **Relation refresh:** `relations_monthly` was traced by hand and matches.

**Mutation probes:** 35 of 40 mutants were killed or equivalent. The survivors are:

| ID | Mutant | Why it survives |
|---|---|---|
| E2 | Decay window uses `<=` t_prev | No command lands on a Month-pulse day |
| E7 | `kept()` ignores b's liveness | No refresh runs on the instant its title holder died |
| E26 | Month refresh reads current titles, not lagged | No title changes on a pulse day |
| E30 | Deferral only for `pending` | `refractory` is never planted |
| E44 | Demote cancels only poll and interrupt | break and refractory_end timers are never planted |

### 3. The 19 amendments

All 19 have `frozen` equal to 0.3.3 and `to` equal to the draft, and the lock matches. They are all justified: `initialization.state` (X4); `rng.trait_draw.use` (F6); `submodels.promote.summary`; `schedule.Day/Agenda.when`; five column meanings; INV-T1-01 to INV-T1-10. X13f holds.

### 4. The drafter's design choices

| Choice | Assessment |
|---|---|
| (1) The cast criterion | Sound in intent, under-specified as a fix (F8) |
| (2) No-op commands dropped as `ended` | Sound under R14 |
| (3) The `clean` control hard-codes 0.3.3 | Acceptable; see N6 |
| (5) `relabelled_minds` here | Sound; SPR-023 T-01 row 4 needs editing |
| A12's thought sum in i32 | Sound |
| INV-T1-13 | Sound; see N5 |
| One `StepCtx::rng` draw per k | Sound as a buildability fix; see F6 |

### 5. Findings

| ID | Sev | Problem | Fix |
|---|---|---|---|
| F1 | MIN | `exits` ends relation rows before `grief` runs, and the frozen `grief` text says it ends the dead's rows. Two steps claim the same rows, and the step order freezes at promotion | An A8 sentence, or amend `grief`'s step text, or run `grief` before `exits` |
| F2 | MIN | A7 (a) filters on kinds that do not exist until T1-I/S; (b) a timer scheduled in the same step is invisible to `own_timers` | Close the vocabulary ("every own timer except `DEATH_DUE` and `MARRY_DUE`"); state the same-step case |
| F3 | MIN | A0 says "T1 has no Registry default in P1a", against R8 | Reword |
| F4 | MIN | A12's gain constant 100 is unlabelled | Add `thought_gain` to `constants` as a design value |
| F5 | MIN | 5 reference mutants survive the golden | Five `Op` lines |
| F6 | MIN | The trait draw changes the distribution to Irwin–Hall(4) (bounded ±3.46 sd, excess kurtosis −0.3), and the amendment does not say so. The rejected alternative (seven `Normal::sample` calls on one stream) is not recorded. SPR-023's compliance row is stale | Record the distribution, the reason and the rejected alternative |
| F7 | MIN | SPR-023 needs edits when T1-A merges: i64 in A11, 26 reads, A5's order, T-01 row 4 and checklist (f), `minds_stale_control` in the promotion checklists | Plan edits |
| F8 | MIN | The cast criterion needs a marker, because `person.named` carries no provenance. M7 reads `cast` from the manifest at init. A T1 column cannot work at tick 0 | A BCK item, or a cheap marker in T-00a; the `world_start` golden changes if adopted |
| N1 | NIT | Wrong citations | Cite plan rows 3, 7, 8 and 9 |
| N2 | NIT | A0 over-claims (`break_refractory_days`) | Narrow it |
| N3 | NIT | A stale `[[rng]]` header comment | Fix |
| N4 | NIT | The stand-in fires owed timers before natural deaths; `drop_timer(dead)` is not exercised | Reorder |
| N5 | NIT | INV-T1-12's test id; INV-T1-13's clause cannot be checked by an observer | Fix |
| N6 | NIT | The `clean` control could use a version-agnostic bump | Optional |
| N7 | NIT | CLAUDE.md's `--next` line does not name the T1 draft | Add it |
| N8 | NIT | A forward reference in the `break_timer` meaning | Remove it |
| N9 | NIT | The reference hard-codes the HEXACO order and A15's order; INV-T1-05's key half needs a creation record | Read them from the spec; record at creation |
| N10 | NIT | Merge `main` | Merge |
| N11 | NIT | The mutant prints a different name | Fix |
| N12 | NIT | Dead text in A12 | Remove |
