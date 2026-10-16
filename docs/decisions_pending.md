# Your decisions, in one place

*Updated 2026-10-16, after your rulings (D28: the saga engine). The SPR items below are unchanged. Full reasoning is in [`12_open_questions.md`](12_open_questions.md) and [`06_adr/`](06_adr/README.md). Every ruling is recorded in [`decisions-log.md`](decisions-log.md).*

**How to answer:** reply in chat with the ID and your choice. Anything you don't answer keeps its default, and work proceeds on it.

## Still waiting on you

| ID | Question | Recommendation (default if unanswered) | Needed by |
|---|---|---|---|
| D29 | **Which image provider, and what monthly budget?** The graphic chronicle renders storyboard panels in batch after a saga is compiled (`05.11` §6; `12` Q24). | **A serverless API, in batch, no GPU VM.** Choose among fal.ai, Replicate and Together AI (FLUX.1 [dev] or SDXL) by a two-week trial on the same 50 panels at the start of P3. About $0.005–0.015 per panel, ≈ $1 per season (estimate, verify at P3). Default ceiling: $10 a month. Confidence: medium | Start of P3 (nothing earlier depends on it) |

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
