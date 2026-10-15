# Architecture Decision Records

Format: *Status · Context · Decision · Alternatives considered · Consequences · Revisit when · Evidence.*
Statuses: **Proposed**, which awaits your sign-off; **Accepted**; **Superseded by ADR-xxxx**.
All ADRs below are **Proposed** until you review them. ADRs that sit on a critical path carry a ★.

| ADR | Title | ★ |
|---|---|---|
| [0001](ADR-0001-core-language.md) | Rust core, Python orchestration, TypeScript console | ★ |
| [0002](ADR-0002-state-model-ecs.md) | State model: SoA tables + pinned bevy_ecs for named actors + SocialGraph store | ★ |
| [0003](ADR-0003-population-ledger.md) | Every person is a ledger row | ★ |
| [0004](ADR-0004-spatial-model.md) | Raster + region graph + shared settlement generator | |
| [0005](ADR-0005-time-model.md) | Integer sim-time, pulses, event-driven agenda, speed bands | ★ |
| [0006](ADR-0006-determinism-contract.md) | Determinism contract | ★ |
| [0007](ADR-0007-chronicle-storage.md) | Typed Parquet chronicle, causal edges, no-delete tiering | ★ |
| [0008](ADR-0008-story-sifting.md) | Custom incremental sifter + SQL twins; LLM non-authoritative | ★ |
| [0009](ADR-0009-drama-policy.md) | Curation, not intervention | |
| [0010](ADR-0010-world-compiler.md) | Mechanics Registry + intent → expander → validators → burn-in; gap reports | ★ |
| [0011](ADR-0011-llm-platform.md) | LLM gateway; llama.cpp local; vLLM/SGLang cluster; frontier batch | |
| [0012](ADR-0012-cognition.md) | Generic psych core + overlays; event-driven IAUS + HTN | |
| [0013](ADR-0013-epistemics.md) | Evidence-typed beliefs; channels; hive as a channel | |
| [0014](ADR-0014-persuasion.md) | Whisper Counsel as ArgumentFrame + deterministic scoring | |
| [0015](ADR-0015-foresight.md) | Foresight = Monte-Carlo forked rollouts with common random numbers | |
| [0016](ADR-0016-console.md) | Web console served by the sim host | |
| [0017](ADR-0017-narrative-compiler.md) | Hierarchical, canon-gated narrative pipeline; style cards | |
| [0018](ADR-0018-3d-client.md) | Godot 4.7+ separate process, Rust gdext, painted miniature, VAT crowds | |
| [0019](ADR-0019-audio.md) | Godot-native systemic audio; licence-vetted TTS | |
| [0020](ADR-0020-scale-out.md) | Single writer per timeline; scale out with timelines | ★ |
| [0021](ADR-0021-licence-policy.md) | Licence allow-list and provenance | |
| [0022](ADR-0022-agentic-development.md) | Spec-first, contract-tested, agent-sized work packages | ★ |
| [0023](ADR-0023-wire-protocol.md) | JSON-RPC control plane + binary columnar state plane | |
