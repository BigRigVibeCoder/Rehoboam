# ODD-R system spec: `<SYS.id>` <name>, fidelity F<n>, model semver <x.y.z>

> **ODD-R v2 (ADR-0033): the canonical spec is now `specs/systems/<id>.odd.toml`**, checked by `specs/systems/lint_odd.py` (template: `specs/systems/_template.odd.toml`). This Markdown form remains as the optional narrative companion for rationale and derivations. It may explain the data but never contradict it.
<!-- ODD-R adapts ODD (Grimm et al. 2010, 2020; A16) to the SystemModel contract (05.1 §2).
     Rules: keep elements 1-7 in this order and numbering; 8-10 are Rehoboam additions.
     Describe what step() does, not what you believe the model does. Same process names in §3, §7 and code.
     Every item is filled or marked "n/a: <reason>". Optional "Rationale:" line per element (cite R0x §n). -->

- **Status:** draft | frozen for WP-<id> · **Crate:** `rh-sys-<name>` · **Registry:** `[systems."<SYS.id>"]` @ <registry_version>
- **Attached to:** `backlog/wp/<id>.md` · **Change class of this revision:** C<0-3> · **Delta from:** <previous spec version or "new">

## 1. Purpose and patterns
- **Purpose** (≤ 3 sentences; specific enough to judge success; no mechanism yet):
- **Higher-level purpose:** substrate for other systems | drama coupling (`05.1` §8 #) | stylized-fact producer
- **Patterns this system must help produce** (fixed before design; they justify §2 choices):

| Test ID (V-SF / V-COUP / V-ERA) | Pattern, qualitative and testable | Identity / emergent | Role: sole / contributing | Evidence (R12 §n) |
|---|---|---|---|---|
| V-SF-.. | | | | |

- **Patterns known NOT to be reproduced yet** (report them; never drop silently):

## 2. Entities, state variables and scales
**Entity types:** persons (ledger) · regions · settlements · households · polities · armies · markets · pops · T1 actors · fields
**Owned columns** (single writer; Registry `[attributes]`/ownership entry exists before code):

| Column / relation | Entity | Type (i64, Q8.8, Q16.16, enum, f32 field) | Units and meaning | Plane | Crosses a threshold? → fixed point |
|---|---|---|---|---|---|

**Read set** (through `rh-contracts` accessors only; never another system's crate):

| Column | Owner system | Accessor | Same instant / lagged (§3) |
|---|---|---|---|

- **Restart test:** a keyframe of the owned columns + this system's pending timers restores the system exactly. Nothing derivable is stored; anything stored but derivable is listed as a cache with its rebuild rule.
- **Scales:** spatial unit (cell / region / settlement), temporal grain (pulse), extent; plane-B/C outputs listed separately.
- **Fidelity ladder** (`05.1` §3):

| Level | Representation | Status (target / available / n.a.) | Boundary adapter (lift / project) | Shared conserved quantities and declared targets |
|---|---|---|---|---|

## 3. Process overview and scheduling
| Process (= §7.x = fn name) | Pulse | Phase (`05.1` §4.2 #) | Active when | Entity order | Update: snapshot (sync) / sequential |
|---|---|---|---|---|---|

- **Order rules:** entities in `PersonId` / region-ID order, or keyed permutation `H(seed, salt, t, id)`; never insertion or hash order. Ties broken by: <rule>.
- **Timers** (ADR-0025):

| Timer | Sampled at (pulse, phase) | Fires in phase | RNG purpose | `resample_on_fork` | Re-sampled by input scope |
|---|---|---|---|---|---|

- **Lifecycle commands emitted** (applied by `rh-core` in `(phase, PersonId)` order): <Birth / Death{cause} / Migrate / TransferOffice / Infect …>
- **Lags** (reads against phase order; reader sees the previous instant):

| Column read | Owner | Why reverse of phase order | Visible value |
|---|---|---|---|

- **Events emitted:** type, phase, roles, anchored attributes (`attr@eN`), contingency edges.
- **Pseudo-code of one instant** (numbered lines; must match `step()`; tests cite line numbers):
~~~text
1 for each <entity> in <order> where <active>:
2   ...
~~~

## 4. Design concepts (one or two lines each)
4.1 **Basic principles:** theory adopted (e.g. Brass logit, Turchin SDT) and where it is extended beyond its source.
4.2 **Emergence:** outputs that emerge vs outputs imposed by equations (must match §1 identity/emergent tags).
4.3 **Adaptation:** decision rules of entities, or "rate-based: no adaptive behaviour".
4.4 **Objectives:** what adaptive entities score (IAUS curves as fixed-point LUTs; `05.1` §4.3).
4.5 **Learning:** do rules or parameters change with experience? How, and where is that state owned?
4.6 **Prediction:** forecasts entities use; from beliefs (plane A belief stores), never ground truth.
4.7 **Sensing:** which state each entity perceives, through which channel (`05.3`), with what lag, noise, reach.
4.8 **Interaction:** direct (lifecycle commands, verbs) or mediated (markets, fields, owned tables).
4.9 **Stochasticity** (all draws `ctx.rng(purpose, entity, k)`; vendored samplers only):

| RNG purpose (`rng_purposes.rs`) | Sampler | Entity key | k range | `resample_on_fork` | Why random (unmodelled variation / event frequency) |
|---|---|---|---|---|---|

4.10 **Collectives:** households, pops, factions, armies: imposed or emergent; who owns their state.
4.11 **Observation:** `declared_aggregates()` (AggregateSpec: statistic, margin, minimum N), Chronicle events, plane-B views.
4.12 **Era-state dependence** (ADR-0027; mechanics never read the era label):

| Parameter | Function of (era-state vars) | Form (effect-language expr / fixed-point LUT; monotone?) | Validated range | Out-of-range behaviour |
|---|---|---|---|---|

4.13 **Scale behaviour** (ADR-0028; mechanics never read the profile name):

| Setting | Micro ≤ 1k | Meso 1k–100k | Macro 100k–1M |
|---|---|---|---|
| Cadence / representation defaults this system contributes | | | |

- Minimum N for each statistic in 4.11 (below it: INSUFFICIENT). Behaviour at N = 1 and N = 0 (extinction). Every division by a count is guarded.

## 5. Initialization
| Owned column | Source (manifest field / template default / expander rule, `05.5`) | Sampled? key `H(seed, "init", …)` | Same in every run of a seed? |
|---|---|---|---|

- Spin-up: length, and what is discarded. Pre-epoch founders (negative `SimTime`) and off-map compartments, if any.

## 6. Input data (time-varying external drivers; parameters are NOT input data, see §7.0)
- **Push inputs** touching this system (Oracle verbs, player actions, catalyst verbs): dependency scope, phase 1 handling.
- **Pull inputs** (deliberation answers, `05.3` §8): key and fallback.
- **Exogenous schedules / boundary flows:** forced technology schedule (V-ERA-06), off-map flows (materialisation adapter).
- If none: "This system uses no input data to represent time-varying processes."

## 7. Submodels
**7.0 Parameters** (Registry; validator rejects out-of-bounds values; one row per parameter):

| Name | Symbol | Type / units | Bounds [lo, hi] | Default | Used in 7.x | Era function (4.12)? | Source (R12 §n, R0x §n, or "design value, unsourced") |
|---|---|---|---|---|---|---|---|

**7.x `<process_name>`** (one block per row of §3)
- **Equations / algorithm** (fixed point: Q-format, rounding mode, saturation or overflow rule; floats only in fields, `libm`, fixed order):
- **Golden vectors:** `specs/<system>/<name>.toml` + generator `selftest_<name>.py`; bit-exact unless stated.
- **Invariants:** `05.1` §9 items that apply + system-specific ones, as asserted in `invariants()`.
- **Tested in isolation:** fixture world, expected band or analytic solution.
- *Rationale (optional):* why this form, alternatives rejected.
- A submodel needing > ~10 pages gets a nested ODD-R (elements 1, 3, 7 only).

## 8. Couplings (hooks between systems)
| Direction (reads / commands / emits to) | Other system | Via (contract column / lifecycle command / event) | Phase, lag | Drama coupling (`05.1` §8) | Under coupling ablation (V-HUM-09) |
|---|---|---|---|---|---|

## 9. Verification and validation hooks
| Test ID | What it checks for this system | Gate / margin (fixed before the run) | Minimum N |
|---|---|---|---|
| V-INV-01, V-INV-04, V-INV-06 | invariants; ownership and derived order; crate checklist (EVO-2) | zero violations | n.a. |
| V-DET-09, V-DET-19 | sampler golden vectors; forbidden APIs | bit-equal; zero hits | n.a. |
| V-INV-07 / V-HYD-10 | Python prototype vs Rust port; F(n) vs F(n+1) | TOST, declared δ | 32 seeds |
| V-SF-.., V-ERA-01, V-SCALE-02 | §1 patterns; era-label blindness; small-N honesty | `09` §3 bands | per band |

## 10. Code map and provenance
| ODD-R item | File | fn / type |
|---|---|---|

- Reused text or models: source and licence (ADR-0021). Open questions → `docs/12_open_questions.md`.

**Completeness check (before the WP is approved):**
- [ ] Every owned column has units, plane and a Registry entry; every read goes through a contract accessor.
- [ ] Every §3 process has a §7 block, pseudo-code lines and its RNG purposes; names match the code.
- [ ] Every parameter has bounds and a source, or is marked "design value, unsourced".
- [ ] Every lag is justified; every §1 pattern maps to a §9 test; 4.12 and 4.13 are filled or "n/a: reason".
