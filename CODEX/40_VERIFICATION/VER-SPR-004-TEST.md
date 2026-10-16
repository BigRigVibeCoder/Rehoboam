---
id: VER-SPR-004-TEST
title: "Contract tester evidence for SPR-004"
type: reference
status: OPEN
owner: tester
agents: [tester]
tags: [verification, testing]
related: [SPR-004, VER-SPR-004, AGT-003-IF]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** The four gating tests pass as the contracts state them: V-WC-04 (all 5 examples and the 16-file corpus, 21 manifests, agree with `manifest_v0.py`), the Registry round trip (35 sections, zero differences against `tomllib`), the ownership lint (every named violation class is caught) and the schema diff (0 structural differences; 237 raw differences, all annotations). `bin/check_all.sh` is green on clean code (77 checks) and turns red on every gate mutation tried, but the parity gate compares only verdict, exit code and counts, so it let four removed Registry checks through (found by the tester corpus, not the shipped one). Adversarial manifests, though, found divergences the shipped corpus cannot see: **eight defects, none blocking G0** (two medium: a UTF-8 BOM is rejected, and integral floats of 2^63 or more are accepted for 64-bit integer fields; six low). Seven contract gaps need an Architect ruling, chief among them: the reference is stricter than the YAML specification (tabs, `?` in flow scalars, empty keys) and the Rust parser follows the specification.

Tested commit: `7a08d15` on `sprint/SPR-004-registry-loader` (PR #6). All probing ran on throwaway copies (`git archive`), and the original tree was only read. Probes were written from the binding contracts before the implementation was read; the plan was fixed first (`probe_plan.md`, scratchpad `ver/`). The Tester runs on a model different from the Developer's (set by the Architect, GOV-005 §4.2). Harness: `diff.py` (runs `manifest_v0.py validate` and `rh-cli manifest validate` on the same file and compares verdict, exit code, error and warning counts, and every error path and message); case generators `cases1..6.py`, `fuzz.py`, `regmut.py`, `own.py`, `regdump.py`, `gm.sh`, `gp.sh`, `mut.sh` (scratchpad, not committed). Python reference: `python/.venv` from `uv sync --frozen --project python`; `cargo-nextest` 0.9.148 and `cargo-deny` 0.20.2 installed with `cargo install --locked`.

## 1. Verdicts

| Gate | Verdict | Basis |
|---|---|---|
| V-WC-04 parity (T-02) | **PASS on the gating set; DEF-1, DEF-2, DEF-3 open** | 21 of 21 manifests agree (5 examples, 16 corpus), including `uz_job` valid and `invalid_draft_style` invalid (8 errors, 3 warnings, identical lines). 3,414 generated manifests: 3,330 agree on verdict and counts, 3,248 of them also on every error path and message; the 84 verdict or count differences and the 82 message-only differences (6 of them harness artefacts) trace to the defects and gaps below. Verdict-level divergences exist only outside the corpus: a BOM, integral floats of 2^63 or more, integers of 2^127 or more, explicit `!!binary`/`!!set`/`!!timestamp` tags, and YAML the reference cannot read (GAP-1). |
| Registry round trip (T-01) | **PASS; DEF-5, DEF-6 open** | Load and re-serialise equals `tomllib` on all 35 top-level sections with 0 differences (types included). `S6.artifacts` (8 params), `S7.vows` (9), `T1.minds` (4) load with their bounds. A synthetic system `X9.new` added to a Registry copy is accepted in manifests with no code change. Unknown fields are refused in every typed section. 113 Registry-variant cases: 59 agree; 54 differ by design (Rust refuses malformed Registries, exit 3; Python crashes or accepts), see GAP-5. |
| Ownership lint (T-03) | **PASS; DEF-8 open** | 43 probes, 38 behave as the contract says (writes to an unowned column, writes to another system's column, undeclared reads, plane-A read of plane-B and plane-C columns, one column claimed by two specs, a second spec for one system, a duplicate inside one spec, a system missing from the Registry, malformed specs); the other 5 are DEF-8. Controls (own column, declared read, plane-B reader of plane A) pass as required. |
| Schema generation (T-04) | **PASS** | `rh-cli manifest schema --diff` reports 0 differences over 32 definitions and the root. Raw comparison without normalisation: 237 differences, all in four annotation keys (§4). A drifted bound in the reference, or in the Rust types, turns the gate red. |
| `bin/check_all.sh` | **PASS** | 77 checks green on a clean copy after `uv sync --frozen --project python` (65 s); report written. Red on all 10 gate mutations that target a gate (§5). The parity gate itself is weak: GAP-3. |

## 2. Results (condensed; one row per probe family)

| Probe IDs | Contract clause | Expected | Actual | Verdict |
|---|---|---|---|---|
| P0: 5 examples, 16 corpus files | SPR-004 T-02; docs/09 V-WC-04 | same verdict, error count, warning count | 21 of 21 agree | PASS |
| P1: 315 cases (`cases1`): seed and other integers as `0o10`, `010`, `0x1F`, `1:30`, `1_000`, `+7`, `-0`, `0b101`, `.inf`, `.nan`, `~`; booleans as `y/Y/yes/on/off/no/N`, quoted and plain; floats such as `1e-1`, `.1`, `1_0.5`, `0.035e1`, `1:30.5`; class-share sums at 1e-6; slug edges (`abc\n`, unicode, 63/64/65 characters); string lengths in code points (é, emoji); merge keys (`<<` single, list, null, scalar, quoted, override); repeated keys (top, nested, flow); anchors and aliases; BOM, CRLF, CR, UTF-16, non-UTF-8; empty and null documents; `%YAML` directives; explicit tags; non-string keys | YAML 1.1 and pydantic lax-mode semantics as in `manifest_v0.py` | 297 agree. Differences: BOM (DEF-1); `!!set`, `!!binary`, `!!timestamp` (DEF-3); Arabic-Indic digits in `registry_version` (GAP-4); non-string keys report another error kind (DEF-4); 38-digit integer reports another kind (DEF-2) | DEF-1, DEF-3, DEF-4 |
| P2 (in P1): `"5.0"`, `5.0`, `"5"`, `5.5`, `"Y"`, `1`, `2`, `1.0`, `' 5 '`, `'+5'`, `True` for integer, boolean and float fields | pydantic coercion | identical accept and refuse sets | all agree (e.g. `flying`: 31 spellings, same verdict each) | PASS |
| P3: 1,287 boundary cases (`cases4`), generated from `manifest_v0.schema.json` for all 33 models: at, below and above each `minimum`, `maximum`, `maxLength`, enum, const; absent, null, wrong type; one extra field per model | union and bound semantics, errors inside `peoples` and `catalysts` unions | same verdict, counts, paths, messages | 1,269 agree; 16 differ in error kind only (integer of 2^63 or more: `int_parsing_size` against `less_than_equal`); 2 differ in verdict (integral float of 2^63 or more: Python SCHEMA INVALID, Rust VALID) | DEF-2 |
| P4: 78 cases (`cases5`) and 24 cases (`cases6`): powers of ten from 10^18 to 10^60 as integers and as floats in `seed`, `population`, `year`, `holdings`, `start_day` | very large integers | same verdict | 26 verdict mismatches. Integers up to 127 bits agree; at 2^127 or more Rust says LOAD ERROR (exit 1) where Python says SCHEMA INVALID (exit 2) or VALID (unbounded field); floats from `9.223372036854775807e+18` up (with an explicit exponent sign) are accepted by Rust and refused by Python | DEF-2 |
| P5: 113 Registry variants x {khor, solo} (`cases3`) | `--registry` contradicting the manifest | same verdict | 59 agree (narrowed bounds, new system, extra section, different version, `1e9` bound, ...); 54 differ by design (GAP-5); one is a defect (3-element bounds, DEF-5) | DEF-5, GAP-5 |
| P6: 210 cases (`cases2`) covering every `err()`/`warn()` site of `validate_against_registry`: systems and parameters at bounds, templates and horizons, speculative worlds, focus regions, activity-resolution warnings at 1,000/100,000 persons, creature energetics (Carbone ratio, mass 310/311, metabolism 0.09/10.01), polities, all five catalyst kinds with errors, wagers, cast, households, off-map actors, vocabularies | docs/05.5 §5; `manifest_v0.py` | same | 202 agree; 7 differ in loc rendering only (dotted keys are shown in backticks by pydantic) and 1 in verdict (10^190, DEF-2) | DEF-2 (cosmetic: DEF-4) |
| P7: 1,500 mutation-fuzz cases (`fuzz.py`: inserted tokens, deletions, value swaps on all five examples) | parity on arbitrary input | same | 1,450 agree on verdict and counts, 1,412 also on every message; 50 verdict differences: 30 tab, 13 other YAML-syntax cases the reference rejects and Rust reads (GAP-1), 7 integers of 2^127 or more (DEF-2) | GAP-1, DEF-2 |
| P7b: 3,312 manifests run through `rh-cli` alone | robustness | no panic, no abort, exit code in {0,1,2} | none | PASS |
| P7c: nesting and expansion | robustness | no crash | 200,000 nested flow sequences: Rust LOAD ERROR in 15 ms, Python RecursionError. 3,000 nested block mappings: Rust handles, Python RecursionError. Alias expansion of 10^9 elements: Python and Rust both run past 60 s with no limit (GAP-6) | PASS; GAP-6 |
| R0, R1: `regdump` | T-01 "every Registry section loads"; S6, S7, T1 entries | typed structures equal the TOML | 35 of 35 sections, 0 differences; 9 sections (`artifact_kinds`, `contaminants`, `event_roles`, `mental_breaks`, `motive_edges`, `place_types`, `visual_style_cards`, `visual_vocabulary`, `vow_kinds`) load through the generic path | PASS; DEF-6 |
| R2 | "no hard-coded system list" | a new system needs no code | `X9.new` accepted in `systems`, in `fidelity_profile` and in bounds checks; `check.rs` names only `M3.agriculture`, `M5.contagion`, `S4.ecology`, exactly as the reference does | PASS |
| R3 (`regmut`, 33 mutations) | unknown fields in typed sections are refused | refuse | refused: unknown field in verb, attribute, era-state, `eras`, system, template, archetype; duplicate ids; unordered, non-finite or non-numeric bounds; unknown act classes; a top-level scalar; non-semver versions (including Unicode digits and a trailing newline). Accepted: 3-element bounds and horizons (DEF-5), typos and wrong types in `artifact_kinds`, `vow_kinds`, `mental_breaks` (DEF-6) | DEF-5, DEF-6 |
| O0 | T-03 clean specs | 0 violations | 29 columns, 4 systems, 0 violations; 11 NOTEs for owners without a spec | PASS |
| O1a..O1m | "a test system writing an unowned column is rejected" | VIOLATION, exit 1 | caught: column nobody owns, column of another system, unknown system, system absent from the Registry, case and whitespace variants, empty string; unknown access field and empty or missing access file refused with exit 3 | PASS |
| O2a..O2d | undeclared read | VIOLATION | caught; own and declared reads pass | PASS |
| O3a..O3h | plane-B read from a plane-A system (ADR-0024) | VIOLATION | caught for plane B and plane C, from derived access and from an access file; plane-B readers of plane A and B pass. Plane `D` and `b` on an owned column are accepted (DEF-8) | PASS; DEF-8 |
| O4a..O4d | column claimed by two specs | VIOLATION | caught for two systems, two files for one system, one column twice in one spec | PASS |
| O5a..O5m | spec well-formedness | as `lint_odd.py` | missing `plane`, missing `system.id`, invalid TOML, non-UTF-8: refused (exit 3). A wrong `owner` in `[[reads]]` and an unknown column `type` are accepted by the Rust lint and caught by `lint_odd.py` (DEF-8) | DEF-8 |
| S0 | T-04 "no unexplained difference" | list and justify | §4 | PASS |
| S1, S2 | schema gate sensitivity | red | a `maximum` changed in the Rust types: `SCHEMA DIFFERENCE $defs.Horizon.properties.years.maximum: 5000.0 vs 5001.0`, exit 1; in the reference: red | PASS |
| G0 | `bin/check_all.sh` clean | green | 77 PASS, exit 0 | PASS |
| G1..G4 | gate mutations | red | §5 | PASS; GAP-3 |
| M1 | corpus adequacy: remove each of the 53 `err()`/`warn()` sites of `check.rs` in turn and run the parity gate | each removal changes a verdict count | 49 killed by the shipped corpus; 4 survive it (unknown template, attribute of arity above 2, household holding not a Registry good, creature capability not permitted); all 4 are killed by the tester corpus | GAP-3 |

## 3. Defects

All reproductions run from the repository root after `cargo build -p rh-cli`. `PY` is `python -I specs/manifest/manifest_v0.py validate`, `RS` is `target/debug/rh-cli manifest validate`.

**DEF-1 (medium): a UTF-8 byte order mark is not skipped.** Clause: V-WC-04 (same verdict as the reference), SPR-004 T-02.
- Reproduction: `printf '\xef\xbb\xbf' | cat - specs/manifest/examples/solo.yaml > bom.yaml`
- Expected: `PY bom.yaml` gives `VALID (0 errors, 0 warnings)`, so Rust must too. PyYAML skips U+FEFF at index 0.
- Actual: `RS bom.yaml` gives `SCHEMA INVALID`, 1 error: `Extra inputs are not permitted` on a key named `"﻿# Reference world 0, ..."`. The BOM and the first comment line are read as one plain scalar. Windows editors add a BOM by default.

**DEF-2 (medium): numbers beyond 64 bits are handled differently.** Clause: V-WC-04; assignment "very large integers".
- (a) False accept. Reproduction: `sed 's/holdings: {grain: 400,/holdings: {grain: 1.0e+19,/' specs/manifest/examples/solo.yaml > f.yaml`. Expected: `PY` gives `SCHEMA INVALID` (exit 2, `households.0.holdings.grain`, integer out of range). Actual: `RS` gives `VALID`. The same holds for every unbounded integer field (`start_day`, `year`, `holdings`, `servants`) at `9.223372036854775807e+18` and above (`9.0e+18` agrees). Python accepts only floats that convert to a 64-bit integer.
- (b) Verdict class. Reproduction: the same file with `grain: 10000000000000000000000000000000000000000` (10^40). Expected: `PY` gives `VALID` (Python integers are unbounded). Actual: `RS` gives `LOAD ERROR ... more than 127 bits`, exit 1. For bounded fields (`seed`, `population`) Python gives `SCHEMA INVALID` (exit 2) and Rust LOAD ERROR (exit 1); the parity gate compares exit codes, so this is a mismatch if it ever appears in the corpus.
- (c) Kind. For integers between 2^63 and 2^127 in a bounded field (`seed: 99999999999999999999999999999999999999`) the count agrees, but Python reports `int_parsing_size` and Rust `less_than_equal`.

**DEF-3 (low): explicit YAML tags `!!binary`, `!!set`, `!!timestamp` fail to load.** Clause: V-WC-04.
- Reproduction: `sed 's/^  seed: 1$/  seed: !!binary MQ==/' specs/manifest/examples/solo.yaml > f.yaml`
- Expected: `PY` resolves the tag and gives `VALID`. Actual: `RS` gives `LOAD ERROR: could not determine a constructor for the tag 'tag:yaml.org,2002:!binary'`. `!!str`, `!!int`, `!!float`, `!!null`, `!!seq`, `!!map` agree. Unlikely in a hand-written manifest.

**DEF-4 (low): error kinds and locations differ for non-string keys and dotted keys (counts agree).** Clause: assignment "same error kinds".
- Reproduction: add the line `5: x` at the top level of `solo.yaml`.
- Expected: `PY` reports loc `5`, `Keys should be strings [type=invalid_key]`. Actual: `RS` reports `Extra inputs are not permitted [type=extra_forbidden]`. The same for `yes:`, `~:` and a date key. Boolean and float keys inside `systems` are printed as `True` and `1.5` against Python's `1` and `` `1.5` ``; keys that contain a dot (`M1.regions`) are printed with backticks by pydantic and without by Rust.

**DEF-5 (low): arrays that must have two elements accept more.** Clause: SPR-004 T-01 ("bounds"); `registry.rs` documents `Bounds` as a two-element array.
- Reproduction: in a copy of `registry_v0.toml` write `harvest_sigma = [0.04, 0.35, 99.0]` (or `horizon_years = [50, 3000, 9]`); run `RS specs/manifest/examples/solo.yaml --registry copy.toml`.
- Expected: the Registry is refused (exit 3). Actual: it loads and the third element is dropped without a word. (Python raises `ValueError: too many values to unpack`.) Cause: `#[serde(from = "[Number; 2]")]` ignores trailing elements.

**DEF-6 (low-medium): the saga vocabulary tables are not typed.** Clause: SPR-004 T-01 ("typed structures, with bounds"); ADR-0010 (closed vocabulary).
- Reproduction: any of these in a copy of the Registry loads without complaint:
  - `relic = { kind_strenght = 0.6, overlay = "corruption" }` (typo)
  - `relic = { kind_strength = "high", overlay = "corruption" }`
  - `relic = { kind_strength = -5.0, overlay = "nonesuch" }` (negative weight and an overlay that is not in `[overlays]`)
  - `AVENGE_KIN = { kind_weight = "x", target = "zzz" }`
  - `ASSASSINATION_ATTEMPT = { act_classes = ["harm"], relief = "lots" }`
  - `ASSASSINATION_ATTEMPT = { act_classes = ["harm"] }` (no relief)
- Expected: refused, like an unknown field in `verbs` or `attributes`. Actual: accepted, because `artifact_kinds`, `vow_kinds`, `mental_breaks`, `contaminants`, `event_roles`, `motive_edges` load through the generic path, which checks only shape, `ids` uniqueness and `act_classes` references. S6 and S7 will read these tables when they are implemented. GAP-2 asks which sections the contract means by "typed".

**DEF-7 (low): the default Registry path depends on the working directory.** Clause: parity with `manifest_v0.py` (default is relative to the script).
- Reproduction: `cd /tmp && /path/to/rh-cli manifest validate /path/to/solo.yaml`
- Expected: loads `specs/manifest/registry_v0.toml` of the repository, as `python /path/manifest_v0.py validate` does from any directory. Actual: `cannot read Registry specs/manifest/registry_v0.toml: No such file or directory`, exit 3. `bin/check_all.sh` runs from the root, so no gate is affected.

**DEF-8 (low): the Rust ownership lint does not validate the spec fields it relies on.** Clause: SPR-004 T-03; `05.1` §2; ADR-0024.
- Reproduction: in a copy of `specs/systems`, give `person.sex` in `M2.demography.odd.toml` the plane `"D"` or `"b"`; or make `S7.vows.odd.toml` declare `person.alive` with `owner = "S6.artifacts"`; or give a column the type `"wibble"`. Run `rh-cli ownership --specs copy`.
- Expected: VIOLATION. Actual: `0 violations`. A column with an unknown plane is treated as not plane B or C, so a plane-A system that reads it passes (fail-open). `specs/systems/lint_odd.py` rejects all three ("needs plane A, B or C", "reads ... but M2.demography owns it", "unknown type"), and it runs in `check_all.sh`, so the combined gate is closed. Only the Rust lint is open, and it is meant to be the lint that SPR-005 binds to code.

## 4. Schema: differences against `manifest_v0.schema.json` (T-04)

`rh-cli manifest schema --diff` normalises before comparing and prints 0 differences. Without normalisation the tester counted **237 differences in 32 definitions and the root**. None is structural: type, `required`, `properties`, `enum`, `const`, `minimum`, `maximum`, `exclusiveMinimum`, `minLength`, `maxLength`, `minItems`, `pattern`, `default`, `additionalProperties`, `discriminator` and `oneOf` are equal after normalising numbers (`150` against `150.0`) and the order of `required`.

| Key | Count | Direction | Justification |
|---|---|---|---|
| `title` | 159 | absent in Rust | Pydantic derives a title from every name. Annotation only. |
| `description` | 38 | Rust has Rust doc comments, reference has Python docstrings (some absent) | Annotation only. |
| `format` | 39 | Rust adds `int64` (21), `double` (16), `int128` (2) | schemars type hints. `format` is an annotation in JSON Schema 2020-12 unless a validator opts in; the bounds that matter are in `minimum`/`maximum`. |
| `$schema` | 1 | Rust names draft 2020-12, reference has none | Annotation only. |

Observation: annotations are dropped by key name at every depth. A manifest field that is itself called `title`, `description` or `format` would disappear from `properties` on both sides and escape the diff. No such field exists in v0.

## 5. Gate mutations (`bin/check_all.sh` on throwaway copies)

| Mutation | First red check | Verdict |
|---|---|---|
| none (clean) | none: 77 PASS | green as required |
| Parity: `CARBONE_RATIO` 0.009 to 0.0091 in the Rust check | Rust: unit tests (cargo nextest) | red |
| Parity: an error message changed in `check.rs` | Rust: unit tests (cargo nextest) | red (the parity gate alone stays green, GAP-3) |
| Parity: Rust Registry drifted (shipped positive control) | `V-WC-04 parity positive control` expects FAIL, gets it | red path live |
| Ownership: unowned-write branch removed | Rust: unit tests | red |
| Ownership: duplicate-owner detection disabled | Rust: unit tests | red |
| Ownership: plane check or undeclared-read check disabled (fmt/clippy-clean form) | Rust: unit tests | red |
| Ownership: positive-control fixture deleted | `Ownership lint positive control: unowned write must FAIL` | red |
| Ownership: positive control edited so that it passes | same check | red |
| Schema: a bound changed in the Rust types | Rust: unit tests; run alone, `schema --diff` prints `SCHEMA DIFFERENCE ... 5000.0 vs 5001.0`, exit 1 | red |
| Schema: a bound changed in `manifest_v0.schema.json` | `Manifest JSON Schema from the Rust types = manifest_v0.schema.json` | red |

For the parity gate alone (`bin/manifest_parity.py` on the shipped 21 manifests), message, path and severity-text mutations and a change of `CARBONE_RATIO` from 0.009 to 0.012 all print `PARITY: all 21 manifests agree`. Removing the 53 error and warning sites one at a time: 49 turn the gate red, 4 do not (GAP-3).

## 6. Contract gaps (for the Architect)

**GAP-1: the reference is stricter than the YAML specification, and the Rust parser follows the specification.** In 43 of 1,500 fuzz cases `manifest_v0.py` stops with a PyYAML error and `rh-cli` reads the file (7 of them reach VALID or INVALID, the rest a schema error):
- a tab after `key:`, or before a flow entry, which the specification treats as separation space: `climate: <TAB>{zone: ...}`, `titles: {authority: <TAB>Mayor}`;
- `?` inside a plain scalar in a flow mapping or sequence (`{authority: High_K?ing}`);
- an empty key written as `: value`.

CLAUDE.md and SPR-004 say the reference wins, which would make these Rust defects. The reference is wrong by the specification, so the tester files a gap. The risk is concrete: a manifest that the Rust engine loads is rejected by `manifest_v0.py` and the `CLAUDE.md` spec checks that use it. Decision needed: loosen the Rust parser to PyYAML's grammar, or declare YAML 1.2 the manifest language and move the reference to a YAML 1.2 reader such as ruamel.yaml. Recommendation (confidence 70%): declare the specification the contract and have the reference read YAML 1.2, after which these three forms join `tests/manifest_parity/` as agreeing cases.

**GAP-2: "typed section" is not defined.** `registry.rs` types 25 sections (and the version) and loads 9 generically; the sprint does not say which sections count as typed. The brief expects "unknown fields in typed sections are refused"; the generic sections carry S6 and S7 data. Decision needed: which of `artifact_kinds`, `vow_kinds`, `mental_breaks`, `contaminants`, `event_roles`, `motive_edges` must be typed. Recommendation (confidence 80%): type `artifact_kinds` (`kind_strength` in [0, 1], `overlay` must be in `[overlays]`), `vow_kinds` (`kind_weight`, `target`) and `mental_breaks` (`relief`) before S6 and S7 read them; DEF-6 closes with it.

**GAP-3: the V-WC-04 oracle compares counts only.** `docs/09` and `bin/manifest_parity.py` compare verdict, exit code, error count and warning count. A different message, a different path, a wrong severity text, or a changed energetics constant passes if the counts stay. The shipped corpus also leaves 4 of 53 check sites unexercised. Recommendation (confidence 85%): compare the ordered list of `[ERROR]`/`[WARNING]` lines (path and message) for exit 0 and 1, and the `(loc, type)` pairs for exit 2; the two validators agree line by line on 1,412 of 1,500 fuzz cases and on 3,248 of 3,414 generated manifests, so this costs almost nothing. Add the four missing cases (unknown template; household holding not in `goods`; creature `flying`/`bondable` on a variant whose archetype lacks the capability; an attribute with more than two arguments, which needs a fixture Registry).

**GAP-4: `\d` in the reference matches Unicode digits.** `registry_version: "٠.١.٠"` is VALID in `manifest_v0.py` (a warning) and SCHEMA INVALID in Rust, which uses ASCII digits. The Registry loader also uses ASCII digits. Rust is right; the reference should use `[0-9]` (slugs already use `[a-z0-9_]`).

**GAP-5: Registry strictness is a Rust-only contract.** The Rust loader refuses 54 Registry shapes that Python accepts or crashes on: inverted, non-finite or boolean bounds; infinite upper bound; integer or missing `registry_version`; missing `fidelity_levels`, `horizon_years`, `default_systems`, `capabilities`; template horizon below 1 or inverted; `ids` mixed with keyed entries; non-string, duplicate or non-semver ids; unknown fields inside typed sections; a stray top-level scalar or datetime. These are sound, but they are specified nowhere (`registry.rs` module doc only). Recommendation (confidence 90%): document the rules in `docs/05_architecture/05.5` §3 and test them with the `regmut` mutations as fixtures under `tests/lint_fixtures/registry/`.

**GAP-6: no limit on alias expansion.** A "billion laughs" file (10 levels of 10 aliases) keeps both validators busy for over 60 s. The manifest is produced by an expander (ADR-0010), so the input is trusted today; if a manifest ever arrives from outside, add a node-count limit to `yaml.rs` and note it in 05.5. Confidence 60% that it matters before P4.

**GAP-7: the default `rh-cli ownership` run checks the specs against themselves.** Without an access file the access declarations are derived from the specs (`writes = owns`), so only the two-owner and Registry rules can fire; "4 access declarations checked" does not mean four systems were checked against code. This is intended until SPR-005 (the module says so), but the gate output reads like a stronger statement than it is. Suggest the line says `0 access declarations from code (stub until SPR-005)`.

## 7. Observations (no action required)

- The Rust YAML reader reproduces PyYAML's YAML 1.1 resolver faithfully, including its quirks: `1e3` and `9.0e18` (no sign in the exponent) are strings, not numbers; `010` is octal; `1:30` is sexagesimal; `yes` is a boolean; `<<` merges, including lists of maps, and a merge key given as a quoted string is a plain key. All of these agreed.
- `rh-cli` never panicked, aborted or timed out on 3,312 generated manifests, and recursion depth is limited (15 ms for 200,000 nested brackets). Python raises `RecursionError` on the same files.
- A second spec file for one system, the same column twice in one spec and a system missing from the Registry each give a cascade of messages (up to 30 lines); the verdict is right and the first line names the cause.
- Warnings for `M5.contagion`, `S4.ecology` and `M3.agriculture` live in `check.rs` by system id, exactly as in the reference; they are not Registry data. That is the reference's design, noted for the day a second creature system appears.
- `bin/check_all.sh` needs `python/.venv` (RUN-001) and `cargo-nextest`/`cargo-deny` on `PATH`; with neither it stops at "Required tools present" in under a second, which is the intended behaviour.
