# ADR-0034 — LLMs translate in and describe out; they never decide

**Status:** Accepted (director, D27, 2026-10-16) · **Date:** 2026-10-16 · **Amends:** ADR-0005, ADR-0006, ADR-0011, ADR-0012, ADR-0015, ADR-0020, ADR-0023, ADR-0024, ADR-0028 · **Spec:** `05.6` §0

## Context

The plan has 19 LLM roles: the job kinds in `05.6` §2 plus the agent workforce. None of them runs before P3b. Only one of them decided anything: the **deliberation consult** (COG-8, ADR-0011 v1.1, `05.3` §8). In it, an LLM picked a named actor's action from a simulation-built menu, and the answer came back as a **pull input** at t + D.

That one role conflicted with four commitments:
- **"The simulation decides; LLMs describe"** (`01` principle 1 and V2; PRJ-001). A model choosing an actor's action is a model deciding.
- **"Nothing slows the engine"** (ADR-0030). A missing consult answer stalled the run at t + D until it arrived or a timeout applied the fallback.
- **The G2 thesis** that coupled *systems* make the history (MOE-2: coupled against coupling-ablated runs). Model-chosen actions blur what the systems produced.
- **Consistent defaults.** ADR-0011, ADR-0012 and D14 had consults off by default; ADR-0028 §2 turned them on by default for `deep` minds.

It also carried a whole mechanism for an off-by-default feature: a second journal class (`Pull`, `PullKey`), wire-protocol additions (`StopCondition::AwaitPull`, `AdvanceReport::Stalled`, `timeline.stalled`), a `DeliberationAnswer` input kind, an `llm_choice` event origin, stall and timeout rules, and replay rules for missing answers. Some of it was due as early as P0 (the journal in SPR-007).

## Decision

**LLMs translate in and describe out; they never decide.** Four roles remain:

| Role | Stage | Phase | Output goes to | Fallback |
|---|---|---|---|---|
| **Workforce:** coding, research and review agents | Build time | Now (ADR-0022, ADR-0032) | Code, specs and Registry entries, through review and CI | People (the director) |
| **Translator in:** whisper parsing (`oracle.parse_argument`, `oracle.meta_classifier`), speech-act extraction (`dialogue.extract_acts`), speech recognition (`asr.*`), the world-compiler front end (`compile.intent`, `compile.repair`) | Runtime, optional | P3b and later | A typed frame, validated against Registry enums and journaled as a **push input**: the player's or author's own act, translated | A menu, a form, or a rule-based parser |
| **Narrator out:** chronicle polish, in-world documents, dialogue realisation (the simulation plans the `SpeechPlan`), saga and novel, sift explanations and re-ranks, TTS | Runtime, optional | P4 and later | Plane C, or plane B for the dialogue facade; read-only | Templates |
| **QA:** claim checks and NLI (`dialogue.claim_check`, `qa.*`) | Runtime, optional | With the narration it gates | A pass, a regenerate or a rejection of narration | Template narration |

The workforce is where LLMs add the most fidelity: they help build richer mechanics from the research. It is not a runtime role.

**Never:**
1. **No LLM output chooses an action, an outcome or a parameter of the simulation.** Named actors decide through the simulation's own utility and HTN machinery (ADR-0012). Speech content is the simulation's `SpeechPlan`. Calibration is deterministic (ADR-0010).
2. **The engine never waits on a model.** No model call sits on the tick's critical path. A translator's output arrives as an ordinary push input at the tip, like any other act.
3. **Replay never re-queries a model** (LLM-2). Translated inputs are read from the journal.
4. **Replay with the gateway disabled gives equal hashes** (V-LLM-06, V-LLM-11).
5. **Only translator outputs are journaled into plane A.** Narrator and QA outputs never are.

**Cut:** the deliberation consult and everything that existed only for it:
- COG-8 and DET-15 (`03`); V-DET-15, V-COG-09 and V-COG-11 (`09`); WP P3.12 (`11`);
- the `consult.deliberation` job (`05.6`);
- the `Pull` input class and `PullKey`; `DeliberationAnswer`; `StopCondition::AwaitPull`; `AdvanceReport::Stalled`; `timeline.stalled` (`05.9`);
- the event origin `llm_choice` (`05.2`);
- ADR-0028's "on by default for deep minds".

It may return only as a possible research experiment after G3b, outside the roadmap, through a new ADR. The requirement, test and WP IDs stay, marked **Withdrawn (ADR-0034)**.

New requirement **LLM-9** (`03`), verified by **V-LLM-11** (`09`).

## Alternatives considered

| Alternative | Why not |
|---|---|
| Keep the consult, off by default (v1.1) | It is still a deciding LLM in the plan, and it still needs the pull-input machinery in the journal, protocol and replay rules from P0 |
| Keep the consult but never stall: apply the fallback at t + D if no answer has arrived | History would then depend on a model's latency: the same seed and the same acts give different journals on a slower cluster. Replayable, but the decision still comes from a model |
| LLM agents for all named actors | Already rejected by ADR-0012: about 2×10¹¹ tokens per millennium, and non-deterministic |
| No runtime LLM use at all | Loses free-text whispers, speech and world prompts, and prose narration, where models serve V2 and V5. Each already has a non-LLM fallback, so keeping them optional and late costs little |

## Consequences

- **The engine never waits on a model**, consistent with ADR-0030's "nothing slows the engine".
- **Every decision comes from the simulation.** The G2 and G3b ratings judge what the systems make.
- **The input journal holds push inputs only.** Replay with the gateway disabled gives equal hashes.
- **Less to build.** No pull class, stall state, timeout fallback or `menu_hash` replay rules. The journal (SPR-007) is simpler from P0.
- **The DDL keeps two reserved columns** for stability of `specs/chronicle/schema.sql` and its SQL-twin self-test: `input_journal.input_class` (always 0, push) and `input_journal.pull_key` (always NULL). Event origin code 2 (formerly `llm_choice`) is reserved.
- **What this supersedes.** Each ADR below keeps its history and carries an "Amended by ADR-0034" line under its status:

| ADR | Section | Change |
|---|---|---|
| ADR-0005 | v1.1 §5 (phase order), §9 (speed and consults) | `Inputs` applies push inputs only; no consult can stall a run |
| ADR-0006 | v1.1 §7 (push and pull inputs) | Push inputs only |
| ADR-0011 | Consequences (first bullet); v1.1 items 1–3 | Consults, their cap and their fallback policy retired |
| ADR-0012 | Revisit when; v1.1 item 1 | Consults retired; a return needs a new ADR after G3b |
| ADR-0015 | v1.1 §8 | Rollouts never call a model, so they need no fallback policy |
| ADR-0020 | v1.1 §3 | "No rollout waits on an LLM" now holds by construction |
| ADR-0023 | v1.1 §2 (input classes), §5 (`timeline.stalled`) | Push only; the stall notification is removed |
| ADR-0024 | §2 ("an accepted LLM choice") | Removed from the paths into plane A |
| ADR-0028 | §2 table (consults on for `deep` minds); Consequences | Retired; every decision of a `deep` mind comes from the simulation |

- **D14** (consult default) is retired by D27.

## Revisit when

- After G3b, a research experiment outside the roadmap shows that model-chosen actions raise MOE-1 in a blind test, without the engine waiting on a model or replay depending on one. It returns only through a new ADR.
- A translator's non-LLM fallback proves unusable in play: improve the fallback; never let the model decide.

## Evidence

- D27 in `docs/decisions_pending.md` (asked 2026-10-16, with the recommendation adopted here); the ruling in `docs/decisions-log.md`.
- The LLM-role inventory behind D27: 19 roles (`05.6` §2 plus the workforce), none before P3b, only one deciding.
- `01` principle 1 and V2; PRJ-001; ADR-0030; MOE-2 and the G2 protocol (`09` §3.4.1).
- ADR-0011, ADR-0012 and D14 (consults off by default) against ADR-0028 §2 (on by default for `deep` minds).
