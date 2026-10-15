# R11 — Product Landscape, Precedents, Player Reception, Platform Policy and Scope Lessons

*Research date: 2026-10-15. Analyst stance: skeptical. Prepared for the Rehoboam systems architect.*

**Evidence limits (read first).** WebFetch could not resolve hosts in this session, so no primary page was opened in full. Verified claims rest on web-search snippets tied to the cited URL, often secondary press, and sales figures from Raijin, SteamSpy and Vaporlens are third-party estimates. **[BK]** marks the analyst's background knowledge, which was **not re-verified**. *Inference* marks analysis.

---

## TL;DR

- **The North Star is a 10-year vision, not a product.** No shipped game combines even two of its four pillars at 100k–1M agents. DF took about 20 years with two developers [1][BK]. CS2, from a funded studio, missed its performance targets [21] and players report the sim crawling at about 200k population [25]. **Verdict: drop it from v1 positioning.**
- **Five modalities is the biggest risk.** Solo and small-team deep sims ship one modality after 6–20 years: Songs of Syx about 6 years to EA [7]; Shadows of Doubt about 7 [12]; Manor Lords about 7 [17]; Caves of Qud 15 to 1.0 [13]; URR still pre-1.0 after 15 [15][16]. **Verdict: ship one wedge.**
- **Wedge: the Time-Machine (modality 2) plus a thin layer of Oracle verbs (modality 4), on a 2D/2.5D map, with no LLM required.** It targets the proven DF-Legends / CK3 / WorldBox / RimWorld audience, shows off what is actually differentiated (determinism, story sifting, causal graphs), and avoids both the 3D cost and the AI backlash.
- **Sentiment toward generative AI is hostile and getting worse.** Among gamers, 85% are below neutral and 63% chose the most negative option. Negativity toward AI quests and dialogue rose from 46% to 77–83% [52]. Among developers, 52% say GenAI is harmful (up from 30% and 18%), including 63% of narrative staff [50][51]. **Verdict: never lead with "AI".**
- **AI-native games mostly land at "Mixed"**: Suck Up! 51% [34], Vaudeville about 48% [38]. The exceptions are self-selected AI-as-premise titles (Whispers from the Star 92% [33], AI2U 88% [47]). No AI-NPC game has verified sales of several hundred thousand. Inworld pivoted to voice infrastructure [32].
- **Steam policy is workable but visible.** Player-facing LLM text is "live-generated": it must be disclosed with guardrails described, and players can report it [56][57]. Procedural generation is not "AI" on itch.io [60]; Epic has no labels [58]. **Verdict: the core needs no disclosure; make the LLM features optional and local.**
- **Remove "novels styled after Herbert/Tolkien/Hamilton" from all marketing.** It targets writers, the group most hostile to GenAI [50], and invites estate and IP trouble (Inference). Hamilton is a living author.
- **The Living Diorama is a funnel, not a business.** Demand is real (Wallpaper Engine about 978k reviews [65]; Bongo Cat about 194k peak CCU [70]; Rusty's about 330k units [68]), but prices run $0–6, the space is saturating [72][73], and a live 100k-agent sim breaks the CPU budget an ambient app must keep. **Verdict: ship it later, as cheap deterministic replay.**
- **"Rehoboam" fits thematically but carries brand risk.** Its main association is Westworld's dystopian AI that predicts and steers lives [75][77]. No USPTO hit was found, but this was not a clearance [79]. **Verdict: use it as the engine or Oracle codename.**
- **AI framing: "the simulation decides; language models only describe."** Off by default, local or bring-your-own model, causal provenance on every sentence, no AI art or voice in marketing, full disclosure. *Clair Obscur*'s scandal was a disclosure mismatch [61]. *Arc Raiders* survived its backlash with a reported 4M+ sales [62]; an unknown indie would not have that cushion (Inference).

---

## Draft claims checked

| Claim (from the project brief or implied) | Verdict | Evidence |
|---|---|---|
| "Bridges the procedural permanence of DF, the macro-logistics of SimCity, the social physics of deep literature and the visual immersion of modern 3D engines" | **Unsupported / aspirational** | DF: about 20 years, two developers, abstract visuals [1][BK]. CS2 missed its launch benchmarks [21] and crawls at about 200k population [25]. No precedent combines even two pillars at this scale. |
| 100k–1M inhabitants with individual-grade drama | **High risk; possible only with LOD simulation** | Songs of Syx (solo): tens of thousands [BK]. CS2: about 200k simple agents strain the CPU [25]. DF: a few hundred dwarves [BK]. CK3 culls characters [BK]. Project Sid's 1,000 LLM agents was a research run [45]. |
| LLM NPC dialogue is a selling point | **Negative for the core audience** | AI dialogue is 83% negative [52]. Suck Up! 51% [34]; Vaudeville about 48%, "evenly split" [36][38]. Exceptions are AI-as-premise titles: Whispers 92% [33], AI2U 88% [47]. |
| Novels "styled after Herbert/Tolkien/Hamilton" | **Reject as positioning** | Narrative staff are 63% negative [50]. Live text needs Steam disclosure plus guardrails [56][57]. Named-author mimicry carries legal and reputational risk (Inference). |
| Living Diorama as a standalone wedge | **A funnel, not a business** | Demand: Wallpaper Engine about 978k reviews [65]; Bongo Cat about 194k peak CCU [70]; Rusty's about 330k [68]. Glut: 940 Idler releases in 2025 [72], lookalikes [73]. |
| The Oracle is a novel mechanic | **Not novel; a proven niche** | KoDP / Six Ages, Black & White, RimWorld's storyteller [BK]. Only the combination with a causal graph and hidden "mimics" is new. |
| Deterministic sim plus LLM facade sidesteps backlash | **Partly** | itch.io exempts procedural generation [60], but any LLM text triggers Steam's live-generated label [56][57], which is what the 85% react to [52]. |
| A solo developer or small team can ship five modalities | **False on precedent** | Table G: URR pre-1.0 after 15 years [15][16]; CoQ 15 years to 1.0 [13]. |
| "AI-native" is a validated category | **Supply up, demand unproven** | About 20% of 2025 Steam releases disclose AI [54]. The biggest AI audiences are free toys: Infinite Craft [44]; Death by AI's 20M claim is unverified [80]. |
| inZOI proves AI NPCs sell | **Confounded** | The 1M first week [26] reflects demand for a Sims alternative, and Smart Zoi is optional [27]. |
| "Rehoboam" is a clean, ownable name | **Strong theme, unverified, brand-risky** | No USPTO hit, only REHOBOT [79], and this was not a clearance search. It collides with Westworld's AI [75][77]. |

---

## Landscape

### A. Deep simulation and "story generator" precedents

| Title | Team | Years to first paid / 1.0 | Scale | What it simulates | Reception / sales | Lesson for Rehoboam |
|---|---|---|---|---|---|---|
| **Dwarf Fortress** (Bay 12 / Kitfox) | 2 brothers, plus Kitfox for the Steam edition | Started about 2002, free alpha 2006, Steam Dec 2022 (about 20 years) [BK] | Fortress: about 100–200 dwarves; world-gen: centuries of history [BK] | Materials through personalities through history; Legends mode; Adventure mode added to Steam later (2024) [BK] | 160k in the first 24 hours (Kitfox had projected that for two months); about 300k in a week [1]; about 500k in December [2]; 800k in a year [3]; 1M+ on Steam [4]; more than $7M to Bay 12 in weeks [1] | Sixteen years of a free community built the demand. The commercial unlock was **UI, graphics and a publisher**, not new simulation. Even DF launched on Steam with one modality (Fortress) [BK]. |
| **RimWorld** (Ludeon) | Tynan Sylvester plus a small team [BK] | Kickstarter 2013, Steam EA 2016, 1.0 2018 [BK] | 3–20 colonists | Needs, moods, relationships; an "AI storyteller" that paces events | The Kickstarter sought about 1,000 sales; it passed 1M [5][6] | The "story generator" pitch sells when a **director** paces drama around a **small, legible cast**, not when the world is large. |
| **Songs of Syx** (Gamatron) | 1 ("Jake", Stockholm) [7] | About 6 years to EA 2020; no 1.0 found in 2026 [7][8] | Tens of thousands of citizens [BK] | City logistics, species, war | About 198k estimated copies, about $3M gross, 95% of 8,930 reviews (Raijin estimate) [8]; 200k–500k owners (SteamSpy) [9] | A solo developer *can* do large populations, using 2D sprites, aggregate simulation and years of Early Access. Rehoboam's scale ambition already has a solo precedent; the 3D does not. |
| **Shadows of Doubt** (ColePowered) | 1 (Cole Jefferies) plus publisher Fireshine [10][11] | About 6–7 years to EA Apr 2023; 1.0 Sep 2024 [10][12] | One city; every citizen has a home, job and schedule | Procedural city; crimes arise from the sim | [BK] well reviewed | A **single crisp verb** (investigate) on top of a city sim made it sellable. Modality 3 should borrow this: give street level a job, not "talk to anyone". |
| **Caves of Qud** (Freehold) | Small core [BK] | 15 years, about 9 in EA; 1.0 Dec 5 2024 at $29.99 [13][14] | One world with procedurally generated history | Generates sultans' histories, then retells them as myth [BK: Grinblat & Bucklew FDG 2017 paper] | Described as stable and steadily growing for years [13] | "Simulate the event, then narrate it in an authored voice" already produces literary-feeling legends **without an LLM**. |
| **Ultima Ratio Regum** | 1 (Mark R. Johnson) | Started Apr 2011, first release Jul 2012 [15]; 0.11 still in progress May 2025 [16] | Planetary cultures | Religion, semiotics, art, history | Free; cult following [BK] | **The cautionary twin of Rehoboam:** a maximal procedural-culture scope is still pre-1.0 after 15 years. |
| **Kenshi** (Lo-Fi) | 1, later a small team [BK] | About 12 years to 1.0 (2018) [BK] | Squads in a faction world | Factions, squads, survival | [BK] commercial success | It sold on freedom plus brutality ("you are not special"). An identity statement beat feature count. |

### B. Character drama, god sims and indirect influence

| Title | Team | Dev / release | Scale | Simulates | Reception | Lesson |
|---|---|---|---|---|---|---|
| **Crusader Kings III** | Paradox (large) | 2020 [BK] | Tens of thousands of characters, with culling [BK] | Dynasties, traits, schemes, succession | [BK] 1M in about a month | Rehoboam's flagship prompt ("feudal + dragons + father-son succession") **is CK3's turf**. Compete on centuries-scale history and causal inspection, not on character drama alone. |
| **WorldBox** | Solo or tiny (Maxim Karpenko) [BK] | Mobile 2018, Steam EA 2021 [BK] | Thousands of pixel units | God sandbox, kingdoms, disasters | [BK] very positive; YouTube-driven | A god toy with lo-fi visuals succeeds through spectacle and creator content. **Proof that modality 2 needs no 3D.** |
| **Wildermyth** | 2-person core [BK] | 1.0 2021 [BK] | A party across generations | Procedural characters in authored comic vignettes | [BK] very positive | Hand-authored story fragments filled with simulated people beat freeform text. |
| **King of Dragon Pass / Six Ages** | Tiny (A Sharp) [BK] | 1999 / 2018 [BK] | One clan | Advisor counsel, rituals, myth, indirect outcomes | [BK] cult classics | **The closest precedent to the Oracle.** "Counsel, not control" works when consequences are legible and delayed. Note the inversion: in KoDP the advisors counsel you, while in Rehoboam you are the advisor. |
| **Black & White** | Lionhead (large) [BK] | 2001 [BK] | One creature plus villages | A learning creature, belief | [BK] strong launch, judged overpromised | The learning creature was the memorable part. Hype ("it learns anything") outran delivery, a direct warning for "talk to any NPC about anything". |
| **Mount & Blade II** | TaleWorlds (mid-size) [BK] | About 10 years, 1.0 2022 [BK] | Hundreds of lords | Sandbox feudal politics | [BK] commercial hit; politics called shallow | Even a mid-size team needed a decade. The combat hook carried a thin political simulation. |

### C. City and life sims at scale, plus cozy dioramas

| Title | Team | Release | Scale reality | Reception | Lesson |
|---|---|---|---|---|---|
| **Cities: Skylines II** | Colossal Order (mid-size) / Paradox | Oct 2023 | Developer: "we have not achieved the benchmark we targeted"; missing LODs, inefficient CPU use [21]. Players: about 1 sim-minute per real second at about 200k population, CPU-bound [25] | CEO's "might not be for you" apology [22]; "rushed rehabilitation" called a cautionary tale [24]; development moved to Iceflake in 2026 [23] | **Render scale and sim scale fail together.** Prove the sim budget headless before adding 3D. |
| **SimCity (2013)** | Maxis | 2013 [BK] | Small plots; agent simulation exposed as shallow [BK] | Launch disaster [BK] | Players notice "fake agents". Don't overclaim individual simulation if most agents are statistical. |
| **The Sims 4 / inZOI** | EA / Krafton (large) | inZOI EA Mar 2025 | One household at a time | inZOI 1M in a week [26]; Smart Zoi uses on-device AI [27] | Life-sim demand is enormous but its cast is tiny. AI is a side feature, not the reason people buy. |
| **Frostpunk** | 11 bit (mid-size) | 2018 [BK] | Hundreds of citizens, abstracted [BK] | [BK] commercial hit | Moral-dilemma framing makes an aggregate simulation feel personal. That is useful for Oracle "counsel" choices. |
| **Manor Lords** | 1 plus contractors | EA Apr 2024 after about 7 years [17] | Village scale | 1M in about 24 hours, 3M wishlists [17]; 2M in under 3 weeks [18]; 3M, then 4M [19] | A **visual trailer drove wishlists**. "Solo" is overstated: contractors joined after a 2020 Patreon and a 2021 Epic MegaGrant [20]. |
| **Tiny Glade / Townscaper** | 2 / 1 [BK] | 2024 / 2020–21 [BK] | No simulation | [BK] Tiny Glade about 600k in its first month | **Diorama toys sell on beauty, with no failure state and no simulation.** The Living Diorama competes on that axis, not on simulation depth. |

### D. AI-native games and products, 2023–2026

| Product | What the AI does | Status | Reception / traction | Praise | Complaints |
|---|---|---|---|---|---|
| AI Dungeon / Latitude | Freeform LLM text adventure | 2019; **Voyage** beta Apr 2026 on Gemini [41] | 1M players in six weeks (2019–20) [42]; 2021 moderation backlash [43] | Unlimited freedom | Incoherence; moderation and privacy fights [43] |
| Hidden Door | Licensed-IP fan-fiction RPG with guardrails | Relaunched Aug 2025, free [40] | Favorable headlines, no user numbers [40]; PC Gamer doubted the audience size [39] | Rights-cleared worlds | Unproven demand |
| Inworld (Origins) | Voice-driven LLM NPC interrogation demo | 2023 demo [31] | The company pivoted to on-engine libraries and voice infrastructure; GDC 2025 presence was "subdued" [32] | Tech wow | No shipped hit; developers wanted local logic [32] |
| Suck Up! | Persuade LLM residents to let you in | Own launcher 2023, Steam 2025 [34] | **Mixed, 51%** [34]; claims 50M social views [35] | Streamer comedy | [BK] Repetition, latency, token costs |
| Vaudeville | LLM-voiced detective | 1.0 2025 [38] | **Mixed, about 48%** [38]; "evenly split" [36] | Novelty | "Lifeless" voices; "questioning to nowhere" [37] |
| 1001 Nights | Words spoken in the story become weapons | Demo only, release TBA [49] | Award nominations [49] | Design concept | Unshipped |
| Whispers from the Star (Anuttacon) | Voice conversation with one AI character | 2025 | **Very Positive, 92%** at launch [33] | Stella's human-likeness; transparent AI co-created with human actors [33] | Divides AI-averse players [33] |
| Dead Meat (Meaning Machine) | Interrogate suspects in your own words | Unconfirmed release [48] | 95% of 68 playtesters enjoyed it (developer co-ran the study) [48] | "Exhilarating" freedom | "Overwhelming" freedom [48] |
| AI2U | AI companion / escape room | Steam | **Very Positive, 88%** of 1,069 [47] | — | — |
| Infinite Craft | LLM combination crafting | 2024, web [44] | Viral, free | Discovery; "First Discovery" badges | Short-session toy |
| Altera Project Sid | 1,000+ LLM agents form a Minecraft society | 2024 research [45] | Press fascination: taxes, religion | Emergent culture | Research only; costly |
| Fable Showrunner | Prompt-to-animated-TV | Public Aug 2025; Amazon investment [46] | About 10k alpha users; no independent reviews [46] | — | — |
| inZOI Smart Zoi / PUBG Ally | On-device small language models via NVIDIA ACE | Optional feature / Arcade test June 2026 [27][28][29] | "Real-time AI teammates are hard to ship reliably" [30] | — | Hardware-gated |
| Death by AI | Party game judged by an LLM | — | Claims 20M players, **unverified** [80] | — | — |

**Pattern (Inference).** Players praise a human-feeling character in a tight frame, streamable comedy and discovery. They dislike lifeless TTS, conversations that go nowhere [37], "overwhelming" open input [48] and the "AI slop" label itself [52][73]. AI works when it **is** the premise, the audience opted in, and the scope is one character or one room. It underperforms when bolted onto a systems game as a facade, which is Rehoboam's case.

### E. Sentiment and storefront policy

**Sentiment (verified).** The headline numbers from Quantic Foundry [52] and GDC 2026 [50][51] are in the TL;DR. Two details matter for segmentation: female and non-binary gamers are markedly more negative [52], and gameplay-system uses such as dynamic difficulty draw less hostility than generated art [53]. *Flashpoints:* the Indie Game Awards revoked *Clair Obscur*'s awards because the studio had declared no GenAI while placeholder AI textures had shipped (Dec 2025) [61]. Eurogamer gave *Arc Raiders* 2/5 over AI text-to-speech voices, yet it reportedly sold 4M+ in under two weeks [62]; it drew boycott calls [81], and Nexon's CEO said to "assume every game company is now using AI" [63]. Larian faced a backlash in Dec 2025 and pledged no GenAI content in the shipped Divinity [64]. *Supply:* about 7% of Steam's catalog, and about 20% of 2025 releases, disclosed AI use, about 60% of it for visuals [54][55].

**Platform rules (verified via secondary coverage; check the Steamworks text before filing).**
- **Steam:** Disclosure has been required since Jan 2024 and is split into *pre-generated* content (shipped assets, which must not be infringing) and *live-generated* content (created while the game runs, for which developers must describe their guardrails) [58][56]. The **Jan 2026 revision** narrowed disclosure to content that players consume; development tools such as coding assistants are exempt [56][57]. Players can report illegal AI content through the overlay, and games without adequate safeguards risk removal [57]. Live-generated Adults Only sexual content is barred [57].
- **itch.io:** Disclosure fields and "AI Generated" sub-tags since Nov 2024. They are mandatory for asset pages and requested for games, and untagged AI assets are excluded from browse pages. **Traditional game AI and procedural generation are explicitly exempt** [59][60].
- **Epic Games Store:** No AI labels. Tim Sweeney (Nov 26, 2025) called such labels meaningless for game stores [58].

**What this means for Rehoboam (Inference).** The deterministic sim, history generation and templated chronicles need **no disclosure**. LLM dialogue, LLM chronicles, prompt-to-world compilation and synthetic audio are **live-generated** content on Steam. That means a store-page label, documented guardrails (local model, filters, no Adults Only content, no training on user data) and a report button for every player. Mitigations: an opt-in "Chronicler" module that is **off by default**, and/or bring-your-own-model through a mod API, which hobbyists already favor [BK: Skyrim's Mantella LLM mod]. Whether Valve treats a first-party BYO hook as live generation is unknown, so assume it does. [BK, legal review needed] The EU AI Act's Article 50 transparency duties took effect in Aug 2026.

### F. The desktop idle / ambient trend and the wallpaper market

| Product | Model | Traction | Note |
|---|---|---|---|
| Wallpaper Engine | About $4–5 utility plus Workshop | About 978k reviews, 98% positive; about 50M "estimated sales" (unreliable) [65]; top-10 on Steam [66]; peak about 150k CCU [67] | Huge install base; heavy use in China inflates engagement [66] |
| Rusty's Retirement | About $6 bottom-of-screen idle farm | About 330k units by mid-2024 (GameDiscoverCo) [68]; about 320k / $1.9M gross (estimate, 2026) [69] | It defined the docked-strip format |
| Bongo Cat | Free plus cosmetics | Peak about 194.5k CCU (May 2025); about 150k monthly average (June 2026) [70] | Played for Steam presence and cosmetic drops |
| Desktop Mate | Free plus character DLC (Miku about $15) | 1.7M downloads (Nov 2025) [71] | Licensed IP DLC is the business |
| Idler genre | — | 572 (2024), 940 (2025), 1,566 (2026 through September) Idler-tagged releases; 5.6% reach 1,000 reviews vs 4.2% across Steam [72]; "54 idle games in one week, eerily similar," many with AI disclosures [73]; Idle/Incremental is #5 by hit count in 2025 [74] | Rapid saturation |

**Verdict (Inference).** "A living diorama as wallpaper" is a **credible acquisition channel** and a weak profit center. The wins are low-priced or free, and they are monetized through cosmetics or licensed DLC. The successful ones are also *computationally trivial*. A 100k-agent simulation plus a 3D renderer running all day on a laptop conflicts with what the format promises. Two fixes: (1) a desktop mode that plays back a **pre-computed deterministic history** (rendering only, so the sim cost is near zero), or (2) a small live world of 1k–5k agents. Possible free teaser: a lightweight web or "scene" wallpaper that links to the game. [BK, unverified] Wallpaper Engine may restrict sharing of application-type wallpapers on the Workshop, so check its rules.

### G. Scope lessons: time to first release

| Game | Team | To first public build | To paid EA / 1.0 | What validated it |
|---|---|---|---|---|
| Dwarf Fortress | 2 | About 4 years (free 2006) [BK] | About 20 years to Steam [BK][1] | Free alpha plus forums and a Let's Play culture |
| RimWorld | About 1–3 | About 1–2 years (Kickstarter alpha) [BK] | About 5 years to 1.0 [BK] | Kickstarter with a playable alpha [5] |
| Songs of Syx | 1 | About 6 years [7] | EA 2020; no 1.0 [8] | Free demo, then EA |
| Shadows of Doubt | 1 | Devblog 2018 [10] | About 7 years to EA [12] | Devblog plus trailer; one verb |
| Caves of Qud | About 2 | About 2010 [BK] | 15 years to 1.0 [13] | Free roguelike plus EA |
| Kenshi | 1, then small | Years before EA [BK] | About 12 years to 1.0 [BK] | Alpha sales |
| Manor Lords | 1 plus contractors | Trailer about 2020 [BK] | About 7 years to EA [17] | Trailer, then 3M wishlists [17] |
| Ultima Ratio Regum | 1 | About 1 year [15] | No 1.0 after 15 years [16] | — |
| Tiny Glade | 2 | About 1–2 years [BK] | About 2–3 years [BK] | Viral GIFs plus demo [BK] |

**Implication (Inference).** The median ambitious solo or small-team simulation reaches paid Early Access after **about 6–7 years**, with **one** modality and **one** crisp hook. Rehoboam lists five modalities, each of which has been the entire product of some precedent: Time-Machine is WorldBox or DF Legends; Oracle is KoDP; Street-Level is Shadows of Doubt plus an LLM; Diorama is Rusty's or Tiny Glade; Compiler is Showrunner. A naive estimate is 5 × 5–7 years. Shared simulation infrastructure reduces that, but the 3D street level alone is a CS2-class rendering problem. Plan for **one modality at EA, about 3–4 years in, with a free playable prototype inside 12 months** to start the DF/RimWorld-style community flywheel.

### H. Naming and brand: "Rehoboam"

- **Biblical:** Rehoboam was Solomon's son, whose harsh answer at Shechem split the united monarchy (1 Kings 12) [78]. **That is a failed father-to-son succession**, an apt echo of the "father-son succession" prompt.
- **Westworld S3 (HBO, 2020):** Incite's machine, built by Serac after a nuclear attack on Paris (its predecessors were Saul, David and Solomon). It predicts and steers lives and sidelines unpredictable "outliers"; Dolores leaks its projections [75][76][77]. It maps exactly onto a Foresight mechanic, and so does its **dystopian frame, an AI manipulating humanity**, at the peak of anti-AI sentiment [52].
- **Trademark and search:** No USPTO REHOBOAM record surfaced, only REHOBOT (machinery) [79]. **This is not a clearance**, so run TESS/EUIPO searches in Classes 9, 41 and 28. [BK] It is also a 4.5-litre Champagne bottle size, which is low risk. Searches for "Rehoboam AI" will return Westworld pages (Inference).
- **Design idea (Inference):** Westworld's Rehoboam is undone by outliers. Make the player *the* outlier, the one variable the Oracle's foresight cannot model.
- **Recommendation:** Keep "Rehoboam" as the **engine or in-world Oracle name**, and choose a clearable consumer title that doesn't signal "AI overlord". Avoid Westworld iconography such as the ring-sphere or Incite-style UI.

### I. Audience segments

| Segment | Proxies | Values | Pays | AI tolerance | Fit for Rehoboam |
|---|---|---|---|---|---|
| Deep sim / strategy players | DF, RimWorld, Songs of Syx, CK3 | Depth, fairness, modding, performance, emergent stories | $20–35 premium plus DLC; will tolerate Early Access [1][5][8] | Low for generated content [52]; high for procedural generation | **Primary.** Largest proven paying base. |
| "Story generator" fans and content creators | DF Legends readers, CK3 AAR writers, WorldBox YouTubers | Shareable moments, legends browsers, export, spectacle | Same as above; they drive discovery | Mixed; they want *their* story, not "slop" | **Primary amplifier.** Build export and share tooling. |
| Writers wanting story seeds | World Anvil / Campfire users [BK] | Control, originality, ownership | Subscriptions for world-building tools [BK] | **Lowest** (narrative staff 63% negative [50]) | Poor for LLM novels; decent for structured "history seed" exports. |
| Ambient / cozy players | Rusty's, Bongo Cat, Wallpaper Engine, Tiny Glade | Calm, beauty, low CPU, no failure | $0–10, cosmetics [68][70][71] | Low, and skews female [52] | Funnel only; keep AI invisible there. |
| AI hobbyists | AI Dungeon, SillyTavern, LLM mods [BK] | Local models, bring-your-own model, hackability | Will pay; small and vocal | High | Secondary: serve them through a mod API, never as the headline. |

---

## Recommendations for Rehoboam

**1. Positioning.** Sell it as a **history engine**: "A world that runs for a thousand years, deterministically, and stops you when the story gets good." Comparable-titles pitch: *Dwarf Fortress Legends meets Crusader Kings, with a time machine and an Oracle.* Lead with determinism, causality ("click any event, see why it happened") and centuries-scale dynasties. Don't mention AI above the fold.

**2. The wedge is modality 2 plus a thin modality 4, in 2D or 2.5D.**
- *Core loop:* generate a world, run it at high speed, auto-pause on a sifted event, inspect the causal graph, then optionally intervene as the Oracle with 3–5 verbs (counsel a ruler, plant an omen, expose a mimic), spending Foresight.
- *Scale target for EA:* 10k–50k individually simulated people plus aggregate cohorts, with 1,000 years of history in minutes. LOD the rest. Don't market "1 million" until it holds on mid-range hardware.
- *World generation:* launch with **hand-built parameterized scenarios** (feudal succession, megacity contagion) and expose the parameters. Offer the LLM "prompt to world config" compiler later as an optional convenience whose output is a human-readable config file. That keeps the LLM out of the replay path.

**3. Sequence the other modalities as expansions, not launch features.**
- *Diorama (1):* the first post-EA add-on, built as **replay of computed history**, docked like Rusty's, with a strict CPU budget. Use it for virality, at free or $5.
- *Narrative Compiler (5):* ship first as a **deterministic Chronicle export** (templated prose in Caves of Qud's style, Markdown, Fountain), with optional LLM "polish" behind a toggle. Drop named-author presets. Offer abstract voices instead, such as "saga", "court chronicle" or "noir".
- *Street-Level (3):* post-1.0, and only with a Shadows of Doubt-style job (investigate a succession plot, find the mimic). Use templated dialogue grounded in simulation facts, with an optional local model. Treat this as the riskiest, most expensive modality.

**4. Metrics and benchmarks.**
- *Prototype (≤9–12 months):* a headless simulation that runs 1,000 years at 10k agents in under 10 minutes on a mid-range CPU; bit-identical replay from a seed; testers rate at least one auto-pause per 10 simulated years as "interesting" (3 or more out of 5).
- *Community:* a free itch.io or Steam Playtest build. Use DF and RimWorld as the model: their demand came from years of public builds [1][5].
- *Pre-EA:* [BK heuristic] 30k–50k wishlists. Manor Lords' 3M is an outlier, not a target [17].
- *EA reviews:* at least 85% positive. Songs of Syx is 95% [8]. AI-native titles sit around 50% [34][38]; that is the floor to stay away from.
- *Year one:* about 1,000 Steam reviews, which [BK, Boxleiter heuristic] implies about 30k–60k units. Median session of 60 minutes or more. At least 20% of players export or share a chronicle.
- *Unit economics:* LLM marginal cost per player close to $0 (local or bring-your-own). No hosted-inference bill against a one-time $25 price.
- *Diorama:* under 5% CPU on a mid-range laptop, auto-pausing for fullscreen apps.

**5. Anti-patterns to avoid.** (1) Launching or marketing five modalities (URR [16]; Black & White's hype [BK]). (2) Building 3D before the headless sim meets its budget (CS2 [21][25]). (3) Leading with "AI", AI art or AI voices (Quantic [52]; Arc Raiders [62]). (4) Under-disclosing (Clair Obscur [61]). (5) "Talk to anyone about anything" (Vaudeville [37]; Dead Meat [48]). (6) Letting LLM output write back into sim state, which breaks replay and makes "the AI decided" true. (7) Named-author style mimicry. (8) Claiming individual simulation for agents that are actually statistical (SimCity 2013 [BK]).

**6. How to present AI (the anti-backlash playbook).**
- Use the phrase **"Simulation-first: every event is computed by the engine. Optional language models only describe what already happened."**
- Turn the Chronicler module off by default. Make it local-first with bring-your-own model, with no cloud and no training on player data.
- Put **provenance in the UI**: every generated sentence links to the event IDs and causal edges it describes. The causal graph is the strongest defense against "slop".
- Complete Steam's live-generated disclosure accurately, and describe the guardrails: local model, filters, no Adults Only content [56][57].
- Use only human-made or procedural art and audio in marketing. Credit human writers for templates.
- On the store page, call the core what it is: "procedural / systemic simulation", which itch.io itself excludes from AI tagging [60].

---

## Risks and unknowns

- **Evidence quality.** Primary pages (Steamworks, GDC, Quantic Foundry) were not opened. The figures come from secondary search snippets, sales numbers are third-party estimates, and [BK] items are unverified.
- **Policy drift.** Steam has revised its AI rules twice in two years [56][58]. Whether a BYO-LLM hook or mod API counts as "live-generated" is unknown, and EU AI Act Article 50 needs legal review [BK].
- **Sentiment.** It is trending more hostile [50][52]. Big hits survive it (Arc Raiders [62]); unknown indies may not.
- **Hardware gating.** Local LLMs need VRAM, which shrinks the market for modalities 3 and 5. inZOI and PUBG Ally used vendor-specific on-device stacks [27][28].
- **Competition and moat.** CK3 owns succession drama, WorldBox owns spectacle, DF owns legends. Determinism plus causal inspection plus story sifting is not yet proven to be a reason to buy.
- **Still unverified:** inZOI's retention; Songs of Syx's 1.0 status; Tiny Glade's sales; Wallpaper Engine's true user count; Death by AI's 20M claim [80]; REHOBOAM trademark status; Valve's exact guardrail wording.

---

## Sources

1. https://gameworldobserver.com/2022/12/13/dwarf-fortress-300k-units-sold-zach-tarn-adams-millionaires — DF: 160k sales on day one (Kitfox), about 300k in a week, more than $7M to Bay 12.
2. https://gamespot.com/articles/dwarf-fortress-sells-nearly-500k-copies-in-december/1100-6510369/ — DF nearly 500k in December 2022 (Tarn Adams forum post).
3. https://www.gamasutra.com/business/dwarf-fortress-has-topped-800-000-sales-in-just-over-a-year — DF 800k in just over a year.
4. https://www.keengamer.com/articles/news/dwarf-fortress-sells-one-million-copies-on-steam/ — DF passes 1M on Steam.
5. https://www.pcgamesn.com/rimworld/rimworld-sales-numbers — RimWorld 1M+; the Kickstarter sought about 1,000 sales.
6. https://blog.habrador.com/2019/06/why-rimworld-sold-more-than-1-million.html — Analysis of why RimWorld succeeded.
7. https://videogamegeek.com/videogamedeveloper/45296/gamatron — Gamatron is "Jake" in Stockholm; about 6 years of development at the time of the entry.
8. https://raijin.gg/app/1162750/Songs_of_Syx — Songs of Syx estimate: about 198k copies, about $3M gross, 95% of 8,930 reviews.
9. https://steamspy.com/dev/Gamatron+AB — Songs of Syx 200k–500k owners (estimate).
10. https://en.wikipedia.org/wiki/Shadows_of_Doubt — Shadows of Doubt EA Apr 2023, 1.0 and consoles Sep 2024.
11. https://giantbomb.com/wiki/Companies/ColePowered_Games — ColePowered is a one-person studio.
12. https://wireframe.raspberrypi.com/articles/walking-the-voxelated-city-streets-in-shadows-of-doubt-a-fully-simulated — About 7 years of development, fully simulated city.
13. https://www.pcgamesn.com/caves-of-qud/1-0-steam — Caves of Qud 1.0 after 15 years, about 9 in EA.
14. https://gamespress.com/CAVES-OF-QUD-10-LAUNCHES-DECEMBER-5-AFTER-15-YEARS-OF-DEVELOPMENT — Caves of Qud 1.0 press release, $29.99.
15. https://en.wikipedia.org/wiki/Ultima_Ratio_Regum_(video_game) — URR started Apr 2011, first release Jul 2012.
16. https://www.markrjohnsongames.com/2025/05/10/ultima-ratio-regum-0-11-update-35-lots-of-book-progress/ — URR 0.11 still in development (May 2025).
17. https://exputer.com/news/pc/manor-lords-1-million-copies-sold/ — Manor Lords 1M in 24 hours; about 7 years, mostly alone; 3M wishlists.
18. https://windowscentral.com/gaming/pc-indie-darling-manor-lords-hits-2-million-copies-sold-three-weeks-after-huge-launch-on-steam-game-pass — Manor Lords 2M in under 3 weeks.
19. https://abit.ee/en/games/manor-lords-slavic-magic-hooded-horse-city-builder-strategy-game-game-sales-en — Manor Lords 3M, then 4M.
20. https://www.euracreative.fr/en/the-success-of-manor-lords-one-person-alone-is-not-enough-to-produce-a-hit — The "solo" framing is disputed: contractors after Patreon and an Epic MegaGrant.
21. https://destructoid.com/?p=419528 — CS2 developer explains missed performance benchmark, missing LODs, CPU inefficiency.
22. https://www.pcgamesn.com/cities-skylines-2/ceo-apology — CS2 CEO apologizes for "might not be for you".
23. https://heise.de/-11083454 — Paradox and Colossal Order part ways; Iceflake takes over in 2026.
24. https://gamasutra.com/production/the-rushed-attempt-to-rehabilitate-cities-skylines-ii-is-becoming-a-cautionary-tale — CS2's rehabilitation called a cautionary tale.
25. https://steamcommunity.com/app/949230/discussions/0/4343240481094062255 — Player-reported CS2 simulation slowdown at about 200k population (anecdotal).
26. https://www.gamedeveloper.com/press-release/krafton-hits-1-million-sales-of-inzoi-in-first-week-of-early-access-launch — inZOI 1M in the first EA week.
27. https://www.digitaltrends.com/gaming/inzoi-pubg-ai-cpc-characters-ces-2025/ — inZOI / PUBG on-device AI "co-playable characters" at CES 2025.
28. https://press.krafton.com/KRAFTON-REVEALS-PLAYTEST-PLANS-FOR-PUBG-ALLY-BUILT-WITH-NVIDIA-ACE — PUBG Ally: on-device small language model, playtest plans.
29. https://labs.invenglobal.com/articles/22837/playerunknowns-battlegrounds-launches-ai-powered-ally-duo-mode-on-the-17th — "Ally Duo" test window, June 2026.
30. https://gamedev.net/news/4114/ — "Real-time AI teammates are hard to ship reliably."
31. https://www.axios.com/2023/08/07/inworld-ai-origins-demo — Inworld Origins demo (2023).
32. https://thefilibusterblog.com/inworld-ai-gdc-2025-qa-major-announcements-expected-despite-aaa-games-desire-for-secrecy/ — Inworld moves to local libraries; subdued GDC 2025.
33. https://www.notebookcheck.net/Steam-launch-New-interactive-fiction-game-debuts-to-Very-Positive-reviews-may-divide-gamers-over-heavy-AI-integration.1087821.0.html — Whispers from the Star at 92% Very Positive; transparent AI use.
34. https://nodal.gg/game/suck-up-2726370 — Suck Up! Mixed, 51% of 189 reviews.
35. https://bouncewatch.com/company/proxima-1 — Proxima: $1.6M seed; claims 50M social views.
36. https://www.gamedeveloper.com/design/vaudeville-pre-mortem — Vaudeville developer: reviews "evenly split".
37. https://www.keengamer.com/articles/previews/vaudeville-preview-the-ai-questioning-to-nowhere/ — Vaudeville criticism: lifeless voices, frustrating questioning.
38. https://gg.deals/ca/game/vaudeville/ — Vaudeville about 48% positive (aggregator).
39. https://pcgamer.com/hidden-door-ai-game-narrative-rpg — Hidden Door preview; doubts about audience size.
40. https://hilarymason.com/projects-press/ — Hidden Door Aug 2025 relaunch coverage index.
41. https://respawn.outlookindia.com/gaming/gaming-news/design-your-own-world-latitude-unveils-voyage-ai-rpg — Latitude Voyage beta (Apr 2026, Gemini).
42. https://opendatascience.com/ai-dungeon-and-the-future-of-ai-powered-storytelling-a-conversation-with-creator-nick-walton/ — AI Dungeon 1M players in six weeks.
43. https://kulturegeek.fr/news-225673/ai-dungeon-developpeur-met-joueurs-dos — AI Dungeon moderation backlash (2021).
44. https://flowingdata.com/2024/02/02/infinite-craft-with-large-language-model/ — Infinite Craft runs on an LLM (Llama 2).
45. https://www.technologyreview.com/2024/11/27/1107377 — Altera Project Sid, 1,000+ agent civilization.
46. https://www.thewrap.com/amazon-invests-fable-ai-company-showrunner-streaming/ — Showrunner public launch; Amazon investment; about 10k alpha users.
47. https://store.steampowered.com/app/2880730/AI2U_With_You_Til_The_End/ — AI2U at 88% of 1,069 reviews.
48. https://bristoldigitalgamelab.blogs.bristol.ac.uk/tag/players — Dead Meat player study (n=68): 95% enjoyed; "overwhelming" freedom.
49. https://store.steampowered.com/app/2542850/1001_Nights/ — 1001 Nights: demo, release TBA.
50. https://www.businesswire.com/news/home/20260129438528/en/2026-State-of-the-Game-Industry-Report-Reveals-Widening-Effect-of-Layoffs-Broader-Perspectives-on-Generative-AI-Unionization-Tariffs-and-More — GDC 2026: 52% negative, 7% positive, 36% use; narrative 63% negative.
51. https://techspot.com/news/111128-more-than-50-game-developers-generative-ai-harming.html — GDC 2026 secondary coverage (2,300 respondents, trend from 18% to 30% to 52%).
52. https://quanticfoundry.com/2025/12/18/gen-ai/ — Quantic Foundry: 85% below neutral, 63% most negative; AI dialogue 83% negative.
53. https://www.statista.com/statistics/1664229/attitudes-towards-genai-in-video-games-by-use-case/ — Gamer attitudes to GenAI by use case.
54. https://www.videogameschronicle.com/news/steam-games-disclosing-generative-ai-use-are-up-800-this-year/ — Totally Human: Steam AI disclosures up about 800% in 2025.
55. https://www.notebookcheck.net/Analysis-Nearly-8-000-games-on-Steam-have-AI-generated-content.1060558.0.html — 7,818 of about 114k Steam games disclose AI; about 60% visuals.
56. https://finalboss.io/valve-narrows-steams-ai-disclosure-to-player-consumed — Jan 2026 Steam revision: player-consumed content only.
57. https://tbreak.com/steam-ai-disclosure-policy-update-developers/ — Steam: pre- vs live-generated content, guardrails, overlay reporting, Adults Only ban.
58. https://pcgamer.com/software/ai/epic-boss-tim-sweeney-thinks-stores-like-steam-should-stop-labelling-games-as-being-made-with-ai-it-makes-no-sense-he-says-because-ai-will-be-involved-in-nearly-all-future-production/ — Sweeney on labels; Epic has none; Steam disclosure since Jan 2024.
59. https://gamingonlinux.com/2024/11/itchio-store-now-requires-ai-generated-content-disclosures-for-assets — itch.io AI disclosure (Nov 2024).
60. https://www.gamesmarket.global/itchio-now-requires-gen-ai-content-to-be-tagged-in-all-games-and-asset-packs-711e824843d410eab51fcd9436836294/ — itch.io tags; procedural generation and game AI exempt.
61. https://www.gosugamers.net/entertainment/news/77758-clair-obscur-expedition-33-stripped-of-indie-game-of-the-year-award-for-ai-usage — Indie Game Awards revoke Clair Obscur's awards over GenAI.
62. https://80.lv/articles/eurogamer-s-2-5-arc-raiders-review-splits-the-gaming-community/ — Eurogamer 2/5 for Arc Raiders over AI voices; commercial hit.
63. https://www.inverse.com/gaming/arc-raiders-ai-voice-controversy-nexon-ceo-every-video-game — Nexon CEO: "assume every game company uses AI".
64. https://www.thesixthaxis.com/2025/12/17/larian-and-bloomberg-clarify-statements-on-ai-usage-in-divinity-development/ — Larian GenAI backlash and clarification.
65. https://vaporlens.app/app/431960/wallpaper_engine/stats/details — Wallpaper Engine: about 978k reviews, 98% positive; about 50M estimated sales (unreliable).
66. https://mein-mmo.de/en/why-the-german-wallpaper-engine-for-4-e-is-one-of-the-10-biggest-games-on-steam,857028/ — Wallpaper Engine among the top 10 on Steam.
67. https://tracker.gg/population/steam/431960 — Wallpaper Engine peak about 150k CCU.
68. https://newsletter.gamediscover.co/p/how-rustys-retirement-idle-farmed — Rusty's Retirement about 330k units (GameDiscoverCo).
69. https://raijin.gg/app/2666510/Rustys_Retirement — Rusty's estimate: about 320k units, $1.9M gross.
70. https://steambase.io/apps/3419430 — Bongo Cat peak about 194.5k CCU; monthly averages.
71. https://gamebiz.jp/news/417918 — Desktop Mate 1.7M downloads.
72. https://www.steampageanalyzer.com/niches/idler — Idler releases 2024–2026 and hit rates.
73. https://pcgamer.com/gaming-industry/steam-week-in-review-54-idle-games-released-on-steam-last-week-and-many-look-eerily-similar — Idle-game glut and lookalikes.
74. https://howtomarketagame.com/2026/01/27/what-the-hell-happened-in-2025/ — Idle/Incremental #5 by hit count in 2025.
75. https://www.theringer.com/2020/04/12/tv/westworld-season-3-episode-5-recap-genre-dolores-serac — Westworld S3E5 recap: Rehoboam origins.
76. https://www.syfy.com/syfy-wire/westworld-season-3-end-biblical-references-breakdown — Westworld's biblical naming (Solomon and Rehoboam).
77. https://westworld.fandom.com/wiki/Engerraund_Serac — Serac, Incite and Rehoboam's predict-and-steer role (fan wiki).
78. https://www.biblicaltraining.org/library/rehoboam — The biblical King Rehoboam and the split of the kingdom.
79. https://trademarks.justia.com/792/26/rehobot-79226987.html — The only near-match mark found (REHOBOT); no REHOBOAM hit (not a clearance).
80. https://nexworld.wiki/en/ai-games/ai-games-panorama-2026/ — Death by AI's "20M players" claim (unverified, low reliability).
81. https://www.gfinityesports.com/article/arc-raiders-is-facing-boycotts-for-its-use-of-ai-content — Arc Raiders boycott calls over AI content.
