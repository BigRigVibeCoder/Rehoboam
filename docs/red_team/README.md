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

| `inbox.md` | Batched design proposals: batches 01 to 04 were disposed of at G0; batch 05 awaits G1a; batch 06 (stream viewer and compiler). Each batch has the Architect's intake notes |
| `RT-2026-10-16-demography-inequality-science.md` | A science dossier on pre-industrial demography, inequality and empire lifespans (batch 05): calibrated against primary sources |
| `RT-2026-10-16-primary-source-calibration-report.md` | Formal calibration report grounding demography and inequality in published historical papers |
| `RT-2026-10-16-G0-dispositions.md` | The G0 dispositions of batches 01 to 04, with the director's rulings on Laws 2 and 3 and item 15 (2026-10-16) |
| `DISPOSITION_LEDGER.md` | The red team's master ledger of all 34 findings (RT-001 through RT-034), Architect rulings, and phase revisit hooks |
| `RT-2026-10-17-voxel-engine-synthesis-and-architect-analysis.md` | Technical synthesis of the Architect's voxel engine analysis, Bevy vs. Veloren/Luanti trade study, and Decision D51 ratification proposal |
| `PHILOSOPHY_AND_BACKLOG.md` | The red team's operating charter, 5 Immutable Laws, autopsies of seven earlier engines, and 25-item action backlog |
| `RT-2026-10-16-simulation-bug-autopsy-and-code-theft.md` | Autopsy of ten simulation bugs in earlier engines, the "still wanted" papers audit, and algorithms and schemas to reuse |
| `RT-2026-10-16-bleeding-edge-frontiers.md` | Four proposed frontiers: social practices, mythic domains, the comic beat grammar and the dynastic thermostat |
| `RT-2026-10-16-sota-science-analysis.md` | The science behind batch 02: the reflection tree, the unexpectedness metric and the relic corruption tensor |
| `RT-2026-10-16-fidelity-gaps.md` | The red team's report on its five fidelity prototypes (batch 01, for G0) |
| `RT-2026-10-16-external.md` | Dispositions of the first external review (folded in, closed) |
| `handoffs/` | Historical directives and handoff dossiers kept for auditability |
| `prototypes/` | Executable stdlib Python and WebGL prototypes (`run_all.sh`). Demonstrates failure modes, fixes, and SOTA voxel stream visualization |
| `dashboard.html` | Interactive red team tactical roadmap, executive tracker, decision war room, and live simulator showcase |
