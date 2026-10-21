# 🐕‍🦺ARCHITECT BACKLOG (`BCK-002` — Continuous Spec Wave Buffer)

> **BLUF:** Ported from `agentic_architect` (`GOV-007 §9.9` & `template_architect_backlog.md`). The Lead Architect Agent (`claude-architect`) is **never idle** while OpenRouter workers implement leaf tasks. Whenever the Forgejo `status:ready` queue drops below the **Low-Water Mark (`< 3` issues)**, `cerberus_orchestrator.py` signals `architect:replenish_needed = 1` and `claude-architect` executes the next wave below.

---

## 1. Architect Continuous Responsibilities (`AGT-001`)

| Category | Code | Trigger Condition | Action Executed by `claude-architect` |
|:--|:--|:--|:--|
| **Wave Spec Prep** | `ARCH-SPEC` | `status:ready < 3` (Low-Water Mark) | Draft next `.spec/active/SPEC-XXX.md`, commit frozen Rust traits & failing test stubs to `main`, spawn 2–10 leaf Forgejo Issues. |
| **Discrepancy Fix** | `ARCH-AMBIG` | `VERDICT: REJECT_SPEC_AMBIGUITY` or `EVO` filed | Clarify ambiguous contract in `.spec/active/SPEC-XXX.md`, update shared crate traits (`rh-core`, `rh-ids`), re-queue leaf issue. |
| **Strike-1 Triage** | `ARCH-STRIKE1` | Worker hits 15-turn / $1.50 fuse | Inspect `wip/issue-N` branch, split overly complex task into two smaller leaf issues or patch the blocker. |
| **Wave Integration Audit** | `ARCH-AUDIT` | All leaf issues in `SPEC-XXX` reach `status:shipped` | Run `python3 cerberus_orchestrator.py audit-wave --spec SPEC-XXX`, generate `VER-SPEC-XXX_WaveAudit.md`, and archive spec. |

---

## 2. Upcoming Spec Wave Queue (`Rehoboam` Master Backlog)

| Wave ID | Spec Target | Domain Crate(s) | Status |
|:--|:--|:--|:--|
| **WAVE-001** | `SPEC-001`: Deterministic Spec-Lint & Demography Births Verification | `specs/demography/*`, `crates/rh-sys-demography/*` | `ACTIVE` |
| **WAVE-002** | `SPEC-002`: Mortality & Cohort Aging Tick Invariants | `crates/rh-sys-demography/*`, `specs/demography/*` | `QUEUED (Replenish Next)` |
| **WAVE-003** | `SPEC-003`: Cross-Subsystem Event Bus Determinism Audit | `crates/rh-core/*`, `crates/rh-ids/*` | `PLANNED` |
