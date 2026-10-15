# ADR-0011 — LLM gateway; llama.cpp local; vLLM/SGLang cluster; frontier batch

**Status:** Proposed · **Date:** 2026-10-15

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

- The simulation never waits on an LLM at MAX speed, because pivotal consults are batched ahead of time.
- Every state-affecting output is journaled.
- Local and cloud paths exist for every job.

## Revisit when

- Every quarter, as models change.
- When on-device engines (NVIGI CUDA-in-Graphics) mature, for GPU sharing with the renderer.

## Evidence

R05 (all); R08 §2, §10; R04 §4.
