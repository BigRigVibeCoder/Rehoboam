# ADR-0021 — Licence allow-list and provenance

**Status:** Proposed · **Date:** 2026-10-15

## Context

The draft's "cannibalisation" list included GPL code (Micropolis, Watabou), AGPL code (df-storyteller, AgentTorch, FLAME GPU 2), anti-commercial code (Ceptre), unlicensed code (Felt, Winnow), and non-commercial weights (many TTS models, MeshAnything, Hunyuan3D territory clauses, nvdiffrast).

## Decision

- **Allow-list**: MIT, Apache-2.0, BSD, ISC, MPL-2.0 (file-level), Zlib, CC0, CC-BY (with attribution tracking), and EPL-2.0 for tools only.
- **Conditional, recorded per item**: revenue-capped community licences.
- **Excluded from shipped artifacts**: GPL, AGPL, non-commercial, territorial, and "research-only" licences.
- **Reference-only projects** are read for ideas; their code is never copied.
- **Provenance**: a `provenance.jsonl` entry for every generated asset (model, version, prompt, seed, licence).
- **CI**: a dependency-licence check (cargo-deny, pip-licenses, license-checker).

## Consequences

- Keeps open the option to publish or sell later, even though commercial success is not currently a goal.

## Evidence

R02 §1; R03 claims table; R06 §6; R07 §4.
