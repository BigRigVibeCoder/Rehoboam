---
id: PRJ-001
title: "Rehoboam — roadmap front page"
type: explanation
status: APPROVED
owner: human
agents: [all]
tags: [roadmap, project-management, sdd]
related: [BCK-001, BCK-002, GOV-007, CONSTITUTION]
created: 2026-10-16
updated: 2026-10-21
version: 2.0.0
---

> **BLUF:** Rehoboam is a saga engine (ADR-0035): **it generates legends, a deterministic world whose dynasties, vows, relics and betrayals become sagas you can listen to, read and scrub through.** The engine only engines: its one output is the World Stream, and every presentation tool sits downstream. Project execution operates strictly under **Specification-Driven Development (SDD v2.0)** on Forgejo Edge. Traditional human sprints (`SPR-001` through `SPR-038`) are archived; active and planned deliverables are tracked as continuous Spec Waves in `.spec/ARCHITECT_BACKLOG.md`. Work runs in gated phases P0–P5 and climbs a complexity ladder from one person (`solo`) to a kingdom of 10k–50k; 1M is optional.

# PRJ-001 — Roadmap

## 1. Vision and success
- **The vision:** `docs/00_executive_summary.md` and `docs/01_vision_and_intent.md`, which hold the north star, the three consumption modes (listen, read, scrub; plus glance), success criteria S1–S4 and measures MOE-1–6 and MOP-1–6.
- **The decision behind the current plan:** ADR-0035 (the saga engine), ratified 2026-10-16.
- **The thesis tested at G2:** coupled, unforgiving systems, with vows and relics in plane A, produce sagas you want to follow.

## 2. Phases, Spec Waves, and Gates

Development execution operates under Specification-Driven Development (SDD v2.0). Each phase maps to discrete Spec Waves (`WAVE-XXX`), and gate ratification is determined by the aggregation of corresponding Wave Audits (`.spec/archive/VER-SPEC-XXX_WaveAudit.md`) and deterministic validation reports.

| Phase | Spec Waves | Outcome | Gate (decided by director from `VER-G<n>` & Wave Audits) |
|---|---|---|---|
| **P0** Foundations | Historical `SPR-001` to `SPR-010` (Archived) | Determinism harness, IDs and RNG, samplers, core skeleton, fixed point, hashing and journal, golden-run CI, benches, Registry v0 | **G0 (PASSED ✅):** Identical hashes across threads and repeats; keyframe round-trip; golden vectors; bisect; spec checks; lints; CI ≤ 15 min (`VER-G0`) |
| **P1a** Bloodlines | **`WAVE-001` to `WAVE-005`** | Rungs 0–2 (`solo`, `household`, `hamlet`); worldgen-lite; M2 demography; M7 succession and dynasts; minds v0 with stress and mental breaks; Chronicle Writer with cinematic event envelope; World Stream v1 | **G1a:** A dynasty lives, dies and inherits deterministically; templated chronicle reads as a family history. Ratified when `VER-SPEC-001` through `VER-SPEC-005` are `PASS` and G1a golden run passes |
| **P1b** Legends | **`WAVE-006` to `WAVE-008+`** | Rungs 3–4 (`village`, kingdom ≈ 10k–50k); `S6.artifacts`, `S7.vows`, mythic catalysts, secrets v0; M3 harvest F0/F1, M4 F0, M6 polity-lite | **G1b:** Sagas form; plausibility bands hold; V-SF-22 and V-SF-23 reported |
| **P2** Sagas & audio v0 | **`WAVE-009+`** | Sifter, saga pattern packs, the keyboard Time-Machine (`rh-tui`), templated chronicle, offline audio exporter and reader v0, pre-registered G2 protocol | **G2 (thesis):** Pre-registered rating of sagas against a coupling-ablated control; saga-mechanics ablation reported |
| **P3** The illustrated saga | Later Spec Waves | Narrative compiler, multi-voice audio drama, graphic chronicle, local Web Graphic Novel / Storyboard Reader with causal timeline | **G3:** Director listens to and reads a season of their world's saga, rated |
| **P4** Deeper worlds | Later Spec Waves | Full epistemics, contagion, war, M4 F1, M6 F1, Oracle, Foresight, translators, world compiler, persuasion | **G4** |
| **P5** 2D diorama | Later Spec Waves | Glyph layer, Kenney CC0 tile packs, settlement growth view, travel map | **G5** |
| Later / optional | TBD | 1M scale-up, M4 F2, era packs, lo-fi street-level inhabitant | — |
| Cut (ADR-0035) | — | All 3D, Godot and VR work | — |

**Kill and pivot criteria:** `docs/11`. If G2 fails three times, the pivot rule applies.

## 3. The complexity ladder (D4; rung 4 re-aimed by ADR-0035)
| Rung | World | Phase |
|---|---|---|
| 0 | `solo`: 1 person, 64×64 valley; ageing and death on day boundaries (D31: its household's food comes in P1b, minute-level daily life Later) | P1a (P1.0 skeleton) |
| 1 | `household`: ~5 | P1a |
| 2 | `hamlet`: ~60 (`uz_job`) | P1a |
| 3 | `village`: ~1k | P1b |
| 4 | **kingdom: ~10k–50k**, 200–2,000 named actors (the default saga world) | P1b, by G1b |
| Later | `val_khor`: ~100k; then 1M | Optional (P3.10) |

A rung is climbed only when everything on the rung below is green. At each rung the director watches the stream (GOV-007 §3.5).

## 4. Execution Model (SDD v2.0)
- **Engine Authority:** Forgejo Edge (`cerberus-runner`) is the sole Git & Issue authority. Development is conducted strictly under Specification-Driven Development (SDD v2.0).
- **Continuous Spec Waves:** Features and subsystems are specified, implemented, verified, and audited in bounded Spec Waves managed via `.spec/ARCHITECT_BACKLOG.md`.
- **Role Separation & Boundary Locks:**
  - **Lead Architect (`claude-architect`):** Maintains `.spec/ARCHITECT_BACKLOG.md`, drafts formal specifications in `.spec/active/SPEC-XXX.md`, freezes shared interfaces/contracts on `main`, and decomposes waves into atomic Forgejo Issue Work Orders.
  - **Workers (`OpenRouter / DeepSeek / Sonnet`):** Work strictly within ephemeral worktrees on assigned leaf issues under strict 3-tier boundary locks enforced by `bin/verify_agent_boundaries.py`.
  - **Verifier (`Gemini CLI`):** Adversarial auditing of PR diffs against frozen contracts prior to merge.
  - **Automated Wave Audits:** Upon completion of all leaf tasks for a wave, the Architect runs automated integration verification and records `VER-SPEC-XXX_WaveAudit.md` into `.spec/archive/`.
- **Legacy Sprint Deprecation:** Legacy human sprint files `SPR-000-ARCH` through `SPR-038` in `CODEX/05_PROJECT/` have been superseded and marked `ARCHIVED`.
