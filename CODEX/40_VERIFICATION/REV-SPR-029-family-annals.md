---
id: REV-SPR-029
title: "Independent review of the family annals v0 spec (PR #69)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-029, SPR-019, SPR-020, SPR-021, SPR-030, REV-SPR-019, REV-SPR-020, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 0.3.0
---

> **BLUF: round 3 on `e3fa495`. Nearly ready: no blocker, 1 major (a non-positive motive edge still reads "moved by"), 4 minors and 3 nits** (§3). Every round-2 mutant now fails by name. Rulings R31–R38 are in §0c.
>
> **Round 2** on `214fbd8`. Changes are needed before the merge: no blocker, 3 majors, 7 minors and 5 nits** (§2). An independent renderer reproduces all 15 golden files, and all 27 gates pass with their controls. The majors:
> - the house walk is not M7's rule;
> - check 1 re-derives `S` rows only;
> - the motive can name an inhibitor.
>
> Rulings R18–R30 are in §0b.
>
> **Round 1** on `5509079`: changes were needed before the merge (no blocker, 10 majors, 11 minors and 5 nits).
> - **What holds.** An independent renderer written from the contract reproduces all ten golden files once three unstated layout choices are adopted. Rendering is byte-stable across hash seeds, locales and row orders. All 15 gates pass, and every lint control fails on its own diagnostic. Every slot binds to something its emitter declares, and nothing feeds plane A.
> - **The majors.**
>   - The contract does not fully determine the output files.
>   - The V-NAR-05 probe checks the sidecar's form, not its truth.
>   - A cause of an out-of-scope kind cannot be rendered.
>   - House membership contradicts M7's rule.
>   - Events with no cast drop silently out of house annals.
>   - Designation does not identify a person, and it hides kinship, which is the G1a reading's main weakness.
>   - "Under strain" is an invented fact.
>   - The motive is taken from the wrong edge.
>   - The sidecar does not say what it was rendered from.
>   - The fixture is not a possible history.
> - **Rulings:** R1–R16 in §0.
> - **Reviewer:** a session on a different model from the drafter's.

# REV-SPR-029: independent review of the family annals v0 spec

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | F1 | **Accepted.** The contract states the text grammar exactly: the two title lines for each group kind, the blank line before each `##` heading, and the capitalisation of a sentence's first character. `templates_digest` prints as the integer's 16 hex digits, like `digest` and every `hash64` in the repository. Ids sort numerically and sources bytewise. Each prologue row names its persons and sources, and the contract lists the records rule 4's house walk cites. Done means an independent renderer written from the text alone reproduces every golden file | 9/10 |
| R2 | F2; D50's text | **Accepted.** Check 1 re-derives the sidecar instead of checking its form:<ul><li>each in-scope event of the group is the own event of exactly one `S` row, of its kind's template;</li><li>a row's events are its own event plus that sentence's told causes;</li><li>`P` rows come only before the first year heading, with no events, and every `kf:` source exists in the extract;</li><li>a row's persons are its cast plus the parents that designation reaches.</li></ul>Mutants P1 and P3–P6 must each FAIL by name. Correct D50's text, which cites §12 and not §9, before the director reads it | 9/10 |
| R3 | F3 | **Accepted.** Add a `[[label]]` table: a noun phrase for each out-of-scope kind that an in-scope kind's edge may point to (`MURDER_ATTEMPT`, `MENTAL_BREAK`, `FLED`, `HOLDINGS_BURNED`, `DESPAIR_PACT`, and any other the specs allow). The lint requires one for every such kind, derived from the edges and causes the emitters declare. Check 1 accepts a cited cause of any kind | 8/10 |
| R4 | F4; decision 5 | **Accepted.** One function for grouping and designation, M7's `on_birth` rule: the keyframe row; else, for a birth during the run, the father's house if he has one; else the mother's. §4 and §5 both cite it | 8/10 |
| R5 | F5 | **Accepted.** An event with no person in its cast joins a group when a told `triggered` edge leads to an event of that group (`INTERREGNUM_CRISIS` to the holder's `DEATH`, `EXTINCTION` to the last `DEATH`). Otherwise it is counted in the trailer by kind, never dropped silently. Correct §12's claim that the sample tells an interregnum | 8/10 |
| R6 | F6; decision 5; the G1a reading | **Revised designation, so that kinship is visible:**<ul><li>a name first;</li><li>else the father first: "the second son of Edric Ashcombe", by birth order among the parent's children of the same sex, read from the life rows;</li><li>else the mother;</li><li>else the id.</li></ul>A child is never designated by the birth sentence being told. The birth sentence names the father, from the child's `LIFE_ENTRY` (decision 1: a declared `father` slot on `BIRTH`, cited `life/<pid>`). A check refuses two persons of one annal with the same designation. Use H64-sized ids in the fixture (F21), so the golden shows what the id fallback will look like at G1a | 8/10 |
| R7 | F7; Q2 | **Removed from v0:** no "under strain" and no stress number in prose. A state word comes back only as a declared derivation against a Registry threshold, cited in the sidecar | 9/10 |
| R8 | F8 | **Accepted.** The motive is the one on the `motivated_by` edge with the largest absolute weight, ties to the lower cause id. A `when` key separates a named motive from an unnamed one, so an unnamed motive has its own sentence | 8/10 |
| R9 | F9; decision 4 | **Accepted.** The sidecar header names the timeline id, the chronicle's canonical digest (the segments' digests, in order), its `registry_version`, the keyframe extract's digest and genesis `state_hash`, the `manifest_hash` behind the names file, and the template set's version and digest. The renderer refuses a mismatch between the Registry it reads and the chronicle's `registry_version`. §1 and §9 list the Registry as an input. Decision 4 is accepted with these conditions: the extract gets a schema and a version, and is bound by digest. It stays a plane-C input file: REV-SPR-019 R1 ruled that back-filled persons have no channel-A records, and SPR-013 now writes segments, so channel A v0 is no longer amended in place | 8/10 |
| R10 | F10 | **Accepted.** Correct the ten impossible records, or derive the fixture by hand from M2's and M7's rules (gestation, no pending pregnancy at genesis, claims at a succession or a deposition, the vacant seat's `SUCCEED` or `INTERREGNUM_CRISIS`, a deposer who still holds a claim, the interregnum's title payload). Each header's `registry_version` names the dictionary its verb ids come from (a test overlay is acceptable if it is named). §12 says what check 5 checks. The reviewer's `fixture_check.py` is a fair model for a plausibility check in the reference | 8/10 |
| R11 | F11 | **Delete the folding sentence.** Instead, define a small plane-C annals manifest: the groups in (kind, numeric id) order, each with its `annals_digest`, and its own digest. State that `STEP_DIGEST`, a channel-A record written by the engine, never covers an offline artefact. How a timeline's `narrative_hash` folds the annals is P2.8's question (`05.11` §9) | 8/10 |
| R12 | F12–F19 | **All accepted.**<ul><li>F12: the payload citation is clause-exact, one slot for each comma-separated item.</li><li>F13: enum ids are checked against the emitter's column codes.</li><li>F14: conversions and format specs are refused; prologues go through the same substitution function; a non-final unconditional variant is flagged; `event` and `witness` are either defined in the renderer or refused by the lint.</li><li>F15: an untold cause records `untold/<id>` as §5 says, with a fixture case.</li><li>F16: after rendering, `sys.modules` holds no network or process module, with a control that imports one inside a function; drop `random` from the allow-list.</li><li>F17: the extinction sentence cites the `DEATH` through `{cause.triggered}`.</li><li>F18: codes outside the enum, and calendars other than 360 or 365 days, are refused by name.</li><li>F19: rule 1 names designation as its one declared exception and lists its sources</li></ul> | 8/10 |
| R13 | F20; Q3 | **Accepted.** The `no-event-id` control plants at the end of the annal, keeps determinism out of its path, and the gate matches the exact problem line | 9/10 |
| R14 | F22–F26 | **Accepted:**<ul><li>variants keyed on the edge or the role (deposed "by" versus "in favour of"; a feud ended by a death versus by reconciliation; a pressed claim with no holder);</li><li>the wording slips, with ordinal headings in one style;</li><li>the two text slips;</li><li>`[[emits]]` keyed by (system, kind), with duplicates refused;</li><li>a `verify` mode that recomputes a committed sidecar's digest</li></ul> | 8/10 |
| R15 | Decisions 1–3, 6, 7; Q1 | **Decision 1** is accepted with R6's `father` slot. **Decision 2** is accepted for v0: succession sentences name no title until M7 declares typed payloads. The Architect files **BCK-002 A-55**: a typed `payload = [...]` list on `[[emits]]` in `lint_odd.py`, declared for M2, M7 and T1, which lands before SPR-021. The annals bind to typed payloads once it lands. It is not added to PR #62, which is verified and waiting to merge. **Decision 3** stands as a stopgap, made clause-exact by R12, until A-55. **Decision 6** is accepted for P1a; correct the citation of `05.4` §5, and make R5's trailer the explicit record of what is left out. **Decision 7** is accepted, restricted by R12. **Q1:** keep the prologue | 8/10 |
| R16 | D49, D50 | **D49 (names):** revise the recommendation to a plane-C name generator scheduled in P2.8a, before the G1a reading, not at P2.8. Real ids are 64-bit, so without it G1a's annals would print 20-digit numbers. The generator is deterministic and keyed (`H(world_seed, person_id, "name")` over a culture's name base). Give its cost in agent-days and its reuse candidates (`docs/14`). The contract defines designation's hook for a generated name, not the generator. **D50:** file it once R2's probe enforces what its text says, worded as the reviewer proposes. Both are confidence-rated for the director | 7/10 |

## 0b. Architect rulings, round 2 (2026-10-17)

Round 2 was reviewed on `214fbd8` by an independent session on a different model from the drafter's (§2). R1–R17 hold or partly hold, as §2's table shows. The drafter's calls 1–7 inside the rulings are accepted. These rulings are binding for round 3.

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R18 | G1 | **Accepted.** `house_of(p)` is exactly §4 rule 4. It is the keyframe row. Else, for a birth during the run (an in-run `LIFE_ENTRY` of origin `born`), it is the father's house if that is not none, else the mother's. Else it is none. There is no recursion into a genesis person's ancestry. Add the probe's case (a genesis grandson through a daughter, and his in-run son by a Thorne) to `gen_fixtures`. The Architect files the disagreement between the agnatic back-fill and M7's cognatic fallback for SPR-019's code sprint and SPR-021 (BCK-002) | 85% |
| R19 | G2; D50 | **Accepted.** Check 1 re-renders the whole group from its inputs (prologue, sentences, trailers, header). It compares the sidecar row by row and field by field, in list order, and names the first field that differs. D50 is enforced as ruled: a `P` row's sources are the keyframe rows it was rendered from. Committed controls: P12 (an altered prologue text), P13 (another house's root cited) and P7b (a forged source), each FAIL by name. State in §11 that check 1 shares the substitution function with the renderer, so it guards the sidecar against the text; the goldens and the independent renderer guard the contract | 90% |
| R20 | G3 | **Accepted.** The motive is the motivated contributor with the largest *positive* weight, ties to the lower cause id. When no weight is positive, the sentence renders without a motive (the unnamed variant). The envelope's coalescing rule has the same exposure: the Architect amends `envelope_v0.md` §5 through SPR-020, so that the coalesced edge's motive takes the sign of the summed weight. That is outside this PR | 75% |
| R21 | G4 | **Accepted, no golden change.** (a) A row's persons are the persons the sentence designates (its roles, its cause labels and a birth's father), plus the parents a designation reaches. (b) A sibling rank's sources are the rows of every child of that parent of the same sex. (c)–(j): state each rule. Ids are 16 lowercase hex digits, zero-padded, and a prefix is taken from that form. Trailer kinds are in name order. Words for numbers are covered by R25. An unnamed house or title is refused by name (R27). Castless chains follow R22. The manifest lists the houses, then `line:<min named id>`. The extract digest follows R23. `chronicle_digest` is the segments' canonical digests in segment order, folded as §9 states | 85% |
| R22 | G5 | **Accepted.** Castless events join in ascending id order, and a castless event that has joined counts as a member event for later ones. Triggered edges point to earlier ids, so one pass in id order computes it | 80% |
| R23 | G6 | **Accepted.** The extract digest is the XXH3 of a canonical byte form: each table's rows sorted by key, one canonical line each, in a stated table order. A shuffled extract gives the same digest; add that as a check. `manifest_hash` is `sha256:<64 hex>` as `rh-golden` prints it; correct the fixture | 85% |
| R24 | G7 | **Disambiguate, don't refuse.** D49's generator makes shared names routine, so refusing would break normal use. When two persons of one annal share a designation, each is followed by its compact label, as in "Maud (p-4a8f)". Check 9 then holds by construction and stays as the guard. Add a control with two persons named Maud | 75% |
| R25 | G10 | **Words to 999,999**, British style ("one thousand and one"), because 1,000-year runs are routine from G1a's performance run. Add known answers (1000, 1001, 1100, 2024, 999,999) to check 4. A larger number is refused by name | 85% |
| R26 | G8 | **Accepted, all three.** The deposition moves to day 2 or later. Drop claim 31, or re-trigger it from a passed-over heir. Check 5 enforces M7's `when` exactly (claims come only from a passed-over heir or a deposition) and the deposition day, each with a committed control. Founder indices: the cast first, as `rh-worldgen` issues them. **Display is 1-based:** `Founder #k` prints k + 1, because readers count from one; the sidecar keeps the full id | 80% |
| R27 | G9, G12 | **Accepted.** Each prologue template has its own vocabulary. Versions are integers, ids must be known, and control characters are refused. A raw id never reaches the text: `house #<id>`, `title #<id>` and the "event N" fallback are each refused by name | 85% |
| R28 | G11 | **Uniqueness over the run's persons:** the keyframe extract's persons and every person the chronicle names. A person's label is then the same in every annal rendered from one chronicle. It may lengthen as a live chronicle grows; state that | 70% |
| R29 | G13, G14, G15 | **All accepted.** Fix the control hygiene: plant P1 for `row-drops-own` and an existing event for `unrelated-event`; give each lint control exactly one diagnostic; commit a check-5 control; run check 6's two renders in two processes under different `PYTHONHASHSEED` values. Match the heading `## Before the chronicle began` exactly. Fix the text slips. §1 states that a chronicle under the frozen dictionary cannot be rendered while the delta exists | 90% |
| R30 | Drafter Q4, Q5 | **Q4:** the broad label set (every out-of-scope kind any `[[emits]]` declares) is accepted. **Q5:** re-derivation is enough for v0. The tool recovers k by re-running `founder(seed, k)` with the deterministic rehash over the genesis rows. `life_course.founder_index` would be an M2 amendment that moves every hash (C3) and is not justified | 75% |

**Round 3:** the drafter applies R18–R30, regenerates the goldens with the tool, and re-runs `check_all --part core`. The Architect verifies round 3 with the round-2 scripts (`work_r2/`: `mutants.py`, `probes.py`, `compare_indep.sh`, `fixture_check.py`, `lint_probes.py`, `determinism.sh`).

## 0c. Architect rulings, round 3 (2026-10-17)

Round 3 was verified on `e3fa495` by the round-2 reviewer, with its scripts re-targeted by row content (§3). Results:
- **Gates:** all 36 pass. Each lint control gives exactly one diagnostic; each self-test control fails its own check.
- **Mutants:** all 28 round-2 mutants fail check 1 by name.
- **Contract:** the 6 annal texts reproduce from the contract alone.
- **Determinism:** renders hold across hash seeds, locales and shuffled extracts.
- **Fixtures:** `fixture_check.py` finds 0 problems.
- **Rulings:** R18, R22, R25, R26, R27 and R28 hold. The rest hold partly, for the reasons below. Drafter call 2 (the infant death that keeps Thorne's extinction) is accepted.

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R31 | H1; drafter call 1 | **Accepted as the reviewer proposes.** A `motivated_by` edge is chosen only if its weight is positive: the largest positive, ties to the lower `cause_id`. An edge of weight zero or less never moved the actor (envelope §5). With no positive edge, no edge is chosen and the kind's plain variant renders. `when.motive = "unnamed"` means the strongest positive edge carries no Registry motive. Add a fixture `FEUD_DECLARED` whose only told edge has a weight of zero or less, so that `feud.declared` is exercised | 80% |
| R32 | H2 | §9's text drops the `\n` from the extract's domain prefix, to match the code and the goldens. No golden changes | 95% |
| R33 | H3 | **Disambiguation is run-wide, which matches R28.** A name borne by two persons of the run is followed by its bearer's compact label. A kin designation whose parent's name is shared is followed by the *designated person's own* label, and the parent stays a bare name inside the phrase. The label cites its source row. Add a golden case with two fathers who share a name | 75% |
| R34 | H4 | Check 1 reports a text shorter than the render ("line N: missing: …"), with a control | 95% |
| R35 | H5 | §1's keyframe-extract row states R30's re-derivation of k | 95% |
| R36 | H6 | Fix the text slips: the deposition's day of the *month*; three births with their father named; three sibling references; §4 rule 2 worded as §8; §8's fold of the segment digests worded as §9 | 95% |
| R37 | H7 | The three R19 controls plant the defects R19 names: an altered prologue text, another house's root, and a forged source row. `fixture-deposed-day-one` moves each deposition's claims with it. The header message names the differing token | 85% |
| R38 | H8 | The person pool includes the parent ids of `LIFE_ENTRY` records. A father named in a birth sentence is then always in the pool, and his label stays unique | 70% |

**Round 4:** the drafter applies R31–R38, regenerates with the tool, runs `check_all --part core`, and pushes once. The Architect verifies round 4 with `annals_review_r3/work_r3/`, then closes the review.

## 1. The review (round 1, independent reviewer)

Copy reviewed: a git archive of `5509079` in the Architect's scratchpad `annals_review/`; its `work/` scripts reproduce each finding.

*Adversarial review for the Architect, 2026-10-17. Source: the git archive of the PR head in `src/`. Every script cited below is in `work/` and runs from there (`python -I <script> ../src`, or `./<script>.sh`). Nothing in the repository was modified.*

### BLUF

**The PR is not ready to merge as the contract the P2.8a tool will be built against.** Counts: **BLK 0, MAJ 10, MIN 11, NIT 5.**

What works is the core machinery:
- The reference renders its fixtures exactly.
- My independent renderer, written from the contract and the TOML, reproduces all 10 golden files byte for byte.
- The 15 gates, the controls and the inventory test pass.
- The output does not depend on dict order, hash seed or locale.
- Nothing reaches plane A.

What fails is the contract and its probes:
1. **The output cannot be reproduced from the contract alone.** The goldens need three choices the text never states. Two of them change every one of the 10 golden files.
2. **The V-NAR-05 probe checks the sidecar's form, not its truth.** Five of six planted sidecar defects pass. The D49 brief tells the director that the probe confines prologue sentences to the opening section, and it does not.
3. **Legal chronicles break the renderer.** These inputs, which the specs allow, are not fixture cases:
   - a feud motivated by a murder attempt renders "in retaliation for event 6." and fails the reference's own provenance check;
   - a child of a daughter of the house is designated "of House Ashcombe" but left out of that house's annal;
   - an interregnum never appears in the house annal it belongs to.
4. **The template set states things the event does not carry.**
   - "under strain" is printed even at stress 0.0.
   - A motive edge with no motive renders "for a motive the chronicle does not name the succession of …".
   - The motive picked is the earliest edge, not the strongest.
5. **Designation is not injective, and the fixture is not a possible history.**
   - Two unnamed sons of one named father get the same words.
   - Ten fixture records could not be emitted by M2, M7 or T1.
   - The G1a sample reads accordingly (§ "The G1a reading").

### Findings

Severity: **BLK**: wrong or unsafe, must fix. **MAJ**: wrong in a way a later sprint inherits. **MIN**: should fix now. **NIT**: cosmetic or wording.

| ID | Sev | Where | Problem and evidence | Fix | Conf. |
|---|---|---|---|---|---|
| F1 | MAJ | `annals_v0.md:97-118`; `selftest_annals_v0.py:735,743,757,773` | **The contract does not determine the files.** My independent renderer (`work/indep_render.py`) was written from §1–§9 and the TOML. It reproduces 10/10 goldens only with three choices the contract never states (`work/compare_indep.sh`, `work/ambiguity_flags.sh`):<br>(a) the two `# The annals of …` / `# From the chronicle of …; templates …` title lines, whose wording differs by group kind. Without them, 10/10 files differ.<br>(b) a blank line before every `##` heading. Without it, 10/10 differ.<br>(c) `templates_digest` printed as the hex of the little-endian bytes (`ce7b0296724cf918`), while the `digest` line and every `hash64` in the repository print the integer (`18f94c7296027bce`). With the integer, the 5 sidecars differ.<br>Two more readings a careful reader would plausibly take also break the goldens:<br>(d) §6's table cites only `kf:tenure`/`title/` for a tenure row, and the house sentence designates no person. The golden adds the holder's designation sources and the living members as persons. Read literally, 2 files differ.<br>(e) "Lists are sorted": ids sorted as tokens give `[10,7]` where the golden has `[7,10]`, so 4 files differ.<br>Also unstated: the order of kinds in the omission trailer, and which records rule 4's house walk cites (the golden cites the father's `kf:member/1005`). | State the text grammar exactly: the title lines per group kind, the blank line, and capitalising the first character. Print `templates_digest` as the integer's 16 hex digits, like `digest`. Say that ids sort numerically and sources bytewise. Give each prologue row's persons and sources. List the records rule 4 cites. | 90% |
| F2 | MAJ | `selftest_annals_v0.py:823-845`; `docs/decisions_pending.md:20`; `annals_v0.md:143` | **The V-NAR-05 probe checks form, not truth** (`work/mutants.py`). It catches only an empty event list (P2). These pass:<br>P1: a row cites its cause but not its own event.<br>P3: a body sentence is retagged `P` with a `kf:` source.<br>P4: a `P` row cites `kf:life/99999`, which does not exist.<br>P5: a row cites an unrelated event.<br>P6: two rows exchange contents.<br>Check 8 (golden bytes) is the only guard, and `--write` would bless a renderer regression. **D49's brief to the director says the probe "confines prologue sentences to the opening section". It does not (P3), and it cites the probe as §9 when it is §12.** | Make check 1 re-derive the sidecar:<br>- every in-scope event of the group is the own event of exactly one `S` row, whose template's kind is that event's kind;<br>- a row's events are {own} plus that sentence's told causes;<br>- `P` rows come only before the first year heading, with `[]` events, and every `kf:` source exists in the extract;<br>- persons are within the cast plus the parents reached by designation.<br>Fix D49's text before the director reads it. | 95% |
| F3 | MAJ | `annals_v0.md:42,143`; `selftest_annals_v0.py:527,841-844` | **A cause of an out-of-scope kind has no rendering.** T1 says `feud.cause_event` is "a harm, a deposition, a pressed claim, a succession" (`T1.minds.odd.toml:378`). The envelope gives `FEUD_ENDED` a `triggered` edge "to the break". A feud motivated by a `MURDER_ATTEMPT` renders "…declared a feud against person #1007 of House Ashcombe in retaliation for event 6." Check 1 then fails with "event 6 (MURDER_ATTEMPT) is outside the template scope" (P14b). §3 says a cause renders "by the cause's own variant", and none exists. | Give every kind an in-scope kind's edges may point to a `label`, through a `[[label]]` table for out-of-scope kinds: `MURDER_ATTEMPT`, `MENTAL_BREAK`, `FLED`, `HOLDINGS_BURNED`, `DESPAIR_PACT`. The lint should require it. Check 1 should accept a cited cause of any kind. | 85% |
| F4 | MAJ | `annals_v0.md:57` vs `:65`; `selftest_annals_v0.py:618` vs `:473`; `M7.dynasty.odd.toml:714` | **House membership contradicts M7, and the contract contradicts itself.** M7 `on_birth` gives a newborn "the father's house, else the mother's (looked up in house_member)". §5 calls its own rule "M7's rule" but falls back to the mother only when there is "no father". §4 rule 4 follows M7. In P17, Edric's daughter 1006 bears 1010 to a man of no house. 1010 is designated "person #1010 of House Ashcombe", is not in House Ashcombe's member set, and their death is missing from House Ashcombe's annal. | One function, M7's rule, for both grouping and designation: the keyframe row; else for an in-run birth the father's house if the father has one; else the mother's. | 85% |
| F5 | MAJ | `selftest_annals_v0.py:748`; `annals_v0.md:65,150` | **Castless in-scope events silently drop out of house and line annals.** `INTERREGNUM_CRISIS` and `EXTINCTION` have no cast, so they never join a group. The omission trailer counts only out-of-scope kinds, so nothing records the drop. The golden `ashcombe_house-9001` sidecar has no event 26, and its trailer lists only `MURDER_ATTEMPT x1`. The house annal says "gained a claim to a vacant title" without ever telling the interregnum of its own title. §12 claims the G1a sample includes "an interregnum"; it does not. | Let an event with no person in its cast belong to a group when a told `triggered` edge leads to an event of the group (`INTERREGNUM_CRISIS` → the holder's `DEATH`; `EXTINCTION` → the `DEATH`). Otherwise count it in the trailer. Correct §12. | 85% |
| F6 | MAJ | `annals_v0.md:52-59`; `selftest_annals_v0.py:439-462` | **Designation is not an identifier.** Rule 3 gives two unnamed children of a named parent the same words: 1005 and a second genesis son 1009 both render "a son of Edric Ashcombe" (P16). SPR-019 measured a median of 7 living members per house, so siblings will be common. §12's bar (iii), "the same person each time", holds only because the fixture has one son. | Rule 3 by birth order among the parent's same-sex children from life rows ("the second son of Edric Ashcombe"), falling back to the id when the rows cannot order them. Add a check that no two persons in one annal share a designation. | 85% |
| F7 | MAJ | `templates_v0.toml:225` | **"under strain" is an invented fact.** The template asserts strain whatever the value. With stress 0.0, it renders "Hugh Thorne, under strain (stress 0.0), declared a feud…" (P15). §3 rule 2 says "no emotion". | Drop the phrase from v0 (Q2), or render a band only through a declared derivation against T1's Registry threshold, citing it. | 85% |
| F8 | MAJ | `annals_v0.md:41`; `selftest_annals_v0.py:213-219,529-533`; `templates_v0.toml:225` | **The motive slot misreads the envelope.**<br>(a) The envelope allows a `motivated_by` edge with a NULL motive ("an opinion term, a trait term", envelope §5). It renders "…declared a feud against a son of Edric Ashcombe for a motive the chronicle does not name the succession of a son of Edric Ashcombe." (P12)<br>(b) The motive is the earliest edge by cause id. A `JEALOUSY_OF` edge of weight 0.03 beats a `RETALIATION_FOR` edge of weight 8.0 (P13).<br>SPR-024's IAUS writes one edge per top contributor, so both cases will be common. | Choose the edge with the largest absolute weight, ties to the lower cause id: the envelope's own coalescing rule for the motive. Add a `when` key for a named or unnamed motive, so the unnamed case has its own sentence. | 80% |
| F9 | MAJ | `annals_v0.md:106,117`; `selftest_annals_v0.py:301-305,773` | **The sidecar does not say what it was rendered from.** The header names neither the timeline, the chronicle's digest, its `registry_version`, the keyframe extract (digest, genesis `state_hash`), the manifest behind the names file nor the build. 05.11 §0 rule 3 requires "the build, Registry version…". The renderer also reads the Registry, an input §1 and §9 do not list, to map `attr_id`s, and ignores the chronicle header's `registry_version`. A chronicle relabelled `timeline=7 registry_version=9.9.9` renders a byte-identical sidecar and digest (`work/header_probe.sh`). After the C3 `registry_version` bump, which moves `attr_id`s, an old chronicle would map snapshots silently wrong. | Add to the header: the timeline id, the chronicle's canonical digest (or segment digests) and `registry_version`, the extract's digest and the genesis `state_hash`, the `manifest_hash`, and the renderer version. Refuse a Registry or chronicle version mismatch. List the Registry as an input in §1 and §9. | 80% |
| F10 | MAJ | `fixtures/ashcombe/chronicle_A.txt`; `annals_v0.md:147` | **The fixture is not a possible history**, and check 5 ("the fixtures are real events") checks only `verb_id`, act classes and tellability. `work/fixture_check.py` finds 10 records M2 0.3.0, M7 0.3.3 or T1 0.3.3 could not emit:<br>- BIRTH 0 on day 73, though no pregnancy is pending at genesis;<br>- BIRTH 11, 439 days after the father's death (gestation is 9 months);<br>- `MURDER_ATTEMPT` 6 `triggered` by a `FEUD_DECLARED`;<br>- `CLAIM_CREATED` 12 and 27 at instants with no succession or deposition;<br>- Hugh's death (15) vacates the seat of Thorne with no `SUCCEED` or `INTERREGNUM_CRISIS`;<br>- `DEPOSED` 20 by a claimant who renounced their claim (event 14);<br>- `DEPOSED` 20 and 29 give the deposed no claim;<br>- `INTERREGNUM_CRISIS` 26 has payload `[]`, though envelope §1 says the payload names the title.<br>Both headers say `registry_version=0.2.0` but use the verb ids of 0.2.0 plus the next-M2 delta: `DEATH` is 15, where 0.2.0 has 14. The G1a sample shows these: "…the second child of Maud renounced a claim." then "…was deposed by the second child of Maud, a claimant." | Correct the ten records, or derive the fixture by hand from M2's and M7's rules. Say what check 5 checks. Make the header's `registry_version` match its dictionary. | 75% |
| F11 | MIN | `annals_v0.md:119` | **`narrative_hash` is promised but not defined.** §9 says the tool "folds each group's `annals_digest` into the timeline's `narrative_hash` in group-id order" and that "this contract fixes … the folding order". No fold function, group set or order is given (string or numeric order?). "STEP_DIGEST.narrative_hash is unaffected until then" implies an engine-written channel-A record would later depend on template text, against 05.11 §0 rules 1 and 6 and sealed segments. | Delete the sentence, or define a plane-C annals manifest (groups, order, fold) and state that `STEP_DIGEST` never covers offline artefacts. | 70% |
| F12 | MIN | `lint_templates.py:261` | **The payload citation is a substring test** (decision 3). `work/lint_probes.py` shows the lint accepting:<br>- L1: `[payloads.MARRIAGE] cited = "Payload: []"` with a `dowry` slot. The renderer then raises `IndexError` (`work/lint_probes_render.py`).<br>- L2: `cited = "Payload"`, one word, with an extra `weapon` slot on `DEATH`. | Match the whole `Payload: [...]` clause of the `when` text and require one slot per comma-separated item. Move to a typed `payload` field on `[[emits]]` when `lint_odd` gains it. | 85% |
| F13 | MIN | `templates_v0.toml:32-38`; `lint_templates.py:238-246` | **The `death_cause` and `sex` codes are checked against nothing** (only `[enums.motive]` is checked against the Registry). L3 exchanges `natural` and `murder`. The lint passes, and every natural death would then render "by murder". | Check each enum's ids against the emitter's column codes (M2's `meaning` lists them), or move the codes into the Registry. | 80% |
| F14 | MIN | `lint_templates.py:57,315-331,377-389`; `selftest_annals_v0.py:639` | **The lint accepts what the renderer cannot honour.**<br>- L4: conversions and format specs pass. The prologue renders through `str.format`, so `{house!r}` prints `'House Ashcombe'`.<br>- L5: an unconditional variant placed first makes the later variants unreachable; nothing flags it.<br>- E1: `snapshot.<attr>.event` (envelope §4 rule 3) lints clean, and the renderer raises `KeyError: 'event'` (`work/extra_probes.py`).<br>- E2: `when.has = ["witness"]` lints clean, and the renderer raises `KeyError: 'witness'`. | Refuse conversions and format specs. Fill prologue templates through the same substitution function. Flag an unconditional variant that is not last. Either define `event` and `witness` in the renderer or refuse them in the lint. | 85% |
| F15 | MIN | `annals_v0.md:42,68`; `selftest_annals_v0.py:213-219,516-523` | **`untold/<id>` is not written where §5 says it is.** §5 says a cause slot for an untold cause "records `untold/<id>`". The reference skips untold edges and records nothing. With the solitary `EXTINCTION` edge set to `cause_told = 0`, the reference writes `S 6 extinction 1 [1] [] []` and my contract-reading renderer writes `[untold/0]` (`work/mutants.py` P11). No fixture exercises the case, so a tool written from the text would diverge unseen. P1a writes `cause_told = 1` everywhere, hence MIN. | Make the reference record the earliest untold edge's id, as the text says. Add a fixture case. | 85% |
| F16 | MIN | `selftest_annals_v0.py:91-110,966-978` | **V-NAR-07's check 3 is a line scan with no control.** An indented `import urllib.request` inside a function passes ("3. no model (V-NAR-07): PASS", P10). `importlib`, which the file uses to execute other files by path, and `random` are on the allow-list. | After rendering, assert that no network or process module is in `sys.modules`: `socket`, `ssl`, `http.client`, `urllib.request`, `subprocess`. Plant a control that imports one inside a function. Drop `random`. | 80% |
| F17 | MIN | `templates_v0.toml:184` | **`house.extinct` narrates a death it does not cite.** It writes "with the death of {target}" literally, so the `DEATH` (event 23) is not among row 35's events, though the event has a `triggered` edge to it. V-NAR-05: "the event IDs it narrates". | `On {date}, with {cause.triggered}, the house of {target} was extinguished.` | 90% |
| F18 | MIN | `selftest_annals_v0.py:63,492-500` | **Codes and calendars outside the domain are not refused.** A payload code outside the enum renders a wrong fact: `DEATH [-1]` gives "Ada died by accident." (P18; Python's negative index). `days_per_year = 364` raises a bare `KeyError: 364` (P9). | Refuse codes outside the enum and calendars other than 360 or 365 with a named error. | 85% |
| F19 | MIN | `annals_v0.md:46` vs `:52-57` | **§3 rule 1 contradicts §4.** Rule 1 says "No slot reads … a keyframe row … or the manifest", but every role slot renders through designation, which reads the names file, `kf:life` and `kf:member`. It is also unspecified whether rule 4 follows keyframe life rows (rule 3 does; the reference's `house_of` does not). | Rule 1 should name designation as the one declared exception and list its sources. Settle the walk. | 80% |
| F20 | MIN | `selftest_annals_v0.py:949-955`; `bin/check_all.sh:1147` | **The `no-event-id` control fails two checks (Q3).** It fails check 1 with 47 cascading problems, because every later line shifts, and also fails check 6, because the plant goes into only one of the two renders. The gate matches only the check header. | Append the planted line at the end so no line shifts. Plant it before the determinism comparison, or into both renders. Match the exact problem text. | 85% |
| F21 | MIN | `fixtures/*/chronicle_A.txt`; `world.toml` | **The fixture understates the id fallback.** Ids are illustrative four-digit numbers. Real ids are H64 hashes (M2 F9; `rh-ids`), so the G1a annals of real seeds will read "person #<up to 20 digits> of House Ashcombe", not "person #1007". This bears on D48's timing. | Use H64-sized ids in the fixture, or say plainly in §12 what G1a will look like. | 85% |
| F22 | NIT | `templates_v0.toml:149,247,208` | **Small overstatements in three templates.**<br>- "was deposed by X, a claimant" asserts agency. M7 has a timer fire with actor = "the leading pressed claimant".<br>- `feud.ended` ("X and Y ended their feud") reads as a reconciliation on a death path whose edge is untold.<br>- `claim.pressed` renders "against no one" when the holder is unfilled. | Variants keyed on the edge or the role. | 70% |
| F23 | NIT | `templates_v0.toml:95,259,274`; headings | **Wording.**<br>- "1 living members".<br>- "has held … since" in a prologue followed by that holder's death.<br>- "slain by Hugh Thorne, by murder."<br>- Headings mix "The twelfth year" with "The 13th year". | — | 90% |
| F24 | NIT | `annals_v0.md:55,132`; `docs/decisions_pending.md:19-20` | **Two text slips.**<br>- Rule 2 glosses parity as "the mother's live births after this one"; M2 says "parity after the birth", which includes this one.<br>- §11 says the Architect files D48 and D49 at the merge, but the PR already edits `decisions_pending.md`. | — | 90% |
| F25 | NIT | `lint_templates.py:172` | **The lint keys `[[emits]]` by kind alone**, so a kind declared by two systems would let the last one silently win. No collision exists today. | Key by (system, kind), and error on a duplicate. | 70% |
| F26 | NIT | `selftest_annals_v0.py:24` (docstring) | **Check 6 does not verify a committed digest.** "The digest recomputes" means the file is rendered twice. Nothing verifies the digest line of a committed sidecar. | A `verify` mode that recomputes a sidecar's digest from its text and sidecar. | 70% |

### The G1a reading (`golden/ashcombe_house-9001.txt`)

Read aloud, it is a register of 29 dated one-line entries, not a family history. Each entry is grammatical and grounded, apart from F7 and F8. But the reader cannot follow the family:
- Of the six people the story turns on, two are named.
- 1005 is "a son of Edric Ashcombe" 9 times.
- Edric's daughter 1006, the antagonist who claims, feuds, deposes, rules and dies, is "the second child of Maud" 11 times. The annal never says that 1006 is Edric's daughter or 1005's sister.
- 1005's sons are "person #1007 of House Ashcombe" 13 times and "person #1008 of House Ashcombe" 6 times. Their father is never named: the birth line reads "a daughter of Hugh Thorne bore a son, person #1007 of House Ashcombe."
- No title is named after the prologue, in a world with two.
- The house's own interregnum is missing (F5).
- The sample tells an impossible sequence: a renounced claimant deposes as "a claimant" (F10).

**The single most damaging weakness is designation.** In a family history, who is whose child is the substance, and this annal hides it although the renderer already reads the life-course rows that state it. Rule 2 runs before the father-first rule 3, which produces "Maud bore a daughter, the second child of Maud." The fallback for 1007 and 1008 drops the father entirely. Real ids will make it worse (F21).

Much of this can be fixed within v0, without D48:
- put the father first;
- use birth order among same-sex siblings (F6);
- name the father in the birth sentence from the child's `LIFE_ENTRY` (decision 1 below);
- never designate a child by the birth being told.

The title gap needs M7's payload (decision 2).

### What holds

- **Determinism.** The rendering is byte-stable under:
  - shuffled keyframe rows;
  - `PYTHONHASHSEED` 0, 1 and 12345 crossed with `LC_ALL` C, C.UTF-8 and tr_TR.UTF-8 (9 renders identical and equal to the golden; P19);
  - repeated renders.

  Sorting covers every list that reaches the output.
- **Exact reproduction, with the F1 caveats.** Under the golden's three layout choices, my independent renderer reproduces all 5 annals and 5 sidecars byte for byte, including every designation, cause label, sources set and digest. The digest formula in §9 is exact once the `templates_digest` byte order is known.
- **Gates.** The 15 gates are in `--part core` (lines 1146–1160, after `part core` at 857) and in `python/tests/test_check_all_inventory.py`. All 15 pass by hand (`work/run_gates.sh`). Each of the nine lint controls fails with exactly one diagnostic, the intended one. The stale-golden control fails `STALE:`. The inventory pytest passes (13 tests). `ruff` and `mypy --strict` are clean. `CLAUDE.md` carries both commands.
- **Fixtures parse.** Both fixtures are valid canonical v0 as amended by PR #61: `canonical_v0.py --digest` accepts them, and they are in R4 order (`work/fixture_check.py`). Roles are within each kind's `[[emits]]`. Every declared `stress(actor)` snapshot is present. Every `DEATH` agrees with its `LIFE_EXIT`. `BIRTH` sex agrees with the `LIFE_ENTRY`.
- **Grounding mostly holds.**
  - Every slot in the set binds to a role, snapshot, payload slot, motive or edge the emitter declares.
  - The two payload layouts quote M2's `when` texts verbatim, and the `[death_cause code]` and `[sex code, parity]` meanings match M2 0.3.0 F9 and S12.
  - No title, cause or date is taken from state.
  - The designation reads only the names file, `BIRTH` events, `LIFE_ENTRY` records and the keyframe extract, all plane-C-legal.
- **Nothing feeds plane A.** The PR is specs and a stdlib reference only, apart from F11's wording.
- **The lint catches the three asked cases:** an undeclared slot, an in-scope kind with no template, and a conditional last variant.

### Decisions 1–7 and Q1–Q3

The numbering below is the brief's. The contract's own §10 numbers these differently, and decisions 5 and 6 live in §4 and §5.

| # | Decision or question | Recommendation | Conf. |
|---|---|---|---|
| 1 | No `parent` role | **Accept**: the father is already in the child's `LIFE_ENTRY`. But add a declared `father` slot on `BIRTH`, read from that `LIFE_ENTRY` and cited as `life/<pid>` (§4 already treats `LIFE_ENTRY` as the chronicle). Then the birth sentence can say whose son 1007 is (the G1a weakness). | 75% |
| 2 | M7 0.3.3 has no payload layout, so successions name no title | **Accept the principle**: the title must come from the payload, not from tenure state. File the M7 amendment now in PR #62's 0.4.0 draft: `Payload: [title id]` per kind, `[house id]` on `HOUSE_EXTINCT`. Make the fixtures carry title payloads already, as envelope §1 says the payload names the title. Until then, F5 hides the interregnum. | 85% |
| 3 | Payload layouts quote the `when` text | **Accept as a stopgap only, with the clause-exact check of F12.** The typed `payload` list on `[[emits]]` is the real fix and is cheap at `lint_odd`'s next change. | 65% |
| 4 | The keyframe extract is an input file | **Accept for the stdlib reference**, on three conditions: define the extract's schema and version, bind it in the sidecar by digest and genesis `state_hash` (F9), and enforce the prologue in the probe (F2). REV-SPR-019 R1 offered SPR-029 a channel-A record kind for back-filled persons. While no build writes a segment, that is an in-place v0 amendment. It would put the prologue on sealed, digest-covered records and remove the keyframe reader from plane C. The PR does not mention it, and the Architect should weigh it with D49. | 65% |
| 5 | The designation rule | **Revise before audit:**<br>- father first;<br>- injective, by birth order among same-sex siblings (F6);<br>- the house walk is M7's `on_birth` (F4);<br>- never designate a child by the birth being told;<br>- state the cited sources (F1). | 80% |
| 6 | No tellability filter | **Accept for P1a scale**, but correct the citation. 05.4 §5 defines Annals as "templates over matches and significant events", not "the whole told record". Make the accidental filter of F5 explicit. | 75% |
| 7 | `string.Formatter` reuse | **Accept** the shared parser, but restrict the grammar: no conversions or format specs, and one substitution function for both vocabularies (F14). | 80% |
| Q1 | A house prologue or not | **Keep it.** A house that opens with no past fails the G1a reading. Make the probe enforce what D49 says (F2), and anchor the extract (F9) or move it to channel A per R1 (decision 4). | 75% |
| Q2 | A stress number in prose | **Remove it from v0 prose.** "(stress 63.0)" is a meter in a historian's register with no scale, and "under strain" is invented (F7). If a state word is wanted, it should be a declared derivation against T1's Registry threshold, cited in the sidecar, with the number kept in the sidecar. | 80% |
| Q3 | A control that fails two checks | **Isolate it** (F20): plant at the end of the annal, keep determinism out of the control's path, and match the exact problem line. | 85% |
| D48 | Names (for the director) | **Approve the plane-C name generator, but schedule it before the G1a reading (P2.8a), not at P2.8.** Real ids are 64-bit (F21), so G1a's annals would otherwise print 20-digit numbers. The generator is deterministic, plane C and small: a name-base table in the Registry, with mewo2's naming-language or Azgaar's name bases as PORT in `docs/14`. Fix the fallback (decision 5) regardless. The plane-A `person.name` alternative is rightly rejected. | 65% |
| D49 | V-NAR-05 and the prologue (for the director) | **Approve the amendment in substance, but only after the probe does what the text says** (F2). Reword: "a sentence of the world's state at genesis maps to the genesis records it was rendered from, which the sidecar identifies, and appears only before the first chronicle sentence". Add R1's channel-A route as the alternative the brief omits. | 70% |

### Not run

- The full `bin/check_all.sh` chain: the Rust build, nextest, cargo-deny, valgrind and the golden part. I ran the 15 SPR-029 gates by hand, `ruff`, `mypy`, and the inventory test with pytest.
- Real engine output. M7 (SPR-021) and T1 (SPR-024) are not built, so every check against M7 and T1 is against spec text.
- PR #62's `next/M7.dynasty.odd.toml` 0.4.0 is not on `main` or in the archive, so decision 2's proposed amendment was not checked against it.
- I could not read the PR description: `gh` returned nothing. Q1–Q3 are taken as the brief states them.

### Scripts (`work/`)

`indep_render.py` (the independent renderer), `compare_indep.sh`, `ambiguity_flags.sh`, `run_gates.sh`, `mutants.py` (P1–P19), `lint_probes.py`, `lint_probes_render.py`, `extra_probes.py`, `fixture_check.py`, `header_probe.sh`.

## 2. The review (round 2, independent reviewer)

Copy reviewed: a git archive of `214fbd8` in the Architect's scratchpad, `annals_review_r2/src/`.
- Every script cited is in `annals_review_r2/work_r2/` and runs from there with `python3 -I -B <script> ../src`, or `./<script>.sh`.
- Nothing in `src/` or the repository was modified.
- The independent renderer was written from `annals_v0.md` §§1–9 and `templates_v0.toml` before the reviewer read `selftest_annals_v0.py`.

### BLUF
**Verdict: changes needed before the merge. 0 blockers, 3 majors, 7 minors, 5 nits.**

**What holds:**
- The independent renderer, written from the contract alone, reproduced all 6 annal texts byte for byte on the first attempt. It reproduces all 15 golden files once two readings the text leaves open are adopted (G4).
- All 27 SPR-029 gates pass, and each control fails for its own reason. Ruff, mypy and the inventory test are clean.
- Output is byte-stable across 96 renders (4 hash seeds × 4 locale settings × 6 groups) and across 5 shufflings of the input rows.
- Round 1's mutants P1 and P3–P6 each fail check 1 by name.
- These rulings hold: R3, R5, R7, R11, R13, R14, R15 and R16, and R12's F12–F19.
- R17's labels: no golden text contains a run of 5 or more digits.
- The impossible fixture records fall from 10 to 3.

**Majors:**
- **G1.** The reference's house walk is neither M7's `on_birth` rule nor the rule §4 states.
- **G2.** Check 1 re-derives only the `S` rows' events, persons, template and text. It does not re-derive prologue rows, any row's sources, versions, trailers or the header, so 17 of 28 planted mutants survive.
- **G3.** R8's "largest |weight|" can name an inhibiting contributor as the motive.

### Rulings R1–R17
| # | Verdict | Evidence |
|---|---|---|
| R1 exact grammar | **Partly** | `./compare_indep.sh out_literal` (the first reading of the text) matches 10 of 15 golden files, including all 6 texts. With `--sibling-src all --cause-persons designated` it matches 15 of 15. The text determines the title lines, blank lines, headings, capitalisation, `templates_digest` as integer hex, numeric ids, bytewise sources, prologue rows, and the manifest digest. Undetermined choices remain (G4) |
| R2 sidecar re-derived | **Partly** | Each of these fails check 1 by name: P1, P3 (an *existing* kf source after the first year heading), P4 and P4b (a non-existent kf source), P5 (an *existing* unrelated event), P6 (rows swapped: "the text is not the rendering of event 9"). 17 other mutants survive (G2) |
| R3 `[[label]]` | **Holds** | The emitters declare 16 out-of-scope kinds (`specs/systems`, with `next/` superseding), and they are exactly the 16 labels. Deleting ENCOUNTER's label is refused by name. A feud motivated by `MURDER_ATTEMPT` renders, and check 1 passes |
| R4 one walk, M7's rule | **Fails on a legal input** | One function (`Kinship.house_of`), cited by §4 and §5, but it is not M7's rule (G1) |
| R5 castless events | **Holds** | House Thorne renders the interregnum, and Ashcombe and the line count it in the trailer. §12 is corrected. Castless chains diverge (G5) |
| R6 + R17 designation | **Holds on the fixtures** | Name, then father, then mother, then label. The mother probe gives "the first son of Ysolde", and twins give `Founder #4`/`#5`. No designation comes from the birth being told. A prefix collision gives `p-aae0ea`/`p-aae0e0` in both renderers. No digit run of 5 or more appears in any golden text. Check 9 catches planted collisions, but only inside the self-test (G7) |
| R7 no strain | **Holds** | The only "stress"/"strain" is a header comment in `templates_v0.toml` |
| R8 motive | **Partly** | A tie at \|128\| goes to the lower cause id, and `when.motive` separates named and unnamed motives. But \|weight\| picks an inhibitor (G3) |
| R9 provenance header | **Partly** | Every field is present, and a `registry_version` mismatch is refused by name. But the extract is bound by the XXH3 of its TOML file's bytes, which the P2.8a Arrow reader does not have (G6) |
| R10 possible history | **Partly** | 3 deviations remain, down from 10 (G8). `gen_fixtures --check` passes; check 5 is live |
| R11 manifest | **Holds** | Groups are in (kind, numeric id) order, each with its digest, and the manifest has its own digest. The `STEP_DIGEST` statement is present and the folding sentence is gone |
| R12 F12–F19 | **Holds** | 18 planted template files: every F12–F19 case is refused by name. Prologue lint gaps remain (G9) |
| R13 no-event-id | **Holds** | The control fails check 1 only, with the exact line |
| R14 F22–F26 | **Holds** | Edge- and role-keyed variants; ordinal headings; duplicate emits refused; `verify` recomputes all 6 digests |
| R15, R16 | **Hold** | §10 records the decisions. D49/D50 are recorded on `main`, with no `docs/` edit on the branch; the generator is at P2.8 |
| R17 compact label | **Holds** | Caveats G11, G12 and G4(c) |

### Findings
Severity: **MAJ**, wrong in a way a later sprint inherits; **MIN**, fix now; **NIT**, cosmetic or unreachable today.

- **G1 (MAJ, 85%): the house walk.**
  - §4 rule 4 says: the keyframe row; else, for a birth during the run, the father's house if he has one, else the mother's. M7's `on_birth` is a single lookup.
  - `Kinship.house_of` walks every `LIFE_ENTRY` person, genesis persons included, and recurses into a father's ancestry when he has no row.
  - **Probe `r4`:**
    - Setup: daughter D (a member); her houseless husband O; their genesis son X, with no row; and an in-run son C of X by Ysolde, a Thorne.
    - The reference puts X and C in Ashcombe and prints "five living members".
    - The contract gives four, and puts C in Thorne.
  - **Fix:** `house_of(p)` is the keyframe row. Else, for an in-run birth, the father's house if not none, else the mother's. Else none. Add the case to `gen_fixtures`, and flag that the agnatic back-fill and M7's cognatic fallback disagree.
- **G2 (MAJ, 90%): check 1 re-derives `S` rows only.**
  - `_check_prologue_row` checks only position, no events and that some `kf:` source exists. `_kf_source_exists` is true for every non-`kf:` source. Sources and versions are never compared, and events and persons are compared as sets.
  - D50 as ruled ("maps to the keyframe rows it was rendered from") is not enforced.
  - **Surviving mutants:**
    - P6b: prologue rows swapped;
    - P7a–d: sources dropped or forged;
    - P8: a version changed;
    - P9: events reordered;
    - P10: a member dropped from a `P` row;
    - P11: a prologue template relabelled;
    - P12 and P12b: the prologue text altered;
    - P13: another house's root cited;
    - P15: the castless trailer deleted;
    - P16: the omitted count changed;
    - P17 and P18: header digests and the registry version swapped;
    - P22b (G14).

    Only the goldens catch these, and `--write` would bless them.
  - **Fix:** check 1 re-renders the group from its inputs (prologue, sentences, trailers, header) and compares the sidecar row by row and field by field, list order included, naming the first field that differs. Add `P`-row and source controls (P12, P13, P7b).
- **G3 (MAJ, 70%): the motive by |weight|.** The envelope's §5 says weight is signed, and an inhibitor is negative. With `JEALOUSY_OF` at −200 and `RETALIATION_FOR` at +90, the annal prints "declared a feud … out of jealousy of …". **Fix:** take the largest *positive* weight, with ties to the lower cause id; if none is positive, render without a motive. The envelope's coalescing rule has the same exposure.
- **G4 (MIN, 85%): choices the text does not determine.**
  - (a) A cited cause's persons: the goldens list only the persons the cause's label designates.
  - (b) A sibling rank's sources: the goldens cite every child of that parent of the same sex.

  Both of these change golden files under the literal reading. Unexercised:
  - (c) hex padding;
  - (d) trailer kind order;
  - (e) numbers of 1000 and above;
  - (f) an unnamed house or title in the prologue;
  - (g) castless chains;
  - (h) which groups the manifest lists;
  - (i) the extract digest's bytes;
  - (j) `chronicle_digest` over several segments.

  **Fix:** state each one.
- **G5 (MIN, 75%): castless chains.** `group_events` makes a single pass, so a castless event triggered by a castless event that joined goes to the trailer. **Fix:** join in ascending id, each joined event counting for later ones, or narrow the text.
- **G6 (MIN, 80%): the extract digest is not canonical.** It is the XXH3 of `keyframe.toml`'s bytes: 5 shuffles of the same rows give 5 digests. **Fix:** a canonical byte form of the extract (sorted rows, one canonical line each). Also state the form of `manifest_hash`: the fixture has 40 hex digits, while `rh-golden` prints `sha256:<64>`.
- **G7 (MIN, 75%): a shared designation.** Check 9 runs only on the fixtures; the renderer prints two "Maud"s without complaint, and D49's generator will make collisions routine. **Fix:** refuse by name, or disambiguate with the `p-` label.
- **G8 (MIN, 80%): three fixture facts the cited specs forbid.**
  1. `DEPOSED 18` falls on day 1 of the month, but M7 places it on days 2 to the last.
  2. `CLAIM_CREATED 31` is triggered by an `INTERREGNUM_CRISIS`, but M7's `when` allows only a passed-over heir or a deposition.
  3. Founder indices follow dict order, not "the cast first".

  **Fix:** correct all three, and add both rules to check 5 with a committed control.
- **G9 (MIN, 85%): the prologue lint.** It accepts `{death_date}` in `prologue.house` (the renderer raises `KeyError`), `version = "two"`, an unknown `[[prologue]]` id, and an embedded `\n## `. **Fix:** a vocabulary per prologue template, integer versions, known ids, and no control characters.
- **G10 (MIN, 80%): no words for 1000 and above.** `cardinal` refuses n ≥ 1000, so year 1001 raises an error. **Fix:** define the words, or the bound, and add known answers.
- **G11 (NIT, 70%): `p-` labels are relative to the annal.** The same person is `p-aae0` in one annal and `p-aae0e` in another. Compute uniqueness over the world's persons, or say so.
- **G12 (NIT, 75%): raw ids can still reach the text:** `house #<id>`, `title #<id>`, and "in retaliation for event 6." None is reachable today. Refuse each by name.
- **G13 (NIT, 85%): control hygiene.**
  - `row-drops-own` plants P2, not P1.
  - `unrelated-event` uses a non-existent id.
  - Two lint controls give 2 diagnostics each.
  - Check 5 has no committed control.
  - Check 6's two renders share one process and one hash seed.
- **G14 (NIT, 85%):** the first year heading is found by substring (`"chronicle" not in line`). Match `## Before the chronicle began` exactly.
- **G15 (NIT, 90%): text slips.**
  - §0 still says "else by id".
  - §12 gives the wrong counts of births with a father named and of sibling references.
  - §3's "the envelope's own coalescing rule" is inexact: the envelope breaks ties by `[motive_edges]` index.
  - PRJ-002 says "two fixture chronicles"; there are 3.
  - §1 should say that a chronicle under the frozen dictionary cannot be rendered while the delta exists.

### Scripts (`annals_review_r2/work_r2/`)
| Script | What it does |
|---|---|
| `indep_render.py` | The independent renderer; each open choice is a flag |
| `compare_indep.sh OUTDIR [flags]` | Every group and manifest against the goldens |
| `mutants.py` | 28 sidecar and text mutants |
| `probes.py` | 19 behaviour probes |
| `fixture_check.py` | The R10 plausibility check |
| `lint_probes.py` | 18 planted template files |
| `run_gates.sh` | The 27 gates |
| `determinism.sh` | Hash seeds, locales and shuffles |

**Not run:** the full `bin/check_all.sh`; real engine chronicles (none are written yet); `en_US.UTF-8` and `tr_TR.UTF-8`, which are not installed.

## 3. The verification (round 3, the round-2 reviewer)

Copy verified: a git archive of `e3fa495` (`annals_review_r3/src/`). The scripts are in `annals_review_r3/work_r3/`: the round-2 scripts re-targeted by row content, and new probes.

**BLUF: changes are still needed, all small. 0 blockers, 1 major, 4 minors, 3 nits.**
- R18, R22, R25, R26, R27 and R28 hold. R19, R20, R21, R23, R24, R29 and R30 hold partly.
- All 36 gates pass, with one diagnostic per lint control.
- All 28 round-2 mutants fail by name. Of 6 new mutants, only P23 survives (H4).
- All 6 annal texts reproduce from the contract alone, and R21's (a) and (b), read literally, match. The only literal mismatch left is H2.
- 96 renders across hash seeds and locales, and shuffled or comment-stripped extracts, are byte-identical.
- `fixture_check.py` finds 0 problems, and all 22 lint probes are refused.

| # | Verdict | Evidence |
|---|---|---|
| R18 | Holds | Probe `r18`. Xan is in neither house. His in-run son is in Thorne. Ashcombe has "four living members" |
| R19 | Partly | Every round-2 mutant fails by name (P12, P13, P7b, P15, P17 …). A shorter text goes unreported (H4) |
| R20 | Partly | "+128/+128" goes to the lower id, and "−200/+90" renders the retaliation. When every weight is ≤ 0, the sentence still reads "moved by" (H1) |
| R21 | Partly | (a) and (b) hold literally. The extract prefix differs (H2), and §4 rule 2 still has the old wording (H6) |
| R22 | Holds | Probe `r22`: the castless chain joins |
| R23 | Partly | The digest is canonical and check 10 runs. The domain prefix differs between text and code (H2) |
| R24 | Partly | Two Mauds render as "Maud (Founder #2)" and "Maud (Founder #4)", and check 9 holds. Scope and kin-phrase placement disagree (H3) |
| R25 | Holds | "The one thousand and first year"; 1,000,000 is refused |
| R26 | Holds | The deposition is on day 2 and claim 31 is dropped. Check 5's controls are committed. The cast is founders 0–3, and `Founder #5` = k 4 |
| R27 | Holds | An unnamed house, title or cause kind is refused by name |
| R28 | Holds | Run-wide pool; zero-padded hex (`p-00bc0`) |
| R29 | Partly | Fixed, except two §12 counts (H6) |
| R30 | Partly | The labels equal the 16 out-of-scope kinds, `next/M7` 0.4.0 included. Q5's sentence is missing from §1 (H5) |

**Findings:**
- **H1 (MAJ):** with every told `motivated_by` weight ≤ 0, "moved by <cause>" cites an inhibiting edge (probe `r20`: −200/−90, −90 and 0).
- **H2 (MIN):** §9's prefix has `\n`, and the code does not. With the code's reading, 15 of 15 files match.
- **H3 (MIN):**
  - R24's text says "of one annal", while the code is run-wide.
  - In a kin phrase the parent's label is printed, and its source is not cited (probe `parent_label_source`).
  - "the first son of Hugh Thorne (Founder #3) succeeded Hugh Thorne (Founder #3)".
- **H4 (MIN):** `check_provenance` compares `zip(got, want)` and never reports missing lines (P23).
- **H5 (MIN):** §14 cites Q5's re-derivation to §1, and §1 lacks it.
- **H6 (NIT):** text slips:
  - "day 2 of its year";
  - two births and two sibling references, where there are three of each;
  - §4 rule 2;
  - §8's fold of the segment digests.
- **H7 (NIT):**
  - The R19 controls plant other defects than R19 names.
  - The day-one control also leaves claims unmoved.
  - The header message prints `None`.
- **H8 (NIT):** whether `LIFE_ENTRY` parent ids are in the pool is unstated.

**Scripts:** `indep_render.py` (literal defaults, new flags), `compare_indep.sh`, `mutants.py` (34 mutants), `probes.py` (20 probes), `compare_worlds.py`, `fixture_check.py`, `lint_probes.py` (22 files), `run_gates.sh`, `determinism.sh`.
