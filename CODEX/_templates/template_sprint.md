---
id: SPR-NNN
title: "<sprint title>"
type: how-to
status: PLANNING
owner: architect
agents: [coder, tester]
tags: [sprint]
related: [BCK-001, "<WP id in docs/11>", "<ADR ids>"]
created: YYYY-MM-DD
updated: YYYY-MM-DD
version: 1.0.0
---

> **BLUF:** <one sentence: what this sprint delivers and which gate or rung it serves>. Developer: <AGT-002-CORE | AGT-002-TOOLS>. Testers: <AGT-003-DET | -STAT | -IF>. Scope-bounded; at most about 3 agent-days.

# SPR-NNN — <title>

- **Roadmap item:** <docs/11 WP id> · **Phase / rung:** <P0 / rung 0>
- **Depends on:** <SPR ids or none>
- **Binding contracts:** <05.9 IF-x §, specs/... files, ADR ids>
- **System spec:** <specs/systems/<id>.odd.toml, which must pass lint_odd.py before ACTIVE> or n/a
- **Requirements covered:** <docs/03 ids>

## Compliance (every task)

| Rule | This sprint |
|---|---|
| Change class (`docs/13` §4.2) | C<0-3>, with the reason |
| Planes touched (ADR-0024) | A / B / C |
| Owned columns written | <from the .odd.toml> |
| Reads (via `rh-contracts`) | <columns and owners> |
| RNG purposes added | <names and keys, or none> |
| Golden-hash effect | none / declared C3 with bisect report |
| Fixed-point quantities | <list or none> |
| Determinism lints (GOV-003 §2) | must pass |
| Tests that gate this sprint | <V-… ids> plus `bin/check_all.sh` |
| Docs updated in the same sprint | <spec, brief, 09 rows, CLAUDE.md checks> |

## Tasks

### T-01: <title>
- **Deliverable:** <files, crates, functions>
- **Acceptance:** <testable criteria, each naming a test>
- **Status:** [ ] not started

### T-02: <title>
- **Deliverable:**
- **Acceptance:**
- **Status:** [ ]

## Verification assignment (Testers)

| Tester | What to verify, from which contract | Report |
|---|---|---|
| AGT-003-<x> | | `VER-SPR-NNN` section |

## Blockers

| # | Blocker | Filed as | Status |
|---|---|---|---|
| — | None | — | — |

## Completion

- [ ] All tasks meet their acceptance criteria.
- [ ] `bin/check_all.sh` passes; the report is attached.
- [ ] The Tester verdicts are PASS.
- [ ] The Architect's audit `VER-SPR-NNN` is PASS.
- [ ] No open DEF- against this sprint.
- [ ] The merge route matches the CI-detected change class.
