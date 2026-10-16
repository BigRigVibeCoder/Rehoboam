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
version: 1.0.0
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
