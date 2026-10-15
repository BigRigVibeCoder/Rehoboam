# R08 — Long-Form Narrative Generation from Structured Simulation Data (Narrative Compiler)

*Research dossier, 2026-10-15. Scope: Modality 5 (sift → novel / Fountain screenplay / audio-drama script): methods, grounding, style, evaluation, cost.*

**Verification legend.** In the Sources section, (V) means read in this session from a primary repo or the vendor's docs. (S) means seen only as a search-result summary. (K) means prior knowledge that was not re-fetched this session (the research cutoff is mid-2026). The session's web-search quota ran out partway through, and most domains other than GitHub could not be reached, including arXiv, eqbench.com, copyright.gov and fountain.io. Legal items are therefore mostly (K) and need a re-check before any product decision.

---

## TL;DR

1. **The thesis is half right.** Grounding in a simulation fixes the failure LLM fiction is *measurably* bad at: plot holes and continuity. LLM-generated stories show roughly 100% more detected plot holes than the human originals [26]. It does **not** fix the failures readers and experts actually complain about most: clichés, sameness, purple prose, telling instead of showing, no subtext [23][24][5]. Expect "consistent and consequential," not "authentic literature."
2. **"A frontier model translates the graph into a novel" is wrong as phrased.** No current model writes a good 100k-word novel in one call. Claude models cap output at 128K tokens and Gemini 3.1 Pro at 64K [1][3]. Every credible system since Re3 (2022) plans hierarchically, drafts scene by scene, then critiques and revises [9][10][12][16][17]. Build a pipeline, not a prompt.
3. **Drop named-author styling, especially Peter F. Hamilton.** None of Herbert, Tolkien or Hamilton is in the public domain. Tolkien's estate litigates. Using a living author's name in product UI invites false-endorsement and passing-off claims. In-context "write like X" imitation is also judged poor by expert readers [33](K). Use **style cards** plus public-domain exemplars instead.
4. **Model choice (Oct 2026):** on the most recent EQ-Bench Creative Writing v3 canonical data I could verify (snapshot 2026-06-19), Claude Fable 5 (Elo 2192) and Claude Opus 4.7 (2179) lead, followed by GPT-5.5 (2019). Gemini 3.1 Pro sits mid-pack (1482), and the best open-weight models are GLM-5.2 (1793) and Kimi K2.6 (1767) [4]. Newer releases (Opus 5.5, Fable 5.1, GPT-6 Astra, Kimi K3) are **not yet in verified data**.
5. **Cost is not the binding constraint.** A gated 100k-word novel costs roughly **$17–$190 at list price with caching**, depending on model and about half that through the Batch API. Recommended config: about $56 cached, $28 batch. A 120-page screenplay is about $15–$30 and a 10×30-min audio-drama script about $19–$38 (my model; see §10). TTS audio cost is separate and probably larger (inference).
6. **Rehoboam's real edge is parallelism.** The sim knows every scene's ground-truth entry state (who is alive, where, what they know). Scenes can be drafted in parallel and checked deterministically, which unlocks batch pricing and per-claim fact checks that free-form LLM novels can't do.
7. **Fact-check graph-first, LLM-second.** Long-context LLM continuity judgment is unreliable [25][27][28]. Verify structured claims against the graph, the rest with a MiniCheck-class NLI checker [29]; whole-book LLM reads only *propose* issues.
8. **LLM-as-judge is a filter, not a certifier.** TTCW's LLM judges didn't track experts [23], and EQ-Bench admits uncontrolled self-bias and slop bias [4]. Use a cross-family critic, deterministic slop/repetition metrics, and human panels.
9. **Screenplays and audio:** the LLM emits *structured JSON scenes*, and code renders Fountain and TTS manifests. Fountain tooling is mature [35]–[39]. Open TTS models have hard speaker-tag limits (Dia: `[S1]/[S2]`) [40], and Microsoft pulled VibeVoice-TTS code over misuse [41].
10. **Honest 2026 quality bar:** fluent, consistent, genre-competent first drafts with recurring tics. Below a good mid-list novelist and far below the named masters. Not every epoch is a novel: default to chronicle, saga or in-world documents, and escalate to a novel only past a story-shape gate.

---

## Draft claims checked

| Claim (draft) | Verdict | Evidence |
|---|---|---|
| Generative fiction fails because ungrounded LLMs hallucinate events without consequences | **Partly true / incomplete** | Plot-hole rate rises 100%+ in LLM-generated stories [26]. But expert critiques center on prose (LAMP's edit taxonomy: clichés, unnecessary exposition, purple prose, lack of specificity [24]); LLM stories passed 3–10× fewer TTCW tests than New Yorker stories [23] (K: ratio from the paper abstract) |
| Grounding in simulation makes stories authentic | **Unproven** | No study tests sim-grounded long fiction. Fable's SHOW-1 is the closest precedent and was a demo, not an evaluated result [44](K). Grounding adds a new risk: "chronicle prose" that summarizes logs |
| Sifter yields 30–50 key events that are enough for a novel | **Plausible, insufficient alone** | 100k words ≈ 80–120 scenes, so 30–50 turning points is the right *spine* density. Each also needs aftermath/"sequel" scenes, secondary events, and per-actor state (beliefs, goals) for interiority |
| A frontier reasoning model translates the graph into full-length novels | **Misleading** | Output caps of 128K (Claude/GPT-6) and 64K (Gemini 3.1 Pro) [1][3]. Hierarchical plan→draft→revise beats direct generation: Re3 showed +14 pts coherence, DOC +22.5 pts over Re3 [9][10]. Long-context comprehension still degrades [27][28] |
| Style "after Herbert, Tolkien, Hamilton" | **Replace** | All three are still under copyright (§6). Hamilton is living and Tolkien's estate is litigious [50](K). In-context imitation of named authors loses to experts on fidelity [33](K) |
| Fountain screenplays with stage directions and dialogue | **Supported** | Plain-text spec with open parsers and renderers [35]–[39]. Dramatron shows the hierarchy works, but professionals called the output "formulaic" and wouldn't write a full play with it [11] |
| Serialized audio drama via multi-voice TTS | **Feasible, separate risk surface** | Needs speaker-tagged intermediate format. Engine limits (e.g. Dia 2 tags [40]). Voice-likeness law (K). Cost is likely dominated by TTS |
| 1M-token context lets the model "see the whole book" | **Technically true, practically weak** | NoLiMa: 10 of 12 models fell below 50% of their short-context baseline at 32K [28]. FABLES: no LLM rater correlated strongly with humans on unfaithful claims [25] |
| LLM judges can certify quality | **False** | TTCW: LLM assessments didn't positively correlate with experts [23]. LitBench's best judges reach about 73–78% agreement on pairwise preference [31](K numbers) |
| Frontier models are now near human authors | **Only for short form, only with non-expert readers** | EQ-Bench truncates to 4,000 characters [4]. No verified evidence of novel-length parity |

---

## Landscape / state of the art

### 1. Long-form fiction generation methods

| System | Date / venue | License | Maturity | Core idea | Fit for Rehoboam |
|---|---|---|---|---|---|
| Re3 [9] | 2022, EMNLP | research code | historical | Plan (setting, characters, outline) → recursive reprompting → rerank → edit; +14 pts coherence, +20 pts premise relevance (human eval) | Template for plan-injection |
| DOC [10] | 2022/ACL 2023 | research | historical | Detailed hierarchical outline plus a controller that keeps passages on-outline; +22.5 pts coherence over Re3; dropped the edit step | **High**: the discourse plan *is* the detailed outline |
| Dramatron [11] | 2022/CHI 2023 | Apache-2.0 / CC-BY | archived colab | Logline → characters → plot points → locations → dialogue. 15 pros: useful for world building and exploration, "formulaic," not for full plays | Screenplay hierarchy |
| Agents' Room [12] | ICLR 2025 | dataset released (encrypted) | research | Orchestrator plus specialized planning agents (conflict, character, setting, plot) and writer agents over a shared scratchpad; expert preference over baselines. Human stories still preferred (S) | Multi-agent role split |
| Weaver [13]; LongWriter/AgentWrite [14] | 2024 | closed / open | dated | Writing-specialized models (1.8B–34B); plan-then-write data giving 10k+ word single outputs | Low |
| LongWriter-Zero [15] | 2025 / ICLR 2026 | open (32B, Qwen2.5 base) | research | Pure-RL ultra-long writing with length, quality and format rewards; SOTA on WritingBench/Arena-Write (S) | Local fallback candidate; dated base |
| StoryWriter [16] | 2025 | open code/data | research | Event-based outline agent → chapter planning agent → writer that compresses history per event; ~5k stories × 8k words | **High**: event-graph-native |
| WriteHERE / "Beyond Outlining" [17] | EMNLP 2025 (oral) | MIT | research | Recursive, heterogeneous decomposition (retrieve/reason/compose) interleaved with execution | Adaptive re-planning mid-book |
| NstAgent [18]; outline-stage comparison [19] | Aug–Sep 2026 preprints | ? | (S only) | Structured narrative-state tracking at 10K–100K words; cross-framework outline study | Same idea as our sim state; unverified |

**Convergent lesson (inference).** Working systems separate *what happens* (plan/state, held in external memory) from *how it's told* (drafting), and revise locally. Rehoboam gets the first layer free, with better ground truth than any LLM outline, so the burden shifts to discourse planning and prose.

### 2. Frontier models, Oct 2026 (limits and prices)

| Model | Ctx / max out | $ in / out per MTok | Cache read | Batch | EQ-Bench CW v3 Elo (2026-06-19) | Notes |
|---|---|---|---|---|---|---|
| Claude Fable 5.1 | 1M / 128K | 10 / 50 | 0.25 | 5 / 25 | n/a (Fable 5 = **2192**, #1) | Thinking always on; no temperature/top_p (400) [1] |
| Claude Opus 5.5 | 1M / 128K | 4 / 20 | 0.20 | 2 / 10 | n/a (Opus 4.7 = 2179, 4.8 = 1988) | Effort default `medium`; thinking can't be disabled [1] |
| Claude Sonnet 5.5 | 1M / 128K | 2 / 10 | 0.20 | 1 / 5 | n/a (Sonnet 4.6 = 1946) | [1] |
| Claude Haiku 5.5 | 1M / 128K | 0.10 / 0.50 (≤100K prompt) | — | 50% | n/a | Checker role [1] |
| GPT-6 Astra (released 2026-09-03) | ~922K / 128K | 10 / 50; >272K input: 20 / 75 | 1.0 | 5 / 25 | n/a (GPT-5.5 = 2019) | Launch messaging is about coding and agents, not fiction [46](S) |
| GPT-6.1 Sol | ~922K / 128K | 2 / 10 | 0.10 | out 5 | n/a | [3] |
| Gemini 3.1 Pro (preview) | 1M / 64K | 2 / 12 (≤200K) | 0.20 | 1 / 6 | **1482** | Gemini 4 reported imminent (S) [47] |
| Kimi K3 (open weights, 2.8T / 104B active) | 1M / ~128K | 3 / 15 (hosted) | 0.30 | — | n/a (K2.6 = 1767) | Custom license; MaaS >$20M revenue needs a separate agreement [34] |
| GLM-5.2/5.3 | 1M / 128K | ~1.1–1.7 / 4–5 | — | — | GLM-5.2 = **1793** | Best open model in verified data [4][3] |
| DeepSeek V4 Pro | 1M / 384K | 1.32 / 3.96 | 0.044 | — | 1561 | [3] |
| Gemma 4 31B (local) | 256K / — | self-host | — | — | 1385 (≈ Claude 3.7 Sonnet) | Fits a single high-memory GPU at 4-bit (inference) |

Prices come from Anthropic's model table (cached 2026-10-06) [1] and the LiteLLM registry [3]. Non-Anthropic prices are secondary; check them before committing. Two caveats on EQ-Bench Elo: the rubric scores are saturated (most frontier models score 16–18/20, and o3 out-rubrics Fable 5 while losing on Elo), and the top-two confidence intervals overlap [4]. A secondary article claims EQ-Bench and arena.ai's creative-writing board rank models in nearly opposite order (S) [8].

**Effective long-context coherence.** No verified Oct-2026 fiction-specific numbers. Older evidence is consistent: humans score ~97% on NoCha book-claim pairs while models score far lower [27]; NoLiMa degrades sharply by 32K [28]; FABLES finds that LLM raters miss unfaithful event and character-state claims [25]. **Design assumption:** keep working calls under ~40–60K tokens of context, and use whole-book reads only as issue finders.

### 3. Failure modes and mitigations

| Failure | Evidence | Mitigation in pipeline |
|---|---|---|
| Plot holes / continuity | 50%+ (summaries) and 100%+ (generation) more plot holes vs. human originals [26] | Graph-grounded scene fact sheets; claim checks; append-only embellishment ledger |
| Slop / clichés | LAMP taxonomy [24]; EQ-Bench slop lists, which even include LLM-favoured fantasy names like "Aelara" and "Aldric" [5]; antislop backtracking samplers [32] | Names come from the sim; banned-phrase lists; slop-density gate. Closed APIs expose no logits, so regex-detect then rewrite |
| Sameness across stories | Homogenization of AI-assisted stories [63](K); plot-level "echoes" [62](K) | Randomized style cards and structure templates; three candidate plans; cross-book n-gram dedupe. **Current Claude models reject temperature** [1], so diversity must be engineered upstream |
| Lost threads | DOC/Re3 long-range errors [10] | Thread table in the discourse plan (open/close chapters), validated |
| Character drift | (K) persona-drift literature | Voice sheets per character from sim traits; per-scene voice check |
| Sagging middle / pacing | Pacing control via outline concreteness [61](K) | Tension-target curve from dramatic weights; midpoint reversal; summary vs. scene allocation |
| Over-telling / exposition | EQ-Bench longform negative criteria: "Tell-Don't-Show," "Purple Prose," "Unearned Transformations" [5] | Critic rubric reuses these criteria; interiority limited to sim-licensed states |

### 4. From fabula to discourse

The simulation produces a **fabula**: events in causal and temporal order. A novel is a **syuzhet/discourse**, a selection and ordering of that fabula with chosen viewpoint, speed and disclosure [56](K). Genette's categories map directly onto plan fields: *order* (analepsis/prolepsis), *duration* (scene, summary, ellipsis, pause), *frequency*, *mood/focalization* (POV, knowledge), and *voice*. This is the classic NLG pipeline: content determination (sifter) → document planning (discourse plan) → microplanning (beats) → realization (prose) [53](K). Data-to-text research already defines faithfulness metrics for it [54].

| Work | What it contributes | Use |
|---|---|---|
| Bae & Young (Prevoyant): flashback/foreshadowing for surprise; Cheong & Young (Suspenser): suspense by withholding [57](K) | Plan-based reordering for affect | Rules for when to use a flashback (a causal parent far back and unshown) and what to withhold |
| Cardona-Rivera et al., Indexter [58](K) | Event salience on 5 situation-model indices (time, space, protagonist, causality, intention) | Score scene transitions; minimize index breaks; choose POV |
| Ware & Siler, Sabre [59](K) | **Fabula**-level planner with intentions and deep theory of mind (not a discourse planner) | Model "who believes what," enabling dramatic irony and blocking epistemic leaks. Ask the sim team to log beliefs |
| Montfort, Curveship [60](K) | Same events, different narrating (order, focalization, speed) | Proof that discourse variation is separable and controllable |
| Story sifting: Ryan [65](K); Felt [42] | Pattern queries over event logs (e.g. "violation of hospitality") | Upstream (R-sifter); the patterns become chapter "micro-arcs" |
| Wiseman et al., RotoWire [54](K) | Relation-generation precision, content selection, content ordering | Direct template for faithfulness metrics |

**Explicit plan decisions:** 2–5 POV characters (backbone participation, vantage diversity, survival span); chapters clustered by time/place/POV with one value change each; scene vs. summary by dramatic weight; a reveal schedule; prolepsis, since the sim knows the future; and epigraphs from *in-world documents* derived from the chronicle. That last one is Herbert's device without Herbert's text.

### 5. Grounding: canon vs. embellishment, story bible, fact-checking

**Policy tiers (recommended):**

| Tier | Content | Rule |
|---|---|---|
| T0 Canon | Graph events (actor, verb, target, outcome), ordering, deaths and births, locations, kinship, possessions, recorded motives, per-actor knowledge | Immutable; any contradiction blocks the scene |
| T1 Derived | Ages, distances, travel times, seasons and weather if simulated | Computed and injected; contradiction blocks |
| T2 Licensed embellishment | Dialogue wording, gestures, sensory detail, interiority consistent with sim traits, needs and beliefs; unnamed extras; off-screen trivia that changes no sim variable | Allowed; anything later scenes depend on goes into an append-only **embellishment ledger** |
| T3 Forbidden | New named agents with agency, new deaths or injuries, new relationships, consequential objects, motives contradicting the record, knowledge a character doesn't have, events after the epoch | Blocked |

**The chronicle stays immutable.** The narrative is a *derived artifact* with provenance: chapter → event IDs, plus the model ID and prompt hash. Because outputs can't be seeded (no temperature control on current Claude [1]), regeneration produces a *new edition*. That is fine, but it should be modeled as such.

**Checking stack (cheapest first):**
1. Scene-level claim extraction into triples with a small model.
2. Deterministic check of structured claims against the graph and ledger.
3. NLI for free-text claims against the scene fact sheet. MiniCheck-class checkers do grounded fact-checking at a fraction of LLM cost; Bespoke-MiniCheck-7B runs locally [29].
4. A whole-book read that *proposes* continuity issues; each is verified against the graph before any fix is made.

**Faithfulness metrics:** canon contradictions per 1k words (target 0 for T0/T1); forbidden-invention rate; backbone coverage (RotoWire-style content-selection recall); causal fidelity (each effect's parent shown or referenced first); story-time consistency under flashbacks; epistemic-leak count; FABLES-style omission typology [25].

### 6. Style control: legal and policy landscape, and the robust alternative

**Legal landscape (all (K); re-verify):**
- The US Copyright Office AI report Part 2 (Jan 2025) holds that prompts alone don't make outputs copyrightable. Part 3 (pre-publication, May 2025) floats "market dilution," including stylistic imitation, as a fair-use harm [48].
- *Bartz v. Anthropic* (June 2025): training on lawfully acquired books was ruled fair use, but pirated library copies were not. The case settled for about $1.5B. *Kadrey v. Meta* (June 2025) went for Meta on the evidence, with the court flagging dilution theories. In the consolidated *In re OpenAI* (SDNY), output-similarity claims survived dismissal [49].
- Style per se is not copyrightable. Coined vocabulary, characters and settings are a different matter: "hobbit," "Kwisatz Haderach," "melange" and Hamilton's named universe are infringement and trademark bait.
- The Tolkien Estate won an injunction against an unauthorized LOTR "sequel" (*Polychron*, 2023) [50].
- Copyright terms: Tolkien (d. 1973) is protected in the UK/EU until 2044, and *The Hobbit* until 2033 in the US. Herbert (d. 1986): *Dune* is protected to about 2061 in the US and until 2057 under life+70. Hamilton is living.
- The Authors Guild opposes unlicensed imitation and runs a "Human Authored" certification [51].
- EU AI Act Art. 50 transparency duties (machine-readable marking of synthetic content) apply from Aug 2026, subject to possible "digital omnibus" adjustments [52].
- Provider policies: image models refuse living-artist styles. For text I found no verified blanket ban, but don't build on provider tolerance.

**Quality evidence (K, re-verify numbers).** Chakrabarty, Ginsburg and Dhillon (2025) [33] found that expert readers strongly preferred MFA writers over *prompted* frontier models for author-style fidelity and quality, and AI detectors flagged nearly all of the prompted outputs. Only *fine-tuning on an author's books* flipped the preference. Fine-tuning on books is exactly the legally hazardous path. So "in the style of Tolkien" by prompt is weak, and the version that works is the one you can't ship.

**Recommended approach: style cards.** A style card is a versioned JSON object with fields such as:
- prose density
- sentence-length mean and variance targets
- dialogue ratio
- POV and tense
- register (archaic, plain, technical)
- figurative-language budget
- chapter-opening device (epigraph / in-world document / cold open)
- interiority depth
- exposition policy
- lexicon seeds from the sim's culture generator
- a banned-term list (slop plus other authors' coined terms)
- 2–4 short exemplars from **US public-domain** texts (published ≤1930 as of 2026)

Example cards:

| Card | Inspired by public-domain exemplars | Fits |
|---|---|---|
| *Mythic Chronicle* | E.R. Eddison's *The Worm Ouroboros* (1922), William Morris, Malory | High-epic, archaic register |
| *Deep Time* | Olaf Stapledon's *Last and First Men* (1930) | Civilizational, multi-generation; ideal for 1,000-year epochs |
| *Saga* | 19th-c. translations of Icelandic sagas | Terse, external, event-driven; most faithful to sim data |
| *Historian* | Gibbon; Crawley's Thucydides | Chronicle and summary modes, epigraphs |
| *Ensemble Space-Opera* | E.E. Smith (1928), Wells | Multi-POV, technical register |

Marketing can describe effects ("ecological epic with in-world epigraphs") without naming living authors. **Evaluating adherence:** stylometric targets (sentence-length distribution, function-word profile/Burrows' Delta against the exemplar set, dialogue ratio, readability band), plus pairwise LLM judging against the card, plus slop density [32].

### 7. Screenplay and audio-drama formats

| Tool / format | License | Role |
|---|---|---|
| Fountain spec, reference parser [35] | MIT (parser) | Plain-text screenplay markup: scene headings (`INT./EXT.` or forced `.`), action, CHARACTER, parentheticals, transitions, `[[notes]]`, `/* boneyard */`, `#` sections, `=` synopses, title page, dual dialogue `^`. No production features (MOREs/CONTINUEDs) |
| afterwriting [36] | MIT | Fountain→PDF, stats (page count, dialogue/action time, locations); node CLI suits a CI gate |
| screenplain [37] | open (K: MIT) | Python library/CLI: Fountain → FDX/HTML/PDF |
| Better Fountain [38] | open | VS Code editing and preview for human revisers |
| Beat [39] | GPL | macOS/iOS editor; FDX import/export |
| Dia [40] | Apache-2.0 | Dialogue TTS with `[S1]/[S2]` tags and nonverbals like `(laughs)`, `(sighs)`. Two speaker tags means multi-cast drama needs per-line synthesis and mixing |
| VibeVoice [41] | MIT (K) | 90-min, 4-speaker TTS (ICLR 2026 oral); **TTS code removed Sept 2025** over misuse |

**Recommendations:**
- The LLM emits **structured scene JSON**: event refs, location, characters, beats, and lines with `speaker_id`, `delivery` enum, and `sfx`/`music`/`ambience` cues.
- Deterministic renderers produce (a) Fountain, with event IDs in `[[notes]]` for provenance, (b) a human-readable radio script, and (c) per-engine TTS manifests.
- Gate on a parse round-trip plus an afterwriting page count (about 1 page ≈ 1 minute; 120 pages ≈ 45–60 scenes).
- Dramatron's lesson: hierarchy works, and professionals used it as a co-writer, not an autonomous author [11].
- Audio conventions: name call-outs for orientation, ≤4–5 voices per scene, no visual-only information, and a narrator role.

### 8. Evaluation

| Benchmark | What / judge | Current leader (verified?) | Caveat |
|---|---|---|---|
| EQ-Bench Creative Writing v3 [4] | 32 prompts ×3; rubric + Glicko pairwise; judge Sonnet 4.6; 4,000-char truncation | Claude Fable 5 (2192), Opus 4.7 (2179), GPT-5.5 (2019) in the 2026-06-19 canonical zip (V) | Short-form; self-bias and slop bias uncontrolled |
| EQ-Bench Longform [5] | Plan → reflect → characters → 8×~1,000-word chapters; Sonnet 4 judge; chapter plus final rubric | "Claude Opus 5, 86.3" per a mirror (S) [7] | ~8k words, not 100k |
| WritingBench [21] | 1,000 queries, 6 domains, 5 query-specific criteria; judge Claude Sonnet 4.5 or a 7B critic | — | Mostly non-fiction |
| TTCW / "Art or Artifice?" [22] | 14 binary Torrance-style tests; 48 stories × 3 experts | — | 2023-era models; best protocol for expert human eval |
| LitBench [31]; HANNA [30] | Human-labeled story pairs / 6-criterion ratings; LLM-judge studies | (K) best off-the-shelf judge ≈73%, trained RMs ≈78% | Short stories |
| LongStoryEval [20], WebNovelBench [19] | Book-length review-based eval; Chinese web novels | (S) | Unverified |

**LLM-as-judge.** Use it for screening, and use a different model family from the writer (self-bias [4]). Calibrate it against a small human-labeled set drawn from Rehoboam's own outputs.

**Human protocol.** Blind pairwise A/B by trained readers on chapter excerpts; TTCW binary tests for flagship releases; a paid continuity hunt on sampled books (FABLES: about $200 per book for full-read annotation [25]); chapter read-through drop-off as the product metric.

**AI vs. humans in 2026.** Short pieces impress non-experts and saturate rubrics. There is no verified book-length parity with professional novelists, and experts still detect and disfavor prompted imitation [33](K).

### 9. Precedents

| Precedent | What happened | Lesson |
|---|---|---|
| Fable Studio SHOW-1 / Showrunner [44](K) | 2023 paper: agent simulation in a town feeds an LLM writer that produces TV-style episodes; Showrunner app followed in 2025 | Simulation supplies *context*, not quality; it still needs a "showrunner" curation layer |
| Dwarf Fortress Legends / LegendsViewer [43] | Centuries of generated history exported as XML and browsed as hyperlinked logs; no prose novelization | Players value raw emergent history; a chronicle/encyclopedia mode is a product in itself |
| Caves of Qud mythic biographies [64](K) | Generates events, then post-hoc rationalizes them as in-world gospels with unreliable narrators | In-world document voice hides the seams; Rehoboam runs the reverse direction (true causes) |
| AI Dungeon / NovelAI lorebooks (K) | Keyword-triggered world info / story cards injected into context | Retrieval-injected canon reduces contradictions but never eliminates them; gate finished books |
| Sudowrite Story Bible → Outline → Chapters (S) [45] | The commercial standard workflow. It adopted Fable 5.1 and GPT-6 Astra but hasn't vouched for long-form quality | Bible-centric pipelines are the market norm; long-form quality remains the open problem |

### 10. Cost model

**Assumptions (my model).**
- 1.4 tokens per word (range 1.3–1.75 depending on tokenizer).
- Thinking tokens are billed as output.
- A 120K-token sifted graph.
- 3 candidate plans plus a judge.
- 30 chapters / 100 scenes.
- 1.3 revision passes per scene.
- Claim extraction and verification per scene; fixes on 30% of scenes.
- 2 whole-book continuity reads.
- Line edits.
- 25% waste/retry.

The result is about **21M input and 2.2M output tokens per novel**, with ~15× more output tokens generated than appear in the final prose.

| Config (100k-word novel) | No cache | Cached | Batch + cache |
|---|---|---|---|
| **Recommended**: Opus 5.5 plan/draft, Sonnet 5.5 critic, Haiku 5.5 checks | $107 | **$56** | **$28** |
| All Opus 5.5 | $126 | $72 | $36 |
| Premium: Fable 5.1 plan/draft, Opus 5.5 critic | $264 | $134 | $67 |
| Budget: Sonnet 5.5 + Haiku | $57 | $32 | $16 |
| GPT-6 Astra all | $316 | $186 | $93 |
| GPT-6.1 Sol all / Gemini 3.1 Pro all | $63 / $68 | $36 / $41 | $18 / $21 |
| Kimi K3 API / DeepSeek V4 Pro API | $95 / $36 | $55 / $17 | — |

- **Sensitivity (recommended, cached):** thinking ×0.5 → $47; ×2 → $75; 1.75 tokens/word → $59. Drafting and revision are about 60% of spend.
- **120-page screenplay** (~25k words, 55 scenes): recommended $29 cached / $15 batch. About $6 less if adapted from an existing novel plan and bible.
- **10×30-min audio-drama script** (~42k words, 80 scenes): $38 / $19.
- **Batch:** 50% off, and it stacks with caching [2]. Rehoboam can use it because scenes are state-independent once the plan exists.

**Local generation (inference).** A consumer GPU running Gemma 4 31B (EQ-Bench 1385, about Claude 3.7 Sonnet level) is viable for checker, summary and format roles and for draft-tier chronicles. It is not viable for flagship novels: the Elo gap is large and longform degradation is likely worse. ~2.2M output tokens at 30–40 tok/s takes about 15–20 hours. A server running GLM-5.x or DeepSeek V4 nears late-2025 closed models. Kimi K3 needs a datacenter.

---

## Recommendations for Rehoboam

**Pipeline** (each stage writes a versioned artifact with provenance):

| # | Stage | Input → Output | Model (default) | Quality gate |
|---|---|---|---|---|
| 0 | Epoch digest and **story-worthiness** | Sifter backbone (30–50 events) + 1-hop causal neighborhood + actor state snapshots (traits, goals, beliefs, relationships) → digest | Deterministic + Haiku 5.5 | Protagonist(s) in ≥5 backbone events; causal graph mostly connected; else route to **Chronicle/Saga mode** (5–20k words) |
| 1 | Story selection | Digest → 3 candidate proposals (logline, theme, POV set, start/stop points, form) | Opus 5.5 (effort high) | Cross-family pairwise judge + coverage heuristic; optional user pick |
| 2 | Discourse plan (JSON schema) | Proposal → chapters, POV, scene/summary/flashback per event, reveal schedule, thread table, tension curve, epigraph sources | Opus 5.5 / Fable 5.1 | Schema-valid; ≥95% backbone coverage or justified omissions; causal parents precede or flash back; no epistemic leaks; no >3-chapter tension plateau; midpoint turn at 45–60% |
| 3 | Bible | Plan + sim → voice sheets, places, culture lexicon, style card, canon ledger, empty embellishment ledger, ban lists | Opus 5.5 | Every named entity traces to the sim; ban list includes other authors' coined terms |
| 4 | Chapter beats | Plan + bible → scene beats (goal/conflict/outcome; sequel beats) referencing event IDs or a licensed embellishment type | Opus 5.5 | Each beat typed T0/T1/T2 |
| 5 | Scene drafting (parallel, batchable) | Style card + PD exemplars + bible extract + **scene fact sheet with entry state** + beats + recent-phrase avoid list → prose + sidecar of new T2 claims | Opus 5.5 (Fable 5.1 premium) | Length ±20% |
| 6 | Scene QA | Prose → claims → graph/ledger check + NLI + style metrics + slop/repetition | Haiku 5.5 + local MiniCheck-class + code | 0 T0/T1 contradictions; 0 T3; style metrics inside the card's ranges; slop density under threshold. Max 2 auto-fix loops, then escalate |
| 7 | Chapter critique → revision | Chapter + rubric (EQ-Bench longform criteria + LAMP taxonomy) → edit list → revised scenes | Critic: GPT-6.1 Sol or Sonnet 5.5 (cross-family preferred); reviser: writer model | Rubric ≥ threshold; re-run stage 6 |
| 8 | Book pass | Manuscript (~140K tok) → issue proposals → graph-verified fixes; cross-chapter n-gram dedupe; line edit | 1M-context model; fixes by writer | 0 critical continuity issues; opening/closing diversity |
| 9 | Exporters | Structured scenes → EPUB/MD (optional annotated "chronicle notes"), Fountain (validated via screenplain/afterwriting), radio script + TTS manifests | Deterministic code | Parse round-trip, page count, voice-map completeness |

**Opinionated choices:**
- **Models:** Claude Opus 5.5 at effort `high` by default; Fable 5.1 for a premium edition. Use a cross-family critic, Haiku 5.5 checks, and Batch for drafting. Re-benchmark on our own eval before locking, since Opus 5.5, Fable 5.1 and GPT-6 Astra lack verified creative data.
- **API realities:** current Claude models have no temperature control and always think. They can return `refusal`, and grim war content may trip classifiers, so enable fallbacks and log refusals [1].
- **Interiority needs data.** Ask the sim to log per-actor beliefs, goals and the recorded reasons for decisions on sifted events. Otherwise all interiority is T2 invention.
- **Form order:** Chronicle, Saga, and Documents first; Novel for gated epochs. Screenplay adapts from the novel plan, and audio drama from the screenplay JSON.
- **"R08-Eval":** 20 fixed epochs × configurations, tracking contradictions, coverage, slop, cross-book similarity and judge Elo, plus a quarterly blind human panel.
- **UI copy:** "A consistent, readable novelization of what really happened in your world, in a chosen voice. A first draft, not literature."

---

## Risks & unknowns

- **Quality and expectations.** Promising Herbert/Tolkien-grade work invites disappointment and legal exposure. Homogenization means every user's novel may sound alike.
- **Legal.** Named-author marketing, coined-term leakage, estate enforcement, EU Art. 50 marking, platform AI-disclosure rules (K), and voice-likeness law for TTS casting (K). §6 needs counsel review.
- **Simulation fitness.** Emergent histories are episodic and crowded, so "chronicle-itis" (log-summary prose) is the likeliest failure. The story-worthiness gate is mandatory.
- **Checker reliability.** NLI and LLM checkers miss indirect contradictions [25][26]. Only graph-structured claims are hard guarantees.
- **Benchmarks.** EQ-Bench is short-form and Claude-judged, so it may favor Claude writers. Longform leaders are unverified, and leaderboards disagree [8].
- **Model churn and non-reproducibility.** Gemini 4 is reportedly imminent [47], API behavior shifts, and outputs can't be seeded. Pin model IDs, and treat prose as immutable editions keyed by plan hash plus model ID.
- **Unverified this session:** current eqbench.com standings; creative quality of Opus 5.5 / Fable 5.1 / GPT-6 Astra; 2026 preprints; legal holdings and dates; TTS pricing.

---

## Sources

1. https://platform.claude.com/docs/en/about-claude/models/overview: Anthropic model IDs, 1M ctx/128K out, prices, sampling/thinking constraints (V, via bundled Claude API reference cached 2026-10-06)
2. https://platform.claude.com/docs/en/build-with-claude/prompt-caching: cache write 1.25×/2×, reads 0.05× on Opus 5.5 and 0.025× on Fable 5.1; Batch 50% stacks with caching (V, via bundled reference)
3. https://github.com/BerriAI/litellm/blob/main/model_prices_and_context_window.json: third-party price/limit registry for GPT-6, Gemini 3.x, Kimi K3, DeepSeek V4, GLM (V, fetched 2026-10-15; secondary)
4. https://github.com/EQ-bench/creative-writing-bench: CW v3 method (Sonnet 4.6 judge, 4,000-char truncation, bias caveats) plus `elo_results.zip` canonical snapshot dated 2026-06-19 (V)
5. https://github.com/EQ-bench/longform-writing-bench: 8×1,000-word chapters, criteria, slop lists (V)
6. https://eqbench.com/creative_writing_longform.html: live leaderboards (not reachable this session)
7. https://sophon.at/evals/eqbench-longform-creative-writing: mirror claiming Claude Opus 5 tops longform at 86.3 (S)
8. https://www.digitalapplied.com/blog/which-ai-model-writes-best-leaderboards-disagree: claims EQ-Bench vs arena.ai creative rankings diverge (S)
9. https://arxiv.org/abs/2210.06774: Re3 (S)
10. https://arxiv.org/abs/2212.10077: DOC (S)
11. https://github.com/google-deepmind/dramatron: Dramatron repo and professional feedback; paper arXiv 2209.14958, CHI 2023 (V)
12. https://github.com/google-deepmind/tell_me_a_story: Agents' Room dataset; paper arXiv 2410.02603, ICLR 2025 (V/S)
13. https://arxiv.org/abs/2401.17268: Weaver (S)
14. https://github.com/THUDM/LongWriter: LongWriter/AgentWrite, arXiv 2408.07055 (V)
15. https://arxiv.org/abs/2506.18841: LongWriter-Zero, ICLR 2026 (V mention / S)
16. https://github.com/THU-KEG/StoryWriter: StoryWriter, arXiv 2506.16445 (V)
17. https://github.com/principia-ai/WriteHERE: Beyond Outlining / heterogeneous recursive planning, EMNLP 2025 (V)
18. https://arxiv.org/abs/2609.35759: NstAgent narrative state tracking, Sep 2026 (S)
19. https://arxiv.org/abs/2608.26177: outline-stage comparison (S, title only); also WebNovelBench https://arxiv.org/abs/2505.14818 (S)
20. https://www.emergentmind.com/topics/longstoryeval: LongStoryEval/NovelCritique summary (S, secondary)
21. https://github.com/X-PLUG/WritingBench: WritingBench, 1,000 queries, Claude Sonnet 4.5 judge (V)
22. https://github.com/salesforce/creativity_eval/tree/main/Art_or_Artifice: TTCW tests and data, arXiv 2309.14556 (V)
23. https://arxiv.org/abs/2309.14556: "Art or Artifice?" results on LLM vs expert, LLM-as-judge (K for numbers)
24. https://github.com/salesforce/creativity_eval/tree/main/Writing_Alignment: LAMP corpus and idiosyncrasy taxonomy, CHI 2025 (V)
25. https://github.com/mungg/FABLES: book-length faithfulness; LLM raters fail on unfaithful claims; $5.2K / 26 books (V)
26. https://github.com/kabirahuja2431/FlawedFictions: plot-hole detection; LLM generation +100% plot holes (V)
27. https://github.com/marzenakrp/nocha: NoCha book-claim pairs; humans ~97% (V)
28. https://github.com/adobe-research/NoLiMa: long-context degradation beyond literal matching, ICML 2025 (V)
29. https://github.com/Liyan06/MiniCheck: efficient grounded fact-checking; Bespoke-MiniCheck-7B (V)
30. https://github.com/dig-team/hanna-benchmark-asg: HANNA and "Do LMs enjoy their own stories?" TACL (V)
31. https://github.com/drfein/LitBench: LitBench code (V); agreement figures (K)
32. https://github.com/sam-paech/antislop-sampler: backtracking anti-slop sampler; also /antislop-vllm and /slop-forensics (V)
33. https://arxiv.org/abs/2510.13939: Chakrabarty, Ginsburg, Dhillon 2025, reader preference, prompting vs fine-tuning author styles (K; ID from memory)
34. https://github.com/MoonshotAI/Kimi-K3: 2.8T/104B open weights, 1M ctx, license terms (V)
35. https://github.com/nyousefi/Fountain: Fountain reference parser; spec at https://fountain.io/syntax (V / K)
36. https://github.com/ifrost/afterwriting-labs: Fountain→PDF, stats, CLI (V)
37. https://github.com/vilcans/screenplain: Fountain→FDX/HTML/PDF (V)
38. https://github.com/piersdeseilligny/betterfountain: VS Code Fountain editor (V)
39. https://github.com/lmparppei/Beat: GPL Fountain editor (V)
40. https://github.com/nari-labs/dia: `[S1]/[S2]` dialogue TTS, nonverbal tags, Apache-2.0 (V)
41. https://github.com/microsoft/VibeVoice: 90-min 4-speaker TTS; TTS code removed 2025-09-05 (V)
42. https://github.com/mkremins/felt: Felt story sifter (V)
43. https://github.com/Kromtec/LegendsViewer: Dwarf Fortress legends export/viewer (V)
44. https://fablestudio.github.io/showrunner-agents/: SHOW-1 / Showrunner agents paper (K)
45. https://feedback.sudowrite.com/changelog/new-ai-model-claude-fable-51: Sudowrite adopting Fable 5.1, long-form untested (S)
46. https://openai.com/blog/gpt-6-astra: GPT-6 Astra launch, 2026-09-03 (S)
47. https://www.infoworld.com/article/4226642/google-plans-gemini-4-release-before-year-end-2.html: Gemini 4 timing (S)
48. https://www.copyright.gov/ai/: USCO AI reports, Parts 2–3 (K)
49. https://www.courtlistener.com/?q=%22Bartz%22+Anthropic: pointer for Bartz v. Anthropic, Kadrey v. Meta, In re OpenAI Copyright Litigation dockets (K)
50. https://www.courtlistener.com/?q=Polychron+Tolkien: Tolkien Estate v. Polychron, 2023 (K)
51. https://authorsguild.org/: Authors Guild AI positions and Human Authored certification (K)
52. https://artificialintelligenceact.eu/article/50/: EU AI Act Art. 50 transparency obligations (K)
53. https://arxiv.org/abs/1703.09902: Gatt & Krahmer, NLG survey (Reiter–Dale pipeline) (K)
54. https://aclanthology.org/D17-1239/: Wiseman et al., RotoWire data-to-document metrics (K)
55. https://arxiv.org/abs/2305.14251: FActScore atomic-fact precision (K)
56. https://en.wikipedia.org/wiki/Fabula_and_syuzhet: fabula/syuzhet; Genette's *Narrative Discourse* categories (K)
57. https://liquidnarrative.csc.ncsu.edu/: Young lab, Bae & Young flashback/foreshadowing, Cheong & Young Suspenser, Darshak (K)
58. https://ojs.aaai.org/index.php/AIIDE: AIIDE proceedings, Cardona-Rivera et al. Indexter, 2012 (K)
59. https://cs.uky.edu/~sgware/: Ware, Sabre narrative planner (AIIDE 2021), Glaive (K)
60. https://nickm.com/curveship/: Montfort, Curveship narrative-variation system (K)
61. https://arxiv.org/abs/2311.04459: Wang et al. 2023, pacing in long-form story planning (K; ID from memory)
62. https://arxiv.org/abs/2501.00273: "Echoes in AI," plot-diversity loss (K; ID from memory)
63. https://www.science.org/doi/10.1126/sciadv.adn5290: Doshi & Hauser 2024, AI raises individual creativity but lowers collective diversity (K)
64. https://dl.acm.org/doi/10.1145/3102071.3110574: Grinblat & Bucklew 2017, Caves of Qud mythic biographies (K; DOI from memory)
65. https://escholarship.org/uc/ucsc: J. Ryan, *Curating Simulated Storyworlds* (UCSC PhD, 2018) (K; exact URL unverified)
66. https://arxiv.org/abs/2407.01119: "Pron vs Prompt," novelist vs GPT-4 (K; ID from memory)
