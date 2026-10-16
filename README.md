# Rehoboam

**A deterministic world engine:** centuries of emergent history you can watch, interrogate, branch, narrate and eventually inhabit.

> The simulation decides. Language models only describe.

**Status:** planning baseline v1.1+ (2026-10-15). There is no engine code yet. This repository contains the research, the plan and machine-readable contracts.

## Start here
1. [`docs/00_executive_summary.md`](docs/00_executive_summary.md): the plan on two pages.
2. [`docs/01_vision_and_intent.md`](docs/01_vision_and_intent.md): your intent restated, the definition of success, MOEs and principles.
3. [`docs/02_design_review_of_draft.md`](docs/02_design_review_of_draft.md): a red-team review of the original drafts.
4. [`docs/decisions_pending.md`](docs/decisions_pending.md): **every decision waiting on you, on one page** (full reasoning in [`docs/12_open_questions.md`](docs/12_open_questions.md)).

## Repository map
| Path | Contents |
|---|---|
| `docs/` | Plan: vision, design review, requirements, ConOps, architecture (`05_architecture/`), ADRs (`06_adr/`), stack, capacity, V&V, risks, roadmap, open questions, agent playbook |
| `research/` | Research dossiers R01–R11, the primary-source verification R12, the reading list, and `papers/` (the source library; private) |
| `specs/` | Machine-readable contracts: Mechanics Registry v0, manifest schema and examples, chronicle DDL and SQL-twin self-test, sifting DSL examples and linter |

## Verify the contracts
```bash
pip install "pydantic>=2" pyyaml "duckdb>=1.5"
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml   # expected to FAIL (8 errors)
python -I specs/chronicle/selftest.py                                                              # TWIN EQUIVALENCE: PASS
python -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml
```
