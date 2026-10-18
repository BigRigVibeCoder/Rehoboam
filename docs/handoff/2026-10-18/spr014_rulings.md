## Architect rulings on the SPR-014 code review (5ad5288; 0 BLK, 0 MAJ, 7 MIN, 6 NIT; 2026-10-18)

Every finding is accepted unless a row says otherwise. FIX = in SPR-014's pre-merge batch. BCK = filed at the audit.

| # | Ruling | Conf. |
|---|---|---|
| K1 | FIX. `submit` refuses any `Arg::Event` when `decl.payload` is `None` (a typed SimError naming the sender, the command kind and the field position). A test reproduces the reviewer's case: an untyped command carrying an event field is refused at submit, not at the next keyframe write | 85% |
| K2 | FIX. `lint_forbidden.py`'s A-50 rule also flags any `use` of `core::env`, `core::option_env` or `std::option_env` (and any `env`/`option_env` path in a `use … as`). Planted controls for `use core::env as e; e!(…)` and `use std::option_env as oe; oe!(…)`, each FAILing with the rule's message, in `check_all.sh` with the inventory test | 90% |
| K3 | FIX. Engine enforcement of R9: at merge, an `outcome = true` command whose `drained_by` binding does not run later in this same instant (its pulse fires at t and its phase is at or after the current one) is `SimError::Timer` naming the sender and the command. A stub test plants a non-day-start sender; `05.1` §2 rule 2 says the engine checks it | 80% |
| K4 | BCK, owned by SPR-038b T-07 (it already adds `Deferred`, the generic `Ended` and the cancel counter; R14): a drain's taken commands must all be settled, applied or dropped with a reason, checked at `finish` (taken = applied + dropped per binding) | 70% |
| K5 | No fix here. The measurements (16/104/1,109 µs per instant at 100/1,000/10,000 rows; the `life_course` scan unbounded under ADR-0007; ~59 allocations per idle solo tick) go into BCK-002 A-60 at the audit and to SPR-038a's developer now, as its stage-A before figures | 90% |
| K6 | FIX. Tie `drain_only` to the spec: a test (or a generated `DRAIN_ONLY` constant) fails when M2's `Day/Vital` entry has any step besides the drain while the binding declares `drain_only`. SPR-015, adding F1's steps there, then has to drop it on purpose | 75% |
| K7 | FIX. `rh-worldgen --test death_path` loops 1, 2, 4 and N threads and asserts equal `EngineState` and equal `dropped` | 90% |
| K8 | BCK (as planned): the sealed `DraftRef` constructor, or `05.1`'s sentence reworded ("not a draw key; the index is engine bookkeeping"), at SPR-021a | 70% |
| K9 | BCK, with K4 to SPR-038b: owners get a two-variant drop reason (`dead`, `ended`; `deferred` after R14) | 70% |
| K10 | FIX. Registration refuses a `key` other than `living`/`any`, and a malformed `payload`, by name (fail closed as `lint_odd.py:858` does) | 80% |
| K11 | FIX. A neutral for a column with no spec yet: a non-zero integer is left untyped (`None`), as the comment says | 80% |
| K12 | Record under SPR-015 N7 (`Marry{a,a}`, a stale former spouse, `death_cause = none`) | 90% |
| K13 | FIX the text: SPR-014's BLUF (the amended T-08: ladders move), `05.1`'s changelog order. CI on the final head is the record | 90% |
