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

## 8. Re-verification at 5f3939d

Tested commit: `5f3939d` (the rebased sprint branch, on `main` 8283f5e). The black-box harness of the first round ran unchanged against `rh-cli` (`diff.py`, `cases1..6.py`, `fuzz.py`, `regmut.py`, `own.py`, `regdump.py`), plus five new probe sets for the open items: `cases7` (63 tab, `?` and empty-key placements), `cases8` (33 `?` and empty-key forms), `cases9` and `cases10` (those forms in a nested context), `cases11` (64 non-string and datetime keys). The reference is the changed one (`Int64` bounds, `[0-9]` version pattern). On a clean checkout of the head `bin/check_all.sh` is green with 102 checks.

### 8.1 Verdict per item

| # | Item | Verdict | Basis |
|---|---|---|---|
| 1 | DEF-1 BOM | **FIXED** | `yaml_bom`, with and without a non-ASCII character after it, agrees with the reference (VALID); the shipped corpus has `valid_bom.yaml`. |
| 2 | DEF-2 (a) floats beyond i64, (b) integers beyond 2^127, (c) error kinds | **FIXED** | Boundary sweep from the new schema (1,293 cases), the power-of-ten sweep (78) and the float sweep from `9.0e+18` to `1.0e+308` (24): 0 verdict differences and 0 message differences. Integers of 2^127 and above now give `SCHEMA INVALID`/`VALID` as Python does; the `int_parsing_size` kind matches. |
| 3 | DEF-4 keys | **FIXED, one cosmetic remainder that does not matter** | Integer, float, boolean, null, date, sexagesimal and dotted keys: kind and location text agree (64 cases). Datetime keys with a time part (`2020-01-01 10:00:00: x`, ISO forms with `T`, `Z`, offsets) agree on verdict, count and kind, but the location text differs: Python prints the repr ``datetime.datetime(2020, 1, 1, 10, 0)``, Rust prints the source text. It does not matter for users, because the manifest is invalid either way and the message still names the key. It would matter for the parity oracle only if such a key were added to `tests/manifest_parity/`; do not add one, or normalise the loc. Invalid dates (`2020-02-30`) crash the reference with `ValueError` and give `SCHEMA INVALID` in Rust: a reference crash, no action. |
| 4 | DEF-5 | **FIXED** | `harvest_sigma = [0.04, 0.35, 99.0]` and `horizon_years = [50, 3000, 9]` are refused (exit 3). Registry round trip: 35 of 35 sections, 0 differences against `tomllib`; S6, S7 and T1 params intact. |
| 5 | DEF-7 | **FIXED** | From `/tmp` and from an empty directory, `manifest validate`, `ownership` and `manifest schema --diff` find the repository's Registry and specs. |
| 6 | DEF-8 | **FIXED** (one residual, not on the list) | Plane `D` and `b` give `VIOLATION [plane]`; a wrong `owner` gives `VIOLATION [single-writer] ... reads person.alive from S6.artifacts, but M2.demography owns it`; an unknown column type gives `VIOLATION [spec]`. 41 of 43 ownership probes behave; of the other two, one is my own wrong expectation (a non-UTF-8 spec is refused with exit 3, which is right), the other is the residual: a `[[reads]]` entry that names a real owner which does not declare the column (`person.ghost` from M2) is accepted by the Rust lint, and `lint_odd.py` still catches it. Observation, not a defect. |
| 7 | GAP-1 | **NOT CLOSED: DEF-9 (medium), DEF-10 (low)** | See 8.2. Tabs are closed. The `?` and empty-key refusals are bypassed by any non-ASCII character earlier in the file. |
| 8 | GAP-3 | **FIXED** | The parity gate now prints line-level `PARITY MISMATCH` and agrees line for line on the clean head (29 manifests). All mutations that were green are now red: `CARBONE_RATIO` 0.009 to 0.0091 and to 0.012, a changed error message, a changed warning text, a changed error path, and a warning promoted to an error. Removing each of the 53 `err()`/`warn()` sites in turn: 53 of 53 turn the shipped gate red (49 of 53 before). Unmodified code agrees. |
| 9 | GAP-4, GAP-7 | **FIXED** | `registry_version: "٠.١.٠"` now agrees (both reject; the shipped corpus has `schema_unicode_version.yaml`). The ownership summary reads `0 access declarations from code (derived from specs until SPR-005: 4; from files: 0)`. |
| 11 | `lint_arch` controls for `rh-manifest` | **PASS** | Both shipped controls are in `check_all.sh` and green. Independently, a plane-A crate (`rh-core`, and `rh-registry`) depending on `rh-manifest` gives `VIOLATION [plane-A-closure]` as a normal, dev-, build-, renamed (`package = "rh-manifest"`), target-specific and optional dependency. The pin: `rh-manifest` declared as plane `A`, plane `B`, kind `machinery`, or with its metadata removed gives `VIOLATION [metadata]`. The allowed edge `rh-manifest` to `rh-core` passes, and the clean workspace reports 8 crates, 0 violations. `lint_forbidden` scans 6 crates as plane A, 0 violations. |

### 8.2 New defects

**DEF-9 (medium): the PyYAML refusals for `?` in flow scalars and for empty keys only work in files that are pure ASCII up to the offending form.** Clause: GAP-1 ruling (Rust refuses the forms PyYAML refuses); V-WC-04.
- Reproduction: `{ printf '# \xc3\xa9\n'; sed 's/  raster_resolution: 64/  : raster_resolution: 64/' specs/manifest/examples/solo.yaml; } > f.yaml`
- Expected: `PY f.yaml` stops with `ParserError ... expected <block end>, but found ':'` (exit 1), so `RS f.yaml` must give `LOAD ERROR: found an empty key written ': value'` (exit 1), as it does for the same file without the first line.
- Actual: `RS f.yaml` reads the file and gives `SCHEMA INVALID` (exit 2). The same happens for `{years: 80, label: his?t}` (expected `LOAD ERROR: found '?' inside the plain scalar`). With a non-ASCII character anywhere before the form, the check is silent; with the character after the form, it fires. Tabs are not affected.
- Likely cause (read from `yaml.rs` lines 150-155, 235, 250): `span.start.index()` from the parser is passed as a position into `self.text.as_bytes()` (`bytes.get(start)` decides whether a collection is a flow collection, and `implicit_empty_key(start)` looks at the text). If the parser's index counts characters, every multi-byte character before the form shifts the position, `flow` is pushed `false`, and `in_flow` is wrong. I did not step through it.
- Impact: all four shipped example manifests (`val_khor`, `aethelgard`, `uz_job`, `invalid_draft_style`) contain non-ASCII text, so real manifests are in the affected class. The shipped corpus does not catch it because its only non-ASCII fixtures (`valid_bom.yaml`, `schema_unicode_version.yaml`) do not contain these forms. 12 of the 1,500 fuzz cases still differ for this reason (fz0045, 0068, 0366, 0451, 0636, 0737, 0846, 0906, 0945, 1046, 1084, 1213). Add one non-ASCII fixture per form to `tests/manifest_parity/`.

**DEF-10 (low): a plain scalar that starts with `?` is accepted.** Clause: GAP-1 ruling.
- Reproduction: in `solo.yaml` replace `epoch_label: "Test era"` with `epoch_label: ?x` (block), or `label: historical` with `label: ?historical` inside the horizon flow mapping, or `flags: [protagonist]` with `flags: [?protagonist]`.
- Expected: `PY` stops with a ParserError in the first two (`?` cannot start a plain scalar there) and reads the flow-sequence case as a one-pair mapping `{protagonist: null}`; `RS` should refuse the first two (exit 1) and give `string_type` for the third.
- Actual: `RS` reads the first as the string `?x` (VALID, exit 0), the second as a schema error, and the third as a string (kind `string_pattern_mismatch` instead of `string_type`). A bare `?` and `?:x` behave the same. Mid-scalar `?` (`histo?rical`, `x?y`, trailing `x?`) is correct in ASCII files.

### 8.3 Regression

| Set | Cases | Verdict or count differs | Message differs only | Agree on every line |
|---|---|---|---|---|
| `cases1` (YAML 1.1, coercion, structure) | 315 | 4 (`!!set`, `!!binary`, `!!timestamp`; `!!int abc`, which crashes the reference) | 0 | 311 |
| `cases2` (every semantic check) | 210 | 0 | 1 (list shown as `[...]`) | 209 |
| `cases4` (bounds sweep from the changed schema) | 1,293 | 0 | 0 | 1,293 |
| `cases5`, `cases6` (very large numbers) | 78 + 24 | 0 | 0 | 102 |
| `fuzz` (1,500 mutation cases) | 1,500 | 12 (DEF-9) | 2 (a date shown as `date(...)`; `[?x]`, DEF-10) | 1,486 |
| **Original total** | **3,420** (was 3,414; the reference schema gained 6 boundary cases) | **16** (was 84) | **3** (was 82) | **3,401** (was 3,248) |

Verdict and counts agree on 3,404 of 3,420 (3,330 of 3,414 before). The 16 divergences by class: 4 explicit YAML tags (DEF-3, deferred; one of them is a reference crash) and 12 the DEF-9 bypass. The 3 message-only differences: the Python repr of a list or date inside a tag message (2, cosmetic) and DEF-10 (1). New probe sets: `cases7` 63 cases, 2 verdict differences (DEF-10: leading `?`); all 28 tab placements agree, including a tab after a quoted scalar before a comment, after a flow comma, before `]`, after an anchor, a tag, `---` or `|`, and inside plain, single- and double-quoted scalars. `cases8`: 33 cases, 5 differences (DEF-10 and leading `?`, all in ASCII files). Registry variants (113): 52 differ by design (GAP-5, now documented in 05.5 §3.1), 2 fewer than before because the 3-element bounds are refused. `regmut`: DEF-5 closed; `artifact_kinds`, `vow_kinds` and `mental_breaks` still load with typos (DEF-6, deferred).

### 8.4 Next step

DEF-9 and DEF-10 need a Developer fix and fixtures (non-ASCII header, one per form). After that, the items to repeat are `fuzz.py` (expect 0 verdict differences apart from the explicit tags) and `cases7`/`cases8`/`cases9`/`cases10`.

## 9. Re-verification at 58aa4d8

Tested commit: `58aa4d8` (on top of `926d06b`). The harness of the earlier rounds ran unchanged against `rh-cli`. New probe sets: `cases12` (232 cases: 22 forms in sequence, mapping and block context, each with no prefix, a 2-, 3- and 4-byte prefix line, and a non-ASCII quoted key on the same line before the form), `cases13` (20 cases: BOM, CRLF, CR, 500 four-byte characters, combining marks, a ZWJ sequence, non-ASCII after the form), and `fuzz2` (1,500 mutations that insert non-ASCII text, YAML line-break characters and the `?`, `:` and tab forms). On a clean checkout of the head `bin/check_all.sh` is green with 102 checks.

### 9.1 Verdict per item

| # | Item | Verdict | Basis |
|---|---|---|---|
| 1 | DEF-9 (`?` in flow scalars, empty keys, after non-ASCII text) | **FIXED** | `cases12`: 232 of 232 agree with the reference on verdict, counts and every line (148 load errors, 68 schema errors, 16 valid, so the set is not vacuous). Prefixes of 2, 3 and 4 bytes, a non-ASCII quoted key on the same line before the form, a non-ASCII character after the form, and BOM, CRLF, CR, 500 four-byte characters, combining marks and a ZWJ sequence (`cases13`, 20 of 20) all behave. `cases7` to `cases10` (119 cases, rerun): 0 differences. Tabs: all 28 placements still agree. |
| 2 | DEF-10 (`?` at the start of a scalar) | **FIXED; my §8 claim was wrong for block context** | I checked the Developer's correction against PyYAML directly. Block value `a: ?x` is the string `?x` (no error); so are `?x: 1` (key) and `- ?x` (sequence item); `a: ? x` and `a: ?` are scanner errors; in flow, `{a: ?x}`, `{a: x?}`, `{a: ??x}`, `{a: ?:x}`, `{a: ?}` are parser errors, `{?x: 1}` is `{'x': 1}`, and `[?x]`, `[? x]` and `[?]` read a one-entry mapping. My `epoch_label: ?x` probe sat inside the flow mapping `calendar`, which is the flow-value case. Rust now matches PyYAML in all of these contexts (`cases12`, `cases8`, 0 differences). Retract the block-context part of DEF-10. |
| 3 | DEF-8 residual | **FIXED** | A `[[reads]]` entry whose named owner has a spec that does not declare the column: `VIOLATION [single-writer] S7.vows: reads person.ghost, which M2.demography's spec does not declare in [[owns]]`. An owner with no spec stays a NOTE (exit 0). A real column with the wrong owner, with an owner that has no spec, and with an unknown owner system, all give a violation. 42 of 43 ownership probes pass; the 43rd (a non-UTF-8 spec) is refused with exit 3, which is right. |
| 4 | Regression | **PASS, one new class (DEF-11)** | Below. |

### 9.2 New defect

**DEF-11 (low-medium): the YAML 1.1 line-break characters U+0085 (NEL), U+2028 and U+2029 are ordinary text in Rust and line breaks in PyYAML.** Clause: the GAP-1 ruling (both validators keep YAML 1.1); V-WC-04.
- Reproduction: `sed $'s/  seed: 1$/  seed: 1\xe2\x80\xa8/' specs/manifest/examples/solo.yaml > f.yaml` (LS after `seed: 1`; the same with `\xe2\x80\xa9` and `\xc2\x85`).
- Expected: `PY f.yaml` gives `VALID`; Rust must too.
- Actual: `RS f.yaml` gives `LOAD ERROR: YAML syntax error: mapping values are not allowed ...`, because the character is part of the scalar `1<LS>` and the next line's key is read as inside it. The reverse also occurs: the character in the middle of a plain scalar (`display_name: Solo<LS>world`) is a PyYAML scanner error (it breaks the line, leaving a key without a colon) and a valid string in Rust; the character inside a comment ends the comment in PyYAML (the rest of the comment becomes text and gives `SCHEMA INVALID`) and is ignored by Rust (`VALID`). Inside a double-quoted scalar both read the file.
- Extent: all 102 verdict differences in `fuzz2` (1,500 cases) contain at least one of the three characters; no case without them differs. 5 of the 6 message-only differences contain one as well. In the shipped examples none appears. The risk is prose pasted from a word processor or a web page, which does contain U+2028 and U+0085 from time to time.
- Fix options: either have the reader normalise NEL, LS and PS to `\n` before parsing (PyYAML's reader treats them as breaks everywhere), or refuse them with a `LOAD ERROR` so the manifest does not silently mean something else. Then add one fixture per character in each of the four positions above to `tests/manifest_parity/`.

### 9.3 Regression

| Set | Cases | Verdict or count differs | Message differs only | Agree on every line |
|---|---|---|---|---|
| `cases1` | 315 | 4 (`!!set`, `!!binary`, `!!timestamp`, `!!int abc`: DEF-3, deferred; the last crashes the reference) | 0 | 311 |
| `cases2` | 210 | 0 | 1 (list shown as `[...]`) | 209 |
| `cases4` (bounds sweep from the current schema) | 1,293 | 0 | 0 | 1,293 |
| `cases5`, `cases6` (very large numbers) | 102 | 0 | 0 | 102 |
| `fuzz` (ASCII mutations) | 1,500 | 0 (was 12) | 1 (a date shown as `date(...)`) | 1,499 |
| **Original total** | **3,420** | **4** (was 16) | **2** (was 3) | **3,414** (was 3,401) |
| `fuzz2` (non-ASCII, line-break and `?` mutations) | 1,500 | 102 (all DEF-11) | 6 (5 DEF-11, 1 a dict shown as `{...}`) | 1,392 |

Verdict and counts agree on 3,416 of the 3,420 original manifests. The 4 remaining are the explicit YAML tags of DEF-3, deferred. New sets: `cases7` 63, `cases8` 33, `cases9` 11, `cases10` 12, `cases12` 232, `cases13` 20: 0 differences in any. `cases11` (64 key forms): unchanged from §8 (datetime-key location text, and invalid dates that crash the reference). Registry variants (113): 52 differ by design (GAP-5), unchanged; Registry round trip 35 of 35 sections with 0 differences.

### 9.4 Next step

DEF-11 needs a Developer decision (normalise or refuse) and fixtures. After that, repeat `fuzz2` (expect 0 verdict differences) and `cases12`. DEF-3, DEF-6/GAP-2 and GAP-6 stay deferred as ruled.

## 10. Re-verification at 3559bf5

Tested commit: `3559bf5` (on top of `086387b`). PR #6 conflicts with `main`, so CI did not run on it; `bin/check_all.sh` was run locally on a clean checkout of the head: **PASS, 102 checks** (including the 49 `tests/manifest_parity` fixtures, 15 of them new per-character, per-position line-break fixtures). New probe sets: `cases14` (103 cases: NEL, LS and PS in 34 positions each, including after a value, after a key, before a key, on a blank line, in a plain scalar and a plain key, in a comment, in double- and single-quoted scalars (also at the end, escaped, and several in a row), in flow mappings and sequences, in `|` and `>` scalars, after `-`, `---` and an anchor, at the start and end of the file, mixed with CRLF), `cases15` (10) and `cases16` (11) (the same constructs written with an ordinary newline, to separate what is new from what is not). `fuzz2` and `cases12` were rerun.

### 10.1 Verdict

| Item | Verdict | Basis |
|---|---|---|
| DEF-11 (NEL, LS, PS as YAML 1.1 line breaks) | **FIXED** | After a value, after or before a key, on a blank line, in a plain scalar, in a plain key, in a comment (also at the end of a line comment), in `\|` and `>` scalars, after `-`, `---` and an anchor, at the start and end of the file, mixed with CRLF, and `mixed_all`: 76 of 76 positions agree with the reference on verdict, counts and every line, for each of the three characters. `cases12` (232 cases) still 0 differences. `fuzz2`: verdict differences fell from 102 to 41, and none of the 41 is a line-break character read as text. |
| Local `check_all.sh` | **PASS** | 102 checks. |

### 10.2 What remains in `fuzz2` and `cases14`: two grammar differences that do not depend on the fix

The 41 `fuzz2` differences, and the 27 `cases14` differences, all fall into two classes. Both reproduce with an ordinary newline (`cases15`, `cases16`) and on the earlier heads `5f3939d` and `58aa4d8` (same results), so DEF-11 did not cause them; making NEL, LS and PS real line breaks only lets a fuzzer reach them more often.

**DEF-12 (medium-low): a flow collection or a quoted scalar that continues on a line indented no more than its parent key is a load error in Rust and valid in PyYAML.** Clause: V-WC-04; the GAP-1 ruling (YAML 1.1 as PyYAML reads it).
- Reproduction (ordinary newline, no special character): in `specs/manifest/examples/solo.yaml` replace `display_name: "Solo test world"` with `display_name: "Solo<newline>world"` (continuation at column 0), or `systems: {}` with `systems: {<newline>}`, or `{years: 80, label: historical}` with `{years: 80,<newline>label: historical}`.
- Expected: `PY` gives `VALID`. Actual: `RS` gives `LOAD ERROR: YAML syntax error: invalid indentation in quoted scalar` or `invalid indentation`. A continuation indented by 4 (`"Solo<newline>    world"`) agrees. A single-quoted scalar, a flow mapping and a flow sequence behave the same; `institutions: [<newline>  standing_army<newline>]` agrees. 6 of 10 `cases15` forms differ.
- Extent: 25 of the 41 `fuzz2` differences and 27 of 103 `cases14` cases (the quoted-scalar and flow positions of each character). A multi-line flow mapping with its closing brace at column 0 is a plausible way to write a long `systems` or `fidelity_profile` block.
- Fix: relax the indentation check for continuation lines of flow collections and quoted scalars (PyYAML does not enforce it), or refuse the form in both validators; the reference is the contract, so relaxing is the consistent choice.

**DEF-13 (low): a line break inside a plain scalar that is an implicit key of a flow mapping is accepted by Rust and refused by PyYAML.** Clause: V-WC-04.
- Reproduction (ordinary newline): replace `{years: 80, label: historical}` with `{years: 80, la<newline>   bel: historical}`.
- Expected: `PY` stops with `ParserError: while parsing a flow mapping` (exit 1), so `RS` must give a `LOAD ERROR`. Actual: `RS` reads the key as `la bel` and gives `SCHEMA INVALID` (exit 2). The same with a quoted key (`"la<newline>   bel"`), a key broken after its first letters, and a key spanning two breaks; 7 of 11 `cases16` forms differ. Values and sequence items with a line break (`label: histor<newline>   ical`, `[protag<newline>   onist]`) agree, and so do multi-line keys in block mappings.
- Extent: 16 of the 41 `fuzz2` differences. Low, because a key split across lines in a flow mapping is a typing accident, and the manifest is invalid either way (the key is unknown); it only changes the exit code and the message.

The remaining `fuzz2` message-only difference is cosmetic (a dict shown as `{...}` in a union-tag message).

### 10.3 Regression

The 3,420 original manifests and the 1,500 ASCII fuzz cases were not rerun on this head: `3559bf5` changes only line-break handling in `yaml.rs` and its fixtures, and the 102-check run includes the 49-fixture parity gate. The earlier sets that exercise this code were rerun: `cases12` 232 and `cases14` 103 (above), `cases15` and `cases16` on three heads. If the Architect wants the full 3,420 and 1,500 repeated after the DEF-12 and DEF-13 fixes, I will run them then.

### 10.4 Next step

DEF-11 is closed. DEF-12 and DEF-13 are not regressions and do not block G0 on their own, but DEF-12 is the one a hand-written manifest can hit (a multi-line flow mapping with a closing brace at the left margin). Suggested: fix DEF-12 with a fixture per construct, decide DEF-13 (fix or accept as a known divergence in `tests/manifest_parity/README.md`), then repeat `cases14` to `cases16` and `fuzz2` (expect 0 verdict differences, apart from the explicit tags of DEF-3).
