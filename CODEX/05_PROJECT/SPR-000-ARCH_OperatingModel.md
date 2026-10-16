---
id: SPR-000-ARCH
title: "Set up the operating model (CODEX, governance, agents, tooling)"
type: how-to
status: CLOSED
owner: architect
agents: [architect]
tags: [sprint, governance]
related: [ADR-0032, ADR-0033, BCK-002, VER-SPR-000-ARCH]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** The Architect's setup sprint for ADR-0032: CODEX areas, adapted governance GOV-001–008, six agent boot docs, templates, the roadmap front page and backlogs, P0 sprints SPR-001–009 planned in full, the generated manifest, and `bin/check_all.sh`. No design document or engine contract was changed. Closed after its audit, VER-SPR-000-ARCH.

# SPR-000-ARCH — Set up the operating model

| Compliance rule | This sprint |
|---|---|
| Change class | C0 (documentation and tooling); no contract, Registry or hash change |
| Planes touched | none |
| Tests that gate it | `bin/check_all.sh` (all checks green, including the new manifest staleness check) |

| Task | Deliverable | Status |
|---|---|---|
| T-01 | `CODEX/` areas with READMEs | [x] |
| T-02 | GOV-001–008 adapted to a deterministic simulation (ADR-0032 §2–3) | [x] |
| T-03 | AGT-001, AGT-002-CORE, AGT-002-TOOLS, AGT-003-DET, AGT-003-STAT, AGT-003-IF | [x] |
| T-04 | Templates: sprint, defect, evolution, verification; system-spec template (`specs/systems/_template.odd.toml`) | [x] |
| T-05 | PRJ-001, BCK-001, BCK-002; SPR-001–009 (P0, rolling wave) | [x] |
| T-06 | `bin/gen_manifest.py` (generated index plus `--check`), `bin/check_all.sh` (PR tier with a report) | [x] |
| T-07 | `AGENTS.md`; `CLAUDE.md` reading order and checks; `docs/13` marked as the engineering supplement | [x] |
