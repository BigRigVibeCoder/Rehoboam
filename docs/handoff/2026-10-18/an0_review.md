# Independent review of AN-0 (arch/AN-0-annals-fold 0eb4243, on main 99a5b3d; compared with 3d26090)

The review ran on a model different from the drafter's, with no cargo build; nothing in the repository was changed. The Architect recorded this report from the reviewer's returned text. Scratch: `an0_review/work/` (oracle.py, refusal and mutant scripts, 28 CLI files).

## BLUF
**AFTER SMALL FIXES: 0 BLK, 0 MAJ, 7 MIN, 6 NIT.** The contract and the fold are correct on every input built: an independent oracle (from §9.1 and canonical_v0.md §4 alone; stdlib XXH3 and C xxHash 4.0.1) agrees on 607 comparisons, 341 with two or more blocks, 0 mismatches (the drafter's cut; each fixture by sim-year; 600 random cuts with 1–400 records, 1–40 blocks, gaps to y999999, base seq to 2^63, free comments, `#^` and `# STATUS` lines). The nine decisions are sound; R7 is a plan gap. Blocking READY: R1 (a record hidden in a comment is rendered but not covered), check 11 leaves mutants alive (R2, R3), §9.1 over-claims (R4, R5), and a mechanical conflict with main (R6).

## Refusals (planted through the CLI)
All 16 drafter refusals reproduced with their wording, plus: CRLF and lone CR (file-level message, no line), trailing/leading space, tab, seven-digit year, 31-hex digest, `+1`, a duplicate first_seq, seq gaps inside and across blocks, a segment line at EOF, two consecutive segment lines, a header plus one segment line, a moved segment line, non-ASCII, no final LF, `#Segment`/`# segment` (refused only by the previous block's checks). Accepted: a header-only file folds to the empty digest (R5); a relabelled year y000003→y000009 folds (R7); a form-feed inside a comment (R1); a single block starting at seq 5 (R11).

## Mutants (35)
Check 11 kills 28 (one by a crash). The control kills 3 (year rule reversed, line digest not compared, writer hashing the whole file), all also killed by check 11. Survivors: an interior free comment hashed into the block digest; a 7-digit year accepted; a year one lower than the last; a writer with a constant 360-day year; `_canonical_records`' header check dropped; any-length hex (benign); the per-line parse-error wrap dropped (benign).

## Findings
- R1 (MIN, 90%) `selftest_annals_v0.py:655`: `load_chronicle` re-splits with `splitlines()`, which also splits on 0x0B, 0x0C, 0x1C–0x1E; a comment `# note<FF>EVENT …` passes the fold as one comment but is rendered as a record (demonstrated on solitary: the digest unchanged, a second death sentence). Fix: split on LF only; a plant.
- R2 (MIN, 85%): plants for the survivors (interior and trailing comments; 7-digit year; y000002 after y000003; a tick between the 360- and 365-day boundaries on both calendars; a differential on `_canonical_records`).
- R3 (MIN, 75%) `:531–546`: the copied canonical checks have no gate; run canonical's 14 planted file refusals through fold; a public `read_file` at canonical_v0's next C2 change (F4).
- R4 (MIN, 90%) `annals_v0.md:152`: "every refusal names the line and the segment" is not true of file-level, rule-1 and some rule-3 refusals; reword.
- R5 (MIN, 80%) `annals_v0.md:155`, `:616`: rule 2 should say "no line begins `#segment`"; a record-less file folds to 99aa06d3… (recommend refusing it).
- R6 (MIN, 100%): one-hunk conflict in `bin/check_all.sh` with main (instant_log_vectors_stale_control, odd_observes_control vs annals_fold_control); resolved in scratch: bash -n clean, inventory OK 520, inventory pytest 20 passed, --check PASS.
- R7 (MIN, 60%) `annals_v0.md:160`: no tick-vs-year check anywhere; the writer sets year = tick // minutes_per_year (`crates/rh-stream/src/writer.rs:234`), so the digest depends on the cut; add a calendar-aware check where the world is known, or say so in §9.1 and amend T-03.
- R8 (NIT): `#Segment`, `#SEGMENT`, `# segment` are free comments; state it or widen.
- R9 (NIT) `:488–490`, `:562`: unbounded integer grammar (4,301+ digits gives a ValueError traceback).
- R10 (NIT) `:2327`, `:2346`: tracebacks on a missing file and on `segment` of a segmented file; an undocumented "one segment" output line.
- R11 (NIT): the first block may start at any seq; say so.
- R12 (NIT): SPR-029 T-03's "a one-byte change … changes its digest and the fold" is refused under AN-0; reword the plan.
- R13 (optional, 55%): move the fold to its own module to shrink the conflict with AN-1.

## The drafter's F1–F8 and decisions
F1 agree 85%; F2 agree 80%; F3 partly 60% (T-03 does not carry it); F4 agree 75% with a differential now; F5 agree 60%; F6 agree 75% (writer byte-verified; calendar unpinned); F7 agree 75%; F8 agree 80% (AN-1's tree leaves solitary alone). Decisions 1–9: 85, 80, 85, 90, 90, 80, 65, 55, 85%.

## Gates (branch head)
selftest_annals --check PASS (check 11 PASS); gen_fixtures 10 files; lint_templates PASS 29; gen_manifest 217; ruff pass; format 3 files; mypy 64 files; pytest 714 (inventory test 20); inventory OK 486, with planted drops each failing; the 14 annals --control runs reaching check 11 give rc 2 with check 11 PASS; annals_fold_control rc 1 matching the gate's fail text; placement in part core, unscoped.

## Merge and AN-1
One conflict (R6). AN-1's working tree (read-only look) does not overlap AN-0's selftest hunks; it regenerates ashcombe and the goldens but not solitary. Expected AN-0/AN-1 conflicts are mechanical (annals_v0.md line 3, §14's table, the CLAUDE.md annals line, check_all's annals block and REQUIRED_GATES). Recommend AN-0 first.
