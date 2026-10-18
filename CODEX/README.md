# CODEX — Rehoboam's operating system

> **BLUF:** This is where the project's day-to-day work lives: the roadmap's front page, the backlogs, sprints, defects, change proposals, verification reports, governance and agent roles. The *design* lives in `docs/`, the *contracts* in `specs/` and the *evidence* in `research/`. Adapted from the director's `agentic_architect` template (ADR-0032).

## Start here

| You are | Read, in order |
|---|---|
| **The director** | `../docs/decisions_pending.md` (what waits on you) → `05_PROJECT/PRJ-001_Roadmap.md` → the latest `40_VERIFICATION/VER-*` |
| **Any agent** | `../HANDOFF.md` (while a handoff is in progress) → `../CLAUDE.md` (non-negotiables; binds every agent) → `../AGENTS.md` → `00_INDEX/MANIFEST.yaml` → `10_GOVERNANCE/GOV-007`, `GOV-005`, `GOV-002`, `GOV-009` (Git and GitHub), `GOV-010` (CI), `GOV-011` (agent sessions and handoffs) → your `80_AGENTS/AGT-*` boot doc → your sprint in `05_PROJECT/` |

## Areas

| Area | Holds | Owner |
|---|---|---|
| `00_INDEX/` | `MANIFEST.yaml`, **generated locally** by `../bin/gen_manifest.py`; not committed; never edit by hand | Architect (generator) |
| `05_PROJECT/` | `PRJ-001` roadmap front page, `BCK-001` development backlog, `BCK-002` architect backlog, `SPR-*` sprints | Architect; director approves priorities |
| `10_GOVERNANCE/` | `GOV-001`–`GOV-011`: documentation, testing, coding, errors and logging, lifecycle, observability, project management, infrastructure, Git and GitHub, CI, agent sessions and handoffs | Director |
| `30_RUNBOOKS/` | Step-by-step procedures, starting with `RUN-001` (laptop setup) | Architect |
| `40_VERIFICATION/` | `VER-SPR-*` sprint audits; `VER-G*` gate reports | Architect, Testers |
| `50_DEFECTS/` | `DEF-*` | Testers, Architect |
| `60_EVOLUTION/` | `EVO-*` change proposals (an accepted architectural change becomes an ADR in `../docs/06_adr/`) | Any agent, director |
| `80_AGENTS/` | Role and boot docs: Architect, Developer (core, tools), Tester (determinism, validation, contracts) | Director |
| `90_ARCHIVE/` | Superseded documents (nothing is deleted) | Architect |
| `_templates/` | Sprint, defect, evolution, verification templates; the system-spec template is `../specs/systems/_template.odd.toml` | Architect |

## How work flows (GOV-005)

conversation → spec (SPR / EVO / DEF, plus `.odd.toml` for a system) → sprint on one branch → `../bin/check_all.sh` and independent Testers → the Architect's audit (`VER-SPR-*`) → merge by change class → gates decided by the director.
