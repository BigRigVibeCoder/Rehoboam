# R09: Simulation science for macro-history in Rehoboam
### Which models to use, with their equations and parameters, so 1,000-year runs are both plausible and dramatic

*Research dossier, 2026-10-15. Evidence conventions: `[n]` cites the numbered source list. Sources tagged **[V]** were checked this session through search-result text. Sources tagged **[B]** are standard literature cited from background knowledge and were **not** re-fetched, because the session's network policy blocked page fetches and the shared web-search budget ran out partway through the research. Statements marked *Inference* are my own reasoning or my own quick numerical experiments (Python, run this session). Treat [B] numbers as starting points that need checking before they go into code.*

---

## TL;DR

1. **Verdict: the draft macro model cannot produce believable 1,000-year history.** Births and deaths have no density dependence, there is no age structure, and there is no economy, polity, war, culture or climate. The likely outcomes are runaway growth, random-walk extinction, or a static map. Every rise-and-fall arc the Time Machine promises would be missing.
2. **Use structural-demographic theory (SDT) as the backbone of macro-history.** Population pressure lowers wages and raises rents, which drives elite overproduction and state fiscal distress, and the resulting instability resets the system. This produces secular cycles of about 200–300 years (plausible range 120–450) with 40–60-year "fathers-and-sons" violence waves layered on top [3][4]. **Correction:** in my integration, the simplified two-variable demographic-fiscal model [6] does *not* sustain cycles. It rises once and then the state goes permanently broke. Adding a lagged internal-warfare/instability variable produced limit cycles with period ≈ 6–7/r years (*Inference*).
3. **Replace `Poisson(λb) − Poisson(λd)` with cohort-component (Leslie) demography with density-dependent vital rates.** A 0.1 %/yr error in r compounds to ×2.7 over 1,000 years. In my simulation of 1,024 chunks with exactly balanced rates, 78 chunks went extinct and the 95th-percentile chunk reached 3.4× the median. With density dependence, none went extinct and the spread stayed within ±11 % (*Inference*).
4. **The draft's hazard Laplacian is the wrong model for contagion and for dragons.** `H' = H + α∇²H − βH + S` is *linear*. It has no epidemic threshold (R₀), no susceptible depletion and no immunity, and it simply relaxes to a static blob around each source. Use metapopulation SEIR(S) on a mobility graph (radiation/gravity flows) [23][25][B27]. Pre-modern plague did travel in diffusive *waves* [28], so a Fisher-KPP-style front is defensible for a feudal map, but only as a reaction term coupled to S/I compartments.
5. **The "witch-hunt" plot falls out of base rates, so it costs nothing to model.** With a 99 %-sensitive, 99 %-specific blood test at 0.1 % prevalence, 91 % of positives are false. With a 95 %/98 % test it is 95.5 % (*Inference*, standard Bayes [32]). Couple that with fear-driven contact reduction [26][27] and scapegoating dynamics [30].
6. **The minimum viable economy is 6–8 goods, regional markets, a transport graph and wages/rents/taxes.** Food demand is price-inelastic (the King–Davenant pattern [B]). Famine happens through *entitlement failure*, meaning wages fall relative to grain prices, and not only through stock depletion (Sen [B]). Pedestrian "errands" should be sampled from real macro flows.
7. **War should be resolved at campaign level.** Supply/forage range, seasonal attrition and disease decide most outcomes. Battles are a Lanchester/Helmbold kernel with morale breakpoints and terrain multipliers [B24]. Sieges are mostly starvation clocks.
8. **Politics combines legitimacy, elite numbers, succession law and empirical hazards.** Eisner found about 22 % of European monarchs (600–1800) died violently and about 15 % were murdered, which is roughly 10 murders per 1,000 ruler-years, with the rate declining over time [12][13]. Use Granovetter/Epstein thresholds for revolts [B4][B5].
9. **Dragons are apex predators and energetics constrains them hard.** About 10,000 kg of prey supports about 90 kg of a carnivore [B16]. A 5-tonne dragon would therefore need roughly 550 t of standing prey, which forces livestock raiding and wide ranges (*Inference*). Resource regeneration must be logistic, so over-harvest beyond MSY = gK/4 collapses the stock. Linear regen can never collapse.
10. **Climate drives the plot.** Use a seasonal clock plus spatially correlated AR(1) harvest anomalies and rare volcanic shocks. Climate anomalies correlate with conflict and population decline (cold phases [B18]; heat and extreme rainfall [B17]) and with witch trials [30]. Back-to-back failures, not single bad years, produce famines [B].
11. **Scenario horizons must differ.** A feudal world supports 50–1,000 years, provided technology diffuses (logistic/Bass adoption [B12]), including possible regression in small, isolated populations [B14]. A 1-million-person modern city run for 1,000 years has no defensible model. Cap it at decades, or label it speculative science fiction.
12. **Plausibility and drama both need a validation harness and a story sifter.** Run seed ensembles against stylized-fact bands (Zipf cities, heavy-tailed wars, regicide rates, cycle periods) using pattern-oriented modelling [B23]. Emergent output is mostly boring unless something curates it (Ryan 2018 [B]).

---

## Draft claims checked

| Draft claim / design element | Verdict | Evidence |
|---|---|---|
| `H' = H + α∇²H − βH + S` models "hazard" (dragons, pathogens) | **Wrong for both** | The equation is linear and its steady state is (β − α∇²)H = S, a static screened-Poisson blob with decay length √(α/β) (*Inference*). Epidemics need nonlinear S·I incidence, a threshold and depletion [B1][B2]. Modern contagion follows mobility networks ("effective distance") [25] and can be predicted from flows without knowing epidemic parameters. Medieval plague spread in waves, consistent with networks that lacked long-range ties [28]. Dragons are discrete agents. Explicit FTCS diffusion is also only stable for αΔt/Δx² ≤ 1/4 in 2-D (*Inference*, standard numerics). |
| `ΔPop = Poisson(λ_birth) − Poisson(λ_death) − casualties` per 16×16 chunk | **Unstable** | This is a linear birth–death process with E[N] = N₀e^{(b−d)t}. Over 1,000 years, r = ±0.5 %/yr gives ×148 or ×0.007 (*Inference*). Pre-modern rates were both high (30–50 ‰) and nearly balanced [15], so tiny errors explode. The model ignores age structure, so it has no demographic momentum, no generation lag and no age-selective famine or war mortality [20][B3]. |
| Regenerative resource layers (linear regen) | **Insufficient** | Linear regen has no maximum sustainable yield and no tipping point, so a tragedy of the commons cannot emerge. Logistic regen with harvest has a fold bifurcation at H = gK/4 (*Inference*, textbook). |
| 512×512 grid with 16×16 chunks as the demographic unit | **Questionable** | That is 1,024 chunks, about 98 people each at 100k (*Inference*). This is too small for stable Poisson demography and arbitrary with respect to geography. Politics, markets and armies operate on provinces and routes, not squares. Keep the grid for terrain and climate and simulate society on a region graph. |
| Drama emerges from "unforgiving physics" alone | **Half true** | Systemic pressure is necessary but not sufficient. Most simulated events are dull, and emergent narrative needs story sifting [B]. CrisisDB finds crisis *outcomes* highly variable and mutually uncorrelated [11], which is good for drama but means outcome menus should be stochastic rather than scripted. |
| Dynasts are promoted when a chunk crosses a narrative threshold | **Partially wrong** | Succession requires genealogy *before* the crisis. Ruling and great houses must always be tracked at the agent tier. They are cheap: a few thousand people (*Inference*). |
| The Time Machine shows 1,000 years of demographic shifts, conquest, dynasties and contagion | **Plausible for feudal only with SDT, succession, war and tech** | Secular cycles (about 2–3 centuries) give 3–5 rise-and-fall arcs per millennium [3]. For a modern city there is no credible 1,000-year model. |
| One generic engine parameterized by an LLM-written manifest | **Feasible if bounded** | The Political Stress Indicator (PSI) model shows "sensitive dependence on its parameters" [9]. LLM-set parameters need hard bounds plus a burn-in check against stylized facts. |
| Psychology vectors (Despair, Hubris, Paranoia, FilialPiety, Corruption) drive the story | **Needs macro anchors** | They map naturally onto SDT quantities: Despair ≈ immiseration/MMP, Corruption ≈ elite income squeeze, Paranoia ≈ fear and purge rate. Without that coupling they are flavour text (*Inference*). |
| A covert hive-mind infection detectable only by blood test | **Coherent and rich** | SEIR plus a covert compartment, an imperfect test and behaviour coupling [26][27]. The PPV collapse at low prevalence makes witch hunts emergent [32]. |
| "Logistics dictate war" | **Unmodelled** | There is no supply, forage or season model in the draft. See §5. |

---

## Landscape / state of the art

### 1. Cliodynamics, SDT, and Malthusian/Ricardian dynamics

**Facts.** *Secular Cycles* (Turchin & Nefedov 2009) extends Goldstone's demographic-structural theory and tests it on population, prices and real wages, elite numbers and incomes, state finances and instability for medieval and early-modern England, France and Russia and for Rome [1]. A summary of the mechanism: population growth raises elite incomes and inequality, more elites compete for positions, and the strain on state finances ends in political crisis [2]. Turchin estimates the period of second-order human population cycles at 120–450 years, most likely 200–300 [3]. Within each secular cycle, violence comes in waves of about two generations (40–60 years, the "fathers-and-sons" cycle). These waves are minor during integrative phases and become civil wars during disintegrative phases, as in the Roman Republic in 133, 91–82 and 49–27 BC [4]. Turchin tested the population–instability feedback on early-modern England, Han and Tang China and Rome [5].

**Demographic-fiscal model (DFM).** One simplified form [6] is:

```
k(S)  = k0 · (1 + c·S/(s0 + S))            carrying capacity rises with state strength
dN/dt = r·N·(1 − N/k(S))                    Malthusian logistic population
dS/dt = ρ·N·(1 − N/k(S)) − β·N ,  S ≥ 0     taxed surplus minus per-capita state expenditure
```

A *Cliodynamics* paper describes the DFM as having "just six adjustable parameters" [34]. The Turchin–Korotayev variant writes the per-capita surplus as ρ(N) = c₁(1 − N/k) and adds internal warfare [8]. Wittmann & Kuehn (2024) propose a "demographic-wealth model" with different taxation assumptions. It has Hopf bifurcations, and limit cycles appear past critical parameter values, analogous to the ecological "paradox of enrichment" [7].

*Inference (own integration, r = 0.01–0.03, ρ = 1, β = 0.1–0.4, c = 3, s₀ = 10):* the simplified DFM above produces **one** boom, bankrupts the state, and settles at N = k₀ with S = 0. In continuous time the logistic never overshoots, so S never recovers. I added a lagged instability term that is active while the state is bankrupt:

```
dW/dt = a·[S = 0]·N/k − b·W           (a = 0.1, b = 0.05)
dN/dt = r·N·(1 − N/k) − δ·N·W         (δ = 0.01–0.05)
```

With this term the model gave **sustained cycles with period ≈ 210–250 years at r = 0.03, ≈ 300–370 at r = 0.02, and ≈ 390–490 at r = 0.015**, which matches Turchin's 200–300-year band. The lesson for Rehoboam: the destructive phase must *overshoot*. Instability has to kill, cut carrying capacity, and above all cull elites. Gentle logistic regulation alone flattens history.

**Political Stress Indicator.** Goldstone's index is Ψ = MMP · EMP · SFD (mass mobilization potential, elite mobilization potential, state fiscal distress) [9]. Turchin's operationalization: MMP = N_urb · A₂₀₋₂₉ / w, where w is the relative wage (wages/GDP per capita), N_urb is the urban fraction and A₂₀₋₂₉ is the youth share. EMP rises with elite numbers and falls with relative elite income. SFD = (debt/GDP) × distrust [9]. Sources disagree on EMP's exact functional form (exponential in [9]; ε⁻¹ · e/s in book notes [9]). The authors of the 2024 Poland study report chaotic sensitivity to parameters, which limits transfer between countries [9].

**Malthus and Ricardo.** In Clark's framework, birth rates rise and death rates fall with income, and the equilibrium income is set by the vital-rate schedules alone. A larger population lowers income through diminishing returns on fixed land [21]. Raising mortality (hygiene, plague) *raises* equilibrium income, a dramatic, counterintuitive lever [21]. Ashraf & Galor found that, for 1–1500 CE, better technology and land productivity raised population *density* but had insignificant effects on income per capita [22]. That is the stylized fact a feudal run should reproduce.

**Seshat.** Turchin et al. (2018) coded 414 societies from 30 regions with 51 variables. One principal component explains about ¾ of the variation in social complexity, meaning scale, hierarchy, governance, information systems and money co-evolve [10]. CrisisDB covers about 168 historian-identified crises. Their consequences (civil war, epidemic, population loss, reform) are highly variable and largely uncorrelated [11].

*Inference: are the cycles believable?* Yes, if three conditions hold. First, the period must scale with r (≈ 6/r). Second, the reset must be violent and elite-selective. Third, stochastic shocks (climate, plague) must jitter phase so that cycles never look clockwork. Pure ODE cycles look fake. Noise-driven quasi-cycles look historical.

### 2. Historical demography

**Facts.** Before the demographic transition, crude birth and death rates were both high (about 30–50 per 1,000) and roughly balanced [15]. For most of history about half of children died before 15. In Sweden in 1750–80 the figure was 40 %, in France about 45 % and in Bavaria about 50 % [14]. Infant mortality ran to roughly a quarter to a third in the first year [14][33]. Black Death mortality estimates range from Ziegler's one-third, through about one-half (Aberth, and commonly for England), to Benedictow's 60 %, which he raised to 65 % in his second edition using about 300 datasets [16][17]. Coale–Demeny model life tables come in four regional families (West, North, East, South) with 25 mortality levels each. The originals cap out at female e₀ ≈ 70–75, and Coale & Guo extended them in 1989 [18][19]. They are available in R as `demogR::cdmltw` [18]. In a Leslie matrix the dominant eigenvalue gives the long-run growth rate and its eigenvector the stable age distribution, and populations return to that distribution after perturbation [20].

**Background [B].** Typical modern-city CBR/CDR is about 8–12 ‰ with TFR below replacement. The Brass relational logit, Y(x) = α + β·Y_s(x) with Y = ½·ln((1−lₓ)/lₓ), lets two parameters (level α, shape β) generate a whole life table from a standard. Hajnal's European marriage pattern (late female marriage around 25, 10–20 % never marrying) was the main *preventive* check.

**Why naive Poisson ΔPop fails (*Inference*, own simulation).** ΔN ~ Poisson(bN) − Poisson(dN) is a linear birth–death branching process:

- Its mean drifts exponentially. With b − d = 0.001 the population grows ×2.7 per millennium, with 0.002 ×7.4, and with −0.005 ×0.007. In my run, a +0.1 %/yr bias turned 100k into 271k.
- Even with b = d exactly, variance grows linearly: Var = 2bN₀t. For N₀ = 98, b = 0.035 and t = 1,000, the standard deviation is about 84, roughly equal to the mean. In my simulation of 1,024 chunks, **78 chunks went extinct**, the median fell to 78, and the maximum chunk reached 613. Without migration, dead chunks stay dead.
- Adding simple density dependence (b·(1 − x), d·(1 + x) with x = N/K − 1) gave **zero extinctions and a 5–95 % band of 87–109**.
- Age blindness matters for drama. Plague, famine and war kill different ages and sexes. War kills young men, which skews sex ratios and the marriage market. Famine kills infants and the old. A succession crisis depends on whether the heir is a *child*. A Leslie/cohort model captures this, along with generational echoes about 25–30 years later.

### 3. Epidemiology for covert, test-detectable infection

**Compartments [B1][B2].** SEIRS with waning immunity:

```
dS/dt = −λ(t)·S + ω·R          λ(t) = β(t)·I/N  (force of infection)
dE/dt =  λ(t)·S − σ·E          1/σ = latent period
dI/dt =  σ·E − γ·I              1/γ = infectious period; R0 = β/γ
dR/dt =  γ·I − ω·R − μ_IFR...   ω = waning rate (SEIRS)
```

**Metapopulation.** With patches i = 1..n and mobility matrix M, λᵢ = βᵢ Σⱼ Mᵢⱼ Iⱼ/Nⱼ. The radiation model derives commuting and migration fluxes from the population distribution alone, with no fitted parameters, and beat gravity in tests (for example, 44 vs 6 observed commuters, against gravity's 1 and 1 and radiation's 66 and 2) [23][24]. Gravity has region-dependent exponents and analytic inconsistencies [23]. Radiation form [B]: Tᵢⱼ = Tᵢ · mᵢnⱼ / ((mᵢ + sᵢⱼ)(mᵢ + nⱼ + sᵢⱼ)), where sᵢⱼ is the population within radius dᵢⱼ. Network-driven contagion "reduces to simple wave propagation" in *effective distance* derived from flows. It predicts arrival times and can find outbreak origins [25].

**Is a reaction-diffusion PDE right?** For modern person-to-person contagion it is not: flows dominate geography [25][B27]. For pre-modern plague it is a partial match. Marvel et al. argue that the Black Death's well-defined waves imply long-distance contacts were exponentially rare, so the small-world effect is "a modern phenomenon" [28]. Noble's 1974 SI reaction-diffusion model of the Black Death produced travelling fronts [29]. A rebuttal using Venice's naval network argues that ports *were* small-world hubs [28]. *Inference:* the right v1 design is a metapopulation on a route graph. In a feudal world with weak long-range links it *produces* wave-like fronts, and sea lanes produce the jumps (Messina, Marseille). This makes the model scenario-agnostic. The Fisher-KPP front speed c = 2√(rD) is a useful sanity check, and observed Black Death inland spread was on the order of 1 mile/day [29].

**Behaviour coupling.** Epstein et al. model fear as a second contagion caught from the sick, the scared, or both. Scared individuals withdraw from circulation, small amounts of fear-driven flight change spatial dynamics dramatically, and when fear decays multiple waves follow [26]. Funk et al. find that awareness lowers final size but does not change the threshold in well-mixed populations. Local awareness can stop an outbreak, especially when the communication and contact networks overlap [27].

**Testing and false positives (*Inference*, Bayes [32]).** PPV = se·p / (se·p + (1−sp)(1−p)).

| prevalence p | se/sp 0.99/0.99 | 0.99/0.999 | 0.95/0.98 | false positives per 1M tested (0.99/0.99) |
|---|---|---|---|---|
| 0.1 % | **9 %** | 50 % | 4.5 % | 9,990 |
| 1 % | 50 % | 91 % | 32 % | 9,900 |
| 5 % | 84 % | 98 % | 71 % | 9,500 |
| 20 % | 96 % | 99.6 % | 92 % | 8,000 |

Early in a covert outbreak, therefore, **most people the state detains are innocent**. Purges of false positives destroy legitimacy and raise paranoia. If hive members avoid testing or infiltrate the labs, effective sensitivity falls while false positives stay constant. That is a "witch-hunt dial" with no scripting needed.

**Historical scapegoating.** Oster found a strong inverse correlation between temperature and witch trials across 11 European regions in 1520–1770, explained as scapegoating after weather-driven economic decline [30]. Critics note that neighbouring regions with the same weather had very different prosecution counts, and that there were no witch hunts in the 1315–17 Great Famine. They propose confessional competition instead (Leeson & Russ) [31]. *Inference:* model persecution as **shock × institutional permission × out-group availability**, not shock alone.

### 4. Economy at the right fidelity

**Game and ABM landscape [B].** In *Victoria 3*, pops (profession, culture, religion, wealth) work in buildings with selectable production methods. Goods prices move with the buy/sell ratio within about ±75 % of a base price, and standard of living feeds radicalism and interest-group clout [B34]. The design is rich, but late-game performance and abstract war have been criticized. *Songs of Syx* simulates individual citizens at city scale with hauling-based logistics, so its errands are real [B37]. *Dwarf Fortress* added an in-fortress dwarven economy (coins, rents) around 2010 and switched it off because of degenerate dynamics. It is a cautionary tale about money layered on agents that have no real needs [B31]. *Patrician/Anno* use town stock-dependent price curves with ship arbitrage, which is legible and fun. EURACE was a massively parallel agent-based model of the EU economy that needed HPC to run [B]. Samuelson's (1952) spatial price equilibrium says that in equilibrium the price difference between two connected markets is at most the transport cost, with trade flowing only where it equals the cost [B].

**Price shocks [B].** The King–Davenant pattern (late 17th c.): a harvest shortfall of 10/20/30/40/50 % raised grain prices by about 30/80/160/280/450 %. Food demand is highly inelastic. Sen's entitlement approach holds that famine is a collapse of command over food (wages relative to prices), which can occur with adequate aggregate stocks [B]. Scheidel's "Great Leveler" argues that only mass-mobilization war, revolution, state collapse and plague have historically compressed inequality [B]. After the Black Death, real wages rose and elites tried to legislate wages back down (Statute of Labourers, 1351) [B]. That is a ready-made drama chain.

**Transport costs [B].** In the Roman world (Diocletian's price edict, the ORBIS model), sea : river : land costs per ton-km were roughly 1 : 5 : 30–50. Water access should therefore dominate city placement and trade.

*Inference: the minimum economy that yields errands, famines, trade wars and inequality.* Six to eight goods, regional stocks, a price rule, an arbitrage rule on a cost-weighted graph, labour reallocation by wage, and class-based extraction (rent/tax). The full specification is in §Recommendations, M4.

### 5. Conflict

**Lanchester [B24].** Square law (aimed fire): dA/dt = −βB, dB/dt = −αA, conserving αA² − βB². Linear law (area fire or one-on-one): dA/dt = −βAB, dB/dt = −αAB, conserving αA − βB. Helmbold generalization: dA/dt = −β·B·(A/B)^{1−W}, with W ∈ [0,1] interpolating between the square law (W = 1) and the exponential/"logarithmic" law (W = 0). Engel (1954) fitted the square law to Iwo Jima [B24]. Fits to large WWII battles (Ardennes, Kursk) were ambiguous between laws [B]. *Inference:* pre-gunpowder melee sits closer to linear (W ≈ 0.3–0.6), because numbers matter less when only the front rank fights.

**Morale, rout and logistics [B].** In pre-modern battles most casualties happen in the rout and pursuit, after cohesion breaks at modest losses. Wargame convention (Dupuy-style breakpoints) uses roughly 10–30 % casualties. Engels (Alexander) and van Creveld (*Supplying War*) show that pre-rail armies were bounded by forage and pack-animal consumption: animals eat their own payload, so supply range is finite and armies had to keep moving. Disease killed more soldiers than battle until the 20th century. Paradox games approximate this with a province supply limit (development/terrain/buildings), monthly attrition above the limit, winter penalties, and sieges as periodic rolls against fort level [B32][B33].

### 6. Politics: legitimacy, succession, regicide, rebellion

**Facts.** Eisner studied 1,513 monarchs in 45 European monarchies between 600 and 1800. About 22 % of deaths were violent and about 15 % were murders, a rate of about 1,000 per 100,000 ruler-years (≈ 10 per 1,000 ruler-years). Battle deaths and homicide both declined significantly from the early Middle Ages to the 18th century [12][13].

**Background [B].** CK3's succession families are partition (confederate, partition, high partition), single-heir (primogeniture, ultimogeniture, seniority) and elective (feudal, princely, tanistry, Scandinavian). Partition fragments realms and breeds sibling wars [B32]. Kokkonen & Sundell (2014) find that adopting primogeniture in European monarchies (1000–1800) reduced the risk of deposition [B25]. Blaydes & Chaney (2013) document lengthening ruler tenure in post-feudal Western Europe [B]. Wrigley's classic estimate for a stationary pre-modern population: roughly 60 % of fathers leave a surviving son, 20 % only daughters and 20 % no surviving children [B]. *Inference check:* with TFR 5, 50 % sons and 50 % child survival, the number of adult sons is about Poisson(1.25), so P(no adult son) = e^{−1.25} ≈ 29 %. **Roughly every third or fourth succession is a heir crisis by demography alone.**

**Rebellion thresholds [B].** Granovetter (1978) gives each person a threshold θ for joining, and participation evolves as r_{t+1} = F(r_t). Shifting one person's threshold can turn a full riot into a single rioter, so outcomes are knife-edge [B4]. Epstein's (2002) civil-violence model sets grievance G = H·(1 − L) (hardship × illegitimacy) and arrest probability P = 1 − exp(−k·(C/A)_v) with k ≈ 2.3. An agent rebels if G − R·P > T (T ≈ 0.1). It produces punctuated outbursts and shows why legitimacy matters [B5]. Kuran's preference falsification explains "now out of never" revolutions in which private dissent is hidden until a cascade [B6]. This fits the hive mind: infected agents falsify their preferences.

### 7. Culture, ideology, religion, technology

**Background [B].**
- **Axelrod (1997):** F features with q traits each. Neighbours interact with probability equal to their cultural overlap and copy one differing trait. Small q yields monoculture and large q yields stable fragmentation, with a nonequilibrium transition [B7][B29].
- **DeGroot:** x(t+1) = W·x(t), which converges to a weighted consensus [B8].
- **Deffuant–Weisbuch:** if |xᵢ − xⱼ| < ε, then xᵢ ← xᵢ + μ(xⱼ − xᵢ). This gives about 1/(2ε) clusters [B9].
- **Hegselmann–Krause:** xᵢ ← mean{xⱼ : |xᵢ − xⱼ| ≤ ε}. Consensus for large ε, polarization or fragmentation for small ε [B10].
- **Abrams–Strogatz (language competition):** dx/dt = (1−x)·c·xᵃ·s − x·c·(1−x)ᵃ·(1−s), with a ≈ 1.31 and s the status of the language. Abrams, Yaple & Wiener later applied it to religious affiliation [B11][B40].
- **Conversion curves:** historic conversion (Bulliet on Islam, Stark's estimate of about 40 %/decade for early Christianity) follows century-scale S-curves [B].
- **Technology:** Bass diffusion f/(1−F) = p + qF [B12]. Kremer (1993): long-run technological growth ∝ population, because more people produce more ideas [B13]. Henrich (2004): small or isolated populations can *lose* skills (the Tasmanian case) [B14].

*Inference:* a feudal world must not be static over 1,000 years. Expect several agricultural, military and administrative innovations to diffuse, and possible regression after collapse.

### 8. Ecology

**Background [B].** Rosenzweig–MacArthur model:

```
dH/dt = r·H·(1 − H/K) − a·H·P/(1 + a·h·H)
dP/dt = e·a·H·P/(1 + a·h·H) − m·P
```

Raising K destabilizes the equilibrium into limit cycles (the "paradox of enrichment") [B15]. Carnivore density scales with prey biomass: 10,000 kg of prey supports about 90 kg of a given carnivore [B16]. *Inference:* a 1-t dragon needs about 110 t of standing prey, a 5-t dragon about 550 t (≈ 1,800 cattle at 300 kg), a 10-t dragon about 1,100 t. A realm of 100k people therefore supports single-digit dragons unless the manifest grants a "magic metabolism" multiplier. Livestock expansion (raising K) can trigger boom-bust dragon ages via the paradox of enrichment. Renewable resources should follow logistic regrowth with harvest, dR/dt = gR(1 − R/K) − H_t: MSY = gK/4 at R = K/2, and harvest above MSY collapses the stock.

### 9. Climate and seasons

**Facts.** Colder periods correlate with more witch trials [30].
**Background [B].** Hsiang, Burke & Miguel (2013) found that each 1σ shift toward hotter or more extreme rainfall raises intergroup conflict by about 14 % and interpersonal violence by about 4 % [B17]. Zhang et al. (2007) link cold phases to war and population decline over 1400–1900 [B18]. The Late Antique Little Ice Age (536–~660 CE) followed volcanic eruptions [B19]. Campbell & Ó Gráda show English famines came from *back-to-back* harvest failures (1315–17), not single bad years [B]. Medieval seed-to-yield ratios were about 3–5:1, so a bad year also eats next year's seed.

### 10. Existing world and history generators

| System | What it simulates | Known weaknesses / lessons |
|---|---|---|
| Dwarf Fortress worldgen [B31] | Civilizations, sites, historical figures, megabeasts, wars over chosen histories (presets up to about 1,050 years); Legends mode | Histories tend toward repetition and stagnation once land is claimed. Most events are raids, abductions and beast attacks. Economy disabled. Lesson: without endogenous decline mechanisms, worlds freeze. |
| Azgaar FMG [B30] | Static generation of states, cultures, religions, provinces, burgs and routes via cost-weighted expansion over Voronoi cells | No time dimension. Excellent as a *t = 0 initializer* for Rehoboam's region graph. |
| Ultima Ratio Regum [B36] | Deep generation of religions, ideologies, art and polities | Generation, not simulation. Good for culture-trait vocabularies. |
| Caves of Qud [B35] | "Sultan" histories: events generated first, causes rationalized afterwards (Grinblat & Bucklew 2017) | Shows that *post-hoc rationalization* produces legible myth cheaply. Rehoboam can use it as a narration layer over real causal chains. |
| WorldBox [B38] | God-sandbox kingdoms, wars, plagues | Shallow economy and logistics; drama comes from the player's hand. |
| Songs of Syx [B37] | Individual citizens, hauling logistics, mass battles | Shows tens of thousands of real-errand agents are feasible at city scale. |
| Victoria 3 [B34] | Pops, markets, production methods, interest groups | Best reference for a pop-plus-market tier. Watch for performance and opacity. |
| CK3 [B32] | Characters, dynasties, succession laws, factions | Best reference for the named tier. Partition is a drama generator. Character-count performance pressure. |
| EU4 [B33] | Provinces, development, supply, attrition, sieges, legitimacy | Logistics abstractions worth copying. History tends to "rail" (missions) and "blob" (AI expansion). |

### 11. Calibration and validation

Pattern-oriented modelling: calibrate against *multiple* patterns at different scales simultaneously, not a single target [B23]. Candidate stylized facts:

- City rank-size close to Zipf (slope ≈ −1) [B20].
- Interstate war severity heavy-tailed, power-law α ≈ 1.5 [B21].
- Empire lifespans roughly exponential with mean ≈ 220 years (Arbesman) [B22].
- Regicide and violent ruler death per Eisner [12].
- Secular-cycle period 200–300 years [3].
- Child mortality 40–50 % [14].
- Seshat co-variation of scale and governance [10].

---

## Recommendations for Rehoboam

### Architecture decisions (opinionated)

1. **Three clocks, two tiers, one region graph.** Keep the 512² grid for terrain, climate, biomes, physical hazards (fire, flood, smoke, dragon fear-zones) and rendering. Run **society on a region graph** of about 300–2,000 provinces, generated Azgaar-style from the grid, with routes weighted by mode cost. The clocks are *year* (demography, elites, legitimacy, culture, tech), *season* (harvest, markets, trade, campaigns, migration) and *day* (epidemics, battles, sieges, only while active). A 1,000-year Time-Machine run is about 4M region-season updates, which is trivial. Agent-tier dynasts are event-driven.
2. **The Laplacian stays only for physical fields.** Contagion, fear and rumour run on the mobility graph.
3. **Conservation invariants across tiers.** People, grain, coin and pathogens must be conserved when an agent is promoted or demoted. Every agent is "carved out of" its region's cohort cells.
4. **Stochastic everything, with binomial draws.** Deaths are Binomial(N, q), never Poisson (a Poisson draw can exceed N). Epidemics use Euler-multinomial tau-leaping. Each seed is a different history, and validation runs on ensembles.
5. **Crisis outcomes from a stochastic menu.** Following CrisisDB [11], a crisis rolls consequences (civil war, epidemic, population loss, reform, dynastic change) with weights from state variables, not fixed scripts.

### Systems library

| ID | System | Priority | Tier | Δt |
|---|---|---|---|---|
| M1 | Region graph, land and carrying capacity | **Must** | Aggregate | static / yearly |
| M2 | Cohort-component demography | **Must** | Aggregate (+ agents carved out) | year |
| M3 | Agriculture, seasons and climate shocks | **Must** | Aggregate | season |
| M4 | Minimal economy: markets, trade, wages, rents, taxes | **Must** | Aggregate | season |
| M5 | Metapopulation SEIRS + behaviour + testing | **Must** | Aggregate (agents for named) | day while active |
| M6 | Polity: elites, fiscal, legitimacy, instability (SDT + PSI) | **Must** | Polity aggregate | year (hazards monthly) |
| M7 | Succession and dynast agents | **Must** | Named agents | event / month |
| M8 | Campaign war: supply, attrition, battle, siege | **Must** | Army objects | day during campaigns |
| M9 | Validation harness | **Must** | Offline | per run |
| S1 | Religion/ideology competition + cultural drift | Should | Aggregate shares | year |
| S2 | Technology diffusion and loss | Should | Aggregate | year |
| S3 | Migration and urbanization | Should | Aggregate flows | season |
| S4 | Ecology: dragons, livestock, logistic resources | Should | Agents (dragons) + aggregate | season |
| S5 | Story sifter / chronicle generator | Should | Offline over event log | per event |
| C1 | Rumour network and preference falsification | Could | Aggregate + agents | week |
| C2 | Household micro-economy for visible pedestrians | Could | Micro (on-screen only) | minute/hour |
| C3 | Wealth-exchange inequality within classes | Could | Aggregate | year |
| C4 | Pathogen evolution (virulence/strain) | Could | Aggregate | per outbreak |
| C5 | Climate regime shifts (LIA-style) | Could | Aggregate | decade |

### System specifications

**M1: Region graph.** *State:* per region, area, arable land T, terrain class, water access, base yield A₀ and route edges with cost τᵢⱼ (sea : river : land ≈ 1 : 5 : 30) [B]. *Carrying capacity:* Kᵢ = T·A·(yield ratio − 1)/(subsistence per capita) × k(S) from M6. *Manifest:* land fertility multiplier [0.5, 2.0]; sea:land cost ratio [1:10, 1:60].

**M2: Demography.** *State:* nᵢ[class ∈ {commoner, burgher, elite}][sex][age 0–4…80+] (≈ 120 numbers per region). *Update (annual):* n(t+1) = L(t)·n(t) with
`fₐ(t) = fₐ⁰ · (y/y*)^{ε_f}` and `qₐ(t) = 1 − (1 − qₐ⁰)·(y/y*)^{ε_m}` (clamped), where y is food entitlement per capita (from M4), plus additive shock mortality from war, plague and famine (age-specific). Baseline qₐ⁰ comes from a Brass logit on a West-model standard [18][B]. *Couplings:* labour supply → M4; youth share → MMP (M6); deaths → succession (M7); age-specific plague/famine/war losses. *Manifest bounds:* e₀ ∈ [20, 45] (feudal) or [60, 85] (modern); TFR ∈ [1.2, 7.5]; ε_f ∈ [0.1, 1.0]; ε_m ∈ [0.5, 3.0]; elite fertility multiplier ∈ [1.0, 1.6]. **Hard guard:** absent tech change, the burn-in long-run mean r must be within ±0.3 %/yr for pre-modern scenarios.

**M3: Agriculture and climate.** *State:* seasonal phase, regional anomaly zᵢ, stored grain, seed stock, soil fertility. *Update:* `z_t = ρ z_{t−1} + σ(√(1−λ) ε_local + √λ ε_shared)`; yield = A·T·L_ag^α·exp(γz − γ²σ²/2)·volcanic(t). Volcanic events ~ Poisson(λ_v), giving −1σ to −3σ for 1–3 years. Seed is set aside before consumption, and spoilage runs at 5–15 %/yr. *Couplings:* yield → M4 prices → M2 entitlement; anomaly → conflict propensity (M6/M8, ≈ +14 %/σ intergroup [B17]); winter → M8 forage = 0; malnutrition → M5 IFR multiplier. *Manifest:* σ ∈ [0.08, 0.35]; ρ ∈ [0, 0.6]; λ (spatial sharing) ∈ [0.2, 0.9]; λ_v ∈ [0, 0.03]/yr; seasons ∈ {2, 4}.

**M4: Economy.** *Goods:* grain, livestock/meat, timber, ore/metal, cloth, tools/arms, luxuries (modern scenarios add energy, medicine/test kits and services). *State per region:* stock Q_g, price p_g, sector labour L_g, wage w, rent, tax rate τ_tax. *Update (seasonal):*
- Production: Y_g = A_g·L_g^α·Input_g^{1−α} (food uses land as input; tools raise A).
- Price: `p_{t+1} = p_t·(D/S)^{1/|ε_g|}`, clamped to [0.25, 6]·p_base for food, so that ε_food ≈ −0.25 to −0.4 reproduces King–Davenant-scale spikes.
- Trade: for each edge, `x_ij += κ·max(0, p_j − p_i − τ_ij)`, capped by merchant and ship capacity (converges toward Samuelson SPE).
- Labour: `L_g ← L_g·(1 + φ(w_g − w̄)/w̄)`.
- Distribution: wage income → commoners, rent (1−α share) → elites and church, tax → state (M6).
- Entitlement: y = (w·L + own production)/p_food feeds M2.

*Errands:* each on-screen pedestrian samples trips from these flows (buy bread, haul grain, report to the levy, queue for a test). One errand represents a fixed quantum of macro flow, so errands are never decorative. *Couplings:* embargo or war closes edges, which spikes prices (trade wars); plague cuts L, so wages rise and rents fall (post-1348 dynamics). *Manifest:* goods list from a fixed menu; α ∈ [0.4, 0.8]; |ε_food| ∈ [0.2, 0.6]; base tax ∈ [0.05, 0.4]; κ ∈ [0.01, 0.2].

**M5: Epidemics.** *State per region:* S, E, I, R (+ covert C for hive-mind), Q (quarantined), D, and tested-positive counts. *Update (daily, Euler-multinomial):* SEIRS as in §3, with λᵢ = βᵢ(t)·Σⱼ Mᵢⱼ Iⱼ/Nⱼ, where M comes from the radiation model on region populations [23] plus trade/army/pilgrim flows. Behaviour: `β_i(t) = β₀·(1 − κ_F·F_i)`, with fear `dF/dt = a_F·(visible deaths + publicized positives)/N − δ_F·F` [26][27]. Testing: positives = τ_test·(se·I_c + (1−sp)·S). Detained positives go to Q. False positives feed Paranoia and drain legitimacy (M6). Hive evasion lowers effective τ_test on C and can corrupt se. *Feudal plague mode:* add a rodent/flea reservoir with recurrence (Poisson reintroduction along ports, ~every 10–20 years historically [B]). *Couplings:* deaths → M2; labour loss → M4; Paranoia → purges and scapegoating (out-group selection from S1); armies and sieges are high-β patches (M8). *Manifest:* R₀ ∈ [1.0, 8.0]; latent days ∈ [1, 365]; infectious days ∈ [2, ∞) (∞ = chronic hive); IFR ∈ [0, 0.8]; ω ∈ [0, 1]/yr; se ∈ [0.5, 0.999]; sp ∈ [0.9, 0.9999]; κ_F ∈ [0, 0.8]; covert-evasion ∈ [0, 0.9].

**M6: Polity (SDT core).** *State per polity:* elite count E, elite income ε, treasury/debt S, legitimacy L ∈ [0, 1], army size, instability W, Ψ. *Update (annual):*

```
ε     = (rent_share·Y − c_status·E)/E
dE/dt = E·[r_e + u·(ε/ε* − 1)] − culling(W, war, purge)
dS/dt = τ_tax·Y − (g0 + g1·E + army_cost) − interest(debt)
MMP   = (w*/w)·urb·youth ;  EMP = (E/E*)·(ε*/ε) ;  SFD = (debt/Y)·(1 − L)
Ψ     = MMP·EMP·SFD      (normalized: Ψ = 1 at founding)
dW/dt = a·max(0, Ψ − Ψ*) − b·W     effects: −δ·N·W mortality, k ↓, elite culling ∝ W, S drain
L    += +victories +prosperity +faith-match −defeats −famine −false-positive purges −usurpation
```

Revolts use a Granovetter cascade per region with thresholds drawn from Beta(α, β) shifted by grievance G = Despair·(1 − L) and repression [B4][B5]. Elite revolt/coup hazard ∝ EMP·(1 − L). *Couplings:* this is the hub, taking wages and rents from M4, youth from M2, deaths from M5 and taxes from M3/M4. *Manifest:* tax ∈ [0.05, 0.4]; u ∈ [0, 0.05]; r_e − r ∈ [0, 0.01]; Ψ* ∈ [1, 3]; δ ∈ [0.005, 0.05]. **Guard:** burn-in must show a dominant period of 120–450 years for pre-modern scenarios [3] unless the manifest explicitly asks for a static utopia.

**M7: Succession and dynasts (agent tier).** *State:* every member of ruling and great houses (always tracked, a few thousand agents), age, health, legitimacy claim, psych vector {Despair, Hubris, Paranoia, FilialPiety, Corruption}, relationships. *Ruler hazards (monthly):* `h_murder = h₀·exp(b₁·log Ψ + b₂·(1−L) + b₃·Hubris_heir·(1−FilialPiety_heir) + b₄·regency)`, normalized so the ensemble mean is ≈ 10 murders per 1,000 ruler-years and violent deaths ≈ 15–25 % of reigns, declining over centuries if a "civilizing trend" flag is set [12]. *Succession law enum:* partition, primogeniture, seniority, elective, tanistry. Partition splits realms (sibling wars). Elective yields faction contests. Primogeniture lowers deposition risk [B25] but produces child kings (regency → hazard ↑). Demography alone yields an heir crisis in about 25–30 % of successions (*Inference*). *Psych mapping:* regional Despair ← entitlement deficit and bereavement; Paranoia ← fear F, purges and regicides; Corruption ← elite-income squeeze; a ruler's Hubris ← victories and legitimacy. *Manifest:* law enum; h₀ ∈ [0.003, 0.02]/yr; FilialPiety cultural mean ∈ [0, 1]; "catalyst" overrides (for example, a scripted aging king with an ambitious heir) as *initial conditions*, never as forced outcomes.

**M8: Campaign war.** *State:* army men M, morale m ∈ [0, 1], supply days d, node and path, sick fraction. *Update (daily during campaign):* movement speed by terrain, road and season. Consumption ≈ 1 ration per man and per animal. Forage = f(local surplus, season). Deficit → attrition 3–10 %/month plus camp epidemic (M5 patch). *Battle:* Helmbold kernel `dA/dt = −β_eff·B·(A/B)^{1−W}` with β_eff scaled by tech, leadership and terrain (defender ×1.0 plains, ×1.3 hills/forest, ×1.6 river/mountain, ×3–10 behind walls). Morale drops with casualty rate and flanking, and the battle ends at breakpoint b ∈ [0.1, 0.35]. The loser takes pursuit losses ×(1.5–3) scaled by the winner's cavalry. *Siege:* garrison food F_g (days); each day F_g −= garrison; starvation surrender when F_g ≤ 0; assault optional through the fort multiplier; besieger disease grows with time. *Couplings:* armies eat regional grain (M4) → famine → unrest (M6); war cost → debt; deaths are young men (M2); armies move pathogens (M5). *Manifest:* W ∈ [0.2, 1.0]; breakpoint ∈ [0.1, 0.35]; campaign season months ∈ [3, 12]; fort multiplier ∈ [2, 10].

**S1: Religion and ideology.** Per-region shares xₖ follow pairwise Abrams–Strogatz competition with status s (state religion, prestige, minus persecution) and a ∈ [1.0, 1.5] [B11][B40], plus inter-region diffusion along M4/S3 flows. Faction opinions follow Deffuant/HK with ε ∈ [0.05, 0.5] [B9][B10]. Axelrod traits (F = 5, q ∈ [3, 30]) provide customs and naming drift [B7]. *Couplings:* faith mismatch → L penalty; minorities are the scapegoat pool in M5/M3 shocks; holy war is a casus belli.

**S2: Technology.** About 20–40 techs in a DAG. Regional adoption: `dF/dt = (p + q·F̄_neighbors)(1 − F)` [B12]. Invention hazard ∝ (connected population)^ψ × stability × literacy, with ψ ∈ [0.5, 1.2] [B13]. Loss: if connected population falls below a threshold, adoption decays [B14]. Effects feed into A, k₀, W, IFR and τ. *Horizon rule:* feudal scenarios 50–1,000 years. Modern-city scenarios run 1–30 years at day/week ticks. Above 50 years the manifest must declare explicit tech and institutional trajectories, and the run is labelled speculative.

**S3: Migration.** Season flows `Mᵢⱼ = μ·Nᵢ·T̂ᵢⱼ^{rad}·σ(uⱼ − uᵢ − cᵢⱼ)`, with utility u = log real wage − hazard − persecution + kin. Flight on famine, war and plague. Urban pull produces the Zipf distribution.

**S4: Ecology.** Wild prey and livestock follow Rosenzweig–MacArthur [B15]. Dragons are individual agents with energy budgets constrained by the Carbone ratio, with a manifest "metabolism multiplier" ∈ [0.1, 10] [B16]. When wild prey is short, a dragon raids livestock, which causes food loss, local Despair and prestige hunts. Forests, fisheries and soil fertility regenerate logistically with harvest, and collapse beyond MSY.

**S5: Story sifter.** Patterns over the event log (e.g., *heir crisis within 5 years of a famine*, *purge of a false positive who was a lord's child*) are scored for salience and handed to the narrator. A Caves-of-Qud-style rationalizer [B35] produces chronicle prose that cites *real* causes from the causal log.

### Drama couplings to implement first

1. **Famine → unrest → succession crisis.** Two bad harvests in a row (M3) → grain ×3–5 (M4) → entitlement collapse → famine deaths among infants and the old (M2), plus migration (S3) → MMP ↑ and tax shortfall → SFD ↑ (M6) → regicide/coup hazard ↑ (M7) → heir is a child (P ≈ 0.3 of no adult son) → regency factions → civil war → armies eat the remaining grain (M8).
2. **Plague → paranoia → witch hunts → labour revolt.** Outbreak (M5) → fear F ↑ → contact ↓ and multiple waves [26] → scapegoat out-group (S1) → purges → L ↓. Survivors face labour scarcity, so wages rise and rents fall, which squeezes elite income → wage-freeze laws → peasant revolt via Granovetter cascade.
3. **Hive-mind detection spiral (modern).** Low prevalence → mostly false positives → detentions of innocents → L ↓, Paranoia ↑ → test avoidance and preference falsification [B6] → covert spread accelerates → when prevalence rises, PPV improves and "the purges were right after all" becomes a political narrative.
4. **Elite overproduction cycle.** Prosperity → elite fertility and upward mobility → E outgrows positions → intra-elite feuds → the "fathers-and-sons" violence wave [4].
5. **Dragon age.** Livestock expansion → enrichment-driven dragon boom → raids → famine pockets → dragon-slaying prestige feeds legitimacy and Hubris.

### Validation harness (M9): stylized-fact tests run on ≥ 32 seeds per manifest

| Test | Target band (pre-modern unless noted) | Source |
|---|---|---|
| Long-run growth | mean r ∈ [−0.1, +0.3] %/yr without tech shocks; pop within [0.3, 5]·K | [15][22] |
| Child mortality to 15 | 35–55 % | [14] |
| Mortality crises | ≥ 1 event of ≥ 10 % loss in 1–2 years per century; plague-class (30–60 %) events allowed | [16][17] |
| Secular cycle | spectral peak 120–450 years in N or W | [3] |
| Violence waves | secondary peak 40–60 years during disintegrative phases (soft) | [4] |
| Ruler death | murders 5–15 per 1,000 ruler-years; violent deaths 15–30 % | [12] |
| Polity lifespans | approximately exponential, mean 100–300 years | [B22] |
| City sizes | top-20 rank-size slope ∈ [−1.3, −0.7] | [B20] |
| War severity | heavy tail, fitted α ∈ [1.3, 1.9] | [B21] |
| Malthusian test | tech shock raises density, not long-run income | [22] |
| Post-plague wages | real wage rises after ≥ 20 % mortality (sign test) | [B] |
| Invariants | conservation of people, grain and coin; no negative compartments | — |

Use Morris/Sobol sensitivity screening to find which manifest parameters actually move these statistics. Freeze the dangerous ones. For fitting, use approximate Bayesian computation or the method of simulated moments over the table [B23].

---

## Risks & unknowns

- **SDT is contested.** PSI shows chaotic parameter sensitivity [9]. Seshat coding has critics [10]. Cycles are a *tendency*, so test for spectral peaks rather than exact periods.
- **My own DFM finding is a quick Euler integration of the simplified form** [6]. Turchin's original *Historical Dynamics* formulation may include mechanisms that sustain cycles. Check against the book before claiming the DFM "doesn't cycle".
- **Calibrating fantasy.** Dragons, magic and hive minds have no ground truth. Calibrate everything else to history and bound the fantastic parameters by energetics.
- **Determinism versus chaos.** Small manifest changes can flip whole histories. That is a feature for replay value and a bug for "the prompt said X". Catalysts should be initial conditions with probability, not guarantees.
- **Tier coupling bugs.** Promotion and demotion can leak population or money. Property tests are needed.
- **LLM manifest risk.** The model may emit parameter combinations that are individually in bounds but explosive together (high TFR + high e₀ + no density dependence). Run a 200-year burn-in gate before showing the user anything.
- **Performance.** Daily epidemics over 1,000 years are fine at the region level. Agent-level contact networks for 1M people are not, so keep them for named and on-screen agents only.
- **Legibility.** Emergent ≠ interesting. Without S5 the Time Machine will feel like noise.
- **Content sensitivity.** Pogroms and witch hunts are emergent outputs, so decide on framing in the narrator.
- **Evidence gaps in this dossier.** Page fetching was blocked by network policy (journals.plos.org, arxiv.org, wikipedia.org and ncbi.nlm.nih.gov were refused) and the search budget ran out. All [B] items, including Lanchester fits, Victoria 3 price caps, the status of the DF economy, the CK3/EU4 mechanics and the opinion-dynamics constants, need a verification pass.

---

## Sources

**Verified this session [V]** (via search-result text; pages not fetched)

1. https://press.princeton.edu/node/20997: Turchin & Nefedov, *Secular Cycles* (Princeton 2009), scope and data series. [V]
2. https://wiki.p2pfoundation.net/Secular_Cycles: summary (quoting Koyama) of the elite-competition mechanism. [V]
3. https://peterturchin.com/wp-content/uploads/2015/04/Turchin_2009_RevSec-1.pdf: Turchin 2009 review; human second-order cycle periods 120–450 years, most likely 200–300. [V]
4. https://slatestarcodex.com/2019/08/12/book-review-secular-cycles/: review; 40–60-year fathers-and-sons cycles, Roman Republic examples. [V]
5. https://escholarship.org/uc/item/0wm663pf: Turchin, "Dynamical feedbacks between population growth and sociopolitical instability in agrarian states" (Structure and Dynamics). [V]
6. https://seslibrary.asu.edu/node/819: simplified demographic-fiscal model equations (N, S, K(S)). [V]
7. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10986950/: Wittmann & Kuehn 2024, PLOS ONE, Demographic-Wealth model, Hopf bifurcations (doi:10.1371/journal.pone.0298318). [V]
8. https://arxiv.org/pdf/1504.04688: Turchin–Korotayev model variant, ρ(N) = c₁(1 − N/k). [V]
9. https://www.arxiv.org/pdf/2405.01163: PSI applied to Poland; Ψ = MMP·EMP·SFD definitions; parameter sensitivity. [V]
10. https://discovery.ucl.ac.uk/id/eprint/10041342/: Turchin et al. 2018 PNAS, Seshat, 414 societies, PC1 ≈ ¾ of variance. [V]
11. https://peterturchin.com/publications/all-crises-are-unhappy-in-their-own-way-the-role-of-societal-instability-in-shaping-the-past: CrisisDB; crisis outcomes variable and uncorrelated. [V]
12. https://www.cam.ac.uk/research/news/killing-kings: Eisner, regicide; ~10 murders per 1,000 ruler-years. [V]
13. https://www.medievalists.net/2011/02/killing-kings-patterns-of-regicide-in-europe-ad-600%e2%80%931800/: Eisner summary; 1,513 monarchs, 45 monarchies, 22 % violent deaths. [V]
14. https://ourworldindata.org/child-mortality-in-the-past: historical child mortality ~50 %; Sweden/France/Bavaria 1750–80. [V]
15. https://en.wikipedia.org/wiki/Demographic_transition: stage-1 crude rates ~30–50 ‰, balanced. [V]
16. https://www.medievalists.net/2023/11/how-many-people-died-black-death/: Black Death estimates (Ziegler ⅓, Aberth ½, Benedictow 60–65 %). [V]
17. https://inquisitivebiologist.com/2022/11/16/book-review-the-complete-history-of-the-black-death-second-edition/: Benedictow 2nd edition, 65 % from ~300 datasets. [V]
18. https://search.r-project.org/CRAN/refmans/demogR/html/cdmltw.html: Coale–Demeny tables in R (demogR). [V]
19. https://u.demog.berkeley.edu/~jrw/Biblio/Eprints/126grad/Coale/Extras/coale.guo.1989_revised.MLTs_Pop.Index.pdf: Coale & Guo 1989 revised model life tables. [V]
20. https://en.wikipedia.org/wiki/Leslie_matrix: Leslie matrix, dominant eigenvalue, stable age distribution. [V]
21. https://faculty.econ.ucdavis.edu/faculty/gclark/Farewell%20to%20Alms/FTA-chapter2.pdf: Clark, *A Farewell to Alms* ch. 2, Malthusian logic. [V]
22. https://www.nber.org/papers/w17037: Ashraf & Galor, "Dynamics and Stagnation in the Malthusian Epoch" (AER 2011). [V]
23. https://arxiv.org/abs/1111.0586: Simini et al., radiation model (Nature 484, 2012). [V]
24. https://news.mit.edu/2012/mobility-migration-model-0227: MIT News on the radiation model; Utah/Alabama comparison. [V]
25. https://www.hu-berlin.de/en/press-portal/nachrichten-en/archive/nr1312/nr_131212_00: Brockmann & Helbing 2013 Science, effective distance. [V]
26. https://www.brookings.edu/articles/coupled-contagion-dynamics-of-fear-and-disease-mathematical-and-computational-explorations: Epstein et al. 2008, fear–disease coupling (doi:10.1371/journal.pone.0003955). [V]
27. https://alnap.org/help-library/resources/the-spread-of-awareness-and-its-impact-on-epidemic-outbreaks/: Funk et al. 2009 PNAS, awareness and epidemics. [V]
28. https://arxiv.org/pdf/1310.2636: Marvel et al., "The small-world effect is a modern phenomenon" (Black Death waves). [V]
29. https://arxiv.org/pdf/2311.02652: review of reaction-diffusion epidemic models, including Noble 1974 Black Death. [V]
30. https://sfu.ca/~poitras/jpe_witches_04.pdf: Oster 2004 JEP, witchcraft and weather, 11 regions 1520–1770. [V]
31. https://cepr.org/voxeu/columns/witch-hunts-western-world-past-and-present: critique of Oster; Leeson & Russ religious-competition alternative. [V]
32. https://metricgate.com/blogs/base-rate-fallacy-diagnostic-screening/: PPV and base-rate explainer (secondary; arithmetic redone here). [V]
33. https://www.statista.com/topics/9347/pre-industrial-demographics: secondary; infant mortality "almost a third", e₀ ≈ 24 in year 1000 (low quality). [V]
34. https://escholarship.org/content/qt230872k1/qt230872k1_noSplash_ec67605c293f139f218aadf237d7912a.pdf: *Cliodynamics* journal article noting the DFM uses "just six adjustable parameters". [V]

**Background literature [B]** (not fetched this session; DOIs from bibliographic memory, so verify)

- B1. https://doi.org/10.1098/rspa.1927.0118: Kermack & McKendrick 1927, SIR foundations.
- B2. https://doi.org/10.1137/S0036144500371907: Hethcote 2000, "The Mathematics of Infectious Diseases".
- B3. https://doi.org/10.1093/biomet/33.3.183: Leslie 1945, matrix population model.
- B4. https://doi.org/10.1086/226707: Granovetter 1978, threshold models of collective behaviour.
- B5. https://doi.org/10.1073/pnas.092080199: Epstein 2002, civil-violence ABM (grievance, legitimacy, arrest risk).
- B6. https://doi.org/10.2307/2010422: Kuran 1991, "Now out of Never", preference falsification.
- B7. https://doi.org/10.1177/0022002797041002001: Axelrod 1997, dissemination of culture.
- B8. https://doi.org/10.1080/01621459.1974.10480137: DeGroot 1974, reaching a consensus.
- B9. https://doi.org/10.1142/S0219525900000078: Deffuant et al. 2000, bounded-confidence mixing.
- B10. https://www.jasss.org/5/3/2.html: Hegselmann & Krause 2002, opinion dynamics and bounded confidence.
- B11. https://doi.org/10.1038/424900a: Abrams & Strogatz 2003, language-death model (a ≈ 1.31).
- B12. https://doi.org/10.1287/mnsc.15.5.215: Bass 1969, diffusion of innovations.
- B13. https://doi.org/10.2307/2118405: Kremer 1993, population and technological change.
- B14. https://doi.org/10.2307/4128416: Henrich 2004, Tasmanian skill loss and demography.
- B15. https://doi.org/10.1126/science.171.3969.385: Rosenzweig 1971, paradox of enrichment.
- B16. https://doi.org/10.1126/science.1067994: Carbone & Gittleman 2002, carnivore density scaling (10,000 kg prey : 90 kg predator).
- B17. https://doi.org/10.1126/science.1235367: Hsiang, Burke & Miguel 2013, climate and conflict.
- B18. https://doi.org/10.1073/pnas.0703073104: Zhang et al. 2007, climate change, war and population decline.
- B19. https://doi.org/10.1038/ngeo2652: Büntgen et al. 2016, Late Antique Little Ice Age.
- B20. https://doi.org/10.1162/003355399556133: Gabaix 1999, Zipf's law for cities.
- B21. https://doi.org/10.1126/sciadv.aao3580: Clauset 2018, severity of interstate wars (power-law tail).
- B22. https://doi.org/10.1080/01615440.2011.577733: Arbesman 2011, life-spans of empires (exponential, mean ≈ 220 years).
- B23. https://doi.org/10.1126/science.1116681: Grimm et al. 2005, pattern-oriented modelling.
- B24. https://doi.org/10.1287/opre.2.2.163: Engel 1954, Lanchester square-law fit to Iwo Jima.
- B25. https://doi.org/10.1017/S000305541400015X: Kokkonen & Sundell 2014, primogeniture and autocratic survival.
- B26. https://doi.org/10.1016/S0378-4371(00)00205-3: Bouchaud & Mézard 2000, wealth condensation.
- B27. https://doi.org/10.1073/pnas.0510525103: Colizza et al. 2006, airline network and epidemic predictability.
- B28. https://doi.org/10.1103/PhysRevLett.86.3200: Pastor-Satorras & Vespignani 2001, epidemics on scale-free networks.
- B29. https://doi.org/10.1103/PhysRevLett.85.3536: Castellano, Marsili & Vespignani 2000, Axelrod-model transition.
- B30. https://github.com/Azgaar/Fantasy-Map-Generator: Azgaar's FMG (states, cultures, religions, provinces).
- B31. https://dwarffortresswiki.org/: DF wiki (worldgen, legends, disabled economy).
- B32. https://ck3.paradoxwikis.com/Succession: CK3 succession laws.
- B33. https://eu4.paradoxwikis.com/Attrition: EU4 supply limit and attrition.
- B34. https://vic3.paradoxwikis.com/: Victoria 3 pops, markets, production methods.
- B35. https://www.cavesofqud.com/: Caves of Qud; see Grinblat & Bucklew 2017 (FDG), "Subverting historical cause & effect".
- B36. https://www.ultimaratioregum.co.uk/: Ultima Ratio Regum (Mark R. Johnson).
- B37. https://songsofsyx.com/: Songs of Syx.
- B38. https://www.superworldbox.com/: WorldBox.
- B39. (no URL) Sen 1981, *Poverty and Famines* (OUP): entitlement approach.
- B40. https://doi.org/10.1103/PhysRevLett.107.088701: Abrams, Yaple & Wiener 2011, decline of religious affiliation.
- B41. (no URL) Engels 1978, *Alexander the Great and the Logistics of the Macedonian Army*; van Creveld 1977, *Supplying War*.
- B42. (no URL) Ryan 2018, *Curating Simulated Storyworlds* (PhD, UC Santa Cruz): story sifting.
- B43. (no URL) Campbell & Ó Gráda 2011, JEH, harvest shortfalls and famines in pre-industrial England.
- B44. (no URL) Scheidel 2017, *The Great Leveler* (Princeton).
- B45. (no URL) Samuelson 1952, "Spatial Price Equilibrium and Linear Programming", AER 42(3).
- B46. (no URL) Turchin 2003, *Historical Dynamics* (Princeton): DFM, asabiya and the Ibn Khaldun cycle.
