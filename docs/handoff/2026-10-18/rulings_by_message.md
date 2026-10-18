# Rulings the Architect sent to sprint sessions by message (2026-10-18), not yet in any CODEX record

These rulings bind the sprints below. Each must be copied into that sprint's audit record (`VER-SPR-0NN`) at the audit.

## SPR-038a (stage A of the per-instant cost work; branch `sprint/SPR-038a-per-instant-cost`, head `2116418`, built on SPR-014's `1a67ab9`; no PR yet)

**Interim results reported by the developer** (base `1a67ab9` vs head, one machine):
- The quiet instant: 81,912 → 14,263 instructions, flat from n1 to n10k/l100k, with zero allocations.
- Solo per instant: 97.6k → 20.7k.
- The rungs over 200 years: 584/644/675 → 135/124/121 ms.
- Debug m2_system: 74.9 → 24.6 s.
- M2 gates summed: 153 → 36 s.
- Ladders, run directories, tail and replay are byte-identical.
- kingdom_10k is about 1,530 s per 1,000 years after stage A (from about 2,450). The projection after stage B with `yearly` is 80–117 s: necessary, not sufficient (D60).

**Rulings:**
- **T-02's death target is met in substance (75%).** Restated as: compaction is at most 5% of `death/n10k_*`'s instructions, and the whole death instant is at least 3.5× faster at L = 10^5 (measured: 4.6× at L = 10^4, 3.7× at 10^5; compaction 3.4% of the instant). The remainder belongs in SPR-038b's T-list: the O(N+L) checks at 48% and the guard's O(L) value checks at about 44%.
- **Deviations accepted (80%), with conditions:**
  1. Guard mode 3 now fails `life-course-one-open-entry`: the control's text names it, and a test shows the append is otherwise legal.
  2. The unmarked-put-back mutant is gated by `ledger-no-dead-row`'s differential, failing by message.
  3. `rh_core::REFERENCE`: the A-52 `mutant_build_control` text names both features, and `test_mutant_features.py` covers `REFERENCE`.
  4. Per-slot write stamps: a DECISION comment states the equivalence, and the Tester checks "since it last ran" at a skipped binding and at a resume.
  5. The 2,500-founder size: the test says why smaller sizes cannot see the mutant.
  6. The debug m2_system target goes in the PR body.
- **Order:** SPR-038a merges main after SPR-014 lands, then opens its draft PR; the Tester runs phase 2.
- **Input for its before/after record:** the SPR-014 code review's K5 figures (`spr014_code_review.md`).

## SPR-017 (PR #90, head `34a7f56`; CI green: core 7:21, golden 14:37, stream 5:17; local check_all 609)

- **GAP-5 (Tester pass 2), fix if about 30 lines or fewer:** `rh-cli effect check` refuses an unknown entity-kind param type and a duplicate param name, by name. The developer did it in about 25 lines with three tests (in `34a7f56`).
- **Golden at 14:37 against T-09's 12 minutes (75%):**
  - the fuzz control runs on seeds 1–4 (developer's local commit `7d83b2e`, not pushed at the handoff);
  - per-gate times come from `artifacts/check_report.md`;
  - if golden is still over 12:00 on the next CI run, the two fuzz gates move to part stream under the same names, with the inventory in the same commit;
  - 15:00 is the hard limit.
- **Deviations 1–10 in the done report:** for the audit to rule on. They are: `effect_env` takes the Registry text; rh-registry → rh-events; `StepDraws` keyed on persons and regions until SPR-021a; `SystemViewHost` never answers `Neutral`; M7's `person.id` read declared in the stub; sortition checked against the step's draw order; FLED's test-only office 299; the nightly timeout at 60 min; the mutant build `--features rh-manifest/mutant-effect`; the golden release run and mutants in part golden.
- **The independent code review of `34a7f56` did not finish** (usage limit). Rerun it before the audit.

## SPR-016 (PR #88, draft)
Stage 1 passed its Tester (T-01 Q32_32 exp/ln, T-02 the F1 schedule). Stage 2 is held until SPR-015 merges (wave 5 order).

## SPR-020 (PR #84, draft, head `512432a`)
Waits for SPR-014 to merge, then merges main by gate name and regenerates its baselines in its final `[C3] [perf-baseline]` commit, as its plan says. Its Tester's probe plan is accepted.
