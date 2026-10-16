---
id: VER-SPR-NNN
title: "Audit of SPR-NNN"
type: record
status: REVIEW
owner: architect
agents: [architect, tester]
tags: [verification]
related: ["SPR-NNN"]
created: YYYY-MM-DD
updated: YYYY-MM-DD
version: 1.0.0
---

> **BLUF:** <PASS / FAIL>. <one sentence on why; DEF- ids if any; the merge route>.

# VER-SPR-NNN — Audit of SPR-NNN

## 1. Automated checks
| Check | Result | Evidence |
|---|---|---|
| `bin/check_all.sh` (PR tier) | | `artifacts/check_report.md` @ <commit> |
| CI-detected change class | C<n> (declared C<m>) | CI log |
| Golden hashes | unchanged / declared C3 with bisect report | |

## 2. Tester verdicts
| Tester | Contract verified | Verdict | DEF- filed |
|---|---|---|---|
| AGT-003-DET | | | |
| AGT-003-IF | | | |
| AGT-003-STAT | | | |

## 3. Acceptance criteria
| Task | Criterion | Evidence | Met |
|---|---|---|---|

## 4. Compliance table (from the sprint)
| Rule | Required | Found | Met |
|---|---|---|---|

## 5. Contract and ADR conformance
<each binding contract: conforms / DEF- / EVO- (ambiguity)>

## 6. Verdict
| Field | Value |
|---|---|
| Verdict | PASS / FAIL |
| Merge route | auto (C0/C1, additive C2, in-tolerance C3) / director (breaking C2, out-of-tolerance or machinery C3) |
| Escapes found later | <filled in if a defect is found after this PASS; feeds the escape rate> |
