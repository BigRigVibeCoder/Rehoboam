Exuberat (AGT-002-CORE): Proceeding to Task T-03 (Fertility F1)

Excellent work. Commit a61e919 is pushed to GitHub on PR #107 (https://github.com/BigRigVibeCoder/Rehoboam/pull/107), and all 175 tests in rh-core and 58 tests in rh-ids pass completely clean.

You are cleared to proceed to the final task of Stage A:

Task T-03: Fertility F1 in rh-sys-demography
Reference:
CODEX/05_PROJECT/SPR-015_BirthsF1.md and docs/handoff/2026-10-18/exuberant_work_order.md

Implement [algorithm.fertility_f1] (F0–F10) in rh-sys-demography:
- Band function F0 and Coale–Trussell conception probabilities from tfr.
- Monthly susceptible scan in ascending person.id order.
- Elite fertility read on the couple per D56/A-62.
- 9-month pregnancy timer (BIRTH_TIMER), conceived_tick, and unborn_father.
- apply_birth (F9) in Day/Vital timer pass: child_base(mother, parity) with rehash, emitting BIRTH event with roles.
- S12 newborn-aware EXTINCTION check.

Verify Against Reference Tools:
Check parity against:
python3 -I bin/gen_m2_f1_reference.py --check
python3 -I specs/demography/selftest_m2_f1.py
cargo test -p rh-sys-demography

Commit & Push:
Commit: feat(SPR-015): T-03 implement fertility F1 algorithm in rh-sys-demography
Push: git push origin sprint/SPR-015-births-f1

🛑 Hard Checkpoint Rule:
Once T-03 is pushed and local tests pass, STOP IMMEDIATELY. Do not start Stage B (partnering/households). Report completion to the Architect for the Tester interim verification pass (VER-SPR-015-TEST Phase 1).