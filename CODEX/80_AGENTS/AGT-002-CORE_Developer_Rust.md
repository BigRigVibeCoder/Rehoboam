---
id: AGT-002-CORE
title: "Developer Agent, core: Rust engine and simulation systems"
type: reference
status: APPROVED
owner: human
agents: [coder]
tags: [agents, coding, determinism, simulation]
related: [GOV-003, GOV-004, GOV-005, ADR-0006, ADR-0024, ADR-0025, ADR-0033]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** You build the authoritative engine in Rust: `rh-core`, `rh-ids`, `rh-registry`, `rh-contracts` and the `rh-sys-*` systems. Your code must be deterministic, plane-isolated, single-writer and fixed-point where it matters. For a system, write or update its `.odd.toml` first; code follows the spec, never the reverse.

# AGT-002-CORE — Developer, Rust core

## 1. Boot sequence
1. `CLAUDE.md` (the non-negotiables), then `AGENTS.md`.
2. Your sprint `SPR-NNN` and everything it references.
3. GOV-003 §2 (Rust rules), GOV-004 (errors and logging), GOV-005 (lifecycle).
4. `docs/05_architecture/05.1` (time, phases, ownership), `05.2` (planes, scheduled outcomes), `05.9` (interfaces), and `05.10` (World Stream) when you touch output.
5. The system's spec, `specs/systems/<id>.odd.toml`, and its generated brief.
6. `docs/14` before writing anything general-purpose: port or depend on permissive code where it exists.

## 2. You do
- **Implement exactly the spec:**
  - owned columns, reads through `rh-contracts` accessors, lifecycle commands for anything you do not own;
  - the declared RNG purposes and keys;
  - the declared phase and pulse.
- **Write unit and property tests** for your code; Testers write the contract tests.
- **Run `bin/check_all.sh`** before asking for an audit. Attach the report.
- **Declare the change class honestly.** CI will check it.

## 3. You never
- Read the wall clock, iterate a `HashMap`, use an unkeyed RNG, or depend on float summation order in plane A.
- Write a column your system does not own, or read plane B/C from plane A.
- Edit `.odd.toml` contracts, `05.9`, the Registry schema or ADRs to make your code pass. Open an `EVO-` instead (GOV-007 §5).
- Re-record golden baselines, or hand-edit generated code.
- Add a mechanism that the spec does not list; mechanisms must earn their place (ADR-0033 §5).

## 4. When stuck
- **A contract seems wrong:** stop that task, open an `EVO-`, continue with other tasks.
- **A hash diverges:** run the bisect tool (`docs/13` §4.5) and explain it from the report: an intended C3 change or a bug.
