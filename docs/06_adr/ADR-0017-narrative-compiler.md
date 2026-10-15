# ADR-0017 — Hierarchical, canon-gated narrative pipeline; style cards

**Status:** Proposed · **Date:** 2026-10-15

## Context

No model writes a good 100k-word novel in one call; output caps are 128K tokens. Prompts that name an author are legally and ethically risky and judged poor by experts. Grounding fixes plot holes, not prose. LLM judges don't track expert ratings.

## Decision

1. **Forms escalate**: annals (templated) → chronicle → in-world documents → saga → novel. The novel stage only runs if the material passes a story-shape gate.
2. **Pipeline stages**: digest → selection → discourse plan → bible → beats → parallel scenes (batch) → scene QA → critique → book pass → exporters.
3. **Canon tiers CT0–CT3**: CT0 and CT1 contradictions block release. A provenance sidecar records sources.
4. **Style cards** with public-domain exemplars. No living or estate-protected author names.
5. **Models**: Opus 5.5 by default, Fable 5.1 for premium output, a critic from a different model family, and a local NLI checker.

## Alternatives considered

| Option | Why not |
|---|---|
| A single-call novel | Exceeds output caps and produces incoherent long-form text. |
| Named-author prompts | Legal and quality risk. |
| Fine-tuning on authors | The quality gain is real, but it is legally hazardous. |

## Consequences

- About $28 per novel with batching and caching.
- Quality expectation: consistent and consequential first drafts.

## Revisit when

- Frontier long-form capabilities change materially. Re-run NOVEL-20 quarterly.

## Evidence

R08 (all); R11.
