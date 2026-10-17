---
id: REV-SPR-020
title: "Independent review of the event envelope and role codes spec (PR #61)"
type: record
status: OPEN
owner: architect
agents: [architect]
tags: [verification]
related: [PRJ-002, SPR-020, SPR-013, SPR-021, SPR-026, SPR-029, EVO-001, BCK-002]
created: 2026-10-17
updated: 2026-10-17
version: 0.1.0
---

> **BLUF: round 1 on `742ed09` found 0 blockers, 3 majors, 9 minors and 5 nits. Not ready to merge.**
> - **Sound:** the role reconciliation, the v2 field list, plane separation, the provenance macros and the canonical order.
> - **The majors:**
>   - abstract-role domains read from `state_interval` at the event's tick are unanchored, so a deposed ruler is never "authority" at their own deposition;
>   - `cause_column` cannot be implemented with today's merge;
>   - four event-level fields are outside both the draft and the digest.
> - **Rulings:** R1–R12 in §0.
> - **The `main` merge** (`67aa812`, `08a8db0`) is covered by the round-2 check.
> - **Reviewer:** a session on a different model from the drafter's.

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
