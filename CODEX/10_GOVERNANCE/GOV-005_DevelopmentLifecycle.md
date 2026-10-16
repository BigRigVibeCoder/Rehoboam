---
id: GOV-005
title: "Development lifecycle: from conversation to merged, verified work"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, workflow]
related: [GOV-007, GOV-002, ADR-0032, ADR-0033]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** Every piece of work follows one loop: conversation → spec → sprint → verification → audit → merge by change class. Specs come before code. One agent works on one branch per sprint, with one commit per task. Merges follow the change classes in `docs/13` §4.2: routine work merges on green tests plus independent audits; the director approves only contract, determinism-machinery and out-of-tolerance changes. Adapted from `agentic_architect` GOV-005.

# GOV-005 — Development lifecycle

## 1. The loop

```
1. CONVERSATION   director or Architect states a need; the agent asks until scope is clear
2. SPECIFICATION  the agent writes the spec: SPR- (sprint), EVO- (change), DEF- (defect),
                  and for a simulation system the .odd.toml (ADR-0033)
3. SPRINT         one Developer agent, one branch, tasks committed one by one
4. VERIFICATION   the PR tier of GOV-002 (bin/check_all.sh); independent Tester(s) against the contract
5. AUDIT          the Architect audits against contracts and ADRs and writes VER-SPR-NNN
6. MERGE          by change class (docs/13 §4.2) through the merge queue
↻ gaps, defects and ideas re-enter at step 1
```

## 2. Specification rules

- **No code without a spec.** A sprint starts only when its `SPR-` is `ACTIVE`, and for a system sprint, its `.odd.toml` passes `lint_odd.py`.
- **The director approves a spec only for C2/C3 work,** reading the generated brief (`lint_odd.py --brief`) and the sprint's compliance table. C0/C1 specs are approved by the Architect.
- **Specs challenge vague requirements.** "Fast" becomes a PERF budget; "realistic" becomes a band in `09`.

## 3. Sprints

- **Scope-bounded:** a sprint ends when its acceptance criteria pass, at most about 3 agent-days; larger work is split.
- **Status:** `PLANNING` → `ACTIVE` → `REVIEW` → `CLOSED`, or `BLOCKED` (waiting on the director or a contract).
- **Every sprint document has a compliance table** (`_templates/template_sprint.md`):
  - change class;
  - planes touched;
  - owned columns;
  - RNG purposes;
  - golden-hash effect;
  - the tests that gate it.

## 4. Branches and commits

| Work | Branch |
|---|---|
| Sprint | `sprint/SPR-NNN-short-name` |
| Defect fix | `fix/DEF-NNN-short-name` |
| Architect sprint | `arch/SPR-NNN-ARCH-short-name` |

- **One agent per branch, one branch per sprint.** No per-task branches.
- **Commits** follow `type(SPR-NNN): T-NN description`, then a body with `Why:`, `What:`, `Refs:` (CODEX and `docs` IDs) and the attribution lines the harness requires. One commit per task.
- **Branches are short-lived** and are deleted after merge.

## 5. Verification and audit

1. The developer runs `bin/check_all.sh` (the PR tier, fail-fast). It writes a report, and **no report means no pass** (GOV-002 §6).
2. The assigned Tester(s) write and run tests from the contract and file `DEF-` items for failures.
3. The Architect audits with `_templates/template_verification.md` and writes `VER-SPR-NNN`. The audit covers:
   - acceptance criteria;
   - compliance table;
   - contract conformance;
   - test evidence;
   - the change class detected by CI.
4. A failed audit sets the sprint to `BLOCKED` with linked `DEF-` items. A pass sets it to `CLOSED` and queues the merge.

## 6. Merge

- **All merges go through the merge queue** (`docs/13` §4.4).
- **Route by class:**
  - **C0/C1** auto-merge on green CI, a Tester pass and the Architect's audit.
  - **C2** additive changes auto-merge; breaking changes need the director.
  - **C3** within tolerance auto-merges; out of tolerance, or touching the determinism machinery or the golden-world corpus, needs the director.
- **No one re-records golden baselines by hand** (`docs/13` §4.2).

## 7. Releases

Rehoboam has no production deployment in phase 1. A **release** is a tagged build that passed a phase gate (`v0.<phase>.<n>`). Saved worlds from earlier releases must load and continue (MOE-4, V-GOLD-04).
