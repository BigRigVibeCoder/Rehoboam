# `specs/systems/next/`: next versions of frozen system specs

A frozen spec (`status = "frozen"` in `specs/systems/`) is a contract: code was built against it,
its skeleton is committed (`bin/gen_skeletons.py`) and any change is a new `spec_version` through
the Architect (C2). The next version has to be written, reviewed and linted against the other
specs *before* it replaces the frozen one, and `lint_odd.py` keys specs by system id, so two files
with one id cannot share `specs/systems/`. This directory holds the next version while it is a
draft.

## What `lint_odd.py` accepts today, and what `--next` adds

Without `--next`: one `*.odd.toml` per system id in `specs/systems/` (a repeated id is refused,
also under `--extra-specs`); `--specs DIR` replaces the whole set (with the smoke world's X-prefix
rules); `[system].fidelity` is one level, checked against the Registry's `fidelity_levels`;
`[[fidelity_ladder]]` is free text; there is no notion of a version, slice or profile of one system.

With `--next DIR` (SPR-015): every `*.odd.toml` in `DIR` **supersedes** the spec of the same id in
`specs/systems/` for that run, so the other specs' reads, commands and `[[applies]]` are checked
against the next version, and the lint then checks that the next version **preserves the frozen
slice** (`lint_next` in `lint_odd.py`):

| Frozen element | Must be preserved in the next version |
|---|---|
| `[system]` | same `id`, `crate`; `spec_version` strictly greater (semver); `status = "draft"`; the frozen spec's status is `frozen` |
| `[[owns]]` | every column, with the same `entity`, `type`, `plane` and `fixed_point`; the same `meaning` or an amendment |
| `[algorithm.<name>]` | every table, equal in every key (the frozen algorithm text, `version`, `check_values`); a frozen step changes only by amendment, and an amended step needs a changed `version` |
| `[[rng]]` | the frozen purposes first, in the same order, with the same `key`, `resample_on_fork` and `draws` (new purposes only after them: the generated `RngPurpose` discriminants are positional); the same `use` or an amendment |
| `[[invariants]]` | every id, with the same `test`; the same `statement` or an amendment |
| `[[applies]]` | every command, with the same `payload`; `from` may only grow |
| `[[schedule]]` | every `(pulse, phase)` binding with the same `when` or an amendment, and every frozen step, equal or amended (steps may be added) |
| `[[emits]]` | every kind, its frozen `when` equal or amended |
| `[initialization]` | the frozen `state` text equal, or amended |
| `[[params]]` | every name; a frozen `default` keeps its value; the same `meaning` or an amendment (REV-SPR-015 round 3 NIT-5). In both modes an entry is q16_16 unless it declares `type = "count"`, an integer with integer bounds and default (`u64` in the skeleton; SPR-016 MR2) |
| `[verification]` | every frozen test id stays in `tests` (new ones may be added; the `prototype` may change) |
| `[[couplings]]` | every frozen `(system, direction)` pair stays (new couplings may be added) |
| **every other frozen leaf** (PR #79's review R3 and N4, deny by default) | equal, or amended through a record on its generic path `<section>.<key>.<field>` (a keyed section's entry by its naming field: `owns.<column>`, `emits.<kind>`, `commands.<command>`, `applies.<command>`, `reads.<column>`, `fidelity_ladder.<level>`, `rng.<purpose>`, `params.<name>`, `submodels.<name>`, `invariants.<id>`, `couplings.<system>/<direction>`, `schedule.<pulse>/<phase>`) or `<table>.<field>` (`system`, `purpose`, `scales`, `design`, `initialization`, `input_data`, `isolation`, `verification`), with `purpose.patterns.<test>.<field>` and `isolation.forcing.<input>.<field>`. A list-valued field is quoted as JSON. A repeated key (T1 declares `S7.vows/in` twice) carries its ordinal, `key`, `key[1]`, `key[2]`, in file order: an amendment names the n-th repeat by its ordinal, and a next version appends repeats after the frozen ones (N14). Entries may be added (a read, an emitted kind, a submodel, a command, a coupling, a pattern, a forcing input); a frozen entry's fields may not change: a read's `lag`, `use` and `neutral`, an applied command's `drained_by`, `key`, `key_field`, `outcome` and `sprint`, a column's `registry`, `initialization.from_manifest`, `input_data.drivers` among them |
| **the exemptions** (the list the Architect rules on, N4) | (1) `system.spec_version`, `system.status` and `system.fidelity`: a next version moves them by definition; `system.id` and `system.crate` are checked by name. (2) Lists that may only grow, each with its own check: `verification.tests`, an `[[emits]]` entry's `roles` and `motives`, an `[[applies]]` entry's `from`. (3) `owns.<column>.values`: an enum's codes are its values' indexes, so a next version may append a value and never rename or move one (the prefix rule; a control plants a renamed code). (4) `[algorithm.*]` tables: their own rule above (equal in every key, a step amended with a `version` bump). (5) `[[amends]]` itself. Nothing else is exempt: `[[reads]]`, which the first draft of this rule left free, are protected. **A frozen keyed entry cannot be withdrawn** (a missing `[[reads]]`, `[[owns]]`, `[[emits]]`, `[[commands]]` or `[[applies]]` entry fails, naming it, and no `[[amends]]` path expresses a withdrawal): withdrawing one is a change to the frozen spec's surface, which needs the Architect's ruling and a new spec version, not a next-version amendment (Architect, PR #79 round 3, P5) |

Two more rules (REV-SPR-015 R6, R15): a frozen `[[applies]]` keeps at least its frozen `from`; and every frozen
text above is compared **for equality, line by line** (a superset that keeps the frozen text inside a longer one no
longer passes: the change goes through an amendment). In `--next` mode the superseding spec is held to
**frozen-level strictness** on `[[applies]]` and `[[emits]]` even though its status is `draft`. In both modes a
spec with a duplicate key (an owned column, parameter, RNG purpose, invariant id, emitted kind, applied command,
schedule binding or step name within a binding) is refused, so a second entry can never shadow the one the lint
compares (R15).

**Amending a frozen statement** (REV-SPR-015 R1). A next version may change a frozen statement only through a
`[[amends]]` record:

```toml
[[amends]]
of = "invariants.INV-M2-03.statement"     # also algorithm.<name>.steps.<Sn>, emits.<KIND>.when, initialization.state,
frozen = "<the frozen text, byte for byte>" #   schedule.<Pulse>/<Phase>.steps.<name>, schedule.<Pulse>/<Phase>.when,
to = "<the new text, what this file now says there>"   # owns.<entity.column>.meaning, rng.<purpose>.use,
reason = "<why>"                                         # params.<name>.meaning, and every generic path of the row above
```

The lint checks `frozen` against the frozen file and `to` against the next file, refuses a record whose texts are
equal or whose path has no text, and prints every amendment as an `AMEND` line and a count
(`amendments (next of <id>): N recorded`), so a reviewer sees each change to the frozen meaning in the run's
output. A change without a record is an error; an amended algorithm step under an unchanged table `version` is an
error too (REV-SPR-015 R17.4).

**The amendments lock** (REV-SPR-015 round 3 MIN-1). A count can be kept while its content changes (a valid record
that amends a frozen arithmetic step while a meaning amendment is withdrawn), so the set is pinned, not counted:
`next/amendments.lock.toml` lists, per system, every amended path with the sha256 of its new text. The lint reads it
on every `--next` run and fails, naming the path, on any amendment added, withdrawn or changed since the lock (and
on a missing lock). It is written only by the tool, `lint_odd.py --next … --write-amendments-lock`, after the
reviewer has read the new `AMEND` lines; it is never typed. `bin/check_all.sh` renders a control with one digest
changed, which must fail, and seventeen deny-by-default controls (one planted change per generic path class on the M7
draft, the reviewer's `q5_summary`, `q5_why` and `q5_frozen_onbirth` cases, which linted PASS before R3, and round 2's
enum code renamed, `drained_by` moved, `from_manifest` edited and a read's `lag` changed, N4).

**A new table never overrides a frozen step.** A frozen step changes only through an `[[amends]]` record on its
own table. The lint refuses a new `[algorithm.*]` table whose step reuses a frozen step id (`S9` in a table of the
next version's own) or whose text names a frozen step as replaced, overridden, superseded, void or no longer
applying. Prose can be written around any regex, so this is also a **review rule**: a reviewer reads every new
table for a sentence that changes what a frozen step computes and demands the amendment.

A draft spec that still reads a column from the owner it had before the next version claimed it (S6's
`household.head` from M4 today) warns in `--next` mode and names the follow-up; a frozen reader errs as always.

**Follow-ups recorded here.** Mortality's entitlement read (REV-SPR-016-FOLD R2): the read of M4's household
entitlement ratio (`household.food_entitlement_ratio`, provisional name) that `[algorithm.mortality_f1]` MR11 and
fertility's hook (`eps_f`) will share, with its M4 in-coupling and its isolation forcing, is deferred to M4's first
spec (P1.5a): a rate read needs a neutral typed by the owner's column, M4 has none, and the frozen-mode lint refuses
the spec at promotion otherwise. MR11 states the form with the factor exactly 1 until then.

**A frozen bump and its next version (REV-SPR-019 R13).** A frozen spec may be bumped (a declaration-only bump such as M7 0.3.2 → 0.3.3 in PR #61) while a next version of it is a draft here. The `[[amends]]` records quote the frozen text byte for byte and every other frozen element must be preserved, so the draft breaks the moment the bump lands. The rule: of the bump and the next version, **whichever lands second re-derives the next version from the new frozen file** (copy the new frozen text, re-apply the draft's amendments and additions, regenerate the lock with the tool) in its own PR, before it is merged. The lock's digests do not change when the amended texts do not.

**One draft per system id.** `next/` holds at most one next version per id, and only a frozen spec takes one, so a
second slice of the same system is drafted standalone and folded into the draft once the first slice has merged (SPR-016's
mortality was drafted as `specs/demography/mortality_f1.toml` under PR #51 and folded into the M2 draft after PR #50).

A `--overlay FILE` given with `--next` may add `[verbs]` and may **replace** the `[systems]` entry
of a system that has a next-version spec (never another), keeping `fidelity_levels`, every
existing parameter's bounds and every existing default; the lint checks those too. A violation of
any row above is an error. The positive controls are rendered by `bin/check_all.sh` from the next spec itself with `sed` (`tests/lint_fixtures/README.md`, `odd_next`), so they cannot drift from it.

Run: `python -I specs/systems/lint_odd.py --next specs/systems/next --overlay specs/systems/next/registry_delta.toml`

A set that does not lint gets its `ERROR` lines and `FAIL` and nothing else: the lint's self-checks (the planted
probes) run only on a clean set, so a broken spec can never bury its diagnostic under a self-check assertion
(REV-SPR-015 round 3 NIT-4).

## Lifecycle of a next version

1. The drafter writes `next/<id>.odd.toml` (the frozen file plus the new slice) and, if the
   Registry must change, `next/registry_delta.toml`.
2. Both lint green here; the reference and the self-tests of the new slice are added to
   `bin/check_all.sh` with planted controls.
3. The director approves the sprint plan that builds the new slice (GOV-005 §2).
4. In that sprint's PR the Architect moves the file over `specs/systems/<id>.odd.toml`, merges
   the delta into `specs/manifest/registry_v0.toml`, regenerates the skeleton, sets
   `status = "frozen"` and empties this directory. `bin/gen_skeletons.py --check` and the ordinary
   lint take over. Promotion checklist (REV-SPR-016-FOLD R6): update the two `CLAUDE.md` lines that
   say "next-version spec" (births' reference) and "specs/systems/next while it is a draft"
   (mortality's); both references then read the promoted spec through their fallback path, which
   stays live. For M2's 0.3.0 that PR is **SPR-016's**, which merges after SPR-015's (R3): SPR-015
   builds against the draft under the interim rule of its `[algorithm.parameters_f1]` P1
   (`mortality_rung0` for everyone, β ≠ 1 refused).

   **Promotion checklist, extended** (the Architect's ruling on the wave-5 plans, 2026-10-17, from
   finding W1 of their independent review, which simulated the promotion on a copy; the same ruling,
   D-1, makes the promoting PR **SPR-015's**: the two sentences above are superseded and are corrected
   at the promotion). Besides the file move, the delta merge, the skeleton and the two `CLAUDE.md`
   lines:
   (a) re-point every draft spec that reads a column the promoted spec now owns from its old owner
   (S6's `household.head` from `M4.economy`: the frozen lint and `--next` both fail on it otherwise);
   (b) give every reference that loads the next spec a fallback to the frozen one
   (`specs/worldgen/selftest_population_synthesis.py` loads `NEXT_SPEC` with no fallback);
   (c) make `specs/annals/fixtures/gen_fixtures.py` and `specs/annals/selftest_annals_v0.py` read the
   verbs and the version from the Registry alone (today they add the delta's verbs and name the
   dictionary `<version>+next`), and regenerate the annals fixtures and goldens with the tools;
   (d) re-render `bin/check_all.sh`'s `odd_next_control` cases from a synthetic next version of the
   promoted spec (as the `clean` case renders one of T1) and re-pin the `--next` gate's self-check line
   to the draft that remains;
   (e) point the stale-check-value controls (`m2_f1_stale_control`, `m2_f1_mortality_stale_control`,
   and `minds_stale_control` for T1) at the promoted spec;
   (f) fix the spec's own text that named the promotion point or a pre-freeze condition, and the
   `CLAUDE.md` `--next` line;
   (g) do not bump `registry_version` with the delta: the bump belongs to the sprint's final `[C3]`
   commit with its baselines, and a bump alone also changes every annals golden, which pins the
   chronicle's version.

`bin/gen_skeletons.py` and `gen_manifest.py` read `specs/systems/*.odd.toml` only: a next version
never changes a committed skeleton or the index until step 4.

## Current contents

| File | What |
|---|---|
| `M2.demography.odd.toml` | 0.3.0 draft: M2 F1 births, partnering, households, kinship (SPR-015) and F1 mortality (SPR-016, `[algorithm.mortality_f1]`, folded after PR #50) on the frozen rung-0 slice (0.2.2: 0.2.0 plus EVO-001's typed payloads and codes, and SPR-020's `roles`); D56 (e) (BCK-002 A-62): `[algorithm.fertility_f1]` F3–F6 read the elite table on the woman's `house_member` row or her husband's (`m2-f1-fertility-v2`); the wave-8 review's G10: `[isolation]` expect (1) gates on the seed-mean r by a 99.9% t-interval against M2's expected r and reports the population range (its `[[amends]]` record re-stated) |
| `M7.dynasty.odd.toml` | 0.5.0 draft (SPR-019, PR #62, then BCK-002 A-56 and A-57 under PR #79, `M7_membership.md`, pending the director's D56): the frozen 0.3.4 with ten `[[amends]]` records: `[initialization]` and `tenure.title` (SPR-019), `house_member.house` (agnatic membership: the father's house, else none, the back-fill's rule), `INV-M7-06` (A-57: one list of naming criteria, naming wins, a named minor is never demoted; a house with a member father's pregnancy pending is not extinct), and under the deny-by-default lint the `on_birth`, `on_death`, `vocation` (later children, R4) and `dormancy` (criterion (d) widened to any title's heir, R6) summaries and the Promote and Demote `why` texts; a new read of M2's `person.unborn_father` |
| `registry_delta.toml` | `[systems."M2.demography"]` with the F1 defaults (`tfr`, `elite_fertility_mult`) and mortality's two parameters with their defaults (`density_capacity`, `density_mortality_slope`); verbs `BIRTH`, `MARRIAGE` (M2 0.3.0); the M7 draft adds none |
| `M7_membership.md` | the analysis behind the M7 0.5.0 draft (BCK-002 A-56, A-57): the three membership rules in M7's isolation regime and the T1-registered regime, the levers that keep a house alive, the named-actor counts, the effect on succession eligibility by law, what the annals would say, the precedence rule, the recommendation (agnatic with the later-children reading and the elite read on the couple, 6/10, adopted by D56 on 2026-10-17), the specs table and D56 |
| `T1.minds.odd.toml` | 0.4.0 draft (the T1-A spec PR, before SPR-023; the Architect's rulings X1, X4, X7, X13, R9 and R14; REV-MINDS M1, M5–M7, M10 and findings 2, 3, 15, 16): the frozen 0.3.3 with `[algorithm.minds_f1]` A0–A16, the A-steps only (the mind table and its key `mind.person`, the derived indexes and their freshness inside a binding, the trait draw, world start by X4, Promote and Demote with the deferred Demote dropped, the instant's exits through `StepCtx::exited()`, liege and vassal rows refreshed monthly, opinion modifiers, thoughts and goals, the Registry tables the A-part reads, the invariants' cadence); new: `mind.person`, `thought.refreshed_at`, the reads `person.id` and `title.id`, the steps `world_start`, `exits` and `relations`, INV-T1-11 to 14; twenty-one `[[amends]]` records (`[initialization]`, `trait_draw`'s use, the promote and grief summaries, Day/Agenda's `when`, the grief step of Day/Vital, five column meanings, the ten invariants' statements with their cadence). Its exact reference is `specs/minds/selftest_minds_v0.py`. The I- and S-steps follow as a 0.5.0 draft (T1-I/S) after SPR-023 T-01 promotes this one (ruling R7) |
| `amendments.lock.toml` | the nineteen amendments of the M2 draft (five texts, seven column meanings, seven parameter meanings; the mortality fold re-stated five and added `e0`'s; SPR-019 re-stated `[initialization]`) and, under the deny-by-default lint (PR #79's review R3), the 26 generic-path records of the M2 draft and the ten of the M7 draft, the twenty-one of the T1 draft (T1-A), path and sha256 of the new text; written by `lint_odd.py --write-amendments-lock`, never typed |
