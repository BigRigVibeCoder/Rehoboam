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
| `[[owns]]` | every column, with the same `entity`, `type`, `plane` and `fixed_point` (`meaning` may change) |
| `[algorithm.<name>]` | every table, equal in every key (the frozen algorithm text, `version`, `check_values`); a frozen step changes only by amendment |
| `[[rng]]` | the frozen purposes first, in the same order, with the same `key`, `resample_on_fork` and `draws` (new purposes only after them: the generated `RngPurpose` discriminants are positional) |
| `[[invariants]]` | every id, with the same `test`; the same `statement` or an amendment |
| `[[applies]]` | every command, with the same `payload`; `from` may only grow |
| `[[schedule]]` | every `(pulse, phase)` binding with every frozen step (steps may be added; a frozen step changes only by amendment) |
| `[[emits]]` | every kind, its frozen `when` kept inside the new text or amended |
| `[initialization]` | the frozen `state` text kept inside the new one, or amended |
| `[[params]]` | every name; a frozen `default` keeps its value |

Two more rows (REV-SPR-015 R6): a frozen `[[applies]]` keeps at least its frozen `from`; a frozen `[[emits]]`
`when`, the frozen `[initialization]` text and every frozen schedule step are kept inside the new text. And in
`--next` mode the superseding spec is held to **frozen-level strictness** on `[[applies]]` and `[[emits]]` even
though its status is `draft`.

**Amending a frozen statement** (REV-SPR-015 R1). A next version may change a frozen statement only through a
`[[amends]]` record:

```toml
[[amends]]
of = "invariants.INV-M2-03.statement"     # also algorithm.<name>.steps.<Sn>, emits.<KIND>.when,
frozen = "<the frozen text, byte for byte>" #   initialization.state, schedule.<Pulse>/<Phase>.steps.<name>
to = "<the new text, what this file now says there>"
reason = "<why>"
```

The lint checks `frozen` against the frozen file and `to` against the next file, refuses a record whose texts are
equal or whose path has no text, and prints every amendment as an `AMEND` line, so a reviewer sees each change to
the frozen meaning in the run's output. A change without a record is an error.

A draft spec that still reads a column from the owner it had before the next version claimed it (S6's
`household.head` from M4 today) warns in `--next` mode and names the follow-up; a frozen reader errs as always.

**One draft per system id.** `next/` holds at most one next version per id, and only a frozen spec takes one, so a
second slice of the same system (SPR-016's mortality, `specs/demography/mortality_f1.toml`) cannot be linted here
until the first next version has superseded the file; it folds into the then-frozen version's next draft.

A `--overlay FILE` given with `--next` may add `[verbs]` and may **replace** the `[systems]` entry
of a system that has a next-version spec (never another), keeping `fidelity_levels`, every
existing parameter's bounds and every existing default; the lint checks those too. A violation of
any row above is an error. The positive controls are rendered by `bin/check_all.sh` from the next spec itself with `sed` (`tests/lint_fixtures/README.md`, `odd_next`), so they cannot drift from it.

Run: `python -I specs/systems/lint_odd.py --next specs/systems/next --overlay specs/systems/next/registry_delta.toml`

## Lifecycle of a next version

1. The drafter writes `next/<id>.odd.toml` (the frozen file plus the new slice) and, if the
   Registry must change, `next/registry_delta.toml`.
2. Both lint green here; the reference and the self-tests of the new slice are added to
   `bin/check_all.sh` with planted controls.
3. The director approves the sprint plan that builds the new slice (GOV-005 §2).
4. In that sprint's PR the Architect moves the file over `specs/systems/<id>.odd.toml`, merges
   the delta into `specs/manifest/registry_v0.toml`, regenerates the skeleton, sets
   `status = "frozen"` and empties this directory. `bin/gen_skeletons.py --check` and the ordinary
   lint take over.

`bin/gen_skeletons.py` and `gen_manifest.py` read `specs/systems/*.odd.toml` only: a next version
never changes a committed skeleton or the index until step 4.

## Current contents

| File | What |
|---|---|
| `M2.demography.odd.toml` | 0.3.0 draft: M2 F1 births, partnering, households, kinship (SPR-015) on the frozen 0.2.0 rung-0 slice |
| `registry_delta.toml` | `[systems."M2.demography"]` with the F1 defaults (`tfr`, `elite_fertility_mult`); verbs `BIRTH`, `MARRIAGE` |
