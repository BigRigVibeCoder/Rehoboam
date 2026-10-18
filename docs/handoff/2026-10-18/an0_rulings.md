## Architect rulings on the AN-0 review (0eb4243; AFTER SMALL FIXES, 0 BLK, 0 MAJ, 7 MIN, 6 NIT; oracle 607 comparisons, 0 mismatches; 28 of 35 mutants killed by check 11)

Every finding is accepted unless a row says otherwise. Owner: F = the fix agent on AN-0's branch; A = the Architect (SPR-029's plan at its next edit).

| # | Ruling | Owner | Conf. |
|---|---|---|---|
| R1 | `load_chronicle` splits on LF only (reuse the lines `_canonical_records` returns), so a record hidden behind a form-feed in a comment is never rendered; a plant in check 11 | F | 90% |
| R2 | Plants for the surviving mutants: free comments inside a block, right after a segment line, and `# STATUS`/`#^` at the end (fold unchanged); a seven-digit year; `y000002` after `y000003`; a one-record world at a tick between the 360- and 365-day year boundaries, cut on both calendars | F | 85% |
| R3 | A differential now: each of `canonical_v0._file_refusals`' planted files must be refused by `fold`. F4's public `read_file` in `canonical_v0.py` waits for that contract's next C2 change | F | 80% |
| R4 | §9.1: a refusal names "the line where the rule has one, and the segment where the rule has one" | F | 90% |
| R5 | Rule 2 reads "no line begins `#segment`". A file with no record is refused ("a chronicle holds at least one record") | F | 80% |
| R6 | Merge `origin/main` and keep both helpers in `bin/check_all.sh` | F | 100% |
| R7 | `fold` stays calendar-free. §9.1 says the digest is a function of the cut, which the exporter fixes by sim-year (`tick // minutes_per_year`). The reference's load path, where the world's calendar is known, refuses a block holding a record outside its year, with a plant. SPR-029 T-03 ports it (A) | F, A | 70% |
| R8 | §9.1: only the exact `#segment` prefix is reserved; other spellings are free comments | F | 80% |
| R9 | The integer grammar is bounded (1–20 digits plus a u64 range check), so overflow is a refusal, not a traceback | F | 85% |
| R10 | `fold` and `segment` turn a missing file or an `AnnalsError` into a clean refusal; §9.1 documents the no-segment output line | F | 85% |
| R11 | One sentence: the first block may start at any seq (canonical's rule); the exporter starts at 0 | F | 85% |
| R12 | SPR-029 T-03's "a one-byte change … changes its digest and the fold" is reworded: a changed record with an unchanged segment line is refused | A | 90% |
| R13 | Declined: the fold stays in `selftest_annals_v0.py`; the AN-1 conflicts are mechanical | — | 60% |
| Order | AN-0 merges before AN-1 | — | 85% |
