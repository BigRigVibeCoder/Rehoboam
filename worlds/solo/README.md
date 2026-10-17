# The golden world `solo` (rung 0)

`solo` is rung 0 of the complexity ladder: one person, Ada, on a tiny map (director's ruling D4). In
P1a it runs ageing and death only (SPR-011 rulings 1, 2): `rh-worldgen` writes Ada as the one founder
(female, aged 25 at tick 0, born at a keyed offset in the year before the epoch), and
`M2.demography` gives her one keyed Bernoulli per month on the rung-0 Brass schedule at the Registry
default e0 = 30 until a death timer fires at a day start. The manifest also names M1, M3, M4 and
S5, which this build lacks: they are `not_built`, skipped and never stubbed, and `rh-cli run solo
--strict` refuses the world (SPR-011 ruling 3).

| File | What it is |
|---|---|
| `world.toml` | The manifest (`specs/manifest/examples/solo.yaml`, the one copy) and its pinned SHA-256, the Registry and the input script (SPR-011 ruling 16) |
| `inputs.toml` | The committed input script: one scope-only material input on Ada in year 10, so epoch-keyed draws are inside the golden run (ruling 15). No death is pending then, so nothing is re-sampled |
| `baselines/solo_yearly_200y.ladder` | The PR tier's baseline: the 200-year yearly hash ladder (ruling 16; V-GOLD-01) |

**The history tuple** (DET-1) is in the ladder's header: `engine_build` (`rh-golden <version>` with
the built systems), `registry_version`, `manifest_hash` (the manifest's SHA-256) and the seed, with the
journal's row count and head; the `systems` line lists the built and not-built systems.

**Tiers.** PR (`bin/check_all.sh --part golden`): 200 years yearly at 1 and N threads, both equal to
the baseline; the hand-edited and the not-`[C3]` baseline controls; `--strict` refused; the 1,000-year
wall time reported (V-SCALE-03). Nightly (`bin/golden_nightly.sh`): 1,000 years monthly at 1, 2, 4
and N threads x 3 repeats, all equal, and the yearly cut of a 200-year monthly run equal to the
baseline.

**Regenerating the baseline.** Never typed: `bin/regen_baseline.sh solo`, in a C3 PR's final commit
whose subject ends with `[C3]`, announced to the director before the merge (SPR-008 T-04; SPR-011
director approval 2). Re-pin `world.toml` only together with the baseline.
