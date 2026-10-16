---
id: BCK-002
title: "Architect backlog"
type: reference
status: ACTIVE
owner: architect
agents: [architect]
tags: [backlog, project-management]
related: [BCK-001, AGT-001, GOV-007]
created: 2026-10-16
updated: 2026-10-16
version: 1.4.0
---

> **BLUF:** The Architect's own work stream, run in parallel with development: operating-model upkeep, audit tooling, gate preparation, and documentation debt.

# BCK-002 — Architect backlog

| # | Item | When | Status |
|---|---|---|---|
| A-01 | Set up CODEX, governance, agent docs, templates, generated manifest, `bin/check_all.sh` | Now | **Done** (SPR-000-ARCH) |
| A-02 | Configure tester and auditor model diversity (a different model or provider from the developers) and record it in each AGT boot doc | Before SPR-002 | **Done**: GOV-005 §4.2; AGT-001, AGT-003-* |
| A-03 | Wire `bin/check_all.sh` into CI as the PR tier (with SPR-001) | SPR-001 | **Done** (`VER-SPR-001`) |
| A-04 | Seal the held-out bands: commit hashes of their definitions, with AGT-003-STAT as custodian | Before P1b tuning | Open |
| A-05 | Escape-rate tracking in VER documents; report it at G0 | From SPR-002 | Open |
| A-06 | Write the rung 1–3 manifests (`household`, `hamlet` from `uz_job`, `village`) | P1a | Open |
| A-07 | Add frontmatter to `docs/` files as they change; split `docs/09` (about 108 KB) when it next changes substantially | Rolling | Open |
| A-08 | Turn `docs/13` into the Rehoboam engineering supplement: remove what GOV-005/007 now cover | With A-07 | Open |
| A-09 | Prepare `VER-G0` | End of P0 | Open |
| A-10 | Cloud adaptation table in GOV-008 | When the director starts the full cloud move | Deferred |
| A-11 | Enforce "no green report, no merge" by hand (GitHub Free: no required checks) | Every merge | Ongoing |
| A-12 | Tell the director in advance when an `adjutant` session is needed, what for and for how long (D6) | As needed | Ongoing |
| A-13 | Set `main` as the default branch on GitHub (director, Settings → General) | Now | **Done** 2026-10-16 |
| A-15 | Coverage gates (GOV-002 §3: line ≥ 80%, branch ≥ 75%) with `cargo-llvm-cov` in CI; owned by no sprint (`VER-SPR-001` §3) | When SPR-002 lands the first real code | Open |
| A-16 | Remind the director to run RUN-001 on the laptop (SPR-001 T-00) | Now | Open |
| A-14 | Add the engine-purity rule to `bin/lint_arch.py` (ADR-0035 §6; EVO-4; V-INSP-05): engine crates (`rh-core`, `rh-ids`, `rh-events`, `rh-registry`, `rh-manifest`, `rh-contracts`, `rh-snapshot`, `rh-sys-*`, `rh-sifter`, `rh-server`, `rh-cli`) may not depend on media, HTTP-UI or LLM-client crates; one deny-list kept in one place (e.g. `image`, `rodio`, `cpal`, `symphonia`, `reqwest`, `axum`, `tower-http`, `async-openai`, `tokenizers`), with a positive control in `tests/lint_fixtures/` | P1a | Open |
| A-17 | Write the `T1.minds` and `M7.dynasty` specs, which P1a now needs (ADR-0035; BCK-001 §3) | Before the P1a sprints | Open |
| A-18 | SPR-005: systems declare RNG purposes from their specs; check `[[rng]]` against the code (`VER-SPR-002` GAP-3) | SPR-005 | Open |
| A-19 | Timeline Manager (P2.6): test that timeline ids are unique, since salt uniqueness depends on it (`VER-SPR-002` GAP-4) | P2.6 | Open |
| A-20 | Gate plane A's external dependencies: a `bin/lint_arch.py` rule that every registry dependency of a plane-A crate appears in one reviewed allow-list, with a one-line note on whether it returns floats, plus a planted-fixture control. Clippy has no lint for float `<`/`>`, so an unnamed float can only enter through such a dependency (`VER-SPR-010` GAP-3; the Developer's proposal) | Before G0 | **Done**: `bin/lint_arch.py` rule `plane-A-external` (`PLANE_A_EXTERNAL_DEPS`), two positive controls |
| A-21 | Implement `rh_core::fixed::UniformDraw` for `rh_ids::RngStream` (a new crate edge between `rh-ids` and `rh-core`) | The first sprint that draws against a probability | Open |
| A-22 | Escapes excuse a line, not an item: when the first system declares an `f32_field`, add the rule that threshold comparisons read a fixed-point column (`VER-SPR-010-TEST` GAP-2) | The first sprint with an `f32_field` | Open |
| A-23 | Relax flow-continuation indentation to PyYAML's if a hand-written manifest hits it: a flow collection or quoted scalar continued at or left of its parent key's column is a Rust LOAD ERROR and VALID in the reference (SPR-004 DEF-12 / GAP-8; out of contract, fails closed; `05.5` §3) | When a hand-written manifest hits it | Open |
| A-24 | Type the saga vocabulary tables (`artifact_kinds`, `vow_kinds`, `mental_breaks`, and `contaminants`, `event_roles`, `motive_edges` when their specs freeze) in the Registry loader. Ruling: a section is typed once its spec is frozen (`VER-SPR-004` DEF-6, GAP-2) | Before the S6, S7 and T1 sprints | Open |
| A-25 | A node-count limit on YAML alias expansion in `rh-manifest`, noted in `05.5` §3 (`VER-SPR-004` GAP-6) | Before manifests can come from outside the expander | Open |
| A-26 | Move `lint_forbidden`'s `float-api` rule from text to rustdoc JSON, so that it sees a float in a `pub trait` method, behind an associated-type projection, in a macro's type tokens or behind an opaque `impl Trait`. Until then the reviewer reads `rh-sample`'s public items (`VER-SPR-003` GAP-11) | When rustdoc's JSON format is stable, or before `rh-sample`'s public surface grows a trait, macro or `impl Trait` | Open |
| A-27 | `lint_forbidden` follows an `include!` whose argument is a string literal resolving to a file inside the repository, and scans that file as source; a non-literal path (`concat!`, `env!`) or one outside the repository stays forbidden. Planted controls both ways. ADR-0033 §3 has system crates include their generated skeletons, which the source-include rule (VER-SPR-001 DEF-4) refuses today; SPR-005 uses a counted `LINT-ALLOW` meanwhile | Before the first `rh-sys-*` crate | Open |
| A-28 | Exact `start_of_day` and `previous_pulse` read lags (`05.1` §2 rule 4): column snapshots taken at the start of the day and at the reader's previous firing. P0 reads all three non-edge lags at the start of the instant (Architect ruling, SPR-005 G-b). That is exact for S5 but not for `previous_pulse` (S6, S7) or for an Hour reader at `start_of_day`. First step, in SPR-006: `Engine::new` refuses those reads with `Unsupported` until this item lands (VER-SPR-005) | Before the first system that declares them runs (P1a: S5, S6, S7) | Open |
| A-29 | Key `StepCtx::rng` draws by the drawn entity's epoch (ADR-0025), which needs a typed entity key in the draw API instead of an untyped `u64`. P0 draws at epoch 0 (SPR-005; SPR-006 carry-forward) | Before the first material input that touches a system that draws | Open |
| A-30 | Same-instant conflicts in lifecycle application: timers due at the same (t, phase) fire simultaneously against the same state (SPR-006 G2), so a person can die and act in one slot. When the queue is applied in `(tick, phase, PersonId, seq)` order, a command for a person who died earlier in that application is dropped, with a diagnostic | With the first death command (M2, P1a) | Open |
