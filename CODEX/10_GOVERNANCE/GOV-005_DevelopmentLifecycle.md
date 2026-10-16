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
version: 1.3.1
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

- **`main` is the only long-lived branch.** Everything else branches from `main` and returns to it through one pull request.
- **One agent per branch, one branch per sprint.** No per-task branches.
- **Commits** follow `type(SPR-NNN): T-NN description`, then a body with `Why:`, `What:`, `Refs:` (CODEX and `docs` IDs) and the attribution lines the harness requires. One commit per task.
- **Branches are short-lived** and are deleted after merge.
- **Cloud sessions.** A cloud agent session is started on its sprint branch (§4.2). The one exception is the Architect's planning session, whose platform-named branch is merged like any other branch.

### 4.1 Pull requests
- **One pull request per sprint**, from the sprint branch into `main`, opened as a draft by the Developer when the first task lands. CI runs on every push to it.
- **The title** is `SPR-NNN: <title> [C<n>]`. **The body** links the `SPR-` document, carries its compliance table, and ends with the check report verdict.
- **Ready for review** only when `bin/check_all.sh` passes and the Developer's report is attached. The Tester's `VER` evidence and the Architect's audit (`VER-SPR-NNN`) are committed to the sprint branch before merge.
- **Merge method: rebase and merge.** It keeps one commit per task on a straight `main`, which the divergence bisect tool (`docs/13` §4.5) needs.
- **One merge at a time.** The Architect merges, then updates the next waiting branch from `main` and lets CI re-run before merging it. At most one C3 pull request is open for merge at a time.
- **Who presses merge:** §6.

### 4.2 Agent sessions per sprint
| Session | Model | Branch | Does |
|---|---|---|---|
| Architect | the default model | its own | Plans, starts the other sessions, audits, merges |
| Developer (`AGT-002-*`) | the default model | `sprint/SPR-NNN-…` | Implements the tasks; opens the draft pull request |
| Tester (`AGT-003-*`) | **a different model** from the Developer (BCK-002 A-02) | the same sprint branch, test files only, after the Developer marks the PR ready | Writes contract tests from the spec first, then runs them; files `DEF-` items |

Sprints with no dependency between them run in parallel sessions. The Architect starts sessions only for sprints whose `SPR-` is `ACTIVE` (§2: C2/C3 specs need the director's approval first).

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

- **All merges follow `docs/13` §4.4.** On GitHub Free there is no merge queue and no enforced required checks, so its fallback applies: **no green check report, no merge**, kept by the Architect.
- **The Architect merges every class** (director's ruling, 2026-10-16). The conditions rise with the class:
  - **C0/C1:** green CI and the Architect's review.
  - **C2:** green CI, an independent Tester pass and the Architect's audit (`VER-`). A breaking change also carries its migration.
  - **C3:** as C2, plus, once golden worlds exist, the divergence bisect report and the CRN-paired ensemble (`docs/13` §4.2). A C3 change that would alter the history of a golden world **out of tolerance** is announced to the director with its evidence before the merge; the director may veto.
    - A C3 change to a sampler (`rh-sample`) or to its reference generator (`bin/gen_sampler_reference.py`) also needs the nightly tier green on its head before the merge: the PR tier's statistics are too weak to catch a small bias (`docs/09` V-DET-09; VER-SPR-003 GAP-8). Approved by the director at G0, 2026-10-16.
- **The director is told, not asked.** Each merge appears in the Architect's merge summary (the PR, its class, the audit verdict). The director may veto or order a revert at any time.
- **The director still approves** sprint plans for C2/C3 work (§2), ADR changes and anything that reverses an earlier ruling: those are decisions, not merges.
- **No one re-records golden baselines by hand** (`docs/13` §4.2).

## 7. Releases

Rehoboam has no production deployment in phase 1. A **release** is a tagged build that passed a phase gate (`v0.<phase>.<n>`). Saved worlds from earlier releases must load and continue (MOE-4, V-GOLD-04).
