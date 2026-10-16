# AGENTS.md — briefing for any AI agent working on Rehoboam

> **BLUF:** Rehoboam is a deterministic world engine, built by AI agents and governed by one human (the director) through CODEX. Read `CLAUDE.md` for the non-negotiables first. Then find your role and your sprint through the generated index. Never change a contract to make your code pass: propose an `EVO-` instead.

## Reading order (every session)
1. `CLAUDE.md`: the non-negotiables (determinism, three planes, single writer, Registry-only mechanics, licences) and the checks.
2. `CODEX/README.md`, then `CODEX/00_INDEX/MANIFEST.yaml` (generated; filter it, do not crawl the repo).
3. `CODEX/10_GOVERNANCE/GOV-007` (who decides what), `GOV-005` (the work loop), `GOV-002` (testing).
4. Your role: `CODEX/80_AGENTS/AGT-*`.
5. Your sprint: `CODEX/05_PROJECT/SPR-*`, and everything it names as binding.

## Roles
| Role | Doc | Writes code? |
|---|---|---|
| Architect (AI project manager) | `AGT-001` | No |
| Developer, Rust core | `AGT-002-CORE` | Yes (authoritative engine) |
| Developer, tools | `AGT-002-TOOLS` | Yes (Python/TypeScript around the engine) |
| Tester, determinism / validation / contracts | `AGT-003-DET`, `AGT-003-STAT`, `AGT-003-IF` | Tests only |

## Commands
| Command | What it does |
|---|---|
| `bin/check_all.sh` | The PR tier, fail-fast; writes `artifacts/check_report.md`. No report, no pass |
| `python -I bin/gen_manifest.py` | Regenerate the document index (`--check` to verify it is current) |
| `python -I specs/systems/lint_odd.py` | Lint system specs; `--brief <id>` for the director's one-page summary; `--rust <id>` for the code skeleton |

## Commits
`type(SPR-NNN): T-NN description`, then a body with `Why:`, `What:`, `Refs:` and the attribution lines the harness requires.
