# GEMINI.md: pointer for Gemini-based agents

This repository's rules are tool-neutral, and they bind every agent whatever its vendor (GOV-011 §1). Read these files, in this order, before any work:

1. `HANDOFF.md`: while a handoff is in progress, what is in flight, who owns it and what comes next.
2. `CLAUDE.md`: the project's non-negotiables and every check. The file name is historical; the file binds you.
3. `AGENTS.md`: the briefing and reading order.
4. `CODEX/10_GOVERNANCE/GOV-009` (Git and GitHub), `GOV-010` (CI) and `GOV-011` (agent sessions and handoffs).

The rules you are most likely to break by habit:
- **Never push to `main`.** It moves only by pull request.
- **Never force-push or rebase a pushed branch.** Bring `main` in with `git merge origin/main`.
- **The Architect merges every PR with a merge commit,** pinned to the head SHA that CI passed (`gh pr merge <N> --merge --match-head-commit <sha>`).
- **Never type a generated file** (baselines, goldens, locks, skeletons). Run its tool.
- **Never commit `CODEX/00_INDEX/MANIFEST.yaml`.**
- **Run `bin/check_all.sh` before asking for an audit.** No report, no pass.
- **Merge conflicts in `bin/check_all.sh` are resolved by gate name.** Then run `python -I bin/check_all_selftest.py inventory`.
