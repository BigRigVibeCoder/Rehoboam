---
id: DEF-NNN
title: "<what is wrong, in one line>"
type: record
status: ACTIVE
owner: architect
agents: [architect, coder, tester]
tags: [defect]
related: ["<SPR id>", "<contract or test id>"]
created: YYYY-MM-DD
updated: YYYY-MM-DD
version: 1.0.0
---

> **BLUF:** <what fails, which contract clause it violates, severity, and who it is assigned to>.

# DEF-NNN — <title>

| Field | Value |
|---|---|
| Severity | P0 / P1 / P2 / P3 (GOV-007 §4) |
| Found by | <agent or director>, in <SPR or test run> |
| Contract violated | <specs/... or 05.9 IF-x §y or ADR-NNNN §z> |
| Failing test | <V-… id, or the new regression test> |
| Class | Developer error / contract ambiguity → (if ambiguity, the EVO- id) |

## Reproduce
<exact command, seed, manifest, journal position; attach the check report or the bisect report>

## Expected (from the contract)

## Actual (with evidence)

## Fix
- **Regression test first** (GOV-002 law 9): <test name>, which fails before the fix and passes after.
- **Fix sprint:** <SPR or fix branch>.
- **Verified by:** <Tester, VER id>.
