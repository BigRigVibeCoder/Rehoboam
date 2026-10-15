# Reading list: papers to pull, verify and assimilate

*Status: v1.2 (2026-10-15). v1.2: prioritised into just-in-time batches; three new pre-modern sources added (Campbell & Ó Gráda 2011, Arbesman 2011, Allen 2001); low-value items dropped.* *v1.1 (2026-10-15). v1.1: received items marked ✓ (extractions in `R12_literature_verification.md`); new Tier A-M for modern-era anchors (ADR-0027).*

*v1.0 note: Why this exists: the research dossiers R01–R11 could not reach arXiv, the ACM DL, AAAI, PLOS, JASSS, NCBI or eScholarship from the sandbox. Many academic claims are therefore tagged [R] or [B] (recalled from prior knowledge, not re-read). R09 alone has 24 [B] items and R04 has 48 [R] items. This list names the papers whose full text would **change a number or a decision in the plan**, not just add a citation.*

**How to hand them over.** Attach the PDFs to a message in this session, in batches (Tier A first). For books, the named chapters are enough. The PDFs stay out of git (`research/papers/` is git-ignored): they are copyrighted, and this container is temporary. What gets committed is the extraction: claims with page numbers, parameter values, data tables usable as calibration targets, and a "delta to the plan" list.

**What comes back.** A verification dossier `research/R12_literature_verification.md`. It updates [R]/[B] tags to [V] or corrects them, and lands concrete changes in:
- the Registry parameter bounds (`specs/manifest/registry_v0.toml`);
- the stylized-fact bands and certification statistics (`docs/09`);
- the sifter semantics and significance model (`05.4`);
- the cognition and persuasion model (`05.3`);
- the LoD and hydration design (`05.2`).

**Priority key.** **A** = needed before P1 (it sets Registry bounds and the certification bands the first engine is judged against). **B** = needed before P2/P3 (sifting, significance, suspense, epistemics, Foresight). **C** = before P4–P6 (long-form generation, LoD details). ✓ = already in hand.

---

## What to send, and when (v1.2 prioritisation)

You have access to everything, so the constraint is not access. It is **integration**: each source has to be read, checked against the plan and turned into bounds, bands or design changes. Numbers for eras we won't build for a year would only sit there and go stale. So pull **just in time**, in batches tied to the phase that needs them. The tier tables below remain the full catalogue.

### Batch 2 — send now (the agrarian thesis: P0–P2)
These set the remaining bands and bounds the first engine is judged against, and the sifter semantics the thesis gate (G2) depends on.

| # | Source | Why now |
|---|---|---|
| 1 | **Turchin & Nefedov (2009), *Secular Cycles*, Ch. 1–2 + one case chapter** (A1) | The secular-cycle band V-SF-04 has no verified source yet. This is the most important item in the batch |
| 2 | Brass (1971) + **Coale & Demeny West tables** (A4) | M2 mortality golden vectors and e₀ bounds |
| 3 | Carbone & Gittleman (2002) (A11) | The energetics validator constant |
| 4 | Milanovic, Lindert & Williamson (2011); Alfani (2021) (A13) | The inequality band |
| 5 | Alfani & Ó Gráda (2017), Introduction (A14); **new:** Campbell, B. M. S. & Ó Gráda, C. (2011). "Harvest shortfalls, grain prices, and famines in preindustrial England." *J. Economic History* 71(4) | Famine frequency (V-SF-03); harvest-to-price coupling in M3/M4 |
| 6 | **New:** Arbesman, S. (2011). "The life-spans of empires." *Historical Methods* 44(3) | V-SF-07 polity lifespans (mean ≈ 220 years) currently rests on memory |
| 7 | **New:** Allen, R. C. (2001). "The great divergence in European wages and prices from the Middle Ages to the First World War." *Explorations in Economic History* 38(4) | Real-wage series for M4 and V-SF-11 (post-plague wages); it also bridges to the industrial era |
| 8 | Grimm et al. (2005); Fagiolo, Moneta & Windrum (2007) (A15) | The certification method itself |
| 9 | **Kreminski et al. (2021), Winnow** (B3) and Felt (2019) (B2) | Winnow may already solve the sifter semantics we designed; read it before P2 specs freeze |
| 10 | Kreminski et al. (2022), "Select the unexpected" (B4) | The significance rarity component |
| 11 | O'Neill & Riedl (2014), Dramatis; Cheong & Young (2015), Suspenser (B6) | The suspense formula behind auto-pause |
| 12 | Behrooz, Swanson & Jhala (2015) (B5) | Retrospective story selection from logs |

### Batch 3 — before P3 (minds, epistemics, war, contagion, Foresight)
A12 ✓ (Bracken, Fricker, Lucas & Turkes received: WWII anchors only) plus **pre-modern war: Helmbold 1965 (*OR* 13:857–859), Helmbold 1969 (RAND P-4199) and 1987 (CAA-TP-87-16) on victory against force ratio and casualties, i.e. breakpoints; Hartley & Helmbold 1995 (*NRL* 42:609–633); Willard 1962; Weiss 1966** · B7 Talk of the Town knowledge propagation · B10 "How Johnny can persuade" · B11 Walton argumentation schemes + ELM · B13 Gneiting, Balabdaoui & Raftery (2007) (PIT; one paper is enough) · B16 Balcan et al. (2009) + Covasim.

### Batch 4 — before P4–P6 (narrative compiler, hydration)
C6 Re3, DOC, Dramatron, Agents' Room · C7 Chakrabarty et al. (2024) + HANNA · C8 MiniCheck + FActScore · C1 Reynolds, Natrajan & Srinivasan (1997) · C3 Sunshine-Hill (2010, 2013) · C11 a fresh scan of long-form preprints.

### Era batches — at the start of each pack
- **E2 early modern:** M8 Tilly · M7 Besley & Persson + Dincecco.
- **E3 industrial:** M3 Lee · M4 Galor & Weil · M5 Allen (2009) + Wrigley (2010) · M6 Acemoglu & Robinson · M11 Comin & Hobijn. Datasets: Maddison, Human Mortality Database.
- **E4 modern:** M1 Archigos · M2 Powell & Thyne + Jones & Olken · M10 Goldstone et al. (2010) · M9 Turchin (2016) · M12 Braumoeller. Datasets: Correlates of War, V-Dem, Archigos.

### Dropped or optional (low marginal value for the plan)
| Item | Why |
|---|---|
| B9 Riedl & Bulitko | The drama policy is decided (ADR-0009) |
| B12 Hegselmann–Krause, Deffuant | Standard models; the equations are well known |
| B14 Glasserman & Yao; B15 Morris, Toni, Schuirmann | Standard methods already implemented in SALib, pyABC and TOST libraries |
| C2 Davis & Hillestad; C4 Brom et al. | C1 and C3 cover the multi-resolution and level-of-detail points |
| C5 RNG papers | The implementations (Random123, rand_chacha) are the reference we need |
| C9 Generative agents; C10 differential dataflow / DBSP | Reference only, or needed only if recursive sifting rules appear |
| A16 ODD protocol | Optional: useful as a documentation template, but no plan decision depends on it |
| B8 Caves of Qud; GDC talks | Optional inspiration |

## Tier A: calibration targets and model forms (before P1)

These matter most. The G2 thesis gate is judged against stylized-fact bands, and the Registry's parameter bounds come from these sources. If the recalled values are wrong, certification will either pass implausible worlds or fail good ones.

| # | Paper | What we extract | Plan item it changes |
|---|---|---|---|
| A1 | Turchin, P. & Nefedov, S. (2009). *Secular Cycles*. Princeton UP. **Ch. 1–2** (theory), plus one case chapter | Secular-cycle phase lengths and amplitudes; the elite-overproduction mechanism; the observed period distribution | M6 model form; the cycle-period band in `09` |
| A2 ✓ | Turchin, P. (2013). "Modeling social pressures toward political instability." *Cliodynamics* 4(2) | The exact PSI (political stress index) formulation: MMP × EMP × SFD, normalisations and lags | M6 `psi_threshold` bounds `[1.0, 3.0]`; the instability hazard |
| A3 ✓ | Goldstone, J. A. (1991). *Revolution and Rebellion in the Early Modern World*. UC Press. **Ch. 1–2** | The original demographic-structural model; which variables drive state breakdown | M6 couplings (population → prices → fiscal → elites) |
| A4 | Brass, W. (1971). "On the scale of mortality." In *Biological Aspects of Demography*; plus Coale & Demeny, *Regional Model Life Tables and Stable Populations* (1983 ed.), **the West tables** | Relational logit: the α/β ranges and a standard table for e0 20–40 | M2 `e0` bounds `[20, 85]`, `eps_*` bounds; the mortality-schedule golden vectors |
| A5 ✓ | Kokkonen, A. & Sundell, A. (2014). "Delivering stability—primogeniture and autocratic survival in European monarchies 1000–1800." *APSR* 108(2) | Ruler-survival and deposition rates by succession law | M7 hazards per `succession_law`; the dynasty-duration band |
| A6 ✓ | Eisner, M. (2011). "Killing kings: patterns of regicide in Europe, AD 600–1800." *British Journal of Criminology* 51(3) | Regicide rate per ruler-year by century | M7 `murder_hazard_base` bounds `[0.003, 0.02]` |
| A7 ✓ | Blaydes, L. & Chaney, E. (2013). "The feudal revolution and Europe's rise." *APSR* 107(1) | Ruler-tenure distributions, Christian vs Muslim polities, 700–1500 | The reign-length band; censoring treatment |
| A8 ✓ | Clauset, A., Shalizi, C. R. & Newman, M. E. J. (2009). "Power-law distributions in empirical data." *SIAM Review* 51(4) | The exact fitting and goodness-of-fit procedure (x_min, KS bootstrap, likelihood-ratio tests vs lognormal) | Certification tail-fit procedure (`09`, ADR-0010) |
| A9 ✓ (Clauset 2018; Cirillo & Taleb still wanted) | Clauset, A. (2018). "Trends and fluctuations in the severity of interstate wars." *Science Advances* 4(2); Cirillo, P. & Taleb, N. N. (2016). "On the statistical properties and tail risk of violent conflicts." *Physica A* 452 | War-severity tail exponents and their uncertainty | The war-size band (M8) |
| A10 ✓ | Cederman, L.-E. (2003). "Modeling the size of wars: from billiard balls to sandpiles." *APSR* 97(1) | How an agent-based model reproduces the war-size power law; which mechanisms were necessary | M8 design check: does our war model *produce* the tail, or only fit it? |
| A11 | Carbone, C. & Gittleman, J. L. (2002). "A common rule for the scaling of carnivore density." *Science* 295 | The prey-to-predator ratio (the plan uses 0.009 kg per kg) and its scatter | The energetics validator; `S4.ecology` bounds |
| A12 | Lanchester/Helmbold fits: Bracken, J. (1995). *Naval Research Logistics* 42(4) (Ardennes); Fricker, R. D. (1998). *NRL* 45(1); Lucas, T. W. & Turkes, T. (2004). *NRL* 51(1) (Kursk and Ardennes); plus any Helmbold attrition-rate report you can find (1960s–1990s) | Fitted exponents and the attrition coefficients that make "Helmbold" defensible; their uncertainty | M8 `helmbold_w` bounds `[0.2, 1.0]`, `breakpoint` |
| A13 | Milanovic, B., Lindert, P. & Williamson, J. (2011). "Pre-industrial inequality." *Economic Journal* 121; Alfani, G. (2021). "Economic inequality in preindustrial times: Europe and beyond." *J. Economic Literature* 59(1) | Gini ranges and the inequality-possibility frontier for agrarian societies | The wealth-inequality band (M4) |
| A14 | Alfani, G. & Ó Gráda, C. (eds.) (2017). *Famine in European History*. CUP. **Introduction** + one chapter | Famine frequency and mortality per century | The famine-frequency band (M3/M4) |
| A15 | Grimm, V. et al. (2005). "Pattern-oriented modeling of agent-based complex systems." *Science* 310; Fagiolo, G., Moneta, A. & Windrum, P. (2007). "A critical guide to empirical validation of agent-based models." *Computational Economics* 30 | Method: multiple weak patterns vs one strong one; validation pitfalls | Certification design (identity vs emergent band tags) |
| A16 | Grimm, V. et al. (2020). "The ODD protocol … second update." *JASSS* 23(2) | A documentation standard for each simulation system | Possibly adopt ODD for the M1–M8 system specs |

## Tier A-M: modern-era anchors (before the first era pack; ADR-0027)

Worlds must be able to move from agrarian to modern conditions in one run (ADR-0027). Each era-sensitive function needs data at **both** ends. Tier A anchors the pre-modern end. These anchor the modern end and the transition between them.

| # | Paper or dataset | What we extract | Plan item it changes |
|---|---|---|---|
| M1 | Goemans, H. E., Gleditsch, K. S. & Chiozza, G. (2009). "Introducing Archigos: a dataset of political leaders." *J. Peace Research* 46(2) **+ the dataset** | Leader tenure and irregular exits (coup, assassination, revolt) from 1875 | The modern end of the M7 ruler hazards; continues Eisner and Blaydes & Chaney |
| M2 | Powell, J. M. & Thyne, C. L. (2011). "Global instances of coups from 1950 to 2010." *J. Peace Research* 48(2); Jones, B. F. & Olken, B. A. (2009). "Hit or miss? The effect of assassinations on institutions and war." *AEJ: Macroeconomics* 1(2) | Coup and assassination base rates; how outcomes depend on institutions | Modern irregular-exit hazards as a function of `exec_constraint` |
| M3 | Lee, R. (2003). "The demographic transition: three centuries of fundamental change." *J. Economic Perspectives* 17(4) | Timing and lags of mortality and fertility decline; e₀ and TFR trajectories | M2 transition mechanism; the transition-shape hindcast band |
| M4 | Galor, O. & Weil, D. N. (2000). "Population, technology, and growth: from Malthusian stagnation to the demographic transition and beyond." *AER* 90(4) | A unified growth model: when productivity growth escapes the Malthusian trap; the quantity–quality switch | M4 growth mechanism; M2 fertility response to literacy |
| M5 | Allen, R. C. (2009). *The British Industrial Revolution in Global Perspective*. CUP. **Ch. 1–2, 6**; Wrigley, E. A. (2010). *Energy and the English Industrial Revolution*. CUP. **Ch. 1–2** | Why industrialisation happened where and when: wages, energy prices, organic vs mineral economies | `energy_per_capita`; the industrial sector in M4; the frontier conditions of the industrial era pack |
| M6 | Acemoglu, D. & Robinson, J. A. (2000). "Why did the West extend the franchise?" *QJE* 115(4) | Franchise extension as a response to the threat of revolution | Endogenous institutional change: reform transitions in M6 |
| M7 | Besley, T. & Persson, T. (2009). "The origins of state capacity." *AER* 99(4); Dincecco, M. (2009). "Fiscal centralization, limited government, and public revenues in Europe, 1650–1913." *J. Economic History* 69(1) | How fiscal capacity grows; how executive constraints and centralisation raise revenue | `fiscal_capacity` and `exec_constraint` dynamics |
| M8 | Tilly, C. (1990). *Coercion, Capital, and European States, AD 990–1990*. Blackwell. **Ch. 1–3** | War-making and state-making; paths to the fiscal-military state | The early-modern era pack; M6–M8 coupling |
| M9 | Turchin, P. (2016). *Ages of Discord*. Beresta Books. **Parts I–II** | Structural-demographic theory applied to the United States, with modern measures of each component | Modern variants of the M6 Ψ components |
| M10 | Goldstone, J. A. et al. (2010). "A global model for forecasting political instability." *AJPS* 54(1) | Which few variables predict modern instability (regime type, infant mortality, neighbours, discrimination) | The modern M6 instability hazard; a band for the modern regime-type effect |
| M11 | Comin, D. & Hobijn, B. (2010). "An exploration of technology diffusion." *AER* 100(5) | Adoption lags across 166 countries and 104 technologies over two centuries | `S2.technology` diffusion parameters across eras |
| M12 | Braumoeller, B. F. (2019). *Only the Dead*. OUP. **Ch. 1–4** | War onset and escalation trends, a counterpoint to Clauset (2018) | Modern war-onset band |
| M13 | **Datasets** (CSV downloads; the hosts are blocked from this sandbox): Maddison Project Database; Human Mortality Database (or UN World Population Prospects); Correlates of War (inter-state and intra-state wars); Archigos; V-Dem or Polity5 | Time series for GDP per head, life tables, wars, leaders and regimes | Era-conditioned bands, hindcast reference curves |

## Tier B: narrative intelligence, epistemics, persuasion, Foresight (before P2/P3)

| # | Paper | What we extract | Plan item it changes |
|---|---|---|---|
| B1 | ✓ Ryan, J. (2018). *Curating Simulated Storyworlds*. PhD thesis, UC Santa Cruz | Already read in full (shipped inside the Viv repo) | — |
| B2 | Kreminski, M., Dickinson, M. & Wardrip-Fruin, N. (2019). "Felt: a simple story sifter." *ICIDS 2019* | Pattern semantics and how sifting was used in play | `05.4` §2 DSL |
| B3 | Kreminski, M. et al. (2021). "Winnow: a domain-specific language for incremental story sifting." *AIIDE 2021* | **The incremental matching semantics and how they handle partial-match explosion.** We chose `selection` / `on_complete` / `dedupe` without reading this; it may already solve the problem differently | `05.4` §2.3 semantics; SIFT-8; the SQL-twin method |
| B4 | Kreminski, M. et al. (2022). "Select the unexpected: a statistical heuristic for story sifting." *ICIDS 2022* | Their rarity/surprise statistic | The significance "rarity" component (`05.4` §3) |
| B5 | Behrooz, M., Swanson, R. & Jhala, A. (2015). "Remember that time? Telling interesting stories from past interactions." *ICIDS 2015* | Retrospective story selection from simulation logs | Retroactive significance; Chronicle selection |
| B6 | O'Neill, B. & Riedl, M. (2014). "Dramatis: a computational model of suspense." *AAAI 2014*; Cheong, Y.-G. & Young, R. M. (2015). "Suspenser." *IEEE TCIAIG* 7(1) | Their suspense formulation, to check against ours (stakes × P(completion)) | `05.4` §2.3 item 8; auto-pause scoring |
| B7 | Ryan, J., Summerville, A., Mateas, M. & Wardrip-Fruin, N. (2015). "Toward characters who observe, tell, misremember, and lie." *EXAG 2015* | Knowledge propagation, rumour mutation, memory decay in Talk of the Town | Channels, fidelity, `variant_id` (`05.3` §5, ADR-0013) |
| B8 | Grinblat, J. & Bucklew, C. B. (2017). "Subverting historical cause & effect: generation of mythic biographies in Caves of Qud." *FDG 2017* | Post-hoc rationalisation of history into legend | In-world documents; unreliable chronicles |
| B9 | Riedl, M. O. & Bulitko, V. (2013). "Interactive narrative: an intelligent systems approach." *AI Magazine* 34(1) | The drama-management taxonomy | ADR-0009 (curation, not intervention) |
| B10 | Zeng, Y. et al. (2024). "How Johnny can persuade LLMs to jailbreak them." *ACL 2024* | The 40-technique persuasion taxonomy we cite for the red-team corpus | ARG-300 adversarial items; `05.6` §6 |
| B11 | Walton, D., Reed, C. & Macagno, F. (2008). *Argumentation Schemes*. CUP. **Ch. 1 + the scheme compendium**; Petty, R. E. & Cacioppo, J. T. (1986). The elaboration likelihood model (*Adv. Exp. Soc. Psych.* 19) | Scheme types and critical questions; central vs peripheral routes | The ArgumentFrame schema and scoring (ADR-0014) |
| B12 | Hegselmann, R. & Krause, U. (2002). *JASSS* 5(3); Deffuant, G. et al. (2000). *Advances in Complex Systems* 3 | Bounded-confidence dynamics and their parameter regimes | Pop-level belief shares; S1.culture |
| B13 | Gneiting, T., Balabdaoui, F. & Raftery, A. E. (2007). "Probabilistic forecasts, calibration and sharpness." *JRSS-B* 69; Gneiting, T. & Raftery, A. E. (2007). *JASA* 102; Hamill, T. M. (2001). "Interpretation of rank histograms." *Monthly Weather Review* 129; Spiegelhalter, D. J. (1986). *Statistics in Medicine* 5 | PIT and rank-histogram tests; proper scores; the Spiegelhalter Z test | Foresight validity tests (ADR-0015, `09`) |
| B14 | Glasserman, P. & Yao, D. D. (1992). "Some guidelines and guarantees for common random numbers." *Management Science* 38(6) | When common random numbers are guaranteed to reduce variance | Intervention A/B in Foresight |
| B15 | Morris, M. D. (1991). *Technometrics* 33(2); Toni, T. et al. (2009). "Approximate Bayesian computation scheme…" *J. R. Soc. Interface* 6; Schuirmann, D. J. (1987). TOST. *J. Pharmacokinetics and Biopharmaceutics* 15 | Screening design sizes; ABC-SMC settings; TOST margins | Deterministic calibration repair; equivalence tests in `09` |
| B16 | Balcan, D. et al. (2009). "Multiscale mobility networks and the spatial spreading of infectious diseases." *PNAS* 106; Kerr, C. C. et al. (2021). "Covasim." *PLOS Comp. Biol.* 17 | Metapopulation coupling; testing with sensitivity, delays and loss to follow-up | M5 model form (we plan to port Starsim/Covasim code) |

## Tier C: long-form generation, LoD, infrastructure (before P4–P6)

| # | Paper | What we extract | Plan item it changes |
|---|---|---|---|
| C1 | Reynolds, P. F., Natrajan, A. & Srinivasan, S. (1997). "Consistency maintenance in multiresolution simulation." *ACM TOMACS* 7(3) | The multi-resolution failure-mode list R01 attributes to them (flagged in R01 as unverified) | Hydration consistency criteria (`05.2` §3.6) |
| C2 | Davis, P. K. & Hillestad, R. (1993). "Families of models that cross levels of resolution." *WSC 1993* | Aggregation/disaggregation consistency | Fidelity switching via migration keyframes |
| C3 | Sunshine-Hill, B. & Badler, N. (2010). "Perceptually realistic behavior through alibi generation." *AIIDE 2010*; Sunshine-Hill, B. (2013). "Managing simulation level-of-detail with the LOD trader." *MIG 2013* | Off-screen agent plausibility; LOD budgeting | The hydration bubble (`05.2` §3) |
| C4 | Brom, C., Šerý, O. & Poch, T. (2007). "Simulation level of detail for virtual humans." *IVA 2007* | LOD for agent AI; what breaks | The T0–T4 ladder |
| C5 | Salmon, J. K. et al. (2011). "Parallel random numbers: as easy as 1, 2, 3." *SC'11*; L'Ecuyer, P. et al. (2017). *Math. Comput. Simul.* 135; Steele, G., Lea, D. & Flood, C. (2014). "Fast splittable pseudorandom number generators." *OOPSLA* | Counter-based RNG guarantees; stream independence | Keyed RNG (`05.1` §6, ADR-0006) |
| C6 | Yang, K. et al. (2022). "Re3." *EMNLP 2022*; Yang, K. et al. (2023). "DOC." *ACL 2023*; Mirowski, P. et al. (2023). "Dramatron." *CHI 2023*; Huot, F. et al. (2025). "Agents' Room." *ICLR 2025* | Planning depth, outline control, multi-agent writing; their measured failure modes | Narrative Compiler stages (`05.4` §6) |
| C7 | Chakrabarty, T. et al. (2024). "Art or artifice? Large language models and the false promise of creativity." *CHI 2024*; Chhun, C. et al. (2022). "Of human criteria and automatic metrics" (HANNA). *COLING 2022* | Honest creative-quality measurement; which automatic metrics track human judgement | NOVEL-20 eval and the "honest bar" (`05.4` §6.3) |
| C8 | Tang, L., Laban, P. & Durrett, G. (2024). "MiniCheck." *EMNLP 2024*; Min, S. et al. (2023). "FActScore." *EMNLP 2023* | Claim decomposition and NLI checking; model sizes and licences | Scene QA; the MiniCheck licence question (`14`) |
| C9 | Park, J. S. et al. (2023). "Generative agents." *UIST 2023* | Reference point only: where LLM-driven agents fail at scale | Why our minds are utility-based (ADR-0012) |
| C10 | McSherry, F. et al. (2013). "Differential dataflow." *CIDR 2013*; Budiu, M. et al. (2023). "DBSP." *VLDB 2023* | Only if recursive sifting rules are needed | The IVM escalation path |
| C11 | Any 2026 preprints on long-form narrative-state tracking (R08 cites NstAgent and an outline-stage comparison, unverified) | Whether anything supersedes our canon-gated pipeline | ADR-0017 |

## Optional: industry talks (if you have GDC Vault access)

Some state of the art here is in talks, not papers:
- Dave Mark and Kevin Dill on Infinite Axis Utility AI (IAUS).
- Tarn Adams on Dwarf Fortress world generation and legends.
- Tynan Sylvester on RimWorld's storyteller.
- Paradox talks on Crusader Kings 3 character AI.

Notes are enough; the talks are not needed in full.

---

**Count.** Tier A: 16 entries (about 22 papers and chapters; 8 received). Tier A-M: 13 entries (about 18 papers and chapters plus 5 datasets). Tier B: 16 entries (about 25). Tier C: 11 entries (about 20). **Tier A alone resolves the largest risk**: certification bands and Registry bounds currently rest on recalled numbers.
