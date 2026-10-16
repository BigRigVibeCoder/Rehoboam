---
id: GOV-001
title: "Documentation standard"
type: reference
status: APPROVED
owner: human
agents: [all]
tags: [governance, documentation]
related: [GOV-007, ADR-0032]
created: 2026-10-16
updated: 2026-10-16
version: 1.0.0
---

> **BLUF:** Every document starts with a BLUF. CODEX documents carry YAML frontmatter, and the index (`CODEX/00_INDEX/MANIFEST.yaml`) is generated from it, never written by hand. Design documents in `docs/` keep their current form and are indexed by inference until they next change substantially, when they gain frontmatter. New documents stay under about 10 KB; split rather than sprawl.

# GOV-001 — Documentation standard

## 1. Where documents live

| Area | Contents |
|---|---|
| `docs/` | The plan: vision, requirements, architecture (blueprints), ADRs, V&V catalogue, roadmap, risks, reuse inventory |
| `specs/` | Executable contracts: Registry, schemas, DDL, golden vectors, system specs (`specs/systems/*.odd.toml`) |
| `research/` | Evidence: R-dossiers and the paper library |
| `CODEX/` | Operations: project (PRJ, BCK, SPR), governance (GOV), verification (VER), defects (DEF), evolution (EVO), agents (AGT), archive, templates |

## 2. Frontmatter (required in `CODEX/`; added to `docs/` files when they next change)

```yaml
---
id: SPR-003            # unique; prefix per GOV-007 §2
title: "Plain title"
type: reference | how-to | explanation | tutorial | record
status: DRAFT | ACTIVE | REVIEW | APPROVED | CLOSED | BLOCKED | SUPERSEDED
owner: human | architect | coder | tester
agents: [all] | [architect, coder, tester]
tags: [from §4]
related: [IDs]
created: YYYY-MM-DD
updated: YYYY-MM-DD
version: 1.0.0
---
```

Immediately after the frontmatter comes the BLUF:

> **BLUF:** what this is, why it matters, the key takeaway, in at most 3 sentences.

## 3. The index

- **Generation.** `bin/gen_manifest.py` writes `CODEX/00_INDEX/MANIFEST.yaml` from frontmatter. For `docs/` files without frontmatter, it infers the ID, title, status and summary from the filename, H1, `Status:` line and first paragraph.
- **Staleness check.** `bin/gen_manifest.py --check` fails if the committed index differs from a fresh one. It runs in `bin/check_all.sh`.
- **Agents start from the index:** filter by `id`, `type`, `status`, `tags`, then read only what matches.

## 4. Tags (controlled)

`governance`, `project-management`, `workflow`, `testing`, `coding`, `observability`, `documentation`, `determinism`, `simulation`, `stream`, `sifting`, `validation`, `research`, `infrastructure`, `agents`, `sprint`, `backlog`, `defect`, `evolution`, `verification`, `roadmap`.

The generator rejects unknown tags.

## 5. Writing rules

- Plain, direct English: short sentences; tables and lists over long paragraphs; no filler.
- One idea per paragraph; the BLUF must stand alone.
- Cite sources: `R12 §n`, `ADR-NNNN`, `docs/09 V-…`.
- New documents ≤ about 10 KB. Existing large `docs/` files are split when they next change substantially (ADR-0032).
- Diagrams in Mermaid.

## 6. Lifecycle

- **Status:** `DRAFT` → `REVIEW` → `APPROVED`; sprints run `PLANNING` → `ACTIVE` → `REVIEW` → `CLOSED`.
- **Superseded documents** move to `CODEX/90_ARCHIVE/` with `status: SUPERSEDED` and a pointer to the replacement. Nothing is deleted.
- **Versions:** major for a changed rule, minor for an added rule, patch for wording.
