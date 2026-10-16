# CLAUDE.md: guidance for AI agents working on Rehoboam

## Project state
P0 (foundations) is under way. SPR-001 created the Cargo workspace (stub crates in `crates/`), the Python tooling (`python/`), the architecture lints and CI. The plan lives in `docs/`, the evidence in `research/`, and executable contracts in `specs/`.

## Operating model (ADR-0032)
Day-to-day work runs through `CODEX/`: roles (`CODEX/80_AGENTS/`), sprints and backlogs (`CODEX/05_PROJECT/`), governance (`CODEX/10_GOVERNANCE/`), and the generated index (`CODEX/00_INDEX/MANIFEST.yaml`). Start each session with `AGENTS.md`. Run `bin/check_all.sh` before asking for an audit; no report, no pass. Be direct: no flattery, no softened bad news; bring the director decisions with a recommendation and a confidence rating, not open questions.

## Read before any work
1. `docs/00_executive_summary.md`
2. `docs/01_vision_and_intent.md` (success criteria S1–S4, principles)
3. The ADRs relevant to your task (`docs/06_adr/`)
4. `docs/13_agentic_development_playbook.md` (work-package template, Definition of Done, change classes)
5. `docs/14_reuse_inventory.md` before writing anything from scratch: check whether a permissive project already solves it (PORT or DEPEND)

## Non-negotiables (from ADRs; changing them needs director approval)
- **The simulation decides; LLMs describe.** LLM output enters state only as a journaled, schema-validated input (ADR-0006, ADR-0011, ADR-0034); LLMs never choose actions and the engine never waits on a model.
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
- **Licences: personal-use mode** (ADR-0021 as amended, D19, 2026-10-16). Rehoboam is a private experiment that is never distributed, so any licence may be used. Two limits remain: no licence whose terms forbid the use itself, and no copying of unlicensed code. The repository stays private. `bin/check_all.sh` keeps a licence ledger (`artifacts/licence_ledger.txt`) that never fails a build; crates come from crates.io only, with no wildcard versions.

## Checks to run after any change
All of them, fail-fast, with a report: `bin/check_all.sh`. It uses `python/.venv` (create it with `uv sync --frozen --project python`; `PYTHON` overrides) and needs uv, cargo-nextest and cargo-deny (RUN-001). Each gate also runs on a planted violation that must FAIL (`tests/lint_fixtures/`). Individually:
```bash
# Rust: workspace in crates/, toolchain pinned in rust-toolchain.toml
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo deny --locked check bans sources            # crates.io only, no wildcards (licences: ledger only, ADR-0021 amended)
cargo nextest run --workspace --locked
cargo test --doc --workspace --locked             # doctests: rh-ids typed-ID compile-fail cases (SPR-002)
# Architecture lints: stdlib Python
python -I bin/lint_arch.py        # crate boundaries (V-INSP-03) and plane isolation (V-PLN-01)
python -I bin/lint_forbidden.py   # forbidden APIs in plane-A crates (V-DET-19)
# Python: configuration in python/pyproject.toml, run from the repository root
python -m ruff check --config python/pyproject.toml .
python -m mypy --config-file python/pyproject.toml
python -m pytest -c python/pyproject.toml --rootdir=.
# Specs
python -I bin/gen_manifest.py --check
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/solo.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/aethelgard.yaml
python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/invalid_draft_style.yaml  # must FAIL
python -I bin/gen_rng_golden.py --check  # rh-ids golden vectors match the independent reference (SPR-002)
python -I specs/chronicle/selftest.py
python -I specs/demography/selftest_brass.py
python -I specs/economy/selftest_economy.py
python -I specs/stream/selftest_virtual_telemetry.py
python -I specs/systems/lint_odd.py
python -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml
```

## Conventions
- Docs are Markdown, and diagrams are Mermaid.
- Requirements use IDs from `docs/03`. ADRs are numbered `ADR-NNNN`.
- Cite research as `R0x §n`.
- Record director rulings and gate outcomes in `docs/decisions-log.md`.
- Every crate declares its plane and kind in `[package.metadata.rehoboam]` (`bin/lint_arch.py` checks it).
- A forbidden API in plane A needs `// LINT-ALLOW(forbidden-api): <reason>` on its line; the gate counts and prints every escape.
