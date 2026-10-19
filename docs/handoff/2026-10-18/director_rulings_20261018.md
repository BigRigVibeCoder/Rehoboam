# Director rulings, 2026-10-18 evening

Recorded by the Architect (GOV-011 §5). Relayed by the Director in session.

1. **PR #112:** approved; merged as `736e8e5b`, pinned to head `4717f421`.
2. **Retroactive Tester passes** for SPR-020 and SPR-038a: approved, on a model different from the authors'. `VER-SPR-020-TEST.md` and `VER-SPR-038a-TEST.md` are due before Wave 6 and Wave 7 build on them.
3. **SPR-027:** owner Adjutant Agent 1 (`AGT-002-TOOLS`, VM `adjutant`); branch pushed; do not reassign. **SPR-021a:** unassigned; SPR-019 stage A precedes it (D58 item 5).
4. **Six old `review/*` remote branches:** deleted.
5. **`docs/red_team/games/`:** held four commercial game archives (about 53 GB), not run artifacts. The Director first ruled keep it outside the repo, then ruled delete all the games and all images extracted from them: a dead end, since the viewer is voxel. Deleted with `assets/sprites/` and the game-unpacking work in `scratch/`. `docs/red_team/games/` added to `.gitignore`.
6. **Probe B09-01:** records corrected (it checks a Python spec model, not the engine); the six traps become Rust tests against `rh-core` / `rh-ids`.
7. **`arch/red-team-batch-04`:** no PR from it; `tools/asset_ripper`, the copyrighted PDFs and the viewer HTML stay out (ADR-0036). A clean branch brings only the Batch 09 dossier and probe (PR #113).
8. **Sequence approved:** merge PR #112; this records PR; clean red-team PR; `VER-SPR-038a` backfill and Tester passes; rerun PR #111; A-item triage, Wave 6 briefs, owed plan edits.
9. **Guidance:** Exuberant follows work order rev 2 in order; the G1a 60 s bar does not move; the +62% quiet-instant cost is diagnosed, not baselined away; A-68 is decided after the SPR-030a probe.
