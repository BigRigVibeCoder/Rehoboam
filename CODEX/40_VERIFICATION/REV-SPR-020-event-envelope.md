---
id: REV-SPR-020
title: "Independent review of the event envelope and role codes spec (PR #61)"
type: record
status: CLOSED
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-020, SPR-013, SPR-021, SPR-026, SPR-029, EVO-001, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF: CLOSED, approved.** The event envelope spec (PR #61) took four rounds:
>
> | Round | Head | Findings | Rulings |
> |---|---|---|---|
> | 1 | `742ed09` | 0 blockers, 3 majors, 9 minors, 5 nits | R1–R12 (§0) |
> | 2 | `9755956` | 1 major, 8 minors, 5 nits | R13–R22 (§2.0) |
> | 3 | `99691b0` | 1 major, 3 minors, 8 nits | R23–R27 (§3.0) |
> | 4 | `c855441` | Verified by the Architect (§4) | — |
>
> - **What it settles:**
>   - one role vocabulary, with domains as anchored attributes, so D45 is withdrawn;
>   - emitter-filled snapshots;
>   - nine typed edge kinds with total coalescing;
>   - the digest v2, with hash-neutral origin tails;
>   - provenance through `set_cause` into `event_ref` columns;
>   - channel A and canonical v0 amended in place.
> - **D44** (DATA-2: eight or nine edge kinds) goes to the director.
> - **Reviewers:** rounds 1 to 3 ran in sessions on a different model from the drafter's.

# REV-SPR-020: independent review of the event envelope

## 0. Architect rulings, round 1 (2026-10-17)

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R1 | MAJ-1 | **Accepted.** Domain membership becomes declared, anchored attributes, filled as snapshots at emission (for example `holds_title(A)@eN`, `kin(A, B)@eN`), as `05.4` already does for kinship.<ul><li>The pattern DSL keeps its surface (`roles = {A = "authority"}`) and compiles a domain to its anchored attribute read, so SIFT-1 ("bindings over abstract roles … anchored state guards") still holds as written.</li><li>The per-row `abstract_role` code leaves `event_role`.</li><li>A domain with no owning source yet (`investigator`, `suspect`, `host`, `guest`, `rival`, `ally`, `subordinate`) is absent until its system exists; `lint_patterns.py` warns on a pattern that uses one.</li><li>D45 is withdrawn: no requirement changes.</li></ul>If the drafter finds a requirement or ADR text that does change, D45 returns, reframed around the anchoring trade-off | 7/10 |
| R2 | MAJ-2 | **Accepted.** A buffered `ctx.set_cause(column, row, EventRef)`, rewritten at the merge like a timer's `cause` (EVO-001 R15). After the merge, `rh-core` checks that a state different from the column's declared default has a non-zero cause; the default is declared per column in `[[owns]]`. S6's `condition_cause` text is corrected to match. `05.1` §2 rule 4 gains the slot, as v1.13 in this PR | 8/10 |
| R3 | MAJ-3 | **Accepted.**<ul><li>For `place_id`, `magnitude`, `secrecy` and `origin`, state who sets each one: the draft, or the merge from the Registry and plane-A state.</li><li>Hash all four in v2, so places (SPR-018) need no v3.</li><li>Specify `05.2` §4.0 criterion 4's origin plumbing (origin carried through timers and commands from the `Inputs` phase) here, as §4.0 defers it to P1.9.</li><li>Resolve §2 against §5: say which edges the merge writes</li></ul> | 7/10 |
| R4 | MIN-1 | Hash the attribute's name, `h(name)`, not its sorted index | 8/10 |
| R5 | MIN-2 | `events.held` carries an unhashed `u8 admitted` | 8/10 |
| R6 | MIN-3 | **Keep the in-place amendment of channel A and canonical v0:** no build writes either yet, and SPR-013 then implements one format. Make the texts agree:<ul><li>both status rules say a change before the first build writes a segment is an in-place amendment recorded in the change log, and a new version after that;</li><li>`05.2` §4.0's `cause_told` sentence;</li><li>envelope §6: SPR-013's channel-A digests are final and only `events.meta` moves at SPR-020.</li></ul>Also draft the SPR-013 sprint-doc amendment (the new tables and dictionaries, 16 files per segment, mostly empty in P1a) in the PR; the Architect approves it at the merge | 8/10 |
| R7 | MIN-4, MIN-5 | `lint_odd.py`:<ul><li>refuses duplicate motives and duplicate snapshots;</li><li>type-checks `algorithm.*` sources and foreign enum codes;</li><li>refuses `of = "witness"`, or defines it per witness;</li><li>makes a missing `roles` an error on frozen specs now (a warning on drafts), with the control turned into a must-FAIL</li></ul> | 8/10 |
| R8 | MIN-6 | `canonical_v0.py` refuses two actors, and a `triggered` edge whose weight ≠ 0 or evidence ≠ 256, with planted lines | 8/10 |
| R9 | MIN-7 | One edge per (effect, cause). Contributors pointing at the same cause coalesce: the weights sum, the motive with the largest \|weight\| is kept, and ties go to the lower motive index | 7/10 |
| R10 | MIN-8 | `emit` refuses a draft that lacks a declared snapshot whose role is filled and whose source is present. A snapshot's value is the pre-act value the rule read | 7/10 |
| R11 | MIN-9 | Count the motive edges in full in the budget; the definition is fixed for SPR-030. A planted control that makes check 1 fail | 8/10 |
| R12 | NITs; open questions | **Nits:** NIT-1 to NIT-5 accepted. For NIT-5, refuse a cast over 256 rows by name.<br>**Open questions:**<ul><li>(a) agreed: SUCCEED's edge is `triggered`, landed before SPR-021 codes it;</li><li>(b) the BIRTH and MARRIAGE roles as drafted; the father stays outside the cast, and a `parent` role is appended only if SPR-029 needs it;</li><li>(c) widen channel A's and the canonical form's `entity_kind` domain to 0–5 now (5 = artifact, reserved), while v0 can still be amended; `rh-ids` adds the variant with S6's sprint;</li><li>(d) keep the keyframe v2 bump.</li></ul>**D44** (DATA-2, nine kinds) stays a director item, filed at the merge | 8/10 |

## 1. The review (round 1, independent reviewer)

Reviewer: independent and adversarial, on a different model from the drafter's. Copy reviewed: `$R/src`, a git archive of `742ed09`. `$R` = the Architect's scratchpad `envelope_review/`. Its scripts stay there.

Independent review for the Architect. The review copy is at `$R/src`, and nothing in the repository was modified. Line numbers refer to `742ed09`.

## BLUF

**Not ready to merge as drafted: 0 BLK, 3 MAJ, 9 MIN, 5 NIT.** These parts are sound:

- the role reconciliation;
- the hash v2 field list;
- the plane separation (`cause_told`, the admission bitset and the macros never reach plane A);
- the plane-B macros;
- the canonical row order.

Every self-test and control passes.

There are three design gaps:

1. **Abstract roles (MAJ-1).** Deriving abstract roles at tick granularity drops the "anchor every state read" non-negotiable. D40 frames this as an Architect matter.
2. **`cause_column` (MAJ-2).** The rule asks a step to store a same-step `EventId` that does not exist until the merge.
3. **Event-level fields (MAJ-3).** `place_id`, `magnitude`, `secrecy` and `origin` are in neither `EventDraft` nor the v2 digest. When places land in P1a, the digest has to move to v3.

## Findings

### MAJ-1: abstract-role domains become a tick-level, unanchored state read

- **Where:** `specs/chronicle/envelope_v0.md:20`; `05.4` §2.1 (line 37); `05.2` §4 table; `docs/decisions_pending.md` "D40".
- **Problem.** Under this PR the sifter evaluates `authority`, `kin` and the other domains "at the clause event's tick from `state_interval`". Intervals have tick granularity, while titles change within an instant: DEATH or DEPOSED, then SUCCEED, in the same tick.
- **Evidence.**
  - With `state_interval` rows (11, [0, 500000)) and (42, [500000, ∞)), `title_holder_at(0, 7, 500000)` returns `[(42,)]`. The deposed or dead ruler, who is the target of that tick's DEPOSED or DEATH, is not `authority` at his own event. So `roles = {A = "authority"}` with `target = "A"` never binds him.
  - The old `abstract_role` was stamped at emission. This is the temporal bug that `event_snapshot` exists to fix.
  - `investigator`, `suspect`, `host`, `guest`, `rival`, `ally` and `subordinate` have no interval source, so "derived from `state_interval` and the life-course log" is undefined for them.
  - CLAUDE.md lists "Sifting patterns must anchor every state read" as a non-negotiable that needs director approval. D40 asks the director only to "approve the Architect's ruling".
- **Fix.** Make domain membership declared anchored attributes (`holds_title(A)@eN`, filled as snapshots), as `05.4` already does for kinship, or key intervals by `(tick, event_id)`. Reframe D40 so it states the anchoring trade-off. **Confidence 75%.**

### MAJ-2: the `cause_column` rule cannot be implemented as stated

- **Where:** `envelope_v0.md:142` (§7).
- **Problem.** The cause must be "an event the same system emitted in the step that wrote the state". Neither part of the rule works with the current engine:
  - EventIds exist only after the merge (`05.1` §2 rule 4).
  - A `DraftRef` "cannot be read as an integer" (`05.1` line 73, EVO-001).
  - The merge rewrites only command and timer slots, not owned columns.
  - `SystemView::write` hands out a `&mut [i64]` slice (see `crates/rh-sys-smoke/src/chronicle.rs:32`), so `rh-core` cannot "refuse a write". It can only scan after the step.
  - "Non-default" is undefined for enums: EVO-001 says code 0 is meaningful.
  - S6's "`condition_cause := the act's EventId`" has the same flaw.
- **Fix.** Add a buffered `ctx.set_cause(column, row, EventRef)`, rewritten at the merge like a timer's `cause`. Then check after the merge that state ≠ declared default ⇒ cause ≠ 0, with a per-column default in `[[owns]]`. **Confidence 75%.**

### MAJ-3: event-level stored fields are outside both the draft and the digest

- **Where:** `envelope_v0.md:32-39`, `:49`, `:107`, `:131`.
- **Problem.**
  - `place_id`, `magnitude`, `secrecy` and `origin` are stored in channel A, and `place_id` keys the §7 index. None of them is on `EventDraft` or in the v2 word list, so "every stored envelope field enters `state_hash`" holds only for the child rows.
  - `05.2` §4.0, criterion 4, defers origin plumbing to P1.9, which is this contract, and the contract does not specify it.
  - §5 says `rh-core` writes the `player`, `oracle` and `director` edges at the merge, but §2 says "the merge never adds a role, a snapshot or an edge".
  - Places land in P1a (SPR-018, P1.1a), which forces a v3 digest and a second C3 rebaseline.
- **Fix.** For each field, state who sets it: either the draft, or the merge from the Registry and plane-A state. Hash those fields in v2 and resolve the §2/§5 contradiction. **Confidence 70%.**

### MIN-1: the attribute index in the v2 digest moves when the Registry grows

- **Where:** `envelope_v0.md:46`, `:126`.
- **Problem.** v2 hashes `attr`, the index in `[attributes]` sorted by name. `stress` is 22 of 31 today. Adding any attribute that sorts earlier (an `emotion_*`, say) shifts it, so a C2 Registry addition moves every T1 world's `state_hash`. By contrast, kinds are hashed by name (`h(kind)`), and roles and motives use append-only indexes.
- **Fix.** Hash `h(attr name)`, or make the attribute index append-only. **Confidence 70%.**

### MIN-2: the admission flag is not carried in keyframes

- **Where:** `envelope_v0.md:137` vs `:49`.
- **Problem.** The merge assigns the admission flag (§2), but the new `events.held` keyframe encoding omits it. A keyframe that holds undrained events loses the flag on load.
- **Fix.** Add a carried, unhashed `u8 admitted`. **Confidence 70%.**

### MIN-3: the in-place amendment of channel A and canonical v0 clashes with `05.2` and with §6

- **Where:**
  - `docs/05_architecture/05.2_world_model_and_data.md:172`, `:199`;
  - `channel_a_v0.sql:6-7`;
  - `canonical_v0.md:3`;
  - `envelope_v0.md:136`.
- **Problem.**
  - §4.0 still says `cause_told` arrives with "channel A format v1 and canonical form v1; in v0 every cause is told". Both files also say "a change … is a new version".
  - §6 says SPR-020 re-records the channel-A canonical digests of SPR-013's worlds. With v0 amended now, SPR-013 writes the final shape, so those digests do not change at SPR-020; only `events.meta` does.
  - SPR-013's sprint doc does not take on the new tables and dictionaries (16 files per segment).
- **Fix.** Either use v1 (two constants), or amend §4.0, both status rules, SPR-013's doc and envelope §6. **Confidence 80%.**

### MIN-4: `lint_odd.py` can be dodged

- **Where:** `specs/systems/lint_odd.py:673-720`.
- **Problem.** I planted variants of the `ok` fixture under `$R/fx/`. Each of these passes the lint:
  - `motives = ["RETALIATION_FOR", "RETALIATION_FOR"]`. Envelope line 62 says a duplicate is an error.
  - A duplicated snapshot. At runtime `emit` would refuse every such event.
  - `from = "algorithm.noop"` for an enum attribute: no type check.
  - `of = "witness"`, although witness repeats on an event.

  Foreign enum columns also skip the check on code order.
- **Fix.** Check duplicates in motives and snapshots, type-check algorithm sources and foreign enum codes, and refuse or define `of = "witness"`. **Confidence 85%.**

### MIN-5: the "missing roles" warning has no trigger to become an error

- **Where:** `lint_odd.py:662`; `envelope_v0.md:62`.
- **Problem.** "Warning until SPR-020 closes" has no mechanism that turns it into an error. Every spec at `742ed09` already declares roles: real, smoke and fixture runs show no such warning.
- **Fix.** Make it an error on frozen specs now and keep the warning for drafts. Turn the `missing_roles` control into a must-FAIL for frozen specs. **Confidence 75%.**

### MIN-6: the canonical parser is weaker than the contract

- **Where:** `canonical_v0.py:625` (`_check_event`).
- **Problem.** The parser accepts lines the contract refuses:
  - two actors (`[1/7/0,1/8/0]`);
  - a weight of 5, or an evidence of 0, on a `triggered` edge.

  §1 and §5 refuse these at `emit` and `append`. `canonical_v0.md` says `rh-stream` must refuse the same lines, and `selftest_envelope` check 2 counts a weight ≠ 0 as a violation.
- **Evidence.** `$R/scripts/canon_probe.py`.
- **Fix.** Add these rules to `_check_event` and to the planted lines. **Confidence 75%.**

### MIN-7: two edge rules collide

- **Where:** `envelope_v0.md:105` vs `:115`.
- **Problem.** §5 says "one `motivated_by` edge per contributor" and also "one edge per (effect, cause)". When two contributors point at the same event (`GRIEF_FOR` and `RETALIATION_FOR` for the same death), `append` refuses and no rule says how to merge them.
- **Fix.** State the coalescing rule, for example: sum the weights, keep the motive with the largest |weight|, and break ties by motive index. **Confidence 70%.**

### MIN-8: the snapshot rule is underspecified

- **Where:** `envelope_v0.md:88-89`.
- **Problem.**
  - `emit` checks only that snapshots are a subset of the declared ones, so "no fewer" is never enforced.
  - "As the emitter has written it by then" lets two implementations disagree when the act changes its own source. For example, a T1 break drops stress by its relief, which changes the hash and changes whether `stress(A)@e` matches.
- **Fix.** `emit` refuses a draft that lacks a declared snapshot whose role is filled and whose source is present. Fix the value as the pre-act value the rule read. **Confidence 65%.**

### MIN-9: the byte budget is not measured like for like

- **Where:** `selftest_envelope.py:370`.
- **Problem.** The measure subtracts the P1.0 edge cost from every edge, including the new motive edges. `05.2` §4.1a counts a motive edge in the budget (9 bytes for the kind and the cause id).
- **Evidence.** Recomputed with motive edges counted (`$R/scripts/budget.py`): 4.35 B/event at design densities and 14.3 B/event fullest, against the drafter's 3.6 and 10.8. Both still pass 40/48. Check 1 also compares the generator with its own oracle, so it cannot fail without a planted defect.
- **Fix.** Count motive edges in full, and fix that definition for SPR-030. **Confidence 80%.**

### NITs

- **NIT-1** (`envelope_v0.md:92`): the q8_8 range "[−32768, 32767.996]" is the raw range. The value range is [−128, 128) (`lint_odd.py` `FIXED_RANGE`).
- **NIT-2** (`envelope_v0.md:78`): §3 gives `FEUD_ENDED` a `stress(actor)` snapshot and `motivated_by` edges, but T1 0.3.3 declares neither (§4 item 7 says "six acts").
- **NIT-3** (`envelope_v0.md:137`): "A v1 keyframe cannot be decoded by a v2 reader" is false when `events.held` is empty. `load_engine`'s history check still refuses it. `channel_a_v0.sql:241` still says keyframe v1.
- **NIT-4** (`lint_patterns.py`): `witness` is now a clause key, but no matcher semantics exist for binding a role that repeats. `reg["event_roles"]` raises a `KeyError` if the section is missing.
- **NIT-5**: `ordinal UTINYINT` caps an event's cast at 256 rows, and no refusal is stated for many witnesses.

## The drafter's open questions

- **(a)** Agree: SUCCEED gets a `triggered` edge, to DEPOSED when there is a deposition. Land the text before SPR-021 codes it. **85%.**
- **(b)** BIRTH (mother actor, child target) and MARRIAGE (husband actor, wife target) are fine for P1a. Declare roles on all four kinds in `next/M2` at the merge. Note that the father is outside the cast, so `last_event_for(father)` and DATA-5 criterion 2 never see a named father's child. Append a `parent` role only if SPR-029 needs it. **70%.**
- **(c)** Reserve `EntityKind::Artifact` (5) now, or at least widen the `entity_kind` domain of channel A and the canonical form to 0–5, while v0 can still be amended. At P1b it would force a format version. **65%.**
- **(d)** Keep the keyframe v2 bump. **80%.**

## Governance

- **D39 (DATA-2, nine kinds):** a requirement amendment, fairly framed; agree it goes to the director.
- **D40:** under-framed; see MAJ-1.
- **The in-place amendment:** breaks stated rules; see MIN-3.

## Sound (checked)

- **Planes:**
  - `cause_told` and the admission bitset are host-side and unhashed;
  - the macros are plane B;
  - nothing in plane A reads them.
- **"Absent, never neutral":** coherent with EVO-001's dropped reads.
- **Hash v2:** every envelope child field appears once, in a stated, length-prefixed order.
- **Smoke:** the claim that only `rh-core/events.meta` changes is consistent with `rh-snapshot/state.rs` (`events.held` is carried, outside the ladder roots).
- **`DraftRef` rewrite:** covers edges, as `05.1` rule 4 says.
- **Row order:** total for roles, snapshots (NULL first) and causes.
- **The four macros:** match brute force on 3,000 queries over 3 timelines with 5,436 same-tick ties (`$R/scripts/macros.py`).
- **Self-tests:**
  - `selftest_envelope`, with its 3 controls each failing by name;
  - `selftest.py`, `selftest_channel_a`, `selftest_channel_c`;
  - `canonical_v0 --check`.
- **Lints:**
  - `lint_patterns` (example and `solitary_life`);
  - `lint_odd` (real and smoke specs, and all 12 fixtures, each failing with its planted message).
- **Generators:** `gen_skeletons --check` (both), `gen_hash_golden --check`, `gen_manifest --check`.
- **Architecture and Python tooling:** `lint_arch`, `lint_forbidden`, ruff, mypy, pytest (619 passed).
- **Dropping `partner`:** safe. No pattern, spec or verb uses it as a cast key, and the `initiator` → `actor` rename keeps the meaning.
- **Byte budget:** passes even when recomputed.

## Not run

- Rust builds and tests (cargo): `rh-golden`, the smoke ladder, the V-DET-11 bisect, keyframe tests.
- The perf and allocation gates (V-PERF-04, V-PERF-11).
- `bin/check_all.sh` as a whole.
- The delta against `main` (`lint_odd --next`, PR #50).

## 2. Round 2 (2026-10-17, `9755956`; `03c0b32` changes fixtures only)

### 2.0 Architect rulings, round 2

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R13 | MAJ-1 | **Accepted.**<ul><li>`Timer.origin` and `QueuedCommand.origin` are hash-neutral at 0 (`sim`), as EVO-001's empty `cause` is: they are hashed only when non-zero.</li><li>State the mapping from journaled input kind to origin. An `OracleMove` maps to `oracle`, a player act to `player`, a director input to `director`; each input kind names its origin in the Registry or the journal schema.</li><li>Restate §6 to match. `smoke` changes in `events.meta`, and also, from its first input, wherever an input's consequence carries a non-zero origin. Predict the bisect's first divergence for `smoke` and `solo`.</li><li>SPR-020's sprint is C3 either way</li></ul> | 8/10 |
| R14 | MIN-1 | **Origin propagates through every hop.** Every timer, command and draft that a job creates inherits that job's origin, so T1's `DEATH_DUE` → `ScheduleDeath` → `DEATH` keeps the input's origin. State the rule once in `05.1` §2 rule 4 (v1.13), and add a two-hop case to the self-test | 8/10 |
| R15 | MIN-2 | **Accepted.** `lint_odd --next`:<ul><li>`roles` and `motives` may only grow;</li><li>`snapshots`, `default` and `cause_column` stay equal unless an `[[amends]]` record changes them;</li><li>planted narrowings must FAIL</li></ul> | 8/10 |
| R16 | MIN-3 | **Accepted.**<ul><li>Define a domain attribute's anchor exactly: the event of the first clause, in pattern order, whose cast binds the role variable. Refuse a pattern in which no clause binds it.</li><li>Fix the three example patterns.</li><li>Split `source` into its two meanings, the column read and the owning system</li></ul> | 7/10 |
| R17 | MIN-4 | **Make `cause = 0` mean "none" unambiguously.** If `rh-events` already assigns EventIds from 1, state that 0 is reserved. If it assigns from 0, use a separate presence bit instead, and do not renumber events. S6 declares its `cause_column` and `default` | 7/10 |
| R18 | MIN-5 | **Accepted.** `canonical_v0.md` catches up with:<ul><li>the in-place-until-first-build status rule;</li><li>R8's parser checks;</li><li>`entity_kind` 0–5</li></ul> | 9/10 |
| R19 | MIN-6 | **Version order.** PR #61 keeps M7 and T1 0.3.3 if it merges before PR #41; PR #41 then takes the next number.<ul><li>SPR-019's M7 work is now a `next/M7.dynasty.odd.toml` draft (REV-SPR-019 R4): it does not bump the frozen version, and it lints against whichever frozen M7 is current.</li><li>The Architect updates the handoff's version line at the merge</li></ul> | 9/10 |
| R20 | MIN-7 | **No `registry_version` bump in this spec PR.** This follows the precedent of REV-SPR-018 R3: SPR-020's sprint PR bumps it with its C3 rebaseline, and a bump here would move every golden ladder's tuple line. Say so in §6 and in the PR body | 8/10 |
| R21 | MIN-8 | **Make coalescing total.** Edges to one cause with different kinds keep the kind first in the precedence order the spec states, for example `contingent` before `motivated_by` before `triggered`, with weights and motives coalesced as in R9 | 7/10 |
| R22 | NIT-1 to NIT-5 | All accepted:<ul><li>stale version entries;</li><li>the next M2's frozen-version mention (0.2.2 once this merges);</li><li>a `[role_domains]` shape check;</li><li>"rows the level wrote" reworded to something the merge can know;</li><li>the parser refuses forward causes and kind 6–8 edges</li></ul> | 9/10 |

**Merge note:** `main` is at `11b300d`, which includes the mortality fold (PR #59) and PR #63. Merge it in the round-3 push. The drafter's trial merge showed one conflict, in the inventory test.

### 2.1 The review (round 2, independent reviewer)

Reviewer: independent and adversarial, on a different model from the drafter's. Copy reviewed: `$R2/src`, a git archive of `9755956`. `$R2` = the Architect's scratchpad `envelope_review_r2/`.

Reviewer: independent and adversarial, on a different model from the drafter's. The review copy is `$R/src`, a git archive of **`9755956`** (`origin/arch/SPR-020-envelope`; merge base with `main` is `f53c192`). `$R` = `envelope_review_r2/` in the Architect's scratchpad. Nothing in the repository was modified. Line numbers refer to `9755956`.

### BLUF

**Not ready to merge: 0 BLK, 1 MAJ, 8 MIN, 5 NIT.**

- **Sound:** R4, R5, R7, R10 and R11 are applied, and so is most of R1, R2, R8, R9 and R12.
- **The MAJ-1 case of round 1 is fixed on paper.** M7's `DEPOSED` timer fire reads `title.holder == ruler` before its own writes, so a pre-act `holds_title(target)` is true and the deposed ruler binds as `authority` at his own deposition. This holds only from M7's next version: in P1a no spec declares a domain snapshot.
- **`set_cause` can be built on `rh-core`'s merge.** The owned columns are back in `self.tables` before `merge` runs (`engine.rs:1150-1176`), and it matches EVO-001 R15: an `EventRef`, rewritten after the events.
- **The one MAJ:**
  - R3's origin on timers and commands is not hash-neutral.
  - Smoke's synthetic inputs are `OracleMove`, so they would carry a non-`sim` origin.
  - So §6's claim that "only `events.meta` moves" is false.
- **Every gate I ran is green** (list under "Run").

### R1–R12

| # | Status | Note |
|---|---|---|
| R1 | Applied, with gaps | **Done:** `[role_domains]`, the 10 attributes, the `none` warning and D45's withdrawal. **Gaps:** <ul><li>the anchor rule "the first clause that binds A" is undefined or wrong for 3 of the 4 example patterns that use a domain (MIN-3);</li><li>`source` conflates two things (MIN-3);</li><li>no Python check of the section's shape (NIT-3);</li><li>stale v1.7 and v1.3 entries (NIT-1).</li></ul> |
| R2 | Applied, with gaps | **Done:** the slot, the check, `default` on `[[owns]]`, `05.1` v1.13 and the S6 text. **Gaps:** <ul><li>"0 = none" collides with EventId 0;</li><li>S6 declares no `cause_column` or `default`;</li><li>a step can still write the cause column (MIN-4).</li></ul> |
| R3 | Partial | **Done:** the draft fields, the v2 hash, and §2 now agrees with §5. **Open:** <ul><li>the origin plumbing is not hash-neutral (MAJ-1);</li><li>it is one hop only (MIN-1);</li><li>no mapping from an input to its origin (MAJ-1).</li></ul> |
| R4 | Applied | `h(attr name)`, `h(motive name)` |
| R5 | Applied | |
| R6 | Partial | `canonical_v0.md`'s status rule is unchanged; §11 says both are (MIN-5) |
| R7 | Applied | All 19 fixtures fail or pass by name. Round 1's planted variants are now refused |
| R8 | Applied in code | `canonical_v0.md` text not updated (MIN-5) |
| R9 | Applied, not total | MIN-8 |
| R10 | Applied | |
| R11 | Applied | <ul><li>Drafter: 4.0 and 14.3.</li><li>My independent split: 4.39 and 14.32.</li><li>`budget-bloat` fails check 6.</li></ul> |
| R12 | Applied, except (c) in the text | `canonical_v0.md:36` and `channel_c_v0.sql:150` still say 0–4 (MIN-5) |

### Findings

#### MAJ-1: origin on timers and commands moves smoke's `agenda.timers` and `engine.commands`

- **Where:** `envelope_v0.md:51`, `:139`, `:143`.
- **Problem:** §6 says "no column, timer, queue or draw moves". It also makes "the bisect names only `rh-core/events.meta`" SPR-020's acceptance criterion.
- **Evidence:**
  1. `:139` hashes `Timer.origin` and `QueuedCommand.origin` "as a one-byte field that is 0". That adds a byte to every row of `agenda.timers` and `engine.commands` (`rh-snapshot/src/state.rs:29-30`). EVO-001 R15 measured exactly this trap: "an empty option written after the payload moved `agenda.timers` at 849 rungs" (`EVO-001_LifecycleQueuesP1a.md:89`).
  2. Smoke's inputs are `InputKind::OracleMove` (`rh-golden/src/run.rs:570`). `apply_inputs` re-samples X3's timers in the `Inputs` phase (`rh-core/src/engine.rs:957-1004`). Under `:51` those timers, the `Settle` commands their fires send, and X4's drained events would carry the input's origin. So the rows move even with a neutral encoding.
  3. Nothing maps a journaled input to an origin. `MaterialInput` holds only its `scope`, and neither `05.2` nor the journal defines `OracleMove`/`Producer::System` → `sim` or `oracle`.
- **Fix:**
  - Write the origin as a tail only when it is not `sim`, as R15 does for `cause`, and say so in the `rh-snapshot` layout.
  - Define the input → origin mapping.
  - Either make a `Producer::System` input `sim`, or list `agenda.timers` and `engine.commands` in SPR-020's C3 regeneration and its bisect criterion.
- **Confidence:** 80%.

#### MIN-1: origin plumbing stops after one hop

- **Where:** `envelope_v0.md:51`.
- **Problem:** a timer carries the origin only when the `Inputs` job inserted or re-sampled it.
- **Evidence:**
  1. An Oracle input re-samples T1's break timer.
  2. Its fire emits `MURDER_ATTEMPT` (oracle) and inserts `DEATH_DUE`.
  3. `DEATH_DUE` was not inserted by "such a job", so its origin is `sim`.
  4. It submits `ScheduleDeath` (`sim`), so the `DEATH` is `sim`.

  Criterion 4 of `05.2` §4.0 misses the death. A drain's own timers and commands lose the origin the same way.
- **Fix:** every timer and command a job creates carries the job's origin, and a drain runs a command's effects (events, timers, commands) under that command's origin.
- **Confidence:** 70%.

#### MIN-2: the next-version lint does not guard the envelope fields

- **Where:** `specs/systems/lint_odd.py:484-490`, `:400`.
- **Problem:** `--next` compares `[[emits]]` `when` but not `roles`, `motives` or `snapshots`. It compares `[[owns]]` but not `default` or `cause_column`.
- **Evidence:** I planted two changes in the next M2. Both pass with "29 planted … 18 amendments" and PASS:
  - DEATH `roles = ["target"]`;
  - EXTINCTION `roles = ["witness"]` plus a motive.

  The roles agree today: DEATH is `[actor, target]` and EXTINCTION is `[]` in both files.
- **Fix:**
  - `roles` and `motives` may only grow, like `[[applies]] from`.
  - `snapshots`, `default` and `cause_column` stay equal unless an `[[amends]]` record changes them.
- **Confidence:** 75%.

#### MIN-3: the domain anchor and `source` are under-defined

- **Where:**
  - `envelope_v0.md:20`;
  - `05.4:38`;
  - `patterns.example.toml:106-181`;
  - `registry_v0.toml:270-284`.
- **Problem:** "Compiles at the first clause that binds A" fails on the example patterns:
  - **`false_positive_purge`:** `P = "kin of S"` is bound by no cast key.
  - **`famine_to_succession_crisis`:** `A` is first bound at a `match_event`, which has no snapshots.
  - **`guest_right_violated`:** `is_guest(G)` is read at G's flight (e1), and a pre-act `is_host(H)` is read at the aid that makes H the host (e2). The pattern can never match, even with sources.

  `source = "M7.dynasty"` names one system, but the snapshot must be declared by whatever emits the binding event. In `authority_purges_successor` that is T1's acts; for a dying ruler it is M2's `DEATH`. Yet the lint stays silent, although no P1a spec declares `holds_title`.
- **Fix:**
  - Let a pattern name the anchor (`G = "guest@e3"`), or anchor at the last binding clause.
  - Refuse a domain on a variable that no cast key binds.
  - Define `source` as the owner of the source column.
- **Confidence:** 65%.

#### MIN-4: `set_cause` details

- **Where:**
  - `envelope_v0.md:150`;
  - `S6.artifacts.odd.toml:85-111` and `:330`;
  - `rh-events/src/lib.rs:270`.
- **Problems:**
  - **"0 = none" collides with a real id.** The first event gets id 0 (`rh-events/src/lib.rs:270`). An artifact placed at world creation can have cause 0, which gives a false `ChangeWithoutCause`.
  - **S6 declares neither key.** `charge`, `condition` and `contaminants` carry no `cause_column` or `default`, although INV-S6-06 now cites "the declared defaults". The lint accepts them: I planted them and it passes.
  - **The rule is not enforced.** `SystemView::write` still hands out the cause column, so "never written by the step" is unenforced.
- **Fix:**
  - Store `EventId + 1`, or encode the cause as an option.
  - Declare the three keys in S6.
  - Leave cause columns out of a step's writes.
- **Confidence:** 70%.

#### MIN-5: `canonical_v0.md` lags R6, R8 and R12 (c)

- **Where:**
  - `canonical_v0.md:3`: the status rule still says "a change … is a new version". §11 (`envelope_v0.md:193`) claims both rules were rewritten.
  - `canonical_v0.md:61`: the record-level checks that `rh-stream` "must refuse" omit R8's rules (two actors, a cast over 256, a weight off `motivated_by`, evidence ≠ 1 on a deterministic kind).
  - `canonical_v0.md:36` and `channel_c_v0.sql:150`: the entity kind range still reads 0–4.
- **Fix:** amend the text and add a change log.
- **Confidence:** 90%.

#### MIN-6: version collisions with the other open PRs

- **Problem:** three PRs claim M7 0.3.3:
  - this PR;
  - PR #41 (`docs/handoffs/2026-10-16_architect_summary.md:111`: "PR #41 … takes 0.3.3");
  - PR #62's fold (`population_synthesis.md:72`: "M7 (0.3.2 → 0.3.3)").
- **Trial merges** (`git merge-tree`):
  - #41 conflicts in `M7.dynasty.odd.toml`, `M7_dynasty.rs`, `CLAUDE.md` and `check_all.sh`;
  - #59 and #60 conflict in `test_check_all_inventory.py` only;
  - #62 conflicts in `PRJ-002` and the fixtures README.

  #61 merged with #59 lints clean (19 amendments, mortality `--check` OK).
- **No other collisions:** M2 0.2.2, T1 0.3.3, `05.1` v1.13, `05.2` v1.8 and `05.4` v1.4 do not collide (#60 touches `05.9` and `05.10` only).
- **Fix:** apply the "next free number at merge" rule. If #61 merges first, the handoff line becomes "#41 takes 0.3.4", and #62's fold says "the frozen version at fold".
- **Confidence:** 85%.

#### MIN-7: `registry_version` is not bumped

- **Where:** `registry_v0.toml:7`.
- **Evidence:** ten attributes now sort before `stress`, so its `attr_id` moves from 22 to 32. That id is the segment and canonical code, and the canonical header's `registry_version` names those dictionaries. Version 0.2.0 was bumped for the same reason (the `verb_id` shift).
- **Fix:** bump the version and regenerate the canonical golden header.
- **Confidence:** 65%.

#### MIN-8: R9's coalescing is not total

- **Where:** `envelope_v0.md:122`.
- **Problems:**
  - The kind is defined only when one contributor is `motivated_by`. Pairs such as `triggered` + `enabled`, or `contingent` + `triggered`, are open, so two implementations can diverge.
  - "Evidence = the largest" turns a motivated edge merged with a deterministic one into evidence 1, which loses the margin.
- **Fix:** a fixed precedence of kinds, and the evidence of the motivated contributors.
- **Confidence:** 60%.

#### NITs

- **NIT-1: stale and out-of-order version entries.**
  - `05.2:9` (v1.7) and `05.4:6` (v1.3) still say "derived in plane C from intervals"; fold each into its newer entry.
  - The entries are out of order (`05.1:14` v1.13 sits above v1.12).
- **NIT-2: the frozen slice is now 0.2.2.** `next/M2.demography.odd.toml:6` and `next/README.md:113` still say 0.2.1.
- **NIT-3: no Python check of `[role_domains]`.** Nothing checks key coverage, arity against `relational`, or `source`. A planted `authority = {attribute = "is_successor", relational = false, source = "M9.bogus"}` passes `lint_patterns`.
- **NIT-4: "Every row the level wrote" is not knowable.** `write` returns `&mut Column`. Say "a scan of the declared columns, or a pre-level guard like `LifeCourseGuard`".
- **NIT-5: the parser accepts lines `append` refuses.** A `cause_id` ≥ its own `event_id`, and kind 6–8 edges. Refuse both, for R8's consistency.

### Answers to the brief

- **Q2: MAJ-1 and `set_cause`.** See the BLUF. The round-1 scripts on the new head give:
  - `macros.py`: 3,000 checks, 0 wrong, 5,436 ties;
  - `canon_probe.py`: two actors, a weight on a `triggered` edge and evidence 0 are now refused;
  - `budget.py`: as in the R11 row.
- **Q3: origin.** Yes, the origin fields move smoke's timer and command hashes. As written, "0 today" is not hash-neutral, unlike R15's empty `cause`. Criterion 4 is served only for one hop (MAJ-1, MIN-1).
- **Q4: `[role_domains]`.**
  - It is a C2 Registry change. `rh-manifest` loads it through the generic `sections` path (`registry.rs:341-345`) and `manifest_v0.py` ignores it, so nothing breaks today; §8's typing and refusals are SPR-020's work.
  - **V-WC-04 holds.** I built `rh-cli` (debug): `manifest_parity.py` reports 60 of 60 agree; `ownership` and `schema --diff` are clean; `cargo test -p rh-manifest` passes 32 + 10 + 2; `rh-registry` `real_specs` passes 12.
  - `registry_version`: bump it (MIN-7).
- **Q5: the `main` merge.**
  - `--next` and plain mode both PASS, with "29 planted changes rejected (3 of them by the amendments lock), 18 amendments recorded and locked".
  - The lint covers next specs: a planted `mother` role is an ERROR, and a missing role on the draft WARNs.
  - The roles match the frozen file. `--next` should compare roles too: MIN-2.

### Run (all green on `9755956`)

- **`selftest_envelope`** and its 4 controls, each failing its check by name.
- **`lint_odd`:**
  - real mode: PASS, 14 warnings;
  - `--next`: PASS, with the self-check line;
  - smoke: PASS;
  - 19 envelope fixtures, each as designed;
  - round 1's 5 planted variants.
- **Other spec gates:**
  - `lint_patterns` on both packs: PASS, with 4 `none` warnings on the example pack;
  - `canonical_v0 --check`, `selftest_channel_a`, `selftest_channel_c` and `selftest.py`;
  - `selftest_m2_f1`.
- **Generators:**
  - `gen_manifest --check`;
  - `gen_skeletons --check` (real and smoke);
  - `gen_hash_golden --check`.
- **Architecture lints:** `lint_arch` and `lint_forbidden`.
- **Python tooling:**
  - ruff: clean;
  - mypy: no issues in 44 files;
  - pytest: 619 passed.
- **Rust:**
  - `rh-cli` built;
  - V-WC-04 parity;
  - `ownership` and `schema --diff`;
  - `cargo test -p rh-manifest`;
  - `rh-registry --test real_specs`.

### Not run

- `bin/check_all.sh` as a whole.
- `cargo nextest` suites: rh-core determinism and agenda, rh-snapshot, rh-golden.
- The smoke golden run, the V-DET-11 bisect, and the perf and allocation gates.
- MAJ-1's ladder movement was not measured. It rests on EVO-001 R15's measurement of the same encoding, and on the code paths cited.
- The `selftest_m2_f1` controls: unchanged from `main`.
- Gates of the other PRs after a trial merge, except #61 + #59's `lint_odd` (both modes) and the mortality `--check`.

## 3. Round 3 (2026-10-17, `99691b0`)

### 3.0 Architect rulings, round 3

| # | Finding | Ruling | Confidence |
|---|---|---|---|
| R23 | MAJ-1 | **Accepted.** Rewrite §6's prediction from the code's actual path.<ul><li>Queued inputs carry the origin, and a resume restores it from the keyframe, so `engine.inputs` moves from rung 1 in `smoke` and in `solo`. `solo` has an `OracleMove` input on `main` since `df4a7f4`.</li><li>The bisect names `engine.inputs` before `events.meta`.</li><li>`agenda.timers` and `engine.commands` are not visible at a yearly rung.</li></ul>Make the SPR-020 sprint's acceptance robust: the audit records the measured first divergence, and every table that differs must be one that envelope v2 names. The prose prediction stays, marked as a prediction | 9/10 |
| R24 | MIN-1, MIN-2 | **Accepted.**<ul><li>State the origin tail's exact width and encoding, and its order relative to EVO-001's timer `cause` tail (origin after cause, both written only when non-zero, each with its presence marker).</li><li>Commands and timers created by a drain inherit the drained command's origin (smoke X4's `Audit`). Add this case to self-test check 7</li></ul> | 8/10 |
| R25 | MIN-3 | **Accepted.** `S6`'s `chain_head` and M7's `claim.cause_event` are set in the same step as their event, so they become `event_ref` columns written through `ctx.set_cause`. This is a declaration-only change: the frozen M7 text changes through its version bump in this PR, and S6 is a draft | 8/10 |
| R26 | NITs | All accepted:<ul><li>a bad `[role_domains]` gives a named refusal, not a traceback;</li><li>a `match_event` clause never anchors a domain;</li><li>the "source" wording;</li><li>the `opt(u64)` claim;</li><li>`default` is refused on an `event_ref` column;</li><li>rows a level inserts are covered by the `ChangeWithoutCause` check;</li><li>materiality is decided by the journal's own rule, and the spec's kind list follows it;</li><li>`lint_odd --next` handles a next spec with zero amendments without crashing (SPR-019's `next/M7` needs it);</li><li>check 7 compares the origin it expects, so a lost origin fails</li></ul> | 9/10 |
| R27 | Merge | Merge `origin/main`: SPR-012 `df4a7f4`, and PR #60 if it has landed by then. Then regenerate `golden/wire_v1.txt` with the tool if PR #60 is already on `main` (the merge order is free; the second to merge regenerates) | 9/10 |

The Architect verifies round 4 without another review agent unless the delta is large.

### 3.1 The review (round 3, independent reviewer)

Reviewer: independent and adversarial, on a different model from the drafter's. Copy reviewed: `$R3/src`, a git archive of `99691b0`. `$R3` = the Architect's scratchpad `envelope_review_r3/`.

Reviewer: independent, on a different model from the drafter's. Head reviewed: **`99691b0`** (`origin/arch/SPR-020-envelope`, equal to PR #61's head SHA by the REST API). Its merge base with `main` is `11b300d`. The review copy is `$R/src`, a git archive of the head. `$R` is `envelope_review_r3/` in the Architect's scratchpad. Nothing in the repository was modified. Line numbers refer to `99691b0`.

### BLUF

**Not ready to merge: 0 BLK, 1 MAJ, 3 MIN, 8 NIT.**

- **Applied:** R15 to R22, cleanly. Most of R13 and R14 is applied too.
- **Gates:** every gate I ran is green, and round 2's planted narrowings now FAIL (details under "Run").
- **The one MAJ:** §6's divergence prediction is wrong once you follow the code.
  - The queued input must carry its origin, because a keyframe resume does not re-submit queued inputs. So `rh-core/engine.inputs` moves from rung 1 in `smoke`, and in `solo` too: `solo` now has an `OracleMove` input on `main`.
  - `engine.inputs` sorts before `events.meta`, so the bisect names it first.
  - `agenda.timers` and `engine.commands` never differ at any yearly rung. The oracle-origin rows they hold live at most about 62 days after day 100.
- **Main has moved.** `main` is now `df4a7f4` (SPR-012, #56), which landed 20 minutes before the round-3 push. A trial merge conflicts in `python/tests/test_check_all_inventory.py` only.

### R13–R22

| # | Status | Note |
|---|---|---|
| R13 | **Partial** | <ul><li>**Done.** The tail is hash-neutral: rows are hashed length-prefixed (`hash.rs` chunk layout), and a 0 origin adds no bytes. The kind → origin mapping is stated (`envelope_v0.md:51`, `schema.sql` `input_journal.kind`). §6 is restated.</li><li>**Open: the prediction is wrong** (MAJ-1).</li><li>**Open: the tail's width and its position next to EVO-001's timer `cause` tail are not stated** (MIN-1).</li></ul> |
| R14 | Applied, with a gap | <ul><li>**Done.** The rule is in `05.1:89` (v1.13) and `envelope_v0.md:51`. Check 7 passes, and `--control origin-lost` fails check 7 by name.</li><li>**Gap.** A drain's own commands and timers have no rule or API (MIN-2).</li><li>Check 7 is shape-only (NIT-8).</li></ul> |
| R15 | Applied | <ul><li>`roles` and `motives` may only grow. Three new amendable paths.</li><li>The self-check reads "33 planted changes rejected (3 of them by the amendments lock), 19 amendments recorded and locked".</li><li>My probes are listed under "Run".</li></ul> |
| R16 | Applied | <ul><li>The anchor is defined, and an unbound domain is refused.</li><li>The three example patterns are fixed. `source` is split into `owner` and `column`.</li><li>Nits: NIT-1 and NIT-2, plus stale wording (NIT-3).</li></ul> |
| R17 | Applied | <ul><li>`event_ref` is an `Option<EventId>`, and no event is renumbered. S6 declares `default` and `cause_column` on its three columns.</li><li>The one Rust line compiles. `nextest -p rh-registry` passes 18/18, and `rh-cli ownership` reports 148 columns and 0 violations.</li><li>It is vocabulary only: no storage changes.</li><li>Nits: NIT-3 and NIT-4.</li></ul> |
| R18 | Applied | `canonical_v0.md`: the status rule, the §3 record checks, `entity_kind` 0–5 and a change log (§8). The golden file still has a motive edge and an untold cause. |
| R19 | Applied | `envelope_v0.md:88`. The `next/` lint path has a pre-existing crash (NIT-7). |
| R20 | Applied | `envelope_v0.md:147` and the PR body. |
| R21 | Applied; deviation stated | <ul><li>The precedence is total: `motivated_by`, then `contingent`, `triggered`, `enabled`, `probabilistic`, `relayed_knowledge`.</li><li>The deviation is argued at §11 (6/10) and in the PR body. I find it sound: `contingent` first would drop the motive edge from a `REVENGE`.</li></ul> |
| R22 | Applied | <ul><li>Version entries are in order and the stale text is gone.</li><li>The next M2 says 0.2.2.</li><li>The shape check exists (NIT-1).</li><li>The guard wording is fixed (NIT-5).</li><li>The parser refuses forward causes and kinds 6–8.</li></ul> |

### Findings

#### MAJ-1: §6's prediction misses `rh-core/engine.inputs` and is wrong at ladder granularity

- **Where:**
  - `specs/chronicle/envelope_v0.md:143` (smoke);
  - `:144` (solo, household and hamlet);
  - the PR body's acceptance checklist.
- **Evidence:**
  1. **The engine must carry the origin through the input queue.**
     - The `Inputs` job needs the input's origin, but `MaterialInput` holds only `scope` (`rh-core/src/engine.rs:102`).
     - Queued inputs are plane-A state, hashed as `rh-core/engine.inputs` (`rh-snapshot/src/state.rs:199`).
     - A resume restores them from the keyframe and does not re-submit them from the journal (`rh-snapshot/src/resume.rs:8`).
     - So the origin has to ride in the queued row. At `oracle`, that row moves whatever the tail rule.
     - The spec never mentions `MaterialInput` or `engine.inputs`.
  2. **Smoke journals and submits all 1,136 inputs before the first instant** (`rh-golden/src/run.rs:502`; ladder header `journal 1136`). `engine.inputs` is non-empty from rung 1 (baseline root `90eef272…` at rungs 1–7), so it moves from rung 1 until the last input in year 994.
  3. **The bisect names the wrong table.** `PlaneDigest::first_divergence` walks tables in name order (`hash.rs:337`), and `rh-core/engine.inputs` sorts before `rh-core/events.meta`. At month 1 the bisect therefore names `engine.inputs`, not `events.meta`, and `events.meta` is not "alone at that instant".
  4. **The re-sampled X3 timers never reach a yearly rung.** They land 1 to 60 days after the input on day 100 (`vital.rs:32`, `resample_timer`). Their `Settle`, and X4's per-`Settle` `Audit`, are drained within a day or two each. No oracle-origin timer or command is pending at any year end, so the ladder never shows `agenda.timers` or `engine.commands`. "Each from the rung predicted here" (year 7) would fail. Those rows are visible only at instant or month granularity inside input years.
  5. **`solo` now has an input.** On `main` (`df4a7f4`), `worlds/solo/inputs.toml` holds one scope-only `OracleMove` on Ada, journaled before the first instant. So `solo`'s `engine.inputs` moves from rung 1 to rung 53. The claim that `solo` "submits no material input" is false once main is merged.
- **Fix:**
  - Add `MaterialInput.origin`, and write the `engine.inputs` row as `… list(entity scope)` followed by a `u8 origin` tail only when non-zero.
  - Restate §6:
    - the yearly ladders of `smoke` and `solo` name `engine.inputs` (`smoke` rungs 1–994, `solo` rungs 1–53) and `events.meta`;
    - the bisect names `rh-core/engine.inputs` first, at month 1;
    - `agenda.timers` and `engine.commands` move only at instants inside an input year, so test them with a targeted monthly ladder over year 7, or an engine test, and not with the yearly ladder.
  - Re-merge `main` (one conflict, in the inventory test).
- **Confidence:** 85%. The movement is inferred from the code paths and the baseline's roots. I did not implement origin or run the ladder.

#### MIN-1: the origin tail's encoding is not exact

- **Where:** `envelope_v0.md:51` (the tail and "a keyframe carries them as a nullable column"), `:139` (§6); `05.1:89`.
- **Problem:**
  - The text says "a tail written after the row's last field only when non-zero", but it gives no width (`u8`?).
  - EVO-001 puts the timer's `cause` slot "after the payload only when present" (`05.1` rule 2). Today the payload is followed by `u8 tombstoned` (`state.rs:29`). So the two optional tails have no stated order.
  - Rows are length-delimited, so a decoder can tell the four lengths apart only if the order and widths are fixed. Two implementations could hash differently.
  - "A keyframe carries them as a nullable column" does not match the keyframe, which stores each row as bytes (`keyframe.rs:13`).
- **Fix:** state the full rows:
  - `agenda.timers`: `… i64 payload, u8 tombstoned[, u64 cause if present][, u8 origin if ≠ 0]`;
  - `engine.commands`: `… opt(u64 draft)[, u8 origin if ≠ 0]`, decoded by the remaining length;
  - the same for `engine.inputs` (MAJ-1).
- **Confidence:** 75%.

#### MIN-2: a drain's own commands and timers have no origin rule

- **Where:** `envelope_v0.md:51` (only `ctx.emit_from(&command, draft)`) against `05.1:89` ("a drain runs a command's effects under that command's origin").
- **Problem:** the two texts give two readings.
- **Evidence:**
  - Smoke's X4 drain submits an `Audit` for some `Settle`s (`estate.rs:60`). Is it the job's `sim`, or the `Settle`'s `oracle`? Nothing names a `submit_from` or a `schedule_from`.
  - X7 emits one aggregate `TRADE` per drain, over audits of mixed origins, and no rule covers it.
  - So `engine.commands` bytes can differ between implementations.
- **Fix:**
  - Add a scoped `ctx.under(&command, |ctx| …)`, or `submit_from` and `schedule_from`, so that everything created for a command carries the command's origin.
  - An effect over several commands carries the job's origin.
- **Confidence:** 70%.

#### MIN-3: event-id state columns set in the same step have no slot (outside R13–R22, surfaced by R17)

- **Where:**
  - `S6.artifacts.odd.toml:48` (`artifact.chain_head`, the latest chain event, which the same step emits);
  - `M7.dynasty.odd.toml:292` (`claim.cause_event`, "the SUCCEED or DEPOSED event that created it").
- **Problem:**
  - Both are `u64` and written by the step, but the step holds only a `DraftRef`. This is round 1's MAJ-2 again.
  - `set_cause` covers only `cause_column` siblings.
  - SPR-021 codes M7 against this text.
- **Fix:**
  - Let the buffered slot write any `event_ref` column (`ctx.set_event_ref`).
  - Type these columns `event_ref` at their next versions.
  - Or state that they are written from a known id only.
- **Confidence:** 60%.

#### NITs

- **NIT-1: a bad `[role_domains]` is refused by a crash, not by name.**
  - **Where:** `lint_patterns.py:126`, `:322`.
  - **Problem:** `lint()` returns the shape errors, and then the self-check `assert`s, which run on the supplied Registry, raise `AssertionError`. Round 2's `reg_bad`, my planted variants and its old `source` shape all exit 1 with a traceback.
  - **Fix:** print and return the shape errors before the self-checks.
- **NIT-2: a `match_event` clause with a cast key anchors a domain.**
  - **Where:** `lint_patterns.py:140`.
  - **Problem:** `{match_event = "m1", actor = "A"}` with `A = "authority"` passes, yet §1 says a `match_event` has no cast.
  - **Fix:** refuse cast keys on a `match_event` clause.
- **NIT-3: stale or inaccurate wording.**
  - `envelope_v0.md:211` says "one whose source is `none` warns"; it should say `owner`.
  - `:151` says `opt(u64)` is "the encoding `rh-snapshot` already defines". That holds for engine tables only. World columns are `i64` rows, 8 bytes (`keyframe.rs:18`), so an `event_ref` world column is a new encoding: say it is the sprint's.
- **NIT-4: a `default` on an `event_ref` column is accepted.**
  - **Problem:** `lint_odd` passes `default = 0` on `artifact.condition_cause`, but 0 is a real id and the type's default is none by definition.
  - **Fix:** refuse a `default` on `event_ref`.
- **NIT-5: rows a level inserts are not "kept rows".**
  - **Where:** `envelope_v0.md:151`, `05.1:89`.
  - **Problem:** a new artifact created with condition below 1 and no cause escapes the `ChangeWithoutCause` check.
  - **Fix:** say that an inserted row counts as changed from the default.
- **NIT-6: materiality is a function of scope.**
  - **Where:** `rh-snapshot/src/journal.rs:233`.
  - **Problem:** a `ManifestSet`, `SettingChange` or `ConsoleMode` row with a scope is material, and has no origin. §2 says they "reach no plane-A job", which holds only by convention.
  - **Fix:** refuse a scope on them at append, or map them to an origin.
- **NIT-7 (pre-existing, from SPR-015): `lint_odd --next` crashes on a next spec with no amendments.**
  - **Where:** `lint_odd.py:1494`.
  - **Problem:** `next(iter(want))` raises `StopIteration`. A clean `next/T1.minds.odd.toml` reproduces it. R19's `next/M7.dynasty.odd.toml` hits it if it amends nothing.
  - **Fix:** guard the lock probes with `if want:`.
- **NIT-8: check 7 cannot catch a lost origin.**
  - **Where:** `selftest_envelope.py:362`.
  - **Problem:** the generator stamps each hop's origin itself, so the check proves only that the data shape is right. It models no timer, command or drain.
  - **Fix:** fine for a spec self-test. Name the engine test that owns the property (V-DET-21 or a new V-*).

### Run (all on `99691b0`)

- **`selftest_envelope`:** PASS (checks 1–7, including "14 inputs, 28 hops"). Each of its five controls fails only its own check, by name.
- **`lint_odd`:**
  - real mode: PASS, 14 warnings;
  - `--next`: PASS, with "33 planted changes rejected (3 of them by the amendments lock), 19 amendments recorded and locked";
  - smoke: PASS;
  - `--write-amendments-lock` leaves no diff.
- **The 19 `odd_envelope` gates of `check_all`:** each passes or fails as designed. This covers round 2's `fx_cases.txt`; the `cause_column` case now expects `event_ref`.
- **The four `odd_next_control` cases:** each FAILs with its diagnostic.
- **Round 2's next-mode probes, re-planted on the merged next M2** (`$R/fx_next/`). The five changes must FAIL, and they do:
  - `next_roles` (DEATH narrowed to `target`);
  - `next_noroles` (DEATH's `roles` removed);
  - `next_badrole` (`mother`, and MARRIAGE's `roles` dropped);
  - `next_t1_motive` (a next T1 drops `VOW_OF`);
  - `next_t1_snap` (a next T1 drops `stress`).

  `next_snap` grows EXTINCTION to `[witness]` and `GRIEF_FOR`, and passes, as R15 allows.
- **The next M2 against `11b300d`:** it differs only in the four `roles` lines and the 0.2.2 mentions.
- **`lint_patterns`:**
  - both packs PASS, with 2 `owner = none` warnings on the example pack;
  - every bad `[role_domains]` probe exits 1, via NIT-1 (`$R/fx_dom/`);
  - `$R/fx_pat/`: an unbound domain on an `unless` clause is refused; NIT-2's planted pattern passes.
- **S6 probes** (`$R/fx_s6/`). These must FAIL, and they do:
  - a missing default;
  - a `u64` cause column;
  - a mistyped default;
  - a foreign cause column.

  Round 2's S6 with a `u64` cause now FAILs. NIT-4's planted default passes.
- **Other spec gates:**
  - `canonical_v0 --check` (78 golden lines);
  - `selftest_channel_a`, `selftest_channel_c` and `selftest.py`;
  - `selftest_m2_f1`, and `selftest_m2_f1_mortality --check`.
- **Generators:**
  - `gen_manifest --check`;
  - `gen_skeletons --check` (8 real, 7 smoke);
  - `gen_hash_golden --check`.
- **Architecture lints:** `lint_arch` and `lint_forbidden`.
- **Python tooling:**
  - ruff: clean;
  - mypy: 44 files;
  - pytest: 619 passed.
- **Rust:**
  - `cargo nextest run -p rh-registry --locked`: 18/18;
  - `rh-cli` built (debug);
  - `ownership`: 0 violations;
  - `manifest schema --diff`: clean.

  The only hand-written Rust change in the PR is `COLUMN_TYPES`. The rest is generated `SPEC_VERSION` constants.
- **Trial merge with `origin/main` `df4a7f4`:** one conflict, in the inventory test.

### Not run

- `bin/check_all.sh` as a whole.
- The `nextest` suites of `rh-core`, `rh-snapshot` and `rh-golden`.
- The smoke and solo golden runs, the bisect, and the perf and allocation gates.
- MAJ-1's ladder movement was not measured: nothing implements origin yet. It rests on the code paths and the baseline's per-table roots.
- The gates after a full merge with `df4a7f4`: only `git merge-tree` was run.
- `manifest_parity.py`.

`$R/target` was deleted after the run.

## 4. Round 4: the Architect's check (2026-10-17, `c855441`)

Everything below ran on an archive copy of `c855441`, which is merged with `main` at `a53cac2`:

| Check | Result |
|---|---|
| `lint_odd.py` default / `--next` / smoke | PASS (8 / 8 / 7 specs). The self-check reads "33 planted changes rejected (3 of them by the amendments lock), 19 amendments recorded and locked" |
| `lint_patterns.py` on both packs | Exit 0 |
| `selftest_envelope.py` (venv Python, DuckDB) | ENVELOPE v0: PASS. `--control origin-lost` and `--control budget-bloat` both FAIL, as required |
| `selftest.py`, `selftest_channel_a.py`, `selftest_channel_c.py` | PASS |
| `canonical_v0.py --check`, `selftest_wire_v1.py --check` | OK: 78 and 191 lines |
| `gen_manifest --check`, `gen_skeletons --check` | Clean; 8 skeletons up to date |
| pytest | 655 passed |
| CI on `c855441` | PASS (core and golden) |

**The text, read:**
- §6 is rewritten from the code's path and marked as a prediction. `engine.inputs` and `events.meta` move at yearly rungs; `agenda.timers` and `engine.commands` move only inside an input year. The acceptance is robust to it: the measured first divergence is recorded, and every table that differs must be a named one.
- R24's tail encodings are stated byte for byte.
- R25 covers S6's `chain_head` and `origin_event` and M7's `claim.cause_event`. **Accepted:** S6 `origin_event` as `event_ref` (7/10).

**Merge:** PR #61 merges `main` at `5bf6ec6` (PR #65's SPR-013 T-09 line; both kept in SPR-013 1.0.3), then merges on green CI. PR #62's `next/M7` 0.4.0 then carries M7 0.3.3's `roles`.

