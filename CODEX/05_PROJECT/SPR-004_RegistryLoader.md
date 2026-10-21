---
id: SPR-004
title: "Registry v0 loader and ownership lint"
type: how-to
status: ARCHIVED
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, PRJ-001, VER-SPR-004, VER-SPR-004-TEST]
created: 2026-10-16
updated: 2026-10-16
version: 1.1.0
---
> [!NOTE]
> **ARCHIVED BASELINE:** Traditional sprint-based tracking has been superseded by Specification-Driven Development (SDD v2.0). All future deliverables are tracked as Spec Waves in `.spec/ARCHITECT_BACKLOG.md`.

> **BLUF:** Load the Mechanics Registry and manifests in Rust, generate their JSON Schema, and enforce column ownership from the Registry and the system specs. Developer: AGT-002-CORE. Testers: AGT-003-IF. Scope-bounded; at most about 3 agent-days.

# SPR-004 — Registry v0 loader and ownership lint

- **Roadmap item:** `docs/11` P0.8 · **Phase:** P0 (gate G0)
- **Depends on:** SPR-001 · **Plan approved by the director 2026-10-16** (P0 batch)
- **Binding contracts:** `specs/manifest/registry_v0.toml`, `specs/manifest/manifest_v0.py` (reference behaviour), `specs/systems/*.odd.toml` (ADR-0033); ADR-0010
- **Requirements covered:** WC-3, SIM-11

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C2 (contract loader) |
| Planes touched (ADR-0024) | none at runtime |
| Owned columns written | none |
| RNG purposes | none |
| Golden-hash effect | none |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | V-WC-04 (the Rust loader agrees with the Python validator on `solo`, `val_khor`, `aethelgard`, `uz_job` valid and `invalid_draft_style` invalid); ownership lint |
| Docs updated in the same sprint | `bin/check_all.sh` gains this sprint's checks; `docs/09` rows if tests change; `CLAUDE.md` checks |

## Tasks

### T-01: Registry loader
- **Deliverable:** `rh-registry` parses `registry_v0.toml` into typed structures, with bounds
- **Acceptance:** Round-trip test; every Registry section loads
- **Status:** [x] done (`rh-manifest::registry`, moved from `rh-registry` by ruling A; round-trip and section tests)

### T-02: Manifest loader
- **Deliverable:** Typed manifest loading with the same errors as `manifest_v0.py`
- **Acceptance:** Parity test on all example manifests (same verdicts and error counts)
- **Status:** [x] done (`rh-manifest::manifest`, `rh-cli manifest validate`; parity on 5 examples + 16-file corpus, `bin/manifest_parity.py`)

### T-03: Ownership lint
- **Deliverable:** Column ownership from `specs/systems/*.odd.toml` checked against each system's `AccessSpec` (stub until SPR-005)
- **Acceptance:** A test system writing an unowned column is rejected
- **Status:** [x] done (`rh-registry::ownership`, `rh-cli ownership`; positive control `tests/lint_fixtures/ownership/`)

### T-04: Schema generation
- **Deliverable:** JSON Schema emitted from the Rust types; diffed against `manifest_v0.schema.json`
- **Acceptance:** No unexplained schema difference
- **Status:** [x] done (`rh-manifest::manifest::schema`, `rh-cli manifest schema --diff`; 0 differences after normalising annotations)

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-IF | The gating tests above, written from the binding contracts before reading the implementation | `VER-SPR-004` |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [x] All tasks meet their acceptance criteria.
- [x] `bin/check_all.sh` passes; the report is attached.
- [x] Tester verdicts are PASS.
- [x] Architect audit `VER-SPR-004` is PASS.
- [x] No open DEF- against this sprint.
- [x] Merge route matches the CI-detected change class.
