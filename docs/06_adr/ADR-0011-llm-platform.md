# ADR-0011 — LLM gateway; llama.cpp local; vLLM/SGLang cluster; frontier batch

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

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

- Deliberation consults are pull inputs due at t + D sim-time. If an answer is missing at t + D, the simulation stalls until it arrives or a timeout applies a journaled fallback. MAX-speed runs and Foresight rollouts use the declared fallback policy.
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
- "pivotal consults are batched ahead of time" was incoherent: you cannot batch questions the simulation has not yet reached. **Deliberation consults** replace pre-batched pivotal consults.

**Changes:**
1. **Deliberation consults** (`consult.deliberation` job).
   - When a consult-enabled decision is due at t, the simulation emits a **pull request** keyed `(actor, decision_type, t, menu_hash)` and continues.
   - The answer, `{action_id ∈ menu, rationale ≤ 40 words}`, is validated and applied as a **pull input at t + D**. D is a fixed sim-time delay per decision type, e.g. 3 days.
   - If no answer has arrived by t + D, the simulation **stalls** (`timeline.stalled`). A configurable timeout applies the deterministic fallback (utility argmax), and **the fallback is journaled**.
   - Replay **hard-fails** on a missing, extra or mismatched pull input.
2. **Off by default, and capped** per simulated year (default ≤ 20 per world). Consults route to the **cluster batch tier**. At the default cap a world-millennium needs about 20,000 calls: about 40M tokens, roughly an hour of one 8-GPU node, or about $40–$120 on a frontier batch API.
3. **MAX-speed runs and Foresight rollouts** use the declared **fallback policy** and are labelled as such.
4. **World compiler jobs.** `compile.intent` also emits the **mapping table**. `compile.repair` fixes schema and semantic errors only. Parameter calibration is deterministic and never an LLM job (ADR-0010 v1.1).
5. **Journal provenance.** LLM-produced inputs carry job ID, model ID, weights hash or provider version, and prompt hash. Their outputs are replayed from the journal, never regenerated.
6. **INTENT-100** includes an adversarial subset of 30 prompts. Gates: gap recall ≥ 0.9; silent approximation ≤ 5%.
