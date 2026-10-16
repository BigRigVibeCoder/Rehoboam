---
id: GOV-002
title: "Testing protocol for a deterministic simulation"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, testing]
related: [GOV-005, GOV-007, ADR-0032, ADR-0006]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** The laws of the director's GOV-002 are kept: proof, not claims; no report means no pass; regression test first; flaky means broken; trace everything. The 17 tiers are mapped onto a headless deterministic simulation, and the five tiers that matter most here are added: determinism, observer invariance, statistical validation with held-out bands, golden vectors, and pre-registered human evaluation. Tests run in three cadences: every PR, nightly, and at gates. `docs/09` is the test catalogue; this document is the policy.

# GOV-002 — Testing protocol

## 1. The laws (kept from the template)

1. **Trust no return value.** Query state to prove it.
2. **The artifact is the proof.** A run produces a report, not just "PASSED".
3. **Zero-error tolerance.** Any `ERROR` log during a test fails it.
4. **No report, no pass.**
5. **Fail fast.** Run the fastest tiers first; the first failure stops the cascade.
6. **Isolation.** Each test starts clean and leaves nothing behind.
7. **Trace everything.** Every test names the requirement (`docs/03`) or contract it verifies, and every requirement has a test (`docs/09` matrix).
8. **Flaky = broken.** Quarantine at once; fix or delete within one sprint.
9. **Regression first.** A bug fix starts with a failing test that reproduces it, and that test stays forever.

**Added for Rehoboam:**

10. **Determinism is a test, not a hope.** Every PR runs the hash ladders. A hash change is a declared C3 change, or a bug.
11. **Held-out stays held out.** Held-out bands are sealed before tuning and evaluated only in sealed runs. Exposing one is a P0 incident.
12. **Oracles beat opinions.** Hashes, golden vectors, SQL twins and bands decide; reviewers' impressions do not.

## 2. Tiers and cadence

| Tier | Rehoboam form | Cadence |
|---|---|---|
| 1. Static | `cargo fmt`, `clippy -D warnings`, `cargo-deny` (licences, ADR-0021), `ruff`, `mypy --strict`, `tsc`; architecture lints (dependency graph, plane isolation, column ownership); manifest validator; pattern linter; **system-spec linter** (`lint_odd.py`); MANIFEST staleness | Every PR |
| 2. Unit | Every public function; < 100 ms each | Every PR |
| 3. Property | proptest / Hypothesis invariants: **conservation** of people, grain and coin; no negative stocks; plane isolation; RNG purpose independence | Every PR |
| 4. Snapshot | `insta` snapshots of stream records, schemas, generated briefs and code skeletons | Every PR |
| 5. Golden vectors | Literature numbers as tests: Brass, the M4 basket, Davenant; sampler known answers | Every PR |
| **6. Determinism** | Hash ladders at 1/2/4/N threads; replay from the journal; keyframe round-trip; **stream determinism** (V-STR-01) | Every PR (short worlds); nightly (full length, cross-machine) |
| **7. Observer invariance** | Viewers and console fuzz leave `state_hash` unchanged (V-DET-06–08); harness isolation (V-PERF-09) | Every PR from P2 |
| 8. Integration and contract | ICD conformance (V-IF); **SQL-twin equivalence** (V-SIFT-01); stream contracts (V-STR); consumer contract tests of changed contract crates | Every PR |
| 9. End-to-end | **The complexity ladder:** `solo` on every PR; higher rungs nightly | Every PR / nightly |
| 10. Performance | criterion benches; > 10% slower fails (V-PERF-04, PERF-4) | Every PR (micro); nightly (reference desktop) |
| 11. Mutation | `cargo-mutants` on `rh-core`, the Registry evaluator, RNG and fixed-point primitives; target ≥ 80% kill | Nightly; a regression blocks the next merge to those crates |
| 12. Fuzz | `cargo-fuzz` on journal, stream, manifest and pattern parsers | Nightly |
| **13. Statistical validation** | Stylized-fact bands, CRN-paired ensembles, submodel tournaments (`09` §3.1, §3.8); held-out bands only in sealed runs | 32-seed ensemble on every C3 PR; full certification nightly and at gates |
| 14. Soak | 1,000-year runs: memory, stream growth, keyframe store | Weekly |
| 15. Crash consistency | `kill -9` during keyframe or segment writes; resume with identical hashes | Nightly |
| **16. Human evaluation** | Rung checkpoints (the director watches the stream); **G2/G3b pre-registered ratings** (`09` §3.4.1) | Rungs and gates |
| Not in phase 1 | DAST and pen testing (localhost only); GUI/Playwright (returns with the web console after G2, V-TM); i18n; IaC (returns with the cloud move); hardware-in-the-loop | — |

## 3. Coverage thresholds

| Metric | Threshold | Applies to | Enforcement |
|---|---|---|---|
| Line coverage | ≥ 80% | `rh-core`, `rh-sys-*`, `rh-registry`, Python tooling | CI blocks merge |
| Branch coverage | ≥ 75% | Same | CI blocks merge |
| Public functions with a test | 100% | Authoritative crates | CI blocks merge |
| Mutation kill rate | ≥ 80% target; reported | Determinism primitives, Registry evaluator | Nightly; regression blocks |

**Not adopted:** MC/DC (aviation-grade, not needed here), and "≥ 2 assertions per test" (easily gamed; property tests serve the same intent better).

## 4. The PR tier, runnable

`bin/check_all.sh` runs the PR tier that exists today, in fail-fast order. It writes `artifacts/check_report.md` with the commit, branch, each check's result and the verdict. As engine code arrives, each sprint adds its checks to the script; the script is part of the contract.

## 5. Traceability

- **Every test ID** in `docs/09` maps to requirements in the `docs/09` matrix.
- **Every sprint task** lists the tests that gate it.
- **Every system spec** lists the tests that earn each submodel its place (ADR-0033).
- **Tests in code** carry `Refs:` with the `V-` ID and the requirement ID.

## 6. Reports

| Report | Written by | Contents |
|---|---|---|
| `artifacts/check_report.md` | `bin/check_all.sh` | Header (time, commit, branch); each check with PASS/FAIL; first failure's output; verdict |
| `VER-SPR-NNN` | Architect | The sprint audit (`_templates/template_verification.md`) |
| `VER-G<n>` | Architect | The gate report: every gate test with its evidence, open defects, metrics (GOV-007 §7) |
| Certification report | M9 harness | Bands with CIs, non-ergodicity and typicality (`09` §3.1) |

## 7. Independence

Tests for authoritative code are written or reviewed by a Tester agent, not by the author (AGT-003-*). For the determinism machinery, the RNG and fixed-point types, and the held-out bands, this is mandatory.

## 8. Flaky tests

- **Quarantine** the test (`#[ignore = "quarantine: DEF-NNN"]` or a pytest marker) and file a `DEF-`.
- **Fix or delete within one sprint.**
- **A flaky determinism test is never flaky:** it is a determinism leak, P0 (`docs/13` §4.5).
