---
id: VER-SPR-011-TEST
title: "Determinism and contract tester evidence for SPR-011"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing, determinism]
related: [SPR-011, VER-SPR-010-TEST, VER-SPR-009-TEST, VER-SPR-008-TEST, AGT-003-DET, AGT-003-IF, ADR-0025, ADR-0026]
created: 2026-10-17
updated: 2026-10-17
version: 1.0.0
---

> **BLUF (final head `e173783`):** T-00, T-01, T-02, T-03 and T-04 meet their acceptance criteria. `bin/check_all.sh` is **315 of 315** on a clean, git-initialised archive with a real `origin/main`, and pytest passes (619) under CI's three event shapes. I found six defects; **all six are fixed and re-verified**: DEF-1 to DEF-4 (and the extension to DEF-1) on `a0316b5`, DEF-5 and DEF-6 on `e173783`. No defect is open. I could not break `Prob32`, the exact decimal boundary, the epoch-keyed draws or the A-37 guard. I reproduced the smoke bisect independently: the first divergence is rung 89 (0007-05, tick 3896639), after the first input at tick 3822127, and rungs 1 to 88 are byte-identical to `main`; the monthly ladder at `e173783` is byte-identical to the one at `a0316b5`. The mutation matrix has 64 mutants; 5 survived my first pass (2 closed by tests I added, 3 remain as test gaps, GAP-2 and GAP-3), and 2 more are equivalent mutants. Nine contract gaps are open or noted (§4), none a defect.

Tested heads on `sprint/SPR-011-engine-prereqs`: phase 2a `1d72e486` (T-00, T-01, T-03, T-04 only), phase 2b `a0316b5b2e57cc7d11de841676815c58f95ecdb2` (everything), and a delta pass on the final head `e173783a73e807d458fd26fff2613aea329adf29` (§2.8), which the Developer pushed while I was writing this report (DEF-5, DEF-6, a rename rule, a sealed `EntityKey`, a message fix). My defects were sent as found and the Developer fixed them between the heads. Every result below is from `a0316b5` unless it says `e173783`; §2.8 lists what I re-ran on `e173783`. All probing ran on `git archive` copies of the SHA, each with its own `CARGO_TARGET_DIR`; the repository tree was only read, and this file is the only thing committed. The probe plan was written from the contracts before the branch was opened (SPR-011 v1.0.0 with rulings 1 to 19 and the launch additions A-37, A-44 and the 20-minute PR timeout; `05.1` §2, §6, §9; `05.2` §1, §4, §7; `05.5` §3.2; ADR-0025, ADR-0026; `docs/13` §3; `docs/09` V-INSP-05, V-DET-01/02/05/14/19, V-INV-01/04, V-WC-04). The Architect's later items (A-1 to A-5, G-1 to G-3, and the DEF-3 and A-2 rulings) were applied when they arrived. Tools: valgrind 3.22, cargo-nextest 0.9.148, cargo-deny 0.20, iai-callgrind-runner 0.16.1, rustc 1.97.0 (pinned), a 4-core x86-64 host. My probes live in the scratchpad (`PROBE_PLAN_SPR-011.md`, `RESULTS_2a.md`, and the `tester_*.rs` files and Python scripts under `work/probes`); none is committed. The Tester runs on a model different from the Developer's (GOV-005 §4.2); which one is not recorded here by instruction.

## 1. Verdicts

| Task | Verdict | Basis |
|---|---|---|
| T-00 engine purity (V-INSP-05), A-44, A-1, A-4, A-5 | **PASS** | 24 deny-list entries each fail as direct dependencies; renamed, `cfg`, target-triple, build, inherited, optional, depth-3 transitive all fail; dev-dependencies pass (§2.1). DEF-1, DEF-2 fixed on `a0316b5`, DEF-5 on `e173783` |
| T-01 `Prob32` | **PASS** | Own u128 reference over edges and random bits; exact tie conversions; one draw per call; 23 of 23 compile-fail controls (§2.2) |
| T-02 epoch-keyed draws (V-DET-14), A-37 | **PASS** | 44,512 draws against my own `rh_ids` oracle; 8 guard cases; smoke bisect reproduced (§2.3, §2.7) |
| T-03 ledger and life-course log | **PASS** | Ledger invariants over 10 sizes, 3 rates, 2 seeds, 40 months; keyframe round trips at every cut; DEF-3, A-2 fixed on `a0316b5`, DEF-6 on `e173783` (§2.4, §2.8) |
| T-04 authored-number boundary | **PASS** | 289,116 decimal checks and 5,078 end-to-end `e0` cases against Python `decimal`, 0 mismatches; 20,000 fuzzed numeric fields, 0 panics (§2.5) |
| `bin/check_all.sh` whole | **PASS** | 299 of 299 on `a0316b5`, **315 of 315 on `e173783`** (§2.6, §2.8) |
| pytest under CI's variables | **PASS** | 574 passed (`a0316b5`) and 619 passed (`e173783`) under `pull_request`, `push` and `workflow_dispatch` shapes, and in the hostile-environment run inside `check_all` |
| Smoke baseline (A-29, `[C3]`) | **PASS** | Only `97f1aa5` touches it; my own bisect and monthly-ladder diff agree with the Developer's (§2.7) |
| V-DET-19 / V-PERF-11 | **PASS** | DEF-4 fixed: the allocation gate passes inside `check_all` with `allocations_v1.txt` unchanged (the Developer reports 819,401 / 856 / 281 at 1 thread and 828,161 / 856 / 305 at 4) |

## 2. Results

### 2.1 T-00: engine purity, A-44, A-1, A-4, A-5

- **Planted dependencies** (44 mini-workspaces, offline): each of the 24 deny-listed crates, plus `tts` and `hyper` (A-5), as a direct normal dependency of an engine crate fails with the path. Also failing: a renamed package (`foo = { package = "reqwest" }`), `[target.'cfg(unix)'.dependencies]`, a target-triple table, a build-dependency, a workspace-inherited one, an optional one behind a feature, depth 3 (`a -> b -> c -> tokenizers`), a build-dependency of a third-party dependency. Passing as expected: a dev-dependency, a dev-dependency of a dependency, a non-engine tool crate.
- **Engine set:** `rh-sys-*`, `rh-contract-*` and plane A are engine by rule. A tool-plane crate named `rh-foo` that is not on the list is not checked (GAP-1).
- **Defined once:** `test_the_deny_list_is_defined_once` fails when I plant `"tokenizers" "ureq"` in a tracked `.md`, `.json`, `.txt` or `.cfg` file, and when I plant only `"png" "gif" "warp" "image"` in a tracked file (G-3 fixed). It needs a git tree and fails closed without one.
- **A-44 (`lint_forbidden`):** About 70 probe files under `--scan`. Flagged on `a0316b5`: `std::process` in every import form (alias, `{self as p}`, grouped, glob parent, `extern crate std as s`, `tokio::process`, spaces and newlines in the path, macro bodies), `exit`, `abort` and `id` too (the whole module is banned by ruling); `std::fs` in every form including `use std::{fs as f, io}` and `fs::{self as g}`; file timestamps (`.modified()`, UFCS `Metadata::accessed(m)`, `<Metadata>::..`, `MetadataExt::ctime`, `.mtime()`, `.st_mtime()`, Windows accessors); `std::env::var`, `vars`, `args` and the `_os` forms in every import form. Not flagged, as intended: comments and string literals. `#[cfg(test)]` code inside `src/` is flagged (only `tests/` is exempt). Not flagged and should have been at this head: DEF-5 (fixed on `e173783`, §2.8).
- **A-1 (clippy):** a scratch module in `rh-core` using `use std::process as p; type C = p::Command`, `Metadata::modified`, `Metadata::accessed`, `MetadataExt::mtime` and `std::env::var` fails `cargo clippy` with five `disallowed_*` errors, one per use.
- **Plane-A closure (T-04):** `rh-core`, `rh-sys-*`, a plane-A crate through a tool crate, and a mis-declared plane all fail when they depend on `rh-manifest`; `rh-cli -> rh-manifest` and a dev-dependency pass.
- **CI workflow:** the `check-all` job's timeout is 20 minutes, the matrix is exactly the declared parts, and `python/tests/test_ci_workflow.py` fails on each of four mutations (§5).

### 2.2 T-01: `Prob32` and keyed uniform draws

- `draw_hits` equals my reference (`d < bits << 32` as u128; `bits = 2^32` always hits) over 8 edge probabilities, the draws at and around each threshold, and 2,000 random probabilities; block counting confirms P(hit) = bits / 2^32 exactly over 3,000 random probabilities and blocks.
- Construction: `from_bits` refuses `2^32 + 1` and `u64::MAX`, accepts 0 and `2^32`. `from_q16_16` is `bits << 16` in `[0, 1]` and refused outside it. `to_q16_16` rounds half to even: every floor value 0 to 65,536 with remainders 0, 1, 0x7FFF, 0x8000, 0x8001, 0xFFFF, plus 200,000 random values, against a reference. `from_ratio` against a u128 reference over 300,000 random pairs and the exact ties `(2k+1) / 2^33`.
- One draw per call: a counting source sees exactly one draw per `chance` for p = 0, 1/2, 1, one below 1 and `ONE`, and for `Q16_16`, so two streams that differ only in p stay aligned (common random numbers).
- Compile-fail controls (my own throwaway crate, `--all-features`): `Prob32 == Q16_16`, `+` with `Q16_16`, `+ - * -` on `Prob32` itself, `From`/`Into`/`TryFrom` with `Q16_16`, `f32`, `f64`, `u64`; `as f64`; comparison with `Q8_8` and `Money`; the tuple field; the tuple constructor; `draw_hits`/`chance` with `f64` or `u32`; a foreign `impl Probability` (sealed); `let p: Prob32 = 0.5`. 23 of 23 are rejected. `Prob32::default()` and the baseline compile, as they should.
- `gen_fixed_vectors.py --check` is current (16,565 vectors in 2a).

### 2.3 T-02: epoch-keyed draws (V-DET-14, A-29) and A-37

My system draws, once per day, one `rng` value per person and per region and writes it to an owned column; the oracle is `SystemKey::new(seed, salt, system_id, epoch).stream_key(purpose, entity).draw(t, 0)` with the epoch counted by me from the inputs.

| Probe | Result |
|---|---|
| 60 random cases: 1 to 150 persons, 1 to 30 regions, 0 to 3 inputs each at one of six instants, scopes of 0 to 11 entities (persons, regions, sometimes `WORLD`), overlapping and duplicate | 44,512 draws equal to the oracle; 2,817 draws that must differ from the no-input run, all differ; every other draw equals the no-input run |
| Entity at epoch 0 | equals the no-input run and the oracle at epoch 0 (the P0 key) |
| Input exactly at a pulse instant | that day's draw already differs (Inputs precede systems) |
| Input one minute after the pulse | that day unchanged, the next day differs |
| Two inputs on one entity at one instant | epoch 2, equal to the oracle at epoch 2 |
| Threads 1, 2, 4, 8 | identical results |
| Sensitivity | `epoch always 0`, `epoch off by one` and `input does not bump epoch` mutants are all caught (§5) |

A-37, one system with two bindings (Year and Month, which fire together at instant 0) and a timer pass:

| Case | Result |
|---|---|
| `stream` then `stream` for one (purpose, entity) | refused, `StreamReuse` naming system, purpose, entity |
| `rng` then `stream`; `stream` then `rng` | refused |
| `rng` then `rng` (allowed by design, per the Architect) | allowed |
| a different entity | allowed |
| `PersonId(77)` then `RegionId(77)` (same raw key) | refused: the pair is the raw key (GAP-8) |
| timer pass draws a stream, the same day's step draws the same pair | refused |
| two different systems, same pair, same instant; same pair on 5 successive days | allowed (the guard is per system and resets each instant) |

### 2.4 T-03: ledger and life-course log

- **Property run:** worlds of 0, 1, 2, 3, 64, 1,023, 1,024, 1,025, 2,049 and 3,000 living people, 7 closed historical rows where the world is large enough, death rates of 10%, 50% and 100% a month, 2 seeds, 40 months. After every instant: all `person.*` columns as long as `person.id`; `person.id` strictly ascending as `u64`; no duplicate id; the set of open life-course entries equals the ledger; no dead row; every `life_course.*` column as long as `life_course.person_id`; the log never shrinks. The rate-100% worlds are empty after month 0 and keep running.
- **Phases:** a death applied in any phase from Inputs to Culture is gone before Bookkeeping hashes. A death applied in `Bookkeeping` (after the compaction) now fails with a ledger error that names the system and the person (G-1 fixed; before, it was an anonymous `rh-core` invariant).
- **Keyframes (V-DET-05):** worlds of 0, 1, 2, 9, 64 and 1,025 people over 300 months, a keyframe cut at every 7th (or 41st) month, loaded and run to the end: all later monthly `state_hash` values equal the continuous run; the hash right after the load equals the hash at the cut; a second-generation keyframe (cut, load, advance 4 months, write, load) is also equal.
- **Misbehaving owners (V-INV-04), each as a system that owns the ledger columns:**

| Behaviour | Result at `a0316b5` |
|---|---|
| removes a ledger row (pop) / appends to `person.id` only | refused, naming the system and the row counts |
| rewrites an id keeping order (`id[1] += 1`); swaps two ids; duplicates an id; remove-first and push-max (same row count) | refused: "an existing row's person.id never changes" (A-2) |
| closes an entry but leaves the person alive | `Invariant life-course-one-open-entry` |
| rewrites `exit_tick` on a row it leaves open, every month | refused (DEF-3 fixed) |
| closes with only `exit_kind`, or `exit_kind` and `exit_tick` | accepted (as ruled) |
| rewrites an exit field of a closed row | refused |
| appends a row open with `exit_tick = 5` | refused, naming the open value |
| appends a row closed with arbitrary values | accepted (as ruled) |
| appends a second open entry for a living person | `Invariant life-course-one-open-entry` |
| **appends a closed entry for a person who is a ledger row** | **accepted at `a0316b5` (DEF-6); refused on `e173783`** |
| genesis tables with swapped or duplicated ids | `Invariant ledger-sorted` at the first Bookkeeping |
| `person.alive` of 2 or -1 at genesis | stays; only 0 leaves |

- **Threads (V-DET-01/02, A-3):** the Developer's stub world with deaths is identical at 1, 2, 4 and 8 threads and across repeats; a mutant that makes the second job of a multi-job level see a different instant is caught (§5, T2).

### 2.5 T-04: the authored-number boundary

- **Decimal reference:** a Rust harness calls `Decimal::parse_{yaml,toml,python}_float(..).to_fixed(f)` for f = 8, 16 and 32 on 96,372 literals (random 0 to 12 integer digits and 0 to 40 fraction digits, signs, `e` exponents, underscores; exact ties `(2k+1) / 2^(f+1)` and ties ± 1e-40 in 33-digit strings; range boundaries) and compares the bits with Python `decimal` under `ROUND_HALF_EVEN`: 289,116 comparisons, 0 mismatches. A plain `float` implementation of the same rule differs from the reference on 8,566 of the 96,369 comparable cases, so the corpus detects a rounding through `f64`.
- **End to end:** 5,078 `e0` literals through `validate_str` and `compile`, once as a manifest value and once as a Registry default, with the bounds widened to ±1e9 so the type's own range decides: every in-range literal gives `round_half_even(v * 65536)`; 20 out-of-range literals are refused with the path and the type (`32768`, `32767.99999237060546875`, a tie that rounds up out of range, `-32769`, `40000.0`); `-32768.000007629394531250` (a tie to even) fits. G-2: exponents up to ±1,048,576 are accepted, beyond that refused by name ("has an exponent beyond ±1048576"), and Registry bounds are compared on the exact decimal ("compared exactly").
- **Fuzz:** 20,000 random strings from a hostile alphabet (`inf`, `.nan`, `0x`, unicode digits, NUL, braces, quotes, base-60 `1:30`, comments) as `e0`, `brass_beta` or `brass_alpha`: 159 compile, 19,841 are refused, no panic.
- Plane-A crates do not depend on `rh-manifest` (§2.1); V-WC-04 parity runs inside `check_all` and passes.

### 2.6 The gate

`bin/check_all.sh` on a copy that is a git repository (the baseline-integrity gate needs history: HEAD `a0316b5`, `origin/main` `132e17d`) with a fresh target directory and `PYTHON` pointing at the repository's venv: **299 of 299, `Verdict: PASS (299 checks)`**, 1,100 s of check time on a 4-core host with other builds running (the Developer reports CI run 247: core 5:36, golden 4:41). At `1d72e48` the same run had failed at the V-PERF-11 gate (DEF-4); with a 3-line early return in `ledger_guards`, applied in my copy only, it was 286 of 286 then, and `c5d28bb` is that fix. Valgrind (`--error-exitcode`, full leak check) on `rh-cli run smoke --years 3` in the release build is clean. The two `[perf-baseline]` commits are consistent: `0589a38` adds the draw-cost bench and `7892c24` re-records only the draw cases (-393 instructions each) and moves two keyframe cases inside the 10% band; benches 7 are unchanged.

### 2.7 The smoke bisect (A-29)

Baseline: `rh-cli` built from `main` `ae70060`. Candidate: `a0316b5`. `rh-cli bisect --years 1000` reports `verdict: divergence`, year 8, month 89 (0007-05, tick 3896639), table `chronicle.census`, row 0, column `chronicle.census`, 56 → 55; the differing tables are `chronicle.census`, `estate.*`, `market.audits`, `person.alive`, `person.pending` and four `rh-core` tables. I confirmed it a second way: monthly ladders of 10 years from both binaries differ first at rung 89, the header and rungs 1 to 88 are byte-identical. The first synthetic input is year 7 plus 99 days 06:07 on the 365-day calendar, tick 3822127 (`synthetic_inputs`), so the divergence is at or after the first input and never before. It appears a month later than the input because a re-sampled timer only matters when it fires. `git log` shows `97f1aa5` ([C3]) as the only commit that touches `worlds/smoke/baselines`; the 1,000-year golden gates (1 thread, N threads, baseline equality) pass inside `check_all`.

### 2.8 Delta pass on `e173783`

The Developer pushed `b638e1c` (DEF-6), `4d2280d` (DEF-5, MAJ-1, MIN-2, nits) and `e173783` (a doctest count) after `a0316b5`. I re-ran on a fresh archive of `e173783`:

- **`bin/check_all.sh` whole:** 315 of 315, `Verdict: PASS (315 checks)`.
- **pytest** under the three CI shapes: 619 passed each.
- **DEF-5:** `std::env::{current_dir, current_exe, temp_dir, home_dir, set_current_dir, split_paths, join_paths}`, `env::current_dir()` after `use std::env;`, and the earlier `var`/`vars`/`args` forms are flagged; `std::env::consts::{OS, ARCH}` and `env!`/`option_env!` are clean, as the Developer's docstring says.
- **MIN-2 (std rename):** `use std as s;`, `use ::std as s;`, `use std::{self as s};` and `extern crate std as s;` are each refused by rule `std-rename`; renamed `s::fs::read`, `s::process::id`, `std::os::unix::fs::symlink` and raw identifiers (`r#fs`, `r#process`) are refused (MAJ-1).
- **All earlier A-44 probes** (about 70 files) behave as on `a0316b5`, plus `set_var` now flagged.
- **DEF-6:** my `M2.closedliving` step is refused by system and person; `M2.appendclosed` (a back-filled ancestor absent from the ledger), `M2.closepartial` and `M2.closekind` are still accepted; the other 12 misbehaviours keep their results.
- **T-01, T-02, A-37, T-03, T-04 probes:** unchanged results (same counts as §2.2 to §2.5); the `StreamReuse` message now names what was drawn.
- **Smoke:** the 10-year monthly ladder from `e173783` is byte-identical to the one from `a0316b5`; valgrind on a 2-year release run is clean.

## 3. Defects

| # | Severity | Where | Status at `a0316b5` |
|---|---|---|---|
| DEF-1 | medium | `lint_forbidden` rules `process` and `file-io` missed a renamed module import (`use std::process as p`, `use std::{fs as f, io}`, `fs::{self as g}`) | **Fixed** (`f820447`). Re-probed in 14 import forms |
| DEF-2 | medium | rule `file-clock` missed UFCS `Metadata::modified(&m)` and `MetadataExt::{mtime,ctime,atime}`; in `rh-registry` and `rh-snapshot` nothing else catches them | **Fixed** (`f820447`). Re-probed |
| DEF-3 | low-medium | the life-course guard let `exit_tick` and `death_cause` change on every step while an entry stayed open | **Fixed** (`5edbc5e`), including the Architect's refinement: re-probed with the terse closings, appended-open and appended-closed rows |
| DEF-4 | medium | V-PERF-11 over budget: +10,320 allocations, +13 at the busiest tick, from a `Vec` that `ledger_guards` built on every level even without a ledger | **Fixed** (`c5d28bb`). The gate passes with the baseline file unchanged |
| DEF-5 | low | the `env` rule matched `var`, `vars`, `args` and their `_os` forms only; the fully qualified `std::env::{current_dir, current_exe, temp_dir, home_dir, set_current_dir, split_paths, join_paths}` passed | **Fixed** (`4d2280d`). Re-probed on `e173783`: all flagged; `std::env::consts` and `env!`/`option_env!` stay clean, as stated |
| DEF-6 | low-medium | a step could append a *closed* life-course entry for a person who is a ledger row and has an open entry | **Fixed** (`b638e1c`). Re-probed on `e173783`: refused by system name; a back-filled ancestor absent from the ledger still passes |

**DEF-5 reproduction (at `a0316b5`):** `fn f() { let _ = std::env::current_dir(); }` with `python -I bin/lint_forbidden.py --scan f.rs` is clean (the same call after `use std::env;` is flagged by the use tree). These read the working directory, the install path, `$TMPDIR` and `$HOME`: process state outside the history tuple. Expected: a violation of rule `env`. Fix: flag any `std::env::<name>` other than `consts`.

**DEF-6 reproduction (at `a0316b5`):** `crates/rh-core/tests/tester_t03.rs` in my scratchpad, system id `M2.closedliving`: a step appends one row to every `life_course.*` column with `person_id = ids[0]` (alive, with its open entry), `exit_kind = 1`, `exit_tick = -99`, `death_cause = 3`. Six monthly instants run with no error. Expected (T-03 acceptance, V-INV-01 subset): a refusal, because a closed entry must be absent from the ledger. `Ledger::check` walks only open entries; the guard allows any closed append. After the step the log says the person died while the ledger and an open entry say they live. Fix: in `check`, build the set of `person_id`s with a closed entry and refuse one that is a ledger row (a back-filled dead ancestor is not).

## 4. Contract gaps

| # | Gap | Recommendation | Confidence |
|---|---|---|---|
| GAP-1 | The engine set for V-INSP-05 is a hand list plus `rh-sys-*`, `rh-contract-*` and plane A. `rh-bench` and `rh-golden` (tool plane) are on neither list, so a media or LLM crate in them is not checked, and a new tool crate is skipped silently | Add an explicit non-engine list and a test that every workspace member is on exactly one list | 70% |
| GAP-2 | Removing `hyper` or `tts` (A-5) from `ENGINE_DENY_LIST` is detected by no test; the "named examples" test holds only the nine names of `docs/13` §3 | Assert the whole constant against a committed golden list, or each A-5 name | 85% |
| GAP-3 | Removing the `\bprocess::\w` alternative from the `process` rule is detected by no test, though `use std::*; process::Command::new(..)` needs it | Add that case as a planted fixture | 80% |
| GAP-4 | The new rules are textual and conservative: an own module `process::f()`, an own accessor `.created()` or `.mtime()`, an own module `fs::foo` are all flagged | Accept (the `LINT-ALLOW` escape is counted and printed; 4 escapes today) and say so in `tests/lint_fixtures/README.md` | 75% |
| GAP-5 | `env!`, `option_env!` and `core::env!` read the build environment into the binary and are not covered (the Developer's docstring states they are out of scope); `engine_build` is the only thing that would tell two such binaries apart | Decide whether `engine_build` hashes the build environment, or ban the macros in plane A | 55% |
| GAP-6 | The `rules process/file-io/env` apply to `src/` and `build.rs` only. A plane-A `tests/` file may spawn a process or read the environment; the 4 escapes show this is used | Fine for the engine; record that `tests/` of plane-A crates are trusted, not scanned | 70% |
| GAP-7 | `PersonId(77)` and `RegionId(77)` under one purpose share one keyed stream and one guard entry (the pair is the raw key). The text says the purpose declares its entity kind, but registration does not enforce that a purpose is used with one kind | Check at the draw that the entity's kind equals the kind the spec declares for the purpose (a later sprint), or document it | 55% |
| GAP-8 | The `StreamReuse` message said "asked for a stream" when the refused call was `rng` after `stream` (older than SPR-011) | **Resolved in `4d2280d`**: it now reads "drew again under purpose .. which it already took as a stream" | 60% |
| GAP-9 | The exponent-bound error for a manifest value is printed twice at the same path (validation and compile each report it) | Report once | 50% |

## 5. Mutation matrix

64 mutants (29 Rust, 35 Python), each run against the Developer's tests together with mine. "Caught by dev" means a test in the repository fails; "mine" means only a probe of mine fails.

### 5.1 Rust (29)

| Mutant | Result |
|---|---|
| P1 `hits`: `<` becomes `<=` | caught (5: golden vectors, dev tests, mine) |
| P2 `hits`: `ONE` not special | caught (7) |
| P3 `to_q16_16` rounds half up | caught (3) |
| P3b `to_q16_16` rounds half down | caught (3) |
| P4 `from_bits` accepts `2^32 + 1` | caught (3) |
| P5 `chance` skips the draw at p = 0 or 1 | caught (5) |
| E1 epoch always 0 | caught (4, both) |
| E2 epoch off by one | caught (9) |
| E3 an input does not bump the epoch | caught (9) |
| A1 A-37 set cleared per job | caught (4) |
| A2 A-37 set cleared at each level | caught (4) |
| L1 compaction skipped | caught (6) |
| L2 compaction only on `person.id` and `person.alive` | caught (6) |
| L3 `ledger-sorted` check removed | **survived the dev tests**; caught by my genesis test |
| L4 closed-entry exit rewrite allowed | caught (1, dev) |
| L5 DEF-3 `closes` always true | caught (1, dev) |
| L6 appended-open value unchecked | caught (1, dev) |
| L7 id fixity unchecked (A-2) | caught (1, dev) |
| L8 `alive` test is `== 1` instead of `!= 0` | **survived the dev tests**; caught by my `alive` of 2 and -1 test |
| L9 ledger never detected | caught (12) |
| L10 row-count guard removed | caught (1, dev) |
| T2 a multi-job level's second job sees another instant under threads | caught (3, dev, A-3 works) |
| X6 DEF-6 guard disabled (`e173783`) | caught (1, dev) |
| X6b DEF-6 guard ignores the closed flag (`e173783`) | equivalent: an open append for a ledger person is then refused by the guard instead of by the `life-course-one-open-entry` invariant; both refuse |
| D1 decimal tie rounds half up | caught (3) |
| D2 decimal tie rounds half down | caught (3) |
| D3 decimal tie ignores later digits | caught (3) |
| D4 exponent bound off by one | caught (1) |
| D5 `Q16_16` range check dropped | caught (1) |

### 5.2 Python lints, CI workflow and gate inventory (35)

Run on the git-initialised copy, each mutant against `pytest` for the relevant file and the planted fixtures. On `a0316b5`: 30 mutants, 27 caught. On `e173783` the rules changed (`env` regex, `std-rename`), so I re-ran with new mutants for them: 32 run, 29 caught, the same three survivors; the old `fs::` mutant no longer applies (MAJ-1 changed the pattern) and is caught by the new `fs_os_symlink` and `fs_renamed_std` fixtures.

| Survivor | What it means |
|---|---|
| L3c `hyper` removed from the deny-list | GAP-2 |
| L3d `tts` removed from the deny-list | GAP-2 |
| F1 `\bprocess::\w` removed from the `process` rule | GAP-3 |

Also caught on `e173783`: the `env` regex without its `consts` exemption, without its `env::<item>` alternative or without its `std::env` alternative; the `std-rename` rule without its `extern crate` or its `use std as x` branch. Caught: dev edges counted; build edges ignored; `reqwest` and `tokenizers` off the list; `rh-cli` not an engine crate; plane A not engine; transitive walk removed; unresolved dependency ignored; missing lock passes; each of the three `BANNED_MODULES` entries removed; `env` regex without `args` or without `var_os`; `file-clock` without `accessed`, UFCS, `MetadataExt`, `[amc]time` or the Windows accessors; `FILE_IO_CRATES` widened to `rh-core`; `file-io` without `OpenOptions` or without `fs::`; CI timeout 30; matrix cut to `[core]`; step without `--part`; both jobs `--part core`; the `part golden` marker removed from `check_all.sh`. In the first pass at `1d72e48` the equivalent mutant "drop the `rh-sys-` prefix from the engine set" (plane A already covers it) and the survivor "drop `fs::\w`" were found; the latter is now caught.

## 6. What was not run

- The nightly tier: `bin/golden_nightly.sh` (monthly ladders at 1, 2, 4 and N threads × 3, V-DET-11 × 64), the `--run-ignored only` tiers (sampler statistics, the full keyframe bit sweep, `rh-ids` stream independence at 10^6 draws). Their PR-tier parts ran inside `check_all`.
- The cross-machine check (it needs the director's machines).
- GitHub's own CI: I read the Developer's report of run 247 and did not re-fetch it. I did not run any gate on a non-Linux host or a second architecture.
- Rust mutants against `rh-snapshot` and the registry beyond the ledger keyframe tests, and against the hash code; SPR-011 changes none of them.
- `StepCtx::chance` was not probed through an engine run of my own; `Prob32` and `draw_hits`/`chance` were, and the P5 mutant on `chance` is caught.
- T-05 onward (SPR-012 and SPR-013).
