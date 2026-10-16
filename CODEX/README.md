# CODEX — Rehoboam's operating system

> **BLUF:** This is where the project's day-to-day work lives: the roadmap's front page, the backlogs, sprints, defects, change proposals, verification reports, governance and agent roles. The *design* lives in `docs/`, the *contracts* in `specs/` and the *evidence* in `research/`. Adapted from the director's `agentic_architect` template (ADR-0032).

## Start here

| You are | Read, in order |
|---|---|
| **The director** | `../docs/decisions_pending.md` (what waits on you) → `05_PROJECT/PRJ-001_Roadmap.md` → the latest `40_VERIFICATION/VER-*` |
| **Any agent** | `../CLAUDE.md` (non-negotiables) → `../AGENTS.md` → `00_INDEX/MANIFEST.yaml` → `10_GOVERNANCE/GOV-007`, `GOV-005`, `GOV-002` → your `80_AGENTS/AGT-*` boot doc → your sprint in `05_PROJECT/` |

## Areas

| Area | Holds | Owner |
|---|---|---|
| `00_INDEX/` | `MANIFEST.yaml`, **generated** by `../bin/gen_manifest.py`; never edit by hand | Architect (generator) |
| `05_PROJECT/` | `PRJ-001` roadmap front page, `BCK-001` development backlog, `BCK-002` architect backlog, `SPR-*` sprints | Architect; director approves priorities |
| `10_GOVERNANCE/` | `GOV-001`–`GOV-008`: documentation, testing, coding, errors and logging, lifecycle, observability, project management, infrastructure | Director |
| `40_VERIFICATION/` | `VER-SPR-*` sprint audits; `VER-G*` gate reports | Architect, Testers |
| `50_DEFECTS/` | `DEF-*` | Testers, Architect |
| `60_EVOLUTION/` | `EVO-*` change proposals (an accepted architectural change becomes an ADR in `../docs/06_adr/`) | Any agent, director |
| `80_AGENTS/` | Role and boot docs: Architect, Developer (core, tools), Tester (determinism, validation, contracts) | Director |
| `90_ARCHIVE/` | Superseded documents (nothing is deleted) | Architect |
| `_templates/` | Sprint, defect, evolution, verification templates; the system-spec template is `../specs/systems/_template.odd.toml` | Architect |

## How work flows (GOV-005)

conversation → spec (SPR / EVO / DEF, plus `.odd.toml` for a system) → sprint on one branch → `../bin/check_all.sh` and independent Testers → the Architect's audit (`VER-SPR-*`) → merge by change class → gates decided by the director.
