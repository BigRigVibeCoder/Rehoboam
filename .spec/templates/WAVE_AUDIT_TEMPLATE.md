---
id: VER-NNN
title: "Spec Wave Audit Checklist (VER-001 Pattern)"
type: reference
status: APPROVED
owner: claude-architect
---

> **BLUF:** Ported from `agentic_architect` (`template_sprint_audit.md` / `/audit_sprint`). Executed automatically via `python3 scripts/cerberus_orchestrator.py audit-wave --spec SPEC-NNN` when all leaf issues for a Spec Wave complete.

# VER-{{SPEC_ID}}: Spec Wave Integration & Governance Audit

- **Spec Wave Audited:** `{{SPEC_ID}}`
- **Audit Timestamp:** `{{TIMESTAMP}}`
- **Lead Auditor:** `claude-architect` + `gemini-verifier`

---

## 1. Workspace Build & Cross-Crate Integration (`GOV-007 §9.5`)
| Check | Command | Status |
|:--|:--|:--|
| Workspace compilation & unit tests | `cargo test --workspace` | `[ ]` |
| Deterministic Python reference specs | `python3 -I specs/...` | `[ ]` |
| Full repository lint & odd-check | `bash bin/check_all.sh` | `[ ]` |

## 2. 5-Gate Governance & Hygiene Compliance (`GOV-001..008`)
| Gate | Authority | Requirement | Status |
|:--|:--|:--|:--|
| **Gate 1: Path & Tier Locks** | `D-14` / `CODEOWNERS` | Zero unauthorized edits to Tier-1 or Tier-2 locked files | `[ ]` |
| **Gate 2: Secret Scan** | `GOV-008` | `gitleaks` + regex scan clean (zero leaked tokens/keys) | `[ ]` |
| **Gate 3: Junk & Ephemeral Cleanliness** | `GOV-005` | Zero `*.pyc`, `*.db-wal`, `*.log`, `.env`, or `target/` files committed | `[ ]` |
| **Gate 4: Determinism & Static Rules** | `GOV-003` / `CONSTITUTION §3` | Zero unseeded `HashMap`/`HashSet`, `SystemTime::now()`, or `#[ignore]` | `[ ]` |
| **Gate 5: 1:1 Test Coverage** | `GOV-002` | Every modified source file has corresponding unit/spec test assertions | `[ ]` |

## 3. Audit Verdict & Archival
- **Verdict:** `PASS / FAIL`
- **Spec Archived:** `.spec/active/{{SPEC_ID}}.md` → `.spec/archive/{{SPEC_ID}}.md`
- **GitHub Mirror Synced:** `YES / NO`
