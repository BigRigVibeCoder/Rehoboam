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
| `inbox.md` | Batched design proposals awaiting the next gate (next: **G0**, end of P0): batches 01 to 04, with the Architect's intake notes |
| `DISPOSITION_LEDGER.md` | The red team's own ledger of its findings and their status. The authoritative record of decisions is `docs/decisions-log.md` |
| `PHILOSOPHY_AND_BACKLOG.md` | The red team's operating charter (approved as its working rules, 2026-10-16), its autopsies of seven earlier engines, and its action backlog `RT-ACT-001` to `RT-ACT-014` (batch 03) |
| `RT-2026-10-16-simulation-bug-autopsy-and-code-theft.md` | Autopsy of ten simulation bugs in earlier engines, the "still wanted" papers audit, and algorithms and schemas to reuse (batch 04, for G0) |
| `RT-2026-10-16-bleeding-edge-frontiers.md` | Four proposed frontiers: social practices, mythic domains, the comic beat grammar and the dynastic thermostat (batch 03, for G0) |
| `RT-2026-10-16-sota-science-analysis.md` | The science behind batch 02: the reflection tree, the unexpectedness metric and the relic corruption tensor (for G0) |
| `RT-2026-10-16-fidelity-gaps.md` | The red team's report on its five fidelity prototypes (batch 01, for G0) |
| `RT-2026-10-16-external.md` | Dispositions of the first external review (folded in, closed) |
| `handoffs/` | The director's and red team's directives as received, kept for the record (the decisions themselves are in ADR-0034, ADR-0035 and `docs/decisions-log.md`) |
| `prototypes/` | The red team's illustrative Python prototypes 1 to 9 (stdlib only; `run_all.sh`). They demonstrate failure modes and proposed fixes with hand-set parameters. They are evidence for a proposal, not measurements of Rehoboam |