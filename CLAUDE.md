# CLAUDE.md: guidance for AI agents working on Rehoboam

## Project state
This repository is in the **planning phase**. There is no engine code yet. The plan lives in `docs/`, the evidence in `research/`, and executable contracts in `specs/`.

## Read before any work
1. `docs/00_executive_summary.md`
2. `docs/01_vision_and_intent.md` (success criteria S1–S4, principles)
3. The ADRs relevant to your task (`docs/06_adr/`)
4. `docs/13_agentic_development_playbook.md` (work-package template, Definition of Done, change classes)
5. `docs/14_reuse_inventory.md` before writing anything from scratch: check whether a permissive project already solves it (PORT or DEPEND)

## Non-negotiables (from ADRs; changing them needs director approval)
- **The simulation decides; LLMs describe.** LLM output enters state only as a journaled, schema-validated input (ADR-0006, ADR-0011).
- **Determinism.**
  - History = f(engine_build, registry_version, manifest_hash, seed, input_journal).
  - Use keyed counter-based RNG only: `H(world_seed, branch_salt(t), system_id, purpose, entity_id, t, k)`.
  - Threshold quantities must be fixed-point.
  - No HashMap iteration and no float-order-dependent reductions in authoritative code.
- **Three planes (ADR-0024).** Plane A (authoritative) never reads plane B (observation: pins, facts ledger, hydrated views) or plane C (narrative: sift matches, significance, visibility). The only way into plane A is a journaled material input.
- **Scheduled outcomes, no back-dating (ADR-0025).** Outcomes are timers that fire for everyone, watched or not. EventId order is emission order. Acting in the past forks.
- **Single-writer column ownership.** Each column has one owning system; others read it through contract crates and request changes through lifecycle command queues (`05.1` §2).
- **No physical compaction of named-actor history.** Hide low-salience events with the visibility sidecar instead (ADR-0007).
- **Sifting patterns must anchor every state read** (`attr@eN`) and declare their match semantics and firing budget. Use `specs/sifting/lint_patterns.py`.
- **Mechanics come only from the Registry.** No free-string semantics in manifests (ADR-0010).
- **Licences:** no GPL, AGPL, non-commercial or territorial-licensed code, weights or assets in shipped crates (ADR-0021). GPL/AGPL tools may run only out of process; see `docs/14_reuse_inventory.md` for traps (EbookLib, xxhash-rust, MiniCheck-7B weights).

## Checks to run after touching `specs/`
```bash
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/aethelgard.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml  # must FAIL
python -I specs/chronicle/selftest.py
python -I specs/demography/selftest_brass.py
python -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml
```

## Conventions
- Docs are Markdown, and diagrams are Mermaid.
- Requirements use IDs from `docs/03`. ADRs are numbered `ADR-NNNN`.
- Cite research as `R0x §n`.
- Record director rulings and gate outcomes in `docs/decisions-log.md`.
