---
id: PRJ-001
title: "Rehoboam — roadmap front page"
type: explanation
status: APPROVED
owner: human
agents: [all]
tags: [roadmap, project-management]
related: [BCK-001, BCK-002, GOV-007]
created: 2026-10-16
updated: 2026-10-16
version: 1.1.0
---

> **BLUF:** Rehoboam is a saga engine (ADR-0035): **it generates legends, a deterministic world whose dynasties, vows, relics and betrayals become sagas you can listen to, read and scrub through.** The engine only engines: its one output is the World Stream, and every presentation tool sits downstream. Work runs in gated phases P0–P5 and climbs a complexity ladder from one person (`solo`) to a kingdom of 10k–50k; 1M is optional. The full roadmap, with every work package, gate test and kill criterion, is `docs/11_roadmap.md`. This page is its front door for agents.

# PRJ-001 — Roadmap

## 1. Vision and success
- **The vision:** `docs/00_executive_summary.md` and `docs/01_vision_and_intent.md`, which hold the north star, the three consumption modes (listen, read, scrub; plus glance), success criteria S1–S4 and measures MOE-1–6 and MOP-1–6.
- **The decision behind the current plan:** ADR-0035 (the saga engine), ratified 2026-10-16.
- **The thesis tested at G2:** coupled, unforgiving systems, with vows and relics in plane A, produce sagas you want to follow.

## 2. Phases and gates (details and durations in `docs/11`)

| Phase | Outcome | Gate (decided by the director from `VER-G<n>`) |
|---|---|---|
| **P0** Foundations | Unchanged: SPR-001 to SPR-010. Determinism harness, IDs and RNG, samplers, core skeleton, fixed point, hashing and journal, golden-run CI, benches, Registry v0 | **G0:** identical hashes across threads and repeats; keyframe round-trip; golden vectors; bisect; spec checks; lints; CI ≤ 15 min |
| **P1a** Bloodlines | Rungs 0–2 (`solo`, `household`, `hamlet`); worldgen-lite; M2 demography; M7 succession and dynasts; minds v0 with stress and mental breaks; the Chronicle Writer with the cinematic event envelope; World Stream v1 | **G1a:** a dynasty lives, dies and inherits deterministically; the templated chronicle reads as a family history |
| **P1b** Legends | Rungs 3–4 (`village`, kingdom ≈ 10k–50k); `S6.artifacts`, `S7.vows`, mythic catalysts, secrets v0; M3 harvest F0/F1, M4 F0, M6 polity-lite | **G1b:** sagas form; plausibility bands hold; V-SF-22 and V-SF-23 reported |
| **P2** Sagas & audio v0 | Sifter, saga pattern packs, the keyboard Time-Machine (`rh-tui`), templated chronicle, the offline audio exporter and reader v0, the pre-registered G2 protocol | **G2 (thesis):** pre-registered rating of sagas against a coupling-ablated control; saga-mechanics ablation reported |
| **P3** The illustrated saga | Narrative compiler, multi-voice audio drama, graphic chronicle, the local Web Graphic Novel / Storyboard Reader with the causal timeline | **G3:** the director listens to and reads a season of their world's saga, rated |
| **P4** Deeper worlds | Full epistemics, contagion, war, M4 F1, M6 F1, Oracle, Foresight, translators, world compiler, persuasion | **G4** |
| **P5** 2D diorama | Glyph layer, Kenney CC0 tile packs, settlement growth view, travel map | **G5** |
| Later / optional | 1M scale-up, M4 F2, era packs, the lo-fi street-level inhabitant | — |
| Cut (ADR-0035) | All 3D, Godot and VR work | — |

**Kill and pivot criteria:** `docs/11`. If G2 fails three times, the pivot rule applies.

## 3. The complexity ladder (D4; rung 4 re-aimed by ADR-0035)
| Rung | World | Phase |
|---|---|---|
| 0 | `solo`: 1 person, 64×64 valley, every minute authoritative | P1a (P1.0 skeleton) |
| 1 | `household`: ~5 | P1a |
| 2 | `hamlet`: ~60 (`uz_job`) | P1a |
| 3 | `village`: ~1k | P1b |
| 4 | **kingdom: ~10k–50k**, 200–2,000 named actors (the default saga world) | P1b, by G1b |
| Later | `val_khor`: ~100k; then 1M | Optional (P3.10) |

A rung is climbed only when everything on the rung below is green. At each rung the director watches the stream (GOV-007 §3.5).

## 4. Where we are
- **Planning is complete:** 35 ADRs (ADR-0035, the saga engine, accepted and ratified 2026-10-16); executable specs and their checks pass (`bin/check_all.sh`).
- **The operating model is set up** (SPR-000-ARCH, ADR-0032).
- **Next:** P0, sprints SPR-001 to SPR-010 in `CODEX/05_PROJECT/`. P0 is unchanged by ADR-0035.
