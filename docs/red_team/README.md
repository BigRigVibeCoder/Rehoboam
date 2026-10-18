# docs/red_team

Input from the director's red team: an external reviewer running on a different model family. It is weighed by the Architect and decided by the director.

## How red-team input is handled (director's rule, 2026-10-16)

| Kind | Route | When |
|---|---|---|
| **Defects** in work in flight (a sprint, an open PR) | Straight to the Architect, who routes them to the sprint | Immediately |
| **Design proposals** (new mechanics, architecture, roadmap) | Appended as a batch to `inbox.md` | Folded in by the Architect at the next planning gate, with a dispositions record (`RT-<date>-*.md`). The director decides anything that changes an ADR or the roadmap |

This keeps in-flight sprints from building on specs that change underneath them.

**Intake (director's rule, 2026-10-16):** the Architect decides whether a batch enters the repository. The batch arrives on a branch and a PR. The Architect checks it, notes where it conflicts with ratified contracts or overstates its evidence, and merges it into `docs/red_team/`. Any batch the Architect is not confident about, or believes is mistaken, goes to the director first. Entering the repository adopts nothing; adoption happens at the gate.

## Contents

| Path | What |
|---|---|
| `RT-2026-10-18-batch-07-dispositions.md` | The Architect's dispositions of the red team's batch 07 ("reality check", RT-035 to RT-038, on its branch): the stream bridge not adopted, the control bridge declined before P2.6, the manor population band adopted into SPR-019 T-07, three ledger rows corrected, and an action on `adjutant`'s port 8089 |
| `RT-2026-10-17-voxel-engine-synthesis-and-architect-analysis.md` | The red team's synthesis of the Architect's voxel-viewer analysis (Bevy against Veloren and Luanti) and its D51 recommendation; D51 was approved, and the decision is ADR-0036 and PRJ-003 |
| `RT-2026-10-16-proposal-voxel-compiler-and-culture-dreamer.md` | The red team's proposal (drafted as "ADR-0036" on its branch): a stream-to-voxel shape grammar and an LLM "culture dreamer" for style cards; with the Architect's intake note placing it in the viewer lane (V.1, proposed V.1b) |
| `RT-2026-10-16-proposal-stream-viewer-and-websocket.md` | The red team's proposal (drafted as "ADR-0037" on its branch): a browser stream viewer over WebSocket; with the Architect's intake note mapping it to `wire_v1` (SPR-026), the pace command (SPR-027), `rh-server` (P2.6) and P2.7 |
| `RT-2026-10-16-plan-matrix-render-engine.md` | The red team's multi-scale viewport and asset-extraction plan; Pillar 2 (asset extraction) is held under D53; the intake note places Pillar 1's tiers in P2.7, P5 and the viewer lane |
| `inbox.md` | Batched design proposals: batches 01 to 04 were disposed of at G0; batch 05 awaits G1a. Each batch has the Architect's intake notes |
| `RT-2026-10-16-demography-inequality-science.md` | A science dossier on pre-industrial demography, inequality and empire lifespans (batch 05): a proposal, not evidence, until its sources are read |
| `RT-2026-10-16-G0-dispositions.md` | The G0 dispositions of batches 01 to 04, with the director's rulings on Laws 2 and 3 and item 15 (2026-10-16) |
| `DISPOSITION_LEDGER.md` | The red team's own ledger of its findings and their status. The authoritative record of decisions is `docs/decisions-log.md` |
| `PHILOSOPHY_AND_BACKLOG.md` | The red team's operating charter (approved as its working rules, 2026-10-16), its autopsies of seven earlier engines, and its action backlog `RT-ACT-001` to `RT-ACT-014` (batch 03) |
| `RT-2026-10-16-simulation-bug-autopsy-and-code-theft.md` | Autopsy of ten simulation bugs in earlier engines, the "still wanted" papers audit, and algorithms and schemas to reuse (batch 04, for G0) |
| `RT-2026-10-16-bleeding-edge-frontiers.md` | Four proposed frontiers: social practices, mythic domains, the comic beat grammar and the dynastic thermostat (batch 03, for G0) |
| `RT-2026-10-16-sota-science-analysis.md` | The science behind batch 02: the reflection tree, the unexpectedness metric and the relic corruption tensor (for G0) |
| `RT-2026-10-16-fidelity-gaps.md` | The red team's report on its five fidelity prototypes (batch 01, for G0) |
| `RT-2026-10-16-external.md` | Dispositions of the first external review (folded in, closed) |
| `handoffs/` | The director's and red team's directives as received, kept for the record (the decisions themselves are in ADR-0034, ADR-0035 and `docs/decisions-log.md`) |
| `prototypes/` | The red team's illustrative Python prototypes 1 to 10 (stdlib only; `run_all.sh`). They demonstrate failure modes and proposed fixes with hand-set parameters. They are evidence for a proposal, not measurements of Rehoboam |