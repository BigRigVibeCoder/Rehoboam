---
id: VER-SPR-001-TEST
title: "Contract tester evidence for SPR-001"
type: reference
status: CLOSED
owner: tester
agents: [tester]
tags: [verification, testing]
related: [SPR-001, VER-SPR-001]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** V-INSP-03, V-PLN-01 and V-DET-19 pass every case the contracts name literally (the five forbidden-API families, the system-crate dependency list, plane isolation), and `bin/check_all.sh` is green on a clean copy and turns red on every gate mutation tried but one. Five defects remain, none blocking: one medium (thread-local functions of `rand` 0.9 pass every determinism gate), one low-to-medium (`check_all.sh` does not stop after a failed first check), three low (lexer and scope bypasses, two untested lint_arch features). The licence allow-list, exactly as ADR-0021 states it, will reject `thiserror` and `serde` derive: a contract conflict for the Architect, not a defect.

> **Architect's note (2026-10-16):** every defect below (DEF-1 to DEF-5), gaps 2–5 and 7, and OBSERVATION-1's cause were addressed in the SPR-001 fix round; gap 1 and gap 3 were overtaken by the director's personal-use licence ruling (ADR-0021 as amended). Disposition per item: `VER-SPR-001` §3. This document is kept as the tester's evidence at commit `39297b3`.

Tested commit: `39297b3` on `ccr-1db07854-bstsd2` (copy, remotes removed; the original was only read). Probes were written from the contracts before the implementation was read (`probe_plan.md`), then adapted to the layout. Harness: `probes.sh` with `helpers/`. Raw results: `results_run3_final.tsv` (286 results) and `results_run4_final.tsv` (206 results) in the scratchpad `ver-spr001/` directory.

## 1. Verdicts

| Gate | Verdict | Basis |
|---|---|---|
| V-INSP-03 (crate boundaries) | **PASS** | 23 lint_arch cases (AX) and 12 end-to-end fixtures (T04-D). Renamed dependencies (`package = ...`), build/dev/target-specific/optional/workspace-inherited edges and system-to-system edges are all caught. Open: DEF-5 (two features untested), GAP-4 (labels are self-declared). |
| V-PLN-01 (plane isolation) | **PASS** | Plane A to B, C, tool and transitive edges are caught. The compile-fail control is live (4 mutations). Open: GAP-7 (mechanism is a fixture, not `trybuild` as `docs/09` says; no plane-C compile-fail case). |
| V-DET-19 (forbidden APIs) | **PASS on the five named families; DEF-1, DEF-3, DEF-4 open** | 58 lexer cases, 19 escape cases, 18 scope cases, 7 pattern-neutering runs. HashMap/HashSet, `thread_rng`, `StdRng`, `mul_add`, `Instant`/`SystemTime` are caught in code, in every import form tested, and in every plane-A crate including `rh-events` and `rh-snapshot`. A tool crate is not scanned; clippy still applies to it. |
| `bin/check_all.sh` end to end | **PASS with DEF-2** | 44 checks green on a clean copy after `uv sync --frozen --project python` (RUN-001 s4-s5); idempotent; leaves the tree clean; every gate mutation turned it red except the survivors in section 3 and GAP-2; the eleven planning-phase gates are still live; a stale green report never survives a red run. DEF-2 breaks "the first failure stops the cascade" for check 1. |

## 2. Results (condensed; one row per probe family)

| Probe IDs | Contract clause | Expected | Actual | Verdict |
|---|---|---|---|---|
| BASE-1, CC-REP-BASE*, CC-ISO, CC-IDEM | GOV-002 s4, s6, laws 6, 8 | green; report has commit, branch, time, per-check result, verdict; tree untouched; same result twice | as expected, 44 checks in 25 s | PASS |
| T01-S*, T01-B1..B4 | SPR-001 T-01; GOV-008 s4 | seven stubs, exact toolchain `1.97.0` honoured, `Cargo.lock` tracked, bare `cargo nextest run` passes with tests | six tests discovered, all pass | PASS |
| T01-N1, N2 | GOV-002 law 4 | failing test turns runner and `check_all` red | red at "cargo nextest" | PASS |
| T01-U[7 crates x 2], UE | GOV-003 s2 | `unsafe`, and `#[allow(unsafe_code)] unsafe`, fail to build in all seven crates | exit 101 in all fourteen (the allow case proves `forbid`, not `deny`) | PASS |
| T01-D[6 crates], D2, D3 | GOV-003 s2 | undocumented item and missing crate doc are errors | clippy and build fail; documented control passes | PASS |
| T02-S*, T02-RUN001, T02-SYNC[A,B], T02-REPRO, T02-SPECDEPS, T02-RUN001-S5, T02-LOCKCHK | SPR-001 T-02; RUN-001 s4-s5 | locked env reproduces; 3.12; spec-check modules inside the lock | two fresh clones freeze identically (18 packages); clean tree after sync; `check_all` green from the fresh clone | PASS |
| T02-L1..L2, R1..R3, M1..M4, C1, X2 | SPR-001 T-02 | ruff and mypy strict live, no auto-fix, `specs/` in scope | all red as required; control green | PASS |
| T02-S3, T02-DR2 | SPR-001 T-02 | exact Python pin; lock drift detected | `requires-python = ">=3.12"`; drift not detected by `--frozen` | GAP-5 |
| T03-F*, C1..C4 | SPR-001 T-03 | fmt and clippy `-D warnings` gates live, test code linted, no rewrite | red for rh-core and rh-cli; file unchanged | PASS |
| T03-U1, U2 | GOV-003 s2 | `unwrap` in library code is an error, in test code allowed | red; green in `#[cfg(test)]` | PASS |
| T03-GPL, AGPL, LGPL, NC, BSL (-d, -e), XX1, XX2 | SPR-001 T-03; ADR-0021; docs/14 | forbidden licences and `xxhash-rust` rejected; MIT and MPL-2.0 accepted | direct `cargo deny` and `check_all` red; negative controls green; real crates.io `xxhash-rust` rejected; an MIT crate of that name is banned by name | PASS |
| T03-WIDEN-GPL | SPR-001 T-03 positive control | adding GPL to the allow-list turns `check_all` red | red at "cargo deny positive control: GPL" | PASS |
| T03-WIDEN-BSL, WIDEN-Unicode | ADR-0021 exactness | widening noticed | `check_all` stays green | GAP-3 |
| T03-LIC-* | ADR-0021 | allow-list equals the ADR set | exactly the eight ids; `xxhash-rust` banned by name; no legacy widening keys | PASS |
| T03-GPLDEV | ADR-0021 (AMB-09) | dev-dependency licences in scope? | a GPL dev-dependency passes cargo-deny | GAP-3 |
| DEPS-* | GOV-003 s2; ADR-0021 | the crates the contracts require are admitted | `libm`, `twox-hash`, `indexmap`, `rand`, `rand_chacha`, `proptest` admitted; `thiserror` and `serde` derive rejected (`Unicode-3.0` via `unicode-ident`) | GAP-1 |
| T04-D0..D11, SCOPE AX-01..AX-22 | V-INSP-03 | illegal edges red, legal edges green | all as expected | PASS |
| T04-P0..P7 | V-PLN-01 | plane A to B, C, intermediate crate red; plane C to A green | all as expected | PASS |
| T04-TB1..TB4 | SPR-001 compliance row (V-PLN-01) | the compile-fail control is not vacuous and fails for its stated reason | making the bad case compile, adding the edge, or breaking it with a syntax error all turn `check_all` red | PASS |
| T04-F0..F3, F5, F6 | V-DET-19 | each named family red in rh-core and every other plane-A crate; unclassified crate fail-closed; alias import caught | all red; BTreeMap control green | PASS |
| T04-NEU[7 tokens] | SPR-001 T-04 acceptance | removing a pattern from the gate turns `check_all` red | all seven red | PASS |
| LEX-01..32, 40, 41, 45..49, 52..56, 58 | V-DET-19 | lexer: no false negative after raw strings, `'"'`, lifetimes, nested comments, `b'`/`c"`/`br#` prefixes | all as expected | PASS |
| ESC-01..15, 18, 19 | SPR-001 T-04 (escape needs a reason) | same line, line above, missing reason, missing colon, blank reason, unused, wrong line, other gate name, block comment, string: handled | all as expected (ESC-16, 17 recorded as INFO) | PASS |
| SF-01..12, 15, 16, 18 | V-DET-19 scope | tests/, benches/, examples/, build.rs, nested modules scanned; tool, plane B and C skipped; undeclared plane scanned; non-UTF-8 is an error | all as expected | PASS |
| T05-W*, X1, X2 | SPR-001 T-05; GOV-008 s1 | hosted Ubuntu runner, caches, prebuilt tools, report upload with `always()`, all gates reachable, mode 100755 | all PASS except the timeout (GAP-6) | PASS |
| T05-H1..H6 | SPR-001 T-05 | installer, idempotent, clean push passes (25 s), rustfmt violation blocked and never reaches the remote | all PASS | PASS |
| CC-FF1..FF3, CC-ST1, CC-CWD, CC-KEEP, CC-TOOL[4], CC-TOOL-PY, CC-R1..R6 | GOV-002 laws 4-5; SPR-001 compliance | fmt plus failing test gives exactly one FAIL and no test run; stale report replaced; runnable from any directory; missing tools turn it red; all eleven planning-phase gates still red when broken | all as expected | PASS |
| CC-FF4 | GOV-002 law 5 | a failed check 1 stops the cascade | cascade continues | **FAIL (DEF-2)** |
| T04-RAND_RANGE, BOOL, RATIO; LEX-38, 39 | GOV-003 s2; V-DET-19 | thread-local `rand` 0.9 functions red | `check_all` green | **FAIL (DEF-1)** |
| LEX-33, 34, 36, 37; SF-13, 14 | V-DET-19 | no false negative from keyword-adjacent quotes, spaced paths, outside-crate modules | not flagged | **FAIL (DEF-3, DEF-4)** |
| CTRL-A12, A13 | SPR-001 T-04 acceptance | breaking build-edge or alias handling turns `check_all` red | stays green | **FAIL (DEF-5)** |
| CC-LEAVE | GOV-002 law 6 | nothing left outside the repo | a stray `/Cargo.toml` exists (created 03:11 during the Developer session, not by `check_all`) | OBSERVATION-1 |

Positive-control neutering (`CTRL-*`, 64 mutations): 54 turned `check_all` red, 2 survived at MUST level (A12, A13: DEF-5) and 8 at rule level (GAP-2). Of the 54 red results, 23 were caught by a named positive control, 19 by `pytest`, 2 by the lint's own run on the real workspace and 10 by `ruff`, `mypy` or `ruff format` (see Not run, item 1).

## 3. Defects

| ID | Clause | Reproduction | Expected | Actual | Severity |
|---|---|---|---|---|---|
| DEF-1 | GOV-003 s2 ("no thread-local or global RNG"); V-DET-19 | Add `rand = "0.9"` to `crates/rh-core/Cargo.toml` (cargo-deny admits it) and a documented `#[must_use] pub fn f() -> u32 { rand::random_range(0..10) }`; run `bin/check_all.sh`. Same for `random_bool`, `random_ratio`. | red | exit 0 (44 checks green). `rand-random` matches only `\brand::random\b`; `clippy.toml` does not list these functions. | Medium. Dormant until `rand` is added (SPR-002). |
| DEF-2 | GOV-002 law 5, s6 | `PYTHON=/usr/bin/python3 bin/check_all.sh`. Also with `cargo-nextest` or `cargo-clippy` absent from `CARGO_HOME`. | check 1 "Required tools present" fails and nothing else runs; verdict names it | checks 2 and 3 pass, check 4 fails, the verdict names check 4 and says "later checks skipped"; with nextest absent the script ends `forbidden_api_controls: command not found`, exit 127. Cause: the `forbidden_api_controls()` definition sits between `&&` links and ends the and-or list. | Low to medium. Always red, never falsely green, but wrong verdict, wrong exit code, wasted run. |
| DEF-3 | V-DET-19 | `python -I bin/lint_forbidden.py --scan f.rs` with `fn f() -> char { return'"'; }` on line 1, `Instant::now()` on line 2 and any later `"`. Also `return"//"; ... HashMap` on one line. | flagged | exit 0. A quote right after an identifier character is lexed as code, so a char literal opens a bogus string. | Low. rustfmt rewrites the spacing; `#[rustfmt::skip]` does not. |
| DEF-4 | V-DET-19 | (a) macro body `std :: time :: Instant :: now()` or `rand :: rng()`; (b) `#[path = "../../shared/bad.rs"] mod bad;` or `include!` of a file outside the crate directory | flagged | not flagged. clippy backs up std paths in (a) and (b); nothing backs up `rand`. | Low |
| DEF-5 | SPR-001 T-04 acceptance; lint_arch docstring ("every dependency kind") | In `bin/lint_arch.py` skip `kind == "build"` edges, or prefer `rename` over `name`; run `bin/check_all.sh` | red | green. Behaviour is correct today (AX-01..03, AX-17 pass); no fixture or unit test would notice a regression. | Low (test gap) |

## 4. Contract gaps (for the Architect; none is a defect)

1. **ADR-0021 versus GOV-003.** `thiserror` (GOV-003 s2) and `serde` derive need `Unicode-3.0` through `unicode-ident`; the exact allow-list rejects them. The first sprint to add either turns the licence gate red. Recommendation: a `[[licenses.exceptions]]` entry for `unicode-ident`, or an ADR-0021 amendment; confidence high. ADR-0021 is still "Proposed".
2. **Control coverage below rule level.** Survivors: `SmallRng`, `from_entropy`, `from_os_rng`, `getrandom` spellings; the explicit system-to-system branch (the "allowed dependency" branch also catches the fixture); removing a real-workspace run from `check_all.sh` while its fixture control stays.
3. **Allow-list widening by an off-list licence** (`BSL-1.0`, `Unicode-3.0`) and a **GPL dev-dependency** are invisible to `check_all`; only my static comparison sees the first.
4. **Self-declared labels.** A PR can relabel `rh-snapshot` as `kind = "contract"` and let a system crate depend on it (AX-23). Only the C2 change class guards this, and no CI step detects it.
5. **Python pin.** `requires-python = ">=3.12"`; `--frozen` cannot detect `pyproject.toml`/`uv.lock` drift (`uv lock --check` can).
6. **CI.** `timeout-minutes: 20` against a 15-minute target; the pre-push hook runs the full tier (25 s warm), where SPR-001 says "fast tier"; actions are pinned by tag.
7. **`docs/09` V-PLN-01** names `trybuild` and "each plane-B and plane-C table"; the sprint uses one fixture workspace for plane B only. Sprint compliance row "docs/09 rows if tests change" is open.
8. **Not owned by any sprint I can see:** float types in threshold positions (V-DET-19), column-ownership lint, coverage gates (GOV-002 s3).

**OBSERVATION-1.** A stray `/Cargo.toml` at the filesystem root (workspace `rh-sys-a`, `rh-sys-b`) breaks `cargo fmt --all` for any path crate under `/`. It looks like a misplaced copy of `tests/lint_fixtures/arch/system_to_system/Cargo.toml`. `check_all` does not recreate it. Please delete it.

## 5. Not run, or weaker than stated

1. Ten neutering mutations went red at `ruff`, `mypy` or `ruff format` rather than at a control (A01, A05, F01, F04..F06, F27, F29, F31, F34); for A01 and F01 (file deleted) that is unavoidable. Tool-clean versions are in `helpers/ctrl_matrix.py` but were not re-run (time-box). Survivors are unaffected.
2. `T04-F4c/F4d/F4e` (tool-crate opt-out with and without a reason) are inconclusive: my `dead_code` shim had no reason, which the project's own `allow_attributes_without_reason` lint rightly rejects. `T04-F4d` FAIL is a harness artifact. Fixed in `probes.sh`, not re-run.
3. Per-crate end-to-end matrices ran for rh-core and rh-cli only; the other five crates ran as direct builds and clippy (`--e2e all` not run).
4. GitHub-side items (15-minute run, merge protection on the Free plan, cache hit, artifact download, `advisories.yml` schedule) and T-00 on the director's laptop: Architect-verified (ARCH-1..4).
5. `probes.sh` was edited while a run was executing; that run started a second pass and truncated its table. The two runs reported here started from a frozen copy and were not edited.
