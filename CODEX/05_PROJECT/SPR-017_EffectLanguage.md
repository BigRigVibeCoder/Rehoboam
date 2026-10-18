---
id: SPR-017
title: "The Registry effect language v0 in rh-registry: the evaluator, succession laws, the kinship prelude and verb effects (P1.15)"
type: how-to
status: CLOSED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-002, PRJ-002, REV-SPR-017, REV-P1a, EVO-001, REV-EVO-001, REV-SPR-019, SPR-014, SPR-015, SPR-019, SPR-020, SPR-021, SPR-024, SPR-026, ADR-0006, ADR-0010, ADR-0024, ADR-0033, ADR-0034]
created: 2026-10-17
updated: 2026-10-18
version: 1.0.0
---

> **BLUF:** Builds the Registry effect language v0 in `rh-registry`. The spec is `specs/registry/effect_language_v0.md` 0.5.0 (PR #41, `50e3c61`), approved by the director as D37 with D14, D4 and D12. The sprint delivers:
> - **the typed, total front end and evaluator,** with the 16 compile-time error classes and the two run-time invariants (`E_TRUNCATED`, `E_RNG_REUSE`);
> - **the prelude, as code (D8):**
>   - kinship walked over M2's life-course log, so it reaches through the dead: `children`, `kin`, `related`, `ancestors`, `descendants`, `descends`, `lineage`, `line`, `line_agnatic`, `up`, `common` and the agnatic forms;
>   - `holders_of` over M7's tenure log;
>   - M7's court relations;
> - **the succession protocol** (eligibility, order, the line), and **verb effects** as ordered, typed commands that the host queues;
> - **the Registry schema change,** with the Rust loader and the real checking environment;
> - **the host seam over `SystemView`** (Q9).
>
> **The central acceptance:** a Rust test reads `effect_language_golden_v0.toml` and reproduces all 558 cases bit for bit, in debug and release builds. `effect_ref.py --check` stays the oracle. A SplitMix64 differential fuzz compares Rust with `effect_ref.py` over generated programs and worlds, and twelve planted mutants each fail their named cases.
>
> **No system evaluates a program yet.** M7's succession is SPR-021 and minds' verbs are SPR-024. So no hash moves, and `registry_version` is not bumped.
>
> - **Developer:** AGT-002-CORE (`rh-registry`), with AGT-002-TOOLS for the loader, `rh-cli effect`, `lint_odd.py`, `effect_ref.py --emit` and the fuzz driver. One Developer session may hold both roles.
> - **Testers:** AGT-003-IF and AGT-003-DET. One Tester session may hold both, on a different model from the Developer's.
> - **Size and class:** about 4.5 agent-days, above GOV-005 §3's ~3 (see "Director items"). **C2** (Registry schema), confirmed in the compliance table: no `state_hash` change and no ladder header change.
> - **Status:** PLANNING. It needs the Architect's rulings in the specs table and the director's plan approval (GOV-005 §2).

# SPR-017 — The Registry effect language v0 (WP P1.15)

- **Roadmap item:** `docs/11` P1.15 · **Phase / rung:** P1a. No rung: no system evaluates a program before SPR-021.
- **Depends on:**
  - **G0 only** (PRJ-002 §2). The sprint can start before SPR-013 merges, and it runs as a parallel lane through wave 4.
  - **No acceptance criterion waits for SPR-014, SPR-015 or SPR-020.** T-07's host adapter meets SPR-014's surfaces: the keyed `SystemView` lookup that returns an absent owner's `neutral`, typed `EventRef` payloads, and A-51's move of the spec loader. "Merge route" says who wires what.
  - **Laws are tested on fixture logs:** the seven golden worlds and the fuzz's generated worlds. Real worlds get parent links only from SPR-015's births and SPR-019's back-fill. The evaluator needs no change when those arrive, because M2 0.2.2 already owns `life_course.mother`, `.father`, `.birth_tick` and `.sex`.
- **Binding contracts:**
  - `specs/registry/effect_language_v0.md` 0.5.0 §0–§13, with `effect_language_golden_v0.toml` (558 cases) and `effect_ref.py` (the oracle);
  - `05.5` §2.1 and §3.2 (authored numbers enter plane A as fixed point);
  - `05.1` §2 rules 1, 2 and 5 (v1.14: a command carries no date; event-typed slots), §4.3, and §6 items 2–5;
  - `crates/rh-core/src/fixed/` and the SPR-010 rulings;
  - the frozen M7 0.3.4, T1 0.3.3 and M2 0.2.2, as the real environment;
  - ADR-0006, ADR-0010 (amendment v1.1 item 4), ADR-0024, ADR-0025, ADR-0033, ADR-0034;
  - `docs/09` V-WC-10 (the evaluator's), V-WC-04, V-DET-19, V-INSP-03, V-PLN-01, V-GOLD-01.
- **System spec:** n/a. No system is built or changed. M7, T1 and M2 are read only by the environment builder and the lints.
- **Rulings (binding):**
  - REV-SPR-017: B1–B3, M1–M8, N1–N6, the minors of rounds 1 and 3, and r4-1 to r4-5;
  - REV-P1a NEW-1, NEW-3, NEW-4, NEW-7;
  - EVO-001 R9 (a command carries no date; an outcome command comes only from its sender's timer) and R12 (an identity read requires its owner);
  - REV-SPR-019 R3 (the differential gate against `effect_ref.py`);
  - the Architect's rulings at this plan (the specs table).
- **Director items:** D37 (approved 2026-10-17, with D14, D4 and D12). For this plan, the size: see "Director items".
- **Requirements covered:**
  - WC-1 (the effect language of the Registry);
  - EVO-1 (a data-only Registry extension needs no Rust change);
  - DET-2 (keyed RNG only);
  - DET-3 (fixed point, subset).

## Specs this sprint needs

| Spec | Exists? | Gap | Writer | Needed by |
|---|---|---|---|---|
| `effect_language_v0.md` 0.5.0, the golden file, `effect_ref.py --check` | Exists (PR #41, `50e3c61`; D37) | — | — | T-01 to T-05 |
| **Identity reads and `E_NO_NEUTRAL`** | **Conflict.** §4.5 and §10 require a `neutral` on every `[[reads]]` entry of a foreign column. EVO-001 R12 gives identity reads (`use = "identity"`) none, because registration refuses a world without their owner. M7 0.3.4's `person.alive` and `life_course.*` are identity reads, so read literally, every law is `E_NO_NEUTRAL` against the real M7. The golden slice predates R12: each of its reads carries a `neutral` | **Ruled (Architect, 2026-10-17; 8/10): accepted as proposed.** an identity read satisfies the rule, because registration refuses any world without its owner, so a neutral can never be needed. `E_NO_NEUTRAL` stays for a rate or mode read that has none. This is an errata to §4.5 and §10's text. No golden row changes, and `X.fixture`'s two `E_NO_NEUTRAL` cases still fail | Architect | T-01, T-06 |
| **Commands carry no date** | **Conflict.** §6, §7.3 and §9 give `ScheduleDeath` and `Marry` a `tick` parameter (REV-SPR-017 M6). EVO-001 R9 removed it: `05.1` §2 rule 2 says "a command carries no date", and an outcome command is submitted only by its sender's timer. M2 0.2.2's typed `[[applies]]` has no `tick`. The golden slice keeps `tick` in 14 cases, and its `ApplyInfluence.delta` is `q16_16` where T1 0.3.3 types it `q8_8` | **Ruled (Architect, 2026-10-17; 7/10): accepted as proposed.** the golden `[registry]` slice stays a fixture environment (§7.4: "one such environment"), unchanged, so D37's rows stand. The real `[commands]` follows the owners' typed `[[applies]]` exactly, with no `tick`. No real verb program lands in this sprint. A program cannot hold a timer (D11), so `MURDER_ATTEMPT`'s outcome path is decided with SPR-024, its consumer. Errata text in §6, §7.3 and §9 | Architect | T-06 |
| **`none` in an event-typed field** | **Conflict.** §6 encodes `none` as 0. `05.1` v1.14 rule 2 stores −1 for `None` in `engine.commands`, because event id 0 is a real event (REV-SPR-020 R17) | **Ruled (Architect, 2026-10-17; 8/10): accepted as proposed.** the evaluator returns event arguments typed (`Own` or `None`), never as integers. The host maps them to `EventRef`, and the merge encodes `None` as −1. `event` and `0` remain the golden file's rendering only | Architect | T-04, T-07 |
| **Enum names and defaults in the real environment** | **Silent.** §3.1 needs every enum named, with a named default. The golden slice declares them. In the real specs, `sex`, `death_cause`, `title_state` and `claim_status` are unnamed `[[owns]] values` with no default, and the Registry id lists (`title_tiers` and the rest) have no default either | **Ruled (Architect, 2026-10-17; 6/10): accepted as proposed.** a typed `[effect_language.enums]` Registry table. Each enum names its source: a Registry id list, or the spec columns whose `values` must be equal. Each names its default, equal to the golden slice's. No frozen spec changes | Architect | T-06 |
| **Declaration order of keyed Registry tables** | **Silent, and a trap.** An enum's ordinal is its declaration index (§3.1). `[vow_kinds]` and `[opinion_modifiers]` are keyed tables, and `toml::Table` sorts its keys unless `toml`'s `preserve_order` feature is on, which it is not in the workspace. Python's `tomllib` keeps source order. The laws read no keyed table, but a verb passing `opinion_modifiers.X` to a command would encode a different ordinal in Rust | **Ruled (Architect, 2026-10-17; 7/10): accepted as proposed.** the loader takes the key order from the source text, and a test checks it against `tomllib`'s order for every keyed table. Turning on `preserve_order` workspace-wide is the alternative; it reaches plane A through feature unification, so it needs the Architect's review | Architect | T-06 |
| **The host's derived indexes** | **Silent beyond children.** §5.1 names only the children-by-parent index. The life-course log keyed by `person_id` (REV-SPR-017 M7), `holders_of`'s tenure rows by title in log order, and the inverse lookups (`titles`, `offices`, `court`, `vassals`, `claims_on`, `claims_of`, `vows_of`) each need an index or a scan | **Ruled (Architect, 2026-10-17; 7/10): accepted as proposed.** one `Indexes` trait from the host, covering children, a person's life-course row, and a title's tenure rows in log order. The inverse lookups get a scan default in ascending id. A test shows that index and scan agree | Developer drafts; Architect accepts | T-07 |
| **A draw source for generated worlds** | **Silent.** `effect_ref.py` raises `KeyError` on a draw its world does not list, so generated programs cannot run | **Ruled (Architect, 2026-10-17; 8/10): accepted as proposed.** a test-only `draw_seed` on a fixture world. An unlisted key then draws SplitMix64 over a stated fold of `(draw_seed, purpose, kind, id, k)`, on both sides. Golden worlds carry no `draw_seed`, so they behave as today | Developer drafts; Architect accepts | T-08 |
| **`effect_ref.py --emit`** | Missing: the reference only compares with expected values | An additive mode. `--check`, and the functions that SPR-019's differential imports (`load_golden`, `prepare`, `succession_line`), stay unchanged (§11) | Developer (TOOLS) | T-08 |
| **The error code of a program with several faults** | §10 says "one of their codes" | **Ruled (Architect, 2026-10-17; 7/10): accepted as proposed.** the fuzz compares the code exactly on single-fault programs, and only accept or reject on the rest. It reports code differences on the rest without failing | Architect | T-08 |
| **V-WC-10's `sortition_citizenry` fixture** | **Conflict.** The example in `05.5` §2.1 needs `adults`, `core_regions` and `citizen`. The v0 prelude has none of them, and adding a relation is a `needs_code` gap (§5.1) | **Ruled (Architect, 2026-10-17; 7/10): accepted as proposed.** land a data-only law that v0 can express (`absolute_primogeniture`, pinned in the golden file), with no Rust change. V-WC-10's method line names it, and `sortition_citizenry` stays a recorded `needs_code` gap | Architect | T-09 |
| **Q8: `emitted_by = "rh-minds"`** | **Conflict.** Renaming the seven `rh-minds` verbs (`ARREST`, `PURGE`, `REVENGE`, `COUNSEL`, `LAMENT`, `RENOUNCE_FAITH`, `RESTORE`) to `T1.minds` makes `lint_odd.py` fail on the frozen T1 0.3.3: "the Registry gives verb … to T1.minds, but [[emits]] does not declare it". Four other verbs carry `agents`, which is not a system id either | **Ruled (Architect, 2026-10-17; 7/10): accepted as proposed.** the loader requires a Registry system id in `emitted_by` only for a verb that has `effects`. The rename waits for SPR-024, which owns T1's `[[emits]]` | Architect | T-06 |
| `neutral` on `[[reads]]` (the spec prerequisite of §10) | Exists (EVO-001, PR #49) on rate and mode reads | See the identity-read row | — | T-06 |

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | **C2** (Registry schema), as PRJ-002 expects; confirmed. No `state_hash` change on any golden world, because no built system calls the evaluator. **One condition: `registry_version` is not bumped.** A bump would change the `tuple … registry_version=…` header line of the `solo` ladder (and of `household` and `hamlet` once SPR-013 lands); smoke's header (`smoke-overlay v1`) would not change. Re-pinning the manifests would also change their SHA-256 pins (`worlds/solo/world.toml`). `bin/check_baseline.py` compares the whole baseline file and refuses a changed baseline in a PR not marked `[C3]` (SPR-008 T-04). So a bump would force a `[C3]` PR with no hash change. SPR-020 T-07 states the same and measures it: the bump alone changes only the header's `tuple` line. History reads none of the new sections until a system evaluates a program, so DET-1 holds without a bump. The bump rides on the next `[C3]` that makes one anyway (SPR-020 T-07; else SPR-015, SPR-018 or SPR-021), as S3's Registry entry did (REV-SPR-018 R3) |
| Planes touched (ADR-0024) | **A:** `rh-registry`, a new module `src/effect/` only. `ownership.rs` and `world.rs` are unchanged, and nothing in `rh-core` changes. **Tool:** `rh-manifest` (the Registry types, the environment builder, the golden-format case runner), `rh-cli` (`effect check`, `effect eval`), `specs/manifest/registry_v0.toml`, `specs/systems/lint_odd.py`, `specs/registry/effect_ref.py` (`--emit`, `draw_seed`), `bin/effect_fuzz.py` (new), `bin/check_all.sh`. **B, C:** none |
| Owned columns written | None. A program writes nothing; its effects are typed actions that the host submits (§6) |
| Reads (via `rh-contracts`) | None. The evaluator reads by column name through the host's `SystemView`. A contract crate cannot hold the walks (D8, REV-P1a NEW-3). A program's reads are its declaring system's `[[reads]]` and owned columns: the checker enforces them at compile time (`E_UNDECLARED_READ`) and `SystemView` at run time |
| RNG purposes added | None. A program draws only under its system's declared `[[rng]]` purposes, here M7's `sortition`, through the host's keyed stream (§4.6) |
| Golden-hash effect | **None.** Every ladder is byte-identical, its header included, and no file under `worlds/*/baselines/` changes |
| Fixed-point quantities | `int`, `q8_8` and `q16_16` values, through `rh_core::fixed` where it has the operation. The operations it lacks (`int` half-even division, Euclidean `div` and `mod`, `abs`, `floor`, `round`) are integer code in `rh-registry`, so `rh-core::fixed` is untouched. A decimal literal compiles by exact half-to-even rounding with no float (§3.2). Parameter bounds and `neutral` literals reach plane A through `rh-manifest`'s exact decimals (`05.5` §3.2). There is no float in `rh-registry` |
| Determinism lints (GOV-003 §2, §6) | Must pass. `rh-registry` gains no file I/O (BCK-002 A-47, A-51), no float, no hash-ordered container (`BTreeMap` and `Vec` only), no `env!` and no dependency |
| Tests that gate this sprint | **V-WC-10** (golden, mutants, fuzz, host seam), V-WC-04 (parity on the new Registry), V-DET-19, V-INSP-03, V-PLN-01, V-GOLD-01 (every ladder unchanged), V-PERF-04 and V-PERF-11 (unchanged: no engine path is touched), plus `bin/check_all.sh` |
| Docs updated in the same sprint | `bin/check_all.sh` and `python/tests/test_check_all_inventory.py` (same commit); the `CLAUDE.md` checks; `docs/09` V-WC-10's method line (test names, the fixture landed); `05.0` §6 (`rh-registry` hosts the evaluator); `05.5` §2.1 (a pointer: the v0 spec supersedes its sketch); `specs/README.md` (`--emit`, the fuzz); the spec's errata lines (Architect); `docs/decisions-log.md` (the Architect's rulings) |

**Rules carried from P0–SPR-013 (every task):**
- **Property tests vary sizes and seeds:**
  - worlds of 1, 12, 120 and 1,200 persons;
  - fan trees on both sides of the lineage bound;
  - at least eight seeds in the PR tier.
- **A positive control fails for the right reason.** Its `check_all` line matches the message (`fail:<text>`), never a bare failure, and each mutant fails its named cases.
- **A test input exercises the path it claims, and a probe asserts it:**
  - the pruning actually dropped members;
  - `neutral_reads > 0` where a neutral is claimed;
  - `dropped_actions > 0` where a drop is claimed;
  - the fuzz prints its coverage counters and asserts that each is non-zero.
- **Python tests read no CI environment variable.** The fuzz takes its seeds and the `rh-cli` path as arguments.
- **No statistical test here.** Timings are reported, never gated.
- **CI time.** Each CI part stays at or under 12 minutes (15 hard), reported in the PR.
- **Merge `origin/main` when main moves,** and re-run both parts.
- **Batch pushes.** Push at task boundaries, not per edit: CI minutes are billed. Docs-only pushes skip the golden part (`bin/ci_scope.py`).
- **Every new gate is in exactly one `check_all.sh` part,** with the inventory test updated in the same commit.
- **No new third-party crate.** Property tests use fixed-seed SplitMix64, as `crates/rh-core/tests/time_properties.rs` does. `proptest` comes only after the Architect's crate review (SPR-026's ruling).
- **Mutants never make a baseline.** The mutant feature is named `mutant-effect`, so BCK-002 A-52's refusal of `mutant-*` features in `rh-cli run` and `bin/regen_baseline.sh` covers it.
- **One commit per task** (GOV-005 §4).

## Tasks

T-01 to T-05 build the evaluator in order. T-06 is tool-side work and may start at once. T-07 follows T-04, T-08 follows T-05, and T-09 comes last.

#### T-01: The front end: lexer, parser, checker and the environment (§2, §3, §4.1, §4.4, §4.7, §7.4, §10; D3, D7, D16, D17; REV-SPR-017 M1, M3, r4-1)
- **Deliverable:** `crates/rh-registry/src/effect/`, with:
  - **`env`:** §7.4's environment as plain data, with no I/O:
    - enums with named defaults;
    - entity kinds, and the life-course log keyed by `person_id`;
    - columns, each with its entity, key, type, enum, reference kind, plane and owner;
    - per system: its reads, each with a `neutral` or marked required (the identity-read row of the specs table); its sends; its `[[rng]]` purposes; its parameters, with bounds already in fixed point and typed `int` or `q16_16`;
    - the `[commands]` signatures;
    - the registered systems;
    - the calendar.
  - **`lex` and `parse`:** the EBNF of §2 exactly, with source spans; §2's disambiguation rules; a keyword after the dot (r4-1); decimal literals to bits by exact half-to-even (§3.2).
  - **`check`:**
    - types (§3), including structural key types (§3.4);
    - resolution (§4.1);
    - constants, guarded division and degrees (§4.4);
    - names (§4.7);
    - planes, declared reads, neutrals, commands and owners (§4.5, §6);
    - the static draw-site lint (§4.6): constant folding, typed sites, every enclosing loop variable in the key, and no draw in a verb whose payload carries an outcome;
    - nesting at most 2, a bound product of at most 65,536, and expression depth at most 64 (§4.8, D12).
  - **Rejections:** a closed `ErrorCode` with §10's 16 codes, reported at the first fault met walking left to right, innermost first (§10).
  - **API (Q9):** `compile(kind, text, &Env, system) -> Result<Compiled, Rejection>`. A rejection carries its code, span and message; the message is not contractual.
- **Acceptance:**
  - **The golden rejections.** All 163 golden cases that expect `error:E_*` give their code. This covers all 16 codes, including the families "syntax (§2)", "names (§4.7)", "constants and division (§4.4)" and "keys (§3.4)" (V-WC-10).
  - **Deep nesting.** A program of 100,000 nested parentheses is refused with `E_BOUND` in a debug build without overflowing the stack (§4.2: the bound makes the evaluator's recursion finite).
  - **No panic on any input.** Unit tests feed truncated, non-ASCII and empty programs; T-08 fuzzes the rest.
- **Status:** [ ] not started

#### T-02: Values, arithmetic, reads and draws (§3.1, §4.2, §4.3, §4.5, §4.6, §4.8; D1, D2, D9, D18, D19; N6, minor 3, r4-2)
- **Deliverable:**
  - **Values and arithmetic.** `int`, `q8_8`, `q16_16`, `bool`, enum, entity and the opaque `event`. Arithmetic follows §4.3's table:
    - `rh_core::fixed`'s saturating add, subtract, negate and multiply, its division, `widen`, `narrow_saturating` and `draw_hits`;
    - the integer rules in `rh-registry`.
  - **Evaluation order (§4.2):** innermost first and left to right; `and` and `or` evaluate both sides; `if` evaluates the taken branch only.
  - **Reads, through the host's `Reads` trait (T-07),** with three outcomes:
    - a row's value;
    - missing: the zero of the type, or the enum's named default, counted as `missing_reads`;
    - an absent owner's neutral, counted as `neutral_reads` (NEW-4).
  - **Draws, through the host's `Draws` trait.** `draw` is `u >> 1`; `chance` is `draw_hits(u, p)` (D9). The run-time ledger is:
    - for a law: one evaluation of one program for one binding (N6);
    - for verbs: all of one system's verb evaluations in one step, held in a `VerbStep` the host owns (r4-2).

    A repeated key returns `EvalError::RngReuse { purpose, kind, id, k }`.
  - **`Diagnostics`:** `missing_reads`, `neutral_reads`, `truncated_relations`, `dropped_actions`. None of them enters a value or a hash (§4.5).
- **Acceptance:**
  - **The golden value families reproduce:**
    - literals (§3.2), 31 cases;
    - int arithmetic, 33;
    - fixed-point arithmetic, 28;
    - conversions, 28;
    - comparisons, logic and conditional, 39;
    - builtins, 27;
    - reads (§4.5), 27;
    - randomness (§4.6), 24;
    - the event type (§3.1, N3), 10.
  - **The diagnostic pins hold:** all 17 `missing_reads`, including `and_evaluates_both_sides` and `or_evaluates_both_sides`, and all 4 `neutral_reads`, including `neutral_fixed_value_is_the_declared_one`.
  - **The run-time ledger:** the two `invariant:E_RNG_REUSE` cases fail, and `succession_sortition_evaluated_twice_in_one_step` returns one line with no invariant (N6).
- **Status:** [ ] not started

#### T-03: The prelude: kinship, tenure, M7's relations, aggregates and keys (§5, §3.4; D4, D5, D6, D8, D12, D13, D14; REV-SPR-017 B2, B3, M2, M7, N1, N2, N5)
- **Deliverable:** every relation and accessor of §5.3, as code in `rh-registry`, with its bound and canonical order (§5.2):
  - **Kinship over the life-course log:** `children`, `kin`, `related`, `ancestors`, `descendants`, `descends`, `line`, `line_agnatic`, `up`, `common`, `up_agnatic`, `common_agnatic`.
    - The log is read by `person_id` through the host's index, never by row id (M7).
    - `line` and `common` are father-first at ties (D4).
    - Siblings are ordered by `(birth_tick, person_id)`.
  - **`lineage`:**
    - it prunes the dead with no living descendant among the members, then applies the bound of 4,096;
    - above the bound after pruning it returns `EvalError::Truncated`, never a cut (§5.2; D12);
    - it yields in `(up, line)` order.
  - **`holders_of`:** over the tenure log, most recent first, each person once at their most recent tenure (B3, D14).
  - **M7's relations:** `titles`, `vassals`, `offices`, `court`, `claims_on`, `claims_of`, and S7's `vows_of`.
  - **Every other relation above its bound** drops the tail and counts `truncated_relations`.
  - **Aggregates (§5.4)** fold in relation order, with `else` for an empty `min` or `max`.
  - **Keys (§3.4):** a total order; a list that is a prefix of another comes first.
- **Acceptance:**
  - **The golden families reproduce:**
    - relations and aggregates (§5), 72 cases;
    - genealogical accessors, 28;
    - ancestors, descendants and related, 25;
    - keys, 19;
    - D5's guard, 4;
    - the tenure log and `holders_of`, 22;
    - the lineage bound and pruning, 6;
    - the tenure-log precondition and the sixteen-tenure window, 6;
    - the guard under uncle–niece marriage, 3.

    These run on the `dynasty`, `deep`, `rota` and `niece` worlds.
  - **The fan worlds** are built in Rust from `[worlds.<name>.fan]`, exactly as `effect_ref.fan_rows` documents:
    - `lineage_fan_under_the_bound` gives 583;
    - `lineage_fan_over_the_bound_is_truncated` gives `invariant:E_TRUNCATED`;
    - `lineage_fan_pruned_before_the_bound` gives 583;
    - `lineage_fan_pruned_members_are_never_read` gives `missing_reads = 0`.
- **Status:** [ ] not started

#### T-04: Program kinds, the succession protocol and verb effects (§6, §7.1, §7.2, §9; D10, D11, D17, D18; N3, N4, N6, r4-2; Q10)
- **Deliverable:**
  - **The kinds of §7.1, with their bound variables and result types:** `candidates`, `eligible`, `order`, `effects`, `fulfilled` and `conflict`. `derive` is reserved (Q10): it has its signature and a unit test, and no Registry entry.
  - **`succession::line(law, holder, title, now, host) -> Result<Vec<PersonId>, EvalError>`** runs §7.2's three steps:
    - evaluate `candidates`;
    - keep the candidates for which `eligible` holds;
    - sort by `(order(x), person_id)`.
  - **Effects** evaluate to ordered, typed actions: `Action { command, person, args }`. Each argument is typed: an int, fixed-point bits, a bool, an enum ordinal, an entity id, or an event argument (`Own` or `None`; specs table). A program never writes and never encodes an event id.
    - An action to an owner the world does not register is not emitted, and it is counted in `dropped_actions` (NEW-4).
    - A verb's payload fields (`success`) are bound like variables (N4, D18).
  - **`render`** gives §6's queue form, for the golden file only.
- **Acceptance:**
  - **The golden families reproduce:**
    - effects (§6), 32 cases;
    - vow predicates (§9), 13;
    - the succession laws, 17: all seven laws, the variants, representation and twins;
    - the two-layer draw rule and the title key, 8;
    - verb programs and the step, 3. This family includes the two cases that run several `events` through one `VerbStep`.
  - **The `dropped_actions` pins hold** (`effects_to_an_absent_owner_are_dropped`, `effects_burn_holdings_is_dropped_while_m4_is_absent`), and `evaluations = 2` gives equal lines (N6).
  - **Timings, reported and not gated:** the seven laws on `fan4_pruned` (583 candidates), and `primogeniture` on a 4,000-member lineage. D12 expects under a millisecond per program.
- **Status:** [ ] not started

#### T-05: The golden test, both builds, and the planted mutants (§13; V-WC-10; REV-SPR-017's mutants)
- **Deliverable:**
  - **The shared case runner, `rh_manifest::effect_cases`** (tool plane: it reads files, and reads TOML float bounds by their shortest round-trip decimal, as `exact.rs` does). It is used by the test and by `rh-cli effect eval`. It:
    - builds an `Env` from a golden-format `[registry]` slice;
    - builds fixture worlds from `[worlds.*]`: tables, `[draws]`, the fan generator;
    - runs each case by kind, with `bind`, `payload`, `events`, `evaluations`, `extra_reads` and `extra_sends`;
    - renders the result in the header's exact text;
    - compares the result and the pinned diagnostics.
  - **`crates/rh-manifest/tests/effect_golden.rs`** reads `specs/registry/effect_language_golden_v0.toml`. It refuses a file whose `version` is not the evaluator's (§11), a duplicate id and an unknown kind, so no case is skipped silently. It prints `EFFECT LANGUAGE v0.5.0 (rust): PASS (558 cases)`.
  - **The `mutant-effect` feature of `rh-registry`** (forwarded by `rh-manifest`): a test-only `Mutant` value passed through the evaluator's options. One build serves every mutant, where SPR-012 used one feature per mutant. The test `mutants_are_killed` runs the golden file under each mutant and asserts two things: that mutant's sentinel cases are among the failures, each with the wrong value the mutant predicts; and `Mutant::None` passes all 558.
- **Acceptance:**
  - **558 of 558 cases reproduce** byte for byte in rendered text, with every pinned diagnostic, in a debug build and in `--release` (§13: as `rh-sample`'s golden vectors at both optimisation levels). `effect_ref.py --check` stays green on the same file.
  - **Positive control:** the golden file with one expected value altered (`check_all`'s `effect_golden_control` pattern) fails the Rust test with `FAIL lit_true`.
  - **Each mutant fails its named cases:**

    | Mutant | Source | Sentinel cases |
    |---|---|---|
    | Life-course log read by row id | REV-SPR-017 M7 | `succession_primogeniture`, among others |
    | `and` and `or` short-circuit | §4.5, minor 3 | `and_evaluates_both_sides`, `or_evaluates_both_sides` |
    | `lineage` stops at height 2 | B2 | `lineage_reaches_great_grandparents` |
    | The bound applied before pruning | N2 | `lineage_fan_pruned_before_the_bound` |
    | A silent cut above the bound | §5.2 | `lineage_fan_over_the_bound_is_truncated` |
    | An absent owner's read gives the type's zero or default | M5; round 4 minor 3 | `neutral_fixed_value_is_the_declared_one`, `neutral_enum_value_is_the_declared_one` |
    | A missing enum read gives ordinal 0 | round 1 minor | `read_enum_of_none_is_the_declared_default` |
    | Fixed-point multiply rounds half up | D1 | `q16_mul_tie_to_even_zero`, `q16_mul_tie_to_even_negative` |
    | `common` and `line` go mother-first | D4 | `common_of_a_brother_is_the_father_first` |
    | The draw ledger is kept per step | N6 | `succession_sortition_evaluated_twice_in_one_step` |
    | The final tie-break by descending `person_id` | §7.2; NEW-7 | `succession_primogeniture` and `succession_seniority` (the twins 16 and 27) |
    | A list key puts the prefix last | §3.4 | `succession_primogeniture` (Kell, 10, before their daughter Thora, 18) |

- **Status:** [ ] not started

#### T-06: The Registry schema change, the Rust loader and the real environment (§7.3, §7.4, §11; D10, D15; Q8, Q13, Q14; EVO-001 R9, R12)
- **Deliverable:**
  - **`specs/manifest/registry_v0.toml`:**
    - **`[effect_language]`:** `version = "0.5.0"`, and `[effect_language.enums]` (specs table): `sex` and `death_cause` from their M2 columns; `title_state` and `claim_status` from M7's; the Registry lists `title_tiers`, `succession_laws`, `vocations`, `office_kinds`, `vow_kinds`, `opinion_modifiers` and `overlays` by name. Each has the golden slice's default.
    - **`[succession_laws]`:** `ids` unchanged, in the same order (`title.law` stores ordinals: §11). Add `[succession_laws.<law>]` with `candidates`, `eligible` and `order` for the seven ids, each text-identical to the golden file's `[registry.programs.succession_laws.<law>]`. Those are the texts D37 approved, with D14 (a) and D4. The variants stay in the golden file.
    - **`[commands]`:** one entry per command that has an `[[applies]]` on `main`, 13 in all:
      - M2: `ScheduleDeath`, `Marry`;
      - M7: `PressClaim`, `PetitionSeat`, `RenounceClaim`, `Resign`;
      - T1: `OpinionModifier`, `AgendaGoal`, `ClearGoal`, `ApplyThought`, `ApplyInfluence`, `Promote`, `Demote`.

      Each entry carries its `owner`, its key parameter (the `key_field`), its parameters in payload order with language types, and `outcome`. The types map as: `PersonId` → `ref person`, `ClaimId`/`OfficeId`/`VowId` → their refs, `u64` → `int`, `q8_8` → `q8_8`, `EventRef` → `event`, and `enum(<column>)` → that column's enum. No entry has `tick` (`05.1` §2 rule 2). Commands to owners without a spec (M4's `BurnHoldings` and `TransferHoldings`, and S6's and S7's) get no entry until an `[[applies]]` types them (Q14).
    - **`[verbs]`:** the optional `payload` and `effects` fields exist. No verb program lands (specs table). `emitted_by` is unchanged (Q8).
    - **`registry_version`:** unchanged (compliance table).
  - **The `rh-manifest` loader:**
    - **Types**, each `deny_unknown_fields`: `SuccessionLaws { ids, programs }` replaces `IdList` for that section; `Verb` gains `payload` and `effects`; `Commands` and `EffectLanguage { version, enums }` are new.
    - **Keyed-table key order** comes from the source text (specs table).
    - **`effect_env(&Registry, specs)`** builds the `Env` from the Registry and the specs' TOML text: `[[owns]]` (entity, type, `values`, `registry`, plane), `[[reads]]` (`use`, `neutral`), `[[commands]]`, `[[rng]]`, and the Registry's parameters in fixed point. It takes the text, so A-51's move of the spec loader (SPR-014) moves only a call site.
    - **`effect_problems`** runs after `Registry::problems`, so SPR-020's refusals keep their order. It refuses, by name:
      - a language version the evaluator does not implement;
      - a program table for an id that is not in `ids`;
      - a law in `ids` without its three programs;
      - an enum whose default is not one of its values;
      - an enum whose columns' `values` differ;
      - a `[commands]` owner that is not a Registry system;
      - a program that does not compile (naming the law, the program and the code);
      - a verb with `effects` whose `emitted_by` is not a Registry system (Q8, scoped);
      - a constant `death_cause.none` passed to `ScheduleDeath` (Q13: a Registry bug);
      - two verb programs of one system that share a draw site (§4.6).
  - **`lint_odd.py`** checks the Registry's `[commands]` both ways: against the owners' typed `[[applies]]` (names, order, types, key field, `outcome`) and against the senders' `[[commands]]`. Every `cause_event` and `source_event` must be typed `event`.
  - **`rh-cli effect check`** compiles every Registry program against the environment built from `specs/systems/`.
  - **`manifest_v0.py` is unchanged:** it reads only `ids` (§7.3).
- **Acceptance:**
  - **The real programs.** `rh-cli effect check` prints `EFFECT PROGRAMS: PASS (7 laws, 21 programs)`, against M7 0.3.4's real reads: the identity reads have no `neutral` (EVO-001 R12; specs table). A test compares the 21 texts with the golden file's (D37).
  - **The real environment matches the golden slice:**
    - for every enum the two share, the defaults and values are equal (§3.1);
    - the declaration order of `[vow_kinds]` and `[opinion_modifiers]` equals `tomllib`'s.
  - **Planted Registries and specs, each refused with its named message** (§7.3, §10):
    - `[succession_laws.primogeniture_open]` (an id not in `ids`);
    - `seniority` without `order`;
    - an unknown field in a law table;
    - `version = "0.6.0"`;
    - a law that reads `person.residence` (`E_UNDECLARED_READ`, naming the law);
    - an M7 copy with `polity.exec_constraint`'s `neutral` removed, under a law that reads it (`E_NO_NEUTRAL`);
    - an enum default outside its values;
    - a fixture verb program sending `death_cause.none` (Q13);
    - a verb with `effects` and `emitted_by = "rh-minds"` (Q8);
    - two T1 verb programs sharing a draw site (`E_RNG_REUSE`, naming both). Two laws that share `sortition`'s site compile, because the laws are exclusive per title (§4.6).
  - **The `lint_odd.py` fixtures, both ways** (`tests/lint_fixtures/registry_commands/`):
    - a `[commands] ScheduleDeath` with a `tick` parameter fails, naming M2's `[[applies]]`;
    - an `[[applies]]` with no `[commands]` entry fails;
    - a `cause_event` typed `int` fails;
    - the real Registry passes.
  - **Every existing reader of the Registry stays green:**
    - V-WC-04: `manifest_parity.py` on every example and parity fixture, with unchanged diagnostics;
    - `lint_odd.py` on the real, smoke and `--next` specs;
    - `lint_patterns.py`;
    - `gen_skeletons.py --check` (no skeleton changes);
    - `gen_manifest.py --check`;
    - `selftest_worldgen_lite.py`, which reads the Registry.
- **Status:** [ ] not started

#### T-07: The host seam: reads through `SystemView`, the indexes, draws and queued commands (Q9; D8, D15; §4.5, §4.6, §5.1, §6; `05.1` §2 rules 1, 2)
- **Deliverable:**
  - **`rh_registry::effect::host`** defines three traits:
    - `Reads`: column and entity in, `Row`, `Missing` or `Neutral` out;
    - `Indexes`: children, a person's life-course row, a title's tenure rows in log order, and the inverse lookups with a scan default in ascending id;
    - `Draws`: purpose, entity and `k` in, a `u64` out; `t = now` comes from the host.
  - **`SystemViewHost`:** an adapter over `rh_core::world::SystemView` and the host's indexes.
    - It reads by column name, so `SystemView` enforces the system's `AccessSpec` at run time.
    - It finds an entity's row through its id column: by binary search where the owner keeps the table sorted by id (the ledger), else through a host index.
    - The neutral path is wired to SPR-014's keyed lookup by whichever of the two sprints merges second (merge route).
  - **`StepDraws`:** an adapter over `StepCtx`'s keyed stream, under the purpose the program names (M7's `sortition`, keyed `(candidate, k = the title, t)`, M7 0.3.4 `[[rng]]`).
  - **`to_command(action, verb_draft)`:** builds an `rh_core::command::Command` with the verb's draft as `cause` (§6). An event-typed argument becomes an `EventRef` once SPR-014 has merged; until then, an action with one is not convertible, and the error says so.
- **Acceptance:**
  - **`crates/rh-manifest/tests/effect_host.rs`, on an `rh-core` stub world.** It sits beside the golden test so that it can load the fixture through `effect_cases`; a dev-dependency of `rh-registry` on `rh-manifest` would form a cycle. Test-only owners hold M2's and M7's columns, and the `dynasty` fixture is loaded into them.
    - A stub host with M7's read declarations evaluates the seven laws in its step through `SystemView` and gets the golden `succession_*` lines (V-WC-10).
    - A planted `AccessSpec` without `life_course.sex` fails with `SimError::UndeclaredRead` naming the column, not a silent zero (V-WC-10: reads checked against the scheduler's read sets).
  - **Draws:** `sortition`'s draws come from the step's keyed stream. Two evaluations in one step give one line (N6), and two titles arbitrated at one instant draw independently (minor 4).
  - **Queued commands:** a stub verb host submits `FLED`'s `Resign` actions through `StepCtx::submit`, with the verb's draft as `cause`. The stub owner takes them in action order (§6; `05.1` §2 rule 2).
  - **Indexes:** index-based and scan-based lookups agree on random worlds of three sizes, with eight seeds each.
- **Status:** [ ] not started

#### T-08: The differential fuzz against `effect_ref.py` (§13; §10; REV-SPR-017's mutants; SplitMix64)
- **Deliverable:**
  - **`effect_ref.py --emit FILE`:** evaluates a golden-format file whose cases need not carry `expect`. It prints one line per case: `id`, the rendered result, and the four diagnostics. A crash prints `crash:<kind>`. A fixture world may carry `draw_seed` (specs table). `--check` and the imported functions are unchanged (§11).
  - **`rh-cli effect eval FILE`:** the same lines from the Rust evaluator, through `effect_cases`.
  - **`bin/effect_fuzz.py`** (standard library only; AGT-002-TOOLS), with fixed SplitMix64 seeds given on the command line.
    - **Worlds** at the sizes of the carried rules, plus fan trees at depth 3 and 4 with fanouts 4–9:
      - life-course logs with row ids that differ from person ids;
      - unknown parents, twins, the dead and compacted, half-siblings;
      - cousin and uncle–niece marriages, and lines seven or more generations deep;
      - tenure logs of 1 to 20 rows per title, rotas included;
      - titles, offices, claims and vows.
    - **Programs:** type-directed over the golden `[registry]` slice, for every kind, including generated laws run as `succession` cases. Depth runs from 1 to 8. Single-fault mutants of well-typed programs aim at each of the 16 codes.
    - **Comparison:** the result text and all four diagnostics, `truncated` included (no golden case pins it). The code is compared exactly on single-fault programs, and only accept or reject on the others (specs table).
    - **On a mismatch,** the driver prints the seed, the case, the program and both results, and writes a reduced repro case under `target/fuzz/`.
    - **Its own tests** (`python/tests/`): one seed gives byte-identical files, and the coverage counters work.
- **Acceptance:**
  - **The PR tier:** at least 16 seeds, sized to run in 90 s or less, with zero mismatches and zero crashes on either side. A crash of the reference is a finding against `effect_ref.py`; its fix keeps every golden case (§11).
  - **Coverage, printed and asserted non-zero:** every error code, both run-time invariants, every relation and builtin, every aggregate, every program kind, and `truncated_relations > 0`.
  - **Power, measured once and reported in the PR and the audit (not a CI gate):** run through a `mutant-effect` build of `rh-cli effect eval`, the fuzz alone kills at least 8 of T-05's 12 mutants.
  - **Positive control:** the driver run against a planted reference copy that rounds fixed-point products half up fails with `MISMATCH`, naming a case.
  - **The nightly tier** runs 64 times the seeds, in `nightly.yml`.
- **Status:** [ ] not started

#### T-09: V-WC-10's data-only extension, the gates and the docs
- **Deliverable:**
  - **The data-only extension (EVO-1, V-WC-10), in its own commit.** That commit touches only TOML and test data, with no `.rs` and no `Cargo.*` file. It is a fixture Registry under `tests/registry_effect/` that adds `absolute_primogeniture` to `ids`, with its golden programs, plus a case file pinning its lines on `dynasty` and `deep`. `rh-cli effect check --registry <fixture>` compiles 8 laws, and `effect_cases` runs the new law by name.
  - **`check_all.sh` part `core`:**
    - the golden test, debug and release;
    - `mutants_are_killed`;
    - `effect_host`;
    - `rh-cli effect check` and the extension's check;
    - the `lint_odd.py` fixtures;
    - the PR-tier fuzz and its control;
    - SPR-019's differential `selftest_population_synthesis.py --effect-ref specs/registry` (REV-SPR-019 R3) is already in part `core`, with its reversed-order control (added by the Architect with this plan). It imports `effect_ref.py`, which this sprint changes, so it must stay green.

    The inventory test is updated in the same commit. If part `core` exceeds 12 minutes, the release run moves to part `golden` by the Architect's ruling, recorded in the audit.
  - **The docs** of the compliance table.
- **Acceptance:**
  - **The extension commit** changes no Rust file (`git show --stat`, checked in the audit), and its law reproduces its pinned lines (EVO-1).
  - **The ladders.** `smoke`, `solo`, and `household` and `hamlet` if SPR-013 has merged, equal their baselines at 1 and N threads, headers included (V-GOLD-01). No file under `worlds/*/baselines/` changes.
  - **V-PERF-04 and V-PERF-11** are unchanged.
  - **CI time.** Both parts run in 12 minutes or less, and the times are in the PR.
- **Status:** [ ] not started

## Director items, and what changes under the alternative

| Item | Planned | Under the alternative |
|---|---|---|
| **Size: about 4.5 agent-days against GOV-005 §3's ~3** (recommendation 7/10: one sprint) | One sprint and one C2 PR. The lane has slack: nothing consumes the evaluator before SPR-021 in wave 6, and PRJ-002 §4 already gives SPR-017 two waves | **Split:** SPR-017 keeps T-01–T-05 and the gates (about 3 days: the evaluator, proven on the golden file, with mutants). A follow-on C2 sprint takes T-06–T-08 (about 1.7 days: the Registry, the host seam and the fuzz), and it must close before SPR-021 starts. The cost is a second Tester pass, a second audit and a second plan approval, for no schedule gain |
| The errata (specs table: identity reads, no `tick`, the event `none` encoding) | Architect's text rulings, following EVO-001 R9 and R12 and `05.1` v1.14. No golden row moves, so D37's approval stands. The director is told in the merge summary | If the Architect chose instead to realign the golden slice (drop `tick` from 14 cases; `ApplyInfluence.delta` to `q8_8`), §11 requires the director to re-approve the golden file. This plan does not do that; the realignment belongs with SPR-024's `MURDER_ATTEMPT` decision |

## Out of scope (explicit)
- **Any system evaluating a program:**
  - M7's succession, V-SAGA-19 and the mapping of `EvalError` to a fatal `SimError` with its crash artifact (SPR-021);
  - T1's verbs (SPR-024);
  - S7's vow predicates (P1b);
  - `derive` (P3.2b).
- **The `registry_version` bump.** It rides on the next C3: SPR-021 must bump it in any case, because M7 then reads the programs.
- **Real verb programs:**
  - `MURDER_ATTEMPT`'s outcome path (SPR-024, under `05.1` rule 2 and D11);
  - `DESPAIR_PACT`'s magnitude source (`[opinion_modifiers]`);
  - `HOLDINGS_BURNED`, which waits for an M4 accessor (Q14, P1b).
- **Q8's full rename** of the `rh-minds` and `agents` verbs (SPR-024; a BCK-002 item).
- **`sortition_citizenry`:** a `needs_code` gap (`adults`, `citizen`).
- **Later language versions:** `schedule_outcome` and `emit_event` (D11, v0.2), and lowering hot expressions to Rust (`05.5` §2.1).
- **Changes to golden rows,** and `proptest` or `loom`.
- **`WorldSpec` carrying the compiled programs:** that is the first consumer's (SPR-021).

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | T-01, T-04, T-06, T-08, written from §2–§4, §6–§7 and §10 and the golden header before reading the code:<ul><li>at least 60 cases of their own, derived from the spec text rather than from the golden file, including `truncated_relations`, which no golden case pins;</li><li>adversarial programs: deep nesting, 10⁵ tokens, long literals, non-ASCII, empty kinds;</li><li>the real environment: identity reads, the enums table, keyed-table order;</li><li>the Registry refusals and the `[commands]` lint, with planted fixtures of their own;</li><li>the fuzz with seeds and a generator variant of their own;</li><li>no panic on any input</li></ul> | `VER-SPR-017-TEST` |
| AGT-003-DET | T-02, T-03, T-05, T-07, T-09, written from §4.2–§4.8, §5.2, §7.2, `05.1` §6 and ADR-0006:<ul><li>debug and release bit-equality;</li><li>results unchanged when the life-course log's rows and the title, office and claim tables are permuted (the tenure log's order is semantic and stays fixed);</li><li>one `Compiled` evaluated from 1 and 4 threads gives equal results;</li><li>the draw ledger scoped per evaluation for laws and per step for verbs;</li><li>the mutants killed for their reasons, plus at least three mutants of their own;</li><li>`SystemView` refusal with a planted `AccessSpec`;</li><li>every ladder unchanged at 1 and N threads, and the CI times</li></ul> | As above |

## Merge route

- **Class and conditions.** C2, merged by the Architect after the Tester pass and the audit (GOV-005 §6). The PR title ends `[C2]`. No golden change, so nothing is announced beyond the merge summary.
- **Not bound by the one-C3 rule** (GOV-005 §4.1). The sprint merges in any gap between C3 merges (PRJ-002 §4: "it merges between C3s"). GOV-005's "one merge at a time" still applies.
- **Recommended order:** before SPR-020 if it is ready by then, so that the `registry_version` SPR-020's `[C3]` commit bumps names a Registry that already carries the programs (6/10). Otherwise it merges after, and the programs ride on the next bump.
- **Shared files with wave 4, and how SPR-017 avoids conflicts:**

  | Area | SPR-014 | SPR-020 | SPR-026 | SPR-017 |
  |---|---|---|---|---|
  | `crates/rh-registry` | A-51 moves `load_specs` and `registry_system_ids` out; `ownership.rs`, `tests/real_specs.rs` | The compiled Registry table (role, motive and attribute ids) | — | A new `src/effect/` only, plus one `pub mod` in `lib.rs`. No file I/O, no change to `ownership.rs` or `world.rs`. `CompiledRegistry` stays in `effect`; whichever of SPR-017 and SPR-020 merges second joins the two tables with one field |
  | `crates/rh-manifest/src/registry.rs` | — | `[event_roles]`, `[motive_edges]`, `[role_domains]`; `Registry::problems` | — | New types in a submodule file. `registry.rs` changes only the `succession_laws` field type, two `Verb` fields and two `Registry` fields. The refusals live in `effect_problems`, which runs after `problems` |
  | `specs/manifest/registry_v0.toml` | — | The three typed sections; the `[C3]` bump of `registry_version` | — | Only `[succession_laws.*]`, `[effect_language]` and `[commands]`. `registry_version` and `[verbs]` keys are untouched |
  | The spec table and `lint_odd.py` | T-01 adds `[[reads]] use`/`neutral`, typed `[[applies]]` and `DRAINS` | The `[[emits]]` constants | — | Parses the spec text it needs in the tool plane, and adds one function to `lint_odd.py`. Whichever merges second adds a test that both parses agree on the real specs |
  | `rh-core` | `SystemView`'s keyed lookup, `EventRef`, typed `submit`, new `SimError` variants | `EventDraft`, new `SimError` variants | — | Nothing: the run-time invariants are `rh-registry` errors |
  | `rh-cli` | — | — | `serve.rs`, `tail --live`, `run --socket` | A new `src/effect.rs`, and one arm in `main.rs` |
  | `check_all.sh`, the inventory test, `CLAUDE.md` | yes | yes | yes | Textual; the second to merge resolves them |

- **What the merge must re-verify** (after merging `origin/main`, both CI parts):
  - **The ladders.** Every ladder equals its baseline at 1 and N threads, headers included, and no file under `worlds/*/baselines/` changed. `check_baseline.py` is green under a `[C2]` title.
  - **The Registry's readers:** V-WC-04 parity, `lint_odd.py` (real, smoke, `--next`), `lint_patterns.py`, `gen_skeletons.py --check` and `gen_manifest.py --check`, all on the merged Registry.
  - **The oracle:** `effect_ref.py --check` (558) and its control, and SPR-019's `--effect-ref` differential, because `effect_ref.py` changed.
  - **With SPR-014 in `main`:**
    - the adapter's neutral path goes through SPR-014's keyed lookup, and a dropped read of an absent owner gives `neutral_reads = 1`;
    - event-typed actions become `EventRef`, and a fixture `DESPAIR_PACT` goes through typed `submit`;
    - the cross-check of the two spec parses passes;
    - A-51's call sites have moved.
  - **With SPR-020 in `main`:**
    - both sets of typed Registry sections load under `deny_unknown_fields`;
    - if SPR-020 bumped `registry_version`, this PR leaves it alone, and the ladders equal SPR-020's new baselines.
  - **CI time** for both parts.

## Blockers

| # | Blocker | Status |
|---|---|---|
| 1 | The Architect's rulings of the specs table: identity reads (needed before T-01); no `tick`, the event `none` encoding, the enums table, key order and Q8 (before T-06); the draw source and multi-fault codes (before T-08); the V-WC-10 fixture (before T-09) | **Resolved:** all ruled as proposed (Architect, 2026-10-17). Key order: from the source text, never `preserve_order` workspace-wide |
| 2 | The director's plan approval (D52, with the size question) | Open |
| 2 | The director's plan approval (GOV-005 §2), including the size (see "Director items") | Open |
| 3 | SPR-013, SPR-014, SPR-020 | Not blockers: see "Depends on" and "Merge route" |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] Tester verdicts are PASS.
- [ ] Architect audit `VER-SPR-017` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] The merge route matches the CI-detected change class (C2). No file under `worlds/*/baselines/` changed, and `registry_version` is unchanged.
- [ ] The spec's errata lines and the Architect's rulings are recorded (`docs/decisions-log.md`). `docs/09` V-WC-10 names the tests and the data-only fixture.
