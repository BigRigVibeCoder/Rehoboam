# SPR-014 independent code review (head `5ad5288`, branch `backup/SPR-014-pre-audit`)

The review ran on a model different from the developer's, for the Architect's audit VER-SPR-014. Nothing in the repository was changed. The Architect recorded this from the reviewer's returned text. The rulings are in `spr014_rulings.md` beside this file; the developer's fix batch for them is `1d351f5` on `backup/SPR-014-pre-audit` (one commit per fix, each with its test).

## Verdict
**PASS AFTER FIXES. BLK 0, MAJ 0, MIN 7, NIT 6.**

No ordering that reaches state, a hash, the journal or an emitted event depends on thread count or `HashMap`/`HashSet` iteration. No system can write a column it does not own. No divergence from EVO-001 or `05.1` §2 on the inputs tried. Two defects have a repro (K1, K2); K7 is a test gap; K3, K4, K6 are contract-versus-enforcement gaps; K5 is A-60's open item with new measurements.

**Not run:**
- `bin/check_all.sh`, the golden ladders, the nightly tier and a release `rh-cli`;
- `rh-worldgen --test death_path`, because the disk filled while another reviewer was building;
- an ablation to attribute K5's cost.

## Findings

| ID | Sev | Where | Defect | Failure scenario | Fix |
|---|---|---|---|---|---|
| K1 | MIN | `rh-core/src/system.rs:905`; `rh-snapshot/src/state.rs:583` | `submit` validates payload and event slots only when `decl.payload` is `Some`. An untyped payload accepts `Arg::Event`, and the keyframe decoder reads it back as `Arg::Int` | **Verified:** with `Note` untyped, `write_keyframe` fails `ReadBack("it decodes to another state or header")` at the first 10-year keyframe | Refuse `Arg::Event` when `decl.payload` is `None` |
| K2 | MIN | `bin/lint_forbidden.py:236`, `:312` | The A-50 `env!` rule is bypassed by renaming the macro on import | **Verified with rustc 1.97.0:** `use core::env as e; e!("HOME")` and `use std::option_env as oe; oe!("HOME")` compile and scan clean | Flag `use` of `core::env`, `core::option_env`, `std::option_env`, and renamed `env`/`option_env` paths; planted controls |
| K3 | MIN | `rh-core/src/command.rs:181`, `register.rs:762`; `engine.rs` `merge` | The engine does not enforce R9 (an `outcome = true` command never waits across an `Inputs` phase); only the lint does | A sender timer at a non-day-start submits `ScheduleDeath`; the drain runs next day start; an input scoping the victim cannot re-sample it (ADR-0025 item 3) | At merge, require the drain's pulse fires at `t` in a later-or-equal phase, else `SimError::Timer`; a stub test |
| K4 | MIN | `system.rs:837` (`take_commands` is `mem::take`), `:577` | "Settles every command" is not machine-checked | A future drain skips a kind with `continue`; the command vanishes with no counter | Taken = applied + dropped as a core invariant, or settle-required handles |
| K5 | MIN | `engine.rs:881` (`compact`), `:1079` (`ledger.check`); `ledger.rs:183–250` | Per-instant work is O(rows): `Ledger::check` scans the whole `life_course` log every instant, release too; it grows without bound (ADR-0007) | **Measured** (release, M2 only, 3 years): 16 / 104 / 1,109 µs per instant at 100 / 1,000 / 10,000 rows; about 59 allocations per idle solo tick | A-60 as written; add the `life_course` term to the A-60 record |
| K6 | MIN | `engine.rs:1332–1335`, `access.rs:199`, `demography.rs:431` | The A-60 "light job" narrowing rests on a code-declared `drain_only` with no tie to the spec | SPR-015 adds steps to `Day/Vital` while `drain_only` stays; value checks are silently skipped | Lint or test that a `drain_only` binding's spec entry has one step |
| K7 | MIN | `rh-worldgen/tests/death_path.rs` | V-INV-03 on the real M2 with two senders runs at one thread only | A merge-order defect visible only at >1 thread passes | Loop 1, 2, 4, N threads; equal state and `dropped` |
| K8 | NIT | `rh-events/src/lib.rs:51`, `:57` | `DraftRef::from_index` is `pub` (`#[doc(hidden)]`) | A forged reference to the step's first event | Reword `05.1` or seal the constructor |
| K9 | NIT | `system.rs:844`, `:852` | Owners may drop with any `DropReason` | A misleading diagnostic | A two-variant owner reason type |
| K10 | NIT | `register.rs:757`; `rh-registry/src/ownership.rs:433` | The Rust loader fails open on a mistyped `key` and a malformed `payload` | `key = "livng"` silently loses the dead-person drop | Refuse both |
| K11 | NIT | `ownership.rs:398` | A non-zero integer neutral on a spec-less column is typed as raw bits | A `q16_16` neutral of 1 served as 1/65536 | Leave it untyped |
| K12 | NIT | `rh-sys-demography/src/demography.rs:383` | `Marry{a,a}`, a stale former spouse, `death_cause = none` admitted | Not reachable in P1a | SPR-015 N7 |
| K13 | NIT | SPR-014 plan BLUF; `05.1` changelog order; process | Stale BLUF; v1.17 out of order; the nightly ran at an older head | — | Text fixes; CI on the final head is the record |

## Checked and found sound (summary)
- **Delivery:** the inbox is taken at level start; the timer pass runs before the binding pass.
- **Thread independence:** an identical `EngineState` and `dropped` at 1, 2, 4 and 8 threads, 3 sizes × 6 seeds.
- **Restore:** equivalence through real keyframes at three splits.
- **Hash neutrality:** diagnostics are outside `snapshot()`.
- **RNG keys:** unchanged.
- **The dead-person drop:** correct.
- **Absent owners, row insert, single-writer ownership:** correct.
- **Gates:** in one part each; the inventory passes; every `mutant-*` feature is listed; `[C3]`/`[perf-baseline]` tagging is correct.

**Tests the reviewer ran:**
- `rh-core` agenda_events: 37 passed.
- `rh-core`, `rh-registry`, `rh-contracts`, `rh-events`: 198 passed.
- `rh-snapshot`: 27 passed.
- fmt and clippy: clean.
- `lint_arch` and `lint_forbidden`: 0 violations.
- `gen_skeletons`: up to date.
- `lint_odd`: PASS.
