# ADR-0033 — Specs as data: exhaustive, machine-checked system specs (ODD-R v2)

**Status:** Accepted (director's direction on D7, 2026-10-16: "take it to the next level; it will be AI-built, so it can be super complex") · **Date:** 2026-10-16 · **Amends:** `docs/templates/system_spec_odd_r.md` (v1 → v2), `13` §2, ADR-0032 · **Tool:** `specs/systems/lint_odd.py`

## Context

The director accepted ODD-R as the spec form for every simulation system and asked to go further. Agents, not people, will write and read the specs, so the specs can be far more detailed than a human team would keep up.

**That is right, with one distinction:**
- **More detail is cheap for agents only when it stays consistent.** A 30-page prose spec that silently disagrees with the Registry, the code or the tests is worse than a short one. Agents lose consistency as documents grow, and the director (2–4 h a week) cannot review 30 pages per system.
- **"More complex specs" is not "more complex simulation."** Every added mechanism adds parameters to calibrate. Without patterns to constrain them, a model becomes unidentifiable (R12 §14: Grimm 2005, Fagiolo 2007). AI makes mechanisms cheap to write, not true.

## Decision

### 1. The canonical spec is structured data
Each system has `specs/systems/<id>.odd.toml`, written and maintained by agents. It holds every ODD element as fields:
- purpose and patterns;
- owned columns with type, plane and fixed-point flag;
- events emitted and lifecycle commands issued;
- the read set with owner and lag;
- scales and the restart test;
- the fidelity ladder;
- schedule (pulse, phase, steps) and keyed RNG purposes;
- all 13 design concepts;
- initialization and input data;
- parameters with meaning and source;
- submodels, each with the tests that justify it;
- invariants with their tests;
- couplings and verification.

There is no length limit. Exhaustive is the goal.

### 2. A linter checks every spec against everything else (`lint_odd.py`; CI on every PR)
- **The Registry.**
  - The system exists, and its fidelity level is declared.
  - The parameter set is **equal in both directions**: no undocumented Registry parameter, no unregistered spec parameter.
  - Every parameter has a source.
  - The era-state variables exist.
- **Other specs.**
  - **Single writer:** no column is owned twice.
  - Every read resolves to an owner that declares the column.
  - A plane-A system never reads plane-B or plane-C columns.
- **The plan.**
  - Requirement IDs exist in `03`, test IDs in `09`, ADRs in `06`.
  - Phases match `05.1` §4.2, and pulses are valid.
  - Submodel names match schedule steps (ODD's "same names in §3 and §7").
  - RNG purposes are unique, keyed and snake_case.
  - Prototypes exist.
- **Evidence.** Every submodel names the tests or bands that **earn its place**. A mechanism with no pattern to answer to cannot be specified.

### 3. Two outputs are generated from the same source, so they cannot drift
- **`--brief <id>`:** the one page the director reads. It covers what the system does, owns, reads and emits; when it runs; its randomness; its settings with Registry bounds; how we know it works; and what is not reproduced yet.
- **`--rust <id>`:** a code skeleton with the system id, schedule, RNG-purpose enum and parameter struct. Code includes the generated file. A spec change that is not regenerated fails CI.

### 4. Prose remains, for reasoning only
`docs/templates/system_spec_odd_r.md` becomes the optional narrative companion: rationale, derivations, citations. It may explain the data but never contradict it; where they differ, the TOML is the spec.

### 5. Complexity budget: AI may add mechanisms freely, but each must earn its place
A new submodel is accepted only if it does at least one of these:
- moves a calibration band toward its target;
- passes a held-out band that a simpler candidate fails (the `09` §3.8 tournaments);
- is required by a coupling another system's patterns need.

**Otherwise it stays a Registry variant switched off by default.** This keeps "super complex" from becoming "unfalsifiable".

## Alternatives considered

| Alternative | Why not |
|---|---|
| Long prose specs because agents can write them | They drift from the code and the Registry, and no one can review them |
| Keep ODD-R v1 (Markdown tables) | Nothing checks it, so a column could be owned twice or a parameter go undocumented |
| Generate the spec from the code | Inverts spec-first development (ADR-0022); the spec must exist before the code |

## Consequences

- **Prototypes, all passing:**
  - `specs/systems/S5.daily_life.odd.toml` (full);
  - `specs/systems/M2.demography.odd.toml` (rung 0 slice);
  - `specs/systems/_template.odd.toml`;
  - `lint_odd.py`, whose self-check rejects a broken spec with 30 errors.
- **`05.1` §4.2 gains phase 5a `Activity`** for S5 (positions, then encounters, before `Contagion`).
- **Agent workflow:** an agent's first output for a system sprint is the `.odd.toml`. The director approves the generated brief (C2 changes only), and code follows.
- **Warnings list the specs not yet written** (today: S3, M4, M6). They become backlog items.

## Revisit when

- Specs grow so large that agents cannot hold one in context: split per fidelity level.
- The linter's cross-checks miss a class of drift found in review: add the check.
