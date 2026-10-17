# Your decisions, in one place

*Updated 2026-10-17: All open gating decisions ratified by Director (D37 effect language, D48 wave-4 plans SPR-014/SPR-020/SPR-026, D38 tellability ceiling, D40 & D41 births, D42 & D43 World Stream v1 wire, D44 causal edges, D46 popsynth, D47 control.sock, D49 Plane-C names, D50 prologue keyframe attribution). G0 passed (`VER-G0`). Full reasoning is in [`12_open_questions.md`](12_open_questions.md) and [`06_adr/`](06_adr/README.md). Every ruling is recorded in [`decisions-log.md`](decisions-log.md).*

**How to answer:** reply in chat with the ID and your choice. Anything you don't answer keeps its default, and work proceeds on it.

## Still waiting on you

| ID | Question | Recommendation (default if unanswered) | Needed by |
|---|---|---|---|
| D29 | **Which image provider, and what monthly budget?** The graphic chronicle renders storyboard panels in batch after a saga is compiled (`05.11` §6; `12` Q24). | **A serverless API, in batch, no GPU VM.** Choose among fal.ai, Replicate and Together AI (FLUX.1 [dev] or SDXL) by a two-week trial on the same 50 panels at the start of P3. About $0.005–0.015 per panel, ≈ $1 per season (estimate, verify at P3). Default ceiling: $10 a month. Confidence: medium | Start of P3 (nothing earlier depends on it) |

## Decided on 2026-10-17

| ID | Ruling |
|---|---|
| D37 | **Approved: Registry effect language v0 (PR #41).** Approved with D14 (title blood counts last 16 holders on tenure log), D4 (sex rules of succession laws), and D12 (lineage depth 3, max 4,096 candidates) as drafted. Unblocks M7 succession sprint and PR #41 merge. |
| D48 | **Approved: Wave-4 sprint plans SPR-014, SPR-020, and SPR-026 (PR #68).** Wave 4 of P1a unblocked: SPR-014 (lifecycle queues & mortality drain), SPR-020 (event envelope & roles, C3 baseline move), SPR-026 (World Stream v1 wire format & `rh-protocol`). Order: SPR-014, then SPR-020; SPR-026 in parallel/after. Start upon SPR-013 merge. |
| D38 | **Approved: Amend DATA-5 with a tellability ceiling.** Commoner life events admitted as chronicle rows while living population at start of year $\le$ 10,000 (`tellable_up_to` in P1a). World-ending verbs (tellability 255) always admitted. Recorded against DATA-5 in `docs/03`. |
| D40 | **Approved: Default fertility at tfr 4.9 with early near-universal marriage pattern for P1a.** Revisit on dynastic fixture world in SPR-030. |
| D41 | **Approved: Births checks V-SF-24 and V-SF-25 reported at G1a, gated from G1b.** Settlement rule (INV-M2-13) remains an invariant gating every run. |
| D42 | **Approved: World Stream live-edge window Option (b).** Live-edge window bounded to current sim-year and previous one, plus lateness of any seal slower than a sim-year. PERF-5 and V-PERF-09 amended. |
| D43 | **Approved: Live edge carries canonical text lines (amend ADR-0031 §1 item 4).** stdlib reference, verifiable line-by-line against sealed digest, stdlib viewers without columnar decode overhead on local socket. Arrow remains format at rest. |
| D44 | **Approved: DATA-2 causal edges amended to nine kinds.** Code 7 (`oracle`) retained for Oracle consult causality; V-DATA-09 amended to "only the nine kinds". |
| D46 | **Approved: Population synthesis v0 deferral.** Defer IPF from manifest marginals to `population` marginal alone for P1a; class/culture shares arrive in P1b. |
| D47 | **Approved: Roadmap text fix.** Move "tip and scheduled inputs" from P1.17 to P2 in `docs/11` (on `control.sock`). |
| D49 | **Approved: Plane-C deterministic name generator at P2.8.** Generated as $H(\text{world\_seed}, \text{person\_id}, \text{"name"})$ over culture name base; Plane A stores zero strings. Kinship fallback for G1a. |
| D50 | **Approved: V-NAR-05 wording amendment for annals prologue.** Pre-epoch prologue sentences cite keyframe rows (`kf:life/<pid>`), not event IDs. |

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
