---
id: VER-SPR-001
title: "Audit of SPR-001"
type: record
status: CLOSED
owner: architect
agents: [architect, tester]
tags: [verification]
related: [SPR-001, VER-SPR-001-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** PASS. `bin/check_all.sh` passes 71 checks locally and in CI on GitHub-hosted runners. The independent Tester passed all three gating tests (V-INSP-03, V-PLN-01, V-DET-19) and found five defects, all fixed in the fix round and each re-verified with the Tester's own reproduction. Change class C2, additive: the Architect merges (GOV-005 §6). One task remains with the director: T-00, running RUN-001 on the laptop.

# VER-SPR-001 — Audit of SPR-001

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` (PR tier) | PASS (71 checks; 44 before the fix round) | `artifacts/check_report.md` at the merge head; the Developer's fresh-clone run at `1544657` |
| CI on GitHub-hosted Ubuntu runners | PASS on every pushed head, about 45–50 s per run (target: ≤ 15 min) | PR #2 check runs; report and licence-ledger artifacts uploaded |
| CI-detected change class | C2 (declared C2): layout, lints and CI; no simulation code | Diff touches `crates/` stubs, `bin/`, `python/`, `tests/lint_fixtures/`, `.github/`, `deny.toml`, `clippy.toml` and docs |
| Golden hashes | None exist yet (no simulation code) | — |

## 2. Tester verdicts
| Tester | Contract verified | Verdict | DEF filed |
|---|---|---|---|
| AGT-003-IF, run on a model different from the Developer's | V-INSP-03, V-PLN-01, V-DET-19 and `check_all` end to end, from the contracts, before reading the implementation | PASS on all three gating tests; `check_all` PASS with DEF-2 | DEF-1 to DEF-5 (`VER-SPR-001-TEST` §3) |

Probes: 286 + 206 results across two frozen runs; 64 positive-control "neutering" mutations, of which 54 turned `check_all` red before the fix round.

## 3. Defects and gaps: disposition
| Item | Severity | Fix (commit) | Re-verified with the Tester's reproduction |
|---|---|---|---|
| DEF-1: `rand` 0.9 thread-local functions passed every gate | Medium | `lint_arch` rule: plane-A crates may not depend on unkeyed RNG crates (`rand_core` and `rand_chacha` only in `rh-ids`); `lint_forbidden` and clippy cover the 0.9 names, `SmallRng`, `OsRng`, `from_entropy`, `getrandom` (`260a4ed`) | `check_all` exits 1 at `VIOLATION [rng-crate] rh-core` |
| DEF-2: a failed first check did not stop the cascade | Low–medium | Function definitions moved above one fail-fast chain; a pytest guards it (`d9ea819`) | exits 1 after exactly one check |
| DEF-3: a quote right after an identifier hid code from the lexer | Low | Literal versus lifetime decided by lookahead (`ec744b4`) | the scan exits 1 on `Instant::now` |
| DEF-4: spaced paths; `#[path]` and `include!` from outside the crate | Low | `::` normalised before matching; `#[path` and `include!(` forbidden in plane A unless escaped with a reason (`b947104`) | all four reported |
| DEF-5: two `lint_arch` features had no positive control | Low (test gap) | Controls for a build-dependency edge and a `package =` alias (`e00e5dc`) | breaking either turns `check_all` red |
| Gap: crate labels were self-declared | — | The seven core crates' (plane, kind) pinned by name; `rh-sys-*` must be A/system; `kind = "contract"` only for `rh-contracts` or `rh-contract-*` (`a5de9c4`) | relabelling reports the pin, the name rule and the edge |
| Gap: no plane-C compile-fail case | — | Second compile-fail fixture (`a0eae91`); `docs/09` V-PLN-01 wording updated (fixtures now, `trybuild` per table later) | — |
| Gap: lock drift invisible to `uv sync --frozen` | — | `uv lock --check --offline` in `check_all`, with a stale-lock control (`1544657`) | a stale lock fails |
| Gaps: ADR-0021 allow-list versus `thiserror`/`serde`; allow-list widening; GPL dev-dependency | — | Overtaken by the director's ruling: personal-use mode (ADR-0021 as amended, D19). Licences now go to a non-blocking ledger with a control (`8598bf6`); crates.io-only and no-wildcard rules stay blocking, each with a new positive control | the GPL fixture appears in the ledger; git and wildcard dependencies fail |
| Gap: float types in threshold positions | — | Owned by SPR-010 T-04 (approved) | — |
| Gap: coverage gates (GOV-002 §3) owned by no sprint | — | BCK-002 A-15: add `cargo-llvm-cov` thresholds when SPR-002 lands the first real code | — |
| Observation: a stray `/Cargo.toml` at the container's filesystem root | — | A fixture copy misplaced during development, outside the repository. The environment's safety check blocks deleting a root-level path, so it is left; it disappears with the cloud container | — |

**Escapes found later:** none so far. This row feeds the escape rate (BCK-002 A-05).

## 4. Acceptance criteria
| Task | Criterion | Evidence | Met |
|---|---|---|---|
| T-00 | `bin/check_all.sh` passes on the director's laptop | Not yet run; the director's action (RUN-001) | **Open (director)** |
| T-01 | `cargo build --workspace` and `cargo nextest run` pass | the `cargo build` and `cargo nextest` checks; Developer's fresh-clone run | Yes |
| T-02 | ruff and mypy pass; `uv sync --frozen` reproduces the environment | Tester T02-REPRO: two fresh clones froze identically | Yes |
| T-03 | A deliberately GPL dependency fails `cargo-deny` | Superseded by D19: the control now proves the GPL dependency is **recorded** in the ledger; bans and sources controls fail as required | Yes (as amended) |
| T-04 | Each lint has a failing fixture that it catches | 24 forbidden-API controls, arch controls including build-dependency and alias, two compile-fail fixtures | Yes |
| T-05 | A PR with a lint violation cannot be merged; a clean PR passes in ≤ 15 min | Clean PR: about 50 s in CI. On GitHub Free a red PR cannot be *blocked*, only reported; the Architect enforces "no green report, no merge" (B-1, `docs/13` §4.4). Tester T05-H: the pre-push hook refused a violating push | Yes (as amended by B-1) |

## 5. Compliance table
| Rule | Required | Found | Met |
|---|---|---|---|
| Change class | C2 | C2 | Yes |
| Planes touched | none | none (stubs only) | Yes |
| Owned columns, RNG purposes, golden hashes | none | none | Yes |
| Determinism lints | must pass | pass | Yes |
| Docs in the same sprint | `check_all` gains checks; `docs/09` rows if tests change; `CLAUDE.md` checks | all three, plus RUN-001 §3–5 and GOV-008 §2 | Yes |

## 6. Contract and ADR conformance
- **GOV-003 §2:** `unsafe_code` forbidden and `missing_docs` denied workspace-wide; no `unwrap`/`expect` in library code; every `allow` needs a reason. Conforms.
- **`docs/13` §3, ADR-0024:** crate-boundary law and plane isolation enforced by `lint_arch` and compile-fail fixtures. Conforms.
- **ADR-0021 (amended):** conforms to personal-use mode.
- **GOV-005 §4 deviation, recorded:** the sprint ran on the platform-named branch `ccr-1db07854-bstsd2` instead of `sprint/SPR-001-…`, because this session's branch is fixed by the platform. One later Developer fix round came from the same agent.
- **Independence:** the Tester ran on a different model from the Developer and wrote its probes before reading the code. In phase 1 it saw the stray `/Cargo.toml`, which named two lint files; it reports that no probe was shaped by it.

## 7. Verdict
| Field | Value |
|---|---|
| Verdict | **PASS** |
| Merge route | Architect merges: C2, additive; plan approved by the director (GOV-005 §6) |
| Carried forward | T-00 (director's laptop run); BCK-002 A-14 (engine-purity lint), A-15 (coverage gates) |
| Escapes found later | — |
