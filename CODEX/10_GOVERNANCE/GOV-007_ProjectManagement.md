---
id: GOV-007
title: "Project management: tiers, documents, planning and escalation"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, project-management]
related: [GOV-005, GOV-002, ADR-0032]
created: 2026-10-16
updated: 2026-10-16
version: 1.1.0
---

> **BLUF:** Rehoboam is built by AI agents and governed by one human, the director. CODEX is the project's operating system: Markdown and TOML files that agents read and write. This document sets who decides what, which documents exist, how work is planned (rolling wave, gated phases), and how disagreements between code and spec are resolved. Adapted from the director's `agentic_architect` template (ADR-0032).

# GOV-007 — Project management

## 1. Three tiers

| Tier | Who | Decides | Never does |
|---|---|---|---|
| **1. Director** | The human | Vision; phase-gate decisions; ADRs; contract changes (C2); determinism-machinery and out-of-tolerance changes (C3); escalated ambiguities; G2/G3b ratings | Routine audits, sprint planning |
| **2. Architect Agent** (AGT-001) | AI | Sprint planning from the backlog; audits against contracts and ADRs; defect triage; the discrepancy protocol; CODEX upkeep | Writes engine code; changes contracts without the director; decides scope |
| **3. Developer and Tester agents** (AGT-002-*, AGT-003-*) | AI | Execute sprints; verify against contracts | Change contracts or blueprints; negotiate with each other |

The director's expected load is **2–4 hours a week** outside rating windows (ADR-0032 §5). If it exceeds 4 hours for two consecutive weeks, the Architect raises the auto-merge threshold or adds a second auditor model.

## 2. Document types

| Prefix | What | Where | Created by | Approved by |
|---|---|---|---|---|
| `PRJ-` | Roadmap front page (phases, gates, ladder) | `CODEX/05_PROJECT` → details in `docs/11` | Director | Director |
| `BCK-` | Backlogs: BCK-001 development, BCK-002 architect | `CODEX/05_PROJECT` | Architect | Director (priorities) |
| `SPR-` | Sprint | `CODEX/05_PROJECT` | Architect | Architect; director for C2/C3 specs |
| `DEF-` | Defect (code violates a contract) | `CODEX/50_DEFECTS` | Tester, Architect, Developer | Architect |
| `EVO-` | Proposal to change scope, a contract or a blueprint | `CODEX/60_EVOLUTION` | Any agent, director | Director (C2/C3), Architect (C0/C1) |
| `VER-` | Verification report: a sprint audit or a gate report | `CODEX/40_VERIFICATION` | Tester, Architect | — (evidence) |
| `AGT-` | Agent role and boot document | `CODEX/80_AGENTS` | Architect | Director |
| `GOV-` | Governance | `CODEX/10_GOVERNANCE` | Director (with Architect) | Director |
| `ADR-` | Architecture decision | `docs/06_adr` | Architect | Director |
| Blueprints | Architecture specs; system specs (`specs/systems/*.odd.toml`, ADR-0033) | `docs/05_architecture`, `specs/systems` | Architect, developers | Director for C2 |
| Contracts | Interfaces and executable specs: `05.9` IF-0–IF-9; `specs/` (Registry, manifest schema, chronicle DDL, golden vectors) | `docs/05_architecture/05.9`, `specs/` | Architect | Director |
| Requirements and tests | `docs/03` (requirement IDs), `docs/09` (test catalogue) | `docs/` | Architect | Director |

**Authority order when documents disagree:** executable contracts (`specs/`, checked by tests) > ADRs > `05.9` ICD > blueprints > sprint text. A disagreement is never resolved by choosing; it goes through §5.

## 3. Planning: gated phases, rolling-wave sprints

1. **Phases and gates are planned up front** in `docs/11`: P0–P7, gates G0–G7 with measurable criteria, kill and pivot criteria, and the complexity ladder (rungs 0–5). Durations are estimates, not deadlines.
2. **Sprints are planned one phase ahead:**
   - the current phase's sprints in full (`SPR-` documents);
   - the next phase's sprints sketched as backlog items;
   - later phases as backlog items only.

   Plans further ahead are mostly wrong in a research-driven project.
3. **A sprint is scope-bounded:** one coherent piece of work, at most about 3 agent-days. Anything larger is split.
4. **Gates are decided by the director** from a gate report (`VER-G<n>`) that the Architect assembles from test evidence. A gate cannot pass on argument; it passes on its listed tests.
5. **Rung checkpoints** (the complexity ladder): when a rung's world first runs green, the director watches its World Stream for 15–30 minutes. The Architect records the director's notes as `EVO-` or `DEF-` items.

## 4. Backlog priorities

| Priority | Meaning | Response |
|---|---|---|
| **P0** | `main` is red; a determinism leak; a held-out band was exposed; data loss | Everything stops; the merge queue freezes |
| **P1** | Blocks the current phase's gate | Next sprint |
| **P2** | Needed in this phase, not blocking | When P1 is clear |
| **P3** | Later phase, nice to have, optimisation | Rolling-wave planning |

## 5. The discrepancy protocol

When output and a contract or spec disagree:

1. **The Architect classifies** the mismatch: a developer error, or a contract ambiguity or error.
   - **Developer error:** file a `DEF-`, assign a fix sprint, and re-audit.
   - **Contract ambiguity or error:** stop the affected task, open an `EVO-` with a recommendation, and escalate to the director. Other tasks continue.
   - **Cannot tell:** escalate to the director.
2. **The Architect never guesses.** Escalations bring a recommendation and a confidence rating (1–10), not just a question.
3. **The director's ruling** is recorded in `docs/decisions-log.md`. The contract is updated, and the change propagates to every affected sprint.

## 6. Independence and diversity

- **Testers test the contract, not the developer's description,** and write tests before reading the implementation diff.
- **Testers report to the Architect, never to developers.** No scope bargaining.
- **Where possible, testers and the Architect's audits use a different model or provider from the developers.** AI auditing AI shares blind spots. Objective oracles (hashes, golden vectors, bands, twins) are the final referees.
- **The validation tester never sees held-out bands before a sealed run** (`09` §3.1).

## 7. Metrics the Architect reports at each phase gate

| Metric | Why |
|---|---|
| **Audit escape rate**: defects found by testers or the director after an Architect PASS | Measures the Architect's audit quality; above 10% triggers a second auditor model |
| Director hours per week | Against the 2–4 h budget (R-25) |
| Sprints closed, re-opened and blocked | Flow |
| Open P0/P1 defects | Gate readiness |
| Flaky tests quarantined, and how long | GOV-002 §8 |
| Spec-lint warnings (systems with no spec yet) | Planning coverage |

## 8. Where things live

- **Process:** `CODEX/10_GOVERNANCE`.
- **Rehoboam-specific engineering rules:** `docs/13` (ownership, change classes, CI gates, merge queue, golden-hash divergence).
- **Day-to-day work:** `CODEX/05_PROJECT`.
- **Index:** `CODEX/00_INDEX/MANIFEST.yaml`, generated locally by `bin/gen_manifest.py` and not committed. CI fails if the index cannot be built (GOV-001 §3).

## 9. Modern multi-agent project management guidelines

Modern autonomous software engineering requires multi-agent project management that eliminates deadlocks, race conditions, and context loss:

### 9.1 The Six-Level Work Breakdown Structure (WBS)
Rehoboam operates on a strict hierarchical drill-down from enterprise strategy to atomic code lines:
1. **Phase (P0–P7):** Strategic multi-month horizons defined in `docs/11`.
2. **Phase Gate (G0–G7):** Objective empirical barriers; passes only when all listed test suites and `VER-G<n>` reports pass.
3. **Complexity Rung (0–5):** Progressive empirical scaling ladder (Rung 0: Solo 1-person → Rung 1: Household 5-person → Rung 2: Hamlet 60-person → Rung 3: Manor → Rung 4: Twin Vales 1,500-person → Rung 5: Kingdom 10,000-person).
4. **Wave:** Coordinated clusters of concurrent sprints sharing dependency horizons (e.g. Wave 5 Mortality/Births, Wave 6 Demography/Space, Wave 7 Dynasties, Wave 8 The Crucible, Viewer Lane).
5. **Sprint (SPR-NNN):** Atomic delivery unit (1–3 agent-days) governed by an approved `SPR-` document, compliance table, and dedicated branch.
6. **Task (T-NN):** The lowest granular unit of execution. Exactly one git commit per task (`type(SPR-NNN): T-NN description`), with compiler check verification and bounded scope.

### 9.2 SOTA Multi-Agent Execution Mechanics
- **Asynchronous Work Order Dispatch:** The Architect dispatches work via `INBOX.md` and `INBOX2.md`. Agents execute sequentially without real-time message dependencies.
- **Physical Host Concurrency (Worktree Isolation):** Sibling agents sharing a physical machine or runner work in isolated git worktrees (`.claude/worktrees/*`). Branch checkouts in the repository root are strictly forbidden during active worktree sessions.
- **Draft PR for CI Activation:** Because GitHub Free lacks merge queues and branch rules, Developer agents open a Draft PR on task T-01 to immediately trigger CI runs (`core`, `golden`, `stream`).
- **Immutable Branch Histories:** Pushed branches are never rebased or force-pushed. Upstream changes are brought in via `git merge origin/main`.
- **Merge Commits Pinned to Verified Head:** The Architect merges PRs using merge commits pinned to the exact 40-character commit SHA verified green in CI.
- **Tripartite Agent Guidance Parity:** `.clinerules`, `CLAUDE.md`, and `GEMINI.md` are actively tuned living governance documents maintained in exact parity (GOV-011 §12).

