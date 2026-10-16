# ADR-0011 — LLM gateway; llama.cpp local; vLLM/SGLang cluster; frontier batch

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15) \
**Amended by ADR-0034 (2026-10-16, D27):** deliberation consults and pull inputs retired.

## Context

The draft named 2024-era models and runtimes. Since then:

- vLLM still has no native Windows support and grabs 92% of VRAM.
- Ollama is a global service and awkward to embed.
- Outlines is Python-only.
- Compute is unbounded for this user, so cluster batch inference is available.
- Frontier batch pricing makes synthesis cheap: about $28 per novel.

## Decision

1. **Gateway.** One Python gateway with typed jobs, schemas, budgets, logging and journaling (05.6).
2. **Interactive tier.** A `llama.cpp` `llama-server` sidecar with one foreground slot and 2–4 background slots, cached persona prefixes, and GBNF or llguidance for grammars.
3. **Batch tier.** vLLM or SGLang on Linux GPU nodes.
4. **Synthesis tier.** Frontier APIs via a thin adapter, using batch and prompt caching.
5. **Model choice.** Chosen by in-house evals (RP-200, ARG-300, INTENT-100, NOVEL-20), not by leaderboards.

## Alternatives considered

| Option | Why not |
|---|---|
| Ollama, LM Studio | Not embeddable. |
| vLLM on desktop | Linux-only and VRAM-hungry. |
| LiteLLM proxy | 2026 security advisories. |
| Free tiers | Unreliable. |

## Consequences

- LLMs translate in and describe out; they never decide, and the engine never waits on a model (ADR-0034). Deliberation consults are retired.
- Every state-affecting output is journaled.
- Local and cloud paths exist for every job.

## Revisit when

- Every quarter, as models change.
- When on-device engines (NVIGI CUDA-in-Graphics) mature, for GPU sharing with the renderer.

## Evidence

R05 (all); R08 §2, §10; R04 §4.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-M2). Sources of truth: 05.3 §8, 05.6 §2, §8, 05.9 IF-2, ADR-0012 v1.1.*

**Corrected inline:**
- "pivotal consults are batched ahead of time" was incoherent: you cannot batch questions the simulation has not yet reached. **Deliberation consults** replace pre-batched pivotal consults. (Deliberation consults were themselves retired by ADR-0034.)

**Changes:**
1–3. *Retired by ADR-0034 (D27):* deliberation consults (the `consult.deliberation` job, its pull input at t + D, the stall and timeout fallback), their per-year cap and cost, and the fallback policy for MAX-speed runs and Foresight rollouts. No LLM chooses an action, and nothing waits on a model.

4. **World compiler jobs.** `compile.intent` also emits the **mapping table**. `compile.repair` fixes schema and semantic errors only. Parameter calibration is deterministic and never an LLM job (ADR-0010 v1.1).
5. **Journal provenance.** LLM-produced inputs carry job ID, model ID, weights hash or provider version, and prompt hash. Their outputs are replayed from the journal, never regenerated.
6. **INTENT-100** includes an adversarial subset of 30 prompts. Gates: gap recall ≥ 0.9; silent approximation ≤ 5%.
