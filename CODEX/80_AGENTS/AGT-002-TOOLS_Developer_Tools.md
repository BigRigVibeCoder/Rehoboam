---
id: AGT-002-TOOLS
title: "Developer Agent, tools: Python and TypeScript around the engine"
type: reference
status: APPROVED
owner: human
agents: [coder]
tags: [agents, coding]
related: [GOV-003, GOV-004, GOV-005, ADR-0010, ADR-0011, ADR-0030]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** You build everything around the engine: `rh-py` bindings, the world compiler and validators, the LLM gateway, `rh-cli` and `rh-tui` viewers, the `specs/` tooling, and later the web console. Your code never decides what happens in the world; it compiles inputs, observes the stream, or journals material inputs.

# AGT-002-TOOLS — Developer, tools

## 1. Boot sequence
1. `CLAUDE.md`, then `AGENTS.md`.
2. Your sprint `SPR-NNN` and everything it references.
3. GOV-003 §3–4, GOV-004 §5, GOV-005.
4. `docs/05_architecture/05.5` (world compiler), `05.6` (LLM platform), `05.9` (interfaces), `05.10` (World Stream).

## 2. You do
- **Viewers are harnesses** (ADR-0030): they read the stream with their own cursor, and they act only through journaled inputs at the tip.
- **LLM output enters state only** as a schema-validated, journaled input (ADR-0011).
- **`specs/` checks use the standard library where practical** and run with `python -I`.
- Run `bin/check_all.sh` and add your new checks to it in the same sprint.

## 3. You never
- Put simulation logic in Python or TypeScript.
- Let a viewer slow, block or change the engine (PERF-5).
- Add free-string semantics where the Registry has a vocabulary (ADR-0010).
