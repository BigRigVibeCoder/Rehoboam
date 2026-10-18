# The remote sessions' handoff reports (2026-10-18, about 11:55 UTC)

At the handoff, each remote work session pushed its unpushed work and sent the Architect a status report, then stopped. Below is the substance of each report, recorded under GOV-011 §5.

**These are the sessions' own claims. The Architect has not re-run them.** What was checked: each branch's head (`git ls-remote`), and that each head is a fast-forward of its sprint branch's head.

## SPR-014 (lifecycle queues), the Developer
- **Branch:** `backup/SPR-014-dev-handoff-20261018` @ `28ab29e`.
  - The review batch through `1d351f5`: K1, K2, K3, K6, K7, K10+K11 and K13, plus `5ad5288`'s K4, GAP-1 and GAP-3. All byte-neutral; baselines unchanged.
  - Then a clean merge of `main` `3d26090` by gate name: main's `[[observes]]` controls with the Developer's next-version gate; BCK-002 united at 1.21.4; the amendments lock unchanged by the tool.
- **`check_all` on `28ab29e`:** all three parts PASS, 0 FAIL rows. Local times: core 11:43, golden 11:25, stream 7:59.
- **Next:**
  1. Fast-forward the sprint branch: `git push origin 28ab29e:refs/heads/sprint/SPR-014-lifecycle-queues` (no force).
  2. Read CI's per-part times on PR #83.
  3. Delete `backup/SPR-014-pre-audit`.
- The "with a event" message wart stays, as accepted.

## SPR-014, the Tester
- **Branch:** `review/SPR-014-tester-handoff-20261018` @ `b03cc79`. It holds:
  - two probes not pushed before: P-E5b (the spouse cleared by M2's own timer) and P-K4 (an event-keyed draw refused by name);
  - a `HANDOFF.md`;
  - a scratch copy of the report.
- **Already in place:**
  - `VER-SPR-014-TEST.md` is on the sprint branch at `1a67ab9`, for head `d1e79e4`, and accepted;
  - the earlier probes are on `review/SPR-014-tester-probes` @ `b299792`.
- **Not done:** the re-check of K1, K2, K3, K6, K7, K10 and K11 and of the merge of `main`.
- **Survivors to carry to BCK-002:** K4 (its test is pending), M4, M5, M7 and S2.
- **Nightly:** last ran on `921d372` and passed.

## SPR-026 (World Stream v1), the Developer
- **Branch:** `backup/SPR-026-dev-handoff-20261018` @ `2323640`, on the sprint head `2de1841`.
- **C1, code done but not fully verified.** `live.rs` copies at most 64 pointers per append. The window holds:
  - one merged chunk per completed year;
  - the open year's 64-append blocks;
  - its tail.

  The new test `live::an_append_copies_a_bounded_tail…` passes, and `rh-stream live` passes 10 of 10. The `mutant-window-gap` control still FAILs, with I1 broken.
- **The C1 timing does not reproduce the reviewer's figures on that machine.** Headless vs `--socket`, monthly ladder, no viewer:

  | Run | Before | After |
  |---|---|---|
  | solo 3,000 y, tmpfs | 21.0 vs 19.5 s | 18.1 vs 16.8 s |
  | hamlet 6,000 y, tmpfs | 39.7 vs 42.1 s | 39.3 vs 35.7 s |
  | solo 3,000 y, disk (3 pairs) | +10%, −3%, −1% | −1%, +2%, −5% |

  Seals were fast there, so little backlog built up.
- **Recipe for the timings:** `rh-cli run W --years Y --ladder monthly --run-dir D [--socket]`. Put run directories on a short path: a Unix socket path is limited to 107 bytes.
- **Not started:**
  - the part-stream overhead line;
  - the `memory.rs` re-measure;
  - DEF-2;
  - C2–C9 and C12–C20;
  - the merge of `main`.
- **`check_all`:** not run on `2323640`.

## SPR-026, the Tester
- **Branch:** `review/SPR-026-tester-handoff-20261018` @ `041659a`, which adds `tests/handoff/spr026_tester/`. It holds:
  - the probes `p_c1.py`, `p_min.py` and `p_mem3.py`;
  - their helpers `host.py` and `mywire.py`;
  - a README with the "before" readings and the re-check list.
- **Status:**
  - `VER-SPR-026-TEST` is on the sprint branch at `2de1841` (phases 3 and 4);
  - DEF-1 is closed and GAP-8 met;
  - DEF-2 is accepted, pending the Developer's accounting test.
- **Lost with the session:** the other phase-2 probes (codec fuzz, stress, kill/resume, channel C, A-58, twin plants, mutants).

## SPR-017 (effect language), the Developer
- **Branch:** `backup/SPR-017-dev-handoff-20261018` @ `7d83b2e`. Its one commit beyond PR #90's head `34a7f56` runs the fuzz control on seeds 1–4. The control still FAILs as it must (MISMATCH, 10 mismatches).
- **State:**
  - T-01 to T-09 are done on `34a7f56`;
  - CI is green: core 7:21, golden 14:37, stream 5:17.
- **Open:**
  1. Rerun the code review of `34a7f56`.
  2. Take `7d83b2e`. If golden is still over 12:00, move the two fuzz gates to part stream.
  3. Write `VER-SPR-017`, with the audit of deviations 1–10.
  4. The Tester's pass 2 on T-07 to T-09.

## SPR-017, the Tester
- **Branches:**
  - `review/SPR-017-tester-handoff-20261018` @ `0bc28fc`: the report (`VER-SPR-017-TEST` §7, with unfinished parts marked "not run") and probes under `tests/tester_cases/spr017/probes/`;
  - `review/SPR-017-tester-cases` @ `0b42e4c`: the 389-case file.
- **Verified, with no new defect:**
  - 0.5.1 on `5f0312c`: DEF-1 and DEF-2 closed, and 100,000 more fuzz programs agree;
  - T-06 on `661a558`: `check_all` 550 of 550;
  - GAP-5: both cases are refused by name.
- **T-07:**
  - Row order through `SystemView`: 12 shuffle seeds, every line equal.
  - Each of M7's 18 reads dropped in turn: 7 are refused as `UndeclaredRead`, naming the column; the 11 unused ones leave every line identical; no silent zero.
  - The Neutral path cannot be exercised before SPR-014.
- **Finding for SPR-021:** M7 0.3.4 lacks a `person.id` read. The real M7 would fail `UndeclaredRead(person.id)` through the seam until SPR-021 adds it.
- **T-08:**
  - The PR tier (16 seeds, 2,608 cases) passes, with coverage asserted.
  - Seeds 2001–2048, the nightly edge 1017–1024, and seeds 0, 1, 2^32 and 2^64−1 all pass.
  - Through the mutant-effect build, the fuzz alone catches all 12 T-05 mutants.
- **Not run:**
  - the whole of `check_all` on `34a7f56` (a worker restart cut it at check 503, with no failures);
  - T-09 (the drift test, the control that fails by name, and the data-only commit having no `.rs` or `Cargo.*` file);
  - the planted reference mutants, and the Tester's 14 Rust mutants against the fuzz;
  - the ladders and CI times on `34a7f56`;
  - deviations 3 to 5 (the sortition line, office 299, the 60-minute timeout).

## SPR-020 (the event envelope), the Developer
- **Branch:** none pushed. Nothing was unpushed: the local head was `512432a`, equal to `origin/sprint/SPR-020-envelope`.
- **Done:** T-01, T-02, T-03 and T-06, in draft PR #84.
- **CI:** core passes. Golden is red only on the expected gates:
  - V-PERF-11;
  - then the four baselines (fail-fast hides them);
  - and the seven accepted baseline controls.
- **To do:**
  - T-04 and T-05, after SPR-014 merges and `main` is merged in by gate name. The commanded-`DEATH` piece of T-03 lands with T-04.
  - T-07's final `[C3] [perf-baseline]` commit, which:
    - regenerates the baselines;
    - bumps `registry_version`;
    - regenerates the annals goldens;
    - re-records the allocation baseline.
- **Lost with the session:** the done-report notes. Their points are in the Architect's rulings of 2026-10-17.

## SPR-038a (per-instant cost, stage A), the Developer
- **Branch:** `backup/SPR-038a-dev-handoff-20261018` @ `789c138`. It holds:
  - `160be40`, the Architect's conditions (T-04);
  - `789c138`, a handoff commit with the draft PR body, `handoff/SPR-038a_evidence.md`: the tables, the G1a arithmetic, and deviations 1–8.
- **Before → after:**
  - the quiet instant: 81,912 → 14,263 instructions, flat to n10k/l100k, with 0 allocations;
  - solo per instant: 97.6k → 20.7k;
  - the rungs over 200 years: 584/644/675 → 135/124/121 ms;
  - debug `m2_system`: 74.9 → 24.6 s;
  - M2's gates: 153 → 36 s;
  - K5 (M2 only, 3 years), per instant at 100 / 1k / 10k: 16.0/93.4/1,154 → 2.0/16.2/125 µs.
- **Byte identity:**
  - the four ladders equal their baselines;
  - the three rungs' run directories (3,221 files each) have equal SHAs;
  - `tail --canonical` matches, and replay is EQUAL;
  - the four 1,000-year monthly nightly ladders equal the base binary's;
  - `golden_nightly.sh` passes.
- **The Architect's conditions, in `160be40`:**
  - S11 at 3,000 founders, three chunks: the mutant passes at 600 rows or fewer and fails at 3,000;
  - guard mode 4 shows the append is legal for the guard;
  - the stamp DECISION comment;
  - the K5 example.
- **Local `check_all`:** PASS, 564 checks, on `160be40`.
- **Next:**
  1. After SPR-014 merges, fast-forward `sprint/SPR-038a-per-instant-cost` to **`160be40`**, leaving out the handoff commit `789c138`. Take the evidence file's text for the PR body.
  2. Merge `origin/main` by gate name, placing the gates in core or golden under the three-part layout.
  3. Re-record both perf baselines with the tools in a `[perf-baseline]` commit, then run `check_all`.
  4. Open the draft PR `SPR-038a: per-instant engine cost, stage A [perf-baseline] [C2]`.
  5. Get the S15 CI medians: three runs each of head and base.
