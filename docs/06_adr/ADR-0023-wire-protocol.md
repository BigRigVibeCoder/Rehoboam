# ADR-0023 — JSON-RPC control plane + binary columnar state plane

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

The draft streamed entity JSON over JSON-RPC 2.0. JSON-RPC has no sequence numbers, baselines, or resync semantics. JSON becomes GDScript Dictionary churn on the client and is about 9× larger than columnar binary (22.6 KB versus 2.6 KB per 200-entity frame). Bandwidth is not the real problem on localhost; semantics and client CPU are.

## Decision

One WebSocket per client carrying two kinds of frame:

- **Text frames**: JSON-RPC 2.0 for control, queries and notifications.
- **Binary frames**: versioned columnar state frames.
  - A full frame every 1 s, previous-frame deltas otherwise.
  - Interest management on a 25 m grid with hysteresis.
  - Transforms at 20 Hz, with verbs and waypoints for client-side interpolation.

The frame format is transport-agnostic, so an in-process GDExtension mode can pass the same bytes later.

## Alternatives considered

| Option | Why not |
|---|---|
| FlatBuffers or Cap'n Proto | Viable, but Godot-side tooling is community-maintained; a fixed columnar layout is simpler. Revisit if the schema grows. |
| WebTransport | No advantage on localhost. |
| Shared memory | Platform-specific; use it only if profiling demands. |

## Evidence

R10 §5, §R5; R06 §8.

## Amendment v1.1 (2026-10-15)

*Sources of truth: 05.9 v1.1, ADR-0024, ADR-0025, ADR-0026. No inline corrections. The transport decision stands. The protocol content gains:*

1. **Plane hashes.** IF-1 `plane_hashes()` returns `{state (A), observation (B), narrative (C)}`. Every method except `advance_until` and `apply_input` leaves `state` unchanged. Observation APIs may change plane B only.
2. **Push and pull inputs.** `JournaledInput.class` is `Push` or `Pull { key: PullKey }`, with `PullKey = (actor, decision_type, t_asked, menu_hash)`. Pull inputs apply at `t_asked + D`. Replay hard-fails on a missing, extra or mismatched pull input. Material inputs carry a dependency `scope`. The timer re-sample key uses each entity's `epoch`, a plane-A counter of the material inputs that have touched it. The epoch is derived state, so it is not journaled.
3. **No input in the past.** `apply_input` with `tick < now()` is refused (`InputError::PastInput`). `timeline.seek` is read-only. A material input sent while seeked into the past forks first and returns the new `timeline_id`.
4. **New control methods.**
   - `console.set_mode(cultivator | oracle)` and `console.mode()`. Mode changes are journaled as `ConsoleMode`.
   - `world.liveness_report` and `world.certification_report`.
   - `compile.mapping` and `compile.calibration_log`.
   - `entity.life_course`. In Oracle mode, `entity.*` results are filtered by the Oracle's knowledge.
5. **New notifications.** `timeline.stalled{pull_key}` when a deliberation answer is due and missing (`AdvanceReport::Stalled`), and `timeline.forked{from, at, timeline_id}`.
6. **Signed ticks.** `SimTime` and the frame header's `tick` are **`i64`** minutes (ADR-0026). The header stays 32 bytes.
