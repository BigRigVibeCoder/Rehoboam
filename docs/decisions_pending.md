# Your decisions, in one place

*Updated 2026-10-17: G0 passed (`VER-G0`); D37 (the effect language) and D38 (DATA-5) wait on you. 2026-10-16 (evening): D30 to D36 approved at G0. Earlier: D30 to D36, the P1a plan, waited for G0. Earlier: your rulings (D28: the saga engine). The SPR items below are unchanged. Full reasoning is in [`12_open_questions.md`](12_open_questions.md) and [`06_adr/`](06_adr/README.md). Every ruling is recorded in [`decisions-log.md`](decisions-log.md).*

**How to answer:** reply in chat with the ID and your choice. Anything you don't answer keeps its default, and work proceeds on it.

## Still waiting on you

| ID | Question | Recommendation (default if unanswered) | Needed by |
|---|---|---|---|
| D39 | **Approve EVO-001, the lifecycle-queue contract for P1a (PR #49)?** `05.1` §2 rule 2 v1.10: a command is taken only by the one binding its owner's spec names; a command carries no date (a later outcome is its sender's timer, so DET-13 and ADR-0025 stand as written); the dead-person drop (A-30) only in worlds with a ledger and only for commands that opt in, so the smoke ladder is unchanged; the queue-age invariant (A-31); the absent-owner rule (a `required` read's owner is refused by name, a non-required read or a send is dropped and reported, a `neutral` serves keyed lookups); event-typed payload fields rewritten at the merge. With it: `M7.dynasty` 0.3.2 (a `Day`/`Vital` murder timer), `T1.minds` 0.3.2 and `M2.demography` 0.2.1 (typed `[[applies]]`, enum codes, `required` on reads), the `lint_odd.py` schema and the `docs/09` rows V-DET-21 and V-INV-15. Reviewed independently on a different model (`REV-EVO-001`, rounds 1 and 2) | **Approve.** The alternative (a dated command as the scheduled outcome) amends DET-13 and ADR-0025 and adds a re-sampling path for queued commands; EVO-001 §4 shows the undated form loses no timing. Confidence: 75% | Before SPR-014 (P1.13) and SPR-017's plan |
| D37 | **Approve the Registry effect language v0 (PR #41)?** The typed, total language that succession laws and verb effects are written in: genealogy over M2's life-course log (it reaches through the dead), title blood over the last 16 holders on M7's tenure log, 558 golden cases with an exact reference. It carries three choices: **D14** title blood counts the last 16 holders of the title, which needs worldgen to back-fill pre-epoch history for dynastic worlds (SPR-019); **D4** the sex rules of the succession laws; **D12** the bounds (lineage depth 3, at most 4,096 candidates). Four independent review rounds on a different model from the drafter's ended with 0 blockers and 0 majors (`REV-SPR-017`) | **Approve, with D14, D4 and D12 as drafted.** On approval the Architect merges PR #41 (`effect_language_v0.md`, its golden cases and reference) and SPR-017 can be planned. Confidence: 75% | Before SPR-017 (P1.15); nothing earlier waits on it |
| D38 | **Amend DATA-5 with a tellability ceiling?** DATA-5 says commoner life is recorded as aggregates and the life-course log, not as `event` rows. The proposed rule (`05.2` §4.0, PR #45) admits an act class's events as rows while the world's living population at the start of the year is at or under that class's ceiling (`tellable_up_to`, design value 10,000 in P1a), always admits world-ending verbs (tellability 255), named participants and material inputs, and leaves rarity for P1.14. Every draft still gets an EventId, so nothing in plane A changes; only what the chronicle tells changes. Below the ceiling, commoner births, deaths and marriages become `event` rows, which DATA-5's second sentence forbids, so this is an amendment, not a clarification (the Architect and the independent reviewer, 70%). P1.0 admits every draft regardless (SPR-011 ruling 11) | **Approve the amendment** and record it against DATA-5 in `docs/03`. Alternative: a ceiling of 0 for `death`, `birth` and `marry`, which keeps DATA-5's wording and makes ruling 11 a standing exception. Confidence: 65% | Before P1.9 (the Chronicle Writer); P1.0 does not need it |
| D29 | **Which image provider, and what monthly budget?** The graphic chronicle renders storyboard panels in batch after a saga is compiled (`05.11` §6; `12` Q24). | **A serverless API, in batch, no GPU VM.** Choose among fal.ai, Replicate and Together AI (FLUX.1 [dev] or SDXL) by a two-week trial on the same 50 panels at the start of P3. About $0.005–0.015 per panel, ≈ $1 per season (estimate, verify at P3). Default ceiling: $10 a month. Confidence: medium | Start of P3 (nothing earlier depends on it) |

## Decided on 2026-10-16

| ID | Ruling |
|---|---|
| D30 | **Approved: the P1a plan and the walking skeleton.** `PRJ-002` is the P1a sprint plan. SPR-011 runs as three bounded sprints: SPR-011 engine prerequisites (the purity lint, the 32-bit probability type, life-course log plumbing); SPR-012 `solo` lives and dies (the M2 slice, `rh-cli run`, the golden ladder); SPR-013 World Stream v0 and the Parquet chronicle (`rh-stream`, `rh-sifter` v0, rungs 1–2) |
| D31 | **Approved: rung 0 is ageing and death on day boundaries.** Food (M4) moves to P1b and minute-level daily life (S5 `minutes`) to Later; recorded in `docs/11` and `PRJ-001` |
| D32 | **Approved: the "P1.0 runs at 100k" requirement is withdrawn** (ADR-0035). P1.0 validates N = 1, 5 and 60; the 10k target is SPR-030's (P1.14) |
| D33 | **Approved: `S5.daily_life` F1 (P1.18) and M1 F1 (P1.8) move to P1b,** where the columns they read (M4, M6) have owners |
| D34 | **Approved: one canonical G1a list, with P1a variants** for tests that need unbuilt systems (`PRJ-002` ruling M-7). V-SAGA-16 splits: the stress-accounting identity gates G1a; the break frequency is reported at G1a and gates G1b on the full kingdom ensemble |
| D35 | **Approved: P1a inherits titles and claims only.** Property inheritance comes with `M4.economy` in P1b |
| D36 | **Approved: the `T1.minds` and `M7.dynasty` specs (PR #32) and the skeleton specs (PR #37).** PR #32 merges first, then PR #37. Review findings NEW-6 (`same_instant` reads of `life_course.*`) and NEW-7 (`birth_tick` and `sex` in the log) are in both PRs and in SPR-014's scope |
| D28 | **Decided and ratified: the saga engine** ([ADR-0035](06_adr/ADR-0035-legends-over-ledgers-saga-engine.md)). "Hero vs. wheat": Rehoboam generates legends. Ratified: the new roadmap (P0 unchanged; P1a Bloodlines; P1b Legends; P2 Sagas & audio v0; P3 the illustrated saga; P4 deeper worlds; P5 2D diorama; Later; Cut), ADR-0035, the three guardrails (lean demography stays as the pressure cooker; vows must be able to fail; the G2 thesis gate stays pre-registered and honest), and **all 3D and VR permanently cut**. Later the same day: "the engine only engines"; audio as local MP3 chapters in a local reader, with nothing published or served; the cinematic event envelope; stress and mental breaks; the red team's five gaps (no spontaneous props, the information horizon, immediate succession, fugitives, the "Legends reverse key"); and "the engine simulates truth; downstream tools generate perception" |
| D27 | **Accepted: LLMs translate in and describe out; they never decide.** The deliberation consult is cut ([ADR-0034](06_adr/ADR-0034-llm-roles-translate-describe-never-decide.md); `05.6` §0) |
| D6 (cloud spend) | **No ceiling: you manage Google Cloud spend yourself.** The VM `adjutant` runs only on demand, when simulating or benchmarking, and is shut down otherwise; there is no nightly schedule. The Architect tells you in advance when a VM session is needed and roughly for how long |
| SPR-001 | **Approved** (C2): repository bootstrap and CI |
| SPR-010 | **Approved** (C3): fixed-point and money types in `rh-core::fixed` (DET-3), the P0 gap found by the SPR-001 audit. You also merge it |
| SPR-002–009 | **Approved as a batch:** the rest of the P0 sprint plans. The Architect merges them after each Tester pass and audit (GOV-005 §6) |
| Branches | **Approved:** `main` plus one branch and one pull request per sprint, rebase and merge, one merge at a time; agent sessions per sprint (GOV-005 §4). `main` is the default branch |
| Q-INFRA-1 | GitHub **Free**: no required checks on a private repository, so the Architect enforces "no green report, no merge" by hand |
| Q-INFRA-2 | Coding agents run **in the cloud** |
| Q-INFRA-3 | The laptop has **153 GB free** (enough; keep ≥ 100 GB) |
| VM | Your Google Cloud VM `adjutant` (8 cores, 64 GB) is the **benchmark machine** from SPR-008 (RUN-002), run on demand only (D6) |
| D1 | World Stream (ADR-0030): **accepted** |
| D2 | The 18 critical-path ADRs: **accepted** |
| D4 | First world: **a minimal test world**, `solo` (one person, tiny map); then add complexity one rung at a time (`11` complexity ladder) |
| D5 | **Linux** |
| D6 | **A ThinkPad P51 laptop** (4 cores, 32 GB, Ubuntu 24.04), local first, cloud after local testing. Cloud spend: see D6 (cloud spend) above. Setup: `CODEX/30_RUNBOOKS/RUN-001` |
| D3 / D26 | **Your Agentic Architect operating model, adapted (ADR-0032): accepted.** Set up in `CODEX/` (SPR-000-ARCH). Your time: about 2–4 hours a week outside rating windows |
| D7 | **ODD-R accepted, taken further:** system specs as machine-checked data (ADR-0033), with a generated one-page brief for you |
| D8–D12 | G2 thesis-gate defaults **accepted for now**; revisit before P2 |
| D24, D25 | Fidelity by zoom (ADR-0031) and its default resolutions: **accepted**. Minute-level daily life is allowed at any scale as a batch run (≈ 1.5 h per sim-year at 1M) |

## Defaults in force (no action needed; change any time)

| ID | Decision | Default | Where |
|---|---|---|---|
| D13 | Calendar | 360-day year, configurable per world | Q5 |
| D14 | AI "deliberation" for named characters | Retired by D27 (ADR-0034) | Q7 |
| D15 | May cloud AI services see world data, to write prose? | Yes, never anything identifying you; a local fallback always exists | Q8 |
| D16 | How atrocities are narrated | "Historian" register: consequences, not gratuitous detail | Q9 |
| D17 | Oracle death | Rebirth with heavy loss; permadeath as a hard mode | Q10 |
| D18 | Foresight cost in Cultivator mode | Free; it costs Foresight in Oracle mode | Q11 |
| D19 | Ever distribute it? | **Decided 2026-10-16: never.** A private experiment, so personal-use licence mode (ADR-0021 as amended) | Q13, Q17 |
| D20 | Multiplayer | Out of scope; worlds are shared as manifest + seed + journal | Q14 |
| D21 | Science-fiction frontiers; when per-era art starts | Decide after the first era pack; art packs after P5 | Q★19 |
| D22 | Suspense pauses in Oracle mode | Off until they can use the Oracle's beliefs | Q23 |
| D23 | The 12 non-critical-path ADRs | Approve as proposed when their phase starts | [ADR index](06_adr/README.md) |
