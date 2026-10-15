# ADR-0023 — JSON-RPC control plane + binary columnar state plane

**Status:** Proposed · **Date:** 2026-10-15

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
