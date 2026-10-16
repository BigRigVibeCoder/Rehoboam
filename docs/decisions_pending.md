# Your decisions, in one place

*Updated 2026-10-16. This is the single list of everything waiting on you. Full reasoning is in [`12_open_questions.md`](12_open_questions.md) (Q-numbers) and [`06_adr/`](06_adr/README.md) (ADRs). Rulings are recorded in [`decisions-log.md`](decisions-log.md).*

**How to answer:** reply in chat with "accept all recommendations", or with the IDs you want to change, for example "D1 yes, D8 keep 80%, D5 Windows". Anything you don't answer keeps its default, and work proceeds on it.

## A. Shapes the engine (answer before P0 starts)

| ID | Decision | Recommendation (default) | If you choose otherwise | Where |
|---|---|---|---|---|
| **D1** | **World Stream** (ADR-0030 v2). The engine streams its world; viewing is a buffer event at your own cursor; actions land at the tip; pace is your setting; no GUI in phase 1 | **Accept.** It is your design, checked against the determinism, plane and no-back-dating rules | — | [ADR-0030](06_adr/ADR-0030-headless-engine-text-harness-first.md), [05.10](05_architecture/05.10_world_stream.md) |
| **D2** | **Approve the 18 critical-path ADRs** (★): 0001–0003, 0005–0010, 0020, 0022, 0024–0030 | **Approve as proposed** | Name any you want reopened; that ADR then needs a revision before P0 | [ADR index](06_adr/README.md); Q★1 |
| **D3** | **Review hours per week** you can give agent output | **≤ 6 h/week** outside rating windows | More hours allow more parallel workstreams and shorten every phase; fewer lengthen them | Q★16 |
| **D4** | **First reference world** | **Feudal, no dragons in P1** (`val_khor`); dragons and the hive arrive in P3a | Modern first means less calibration data and a weaker thesis test | Q★2 |
| **D5** | **Your operating system** for development and play | *No good default. Please tell me.* Assumed: Linux or macOS for development, CI on all three | Decides packaging and later wallpaper hosting; phase 1 is terminal-only, so it matters less now | Q★3 |
| **D6** | **Your compute**: desktop spec, GPU, cloud budget ceiling | *Please tell me.* Assumed: a strong 8-core desktop and $50–200/month of cloud | Sizes CI runners, 1M-scale targets and local model choices | Q★4 |
| **D7** | **ODD-R** as the required spec format for every simulation system | **Adopt** | Specs stay free-form; agents and reviewers lose a checklist | [template](templates/system_spec_odd_r.md); decisions log |
| **D24** | **Fidelity by zoom** (ADR-0031). The World Language; **virtual telemetry**, meaning anyone's whereabouts on any day of history, computed and never stored; authoritative daily life where you declare it | **Accept** | Either store movement (≈ 11 GB per sim-year at 1M) or have no street-level history at all | [ADR-0031](06_adr/ADR-0031-fidelity-by-zoom-virtual-telemetry.md) |
| **D25** | **Default activity resolution**: micro worlds `blocks` (daily life counts); meso and macro `life_course` plus focus regions you name | **Accept** | `blocks` everywhere at 100k costs ≈ 1.5 h per millennium, against the 2-minute budget; `minutes` is for micro worlds only | ADR-0031 §3 |

## B. The thesis gate, G2 (fix before P2 starts)

| ID | Decision | Recommendation (default) | If you choose otherwise | Where |
|---|---|---|---|---|
| **D8** | **MOE-2 pass rule**: the chronicle A/B test of the coupled engine against a coupling-ablated one | **40 blind pairs; pass when the lower 80% confidence bound is above 50%; 80% stays as the target you see reported** | Keeping "≥ 80% of 20 pairs" means a good engine (true preference 70%) fails 3 times in 4 | Q21; `09` V-HUM-02 |
| **D9** | **Commit to the pre-registered G2 protocol**: frozen build, sealed held-out world, blind rating, fixed decision rule | **Yes** | Without it, a gate you both build and rate drifts toward "pass" | Q18; `09` §3.4.1 |
| **D10** | **Rate template text only** at G2 (no LLM prose in rated items) | **Yes** (already the default) | G2 would measure the narrator's prose, not the history | Q20 |
| **D11** | **G2 measures your taste**, stated explicitly | **Yes**; an outside reader panel is optional, after P4 | An outside panel now costs 24+ readers' time | Q22 |
| **D12** | **Rating time**: about 35–45 hours over a 3–5 week window after P2, and again after P3b | **Yes**, at about 8–10 h/week in the window | The thesis test date moves with this window | Q6 |

## C. Can wait (defaults apply until you say otherwise)

| ID | Decision | Default | Where |
|---|---|---|---|
| D13 | Calendar | 360-day year, configurable per world | Q5 |
| D14 | LLM "deliberation" for named actors | Off until an A/B test shows it helps. At MAX speed it never stalls the engine; a late answer uses its journaled fallback | Q7 |
| D15 | May frontier LLM APIs see world data for synthesis? | Yes, never anything identifying you; local fallback always available | Q8 |
| D16 | How atrocities are narrated | "Historian" register: consequences, not gratuitous detail | Q9 |
| D17 | Oracle death | Rebirth with heavy loss; permadeath as a hard mode | Q10 |
| D18 | Foresight cost in Cultivator mode | Free (it is the analysis mode); it costs Foresight in Oracle mode | Q11 |
| D19 | Ever distribute it? | Keep the option open: strict licences in core crates | Q13, Q17 |
| D20 | Multiplayer | Out of scope; worlds are shared as manifest + seed + journal | Q14 |
| D21 | Science-fiction frontiers, and when per-era art starts | Decide after the first era pack; art packs after P5 | Q★19 |
| D22 | Suspense pauses in Oracle mode | Off until they can use the Oracle's beliefs, not hidden truth | Q23 |
| D23 | The 12 non-critical-path ADRs (0004, 0011–0019, 0021, 0023) | Approve as proposed when their phase starts | [ADR index](06_adr/README.md) |

**Already decided** (for reference):
- scale 1 → 1M (ADR-0028);
- era order E2 → E3 → E4, provisional;
- lo-fi first (ADR-0029);
- success is a creative engine plus a world to inhabit;
- the workforce is you plus AI agents.

See [`decisions-log.md`](decisions-log.md).
