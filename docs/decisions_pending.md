# Your decisions, in one place

*Updated 2026-10-16 (afternoon): D30 to D36, the P1a plan, wait for G0. Earlier: your rulings (D28: the saga engine). The SPR items below are unchanged. Full reasoning is in [`12_open_questions.md`](12_open_questions.md) and [`06_adr/`](06_adr/README.md). Every ruling is recorded in [`decisions-log.md`](decisions-log.md).*

**How to answer:** reply in chat with the ID and your choice. Anything you don't answer keeps its default, and work proceeds on it.

## Still waiting on you

| ID | Question | Recommendation (default if unanswered) | Needed by |
|---|---|---|---|
| D29 | **Which image provider, and what monthly budget?** The graphic chronicle renders storyboard panels in batch after a saga is compiled (`05.11` §6; `12` Q24). | **A serverless API, in batch, no GPU VM.** Choose among fal.ai, Replicate and Together AI (FLUX.1 [dev] or SDXL) by a two-week trial on the same 50 panels at the start of P3. About $0.005–0.015 per panel, ≈ $1 per season (estimate, verify at P3). Default ceiling: $10 a month. Confidence: medium | Start of P3 (nothing earlier depends on it) |
| D30 | **Approve the P1a plan and the walking skeleton?** `CODEX/05_PROJECT/PRJ-002` plans P1a as 22 sprints (SPR-011 to SPR-032). `SPR-011` is the walking skeleton in full: manifest → founders → monthly deaths → events → the Parquet chronicle and World Stream v0 → `rh-cli tail` → a trivial sifter pattern → journal replay, on `solo`, `household` and `hamlet`. It is about 9 agent-days, so it runs as three sprints (SPR-011, -012 and -013) under one plan. It is C3: four new crates, the first real golden worlds, and `parquet` as a plane-A dependency. | **Approve**, as three sprints. 75% | G0 |
| D31 | **Rung 0 in P1a: ageing and death, without food or minute-level daily life?** The `docs/11` ladder puts "a household's food" and "every minute authoritative" at rung 0, but food is P1b and `minutes` is Later. `solo.yaml` names systems P1a does not build; the run header lists them as not built and skips them (SPR-011 rulings 2 and 3). | **Yes**: record it in `docs/11` and PRJ-001, rather than let rung 0 look incomplete. 75% | G0 |
| D32 | **Withdraw the "P1.0 runs at 100k" claim?** ADR-0028, `docs/12` Q12 and V-SCALE-01 predate ADR-0035, which made 100k and 1M optional. P1.0 is tested at N = 1, 5 and 60; 10k comes with P1.14 (SPR-011 ruling 19). | **Yes**: an ADR-0028 consequence and two document edits. 80% | G0 |
| D33 | **Move `S5.daily_life` F1 (P1.18) and M1 F1 (P1.8) to P1b?** S5 F1 reads M4 and M6 columns, which have no owner in P1a, so it would run on neutral values only. ADR-0031 had placed S5 in P1b. M1 F1's first consumer is the fugitives of P1b (PRJ-002 rulings M-4 and M-5). | **Yes for S5** (70%); **yes for M1 F1** (60%) | G0 |
| D34 | **Make G1a's tests runnable in P1a?** As written, some G1a tests need things P1a cannot have: `val_khor` (V-INV-12), occupation (V-DATA-04), M5 and M8 deaths (V-INV-03), and a 100k world with a hydration bubble (V-PERF-09). `docs/11` and `docs/09` also list two different G1a sets. Proposal (PRJ-002 ruling M-7): P1a variants of those four tests, and one G1a list. Also V-SAGA-16 (independent review, REV-P1a MAJ-8): its stress-accounting identity gates G1a; its break band is reported at G1a and gates G1b, because the band is declared on the P1b kingdom ensemble and P1a has only two stress sources. The numbers are fixed before the phase, as `docs/11` requires. | **Yes**. 75% | G0 |
| D35 | **P1a inherits titles and claims only, not property?** P1.7 says "titles and property", but property belongs to M4 (P1b) (PRJ-002 ruling M-8). | **Yes**: property inheritance comes with M4 F0 in P1b. 70% | G0 |
| D36 | **Approve the `T1.minds` and `M7.dynasty` specs (C2)?** Draft PR #32: minds v0 (stress and breaks under Laws 2 and 3, grief damping, relic thoughts) and succession (immediate succession, finite seats, vocations, no count controller). Review is done: four rounds of independent review (`CODEX/40_VERIFICATION/REV-P1a-specs-and-plan.md` 0.3.0) took it from 1 blocker and 9 majors to **0 blockers and 0 majors**, and the reviewer has no objection. The main fixes: deaths at day starts so succession and grief react in the death's instant; kinship walked over the life-course log, so heirs and grief reach through the dead; IAUS considerations and curves defined in the Registry. The briefs are in the PR. The stacked PR #37 (the skeleton specs SPR-011 needs: M2's rung-0 freeze, Registry verbs, channel A) merges right after it | **Approve**, merging #32 then #37. 80% | G0 |

## Decided on 2026-10-16

| ID | Ruling |
|---|---|
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
