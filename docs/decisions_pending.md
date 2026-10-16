# Your decisions, in one place

*Updated 2026-10-16, after your rulings. Full reasoning is in [`12_open_questions.md`](12_open_questions.md) and [`06_adr/`](06_adr/README.md). Every ruling is recorded in [`decisions-log.md`](decisions-log.md).*

**How to answer:** reply in chat with the ID and your choice. Anything you don't answer keeps its default, and work proceeds on it.

## Still waiting on you (3)

### D3 — How many hours a week can you spend checking the AI's work?

**In plain words.** The engine is built by AI coding agents, and you are the only human checking their work. A typical week of checking looks like this:
- **Approving plans** before a piece is built. Each is a one-page description such as "the population system: what it controls, what it reads, how it is tested". About 15 minutes each.
- **Looking at results**: run the test world, read the stream, say "that looks right" or "that's wrong".
- **Answering questions** when the agents hit a real choice.

Everything else (tests passing, formatting, routine fixes) is checked automatically and merges without you.

**Why it matters.** Your hours set how many agents can work at once, and so how fast the project moves:

| Your hours per week | Agents working in parallel | Effect on the schedule |
|---|---|---|
| About 3 | 1–2 | Slower: phases take roughly 1.5× longer |
| **About 6 (default)** | 2–3 | As planned in `11_roadmap.md` |
| About 10–12 | 4–5 | Faster: phases roughly a third shorter |

**What I need:** a number, even a rough one. If you say nothing, I assume about 6.

### D7 — Use a standard fill-in-the-blanks form for describing each simulation system?

**In plain words.** Before an agent builds a piece of the simulation (population, economy, war), it writes a description of that piece. Scientists who build simulations like ours use a standard form called **ODD** (Overview, Design concepts, Details), so that anyone can understand or rebuild a model. "ODD-R" is that form adapted to Rehoboam ([template](templates/system_spec_odd_r.md)). It makes every description answer the same questions:
- what the system controls, and what it is allowed to read;
- when it runs, and in what order;
- what randomness it uses;
- every setting, with its allowed range and the source of that range;
- how it is tested, and against which historical facts.

**Why I recommend it.** Agents produce consistent descriptions; you review the same layout every time; and the usual failures (vague timing, undocumented settings) get caught before code is written. A 2010 review of 54 published models found exactly those failures in most of them.

**The cost.** Each description is 2–3 pages instead of 1.

**What I need:** yes or no. **Default: yes.**

### D6 (the rest) — Memory, graphics card and cloud budget

You answered "8-core desktop, local first, cloud later". I read your "* core" as 8-core; correct me if not. Phase 1 needs no graphics card (no GUI, no AI language models). Before P3 I need:
- the desktop's RAM;
- its graphics card;
- a monthly cloud budget ceiling for when you move to the cloud.

**Default until then:** 32–64 GB RAM, no GPU assumptions.

## Decided on 2026-10-16

| ID | Ruling |
|---|---|
| D1 | World Stream (ADR-0030): **accepted** |
| D2 | The 18 critical-path ADRs: **accepted** |
| D4 | First world: **a minimal test world**, `solo` (one person, tiny map); then add complexity one rung at a time (`11` complexity ladder) |
| D5 | **Linux** |
| D6 | **8-core desktop**, local first, cloud after local testing (RAM, GPU and budget still open; see above) |
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
