# ADR-0021 — Licence allow-list and provenance

**Status:** Accepted, amended to **personal-use mode** (director, D19, 2026-10-16) · **Date:** 2026-10-15

## Amendment 2026-10-16: personal-use mode (D19)

The director ruled that Rehoboam is a **private experiment that will never be distributed**: not sold, not published, not shared outside the director's own machines. Licence obligations such as GPL source release or non-commercial terms bind distribution or commercial use, neither of which happens here.

**In force from 2026-10-16:**
1. **Any licence may be used**, in any crate, tool, model weight or asset, including GPL, AGPL, LGPL, BSL, non-commercial and research-only licences.
2. **Two limits remain:**
   - a licence whose terms forbid the use itself (for example, a territorial exclusion that covers where the director lives) is not used;
   - code with **no licence** is read for ideas, not copied (no licence grants no permission).
3. **The licence ledger.** `bin/check_all.sh` writes every Rust dependency's licence to `artifacts/licence_ledger.txt`. It never fails a build. It exists so that, if the director ever changes this ruling, the cost of replacing restricted components can be read off at once.
4. **The repository stays private.** It already holds copyrighted research papers.
5. **Supply-chain checks stay blocking:** crates come from crates.io only and never with wildcard versions (`cargo deny check bans sources`). These protect the build, not a licence.

**The trade-off, recorded:** if the ruling is ever reversed, every restricted component in authoritative code (`rh-core`, `rh-sys-*`, `rh-worldgen`, `rh-sifter`) must be replaced or the engine relicensed. Replacing authoritative code changes history hashes, so golden runs and saved worlds would need re-recording (`docs/14`). The ledger keeps that cost visible.

The original decision below is kept as history; it governs only if D19 is reversed.

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
