# ADR-0014 — Whisper Counsel as ArgumentFrame + deterministic scoring

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

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

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-M13, RT-m3). Sources of truth: 05.2 §3.2, 05.3 §6, 05.9 IF-2, IF-3, ADR-0013 v1.1. No inline corrections.*

**Changes:**
1. **"What the Oracle knows" is the Oracle's plane-A belief store** (ADR-0013 v1.1). Grounding labels each claim verified, lie or bluff against that store. It also checks the claim against the target's beliefs: consistent, contradicting or novel.
2. **Show** includes the target's reaction only if the Oracle knows it.
3. **Persuasion balance (RT-m3).**
   - **Oracle leverage** is ΔP (Foresight, common random numbers) per whisper, per unit of Foresight spent.
   - **Target bands**, e.g. 5–15 points for a strong, verified whisper on a receptive target, and under 3 for a weak or bluffed one.
   - **Parameter sweeps** tune the scoring constants against those bands.
   - An **adversarial player bot** maximises ΔP per Foresight across red-team texts and argument strategies. It runs as a **CI gate**: no strategy may exceed the cap envelope.
4. **Console modes.** Oracle verbs are enabled in **Oracle** mode. In **Cultivator** mode they are disabled by default; if enabled, they are journaled as `omniscient`.
5. **Journaling.** A whisper is a journaled material input (`OracleWhisper{frame}`) with a dependency scope (05.2 §3.2). The console shows the parse before you confirm it (`oracle.whisper` → parse preview → `confirm`).
