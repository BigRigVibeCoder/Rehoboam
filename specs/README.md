# specs/ — Machine-readable contracts

These artefacts are the executable side of the plan in `docs/`. AI agents implement against them. Each one is validated by running it.

| Artefact | Purpose | How to check it |
|---|---|---|
| `manifest/registry_v0.toml` | Minimal **Mechanics Registry**: the closed vocabulary of systems with bounded parameters, archetypes, governments, succession laws, institutions, technologies, act classes, roles, channels, overlays, pattern packs, style cards, templates, values, doctrine tags, heightmap templates, climate zones, and the sifter's typed attributes and verbs | Loaded by the two tools below |
| `manifest/manifest_v0.py` | World-manifest schema v0 (Pydantic v2): discriminated unions, no free-string semantics. Also performs Registry-aware semantic validation: bounds, vocabulary (values, doctrines, heightmaps, climate zones), required roles, scenario horizon and speculative trajectory, creature capabilities and the S4.ecology requirement, testing se/sp, population envelope, Carbone-ratio energetics | `python -I specs/manifest/manifest_v0.py validate specs/manifest/examples/val_khor.yaml` |
| `manifest/manifest_v0.schema.json` | JSON Schema generated from the Pydantic model. Constrained decoding uses it after enums are injected from the Registry | `python -I specs/manifest/manifest_v0.py schema` |
| `manifest/examples/val_khor.yaml` | Reference world 1: feudal realm with dragons and a father–son succession catalyst | Validates with 0 errors |
| `manifest/examples/uz_job.yaml` | Reference world 3 (micro, N ≈ 60): the trials of Job. Explicit cast, a household with holdings, off-map raiders, and an Adversary under a staged wager catalyst (ADR-0028) | Validates with 0 errors |
| `manifest/examples/aethelgard.yaml` | Reference world 2: modern metropolis with a covert hive infection and a 10-year horizon | Validates with 0 errors |
| `manifest/examples/invalid_draft_style.yaml` | Negative test reproducing the draft schema's failure modes | **Must fail** with 8 errors: horizon, two bounds violations, unknown system, unimplemented fidelity, missing successor role, creatures without S4.ecology, dragon energetics |
| `demography/brass_general_standard.toml` + `demography/selftest_brass.py` | Brass (1971) General Standard life table and Table 19 golden schedules; M2 mortality golden vectors | `python -I specs/demography/selftest_brass.py` prints `BRASS GOLDEN VECTORS: PASS` |
| `chronicle/schema.sql` | Chronicle DDL (DuckDB dialect; Parquet is the system of record), tagged by plane (A/B/C): anchored snapshots, typed causal edges, knowledge and diffusion fronts, aggregate events, SCD2 intervals, the life-course log, the facts ledger, sift matches, the visibility sidecar, base rates, the input journal (push/pull, scope, epoch, model hashes), plane hashes and the keyframe index. Includes **SQL twins** for one pattern under three declared semantics, and macros | `python -I specs/chronicle/selftest.py` |
| `chronicle/selftest.py` | Generates a uniform and a **clustered** 200k-event synthetic chronicle and asserts that the SQL twins and a reference **incremental matcher** produce **identical match sets** under `all`, `latest` + `clear_binding`, and `dedupe` semantics. Reports firing rates (on the clustered fixture, `all` fires 4.5× more than the default). Also exercises the causal-ancestor walk and as-of title queries | Exits with code 0 and prints `TWIN EQUIVALENCE: PASS` |
| `sifting/patterns.example.toml` | Pattern DSL v0 examples: setting-agnostic roles, anchored state reads, `unless`, composable match events, declared match semantics (`selection`, `on_complete`, `dedupe`), firing budgets and completion hazards | `python -I specs/sifting/lint_patterns.py specs/sifting/patterns.example.toml specs/manifest/registry_v0.toml` |
| `sifting/lint_patterns.py` | Linter. Rejects unanchored state reads (the draft's temporal bug), unknown keys and vocabulary, undeclared or invalid match semantics, and missing firing budgets | Prints `PASS` and shows the draft rule being rejected |

Dependencies for running the checks: Python ≥ 3.11, `pydantic>=2`, `pyyaml`, `duckdb>=1.5`.

These are **v0 contracts**. They are frozen and extended in P0/P1 (`docs/11_roadmap.md`, WP P0.8), and the Rust `rh-registry` crate becomes their authoritative implementation.
