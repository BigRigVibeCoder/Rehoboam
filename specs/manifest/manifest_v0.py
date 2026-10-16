"""
Rehoboam World Manifest — schema v0 (Pydantic v2) + Registry-aware semantic validation.

Spec: docs/05_architecture/05.5_world_compiler_and_dsl.md, ADR-0010.

Design rules (vs. the draft schema):
  * No free-string semantics: every mechanic reference is a Registry ID, checked against registry TOML.
  * No Dict[str, Any]: catalysts and peoples are discriminated unions.
  * Catalysts are INITIAL CONDITIONS, never forced outcomes (no trigger/consequence strings).
  * Syntactic validation (this file) is necessary but not sufficient: the fast liveness gate
    (200 y x 8 seeds) and the background certification run (1000 y x 32 seeds) against stylized
    facts live in the engine (05.5 §4.2, docs/09).
  * Flavour text (names, naming seeds, prose) is carried here only so the expander can move it into
    the flavour asset. No simulation crate may read it.

Usage:
  python manifest_v0.py validate examples/val_khor.yaml --registry registry_v0.toml
  python manifest_v0.py schema > manifest_v0.schema.json
"""
from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path
from typing import Annotated, Literal, Union

from pydantic import BaseModel, ConfigDict, Field, ValidationError, model_validator

Slug = Annotated[str, Field(pattern=r"^[a-z][a-z0-9_]{1,63}$")]
Unit = Annotated[float, Field(ge=0.0, le=1.0)]
# The engine stores an integer in at most 64 bits, so no manifest integer may exceed i64
# (SPR-004 DEF-2, Architect ruling 2026-10-16). Fields with a tighter bound keep their own.
Int64 = Annotated[int, Field(ge=-(2**63), le=2**63 - 1)]


class Strict(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)


# --------------------------------------------------------------------------- world meta
class Calendar(Strict):
    days_per_year: Literal[360, 365] = 360
    months: Literal[12] = 12          # fixed in v0: the M3 harvest calendar and the monthly tick assume it
    seasons: Literal[2, 4] = 4
    epoch_label: str = "Year"


class Horizon(Strict):
    years: Annotated[int, Field(ge=1, le=5000)]
    label: Literal["historical", "speculative"] = "historical"


class Milestone(Strict):
    id: Slug
    by_year: Annotated[int, Field(ge=0, le=5000)]


class Trajectory(Strict):
    """Dated technologies and institutions a speculative world is expected to reach (05.5 §5)."""
    technologies: list[Milestone] = []
    institutions: list[Milestone] = []


ActivityResolution = Literal["life_course", "blocks", "minutes"]   # ADR-0031: a world setting, never a side effect of viewing


class Focus(Strict):
    """A focus region: daily life there is authoritative at a finer resolution (ADR-0031 §3). The settlement is one the
    manifest names (e.g. a polity seat); choosing one after world generation is a journaled migration (05.1 §3)."""
    settlement: Slug
    activity_resolution: Literal["blocks", "minutes"]


class WorldMeta(Strict):
    id: Slug
    display_name: Annotated[str, Field(min_length=1, max_length=120)]
    template: Slug
    seed: Annotated[int, Field(ge=0, le=2**63 - 1)]
    calendar: Calendar = Calendar()
    horizon: Horizon
    trajectory: Trajectory | None = None
    fidelity_profile: dict[str, Literal["F0", "F1", "F2", "F3"]] = {}
    activity_resolution: ActivityResolution | None = None   # None: the scale profile's default (blocks <= 1k, else life_course)
    focus: list[Focus] = []

    @model_validator(mode="after")
    def _speculative_needs_trajectory(self) -> "WorldMeta":
        if self.horizon.label == "speculative":
            t = self.trajectory
            if t is None or not (t.technologies or t.institutions):
                raise ValueError("horizon.label = 'speculative' requires world.trajectory with at least one dated technology or institution")
        return self


# --------------------------------------------------------------------------- geography
class Climate(Strict):
    zone: Slug                        # Registry `climate_zones`
    harvest_sigma: float = 0.2
    autocorr: float = 0.1
    spatial_share: float = 0.6
    volcanic_rate: float = 0.004


class Ecology(Strict):
    livestock_kg_per_capita: Annotated[float, Field(ge=0.0, le=2000.0)] = 150.0
    wild_prey_tonnes: Annotated[float, Field(ge=0.0)] = 0.0


class Geography(Strict):
    raster_resolution: Literal[64, 128, 256, 512, 1024, 2048, 4096] = 512
    region_count: Annotated[int, Field(ge=1, le=5000)] = 600     # 1 for a single household (ADR-0028)
    heightmap_template: Slug
    climate: Climate
    ecology: Ecology = Ecology()


# --------------------------------------------------------------------------- peoples (discriminated)
class Culture(Strict):
    id: Slug
    name: str
    naming_seed: Annotated[str, Field(max_length=200, description="Flavour only: seeds the name generator. Never mechanics.")]
    value_bias: dict[str, Annotated[float, Field(ge=-1.0, le=1.0)]] = {}


class Faith(Strict):
    id: Slug
    name: str
    doctrine_tags: list[Slug] = []


class ClassShares(Strict):
    commoner: Unit
    burgher: Unit
    elite: Unit

    @model_validator(mode="after")
    def _sum_to_one(self) -> "ClassShares":
        s = self.commoner + self.burgher + self.elite
        if abs(s - 1.0) > 1e-6:
            raise ValueError(f"class shares must sum to 1.0 (got {s:.4f})")
        return self


class HumanPeople(Strict):
    archetype: Literal["human"]
    population: Annotated[int, Field(ge=1, le=10_000_000)]     # target range 1 → 1M (ADR-0028); > 1M is a warning
    cultures: Annotated[list[Culture], Field(min_length=1)]
    faiths: Annotated[list[Faith], Field(min_length=1)]
    class_shares: ClassShares


class CreatureParams(Strict):
    mass_kg: float
    metabolism_multiplier: float = 1.0
    flying: bool = False
    bondable: bool = False


class CreaturePeople(Strict):
    archetype: Literal["apex_predator"]
    variant: Slug
    population: Annotated[int, Field(ge=1, le=100_000)]
    params: CreatureParams


People = Annotated[Union[HumanPeople, CreaturePeople], Field(discriminator="archetype")]


# --------------------------------------------------------------------------- polities
class Dynasty(Strict):
    id: Slug
    name: str
    generations: Annotated[int, Field(ge=1, le=6)] = 3


class Polity(Strict):
    id: Slug
    name: str
    government: Slug
    succession_law: Slug
    titles: dict[Slug, str] = Field(description="abstract role -> concrete title, e.g. authority -> High_King")
    dynasties: list[Dynasty] = []


# --------------------------------------------------------------------------- catalysts (discriminated; initial conditions only)
class DynasticTensionInitial(Strict):
    opinion_successor_of_authority: Annotated[int, Field(ge=-100, le=100)] = 0
    opinion_authority_of_successor: Annotated[int, Field(ge=-100, le=100)] = 0
    fear_authority: Unit = 0.0
    heir_age_years: Annotated[int, Field(ge=0, le=80)] = 20


class DynasticTension(Strict):
    kind: Literal["dynastic_tension"]
    polity: Slug
    initial: DynasticTensionInitial


class OverlayEffect(Strict):
    overlay: Slug
    holder_delta: Annotated[float, Field(ge=-1.0, le=1.0)]


class Artifact(Strict):
    kind: Literal["artifact"]
    name: str
    polity: Slug
    holder_role: Slug
    effects: Annotated[list[OverlayEffect], Field(min_length=1)]


class CovertOutbreak(Strict):
    kind: Literal["covert_outbreak"]
    seed_infected: Annotated[int, Field(ge=1, le=10_000)]
    seed_region_hint: Literal["capital", "port", "periphery", "random"] = "random"
    channel: Literal["hive"] = "hive"
    start_day: Annotated[Int64, Field(ge=0)] = 0


class ClimateShock(Strict):
    kind: Literal["climate_shock"]
    year: Annotated[Int64, Field(ge=0)]
    severity_sigma: Annotated[float, Field(ge=0.5, le=4.0)]
    duration_years: Annotated[int, Field(ge=1, le=10)] = 1


class WagerStage(Strict):
    allow: Annotated[list[str], Field(min_length=1)]            # Registry verbs the agent may use in this stage
    spare_life: bool = True                                     # the protagonist may not be killed by the agent's verbs
    unlock_when: Annotated[str, Field(max_length=200)] | None = None   # plane-A predicate in the effect language (05.5 §2.1)


class Wager(Strict):
    """An agentive catalyst (ADR-0028 §3): an in-world agent may intervene within a permission scope.
    Its verbs are journaled material inputs; consequences and the protagonist's response are simulated, never forced."""
    kind: Literal["wager"]
    agent: Slug
    protagonist: Slug
    stages: Annotated[list[WagerStage], Field(min_length=1)]


Catalyst = Annotated[Union[DynasticTension, Artifact, CovertOutbreak, ClimateShock, Wager], Field(discriminator="kind")]


# --------------------------------------------------------------------------- explicit cast, households, outside world, agents (ADR-0028)
class CastMember(Strict):
    """A named person the expander must create exactly; everyone else is generated."""
    id: Slug
    name: str
    sex: Literal["male", "female"]
    age_years: Annotated[int, Field(ge=0, le=120)]
    household: Slug | None = None
    flags: list[Literal["protagonist", "deep"]] = []
    initial: dict[Slug, Annotated[float, Field(ge=-1.0, le=1.0)]] = {}   # Registry attributes of arity 1, e.g. faith, integrity


class Children(Strict):
    sons: Annotated[int, Field(ge=0, le=40)] = 0
    daughters: Annotated[int, Field(ge=0, le=40)] = 0


class Household(Strict):
    id: Slug
    head: Slug                                                  # a cast member
    children: Children = Children()
    servants: Annotated[int, Field(ge=0, le=100_000)] = 0
    holdings: dict[Slug, Annotated[Int64, Field(ge=0)]] = {}      # Registry goods → quantity


class OffMapActor(Strict):
    id: Slug
    name: str
    kind: Slug                                                  # Registry off_map_kinds
    strength: Unit = 0.5


class Agent(Strict):
    id: Slug
    name: str
    archetype: Slug                                             # e.g. supernatural_agent
    verbs: Annotated[list[str], Field(min_length=1)]
    policy: Literal["deterministic", "player"] = "deterministic"



# --------------------------------------------------------------------------- narrative & presentation
class Narrative(Strict):
    pattern_packs: Annotated[list[Slug], Field(min_length=1)]
    default_style_card: Slug


class Presentation(Strict):
    era_art: Slug
    palette: Slug


# --------------------------------------------------------------------------- root
class Manifest(Strict):
    manifest_version: Literal[0]
    registry_version: Annotated[str, Field(pattern=r"^[0-9]+\.[0-9]+\.[0-9]+$")]  # ASCII digits (SPR-004 GAP-4)
    world: WorldMeta
    geography: Geography
    peoples: Annotated[list[People], Field(min_length=1)]
    polities: list[Polity] = []
    cast: list[CastMember] = []
    households: list[Household] = []
    off_map_actors: list[OffMapActor] = []
    agents: list[Agent] = []
    systems: dict[str, dict[str, float]] = Field(description="system id -> {param: value}; params bounded by Registry")
    institutions: list[Slug] = []
    technologies: list[Slug] = []
    catalysts: list[Catalyst] = []
    overlays: list[Slug] = []
    narrative: Narrative
    presentation: Presentation


# =========================================================================== Registry-aware validation
# Carbone & Gittleman (2002): "10,000 kg of prey supports about 90 kg" of a carnivore species (p. 2273). 0.009 is their
# central, mass-invariant estimate for mammalian Carnivora of 0.14–310 kg; the fitted form gives ≈ 0.006 at 1–5 t, and real
# species span 0.001–0.05. It is a design-conservative plausibility bound, not a biological maximum (R12 §10).
CARBONE_RATIO = 0.009  # kg carnivore supported per kg standing prey
CARBONE_MAX_MASS_KG = 310.0  # heaviest species in their data; above this the rule is extrapolated


class Issue(BaseModel):
    level: Literal["error", "warning"]
    path: str
    message: str


def _ids(reg: dict, section: str) -> set[str]:
    sec = reg.get(section, {})
    return set(sec.get("ids", [])) if "ids" in sec else set(sec.keys())


def validate_against_registry(m: Manifest, reg: dict) -> list[Issue]:
    out: list[Issue] = []

    def err(path: str, msg: str) -> None:
        out.append(Issue(level="error", path=path, message=msg))

    def warn(path: str, msg: str) -> None:
        out.append(Issue(level="warning", path=path, message=msg))

    if m.registry_version != reg.get("registry_version"):
        warn("registry_version", f"manifest pins {m.registry_version}, registry is {reg.get('registry_version')}")

    # template + horizon
    tpl = reg.get("templates", {}).get(m.world.template)
    if tpl is None:
        err("world.template", f"unknown template '{m.world.template}'")
    else:
        lo, hi = tpl["horizon_years"]
        if m.world.horizon.years > hi and m.world.horizon.label != "speculative":
            err("world.horizon", f"template '{m.world.template}' supports ≤{hi} historical years; "
                                 f"set horizon.label='speculative' (and declare a tech/institution trajectory) for {m.world.horizon.years}")
        if m.world.horizon.years < lo:
            warn("world.horizon", f"horizon {m.world.horizon.years}y shorter than template minimum {lo}y")

    # systems, params, fidelity
    reg_sys = reg.get("systems", {})
    for sid, params in m.systems.items():
        if sid not in reg_sys:
            err(f"systems.{sid}", "unknown system")
            continue
        bounds = reg_sys[sid].get("params", {})
        for p, v in params.items():
            if p not in bounds:
                err(f"systems.{sid}.{p}", "unknown parameter")
            else:
                lo, hi = bounds[p]
                if not (lo <= v <= hi):
                    err(f"systems.{sid}.{p}", f"value {v} outside Registry bounds [{lo}, {hi}]")
    for sid, lvl in m.world.fidelity_profile.items():
        if sid not in reg_sys:
            err(f"world.fidelity_profile.{sid}", "unknown system")
        elif lvl not in reg_sys[sid]["fidelity_levels"]:
            err(f"world.fidelity_profile.{sid}", f"fidelity {lvl} not implemented (available: {reg_sys[sid]['fidelity_levels']})")
        if sid not in m.systems and sid not in (tpl or {}).get("default_systems", []):
            warn(f"world.fidelity_profile.{sid}", "fidelity set for a system that is neither a template default nor in `systems`")

    # climate params share M3 bounds
    m3 = reg_sys.get("M3.agriculture", {}).get("params", {})
    for p in ("harvest_sigma", "autocorr", "spatial_share", "volcanic_rate"):
        if p in m3:
            lo, hi = m3[p]
            v = getattr(m.geography.climate, p)
            if not (lo <= v <= hi):
                err(f"geography.climate.{p}", f"value {v} outside Registry bounds [{lo}, {hi}]")

    # world vocabularies (no free-string semantics)
    if m.geography.heightmap_template not in _ids(reg, "heightmap_templates"):
        err("geography.heightmap_template", f"'{m.geography.heightmap_template}' not in Registry")
    if m.geography.climate.zone not in _ids(reg, "climate_zones"):
        err("geography.climate.zone", f"'{m.geography.climate.zone}' not in Registry")
    values, doctrines = _ids(reg, "values"), _ids(reg, "doctrine_tags")
    for i, p in enumerate(m.peoples):
        if isinstance(p, HumanPeople):
            for j, c in enumerate(p.cultures):
                for k in c.value_bias:
                    if k not in values:
                        err(f"peoples[{i}].cultures[{j}].value_bias.{k}", "not a Registry value")
            for j, fa in enumerate(p.faiths):
                for d in fa.doctrine_tags:
                    if d not in doctrines:
                        err(f"peoples[{i}].faiths[{j}].doctrine_tags", f"'{d}' not in Registry")
    if m.world.trajectory is not None:
        for kind, allowed in (("technologies", _ids(reg, "technologies")), ("institutions", _ids(reg, "institutions"))):
            for j, ms in enumerate(getattr(m.world.trajectory, kind)):
                if ms.id not in allowed:
                    err(f"world.trajectory.{kind}[{j}]", f"'{ms.id}' not in Registry")
                if ms.by_year > m.world.horizon.years:
                    warn(f"world.trajectory.{kind}[{j}]", f"milestone year {ms.by_year} beyond horizon {m.world.horizon.years}")

    # vocabulary references
    vocab = {
        "institutions": (m.institutions, _ids(reg, "institutions")),
        "technologies": (m.technologies, _ids(reg, "technologies")),
        "overlays": (m.overlays, _ids(reg, "overlays")),
        "narrative.pattern_packs": (m.narrative.pattern_packs, _ids(reg, "pattern_packs")),
        "narrative.default_style_card": ([m.narrative.default_style_card], _ids(reg, "style_cards")),
    }
    for path, (vals, allowed) in vocab.items():
        for v in vals:
            if v not in allowed:
                err(path, f"'{v}' not in Registry")

    roles = _ids(reg, "roles")
    for i, pol in enumerate(m.polities):
        if pol.government not in _ids(reg, "governments"):
            err(f"polities[{i}].government", f"'{pol.government}' not in Registry")
        if pol.succession_law not in _ids(reg, "succession_laws"):
            err(f"polities[{i}].succession_law", f"'{pol.succession_law}' not in Registry")
        for r in pol.titles:
            if r not in roles:
                err(f"polities[{i}].titles.{r}", "unknown abstract role")
        for needed in ("authority", "successor"):
            if needed not in pol.titles:
                err(f"polities[{i}].titles", f"missing required role '{needed}'")

    polity_ids = {p.id for p in m.polities}
    humans = sum(p.population for p in m.peoples if isinstance(p, HumanPeople))
    if humans > 1_000_000:
        warn("peoples", f"{humans:,} persons: above the 1M target range (ADR-0028); expect batch-speed runs (docs/08)")

    # activity resolution (ADR-0031): cost scales with persons x resolution; estimates at ~1e8 updates/s
    # 'minutes' is allowed at any scale (director, 2026-10-16: ~1.5 h per sim-year at 1M is acceptable); the estimate assumes
    # simple activity updates, and richer Sims-like agents (needs, object choice, pathing) may cost 10-100x more.
    res = m.world.activity_resolution
    if res == "minutes" and humans > 1_000:
        lo_h = humans / 1000 * 5 / 3600
        warn("world.activity_resolution", f"'minutes' for {humans:,} persons: about {lo_h:.2f} h per sim-year with simple activity "
                                          f"updates, up to {lo_h * 100:.0f} h with Sims-like agents; a batch run (ADR-0031)")
    if res == "blocks" and humans > 100_000:
        warn("world.activity_resolution", f"'blocks' for {humans:,} persons: about {humans / 1000 * 0.06 * 1000 / 3600:.1f} h per "
                                          "simulated millennium, above PERF-1; consider focus regions (ADR-0031)")
    seen = set()
    for i, f in enumerate(m.world.focus):
        if f.settlement in seen:
            err(f"world.focus[{i}]", f"duplicate focus settlement '{f.settlement}'")
        seen.add(f.settlement)
        if res == "minutes" or (res == "blocks" and f.activity_resolution == "blocks"):
            warn(f"world.focus[{i}]", "focus resolution is not finer than the world's; it has no effect")

    # cast, households, outside world, agents (ADR-0028)
    attrs = reg.get("attributes", {})
    cast_ids = {c.id for c in m.cast}
    hh_ids = {h.id for h in m.households}
    if len(cast_ids) != len(m.cast):
        err("cast", "duplicate cast ids")
    named = len(m.cast) + sum(h.children.sons + h.children.daughters + h.servants for h in m.households)
    if named > humans:
        err("cast", f"cast, children and servants ({named}) exceed the human population ({humans})")
    for i, c in enumerate(m.cast):
        if c.household is not None and c.household not in hh_ids:
            err(f"cast[{i}].household", f"unknown household '{c.household}'")
        for k in c.initial:
            if k not in attrs:
                err(f"cast[{i}].initial.{k}", "not a Registry attribute")
            elif len(attrs[k].get("args", [])) > 2:
                err(f"cast[{i}].initial.{k}", "only attributes of the person (optionally towards one deity) can be set")
    goods = _ids(reg, "goods")
    for i, h in enumerate(m.households):
        if h.head not in cast_ids:
            err(f"households[{i}].head", f"'{h.head}' is not a cast member")
        for g in h.holdings:
            if g not in goods:
                err(f"households[{i}].holdings.{g}", "not a Registry good")
    for i, o in enumerate(m.off_map_actors):
        if o.kind not in _ids(reg, "off_map_kinds"):
            err(f"off_map_actors[{i}].kind", f"'{o.kind}' not in Registry")
    reg_verbs = set(reg.get("verbs", {}).keys())
    agents = {a.id: a for a in m.agents}
    for i, a in enumerate(m.agents):
        if a.archetype not in reg.get("archetypes", {}):
            err(f"agents[{i}].archetype", f"unknown archetype '{a.archetype}'")
        for v in a.verbs:
            if v not in reg_verbs:
                err(f"agents[{i}].verbs", f"verb '{v}' not in Registry")
    for i, c in enumerate(m.catalysts):
        if isinstance(c, Wager):
            if c.agent not in agents:
                err(f"catalysts[{i}].agent", f"unknown agent '{c.agent}'")
            if c.protagonist not in cast_ids:
                err(f"catalysts[{i}].protagonist", f"'{c.protagonist}' is not a cast member")
            for j, s in enumerate(c.stages):
                extra = set(s.allow) - set(agents[c.agent].verbs if c.agent in agents else [])
                if extra:
                    err(f"catalysts[{i}].stages[{j}].allow", f"verbs not held by the agent: {sorted(extra)}")
    for i, c in enumerate(m.catalysts):
        if isinstance(c, (DynasticTension, Artifact)) and c.polity not in polity_ids:
            err(f"catalysts[{i}].polity", f"unknown polity '{c.polity}'")
        if isinstance(c, Artifact):
            for j, e in enumerate(c.effects):
                if e.overlay not in m.overlays:
                    err(f"catalysts[{i}].effects[{j}]", f"overlay '{e.overlay}' not enabled in manifest.overlays")
        if isinstance(c, CovertOutbreak):
            if "M5.contagion" not in m.systems:
                err(f"catalysts[{i}]", "covert_outbreak requires system M5.contagion")
            if not ({"blood_assay", "pcr_testing"} & set(m.technologies)):
                warn(f"catalysts[{i}]", "covert outbreak without a testing technology: detection only via behavioural tells")
            if c.seed_infected > max(1, humans // 100):
                warn(f"catalysts[{i}].seed_infected", "seed >1% of population: the 'covert' phase may be skipped")
            m5 = m.systems.get("M5.contagion", {})
            if {"blood_assay", "pcr_testing"} & set(m.technologies):
                for p in ("test_sensitivity", "test_specificity"):
                    if p not in m5:
                        err(f"systems.M5.contagion.{p}", "a testing technology is declared: set test_sensitivity and test_specificity explicitly")

    # creatures: archetype/variant, capabilities, ecology system, energetics (Carbone ratio)
    arche = reg.get("archetypes", {})
    if any(isinstance(p, CreaturePeople) for p in m.peoples) and "S4.ecology" not in m.systems:
        err("systems", "creature peoples require system S4.ecology (prey, grazing, predator energetics)")
    prey_t = humans * m.geography.ecology.livestock_kg_per_capita / 1000.0 + m.geography.ecology.wild_prey_tonnes
    for i, p in enumerate(m.peoples):
        if isinstance(p, CreaturePeople):
            a = arche.get(p.archetype, {})
            if p.variant not in a.get("variants", []):
                err(f"peoples[{i}].variant", f"'{p.variant}' not a Registry variant of {p.archetype}")
            caps = set(a.get("capabilities", []))
            for cap in ("flying", "bondable"):
                if getattr(p.params, cap) and cap not in caps:
                    err(f"peoples[{i}].params.{cap}", f"capability not permitted by archetype {p.archetype}")
            lo, hi = a.get("params", {}).get("mass_kg", [0, float("inf")])
            if not (lo <= p.params.mass_kg <= hi):
                err(f"peoples[{i}].params.mass_kg", f"outside Registry bounds [{lo}, {hi}]")
            mm_lo, mm_hi = reg_sys.get("S4.ecology", {}).get("params", {}).get("metabolism_multiplier", [0.1, 10.0])
            if not (mm_lo <= p.params.metabolism_multiplier <= mm_hi):
                err(f"peoples[{i}].params.metabolism_multiplier", f"outside Registry bounds [{mm_lo}, {mm_hi}]")
            if p.params.mass_kg > CARBONE_MAX_MASS_KG:
                warn(f"peoples[{i}].params.mass_kg", f"{p.params.mass_kg:.0f} kg is beyond the 0.14–310 kg range behind the Carbone ratio; "
                                                     "the energetics bound is extrapolated (metabolism_multiplier = field metabolic rate "
                                                     "relative to a mammalian carnivore of equal mass)")
            predator_t = p.population * p.params.mass_kg / 1000.0
            supportable_t = CARBONE_RATIO * prey_t / max(p.params.metabolism_multiplier, 1e-9)
            if predator_t > supportable_t:
                err(f"peoples[{i}]", f"energetics: {p.population}×{p.params.mass_kg:.0f} kg = {predator_t:.0f} t of predator, "
                                     f"but standing prey {prey_t:.0f} t supports only {supportable_t:.0f} t at metabolism "
                                     f"×{p.params.metabolism_multiplier} (Carbone ratio {CARBONE_RATIO}). Lower population, "
                                     f"lower metabolism_multiplier, or declare wild_prey_tonnes.")
    return out


def load_manifest(path: Path) -> Manifest:
    import yaml  # PyYAML
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    return Manifest.model_validate(data)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    v = sub.add_parser("validate")
    v.add_argument("manifest", type=Path)
    v.add_argument("--registry", type=Path, default=Path(__file__).with_name("registry_v0.toml"))
    sub.add_parser("schema")
    args = ap.parse_args(argv)

    if args.cmd == "schema":
        print(json.dumps(Manifest.model_json_schema(), indent=2))
        return 0

    try:
        m = load_manifest(args.manifest)
    except ValidationError as e:
        print(f"SCHEMA INVALID: {args.manifest}\n{e}")
        return 2
    reg = tomllib.loads(args.registry.read_text(encoding="utf-8"))
    issues = validate_against_registry(m, reg)
    errors = [i for i in issues if i.level == "error"]
    for i in issues:
        print(f"[{i.level.upper()}] {i.path}: {i.message}")
    print(f"{'INVALID' if errors else 'VALID'}: {args.manifest} "
          f"({len(errors)} errors, {len(issues) - len(errors)} warnings) — next gate: liveness burn-in (engine)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
