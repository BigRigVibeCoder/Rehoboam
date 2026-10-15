# 09 — Verification & Validation Plan

*Status: Draft v1.0 (2026-10-15). **Verification** asks "did we build it right?" against the requirements in `03`. **Validation** asks "did we build the right thing?" against the success criteria and MOEs in `01`. Every gate below is automated unless marked **[H]**, which means a human judgement by you.*

---

## 1. Test architecture

```
                         ┌─────────────────────────────────────────┐
     Validation [H]      │  Rating sessions · blind A/B · MOE-1..6  │  per phase gate
                         ├─────────────────────────────────────────┤
     System V&V          │  Stylized-fact ensembles · coupling      │  nightly / labelled PRs
                         │  scenarios · Foresight calibration       │
                         ├─────────────────────────────────────────┤
     Integration         │  Golden runs (hash) · sifter twins ·     │  every PR
                         │  protocol conformance · hydration tests  │
                         ├─────────────────────────────────────────┤
     Component           │  Property tests (invariants) · sampler   │  every PR
                         │  golden vectors · unit tests · benches   │
                         └─────────────────────────────────────────┘
```

## 2. Verification suites

### 2.1 Determinism (DET-*)
| Test | Method | Gate |
|---|---|---|
| Thread-count invariance | Run reference worlds at 1, 2, 4 and N threads for 50 sim-years; compare monthly hashes | All equal |
| Repeat invariance | Same binary, 3 runs | All equal |
| Cross-platform (level 2 goal) | Nightly runs on Linux x86-64, Windows x86-64, macOS ARM; compare yearly hashes | Report. Becomes a gate once achieved |
| Replay | Record a session with inputs (Oracle actions, forks), then replay from the journal | Final hash equal |
| Keyframe round-trip | Write a keyframe, load it, advance 10 years; compare against a continuous run | Equal |
| Observer invariance (DET-6) | Run A headless. Run B with a scripted camera that hydrates and inspects throughout, with no material inputs | Hashes equal |
| Sampler golden vectors | Each vendored sampler × fixed keys | Bit-equal across releases |
| Unsynced guard | Fuzz the read-only APIs (console queries, frame builders, LLM context builders) | No hash change |

### 2.2 Invariants and properties (SIM-8, EVO-2)
- **proptest** strategies generate small random worlds and manifests within bounds. After every pulse, assert the 05.1 §9 invariants: conservation, non-negativity, genealogy acyclicity, exclusive relations, and agreement between the ledger and the named-actor table.
- **Long-run soak.** 1,000 years with invariants enabled in debug builds, nightly.

### 2.3 Golden worlds (DET-5, MOE-4)
- `worlds/` holds reference manifests: `val_khor` (feudal plus dragons), `aethelgard` (modern hive, 10 years), `delta_republic` (merchant republic), and a minimal `smoke` world.
- Each has recorded hash ladders (yearly) per engine version. A PR that changes them must declare the divergence: name the affected systems and attach an ensemble comparison of stylized facts before and after. A reviewer approves the new baseline.
- **Save compatibility corpus.** Every release must load the previous releases' saved worlds, either bit-identically under pinned model versions or through a logged migration (MOE-4).

### 2.4 Sifter (SIFT-*)
- **Twin test.** For each pattern, the live automaton and the DuckDB SQL twin must produce identical match sets on recorded chronicles. Fixture chronicles include adversarial cases: `unless` interleavings, deaths mid-pattern, window boundaries.
- **Anchoring lint.** The pattern compiler rejects bare (unanchored) state reads.
- **Throughput.** `bench/sifter_throughput` must not regress.

### 2.5 Hydration and multi-resolution consistency (05.2 §3.6)
| Criterion | Test |
|---|---|
| Identity stability | Hydrate the same person in different orders, at different times and on different machines: traits are identical |
| Conservation | After every sync, macro counts equal the sum of rows, including persons under lease |
| Non-contradiction | Display facts, dehydrate, advance a century, display again: there are no contradictions with `facts_ledger` |
| Temporal | Decrees enacted in the bubble give the same outcomes as when the person is unobserved |
| Distributional fidelity | Distributions of activity and destination for T2c proxies against full-sim samples: KS test p > 0.05 |
| Observer independence | See §2.1 |
| Fidelity ladder (EVO-3) | For each system with F(n) and F(n+1), the declared aggregates match within tolerance on 32-seed ensembles |

### 2.6 Protocol and interface conformance (IF-*)
- **JSON-RPC.** A schema conformance suite runs against the server, and the console's mock server is checked against the same suite.
- **Binary frames.** Encode/decode round-trip, delta-against-baseline correctness, and fuzzing of corrupt frames.
- **Python bindings.** Results from the Python bindings match the CLI.

### 2.7 Performance (PERF-*)
The benchmarks listed in `08` §9 run under criterion in CI on fixed runner hardware, plus a nightly run on the reference desktop. A slowdown of more than 10% fails the build.

### 2.8 LLM-facing verification (LLM-*)
| Eval | Gate |
|---|---|
| RP-200 (role-play over our belief store) | Leak = 0 on concealment items; unsupported claims < 2%; TTFT p95 < 0.5 s |
| ARG-300 (whisper parsing) | Field F1 ≥ 0.85; meta-instruction recall ≥ 0.95 |
| Red-team corpus (injections, master-key tokens, persuasion taxonomy) | No state delta beyond the documented caps; all meta-instructions penalised |
| INTENT-100 (world compile) | Template match ≥ 90%; post-repair validity ≥ 98% |
| Schema robustness | Truncation, refusal and enum-case drift inputs are handled without crashes |

### 2.9 Narrative QA (NAR-*)
- **Per-scene.** Zero T0/T1 contradictions and zero T3 inventions (graph check, then NLI).
- **Per-book.** Backbone coverage ≥ 95%. Every causal parent is shown or referenced before its child. Zero epistemic leaks.
- **Prose.** Style metrics within the style card's bands. Slop density and repetition below thresholds. Cross-book n-gram similarity below a threshold.
- **Exporters.** Fountain parse round-trip and page-count band. TTS manifest has complete voice mappings.

## 3. Validation (fit for purpose)

### 3.1 Stylized-fact harness (MOP-5; R09 §11)
Each reference world runs 32 seeds per manifest. A run passes when its statistics fall inside these bands:

| Fact | Band (pre-modern unless noted) |
|---|---|
| Long-run growth without technology shocks | mean r ∈ [−0.1, +0.3]%/yr; population ∈ [0.3, 5]·K |
| Child mortality to age 15 | 35–55% |
| Mortality crises | ≥ 1 event of ≥ 10% loss within 1–2 years per century |
| Secular cycle | Spectral peak at 120–450 years in N or W |
| Violence waves (soft) | Secondary peak at 40–60 years during disintegrative phases |
| Ruler deaths | 5–15 murders per 1,000 ruler-years; 15–30% of deaths violent |
| Polity lifespans | Approximately exponential, mean 100–300 years |
| City sizes | Top-20 rank-size slope ∈ [−1.3, −0.7] |
| War severity | Heavy tail, fitted α ∈ [1.3, 1.9] |
| Malthusian test | A technology shock raises density, not long-run income |
| Post-plague wages | Real wage rises after ≥ 20% mortality (sign test) |
| Hive / testing (modern) | At prevalence < 1% with se/sp = 0.99, more than 50% of detentions are false positives |

The gate is ≥ 90% of bands passed across the ensemble. Morris and Sobol screening, run on labelled PRs, flags parameters with outsized influence.

### 3.2 Drama-coupling scenarios (05.1 §8)
For each designed coupling (famine → succession crisis; plague → purge → revolt; hive detection spiral; elite overproduction; dragon age), an ensemble checks two properties:
- **Possible.** The full chain completes in ≥ x% of seeds.
- **Not forced.** The chain is absent in ≥ y% of seeds.

The thresholds x and y are set per coupling in P1–P3.

### 3.3 Foresight calibration (ORA-5)
Back-test with ≥ 200 prophecies over historical forks of recorded timelines and draw a reliability diagram. Events forecast at probability p must occur at rate p ± 0.05 in each decile bucket that holds ≥ 20 samples.

### 3.4 Human validation [H] (MOE-1, MOE-2, MOE-6)
| Activity | Protocol | Gate |
|---|---|---|
| **Story-yield sessions** | Ten random seeds per phase gate. You run the Time-Machine with auto-pause and rate every pause 1–5 (TM-6) | P2: median ≥ 3 events rated ≥ 4 per century. P3: ≥ 6 |
| **Chronicle A/B** | Blind pairs: a sifter-curated chronicle against a random-event chronicle from the same run | ≥ 80% preference for curated |
| **Bad-run audit** | Automated: count seeds with fewer than N completed arcs per century | < 10% |
| **Interrogation sessions** (P5/P6) | Interrogate 20 random people per session; log contradictions | 0 hard, < 1 soft per 20 |
| **Narrative read** (P4) | Blind: Rehoboam saga against an ungrounded LLM saga on the same premise; rate on HANNA criteria plus "would keep reading" | Rehoboam preferred on coherence and surprise; no regression on engagement |
| **Diorama believability** (P5) | 30-minute ambient sessions; note "fake" moments: pops, loops, impossible errands | Trend toward 0 per session |

### 3.5 Personal preference model
Ratings train a small model over significance components, which personalises auto-pause thresholds. **Validate by hold-out.** On each new session, its ranking of pauses should correlate with your ratings at Spearman ρ ≥ 0.4 by P3.

## 4. Observability for V&V
- **Rerun.** Per-system timing, invariant counters, sampled agents, the hydration bubble and AOI cells on a `sim_tick` timeline.
- **Textual TUI.** Ticks/s, memory, pattern hits and logs for headless runs.
- **Run report** (auto-generated per run): stylized-fact table, bad-run metrics, top arcs, timings, costs.

## 5. Phase gates (summary; see `11_roadmap.md`)
Each phase ends with:
1. All verification suites green.
2. The validation activities scheduled for that phase completed.
3. A **go / iterate / pivot** decision recorded in `docs/decisions-log.md`, with evidence links.
