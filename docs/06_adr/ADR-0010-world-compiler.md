# ADR-0010 — Mechanics Registry + intent → expander → validators → burn-in; gap reports

**Status:** Proposed · **Date:** 2026-10-15

## Context

The draft's manifest carried free-string semantics (`consequence_action`, `trigger_event`) and `Dict[str, Any]` fields. Constrained decoding guarantees syntax only. A world can be syntactically valid and still explode within 200 years.

## Decision

- A versioned **Mechanics Registry** is the closed vocabulary.
- The LLM emits a small **Intent** whose enums come from the Registry; it reasons first, then emits the constrained output.
- A deterministic **expander** builds the manifest.
- Validation runs in four stages: schema, referential, semantic (energetics, carrying capacity, vital-rate and horizon guards), then a **burn-in** of 200 years × 8 seeds against stylized facts.
- Failures go to a repair loop of at most 3 rounds.
- Concepts the Registry cannot express produce **gap reports**, which become the agents' backlog.
- Hand-authored YAML manifests are first-class.

## Alternatives considered

| Option | Why not |
|---|---|
| The LLM writes the full manifest | Unreliable, and it wastes tokens. |
| Free-form "rules" interpreted at runtime | Non-executable semantics. |

## Consequences

- The engine's expressiveness grows only by adding to the Registry, which is the S4 loop.
- Compiling a world costs cents. Burn-in costs about a minute of CPU.

## Revisit when

- The Registry grows too large for constrained decoding. Then switch to a two-stage selection: template first, then details.

## Evidence

R05 §5, §Recommendations; R09 §Recommendations; R02.
