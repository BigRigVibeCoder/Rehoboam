<!-- GOV-005 §4.1. Title: SPR-NNN: <title> [C<n>] -->

## Sprint
`CODEX/05_PROJECT/SPR-NNN_….md` · change class **C?** · planes touched: …

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
- [ ] Architect merges (C0, C1, additive C2, C3 within tolerance without machinery changes)
- [ ] Director merges (breaking C2, C3 touching the determinism machinery or out of tolerance, anything touching an ADR)
