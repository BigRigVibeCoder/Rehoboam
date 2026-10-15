# ADR-0010 — Mechanics Registry + intent → expander → validators → burn-in; gap reports

**Status:** Proposed · **Date:** 2026-10-15 · **Amended:** v1.1 (2026-10-15)

## Context

The draft's manifest carried free-string semantics (`consequence_action`, `trigger_event`) and `Dict[str, Any]` fields. Constrained decoding guarantees syntax only. A world can be syntactically valid and still explode within 200 years.

## Decision

- A versioned **Mechanics Registry** is the closed vocabulary.
- The LLM emits a small **Intent** whose enums come from the Registry; it reasons first, then emits the constrained output.
- A deterministic **expander** builds the manifest.
- Validation runs in four stages: schema, referential, semantic (energetics, carrying capacity, vital-rate and horizon guards), then a **burn-in**. The burn-in is a blocking **liveness gate** (200 years × 8 seeds) plus a background **certification** run (1,000 years × 32 seeds) against stylized facts.
- Schema and semantic failures go to an LLM repair loop of at most 3 rounds. Liveness failures go to **deterministic calibration repair**, never the LLM.
- Concepts the Registry cannot express produce **gap reports**, which become the agents' backlog.
- Hand-authored YAML manifests are first-class.

## Alternatives considered

| Option | Why not |
|---|---|
| The LLM writes the full manifest | Unreliable, and it wastes tokens. |
| Free-form "rules" interpreted at runtime | Non-executable semantics. |

## Consequences

- The engine's expressiveness grows only by adding to the Registry, which is the S4 loop.
- Compiling a world costs cents. The liveness gate takes about 60 s wall on 8 cores at 100k persons (≈ 1 CPU-min per seed). Certification takes ≈ 160 CPU-min in the background.

## Revisit when

- The Registry grows too large for constrained decoding. Then switch to a two-stage selection: template first, then details.

## Evidence

R05 §5, §Recommendations; R09 §Recommendations; R02.

## Amendment v1.1 (2026-10-15)

*Triggered by the red-team review (RT-M9, RT-M10, RT-M12). Source of truth: 05.5 v1.1, especially §2.1, §3, §4.1–4.2 and §6.*

**Corrected inline:**
- The single "200 years × 8 seeds against stylized facts" burn-in is split. A 200-year run cannot see the 1,000-year failure modes (secular-cycle periods, dynasty extinction rates, tail exponents), and a 1,000-year × 32-seed run is too slow to block play.
- The repair loop: the LLM fixes only schema and semantic errors. It never tunes numbers.
- The cost line now states both stages.

**Changes:**
1. **Burn-in stages.**

   | Stage | Runs | Blocks play? | Checks |
   |---|---|---|---|
   | **Liveness gate** | 200 y × 8 seeds | Yes | No extinction or explosion; vital-rate guard; ≥ 2 completed arcs per century; bad-run rate ≤ 1/8; no invariant violations |
   | **Calibration repair** | Morris screening, then Bayesian optimisation or ABC over the flagged parameters | Only when the gate fails | Re-runs the gate; capped at 30 gate runs |
   | **Spin-up** (optional) | 100–200 y | Yes, if chosen | Relaxes the expander's synthetic start. Only the end state is kept, as year 0 |
   | **Certification** | 1,000 y × 32 seeds | **No** (background) | Stylized-fact bands (cycle and phase statistics on N/K, Clauset–Shalizi–Newman tail fits, censoring-aware statistics; calibration vs held-out bands, CI-based pass/fail, `09` §3.1). Each band is tagged *identity* or *emergent* |

   If certification fails after play has started, Genesis labels the world "uncertified" and lists the failing bands. Nothing is rolled back.
2. **Deterministic calibration repair.** Parameters move only **within Registry bounds**. Each changed value is logged with its before and after values and shown on Genesis. The LLM is never in this loop.
3. **Mapping table (RT-M10).** Alongside the Intent, the LLM maps every content-bearing prompt span to Registry IDs, marked **exact**, **approximate** or **unsupported**.
   - An approximate or unsupported row **always auto-files a gap report**. Genesis shows the table before you press run.
   - A deterministic checker verifies that every ID exists and that every ID the Intent uses appears in the table.
   - INTENT-100 includes an adversarial subset of 30 prompts. Targets: gap recall ≥ 0.9; silent approximation (an approximate mapping marked exact) ≤ 5%.
4. **Registry effect language (RT-M12).** A small, typed, total expression language lets **data-only extensions** (laws, overlays, verb effects) land without code.
   - Fixed-point, saturating arithmetic.
   - Iteration only over declared relations, with a declared bound.
   - Reads limited to the declaring system's plane-A read set.
   - Effects only as queued, typed commands.
   - Randomness only through the keyed RNG with a declared purpose.
   - It cannot add columns, clocks or event types. New systems are still code.
5. **Gap reports carry `kind: data_only | needs_code`.** A `data_only` gap is closed by a Registry edit plus tests. A `needs_code` gap becomes an agent work package. Reports are filed in `backlog/gaps/`.
6. **Flavour asset.** Naming seeds, display names and descriptive prose go into `flavour/<world_id>.toml`, which is hashed into the manifest. The naming and chronicle generators read it. A CI lint fails the build if `rh-core` or any `rh-sys-*` crate depends on the flavour asset types, so prose can never change an outcome.
7. **Genesis record.** The hashed manifest and flavour asset are written to the input journal.
