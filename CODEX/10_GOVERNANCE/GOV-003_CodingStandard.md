---
id: GOV-003
title: "Coding standard: Rust core first, Python and TypeScript tooling"
type: reference
status: APPROVED
owner: human
agents: [coder, tester, architect]
tags: [governance, coding]
related: [GOV-002, GOV-004, ADR-0006, ADR-0024, ADR-0021]
created: 2026-10-16
updated: 2026-10-16
version: 1.2.0
---

> **BLUF:** The authoritative engine is Rust. Its code must be deterministic, fixed-point where it matters, plane-isolated and owned per column. Python and TypeScript follow the director's template principles (types everywhere, no magic numbers, guard clauses, readable for incidents). Every rule here is either checked by a tool or by the Architect's audit.

# GOV-003 — Coding standard

## 1. Universal principles (from the template)

- **The 30-second rule.** A reader understands what a function does, and why, within 30 seconds. Comments explain *why*, not *what*.
- **Decision comments** at non-obvious choices: `DECISION:`, `ALTERNATIVES CONSIDERED:`, `TRADEOFF:`, `REF:` (an ADR, `docs` section or R12 §).
- **Failure comments** on risky paths: `FAILURE MODE:`, `BLAST RADIUS:`, `MITIGATION:`.
- **No magic numbers.** A constant is either a Registry parameter (with bounds and a source) or a named constant with a `REF:`.
- **Guard clauses first;** validate all inputs at public boundaries.
- **Small units:** functions ≤ 60 lines and cyclomatic complexity ≤ 10, unless a `DECISION:` comment justifies more. Files ≤ 800 lines.

## 2. Rust (authoritative crates: `rh-core`, `rh-sys-*`, `rh-registry`, `rh-ids`, `rh-contracts`)

| Rule | Check |
|---|---|
| `cargo fmt`; `clippy -D warnings` (`clippy::pedantic` where practical) | CI |
| `#![forbid(unsafe_code)]`, except crates on a reviewed allow-list (e.g. SIMD kernels) with a `SAFETY:` comment per block | CI lint |
| **No `HashMap`/`HashSet` iteration** in authoritative code; use `BTreeMap`, `IndexMap` with a deterministic insertion order, or sorted vectors | Architecture lint |
| **No float-order-dependent reductions** in authoritative code: fixed point, or ordered reductions with a declared order | Lint plus review |
| **Fixed point** (`q8_8`, `q16_16`, `i64` minutes) for any quantity that crosses a threshold; floats only for declared fields | Spec (`.odd.toml` `fixed_point`) plus lint |
| **Keyed RNG only:** `H(world_seed, branch_salt(t), system_id, epoch, purpose, entity_id, t, k)`. Each purpose is declared in the system spec; no thread-local or global RNG | Lint against the spec's `[[rng]]` |
| **No wall-clock reads** (`Instant::now`, `SystemTime`) in plane A | Lint |
| **Single writer:** a system writes only columns its spec owns; others are requested through lifecycle commands | `AccessSpec` checker against the Registry |
| **Plane isolation:** plane-A crates never depend on plane-B or plane-C crates | Dependency-graph lint |
| **Errors:** `Result` with typed errors (`thiserror`); no `unwrap`/`expect` in library code except where an invariant makes failure impossible, documented with `INVARIANT:`. `panic!` only for violated invariants, which fail the run loudly | clippy (`unwrap_used`) plus review |
| **Generated code** (`lint_odd.py --rust`, schemas) is never hand-edited | CI regenerate-and-diff |
| **Docs:** every public item has a doc comment; every module starts with its purpose and its `REF:` | `missing_docs` lint |

## 3. Python (world compiler, LLM gateway, narrative tools, specs)

- `ruff` and `mypy --strict`; type hints on every public function; Google-style docstrings.
- `python -I` for anything that reads untrusted files.
- Scripts in `specs/` use only the standard library where practical, so checks run anywhere.
- No mutable module globals; configuration through explicit parameters.

## 4. TypeScript (web console after G2)

- `tsc --strict`; no `any`; ESLint; `data-testid` on every interactive element (for Playwright later).

## 5. Forbidden everywhere

- Silent error swallowing (`except: pass`, `let _ =` on a `Result` without a comment).
- Free-string semantics in manifests or code where the Registry has a vocabulary (ADR-0010).
- Code, weights or assets whose licence forbids the use itself, and copied code that carries no licence (ADR-0021 as amended: personal-use mode, D19, 2026-10-16; any other licence is allowed, and the licence ledger records it).
- Re-recording golden baselines by hand (`docs/13` §4.2).

## 6. Known failure modes and anti-patterns (binding in review)

Director's ruling at G0, 2026-10-16 (red-team item 15; `docs/red_team/RT-2026-10-16-G0-dispositions.md` §3). Each row is a bug that earlier simulation engines shipped, paired with the guard that keeps it out of Rehoboam. A reviewer cites the row by number. A change that reintroduces a failure mode without its guard is refused. These rows are not `CLAUDE.md` non-negotiables: each guard lives in the spec or test named here.

| # | Failure mode | Its guard | Status |
|---|---|---|---|
| 1 | Repath storm: agents re-planning routes every tick | A static route graph at M1 F0; an unreachable target fails with a backoff timer | Add the backoff to the movement spec (fugitives, P1b) |
| 2 | Item clutter: unbounded loose objects | SAGA-12 and V-SAGA-17: an artifact has exactly one bearer or location; goods are aggregate stocks (M4) | Covered |
| 3 | Per-tick temperature and other over-fine physics | M3 F0 is an annual anomaly; each system has its own fidelity ladder (`05.1` §3) | Covered |
| 4 | A trivial act cascading into a death (a cat licking spilt beer) | The abstraction horizon is the fidelity ladder; plane A has no hygiene model | Covered |
| 5 | Full-map job scans for every actor | Candidate actions come from the agenda and the actor's situation, never a world scan | Specified: `T1.minds` submodel `decide` builds candidates only from the actor's situation sets (its court, relation rows, own claims and agenda goals; PR #32) |
| 6 | Allocation stutter | V-PERF-11: an allocation budget per bench, failing like an instruction-count regression (SPR-009 T-05; BCK-002 A-32) | Covered: a budget, not a blanket ban |
| 7 | Nested containers | S6: one bearer or location; no containers in the schema | Add "no nested containers" to S6 explicitly (P1.19) |
| 8 | Seams at the edge of a "reality bubble" | The world runs whole, watched or not (ADR-0025); V-DET-07, observer invariance | Covered |
| 9 | Rivers flowing uphill | Priority-Flood depression filling before flow routing, with a total-order tie-break (red-team item 11) | Adopted for worldgen |
| 10 | Subgraph explosion in pattern search | Ordered, anchored sifting patterns with declared budgets (`05.4`; `specs/sifting/lint_patterns.py`) | Covered |
