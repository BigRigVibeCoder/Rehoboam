<!-- GOV-009 §4. Title: SPR-NNN: <title> [C<n>] (add [C3] if a baseline under worlds/ changes, [perf-baseline] plus a `perf-baseline-reason:` line if a perf baseline changes) -->

## Sprint
`CODEX/05_PROJECT/SPR-NNN_….md` · change class **C?** · planes touched: …
Agent: <role>, <agent or tool> (the branch's owner; GOV-011 §2)

## Compliance (copy from the sprint document)
| Rule | This change |
|---|---|
| Change class (`docs/13` §4.2) | |
| Planes touched (ADR-0024) | |
| Owned columns written | |
| RNG purposes | |
| Golden-hash effect | |
| Tests that gate it | |

## Tasks
- [ ] T-01 …

## Verification
- `bin/check_all.sh` verdict: …
- Tester evidence: `CODEX/40_VERIFICATION/…`
- Architect audit: `CODEX/40_VERIFICATION/VER-SPR-NNN.md`

## Merge route (GOV-005 §6)
- [ ] The Architect merges (every class; GOV-005 §6) with a merge commit pinned to the green head SHA (GOV-009 §6). Conditions met: CI green on that head; Tester pass (C2/C3); audit `VER-` PASS (C2/C3)
- [ ] Out-of-tolerance C3 on a golden world: announced to the director with evidence before the merge
