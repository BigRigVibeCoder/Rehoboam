# Decisions Log

A running log of director rulings, phase-gate outcomes and approved ADR changes. Newest entries go at the top.

| Date | Decision | By | Evidence / links |
|---|---|---|---|
| 2026-10-16 | **Proposed:** fidelity by zoom (ADR-0031). The World Language (Registry vocabulary + typed event grammar + Arrow/Parquet container + generative core); `S5.daily_life` with virtual telemetry for anyone, any day; authoritative daily life by activity resolution per world or focus region, never by viewing. | Systems design, at the director's request | ADR-0031; STR-6, STR-7; V-STR-06–08; `specs/stream/` |
| 2026-10-16 | **Proposed** (from the director's direction, v2): the engine is headless and **streams its world** (the World Stream). Viewing is a buffer event: each viewer reads the stream with its own cursor at its own rate. Interaction is a tip event: actions land at the tip, or at a scheduled future tick, and acting at a past moment is a fork. Pace is the director's setting; no viewer can slow the engine. Phase 1 has no GUI (`rh-cli tail`, then `rh-tui`). | Director; systems design | ADR-0030 v2; `05.10`; STR-1–5; V-STR; PERF-5 |
| 2026-10-15 | **Proposed** (needs your ruling): adopt ODD-R as the mandatory spec format for every simulation system and fidelity level. | Systems design | `docs/templates/system_spec_odd_r.md`; R12 §15 |
| 2026-10-15 | Era order (provisional): E2 early modern → E3 industrial → E4 modern, after G3b, sim-only first, under the ADR-0027 scope guardrails. | Director | `docs/11_roadmap.md` era packs; `12` Q19 |
| 2026-10-15 | Success is defined as a creative engine plus a world to inhabit and invest in, improvable in fidelity and complexity. Commercial success is not the bar. | Director (intake answers) | `docs/01_vision_and_intent.md` §2 |
| 2026-10-15 | The workforce is the director plus AI coding agents. | Director | ADR-0022, `docs/13` |
| 2026-10-15 | There is no compute limit. Fidelity matters, scaling starts low. | Director | ADR-0020, `docs/08` |
| 2026-10-15 | The first playable slice is the Time-Machine plus the Chronicle. | Director | `docs/11_roadmap.md` P0–P2 |
| 2026-10-15 | v1.1 after red-team review: ADR-0024–0026 added; v1.1 amendments to ADR-0002–0018, 0020, 0022, 0023. All 26 ADRs and all amendments are **Proposed**, awaiting director review. | Systems design | `docs/06_adr/` |
| 2026-10-15 | All ADRs 0001–0023 are **Proposed**, awaiting director review. | Systems design | `docs/06_adr/` |
