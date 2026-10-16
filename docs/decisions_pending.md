# Your decisions, in one place

*Updated 2026-10-16, after your rulings. Full reasoning is in [`12_open_questions.md`](12_open_questions.md) and [`06_adr/`](06_adr/README.md). Every ruling is recorded in [`decisions-log.md`](decisions-log.md).*

**How to answer:** reply in chat with the ID and your choice. Anything you don't answer keeps its default, and work proceeds on it.

## Still waiting on you (4 for SPR-001, 1 for later)

### SPR-001 approval (C2)
SPR-001 (`CODEX/05_PROJECT/SPR-001_RepoBootstrap.md`) creates the repository's contracts: layout, lints, CI. As a C2 change it needs your yes before the developer agent starts. **Default:** none; work waits for you.

### Q-INFRA-1 — Which GitHub plan is the account on?
The repository is private. On GitHub's **Free** plan, a private repository cannot have protected branches or required checks, so CI can report a failure but cannot block the merge. **Pro** (a few dollars a month) allows required checks. GitHub's **merge queue** on a private repository needs Enterprise Cloud, which this project does not need: `docs/13` §4.4 now has a fallback (branches must be up to date, merges one at a time). Private repositories also get a monthly allowance of Actions minutes (about 2,000 on Free, 3,000 on Pro; check your billing page), which the PR tier fits but nightly 1,000-year runs may not.
**Default:** proceed on whatever plan you have. On Free, the Architect enforces "no green report, no merge" by hand. Upgrading to Pro is recommended before SPR-002.

### Q-INFRA-2 — Where do the coding agents run?
Either Claude Code on your laptop, or cloud sessions like this one. Cloud sessions leave the laptop free and cool, and the laptop then only runs your checks and the nightly cross-machine comparison. Agents on the laptop compete with you for 4 cores and heat it up.
**Default:** cloud sessions for agents; the laptop for your checks, watching the stream, and the nightly cross-machine run.

### Q-INFRA-3 — How much free disk space does the laptop have?
Rust build folders grow to 10–30 GB, and a 100k-person world is 6–18 GB per 1,000 years. **Default:** keep at least 100 GB free; tell me if you have less.

### D6 (the rest, needed before P3) — Cloud budget
Answered 2026-10-16: the machine is a ThinkPad P51 laptop (4 cores, 32 GB, Quadro M2200 4 GB). Its graphics card is too small and too old for local AI language models, so LLM work in P3 and later goes to cloud services or a later machine. Still open: a monthly cloud budget ceiling. **Default:** none needed until P3.

## Decided on 2026-10-16

| ID | Ruling |
|---|---|
| D1 | World Stream (ADR-0030): **accepted** |
| D2 | The 18 critical-path ADRs: **accepted** |
| D4 | First world: **a minimal test world**, `solo` (one person, tiny map); then add complexity one rung at a time (`11` complexity ladder) |
| D5 | **Linux** |
| D6 | **A ThinkPad P51 laptop** (4 cores, 32 GB, Ubuntu 24.04), local first, cloud after local testing. Cloud budget still open (see above). Setup: `CODEX/30_RUNBOOKS/RUN-001` |
| D3 / D26 | **Your Agentic Architect operating model, adapted (ADR-0032): accepted.** Set up in `CODEX/` (SPR-000-ARCH). Your time: about 2–4 hours a week outside rating windows |
| D7 | **ODD-R accepted, taken further:** system specs as machine-checked data (ADR-0033), with a generated one-page brief for you |
| D8–D12 | G2 thesis-gate defaults **accepted for now**; revisit before P2 |
| D24, D25 | Fidelity by zoom (ADR-0031) and its default resolutions: **accepted**. Minute-level daily life is allowed at any scale as a batch run (≈ 1.5 h per sim-year at 1M) |

## Defaults in force (no action needed; change any time)

| ID | Decision | Default | Where |
|---|---|---|---|
| D13 | Calendar | 360-day year, configurable per world | Q5 |
| D14 | AI "deliberation" for named characters | Off until an A/B test shows it helps; never stalls the engine | Q7 |
| D15 | May cloud AI services see world data, to write prose? | Yes, never anything identifying you; a local fallback always exists | Q8 |
| D16 | How atrocities are narrated | "Historian" register: consequences, not gratuitous detail | Q9 |
| D17 | Oracle death | Rebirth with heavy loss; permadeath as a hard mode | Q10 |
| D18 | Foresight cost in Cultivator mode | Free; it costs Foresight in Oracle mode | Q11 |
| D19 | Ever distribute it? | Keep the option open: strict licences in core code | Q13, Q17 |
| D20 | Multiplayer | Out of scope; worlds are shared as manifest + seed + journal | Q14 |
| D21 | Science-fiction frontiers; when per-era art starts | Decide after the first era pack; art packs after P5 | Q★19 |
| D22 | Suspense pauses in Oracle mode | Off until they can use the Oracle's beliefs | Q23 |
| D23 | The 12 non-critical-path ADRs | Approve as proposed when their phase starts | [ADR index](06_adr/README.md) |
