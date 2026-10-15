# ADR-0014 — Whisper Counsel as ArgumentFrame + deterministic scoring

**Status:** Proposed · **Date:** 2026-10-15

## Context

The draft injected the player's free text into the target's mind. That is open to prompt injection, LLM-judge bias, and over-persuadable LLM targets. Its targets are also numeric minds, so the text has nowhere to land.

## Decision

The pipeline is **parse → ground → score → apply → show**:

1. **Parse.** A tool-less model at temperature 0, with an independent meta-instruction classifier. The output is a closed-schema `ArgumentFrame`.
2. **Ground.** Claims are checked against what the Oracle knows and against the target's beliefs.
3. **Score.** Deterministic: ELM route × VAF congruence × evidence × trust × habituation × cap × seeded resistance, with a reactance flip.
4. **Apply.** The effect is a bounded Thought modifier.
5. **Show.** The parse is displayed to the player.

Meta-instructions are penalised in-world.

## Alternatives considered

| Option | Why not |
|---|---|
| LLM-as-judge outputs a persuasion score | Biased and injectable. |
| An LLM target "decides" whether it is convinced | Trivially persuadable. |

## Consequences

- The worst case is bounded (SEC-1).
- Players learn the system through the parse preview, which turns persuasion into a skill rather than an exploit.

## Revisit when

- Playtesting shows the parse is too opaque or too gameable.

## Evidence

R04 §5, §4.4, §4.6.
