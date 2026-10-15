# ADR-0017 — Hierarchical, canon-gated narrative pipeline; style cards

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

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

- About $28–35 per novel with batching, caching and a cross-family critic (about $28 with R08's same-family critic; about $67 for the premium configuration).
- Quality expectation: consistent and consequential first drafts.

## Revisit when

- Frontier long-form capabilities change materially. Re-run NOVEL-20 quarterly.

## Evidence

R08 (all); R11.

## Amendment v1.1 (2026-10-15)

*Sources of truth: ADR-0024, 05.4 §5–6.*

**Corrected inline:**
- Canon tiers are labelled **CT0–CT3** (previously T0–T3), so they are not confused with representation tiers T0–T4.
- The cost line. $28 is R08's recommended configuration, which uses a **same-family** critic. This ADR specifies a cross-family critic, which costs about $28–35 (05.4 §6.3).

**Changes:**
1. **Plane C.** Compiler output (chronicles, in-world documents, prose and provenance) is plane C. Plane A never reads it.
2. **The facts ledger is CT0 canon.** Facts you were shown (plane B) are as binding on narration as the chronicle record.
