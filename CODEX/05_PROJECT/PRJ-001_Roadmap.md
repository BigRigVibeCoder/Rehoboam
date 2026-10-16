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
version: 1.0.0
---

> **BLUF:** Rehoboam is a deterministic world engine, from one person to one million, over up to a thousand years. Its world streams out as text that any viewer can tap, and LLMs describe but never decide. Work runs in gated phases P0–P7 and climbs a complexity ladder from one person (`solo`) to 1M. The full roadmap, with every work package, gate test and kill criterion, is `docs/11_roadmap.md`. This page is its front door for agents.

# PRJ-001 — Roadmap

## 1. Vision and success
- **The vision:** `docs/00_executive_summary.md` and `docs/01_vision_and_intent.md`, which hold success criteria S1–S4 and measures MOE-1–6 and MOP-1–5.
- **The thesis tested at G2:** coupled, unforgiving systems produce history you want to follow.

## 2. Phases and gates (details and durations in `docs/11`)

| Phase | Outcome | Gate (decided by the director from `VER-G<n>`) |
|---|---|---|
| **P0** Foundations | Determinism harness, IDs and RNG, samplers, core skeleton, hashing and journal, golden-run CI, benches, Registry v0 | **G0:** identical hashes across threads and repeats; keyframe round-trip; golden vectors; bisect; spec checks; lints; CI ≤ 15 min |
| **P1a** Core time, ledger, demography | Rungs 0–2 running (`solo`, `household`, `hamlet`); World Stream v1; virtual telemetry | **G1a** |
| **P1b** History systems and chronicle | Rung 3 (`village`), then rung 4 (`val_khor`) | **G1b:** stylized facts; ≤ 2 min per 1,000 years at 100k |
| **P2** Sifter and text Time-Machine | `rh-tui`, sifter, pre-registered G2 protocol | **G2 (thesis):** pre-registered rating of history against a coupling-ablated control |
| **P3a / P3b** Minds, epistemics, the remaining systems; Oracle, Foresight, 1M | Rung 5 (1M) | **G3a, G3b** |
| **P4** Narrative and world compilers | | **G4** |
| **P5–P7** Lo-fi Diorama, street-level, travel map (viewers) | | **G5–G7** |

**Kill and pivot criteria:** `docs/11`. If G2 fails three times, the pivot rule applies.

## 3. The complexity ladder (D4)
| Rung | World | Phase |
|---|---|---|
| 0 | `solo`: 1 person, 64×64 valley, every minute authoritative | P1a (P1.0 skeleton) |
| 1 | `household`: ~5 | P1a |
| 2 | `hamlet`: ~60 (`uz_job`) | P1a |
| 3 | `village`: ~1k | P1b |
| 4 | `val_khor`: ~100k | by G1b |
| 5 | 1M | P3b |

A rung is climbed only when everything on the rung below is green. At each rung the director watches the stream (GOV-007 §3.5).

## 4. Where we are
- **Planning is complete:** 33 ADRs; executable specs and their checks pass (`bin/check_all.sh`).
- **The operating model is set up** (SPR-000-ARCH, ADR-0032).
- **Next:** P0, sprints SPR-001 to SPR-009 in `CODEX/05_PROJECT/`.
