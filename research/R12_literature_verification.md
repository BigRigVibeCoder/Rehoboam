# R12: Literature verification (primary sources supplied by the director)

*Research dossier, 2026-10-15. Full texts were supplied by the director and read in full; PDFs stay outside git. Page numbers are journal pages unless stated. Figure readings are approximate and marked. Effect numbers marked *derived* were computed by us from the paper's own figures or tables. Batch 1: eight Tier-A sources from `reading_list.md`, all extracted.*

---

## TL;DR

1. **The models are generic; the numbers are era-specific, and the papers say why.** Ruler hazards, reign lengths and deposition track **institutions**, not dates:
   - Primogeniture cuts the deposition hazard to 0.25–0.37 of elective succession, with no calendar trend left after controls (Kokkonen & Sundell 2014).
   - Reigns lengthened after a break around 790 CE, with the arrival of feudal institutions. Over the same centuries the Islamic world's deposition rate *rose* (Blaydes & Chaney 2013).
   - Regicide fell from 25 to about 2 per 1,000 ruler-years between the 7th and 18th centuries (Eisner 2011).

   That is the evidence base for ADR-0027 ("eras are state, not templates"). Model the institutions and let the eras follow.
2. **Some facts are era-invariant.** The *shape* of the war-severity tail is stable over 1823–2003, at α̂ = 1.53 ± 0.07 (Clauset 2018), and so is the memoryless onset process. Scale and onset rate come from population, polities and technology.
3. **Our war model must *produce* the tail, not fit it.** In Cederman's model the tail exists only with technological change and war contagion. Without either, α rises to about 2.3. M8 needs an onset hazard with contagion, and severity that emerges from escalation.
4. **The structural-demographic mechanism carries into the modern era.** Turchin (2013) applies the same Ψ to the antebellum and post-1960 United States. The components are dimensionless ratios, which survive a tenfold rise in GDP per head.
5. **Goldstone explains why breakdowns faded after 1850**: fiscal reform, elite birth limitation, industry absorbing labour, trade and fertiliser breaking the local food constraint, power sharing, policing. That is a concrete agenda for the early-modern and industrial era packs. He also says a breakdown needs *all three* strains, while two give a crisis.
6. **Corrections to the plan** (all applied; see the delta table):
   - R09's elite dynamics and EMP formula were wrong.
   - The murder-hazard floor was too high for later centuries.
   - Deposition had no parameter of its own.
   - "Regency raises hazard" is unsupported.
   - The war-tail test used p ≥ 0.1 where the source says p > 0.1.
   - City sizes were to be fitted by OLS, which the method paper rejects.
7. **A testability warning.** Heavy-tail bands need a pooled tail of at least 100 events, and per-seed tests need at least 50 per seed. A 100k-person world may not produce that many wars or cities. The P1b and P3a measurements will tell us; until then V-SF-08 is soft, and V-SF-09 can come out INSUFFICIENT.

---

## Plan deltas applied (this batch)

| Change | Where | Source |
|---|---|---|
| ADR-0027: era-generic mechanisms, endogenous institutions, era packs | `docs/06_adr/ADR-0027`, `05.1` §3.1, `01`, `03` SIM-13/14, `11`, `12` Q19 | All; esp. Eisner, Blaydes & Chaney, Kokkonen & Sundell, Turchin |
| M7: competing exit hazards (murder, deposition, battle, natural); per-law hazard ratios; feud stock; no date term | `05.1` §3, registry M7 params, `11` P1.7 | Eisner; Kokkonen & Sundell; Blaydes & Chaney |
| `murder_hazard_base` [0.003, 0.02] → **[0.001, 0.03]/yr at a reference institutional state**; new `deposition_hazard_ref` [0.01, 0.05], `primogeniture_hr` [0.2, 0.55], `seniority_hr` [1.3, 2.6], `predecessor_killed_or` [1.5, 3.5] | `specs/manifest/registry_v0.toml` | Eisner; Kokkonen & Sundell |
| M6: Ψ per Turchin; threshold on the geometric mean Ψ̄; `elite_mobility_u` → μ₀ in [0.0005, 0.01]/yr; `wage_stickiness_years` [3, 10] | registry M6; R09 M6 correction block | Turchin 2013 |
| M8: onset hazard = force ratio × contagion; no global cooldown; severity emerges; war unit = space-time component; scale from population × fiscal capacity × technology | `05.1` §3, `11` P3.5 | Cederman; Clauset 2018 |
| V-SF-06 split into era-conditioned murder rates (06), deposition by law (06b), reign length by institutions (06c), regicide contagion (06d) | `09` §3.1 | Eisner; Kokkonen & Sundell; Blaydes & Chaney |
| V-SF-08: MLE tail exponent α ∈ [1.8, 2.6], soft while n < 100 | `09` §3.1 | CSN 2009 |
| V-SF-09: deterministic CSN recipe, p > 0.1, per-seed plausibility, cutoff policy, INSUFFICIENT rule; new 09b (memoryless onsets) and 09c (ablation: the tail is produced) | `09` §3.1 | CSN 2009; Clauset 2018; Cederman |
| V-SF-05 limited to internal violence; new V-SF-05b lead–lag (soft) | `09` §3.1 | Clauset 2018; Turchin 2013 |
| New V-ERA suite (era transitions) | `09` §3.6 | ADR-0027 |
| M6 outcome gating by the number of elevated components (3 → breakdown, 2 → crisis/reform, 1 → none); fiscal distress per Goldstone; Goldstone's additive MMP supported as a variant | `05.1` §3 | Goldstone 1991 |
| Era packs E2/E3 scoped by Goldstone's "why breakdowns faded" mechanisms | `11` era-pack track | Goldstone 1991 |
| R09 annotated where primary sources disagree | `research/R09…` | All |

**Resolved in batch 2 (§12):** the 120–450-year band is not in *Secular Cycles* either. V-SF-04 is rewritten from the book's 12 observed cycles. The regency effect needs another source. The modern ends of every era-sensitive function wait for Tier A-M (Archigos and others).

---

## 1. Eisner, M. (2011). "Killing kings: patterns of regicide in Europe, AD 600–1800." *British Journal of Criminology* 51, 556–577. doi:10.1093/bjc/azr004

**Data.** 1,513 monarchs (40 female) in 45 monarchies and 6 regions, AD 600–1800. Sources are Wikipedia ruler lists, cross-checked against Morby, Vauchez and national biographical dictionaries (pp. 560–561). Deaths are coded as accident, battle (including civil war), execution after trial, or murder. Murder means intentional killing away from the battlefield and outside any judicial process, including of deposed rulers; it is split into certain and doubtful. Exposure is years as ruler. Claimants, counter-monarchs and regents are excluded.

**Rates (Table 2, p. 563; 21,843 ruler-years).**

| Cause | N | % of deaths | per 1,000 ruler-years |
|---|---|---|---|
| Murder (all) | 219 | 14.6 | 10.03 |
| – certain | 159 | 10.6 | 7.28 |
| Battle | 90 | 5.9 | 4.12 |
| Executed after trial | 8 | 0.5 | 0.37 |
| Any violent | 338 | 22.4 | 15.47 |

**By century (Figs 1–2, pp. 568–569; per 1,000 ruler-years; murders all / certain / battle).**

| 600s | 700s | 800s | 900s | 1000s | 1100s | 1200s | 1300s | 1400s | 1500s | 1600s | 1700s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 25.0 / 23.0 / 9.6 | 17.4 / 14.3 / 6.4 | 9.5 / 5.8 / 6.5 | 12.8 / 7.7 / 7.0 | 17.8 / 11.4 / 11.4 | 10.3 / 9.0 / 3.9 | 8.5 / 6.8 / 3.0 | 11.8 / 6.9 / 0.8 | 5.2 / 2.8 / 1.4 | 2.6 / 1.1 / 1.6 | 4.1 / 4.1 / 0.7 | 1.9 / 1.9 / 0.6 |

Counts in the late centuries are tiny (about 1,500 ruler-years per century; about 3 murders in the 1700s).

**Other findings:**
- **Perpetrators.** Most killings were "succession by murder"; an alleged participant later became monarch in over 40% of cases. The usual killers were nobles. Outsiders account for only 10–15 cases in 1,200 years (pp. 564–566, 571).
- **Age.** Murder victims acceded 4.7 years younger and reigned 6.1 years less than rulers who died naturally (Table 3, p. 567).
- **Contagion.** A murdered predecessor gives an odds ratio of 2.98 for the next ruler's murder; the one before that, 2.4 (p. 570).
- **Trend.** Mean reign rose from 8.2 to 20.5 years and mean accession age from 25.1 to 39.1 (p. 562). Eisner offers explanations for the decline but tests none: the civilising process, sacral kingship, chivalry, codified primogeniture, target hardening (pp. 568, 571–572).

**Era-dependence.**
- *Invariant:* elite coalitions kill for the throne; killings cluster across successive reigns; young rulers face extra risk.
- *Changes:* the mix of perpetrators. Radicalised outsiders appear only in the modern era (pp. 565, 571), and rulers stop dying in battle.
- *No modern data.*

**Verification of repo claims:**
- R09 pooled rates: **confirmed**.
- "22.4% of reigns": **corrected**; it is 22.4% of *deaths*.
- h₀ bounds [0.003, 0.02]: **corrected**, since the century rates span 0.0019–0.025.
- "Civilising trend" flag: **not in the paper**.
- V-SF-06 at 5–15 per 1,000: **confirmed for pooled data only**.
- 05.1's "fiscal stress raises the regicide hazard": **not in the paper**.

**Caveats.**
- The counts do not fully reconcile: Table 1 sums to 1,505, and the appendix lists about 215 murders.
- Popes and Doges, 305 rulers with low risk, pull the pooled rate down.
- Europe only; nothing after 1800.

## 2. Kokkonen, A. & Sundell, A. (2014). "Delivering stability — primogeniture and autocratic survival in European monarchies 1000–1800." *APSR* 108(2), 438–453. doi:10.1017/S000305541400015X

**Data and method.**
- 961 monarchs in 42 states, 1000–1800.
- Cox survival model per reign, with frailty by country, and stratified by country to use within-country variation only.
- *Deposition* means murder by domestic actors, forced abdication, or death in civil war. Natural death, voluntary abdication, foreign deposition and revolution are censored (pp. 442–445).
- Succession law has three categories. Election is the reference and includes appointment and tanistry (p. 440). The other two are agnatic seniority and primogeniture, which counts only after one clean turnover (p. 443).
- The online appendix, which has the adoption dates, was not available.

**Table 1 (p. 445)** with derived rates:

| | Primogeniture | Election | Agnatic seniority |
|---|---|---|---|
| Monarchs | 451 | 386 | 124 |
| % deposed (authors / Morby) | 16 / 8 | 49 / 21 | 57 / 43 |
| Mean tenure (years) | 21.6 | 12.4 | 9.1 |
| *Depositions per ruler-year (derived)* | *0.0074* | *0.040* | *0.063* |

**Hazard ratios (Cox, reference = election; CIs derived from t).**
- Primogeniture:
  - 0.25 [0.17, 0.37] (Model 1a, p. 447);
  - 0.37 with controls;
  - 0.34 [0.18, 0.63] stratified by country (p. 450).
- Agnatic seniority: 1.98 [1.17, 3.35]; 2.57 stratified.
- Other terms: predecessor deposed 1.56–1.80; foreign depositions per century 1.39; parliament 0.50 between countries but not significant within them; age at accession 1.02 per year (younger is *safer*).
- **No time trend remains after controls** (p. 446).
- In the 12 countries that switched law, deposition risk was 66–79% lower under primogeniture (p. 446).
- Smoothed hazards (Fig. 2, p. 449): primogeniture about 0.0095 falling to 0.005 per year; election 0.026 to 0.014. All rise again after about 40 years in office.

**Mechanism and era.**
- *Mechanism:* the elites' coordination problem and the crown-prince problem; the latter is "a graver threat" (p. 451). The same pattern appears in modern Gulf monarchies (Table 5, p. 452).
- *Adoption:* primogeniture spread from the 12th century. It was slow because elites wanted adult war leaders, and it raised foreign-conquest risk at first; it paid off only after 200–300 years (p. 448).
- *Survival:* 10 of 19 states that always used primogeniture survived to 1800, against 0 of 11 that always used election or seniority (Table 4, p. 451).

**Verification of repo claims:**
- R09's primogeniture effect: **confirmed**.
- Regency raises hazard: **not in the paper**; younger accession is safer, and regents are excluded.
- The murder-hazard floor: **corrected**.
- Partition and sortition: **not estimated**.

## 3. Blaydes, L. & Chaney, E. (2013). "The feudal revolution and Europe's rise." *APSR* 107(1), 16–34. doi:10.1017/S0003055412000561

**Data.**
- The BM dataset (Bosworth plus Morby): N = 3,047 rulers.
- The Nüssli/Euratlas dataset: N = 1,785.
- Both cover rulers taking power in 700–1500.
- The regressions are OLS on entry-century dummies by region. There is no survival model and no treatment of censoring.

**Mean reign by entry century (BM dataset, Table 1, p. 20; Islam / Western Europe):** 700s 11.5 / 10.7; 900s 13.9 / 18.6; 1100s 14.4 / 20.2; 1300s 11.3 / 21.6; 1400s 11.1 / 19.9. The gap is zero over 700–900 and significant over 1000–1400; with geographic controls it is 10–12 years over 1100–1500.

**Structural break.** A Quandt likelihood-ratio test puts the break in the Western series at **790 CE**: reigns go from about 10 years to about 17, rising to about 21 (pp. 25–26).

**Deposition (Fig. 2, read from the figure).** Western Europe about 0.20 in the 9th century, falling to 0.05–0.12 by 1100–1500. Islamic polities 0.2–0.4, rising.

**Mechanism.**
- **Western Europe:** low fiscal capacity, so the military came from landed elites. That brought constraints on the ruler, then parliaments, then longer reigns.
- **Islamic polities:** high fiscal capacity, so rulers could buy slave soldiers (mamluks) and bypass local elites.
- **Supporting results:**
  - A parliament meeting is associated with +5 to +15 years of reign.
  - Islamic reign length falls with position in the dynasty; Western reigns do not.
- **Ruled out:** religion, life expectancy and father-to-son succession (pp. 28–31).

**Era-dependence.**
- *Changes:* tenure follows institutions — executive constraint, where the military is recruited, fiscal capacity — not the calendar.
- *Beyond 1500:* the authors *speculate* that the relationship is non-monotonic, with modern "hyper-institutionalised" democracies having short tenures (p. 33, fn 45). There is no data for this.

**Verification of repo claims:**
- R09's "post-feudal" lengthening: **corrected** to *feudal-era*, with the break in 790.
- "Reign-length band with censoring": **corrected**; the paper uses cohort means only.
- The "civilising trend" flag: **contradicted**, because deposition fell in one region and rose in another over the same centuries.

## 4. Turchin, P. (2013). "Modeling social pressures toward political instability." *Cliodynamics* 4(2), 241–280. doi:10.21237/C7clio4221333

**Model (pp. 246–247).** Ψ = MMP × EMP × SFD. The concept comes from Goldstone; the functional forms are Turchin's.
- **MMP** = w⁻¹ · (N_urb/N) · A₂₀₋₂₉, where w is the wage relative to GDP per head.
- **EMP** = ε⁻¹ · E/(s·N), where ε is elite income relative to GDP per head and s is offices per head.
- **SFD** = (debt/GDP) · distrust. Equation 14 misprints this as Y/N.
- **Normalisation is relative only** (p. 247). The paper gives **no absolute scale, no threshold and no Ψ → hazard function**.

**Submodels (pp. 248–252).**
- **Wages:** W(t+τ) = a·(G/N)^α·(D/S)^β·C^γ, with stickiness τ of 3–10 years.
- **Elite numbers:** dE/dt = r·E + μ₀·(w₀/w − 1)·N.
- **Elite income:** ε = (1 − w·λ)/e, with λ ≈ 0.5.
- **Antebellum parameters:** μ₀ = 0.002/yr, r_rur 0.025, r_urb 0.015, K = 3.5M (pp. 252–259).

**Results.**
- **Antebellum Northeast US:** Ψ bottoms out around 1830 and "explodes" in the 1850s (p. 259). It is offered as a leading indicator of small-scale violence, which in turn leads civil war (p. 260).
- **US 1927–2012:**
  - Wage regression R² = 0.98 with GDP, labour demand/supply and the minimum wage (Table 1, p. 269).
  - Relative elite numbers roughly triple after 1980; debt/GDP goes from about 33% to about 100%; distrust from about 25% to about 80%.
  - Ψ is about 1–2 before 1980 and about 37 by 2012 (read from the figure).
- **Lead–lag:** the relative wage falls about 20 years before relative elite numbers rise, in both centuries (p. 277).

**Era-dependence.**
- *Invariant:* the causal chain, from labour oversupply to falling relative wages, elite overproduction, intra-elite competition and instability (pp. 276–277). The components are dimensionless ratios (p. 248).
- *Era-specific:*
  - the source of labour oversupply;
  - how wages are set (market, institutional, command or slave);
  - labour demand (land vs productivity);
  - elite demography (no fertility premium in modern times, p. 251);
  - the size of the state.

**Verification of repo claims:**
- Ψ product, MMP and SFD: **confirmed**.
- EMP form, elite dynamics and ε normalisation: **corrected**.
- "Ψ = 1 at founding", the threshold, the dW/dt rule and the coup hazard: **not in the paper**.
- The 120–450-year cycle: **not in this paper**; it waits for *Secular Cycles*.

**Design consequence.**
- Compute log Ψ in fixed point, with each component a ratio to a reference value that is re-baselined only by a journaled era-transition event.
- Apply `psi_threshold` to Ψ̄ = Ψ^{1/k}. On that scale [1, 3] is defensible: the US was at about 1.4 in 1990, 1.9 in 2000 and 3.3 in 2012 (readings).
- Use a smooth hazard above the threshold rather than a step.

## 5. Cederman, L.-E. (2003). "Modeling the size of wars: from billiard balls to sandpiles." *APSR* 97(1), 135–150

**Empirical fits.** These are OLS on the log cumulative distribution, so α = 1 − D is our conversion and not directly comparable with MLE:
- COW interstate wars 1820–1997 (N = 97): α ≈ 1.41.
- Levy great-power wars 1495–1965: α ≈ 1.57.
- Ancient China: steeper, on incomplete data (p. 136).

**Model (GeoSim).**
- 200 states on a 50×50 grid; provincial extraction falls with distance from the capital.
- Unprovoked attacks are logistic in the local power ratio (threshold 3:1).
- **Context activation:** a state that is fighting, or has a neighbour fighting, attacks far more readily.
- Conquest proceeds province by province; taking the capital collapses the state.
- **Technology raises power-projection range over time.**
- A war is a space-time cluster of fighting states.

**Table 1 (p. 143; 15 seeds per configuration):**

| Configuration | α | Range of the tail |
|---|---|---|
| Base | 1.55 (1.49–1.64) | 4.2 decades |
| **No technology shocks** | **≈ 2.32** | 1.4 decades |
| **No context activation** | **≈ 2.34** | 1.6 decades |

**Both technological change and contagion are necessary.**

**Era.** The rules are era-invariant; era enters only through technology. The tail emerges as the system consolidates (200 → about 35 states) and is pooled over the whole run. Democratic security communities "transcend" geopolitics (p. 146). Alliances and nationalism are omitted.

**Verification of repo claims:**
- The band [1.3, 1.9]: **consistent**, and it excludes the failed ablations.
- V-SF-09's "pre-modern" scope: **corrected**; the evidence covers 1495–1997.
- M8 had no onset or contagion mechanism: a **gap**, now filled.

**Caveats.** The time unit is abstract, so only the exponent transfers. The author concedes the model is not fully parameter-insensitive (p. 145).

## 6. Clauset, A. (2018). "Trends and fluctuations in the severity of interstate wars." *Science Advances* 4(2), eaao3580. doi:10.1126/sciadv.aao3580

The paper is CC BY-NC: its numbers may be cited, but its figures and text must not be shipped.

**Data.** CoW interstate wars v4.0: 95 wars, 1823–2003. Severity is absolute battle deaths, with a minimum of 1,000 and a maximum of 16.6M for WWII.

**Tail.** α̂ = **1.53 ± 0.07** for x ≥ 7,061. Bootstrap 95% CI **[1.37, 1.76]**; p_KS = 0.78. The tail is the 51 largest wars (pp. 2–4).

**Onsets.**
- One every 1.91 years on average.
- Gaps are geometric (q̂ = 0.43, p = 0.13).
- 79% of wars end within 2 years (p. 4).

**Stationarity.**
- The severity and gap distributions for 1940–2003 cannot be told apart from 1823–1939.
- The "long peace" is not yet significant: it needs another 100–140 years (pp. 4–6).
- P(a war of WWII size or larger within 100 years) = 0.43 (p. 6).

**Era-dependence.**
- *Invariant:* the tail shape α, and memoryless onsets.
- *Not invariant:*
  - absolute scale, which depends on population;
  - per-capita violence, which falls as population grows;
  - very-long-run behaviour, since no 10⁹-death war has ever happened. "Changes in world population, technology, and political structures have likely all played some role" (p. 7).
- *Scope:* no evidence before 1823.

**Verification of repo claims:**
- R09's α ≈ 1.5: **confirmed**.
- The band [1.3, 1.9]: a **Rehoboam widening** of the paper's CI; now labelled as such.
- Pooled plausibility: **corrected** to also run per seed.
- Missing so far: bands for onset rate, memoryless gaps and duration. 09b was added; duration stays soft.

**Testability.**
- SE(α̂) ≈ (α − 1)/√n_tail.
- At least 50 tail wars per seed means at least about 100 interstate wars per 1,000 years.
- At least 500 tail wars across the ensemble gives an SE of 0.025 or less.

## 7. Clauset, A., Shalizi, C. R. & Newman, M. E. J. (2009). "Power-law distributions in empirical data." *SIAM Review* 51(4), 661–703. doi:10.1137/070710111

**Recipe (Box 1, p. 663):**
1. **Fit the exponent.**
   - Continuous MLE: α̂ = 1 + n/Σ ln(xᵢ/x_min), SE = (α̂ − 1)/√n (eqs. 3.1–3.2).
   - Discrete MLE via the Hurwitz zeta (eqs. 3.4–3.6).
   - The approximation α̂ ≈ 1 + n/Σ ln(xᵢ/(x_min − ½)) is good once x_min ≳ 6 (p. 667).
2. **Choose x_min** to minimise the KS distance (p. 672).
3. **Goodness of fit** by semi-parametric bootstrap with a full refit of each synthetic set, using at least ¼ε⁻² replicates: 2,500 for ε = 0.01 (p. 677).
4. **Rule:** "ruled out if p ≤ 0.1" (p. 677).
5. **Alternatives.**
   - Non-nested likelihood-ratio tests (normalised R, p = erfc(·)): trust the sign only if p < 0.1 (pp. 680, 697).
   - The cutoff comparison is nested, so it uses Wilks' χ² test (p. 698).

**Sample size.**
- "n ≳ 50" for a reliable α̂ (p. 669).
- At n ≲ 100, lognormal and exponential samples also pass (p. 678).
- Telling power law from lognormal is "extremely difficult" (p. 689).

**Reference values (Table 3, p. 684):**
- War intensity (deaths per 10⁴ population, 1816–1980): n = 115, α = 1.7 ± 0.2, p = 0.20. The exponential is rejected; lognormal and cutoff fits are undecided.
- US cities: α = 2.37 ± 0.08, p = 0.76.
- Net worth: p = 0.00, so no power law.

**Applied to the plan:**
- **V-SF-09** now specifies the exact deterministic recipe:
  - sort keys (value, seed, event_id);
  - RNG keyed by (world, band, purpose, replicate, draw);
  - pass iff p > 0.1, unrounded;
  - the exponential must be rejected;
  - pooled n_tail < 100 is INSUFFICIENT.
- **V-SF-08** moves from an OLS rank-size slope (biased) to an MLE estimate.
- **Wealth** cannot be tested: the ledger stores it as a binned band, and CSN reject a pure power law for top wealth anyway.

**Testability at 100k persons × 1,000 years:**
- Wars are probably testable when pooled.
- Cities are marginal, so V-SF-08 is soft.
- Epidemics are testable only with at least 10 large outbreaks per seed, and an upper cutoff is expected.
- Dynasty and polity sizes are probably INSUFFICIENT.

## 8. Goldstone, J. A. (1991). *Revolution and Rebellion in the Early Modern World*. UC Press

Page numbers are print pages. Chapters: 1 model (pp. 1–62); 2 England (63–169); 3 France and the 19th-century wave (170–348); 4 Ottoman and Ming–Qing (349–415); 5 ideology (416–458); 6 the present (459–497).

**The causal chain (Ch. 1, pp. 24–27; Fig. 3, p. 142).** Mortality falls for exogenous reasons; population rises; grain prices rise. Three strains follow:
1. **State fiscal distress.** Taxes fixed in nominal terms lag prices, while real costs rise.
2. **Elite mobility and competition.** This means turnover and displacement. Absorbing newcomers is harmless (pp. 109–122).
3. **Mass mobilisation potential.** Driven by falling real wages, cities outgrowing their administration, and the youth share (pp. 134–140).

Population also acts *directly* on the elites (more surviving younger sons) and on mobilisation potential. Heterodox ideology follows the three strains; it does not cause them (p. xxiv).

**The index (pp. 105, 139, 142–144).** Ψ = fiscal distress × elite mobility/competition × mobilisation potential.
- **Fiscal distress.** A five-step scale running from adequate funding to total credit failure (p. 105).
  - It is driven by ∏ e^{k(Xᵢ − B)}, where X is decadal inflation and B is the revenue growth possible without new taxes.
  - Fitted over 1500–1750: B = 0.5–0.7 %/yr, k = 0.25–0.75, R² ≈ 0.70 (Table 4, p. 108).
- **Elite term.** Measured by enrolments.
- **Mobilisation potential.** It is **additive**: W̄/W + (W̄/W − 1)·Urban·Age (p. 139), with Age = population aged 10–29 ÷ population aged 30+. With high wages, urbanisation and youth *reduce* it.

**Gating (p. 469).** A sustained rise in all three strains goes with state breakdown. A rise in two can produce a state crisis. A rise in one alone rarely ends a regime.

**Lags.** Pressure builds over several generations of 35–100 % population growth (p. 405). Triggers only release it (pp. 35–36). Ψ measures "the strain on institutions", not raw ecological pressure (p. 150).

**Numbers.**
- **England 1500–1650.**
  - Population about 2M to over 5M; grain prices +600 %; real wages down to about 50 %.
  - Elasticities: prices 2.24 and real wages −1.78 with respect to population (pp. 87–91).
  - Gentry families roughly tripled; enrolments rose 400 % (pp. 113–124).
  - Subsidy assessments fell to 3 % of their real value (p. 98).
  - Ψ peaked around 2.4 in the 1630s (Fig. 5).
- **France 1700–90.**
  - Population 21.5M to 28.1M; price elasticity 1.94, wage elasticity −0.86.
  - Ψ index about 1 in the 1680s, about 11 in the 1780s (Fig. 8; read from the figure).
- **Qing 1770–1850.** Population 270M to 400M against 25 % more farmland. Top degrees grew 33 % and posts 13 % while population doubled (pp. 391–397).
- **Europe overall.** No crises in the low-growth period 1650–1750 (Fig. 14).

**Why state breakdowns faded in 19th-century Western Europe (Ch. 3 §C pp. 288–289; pp. 26, 315–338, 460–461).** This is the agenda for the early-modern and industrial era packs:
1. **Fiscal.** States moved to excise taxes and funded debt; English debt was refinanced at 3 % (pp. 319–320). Crises then came without bankruptcy.
2. **Elites.** Families limited their births, so there was displacement but little turnover. The civil service expanded: +250 % in Britain over 1830–70 (pp. 289, 314, 329).
3. **Economy.** Industry absorbed surplus labour. Railways, steamships, American grain and fertiliser ended the local food constraint (pp. 26, 302, 315).
4. **Institutions.** Reforms that shared power, and better policing (pp. 292, 316–318).
5. **The exceptions.** Russia, China and the Ottoman Empire lacked this flexibility and broke down in the early 1900s (p. 461).
6. **Modern states.** They can still raise Ψ by policy: too much debt, favouring cities, more graduates than posts, shutting out rising groups (pp. 472–473).

**Ideology (Ch. 5).**
- The causes are material; Ψ has no ideological term (pp. 27, 282).
- After a breakdown the sequence is: networks build the dominant coalition; programmes move from rectification to redistribution to nationalism; radicalisation follows the threat of counter-revolution; then terror and war; then populist military rule within 10–20 years (pp. 421–436, 478).
- A revolutionary rebuild needs marginal elites shut out on principle *and* a culture with an "eschatological" view of time. Otherwise the outcome is a conservative restoration (pp. 444–450).
- Restorations that enforced orthodoxy stagnated (pp. 450–458).

**Era-dependence.**
- *Invariant:* the rule that three strains must coincide; nonlinear effects on marginal groups (doubling population multiplies the marginal group 6.5–11 times, pp. 33–34); booms in credential-seeking; urban growth against administrative capacity; the sequence after breakdown.
- *Era-specific:* the driver itself — rigid nominal taxes, land-based status, local grain markets.
- *Institutions set the direction:* Japan's rice-denominated taxes made even a stable population ruinous (pp. 404–415).

**Verification of repo claims:**
- R09's "EMP" and "debt × distrust" are **Turchin's** terms; Goldstone's middle term is elite mobility/competition, and his fiscal term is the five-step scale.
- R09's "MMP = (w*/w)·urb·youth" is Turchin's multiplicative form; **Goldstone's is additive**.
- R09's "instability resets the system": **not in source**. In Goldstone, exogenous disease and climate end the growth phases (pp. 25–30).
- `psi_threshold`: **not in source**.
- 05.1's Granovetter thresholds: **confirmed** (p. 138, n. 10).
- The W overshoot: **not in source**.
- ADR-0027's "modern application": **confirmed** (pp. 469–475).

**Design consequences (applied).**
1. **Count the elevated components.** M6 turns Ψ into outcomes by how many components are elevated relative to their baselines: three gives a breakdown hazard, two gives a crisis or reform window, one gives nothing.
2. **Fiscal distress per Goldstone.** It follows exp(k·Σ(π − B)), with B a function of `fiscal_capacity` and the share of indirect taxes. Each step on the five-step scale is emitted as a Chronicle event.
3. **Support both mortality regimes.** Goldstone's exogenous mortality phases and Turchin's endogenous ones should both be possible.
4. **Era packs E2 and E3** implement the six "why breakdowns faded" mechanisms above as functions of era state:
   - fiscal instruments that raise B;
   - trade and fertiliser technology that decouples local prices from local population;
   - elite fertility limitation;
   - growth in offices and policing;
   - power-sharing reforms;
   - industrial absorption of labour.
5. **S1.culture.** It carries a time-orientation trait and an orthodoxy level that suppresses invention; a post-breakdown phase machine drives the revolutionary sequence.

**Caveats.**
- The data are contested (pp. 343–345), and the regressions use few decades.
- Ψ is normalised differently in each case study.
- The Asian cases have no mathematical model.

---

# Batch 2 (sources filed in `research/papers/`)

## 9. Brass, W. (1971). "On the scale of mortality." In W. Brass (ed.), *Biological Aspects of Demography*, pp. 69–110. Taylor & Francis

Page numbers are book pages; the PDF page is the book page + 12. We also used Barrett, "A Monte Carlo simulation of reproduction" (pp. 11–30), from the same volume.

**Model.**
- **The logit:** Y(x) = ½·ln((1 − l(x))/l(x)) (p. 74).
- **The relational form:** Y(x) = α + β·Ys(x) (p. 77).
- **α is "level".** Ys = 0 at age 51, so Y(51) = α.
- **β is "slope", not "shape"** (p. 78). It sets child mortality against adult mortality. For example, l(5) can be about the same, 93.8% vs 94.9%, while l(70) differs: 78.6% vs 29.7%.
- **Fitting:** split ages 5–80 into two groups and solve (pp. 84–85). We re-ran this and recovered Brass's UN fits to ±0.007 for α and ±0.017 for β.

**The General Standard** (Table 4, p. 77; e₀ = 43.44) is now a machine-readable golden vector: `specs/demography/brass_general_standard.toml`. The self-test `specs/demography/selftest_brass.py` checks:
- the standard's logits, to within 0.0006;
- the Table 19 schedules, to within 1 per 10,000;
- e₀, to within 0.06 years.

**Observed ranges** (Tables 7, 12, 14A/B): α from −1.36 to 0.55, β from 0.64 to 1.64. The Coale–Demeny West schedules fit the standard closely: α = 0.55 / −0.04 / −0.89 for high, medium and low mortality (p. 90).

**Weak at both ends.**
- Age 1 is off by up to 5 points (p. 86, endogenous neonatal deaths), and old age is also weak (p. 95).
- l(100) = 0 for every α, so modern eras need an old-age tail, such as Beard's Perks logistic (p. 59).
- Ys(0) = −∞, so the first year needs its own sub-schedule.

**Dynamics (p. 108–110).**
- Long-run change happens mostly in α. In Sweden and England & Wales, α moved about 1 unit in 80 years.
- Moving α from −1 to −2 adds only 14 years of e₀, against 27 years for 0 to −1.

**Fertility.** The volume has no Brass relational fertility model. Barrett gives a per-woman Monte Carlo instead:
- fecundability 0.2 per month;
- foetal loss rising with age;
- postpartum non-susceptibility averaging 11 months;
- sterility onset at 20 + U/0.01, and menopause averaging 47.6;
- the result: 10.6–12.8 live births for women married at 20 with no widowhood.

These are the micro-scale fertility mechanics for `uz_job` (ADR-0028).

**Verification of repo claims:**
- The R09 formula: **confirmed**.
- "Brass logit on Coale–Demeny West": **corrected** to Brass's own General Standard, which West fits closely. The Coale & Demeny tables are cited separately.
- e₀ ∈ [20, 85]: **qualified**. The range needs α from −2.04 to 0.74, and both ends are extrapolation.
- **A real tension.** V-SF-02's q(15) of 35–55% implies e₀ of 23.4–36.6 at β = 1, so the agrarian e₀ range of [20, 45] cannot reach 45 unless β < 1.

**Deltas applied:**
- M2 uses the fixed-point odds form, with α as a function of era state.
- New parameters `brass_alpha` and `brass_beta`.
- V-SF-02 is restated in Brass terms, with an INSUFFICIENT threshold of about 400 births.
- A new V-SF-02b pins β.
- The golden-vector spec and self-test are added.

## 10. Carbone, C. & Gittleman, J. L. (2002). "A common rule for the scaling of carnivore density." *Science* 295, 2273–2276. doi:10.1126/science.1067994

**Data.** 25 species of Carnivora from 0.14 kg to 310 kg. "Prey" is the standing biomass of the main prey species (≥ 70% of the diet) per 100 km² (Table 1, p. 2274).

**Rule.** "10,000 kilograms of prey supports about 90 kilograms of a given species of carnivore, irrespective of body mass" (p. 2273).
- The fit is 89.1 × M^−1.05 carnivores per 10,000 kg of prey (N = 25, R² = 0.83). The 95% CI is −0.845 to −1.25 for the exponent and 47–169 for the constant.
- Against prey *productivity*, the exponent is −0.66, which is not significantly different from −0.75.
- The energetic model (note 10) is N_max = a·R/M^0.75. Populations are sustained by productivity, and the −1 exponent on standing biomass follows because the productivity-to-biomass ratio scales as M^−0.25.

**Our refit of Table 1 (derived):**
- Species ratios run from 0.0009 to 0.048, with a median of 0.0069.
- The 1σ scatter is about ×2.65.
- At 5 t the central estimate is about 0.0058, with a 95% prediction interval of 0.0005–0.065.

**Verdicts:**
- `CARBONE_RATIO = 0.009` correctly reads the headline, but it is a central tendency, not a ceiling.
- Dividing by a metabolism multiplier is consistent with note 10, provided the multiplier means field metabolic rate relative to a mammalian carnivore of equal mass.
- The [0.1, 10] bounds are **not in the source**.
- R09's "a 5-t dragon needs about 550 t of prey" is **corrected** to about 860 t with the fitted exponent (CI of the mean 230–3,100 t).

**Deltas applied.** The validator comment now states the scope. A warning is raised above 310 kg (extrapolation), which `val_khor`'s dragons trigger, as they should. Under a mass-adjusted ratio the reference results are unchanged: `invalid_draft_style` still fails and `val_khor` still passes.

## 11. Lanchester attrition fits: Bracken (1995), *NRL* 42, 559–577; Fricker (1998), *NRL* 45, 1–22; Lucas & Turkes (2004), *NRL* 51, 95–116

**Notation.** Losses follow Ḃ = a·R^p·B^q. The force ratio's trajectory depends only on p − q: 0 is the linear law, 1 the square law, −1 the logarithmic law.

- **Bracken** (Ardennes, days 2–11, grid search):
  - The best fits run from (p, q) = (1.3, 0.7) to (0.7, 1.3) and are nearly equal.
  - He finds the linear law fits and square-law concentration "apparently not present" (p. 574).
- **Fricker** (Ardennes, all 32 days, with air sorties, log-OLS):
  - p is insignificant; q is 3.2–5. Both the logarithmic law and Hartley's model are rejected.
  - He concludes: "Lanchester's basic models (linear and square) do not hold when fit to known data from an actual battle" (p. 19).
  - Timing of reserve commitment decides outcomes.
- **Lucas & Turkes** (Kursk and both Ardennes formats, bootstrap):
  - Kursk is best at p ≈ 5.7, q ≈ 1.3, but with R² = 0.24. The near-optimal region spans p from below 2 to over 10.
  - Fitting four phases separately gives R² = 0.83, against 0.80 from the phase means alone.
  - "It does not matter which of these laws you use, for with the right coefficients you will get about the same result" (abstract).

**Mapping onto our Helmbold kernel.** dA/dt = −β·B^W·A^(1−W), so W_equiv = (1 + p − q)/2. Estimates scatter from about −2 to about 2.9 depending on data formatting, and the defensible range is [0, 1].

**Deltas applied:**
- `helmbold_w` is now [0, 1] (was [0.2, 1]).
- W is derived from force composition, which is state: aimed fire pushes it toward 1, melee and area fire toward about 0.5. It is not a per-world constant, and **not an era constant** (ADR-0027 corrected).
- Phase and posture carry most of the variance.
- Breakpoints apply per engagement. Campaign losses of 16–23% occurred without collapse.
- `breakpoint` and `fort_multiplier` are marked as design values with no source yet.

**Still needed (pre-modern).** Helmbold 1965 (*OR* 13), 1969 (RAND P-4199) and 1987 (CAA-TP-87-16); Hartley & Helmbold 1995 (*NRL* 42); Willard 1962; Weiss 1966. Added to reading-list Batch 3.

## 12. Turchin, P. & Nefedov, S. A. (2009). *Secular Cycles*. Princeton UP

Read: Ch. 1 (theory), Ch. 2 (Plantagenet England) and Ch. 10 (general patterns); the other case chapters were skimmed for numbers. Page numbers are book pages (PDF page = book page + 11).

**Scope and model.** The theory covers **agrarian** states, meaning more than 50% of people work in agriculture (p. 6). It has four variables: population relative to carrying capacity (N/K), elites, state fiscal strength, and instability (pp. 29–30).
- As N approaches K, rents and prices rise and real wages fall.
- Elites multiply. **The elite peak lags the population peak** (p. 10).
- Income per elite falls, which drives intraelite competition and counter-elites.
- State expenses outrun revenue.
- Breakdown reduces both N and K (pp. 7–18).

**Phases** (pp. 19–21): expansion → stagflation (integrative), then crisis → depression or intercycle (disintegrative).
- The predicted length is 2–3 centuries for Western-European-type societies (p. 23).
- For Ibn Khaldun-type societies, with polygynous elites and desert counter-elites, it is about one century (p. 24).
- "Not periodic in the strictly mathematical sense" (p. 21).

**Cycles observed (Ch. 10).**

| Cycle | Years | Expansion / stagflation / crisis / depression (years) | Instability per decade: growth → decline |
|---|---|---|---|
| Plantagenet | 1150–1485 | 110 / 55 / 85 / 85 | 0.78 → 2.53 |
| Tudor-Stuart | 1485–1730 | 95 / 60 / 20 / 70 | 0.47 → 2.44 |
| Capetian | 1150–1450 | 100 / 65 / 50 / 85 | 0.80 → 3.26 |
| Valois | 1450–1660 | 70 / 50 / 30 / 60 | 0.75 → 6.67 |
| Republican Rome | 350–30 BCE | 170 / 50 / 100 / – | 0.41 → 4.40 |
| Principate | 30 BCE–285 CE | 122 / 69 / 32 / 88 | 0.61 → 3.83 |
| Muscovy | 1460–1620 | 70 / 35 / 50 / – | 0.60 → 3.80 |
| Romanov | 1620–1922 | 180 / 105 / 17 / – | – |

- **Table 10.2** (p. 310): instability averages 0.6 in growth phases against 3.8 in decline phases, "in all cases … (t-test; P << .001)".
- **China** (Table 10.3): 3.4 against 11.3 events per decade.
- **Table 10.1** adds four more Western cycles. Across all 12 (derived), cycle lengths run from 160 to 415 years, median about 270, which is about 3.6 cycles per millennium.

**Fathers-and-sons waves.** These recur every "roughly two generations (forty to sixty years)" (p. 28), and Ch. 10 gives 50 ± 10 years in all 8 cases (p. 310). They sit outside the theory proper (p. 27) and also appear, more mildly, in growth phases.

**Predicted vs observed:**
- Population and elite overproduction behave as predicted (pp. 303–305).
- Elite overproduction is the one factor always present at high instability (p. 314).
- Fiscal crisis is a weak predictor: Rome had none before its civil war (p. 306).

**Era-dependence.**
- The inverse link between population and real wages "holds only before the Industrial Revolution" (p. 304).
- Carrying capacity rises with technology. For example, English yields doubled in the 17th century, so a crisis came without a population collapse. This is why N/K, not N, is the right variable.
- The chronology stops in 1870 for Western Europe and 1922 for Russia. The book is silent on whether the cycles persist after industrialisation.

**Verification of repo claims:**
- 200–300 years as the model value: **confirmed**.
- **120–450 years: not in the source.** It appeared in R09, V-SF-04 and the burn-in guard.
- A spectral-peak test: **inappropriate**, because the cycles are not periodic and a 1,000-year run holds only 3–4 of them.
- 40–60-year waves: **confirmed**.
- Rome's dates: **corrected** to 133–121, 90–71 and 50–31 BCE (p. 205).
- "Breakdown requires all three strains": **weakened**. It is a tendency, as Rome shows.

**Deltas applied (`09`):**
- **V-SF-04 rewritten:**
  - cycle lengths between turning points of N/K: median 200–330 years, ≥ 80% in 150–450;
  - phase lengths;
  - N/K amplitude ≥ 1.5;
  - the elite lag;
  - agrarian windows only;
  - INSUFFICIENT below 30 completed cycles in the ensemble.
- **New V-SF-04b:** instability contrast between phases. It must exceed 1 in every cycle, with a median of 3–10. Phases come from N/K, not from instability, to avoid circularity.
- **V-SF-05:** the test becomes inter-peak intervals, with median 40–60 years.
- **V-SF-14:** the magnitudes are added and the rule softened.

**Implications for the build.** M6 tracks N/K (with K moved by technology) rather than raw N. Elite dynamics must produce a lag of about a generation. An "Ibn Khaldun" regime is a parameter set — elite polygyny, nomad counter-elites — not a separate system.
