# Tester commission: retroactive passes for SPR-020 and SPR-038a

Issued by the Architect (AGT-001), 2026-10-18, on the Director's ruling of the same day (`director_rulings_20261018.md` item 2). Both sprints merged without an independent Tester pass (BCK-002 A-78). The records are due before Wave 6 and Wave 7 build on them.

## Who

- Role `AGT-003-DET` (determinism and validation) for both, `AGT-003-IF` for SPR-020's schema and interface items.
- **Model:** a different model from each sprint's author (GOV-011 §4). SPR-038a's Developer was a Gemini model; SPR-020's author is named in its PR (#84). The Tester states its model in the record's header in the neutral label form the repository uses.
- **Tester writes tests and records only.** No engine code.

## Rules for both

- Work on a fresh branch from `main` (`review/SPR-NNN-tester-…`, GOV-009), in a worktree. Push only your own branch.
- One cargo-building `check_all` at a time per machine; delete `target/` when done.
- Record findings as in `VER-SPR-NNN-TEST.md` of earlier sprints. A claim not run by you is labelled as not run.
- Do not edit a baseline, golden, lock or skeleton by hand. Report instead.

## SPR-038a (PR #104, merge `4eaf574d`)

Follow section 6 of `CODEX/40_VERIFICATION/VER-SPR-038a.md`, and write `CODEX/40_VERIFICATION/VER-SPR-038a-TEST.md`. In particular:
1. Re-run the instant bench on the merge commit and compare with the PR body's table on one stated allocator.
2. Check write stamps "since it last ran" at a skipped binding and at a resume (ruling item 4).
3. Confirm each mutant control fails on its planted violation.
4. Confirm ladders, run directories, tail and replay are byte-identical to the base `1a67ab9`.
5. Measure the debug `m2_system` gate.

## SPR-020 (PR #84, merge `16a8559`)

Read `CODEX/40_VERIFICATION/VER-SPR-020.md` (the Architect's runs only) and `specs/chronicle/envelope_v0.md`, then write `CODEX/40_VERIFICATION/VER-SPR-020-TEST.md`. In particular:
1. V-DET-22 (input origin across timers, commands, drains and emitted events) and V-DATA-08 (pre-act snapshot oracle) on adversarial inputs the Developer did not write.
2. Coalescing totality and sign saturation at the i64 edges; append refusals by name.
3. Keyframe format v2 and digest v2: kill and resume across a keyframe, `cause_told` across kill and resume.
4. `ChangeWithoutCause` guard: a planted violation must fail by message.
5. Channel A tables (`event_role`, `event_snapshot`, `event_edge`) and the DuckDB typing against the engine's own output.

## Return

When done, post the record's path and branch to the Architect. The Architect then updates `VER-SPR-038a` and `VER-SPR-020` and may mark each audited. Until then neither sprint is described as verified.
