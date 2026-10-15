# 12 — Open Questions and Decisions Needed From You

*Status: 2026-10-15. Each item lists my recommendation and the default I will assume if you don't answer. Items marked ★ block P0/P1.*

---

| # | Question | Recommendation (default if unanswered) | Why it matters |
|---|---|---|---|
| ★1 | **Approve the core ADRs?** Rust core (0001), every person a ledger row (0003), determinism contract (0006), no hidden director (0009), single writer per timeline (0020). | Approve as proposed. | These decide the architecture's direction. Reversing them later is expensive. |
| ★2 | **Which reference world first?** Feudal + dragons (`val_khor`) or modern + hive (`aethelgard`)? | **Feudal first, without dragons in P1.** Historical calibration data (demography, regicide, secular cycles) is rich, and the 1,000-year horizon fits. Dragons come in P3 (S4 ecology) and the hive in P3 (M5). | It determines which systems and stylized facts come first. |
| ★3 | **Development and play OS?** | Develop on Linux or macOS with CI on all three platforms. Play on whatever you use daily. Name it so wallpaper and packaging can be prioritised. | Affects the determinism guarantee level, the Tauri/Electron choice, the wallpaper host and GPU vendor paths. |
| ★4 | **Compute you actually have?** Local workstation spec, cluster or cloud account, GPU types, monthly cloud budget ceiling. | Assume a strong desktop plus pay-as-you-go cloud for batch LLM work and ensembles, with a $50–200/month soft ceiling until you set one. | Sizes CI runners, Foresight N defaults and model tiers. |
| 5 | **Calendar:** a 360-day simplified year or a realistic 365-day year? | 360-day by default, configurable per world. | Simpler arithmetic and cleaner sifter windows. |
| 6 | **Will you commit to rating sessions at each gate?** (About 1–2 hours per gate.) | Yes. MOE-1 cannot be measured any other way. | Your ratings are the north-star metric and train the preference model. |
| 7 | **Pivotal LLM consults for named actors:** on or off by default? | Off until an A/B shows they raise MOE-1. When enabled, pre-batched before MAX-speed runs. | Trade-off between cost, determinism hygiene and possibly richer decisions. |
| 8 | **Frontier APIs may receive world data** (chronicles, character states) for synthesis? | Yes for synthesis jobs, with a local or cluster fallback always available. No player-identifying data is ever sent. | Privacy and data-handling posture. |
| 9 | **Content boundaries** for narration of atrocities (purges, pogroms, witch-hunts) | "Historian" register: depict consequences, avoid gratuitous detail. Adjustable per world. | Shapes narration prompts, style cards and refusal handling. |
| 10 | **Oracle mortality.** Permadeath, or rebirth into a new "vessel" with lost Foresight? | Rebirth with heavy loss (years of absence, Foresight reset) to keep stakes. Permadeath as a hard mode. | Game-loop design for P3/P7. |
| 11 | **Foresight in sandbox mode.** Free, or costing the Oracle's Foresight? | Two modes: *Cultivator* (free, a developer and analysis tool) and *Oracle* (costs Foresight). | Lets you use compute freely while keeping the game mechanic meaningful. |
| 12 | **Scale ladder.** 100k (P1) → 1M (P3) → beyond? | Hold at 1M until P5. Above 10M needs implicit individuals (ADR-0003). | Avoids premature optimisation. |
| 13 | **Publishing intent, ever?** | Keep the option open: strict licence hygiene (ADR-0021), and clear the trademark before any release. | Commercial use was not chosen as the success bar, but keeping the door open costs little now. |
| 14 | **Multiplayer or shared worlds?** | Out of scope. Worlds can be *exported and shared* as manifest + seed + journal. | Large architectural cost. |
| 15 | **Who reviews agent PRs?** | You approve ADR changes, Registry semantics and gate decisions. Routine PRs auto-merge when all gates are green and a second, independent agent review passes. | Throughput versus control. |

## Questions the research could not settle (to resolve in spikes)
- Godot crowd limits on your hardware → Spike 1.
- TTFT with hybrid-attention local models under rendering load → P5 / P6 spike.
- Belief-store memory at 1M scale → P3 measurement.
- Cross-platform bit-exactness (level 2) → P0/P1 nightly matrix.
- Which local model wins RP-200 on your GPU → P6.4.
