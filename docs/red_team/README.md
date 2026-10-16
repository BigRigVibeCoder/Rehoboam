# docs/red_team

Input from the director's red team: an external reviewer running on a different model family. It is weighed by the Architect and decided by the director.

## How red-team input is handled (director's rule, 2026-10-16)

| Kind | Route | When |
|---|---|---|
| **Defects** in work in flight (a sprint, an open PR) | Straight to the Architect, who routes them to the sprint | Immediately |
| **Design proposals** (new mechanics, architecture, roadmap) | Appended as a batch to `inbox.md` | Folded in by the Architect at the next planning gate, with a dispositions record (`RT-<date>-*.md`). The director decides anything that changes an ADR or the roadmap |

This keeps in-flight sprints from building on specs that change underneath them.

## Contents

| Path | What |
|---|---|
| `inbox.md` | Batched design proposals awaiting the next gate (next: **G0**, end of P0; Batches 01, 02, 03, 04) |
| `DISPOSITION_LEDGER.md` | Master living ledger tracking all 28 findings, Architect dispositions, and phase revisit hooks |
| `PHILOSOPHY_AND_BACKLOG.md` | Red Team Operating Charter, 5 Immutable Laws, 14-item Action Backlog (`RT-ACT-001`–`014`) |
| `RT-2026-10-16-simulation-bug-autopsy-and-code-theft.md` | Deep autopsy of 10 simulation bugs across 7 engines, 10 "still wanted" papers, and open code |
| `RT-2026-10-16-bleeding-edge-frontiers.md` | Bleeding-edge frontiers: Versu practices, Qud domains, Wildermyth grammar, Nobility Thermostat |
| `RT-2026-10-16-sota-science-analysis.md` | Mathematical formulations for Park reflection trees, Kreminski unexpectedness, Relic tensors |
| `RT-2026-10-16-fidelity-gaps.md` | The red team's report on its initial five fidelity prototypes (Batch 01) |
| `RT-2026-10-16-external.md` | Dispositions of the first external review (folded in, closed) |
| `handoffs/` | Historical directives and handoff dossiers kept for auditability |
| `prototypes/` | Nine executable stdlib Python prototypes (`run_all.sh` 9/9 passing) demonstrating engine failure modes and mathematical fixes |
