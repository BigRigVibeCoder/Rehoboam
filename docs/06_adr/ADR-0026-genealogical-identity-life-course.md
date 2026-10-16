# ADR-0026 — Genealogical content-addressed identity; life-course log for every person; signed time

**Status:** Accepted (director, 2026-10-16) · **Date:** 2026-10-15 · **Amends:** ADR-0003

## Context

Baseline v1.0 had two identity problems:

1. **Global birth-serial IDs.** IDs came from one counter, so a single extra birth in a branch shifted every later ID. That has three consequences (RT-C3):
   - It destroys common-random-number (CRN) pairing for actor-level Foresight.
   - It breaks the notion of "the same person" across branches.
   - It leaves immigrants without an ID rule.
2. **Genealogy depended on pins.** The draft kept genealogy only for lineages of already-pinned people. Pinning a commoner today found her grandparents already compacted away, so "genealogy for everyone" was false (RT-M14).

There were also two time problems:
- Founders need birth times before the epoch.
- A `u32` minute birth field overflows after about 8,000 years.

## Decision

1. **PersonId = H64(mother_id, parity)**, i.e. "the same mother's n-th child".
   - Founders: `H64(seed, "founder", k)`.
   - Immigrants or off-map materialisation: `H64(seed, "immigrant", origin, t, k)`.
   - Creatures use the same scheme in their own namespace.
   - **Collision handling.** A collision table is checked at birth. A collision is resolved deterministically by rehashing with a counter. The birthday bound is about 4×10⁻⁵ over 4×10⁷ lifetimes, so collisions are rare but handled.
2. **The live ledger holds the living only.** Dead rows leave immediately.
3. **Life-course log for every person ever born.** It is append-only and stored in Parquet:
   - `(id, mother, father, birth, death, cause)`;
   - interval tables for residence, occupation, faction and marriage.

   At ~40–70 B per lifetime this is ~1.5–2.5 GB per 1,000 years at 1M people, and ~0.2 GB at 100k. Any person's history, however obscure, can be reconstructed. It is plane A.
4. **Minimal partnering in M2 F1**, so that fathers exist.
5. **Flavour taxonomy:**
   - **Inert flavour.** Hash-derived, presentation only, e.g. a turn of phrase. It is plane B once shown.
   - **Behavioural flavour.** Hash-derived *simulation attributes* that plane-A systems also read, e.g. "favourite tavern" read by the errand and venue systems. The 3D world can then never contradict a bio.
   - **Historical claims.** Only from the life-course log or the chronicle, stated *as of* a tick.
6. **Time is `i64` minutes with an epoch offset**, so pre-epoch founder births are representable and there is no millennial overflow.

## Consequences

- CRN pairing survives across intervention rollouts for every unaffected lineage. Test: the correlation of per-person death times across a pair is ≥ 0.9 at 10 years for unaffected lineages.
- Cross-branch diffs (OS-6) and failure-path clustering can match people across timelines.
- "Genealogy for everyone" becomes true.

## Evidence

RT-C3, RT-M14, RT-m1; R01 §R2; R03 §4.
