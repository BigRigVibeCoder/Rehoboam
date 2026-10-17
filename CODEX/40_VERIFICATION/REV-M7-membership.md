---
id: REV-M7-membership
title: "Independent review of PR #79: great-house membership (BCK-002 A-56), dormancy precedence (A-57), the M7 0.5.0 draft and D56"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [BCK-002, PRJ-002, SPR-015, SPR-019, SPR-021, SPR-029, SPR-030, REV-SPR-015, REV-SPR-029, REV-SPR-030]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF: CLOSED on the Architect's closing commit of PR #79 (round 3 verified, 2026-10-17; `check_all --part core` 431 PASS on the closing tree).** The M7 0.5.0 draft carries agnatic house membership (BCK-002 A-56) and the dormancy precedence "naming wins" (A-57). D56 goes to the director.
> - **Round 1** reviewed `f27409c` and found 0 BLK, 2 MAJ, 6 MIN and 6 NIT (§1).
>   - Both fixture worlds register T1, and under T1 named persons marry only named persons, so the bounded rule's path through the mother rarely fires. D56 became "how do great houses persist in a T1 world" (R1).
>   - Rulings: §0.
> - **Round 2** verified `0f7187b` and found 1 BLK, 2 MAJ, 6 MIN and 6 NIT (§2):
>   - `check_all` was red (backticks run as commands);
>   - the holy-orders share of the later-children lever was wrong;
>   - D56's D38 sentence was inverted.
>   - Rulings: §0b.
> - **Round 3** verified `b38d10a` and found 0 BLK, 0 MAJ, 1 MIN and 6 NIT (§3). The closing commit fixes them (§0c).
> - **Result:**
>   - agnatic membership;
>   - vocations for later children;
>   - no resurrection of an extinct house;
>   - any holder named;
>   - a deny-by-default next-version lint with 22 controls;
>   - the projection in both regimes, with the levers.
>
>   Recommendation in D56: agnatic plus later-children vocations (6/10). Elite fertility read on the couple (e) is asked as direction only.
>
> **Reviewer:** one session on a different model from the drafter's, across all three rounds.

# REV-M7-membership: independent review of PR #79

## 0. Architect rulings, round 1 (2026-10-17)

An independent reviewer on a different model reviewed `f27409c` and found 0 BLK, 2 MAJ, 6 MIN and 6 NIT (§1).
- `check_all --part core`: 415 of 415 PASS.
- The four engine baselines recompute equal, and the projection reproduces.

**The result changes the question D56 asks.** In a world that registers T1, which includes both fixture worlds, named persons marry only named persons, so the bounded rule's path through the mother rarely fires. Under T1, agnatic and bounded both shrink the houses: about 3.1 members per house on `twin_vales` at year 300, 1.3 on `kingdom_10k` uncapped, and 0.7 capped.

D56 therefore becomes **"how do great houses persist in a T1 world"**, with membership as one lever among several.

| # | Ruling | Confidence |
|---|---|---|
| R1 (M1) | **Accepted.** Add a coupled-market variant to `project_population.py` and its golden: the named core marries within itself at the elite rate, and inflow comes only from women who become eligible after an M2 marriage. Label every number with its regime ("M7 isolation, T1 stubbed" or "T1 registered"). Restate §5 and D56 for each regime. Re-derive the recommendation from the T1-registered regime, because that is the saga world. Compare at least these levers, each with its projection line under both regimes and its cost:<ul><li>(a) cognatic;</li><li>(b) agnatic;</li><li>(c) bounded;</li><li>(d) b or c with holy orders for later children only (R4);</li><li>(e) b or c with M2's elite fertility read from the couple, not the woman's row;</li><li>(f) T1's marry candidates widened to unnamed persons of the same polity (hypergamy), a T1 change;</li><li>(g) any combination you judge necessary.</li></ul>If agnatic ≈ bounded under T1, prefer the simpler rule unless a lever separates them | 85% |
| R2 (M2) | **Accepted: `HOUSE_JOINED` is always admitted.** Give it its own act class whose `[event_rows]` ceiling `tellable_up_to` is `4294967295` ("always", `05.2` §4.0 criterion 1). Do not use verb tellability 255: that level is reserved for the world-ending verbs. Carrying the house on the born `LIFE_ENTRY` would be a channel-A schema change (C3) and is the fallback. State it in the `[[emits]]` `when` text and in annals §16, and add the D38 interaction to D56: the 10k world passes the ceiling under every rule but cognatic-capped | 75% |
| R3 (M3, Q5) | **Accepted, in this PR.** `lint_odd --next` becomes deny by default: any changed field of the frozen slice outside an `[[amends]]` record fails. Cover at least:<ul><li>submodel summaries;</li><li>command `why`;</li><li>couplings;</li><li>`isolation.*`, `design.*` and `purpose.*`;</li><li>`params.*.source`.</li></ul>Add one planted control per newly covered path, and the reviewer's three planted cases (`q5_*`) as controls. Re-derive both next drafts' `[[amends]]` records and the lock **with the tool** (`--write-amendments-lock`). M2 0.3.0's newly recorded amendments are listed in the PR body for the Architect to read. This must land before SPR-015 promotes M2 0.3.0 | 85% |
| R4 (M4) | **Accepted.** Put the vocation-reading conflict in the specs table. The primary reading is "**later children**" (the first-born exempt), per the three frozen texts and the source, RT-2026-10-16 item 12. Amend the submodel's "not first in any title's line" to match, as a recorded amendment, or say why the submodel's reading must win. Keep the other reading as a sensitivity line in the golden | 65% |
| R5 (M5) | **Accepted.** Settle the six edge cases with one clause each, using these defaults (propose otherwise in the specs table if you must):<ul><li>(i) a mother who died in the birth slot counts by her state at the birth instant's start;</li><li>(ii) a holder who dies this instant still holds at `on_birth`, which runs before `on_death`;</li><li>(iii) newborns are processed in ascending PersonId;</li><li>(iv) "the line" is the line after M7's exclusions;</li><li>(v) a sortition title has no first or second outside its arbitration instant, so (d) does not apply to it;</li><li>(vi) the mother branch is for great houses only</li></ul> | 75% |
| R6 (M6) | **Accepted: widen (d)** to any great-house member who is first or second in the line of any title. An heir apparent is named whatever their house. Carry it into the SIM-9 amendment text (R11) | 70% |
| R7 (M7) | **Accepted.** The possible-history check walks `house_of` (the father has none, the mother has one, one record per child). Add `fixture-joined-untriggered` and `fixture-joined-housed-father` as `check_all` controls, plus the reviewer's in-run-father probe as a case | 90% |
| R8 (M8) | **Accepted.** Reword D56 after R1, with numbers that match the golden and the costs and alternatives listed. D40 sets no trigger; say that its revisit runs in SPR-030 regardless. Put D56's row after D55 | 90% |
| R9–R14 (M9–M14) | **Accepted.** M9: the envelope row and the texts. M10: `HOUSE_JOINED` carries the HouseId; fold the rest into A-55. M11: one BCK item for SPR-021 (same-instant learning of another system's EventId; `SUCCEED` to `DEATH` too). M12: name it in the promotion cost. M13: align the texts. M14: one line while INV-M7-06 is amended | 90% |
| Q4 | Agree, with R2 and R10 | 75% |
| Q6 | Decided by R1's comparison; the elite read on the couple is a lever there, not only a later one | — |
| Q7 | Agree: no new parameter. State the coupling between `dormancy_degrees` and membership in `fixture_worlds.md` §6 and in D56 | 70% |
| Q9 | Agree | 90% |
| Q10 | Superseded by R2: the class is chosen for admission | — |
| Q11 | Agree to keep the house-scoped (c). Draft the SIM-9 amendment text: "more than N degrees from every holder of a title, claim or office of their house, and not first or second in the line of any title (R6)". `docs/03` changes on the director's D56 answer | 70% |

**Round 2:** the drafter applies R1–R14 on `arch/M7-membership` and pushes once. An independent reviewer then re-checks. D56 goes to the director only after that verification.

## 0b. Architect rulings, round 2 (2026-10-17), on `0f7187b`

The second independent review found **1 BLK, 2 MAJ, 6 MIN and 6 NIT**.

**What holds:**
- The agnatic recommendation (6/10).
- R2, R5, R6, R7 (the walk), R9–R13, Q7 and Q11.
- The amendments lock, which is the tool's output.
- The thirteen controls, each failing with its own diagnostic.
- **M2 0.3.0's 26 newly recorded amendments.** Each was traced to SPR-015's births PR (#50) or SPR-016's mortality fold (PR #51/#59). The M2 next file's text is unchanged in this PR, so there is no new semantic change. I accept them as recorded.

**What does not hold:**
- `check_all` part core fails (N1).
- The (d) lever's holy-orders share is wrong, and with it every (d) and (d)+(e) number in D56 (N2).
- D56's D38 sentence is inverted (N3).
- The lint is an allow-list, not deny-by-default (N4).
- M14 went beyond its one line (N6).

**Every finding is accepted unless a row says otherwise.**

| # | Ruling | Confidence |
|---|---|---|
| N1 (BLK) | **Accepted.** Escape every backtick inside double-quoted `check_all.sh` text, or drop the backticks, and update the four inventory names to match. **Run the changed `check_all.sh` lines through bash before you push.** Replaying lines in Python is not enough. If the Rust gates cannot run in your container, run `bash -n bin/check_all.sh` and execute each changed `run` line with `bash -c` exactly as written, then say so. I run the full part core before merging. | 95% |
| N2 (MAJ) | **Accepted, with the reading ruled (see N5).** A child is a "later child" when an elder child of the same father is **alive at the vocation draw**: the eldest living child of the father is exempt. A child first or second in any title's line is also exempt. Derive v from the M2 F1 chain under that reading, as the reviewer did (its "eldest surviving to 16" gave about 0.121 for commoners and 0.132 for the elite). Regenerate the golden with the tool, and restate D56, the `M7_membership.md` BLUF, §1, §4 and §8, and `fixture_worlds.md` §3.3 from it. Keep the frozen reading (v = 0.2) and the literal "eldest by birth_tick, dead included" reading (about 0.152 and 0.160) as sensitivity lines. Drop the "elite overproduction, 11%" framing unless the new numbers support it. | 65% |
| N3 (MAJ) | **Accepted.** Invert the sentence. Under every rule except cognatic-capped, the 10k world sits **above** D38's 10,000 ceiling (agnatic: 10,433 capped, 31,260 uncapped). From about year 1, commoner acts are then told only with a named participant. D38's premise of a fully told 10k world fails under every rule but cognatic-capped. D56 says so. | 95% |
| N4 (MIN) | **Accepted. Make the lint truly deny-by-default:** walk every frozen leaf generically, with an **explicit exemption list in `specs/systems/next/README.md`** for me to rule on at the re-check. The defaults:<ul><li>exempt: versions, `fidelity`, and lists that may only grow;</li><li>new `[[reads]]`, `[[emits]]` and `owns.*.values` entries may be added, but an existing entry's fields are protected. That covers a read's lag and its neutral, and an enum code's name and value;</li><li>protected: `owns.*.registry`, every `[[applies]]` field, `initialization.from_manifest` and `input_data.drivers`.</li></ul>Add one planted control each for an enum code renamed, `drained_by` changed, `from_manifest` edited, and a read's lag changed from `same_instant` to `lagged`. Re-derive both drafts' records and the lock with the tool. | 80% |
| N5 (MIN) | **Accepted.** The vocation text, in the submodel and the Year step alike, becomes: "a later child (an elder child of the same father is alive at the draw) who is not first or second in any title's line". This matches the frozen step, R4 and N2. | 70% |
| N6 (MIN) | **Ruled: the one-line alternative. Revert M14's deferral.** A child conceived by the last male of a house that is extinct at the child's birth gets **no `house_member` row**: no house is resurrected. Remove the `person.unborn_father` read and the deferred `HOUSE_EXTINCT`. `on_death`, `house.extinct_at`, the `[[emits]]` and the envelope row stay as frozen. This keeps M7 0.5.0 independent of M2 0.3.0's freeze, and avoids the back-dating risk. | 75% |
| N7 (MIN) | **Ruled: criterion (a) names any holder of a title, office or claim, member or not.** Promote is widened to cover such a person. A houseless heir who succeeds is therefore named, which keeps SIM-9's "ruling houses … tracked". State this in INV-M7-06's text and the dormancy summary, as recorded amendments. D56's costs say that this path is the one difference between bounded and agnatic under T1. | 70% |
| N8 (MIN) | **Accepted.** D56's costs for (e) list:<ul><li>B6's back-fill change, and with it every back-filled genealogy, `fixture_worlds_v0.txt` and the population-synthesis goldens;</li><li>reopening REV-SPR-015's F6 and the meaning of `elite_fertility_mult`.</li></ul>D56 asks the director for direction on (e) only. If it is approved, (e) is its own M2 amendment PR with its own review, before SPR-015 freezes 0.3.0. Correct §8's B6 sentence. | 85% |
| N9 (MIN) | **Accepted.** Make every house-walk probe a possible history under T1. Nell becomes a named daughter of a house; the reviewer suggests Thorne. Make Xan–Tilda the same, or remove that probe. Check 5's "a possible history" must hold. | 85% |
| N10–N15 (NIT) | **Accepted.**<ul><li>N10: fix the stale texts.</li><li>N11: 1.116, and "within 1.1%".</li><li>N12: one interpolation for members and population.</li><li>N13: "a holder's own child, if the holder is male".</li><li>N14: document "append after" for repeated keys.</li><li>N15: restore Q11's numbering or cross-reference it, mention the read in the couplings `via` text (moot after N6), and state "60 lever lines".</li></ul> | 90% |

**Merge with main.**
- `origin/main` is `772478f`. It merges cleanly; N1 survives the merge.
- **SPR-014 may merge before your push.** It rewrites `specs/systems/next/amendments.lock.toml`, adds the next M2's 20th amendment against the frozen M2 0.2.3, and changes frozen M2 to 0.2.3.
- If `origin/main` contains SPR-014 when you push:
  - merge it;
  - regenerate the lock **with the tool**;
  - add any `[[amends]]` records the deny-by-default lint now asks for against frozen 0.2.3;
  - list them in the PR body.

**Round 3.** Apply everything above on `arch/M7-membership`, and push once. The same reviewer re-checks. D56 goes to the director only after that.

## 0c. Architect close, round 3 (2026-10-17)

The round-3 verification of `b38d10a` (§3) found **0 BLK, 0 MAJ, 1 MIN and 6 NIT**:
- `check_all --part core` passes 431 checks.
- The drafter's v (0.116 / 0.128) is confirmed by an independent Monte Carlo (0.1164 / 0.1277).
- The lint catches every frozen leaf that is not exempt: 264 of 271 in M2 and 626 of 630 in M7. The rest are exempt or checked by name.
- No M14 residue is left.

The Architect applied the remaining text fixes in the closing commit, so CI ran once:

| # | Disposition |
|---|---|
| P1 (MIN) | **Fixed.** The dormancy summary's precedence, Promote, world-start and named-set sentences now say "person" (Promote: "a member, or a holder of no house under (a)"). The Demote `why` reads "A named adult (a great-house member, or a holder of no house named under criterion (a))". The two `[[amends]]` records and their reasons follow, and the lock was regenerated by the tool. The `q5_why` control's plant is retargeted at the new text and still fails with its own diagnostic. |
| P2 (NIT) | **No change.** Round 2's N2 ruling named the frozen reading and the literal reading as the sensitivity lines; the eldest-surviving reading was the reviewer's round-2 estimate, not a ruled reading. |
| P3 (NIT) | **Fixed.** `fixture_worlds.md` points at `step()` and its comments. |
| P4 (NIT) | **Fixed.** `M7_membership.md` §4: in the model, agnatic and bounded differ only under hypergamy; in the engine they also differ on N7's path, which criterion (a) closes. |
| P5 (NIT) | **Ruled and fixed.** A frozen keyed entry cannot be withdrawn in a next version. Withdrawing one changes the frozen spec's surface, which needs the Architect's ruling and a new spec version. `next/README.md` says so. |
| P6 (NIT) | **Fixed.** The annals readability bar describes the current sample (two marriages, four births; Col walks to Ashcombe through his father; Bram has no house). |
| P7 (NIT) | **Fixed.** The projection's limits state the per-mother approximation, with the Monte Carlo's split by husband (0.111 / 0.122). |

**The exemption list (round 2, N4), ruled sound item by item:**
1. versions, status and fidelity;
2. lists that may only grow, each with its own check;
3. the `owns.*.values` prefix rule;
4. the `[algorithm.*]` tables' own rule;
5. `[[amends]]`;
6. plus P5's no-withdrawal line.

**M2 0.3.0's 26 newly recorded amendments are accepted.** Each was traced to SPR-015's births PR (#50) or SPR-016's mortality fold (PR #51/#59). There is no new semantic change, and the M2 next file's text is unchanged in this PR.

**Outcome:**
- The M7 0.5.0 draft carries **agnatic membership** (the father's house, else none; the back-fill's rule).
- Vocations go to later children. A child of the last male of a house that is extinct at the birth gets no row.
- Criterion (a) names any holder of a title, office or claim.
- `lint_odd --next` is deny-by-default.
- **D56** goes to the director, recommending agnatic with (d) in the M7 draft (6/10). D56 asks for direction on (e), which, if approved, is its own reviewed M2 amendment before SPR-015 freezes 0.3.0.
- BCK-002 A-56 and A-57 are resolved by this draft. A-59 is filed for SPR-021.

## 1. Round 1: the review of `f27409c`

#### BLUF

**Verdict: ready after fixes. Counts: 0 BLK, 2 MAJ, 6 MIN, 6 NIT.**

The drafted rule works as a plane-A spec:
- It is buildable and stays on M7's own columns plus M2 reads it already declares.
- It never reads T1, respects single-writer ownership and never reads plane B or C.
- It is deterministic, apart from six edge cases it leaves unstated (M5).
- Every gate passes: `bin/check_all.sh --part core` ran 415 of 415 PASS. The smoke, solo, household and hamlet engine baselines recompute equal, and no crate reads any file the PR changes.

D56 should not go to the director as written, for two reasons:
- **M1.** The projection behind "bounded" assumes everyone finds a spouse through M2's random partnering. Both fixture worlds register T1, and in a T1 world named persons marry only named house members. That mostly closes the bounded rule's path through the mother, and agnatic and bounded converge. M1 may change the recommendation.
- **M2.** The `HOUSE_JOINED` record, which is what makes the annals' house walk equal plane A, is not always admitted to the chronicle once the population passes D38's 10,000 ceiling. `kingdom_10k` passes it under the bounded rule's own projection.

#### Findings

| ID | Sev | File:line | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| M1 | MAJ | `specs/systems/next/M7_membership.md`:5, 21, 83, 111; `specs/worldgen/project_population.py`:47-50; `docs/decisions_pending.md`:14 | **The projection and D56 model only the T1-less regime.** Both fixture worlds register T1 (`twin_vales.yaml`:33, `kingdom_10k.yaml`:22). M2 N3/N4 keep named persons out of M2's partnering (next `M2.demography.odd.toml`:639-640, 371). T1's marry candidates are "unmarried named adults" only (`T1.minds.odd.toml`:903), and only house members get Promoted. So a named-eligible woman, who is exactly the bounded rule's mother, is named and can marry only a house member; her child takes the father's house under every rule. `HOUSE_JOINED` then fires only for a woman who became eligible after an M2 marriage. The same applies to the "daughter's son keeps the seat" argument: in a T1 world he belongs to his father's house under all three rules, so the "agnatic loses the seat one succession in five" point applies to every rule. | The PR's own probe cannot happen under these rules. Tilda, Hugh's daughter, is named from world start; Xan has no house, is not cast and so is never named (`gen_fixtures.py`:230-235, 398), so they cannot marry. My coupled-market variant of the drafter's recursion (`work/coupled_variant.py`): agnatic and bounded both reach 3.1 members per house (`twin_vales`, year 300), 1.3 (`kingdom_10k` uncapped) and 0.7 (capped). Cognatic still saturates. | Add a coupled-market variant to the projection and its golden: the named core marries within itself at the elite rate; inflow comes only from women eligible after an M2 marriage. Label which regime each number is for (today's numbers are M7's isolation run, T1 stubbed). Restate §5 and D56 for each regime, then re-derive the confidence. If agnatic ≈ bounded under T1, simplicity may favour agnatic. |
| M2 | MAJ | `specs/systems/next/registry_delta.toml`:28; next `M7.dynasty.odd.toml`:388; `specs/annals/annals_v0.md` §16 | **The record is not always admitted.** Under DATA-5/D38 (05.2 §4.0 criterion 1, line 181), act class `bond` is told only while the population is at or under 10,000; otherwise only with a named participant (T1's lagged `person.named`, or nobody without T1). The bounded projection puts `kingdom_10k` at 10,430 capped and 31,121 uncapped. A `HOUSE_JOINED` whose mother is not yet named then gets no event row, and the annals walk says "no house" where plane A wrote one. That breaks Q4's stated reason for the record. D38's premise that the dynastic 10k world is fully told also fails under bounded or agnatic, and D56 does not say so. | `population_projection_v0.txt`: "year 1000 population 10430". | Make `HOUSE_JOINED` always admitted: its own act class at 4294967295 in P1.9's `[event_rows]`, or carry the house on the born `LIFE_ENTRY`. State it in the `[[emits]]` `when` text and in annals §16, and add the D38 interaction to D56. |
| M3 | MIN | `specs/systems/lint_odd.py`:89-94, 296-308; next M7 :403, 408, 728, 760 | **Q5's lint gap is real and broader than stated.** Text the lint does not protect can change with no `[[amends]]` record. | Three planted changes in a scratch copy each lint PASS (rc=0): `on_birth` reverted to an unconditional mother fallback; Demote applied to minors; Demote winning for a far heir. A change on a protected path (INV-M7-06) fails (rc=1). A field-by-field diff shows the draft's unprotected changes are exactly the Promote and Demote `why` texts and the `on_birth` and `dormancy` summaries. The M2 draft also changes `[[couplings]]`, `[design]`, `[isolation]`, `[purpose]` and `params.*.source` unrecorded. Mitigation: the bounded rule and naming-wins are also stated in protected text (`house_member.house` meaning, INV-M7-06). "Minors beyond the degrees are not named at start" and "criteria (a), (c), (d) only" are not. | See the Q5 verdict. |
| M4 | MIN | next M7 :128-133, 582, 713-715, 756; `project_population.py`:49-51 | **The vocation reading drives every factor and is in conflict.** The 0.83 and 0.996 factors apply holy orders (v = 0.2) to all members, following the submodel ("not first in any title's line"). Three frozen texts and the source, RT-2026-10-16 item 12, say "later children". The schedule step says "place in a line". The conflict pre-dates this PR and is not in the specs table. | The drafter's own model at an effective v of 0.1 (first-born exempt): the member male line reproduces at 0.931, a member woman at 1.120. Cognatic then explodes (293,842 uncapped at year 1,000; no knife edge). Bounded holds 14–23 per house, agnatic 0.3–5.6. The ranking survives; the magnitudes and the cognatic "knife edge" do not. | Add the conflict to the specs table, state which reading the projection uses, and add one sensitivity line to the golden. |
| M5 | MIN | next M7 :125, 562, 728, 760; next M2 :590 (F9) | **Six edge cases are unstated.** (i) The mother died in the birth slot, which F9 allows: (d)'s line requires her alive, and (a)/(c) are unclear. (ii) `on_birth` runs before `on_death`, so a holder who died this instant still holds; say whether he counts. (iii) The order of newborns is unstated, yet `on_birth` now emits events and EventIds follow emission order (`on_death` states ascending TitleId). (iv) For (d), is the line the law's line or the line after M7's exclusions (holy orders, the deposed)? (v) A sortition line is a keyed draw at "the arbitration instant", which is undefined at `on_birth` or the yearly pass. (vi) Say that the mother branch never fires for a non-great house. | Text reading. | One clause each in `on_birth`, `dormancy` and the `house_member.house` meaning. |
| M6 | MIN | next M7 :760; `docs/03_requirements.md`:52 | **Criterion (d) is narrowed to the person's own house.** It names only someone first or second in the line of a title "a member of its house holds". The frozen text said "a ruling house's heir". An heir from another house or from no house is not named; in a T1 world that includes a holder's daughter's son. A sovereign's heir apparent can therefore stay un-named until he succeeds. | Invisible at genesis: all 39 genesis lines have `outside_house 0`. | Decide (d)'s scope explicitly. I recommend: any great-house member first or second in any title's line. Carry the decision into SIM-9 (Q11). |
| M7 | MIN | `specs/annals/selftest_annals_v0.py`:1624 | **The annals' new possible-history check for `HOUSE_JOINED` is weak.** It tests "the father has no house" as `row.father in w.members or row.father in c.joined`. That misses a father born during the run to a member. It also never checks that the mother has a house or that there is only one record per child, and no planted control exercises any of the three new rules. | `work/joined_father_probe.py` gives Bram an in-run son of a member as father (12313099716791886200): the check reports nothing, and `house_of` silently returns Ashcombe, ignoring the record. | Test with the walk (`house_of(father) is None`, `house_of(mother) is not None`). Add `fixture-joined-untriggered` and `fixture-joined-housed-father` controls to `check_all`. |
| M8 | MIN | `docs/decisions_pending.md`:14; `M7_membership.md`:5, 111-113 | **The D56 row overstates and omits.** (a) "Agnatic empties every house by year 300 to 500" contradicts the PR's own table: 1.7 per house on `twin_vales` at year 300; 0.4 per house on `kingdom_10k` at year 500 and about 0 by 750. (b) "6 to 11 per house at year 1,000": the 11 is `twin_vales` at year 300; at year 1,000 the figure is 6.4–7.7. (c) Costs not listed: membership now depends on title state; M2; Q5 must land before promotion; `dormancy_degrees` doubles as the membership lever (d). (d) Dropping it to 1, the fallback for the 500-named bound, cuts measured W_n from 1.00 to 0.40–0.60 per house and from 0.67 to 0.33 (`work/wn_degrees.py`), so bounded houses shrink by roughly half. (e) Alternatives not listed: agnatic with vocations for later children only, or with M2's elite fertility read from the couple rather than the woman. (f) "D40's revisit is not triggered": D40 sets no trigger; the revisit is scheduled for SPR-030 regardless. (g) The row sits between D54 and D55. | The golden and the D40 text. | Reword after M1. |
| M9 | NIT | `specs/chronicle/envelope_v0.md`:78-90; next M7 :640, 41-44 | The envelope's roles table has no `HOUSE_JOINED` row (it lists M2's 0.3.0 drafts). `[design].observation` omits the verb. "Nothing else of 0.3.3 changes" is stale. | | Add the row and fix both texts. |
| M10 | NIT | `registry_delta.toml`:28; `ashcombe_house-7252932463130708544.txt`:44 | The record's payload is empty. The sentence reads "received into the house of the second daughter of Hugh Thorne, the mother". | | Carry the HouseId, as `HOUSE_EXTINCT` does, so the sentence can name the house and the walk need not recurse into the mother. Fold into A-55. |
| M11 | NIT | `crates/rh-events/src/lib.rs`:51 | The `triggered` edge to M2's `BIRTH` needs M7 to learn another system's EventId in the same instant. No engine path for that exists today; `SUCCEED` to `DEATH` has the same gap. | | One item for SPR-021. |
| M12 | NIT | `specs/annals/fixtures/ashcombe/chronicle_A.txt` | Verb ids are indexes into the name-sorted verb list, so the new verb shifts every verb after it (MARRIAGE 29→30, SUCCEED 41→42, …). At promotion the stream goldens re-record. Plane-A ladders do not move, because the event hash keys on the kind name (`rh-events` lib.rs:371). | | Name it in the promotion cost. |
| M13 | NIT | next M7 :728 vs :760 | `on_birth` promotes a newborn on (d) only, while `dormancy` says "Promote … at this pass or at on_birth" for any criterion. | | Align the two texts. |
| M14 | NIT (pre-existing) | next M7 :384, 859 | A posthumous child of a house's last male joins an already extinct house, against `[isolation]` "no house is resurrected". | | One line while INV-M7-06 is being amended. |

#### Verdicts on the specs-table items

- **Q4: agree, 7/10.** A chronicle record is better than reconstructing titles in plane C, provided M2 (always admitted) and M10 (HouseId in the payload) are done. The alternative worth naming is the house on the born `LIFE_ENTRY`: always written, but a channel-A schema change (C3).
- **Q5: agree it is real, 9/10, but it is broader than stated.**
  - Make `lint_odd --next` deny by default: any changed field of the frozen slice outside an `[[amends]]` record fails.
  - At minimum cover:
    - `submodels.<name>.summary`
    - `commands.<target>/<command>.why`
    - couplings, `isolation.*`, `design.*`, `purpose.*`
    - `params.<name>.source`
  - Add planted controls.
  - File it as a BCK item and land it before SPR-015 promotes M2 0.3.0 (D54: before SPR-021), then re-derive both drafts' records with the tool. C1.
- **Q6: partly agree, 6/10.** Keep 1.2 / 0.2 and the elite read on the woman's row for now. Do not decide D56 on the random-mating model alone: add M1's coupled variant and resolve M4 first. List "elite read on the couple" as an alternative, not only as a later lever.
- **Q7: agree, 6/10.** No new parameter now, but state the coupling (M8 d) in `fixture_worlds.md` §6 and in D56.
- **Q9: agree, 9/10.** No M7 run exists before SPR-021, so the gap is theoretical.
- **Q10: accept, 6/10.** `bond` is today a pact class (`DESPAIR_PACT`); `birth` is semantically closer. Admission (M2) matters more than the class. Tellability 104 is reasonable.
- **Q11: agree to keep the house-scoped (c), 7/10.** The tension has two parts: SIM-9's "any" wording, and SIM-9's lack of any heir clause, so naming-wins contradicts SIM-9's literal "become dormant". Amend SIM-9 to "more than N degrees from every holder of a title, claim or office of their house, and not first or second in the line of a title their house holds", after deciding M6.

#### Checked and sound

- **Gates.**
  - Core: 415 of 415 PASS at f27409c.
  - Golden part: not run, because this container lacks `iai-callgrind-runner`. Instead I confirmed no crate reads a changed file, and recomputed the smoke (1,000 years), solo, household and hamlet baselines: all BASELINE OK.
  - The worktree is clean; I deleted my scratch build output.
- **Projection.**
  - `--check` PASS, and every number in §3 and in `fixture_worlds` §3.3 matches the golden.
  - `--derive-caps` reproduces W_n (5, 5, 4 over five houses; 8 over twelve).
  - Arithmetic: 0.8 × 1.2449 = 0.9959; 0.8 × 1.0341 = 0.8273; the bounded fixed point is 11.5 H·W_n, giving 57.7 and 92.2; the capped equilibrium is 10,433; the uncapped growth is ×3.13 over 1,000 years and +39% over 300.
  - The model implements the elite read on the woman's row (M2 F4) and the cognatic fallback correctly. The bounded fixed point is structural (a capped inflow against a proportional loss), not an artefact of discrete generations. Its size is linear in W_n, which is a genesis cross-section held constant and taken while offices are vacant.
- **The rule itself.**
  - It is evaluated at Day/Polity of the birth instant, as the first step, with same-instant reads of M2 and M7's own columns.
  - It reads nothing of T1 and draws no random numbers.
  - An unknown father falls to the mother's branch; a mother of no house gives no row; parents of different houses give the father's.
  - A newborn cannot displace its mother in any lineage law.
- **Claims.**
  - "No succession law reads house membership" holds (`effect_language_v0` §7.2 and §8; `effect_ref.py` has no house access).
  - A-57 agrees with the amended INV-M7-06, the Promote and Demote texts, and the fixture reference's `house_report`. The fixture golden is unchanged, `--tier pr` and the `dormancy-one` control pass, and the genesis counts are 33/35/26 and 65/71.
- **Annals.**
  - The renderer reads only channel A and the keyframe extract.
  - The `untold` and `solitary` goldens change only in the template version and digest; their chronicle digests are unchanged. `ashcombe` gains one sentence in three annals, with consistent EventId shifts.
  - All annals gates and controls pass, and the lint covers 31 templates.
- **`HOUSE_JOINED` declaration.** It is consistent between the Registry delta and M7's `[[emits]]`.
- **The `odd_next_control` change in `check_all.sh`.** It is necessary: without the M7 draft, the frozen M7 fails because the delta gives it a verb it does not emit. Each amended control still fails with only its own diagnostic.
- **Back-fill agreement** (B4/B6) holds.

All scripts and logs are in `the Architect's scratchpad: m7_review/work/`. The main ones:
- `check_report_core.md`
- `baselines.log`
- `coupled_variant.py`
- `vocation_sensitivity.py`
- `wn_degrees.py`
- `joined_father_probe.py`
- `fielddiff.py`
- the `q5_*` cases

## 2. Round 2: the verification of `0f7187b`

#### BLUF
**Verdict: not ready. 1 BLK, 2 MAJ, 6 MIN, 6 NIT.**

- **The blocker:** `bin/check_all.sh --part core` fails at the PR head. Bash treats the backticks in six new `run` lines as command substitution, so the `q5_summary` control fails. GitHub's check-run for `0f7187b` agrees: "PR tier (--part core)" concluded **failure**, and the golden part succeeded.
- **With the backticks escaped** (my scratch fix, not committed), core passes 428 of 428 and the four engine baselines recompute equal.
- **The agnatic recommendation stands.** Under T1, bounded adds only an inflow the model sets to zero.
- **D56's headline numbers do not stand.** The (d) lever's holy-orders share uses a wrong formula, which overstates (d)+(e) by 2.5 to 7 times. D56's D38 sentence is also inverted.
- The deny-by-default lint is still an allow-list with gaps. M14 adds new semantics without amending the texts that depend on them.

#### Findings
| ID | Sev | File:line | Problem | Evidence | Fix |
|---|---|---|---|---|---|
| N1 | BLK | `bin/check_all.sh`:1181-1183, 1190-1192; `python/tests/test_check_all_inventory.py`:605, 607, 615, 633 | Backticks inside double quotes (`` `why` ``, `` `via` ``, `` `source` ``, `` `to` ``) are run as commands. Three labels are mangled. The three q5 expected strings lose `to`, so they cannot match. | Core run: `line 1181: why: command not found` … `FAIL lint_odd next positive control: … q5_summary` (gate 239), rc=1, 73 s. The control itself fails correctly with "`to` is not what the next version says there". The drafter's "107 lines replayed" never went through bash. | Escape them as `` \` `` (or drop the backticks) and update the 4 inventory names to match. Done in scratch: 428/428 PASS. |
| N2 | MAJ | `specs/worldgen/project_population.py`:196-202 vs next M7 :757 (vocation) | `holy_share` = v(1 − 1/(2 NRR_c)) counts every woman born as a mother with a first-born. The amended submodel exempts "the eldest of his children by life_course.birth_tick", dead included. | I extended the M2 F1 widowhood chain with a birth counter: E[B] = 2.119 and 2.551, P(B≥1) = 0.505 and 0.511.<br>• Literal reading: exempt share 0.238 (commoner) and 0.200 (elite), so **v = 0.152 and 0.160**.<br>• "Eldest surviving to 16": v = 0.121 and 0.132.<br>• Drafter: 0.1033.<br>The (d)+(e) periphery product is 1.046 (literal) or 1.081, not 1.116. Capped 10k at y1000: **13.9 or 38.8 per house, not 97.4**. Growth is 0.15–0.26%/yr, not 0.4%; the member share is 1.6–4.4%, not 11%. The direction holds (product > 1). | Derive v from the chain under the submodel's stated reading, regenerate the golden, and restate D56, the `M7_membership.md` BLUF, §1, §4 and §8, and `fixture_worlds.md` §3.3. |
| N3 | MAJ | `docs/decisions_pending.md`:15 (D56, "D38:") | D56 says "the 10k world stays near the … tellability ceiling, 10,433 capped, so commoner acts stay told". That is inverted. | `05.2` §4.0 criterion 1 (:181) tells a class only while `living_at_start(y)` ≤ 10,000. Projected populations:<br>• Agnatic: 10,433 capped, 31,260 uncapped.<br>• Levers: 10,437–10,750 capped.<br>All are above the ceiling, so from about year 1 commoner acts are told only with a named participant. Only cognatic-capped (9,090) stays under. R2 asked for exactly this sentence. | Invert the sentence, and state that D38's "fully told 10k world" premise fails under every rule except cognatic-capped. |
| N4 | MIN | `specs/systems/lint_odd.py`:98-105, 308, 366-400; `specs/systems/next/README.md`:36 | "Deny by default" is an enumerated allow-list. Changes to these frozen fields are not detected:<br>• `owns.*.values` (enum codes: 6 M2 columns, 3 M7)<br>• `owns.*.registry` (4)<br>• `[[applies]]` `key`, `key_field`, `drained_by`, `outcome`, `sprint`<br>• `initialization.from_manifest`<br>• `input_data.drivers`<br>• every field of `[[reads]]`, which the drafter declares "free", an exemption the ruling did not grant | In-memory plant on every frozen leaf: M2 has 25 of 271 leaves uncaught, M7 84 of 630; the by-design ones are versions, `fidelity` and lists that may only grow. File-level plants each lint rc=0:<br>• an enum code renamed<br>• `drained_by` set to Year/Polity<br>• `from_manifest` edited<br>• `person.sex` read changed from same_instant to lagged | Walk all frozen leaves generically, with an explicit exemption list for the Architect to rule on, and add controls for `values` and `[[applies]]`. |
| N5 | MIN | next M7 :570 (Year step) vs :757 (vocation); `M7_membership.md`:22 | (a) The unamended schedule step says "without a seat, office, claim or place in a line" (the line reading), yet §1 fact 3 cites it as support for "later children".<br>(b) Under the amended text, a later child who is heir apparent or second draws holy orders (p = 0.2) and leaves every line. At e0 30, l(16) = 0.55, so the first-born is often dead and many heirs are exposed. Neither §5 nor D56 mentions this. | Text reading; l(16) from the chain. | "A later child not first or second in any title's line." This matches the step and R4's intent, with negligible effect on v. |
| N6 | MIN | next M7 :734 (`on_death`), :113, :386, :761; `specs/chronicle/envelope_v0.md`:84 | M14 defers `HOUSE_EXTINCT` to "the yearly dormancy pass", but four things were not updated:<br>• the dormancy summary and Year step do not carry that duty;<br>• the `house.extinct_at` meaning still reads "Sim-time its last living member died";<br>• the `[[emits]]` `when` and the role ("whose death ended the house") are unchanged;<br>• the envelope's `triggered`-to-the-death row is unchanged.<br>The deferred event's instant, its cause edge and the `extinct_at` value are unstated, which is a back-dating risk under ADR-0025. | Otherwise sound:<br>• `person.unborn_father` is M2 0.3.0's own plane-A column (next M2 :183), read `same_instant` as identity (:497-500).<br>• The wait is bounded (about 10 months) and deterministic.<br>But this is a new semantic beyond R14's "one line", ties M7 0.5.0 to M2 0.3.0 being frozen first, and is in neither §10 nor D56. | Amend the four texts, or rule the one-line alternative (no row for a child whose father's house is extinct). |
| N7 | MIN | next M7 :762, :398 (Promote); `M7_membership.md` §5 | Under agnatic membership a title can reach a houseless holder whom no criterion names, because Promote is for members only and R6 widened (d) to members. The path:<br>1. A periphery woman marries a commoner on M2's market while she is un-named.<br>2. She later becomes heir (primogeniture, partition or ultimogeniture, once the near line has died out, which the frozen-reading projection makes common).<br>3. Her children have no house, and her heir succeeds un-named, against SIM-9's "ruling houses … tracked".<br>This is also the one place bounded ≠ agnatic under T1. §5 assumes she was named when she married. | Text reading. There is no `title.house` column (next M7 :138-240). | State it in D56's costs. Either name any titleholder under (a), or rule what happens instead. |
| N8 | MIN | `specs/worldgen/population_synthesis.toml`:115 (B6); `M7_membership.md` §8 | (e)'s costs are incomplete:<br>• B6 mirrors F6 ("ordinary table c_i … because a wife married in has no house_member row"), so under (e) the spine wives draw c_i^e. That changes every back-filled genealogy, `fixture_worlds_v0.txt` and the population-synthesis goldens.<br>• §8's "B6 already notes the case" refers to a wife who has her own row, which is a different case.<br>• (e) also reopens REV-SPR-015's reviewed F6 and the `elite_fertility_mult` meaning (SPR-015 is still PLANNING). | Text reading. | List all of this in D56's costs; (e) needs its own review. |
| N9 | MIN | `specs/annals/fixtures/gen_fixtures.py`:237, 446-453, 466-472 (and 399-402) | Both house-walk probes are impossible histories in a T1 world.<br>• Cedric (holder again after deposing Wulf, so named) marries Nell, a houseless founder in a fixture with no cast. M2's N4 excludes named men, and T1 marries named persons only to named persons.<br>• Xan–Tilda (y20) is the same pattern, already shown impossible in round 1.<br>The walk test itself is valid, but check 5 claims "a possible history". | Specs T1 :903; next M2 :639-640. | Make Nell a daughter of Thorne (named). Col still walks to Ashcombe, and the probe then also shows the mother's house never passes. |
| N10 | NIT | `check_all.sh`:334-335; `project_population.py`:27; `fixture_worlds.md` §3.3; next M7 :52 | Stale text:<br>• the `HOUSE_JOINED` comment;<br>• "bounded (M7 0.5.0)";<br>• "the script's docstring states that recursion and the levers" (it does not; they are only in `step()` comments);<br>• "0.3.3's declarations" (the frozen spec is 0.3.4). | | Fix the text. |
| N11 | NIT | `M7_membership.md` BLUF, §1, §4; D56; `fixture_worlds.md` §3.3 | 0.8967 × 1.2449 = 1.1163, which rounds to "1.116", not "1.117" (moot after N2). "Cognatic within 1%": `twin_vales` is 1.1% off. | | |
| N12 | NIT | `project_population.py`:277-285 | The cognatic + (d) lines print more members than people (3,233 > 3,229; 261,028 > 260,970), because population is interpolated geometrically and members linearly. | Golden lines | Use one interpolation for both. |
| N13 | NIT | next M7 :730 | "which a holder's own child always is" is false for a female holder: her child takes the father's house. | | |
| N14 | NIT | `lint_odd.py` `keyed_paths` | Ordinals depend on order. Inserting a coupling with the same key before a frozen one fails ("…/in.via changed", "lag is missing"); appending after it passes. It fails closed, but this is undocumented. | Scratch plant | Document "append after". |
| N15 | NIT | `M7_membership.md` §10; BCK-002 A-59 | The specs table was renumbered: the Architect's Q11 (SIM-9) is now Q10, and "Q11" is now A-59. The new read is not mentioned in either spec's couplings `via` text. The PR claims "51 lever lines"; the golden has 60. | | |

#### Per-ruling verdicts
- **R1: partly applied.**
  - Done: the two regimes and the levers (a)–(g); lever lines in both regimes, except hypergamy in isolation, where nobody is named; every line labelled by regime; the recommendation re-derived from the T1 regime; the simpler rule preferred.
  - Not done right: lever (d) uses a wrong v (N2).
- **R2: applied (moot).** No verb is added. The bounded rule's requirement (an always-admitted act class at 4294967295) is recorded in `M7_membership.md` §2 and annals §16.
- **R3: partly applied.**
  - Done: generic paths; 13 controls, each producing exactly one ERROR, its own (verified); records re-derived; the lock is the tool's output (regenerated in scratch: identical except the command-path comment); M2's 26 amendments listed.
  - Not done: the lint is not deny by default (N4), and `check_all` fails (N1).
- **R4: partly applied.** The submodel is amended and the frozen reading stays in the golden's blocks. The amended submodel conflicts with the Year step and exposes heirs to holy orders (N5), and the projection's v does not match its text (N2).
- **R5: applied.** All six clauses are in `on_birth` (:730). Clause (i) is adapted to the agnatic rule.
- **R6: applied** in the dormancy summary (:762), the Promote `why` and the SIM-9 text.
- **R7: applied for the walk.** `house_of` (`selftest_annals_v0.py`:642-655) crosses in-run rows, and Col → Cedric → Osric → Ashcombe is pinned. The record controls are moot. The probe's marriage is impossible under T1 (N9).
- **R8: partly applied.**
  - Done: reworded; placed after D55; D40 sets no trigger; the rest of D56's numbers match the golden.
  - Not done: the golden's (d)+(e) is overstated (N2), D38 is inverted (N3), and the costs are incomplete (N7, N8).
- **R9: applied (moot),** with a residual NIT (N10). **R10: moot. R11: applied.** A-59 is a 4-column row and the version goes 1.18.0 → 1.19.0. **R12: applied** (§9). **R13: applied.**
- **R14: partly applied** (N6).
- **Q7: applied** (`fixture_worlds.md` §6, D56's costs). **Q11: applied.** The text equals the ruling's wording (now numbered Q10).

#### Projection under T1, agnatic: living members per house (reviewer's number vs drafter's)
I wrote the recursion independently from the model's equations and computed the rates myself from the chain: NRR_c 1.0341, NRR_e 1.2449, T 29.38, l(T) 0.4610.

| Lever (v) | product | `twin_vales` y300 | `kingdom_10k` y1000 uncapped | capped |
|---|---|---|---|---|
| frozen (0.2) | 0.827 | 3.3 / 3.3 | 1.3 / 1.3 | 0.7 / 0.7 |
| (d), drafter's v 0.1033 | 0.927 | 8.2 / 8.2 | 4.0 / 4.0 | 2.5 / 2.5 |
| (d), eldest surviving, 0.121 | 0.909 | 6.9 | 3.0 | 2.1 |
| (d), literal text, 0.152 | 0.877 | 5.1 | 2.0 | 1.5 |
| (e) | 0.996 | 10.7 / 10.7 | 8.5 / 8.5 | 2.8 / 2.8 |
| (d)+(e), drafter's v | 1.116 | 34.5 / 34.5 | 412.7 / 412.7 | 97.4 / 97.4 |
| (d)+(e), eldest surviving, 0.132 | 1.081 | 24.8 | 136.9 | 38.8 |
| (d)+(e), literal text, 0.160 | 1.046 | 17.7 | 44.9 (33,915 persons) | 13.9 (10,467 persons) |

- The first reviewer's 3.1 was taken at year 323, with no interpolation. At year 300 the figure is 3.3, so it reproduces.
- All nine isolation blocks equal the old golden line for line.
- **Model fairness.**
  - Optimistic: the v formula; every member of the named core marries at NRR_e, although T1 only marries within the court or relation set, excluding parent, child and sibling.
  - Pessimistic: W_n is a genesis cross-section, and the core counts 2·H·W_n persons although W_n counts only women aged 15–49. Doubling W_n gives frozen 5.0 / 2.6 / 1.2.
- **"Agnatic = bounded under T1"** holds by construction: the model's extra inflow is zero without hypergamy. The real difference is post-marriage eligibility (N7), small in members but not in kind.
- **The recommendation.** Agnatic at 6/10 is supported. The (d)+(e) magnitudes and the "elite overproduction, 11%" framing are not.

#### Amendment classification
**M2 0.3.0's 26 newly recorded amendments.** The M2 next file's text is unchanged in this PR (`8f7edb4` → `0f7187b` only adds amends #19–#44). Every text predates the PR: SPR-015 births (PR #50, `421f2e1`…`d6dbdb9`, `13fbb70`) or the SPR-016 mortality fold (`6c33f10`, `1d88ce9`). **There is no new semantic change.**

| Amendment | Class | Origin |
|---|---|---|
| `submodels.mortality.summary` | semantic restatement of `[algorithm.mortality_f1]` | fold |
| `apply_death.summary` | semantic restatement; mirrors the recorded `emits.EXTINCTION.when` | SPR-015 |
| `apply_death.earns_its_place_by` | editorial (+V-INV-03) | SPR-015 |
| `params.tfr.source`, `params.elite_fertility_mult.source` | editorial | SPR-015 |
| `fidelity_ladder.F1.representation` | descriptive restatement of F1 | SPR-015 and fold |
| `couplings.T1.minds/out.via`, `couplings.M7.dynasty/out.via` | descriptive (births coupling) | SPR-015 |
| `purpose.statement`, `purpose.not_yet` | scope | SPR-015 and fold |
| `isolation.expect` | semantic (F1 isolation gates) | SPR-015 |
| nine `design.*` | descriptive restatement of the F1 design | SPR-015 and fold |
| four `scales.*` | descriptive restatement | SPR-015 and SPR-018 |
| `verification.prototype` | editorial | SPR-015 |
| `system.requirements` (+SIM-8, SIM-9) | binding | SPR-015 |

**M7's ten amendments.**
- From SPR-019 and only recorded now: `initialization.state` and `tenure.title`.
- Semantic, new in this PR: `house_member.house` (A-56, agnatic), INV-M7-06 (A-57 and M14), `on_birth` (agnatic, R5, M13), `on_death` (M14), `vocation` (R4), `dormancy` (A-57, R6), and the Promote and Demote `why` texts (restatements).
- Every one traces to a ruling. Two go further than their ruling: `on_death`/M14 (N6) and `vocation` (N5).
- No field change is unrecorded. The only declaration added is the `person.unborn_father` read.

#### Checked and sound
- **No HOUSE_JOINED residue** in specs, the Registry delta, templates, goldens or the annals selftest. It survives only in the `check_all.sh`:334 comment and in the history prose.
- **Back-fill.** B4 and B6 agree with the agnatic rule.
- **Annals.** `gen_fixtures --check` and `selftest_annals_v0 --check` pass. All 38 annals and template-lint gates pass, including `text-truncated` at line 73. The lint covers 29 templates.
- **Agnatic text consistency.** `house_member.house`, `on_birth`, INV-M7-06, the Promote and Demote texts, annals §4 and §16, and the Registry delta all say "the father's house, else none". The `[isolation]` and `[design]` texts are unchanged and carry no cognatic residue.
- **Lint false positives.** Layout-only changes (blank lines, a comment, key order, a literal string) pass, and a field that is equal in both versions is never flagged.
- **D56 placement.** D56 sits after D55, and D40 sets no trigger. A-59 is well formed. The SIM-9 text equals R6 and Q11.

#### Gates run
- `check_all --part core` at `0f7187b`: **FAIL** at gate 239 after 238 PASS, 73 s.
- Same, with N1 patched in scratch: **428/428 PASS**, 780 s. Python: ruff, mypy, pytest 681.
- Golden part locally: `iai-callgrind-runner` is absent, so not run.
  - Release build took 1 min 10 s. smoke (1,000 years), solo, household and hamlet all report BASELINE OK; 123 s in total.
  - No crate reads any changed file, and no engine input changed.
  - CI's golden part succeeded at `0f7187b`.
- Trial merge of `origin/main` (`772478f`) into `0f7187b`: clean (`git merge-tree`). Only `check_all.sh` differs: the CRN gate moves to part golden, once. The inventory test passes 13/13 on the merged script. N1 survives the merge.
- Other runs: the lock regenerated in scratch (identical except the command-path comment); the leaf-plant sweeps; the first-born chain; the independent projection.
- Housekeeping: the worktree's `target/` is deleted and `git status` is clean.

Scripts and logs are in `the Architect's scratchpad: m7_review/work2/`:
- `scripts/indep_proj.py`
- `scripts/firstborn2.py`
- `scripts/plant_all_replace.py`
- `scripts/plant_files.py`
- `check_core.log`
- `check_core3.log`
- `bl.log`

## 3. Round 3: the verification of `b38d10a`

#### BLUF
**Verdict: ready after one text fix. 0 BLK, 0 MAJ, 1 MIN, 6 NIT.**

- **Gates.** `bin/check_all.sh --part core` passes 431, with 0 failures and 1 SKIP (part golden), in 770 s.
- **v is right.** The drafter's v = 0.116 / 0.128 under the ruled reading is correct; my round-2 0.121 / 0.132 was a different reading. My own Monte Carlo of the chain reproduces their numbers, and the golden lines and D56's numbers match.
- **The lint.** It now catches every frozen leaf that is not exempt. The four new controls each fail with only their own diagnostic. I found no false positives.
- **M14 is gone.** No residue remains in the draft, the envelope, BCK-002 or D56.
- **The one MIN:** N7 is applied in criterion (a) and the Promote `why`, but the dormancy summary's own Promote and Demote sentences still say "member" only.

#### Findings
| ID | Sev | File:line | Problem | Fix |
|---|---|---|---|---|
| P1 | MIN | next M7 dormancy summary (:761); Demote `why` (:403-406) | N7 is only partly applied. Criterion (a) "applies to any person", but the same summary still says:<br>• "Promote: a member not named … that meets a criterion"<br>• "At world start the pass names exactly the members meeting a criterion"<br>• "The named set after a pass is … the members meeting a criterion plus the named minors"<br>The Demote `why` still reads "A named great-house member, an adult". So the pass's own text never promotes a houseless holder. If he is named anyway (the Promote `why` and INV-M7-06 say so), he is never demoted after he is deposed or resigns, so the named set leaks. | Change "a member not named" to "a person not named", and "the members meeting a criterion" to "the persons meeting a criterion". In Demote, use "a named adult (a member, or a holder of no house) meeting none". Re-run `--write-amendments-lock`. |
| P2 | NIT | `golden/population_projection_v0.txt`:332-337 | N2 kept "your two readings" as sensitivity lines, but only `literal` is in the golden. The eldest-surviving reading (24.7 / 136.9 / 38.8) is not. | Add the second reading's lines, or record that it is dropped. |
| P3 | NIT | `specs/worldgen/fixture_worlds.md`:114 | N10 is only partly applied. The text still says "the script's docstring states that recursion and the levers", but the module docstring of `project_population.py` has no T1, core, lever, couple or hypergamy text (checked with `ast.get_docstring`). | Add the T1 recursion to the docstring, or point at `step()`. |
| P4 | NIT | `M7_membership.md` §4 "Reading" vs `decisions_pending.md`:15 | §4 says "Agnatic and bounded differ only under hypergamy". D56 says the houseless-heir path "is the one difference between bounded and agnatic under T1". | "In the model, only under hypergamy; in the engine, also the post-marriage-eligibility path (N7)." |
| P5 | NIT | `lint_odd.py` `frozen_leaves`; `next/README.md` exemption row | A frozen keyed entry can no longer be withdrawn at all. Removing the `person.sex` read gives three "… is missing from the next version" errors, and no `[[amends]]` path expresses a withdrawal. This fails closed, but the exemption list does not say so. | Add one line for the Architect: entries cannot be withdrawn, or here is how a withdrawal is recorded. |
| P6 | NIT | `annals_v0.md`:175 (readability bar) | Stale since round 1. It says Ashcombe's annal has "a marriage, three births" (the golden now has 2 marriages and 4 births). It also says Thorne's annal holds "a houseless founder's son who joins Thorne through his mother", which is false under the agnatic rule; Bram's mother is now Sibby. | Re-describe the sample. |
| P7 | NIT | `project_population.py` `holy_share` and `vocation_shares` | "Same father" is modelled per mother. My Monte Carlo split by husband gives v = 0.111 / 0.122. A father's children by other wives (widowers who remarry) push the other way, and neither effect is modelled. | Add one line to the limits section. |

#### Per-ruling verdicts
- **N1: applied.**
  - No `run` line in `check_all.sh` has an active backtick: the remaining hits are a comment (:519) and a single-quoted block (:773-781).
  - The q5 expectations are single-quoted, the inventory test passes, and the full core run passes.
- **N2: applied** (P2 and P7 are NITs).
  - My Monte Carlo runs 400k women per level over two seeds, using `Chain`'s own transitions with birth months recorded.
  - It gives E[B] = 2.123 / 2.554 and P(B≥1) = 0.505 / 0.512.
  - Ruled reading: v = **0.1164 / 0.1277** (seeds agree to ±0.0001); the golden has 0.1161 / 0.1275.
  - Literal reading: 0.152 / 0.160.
  - **Why my round-2 figure differed.** My 0.121 / 0.132 exempted only the eldest child who *reaches* 16. The ruled reading also exempts a younger child whose elder siblings reached 16 but are dead by the younger child's majority, and it can exempt several children in one family. That lowers v. The drafter's independent-gap convolution is accurate to about 0.0003; the difference comes from the reading, not from the convolution.
- **N3: applied.** D56 now says commoner acts are told only with a named participant above 10,000 (10,433 capped, 31,260 uncapped), and that the premise holds only under cognatic-capped (9,090).
- **N4: applied.** My leaf-plant sweep at `b38d10a`:
  - M2: 264 of 271 leaves caught. The 7 not caught are `algorithm.version`, `system.fidelity`, `system.id`, `odd_r`, and three plants that only add to an empty or growth-only list.
  - M7: 626 of 630 caught. The 4 not caught are `system.fidelity`, `system.id`, `odd_r` and one roles addition.
  - `odd_r` has its own check: "odd_r must be 2".
  - The four controls each produce exactly one ERROR, their own: `owns.title.state.values changed`, `applies.PressClaim.drained_by`, `initialization.from_manifest`, `reads.person.sex.lag`.
  - False-positive probes all pass:
    - an enum code appended;
    - `[[reads]]` blocks swapped;
    - `[[applies]]` blocks swapped;
    - keys reordered inside an M2 `[[owns]]` entry;
    - blank lines and a comment.
  - Real changes fail: an enum reorder, and a schedule `when` change.
  - The lock was regenerated by the tool and is identical apart from its command comment (55 paths: M2 45, M7 10).
  - Self-check counts: M2 46 planted (+3; M2 has no frozen `[[reads]]`, so it gets three new probes, not four); M7 42.
- **The exemption list, item by item:**
  1. `spec_version`, `status`, `fidelity`: **sound.** `fidelity` is a scalar level, and `id` and `crate` are checked by name.
  2. Lists that may only grow: **sound.** Each has its own check: a roles shrink fails, frozen tests must remain, and dropping an `applies.from` sender is a self-check probe.
  3. `owns.values` prefix rule: **sound.** Appending passes; renaming and reordering fail.
  4. `[algorithm.*]` tables: **sound.** Only `version` is free, and it moves with an amended step.
  5. `[[amends]]`: **sound.**
  6. P5 (entries cannot be withdrawn) needs one line.
- **N5: applied.** The submodel and the Year step (:570) say the same thing, and the Year step has its own record, `schedule.Year/Polity.steps.vocation`.
- **N6: applied.**
  - `on_birth`, `house_member.house` and INV-M7-06 carry the no-row rule (`house.extinct_at = 0`).
  - `on_death`, `[[reads]]`, `[[couplings]]` and `[[emits]]` are byte-equal to the frozen spec.
  - `unborn_father` appears nowhere outside M2, apart from history prose in `M7_membership.md` §7 and §13.
  - The same-instant case is consistent: `on_birth` runs before `on_death`, so the newborn keeps the house alive.
- **N7: partly applied** (P1). Criterion (a), the Promote `why`, INV-M7-06, `on_birth` and D56's costs are done.
- **N8: applied** (D56, §4, §8, Q4; the B6 sentence is corrected).
- **N9: applied.** Both marriages are now possible under T1:
  - Xan and Sibby are un-named and houseless, so they marry on M2's market.
  - Nell is named: she is Giles's sister, within 2 degrees. Cedric is named: he holds the seat again from y25. Nell is Cedric's maternal aunt (Ysolde's sister), so she is in his relation set; T1 excludes only parent, child and sibling.
  - An aunt–nephew match is legal under T1's rule; noted only because a reader may find it odd.
  - Check 5 and its two controls pass.
- **N10: partly applied** (P3).
- **N11, N12, N13, N14, N15: applied.** The cognatic + (d) lines now have members equal to the population; "1.1%"; "if the holder is male"; the README states "append after"; Q10 is cross-referenced to round 1's Q11.

#### Numbers: T1 regime, agnatic, living members per house
I re-derived these with an independent recursion that splits v by the mother's chain and interpolates members geometrically. Each cell is mine / the golden's; populations for (d)+(e) are 2,318 / 38,165 / 10,568 against the golden's 2,319 / 38,200 / 10,569.

| Lever | `twin_vales` y300 | `kingdom_10k` y1000 uncapped | capped |
|---|---|---|---|
| frozen | 3.3 / 3.3 | 1.3 / 1.3 | 0.7 / 0.7 |
| (d), ruled reading | 7.0 / 7.0 | 3.0 / 3.0 | 2.0 / 2.0 |
| (e) | 10.7 / 10.7 | 8.5 / 8.5 | 2.8 / 2.8 |
| (d)+(e), ruled reading (product 1.086) | 26.0 / 26.0 | 161.4 / 162.4 | 44.8 / 45.1 |
| (d), literal | 5.0 / 5.0 | 1.9 / 1.9 | 1.5 / 1.5 |
| (d)+(e), literal | 17.7 / 17.7 | 44.9 / 44.8 | 13.9 / 13.9 |

- The small (d)+(e) gaps come from my v_e being 0.1277 against the golden's 0.1275.
- D56 checks out: 0.91 alone; 1.086; about 45 per house capped and 162 uncapped; 5.1% of the world; 0.28%/yr; 14 / 45 under the literal reading; 2.8 / 8.5 for (e) alone; cognatic + (d) 122,429.
- The §4 hypergamy rows match the golden: (d)+(f) 2.5 / 0.0 / 0.0 agnatic and 6.4 / 2.4 / 1.6 bounded; (d)+(e)+(f) 17.7 / 96.6 / 20.6 and 26.0 / 162.4 / 45.1.
- No stale numbers remain (I searched for 0.103, 1.117, 412.7, 97.4, 11%, 0.4%).
- The projection gate passes in 5.6 s.

#### Gates
- **Core:** `bin/check_all.sh --part core` at `b38d10a`: **431 PASS, 0 FAIL**, 1 SKIP (part golden), 770 s. That includes all 22 next-lint controls, the projection, the fixture worlds, the annals fixtures, the selftest and its controls, and pytest.
- **Baselines:** not recomputed. No engine input changed since `0f7187b`, where all four recomputed equal: `crates/`, `worlds/`, the frozen specs and `specs/manifest` are untouched, and `772478f` changed only `check_all.sh`.
- **Housekeeping:** the worktree's `target/` is deleted and `git status` is clean.

Everything is in `the Architect's scratchpad: m7_review/work3/`:
- `vocation_mc.py`
- `indep_proj2.py`
- `plant_all_replace.py`
- `fp_probes.py`
- `check_core.log`
- `check_report_core.md`
