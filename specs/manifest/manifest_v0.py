"""
Rehoboam World Manifest — schema v0 (Pydantic v2) + Registry-aware semantic validation.

Spec: docs/05_architecture/05.5_world_compiler_and_dsl.md, ADR-0010.

Design rules (vs. the draft schema):
  * No free-string semantics: every mechanic reference is a Registry ID, checked against registry TOML.
  * No Dict[str, Any]: catalysts and peoples are discriminated unions.
  * Catalysts are INITIAL CONDITIONS, never forced outcomes (no trigger/consequence strings).
  * Syntactic validation (this file) is necessary but not sufficient: burn-in against stylized
    facts is the final gate and lives in the engine (docs/09 §3.1).

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


class Strict(BaseModel):
    model_config = ConfigDict(extra="forbid", frozen=True)


# --------------------------------------------------------------------------- world meta
class Calendar(Strict):
    days_per_year: Literal[360, 365] = 360
    months: Annotated[int, Field(ge=1, le=24)] = 12
    seasons: Literal[2, 4] = 4
    epoch_label: str = "Year"


class Horizon(Strict):
    years: Annotated[int, Field(ge=1, le=5000)]
    label: Literal["historical", "speculative"] = "historical"


class WorldMeta(Strict):
    id: Slug
    display_name: Annotated[str, Field(min_length=1, max_length=120)]
    template: Slug
    seed: Annotated[int, Field(ge=0, le=2**63 - 1)]
    calendar: Calendar = Calendar()
    horizon: Horizon
    fidelity_profile: dict[str, Literal["F0", "F1", "F2", "F3"]] = {}


# --------------------------------------------------------------------------- geography
class Climate(Strict):
    zone: Literal["temperate_continental", "temperate_maritime", "mediterranean", "arid", "tropical", "boreal", "urban_temperate"]
    harvest_sigma: float = 0.2
    autocorr: float = 0.35
    spatial_share: float = 0.6
    volcanic_rate: float = 0.004


class Ecology(Strict):
    livestock_kg_per_capita: Annotated[float, Field(ge=0.0, le=2000.0)] = 150.0
    wild_prey_tonnes: Annotated[float, Field(ge=0.0)] = 0.0


class Geography(Strict):
    raster_resolution: Literal[256, 512, 1024, 2048, 4096] = 512
    region_count: Annotated[int, Field(ge=20, le=5000)] = 600
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
    population: Annotated[int, Field(ge=100, le=20_000_000)]
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
    start_day: Annotated[int, Field(ge=0)] = 0


class ClimateShock(Strict):
    kind: Literal["climate_shock"]
    year: Annotated[int, Field(ge=0)]
    severity_sigma: Annotated[float, Field(ge=0.5, le=4.0)]
    duration_years: Annotated[int, Field(ge=1, le=10)] = 1


Catalyst = Annotated[Union[DynasticTension, Artifact, CovertOutbreak, ClimateShock], Field(discriminator="kind")]


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
    registry_version: Annotated[str, Field(pattern=r"^\d+\.\d+\.\d+$")]
    world: WorldMeta
    geography: Geography
    peoples: Annotated[list[People], Field(min_length=1)]
    polities: list[Polity] = []
    systems: dict[str, dict[str, float]] = Field(description="system id -> {param: value}; params bounded by Registry")
    institutions: list[Slug] = []
    technologies: list[Slug] = []
    catalysts: list[Catalyst] = []
    overlays: list[Slug] = []
    narrative: Narrative
    presentation: Presentation


# =========================================================================== Registry-aware validation
CARBONE_RATIO = 0.009  # kg carnivore supported per kg standing prey (R09 §8)


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
        if sid not in m.systems:
            warn(f"world.fidelity_profile.{sid}", "fidelity set for a system not enabled in `systems`")

    # climate params share M3 bounds
    m3 = reg_sys.get("M3.agriculture", {}).get("params", {})
    for p in ("harvest_sigma", "autocorr", "spatial_share", "volcanic_rate"):
        if p in m3:
            lo, hi = m3[p]
            v = getattr(m.geography.climate, p)
            if not (lo <= v <= hi):
                err(f"geography.climate.{p}", f"value {v} outside Registry bounds [{lo}, {hi}]")

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

    # creatures: archetype/variant + energetics (Carbone ratio)
    arche = reg.get("archetypes", {})
    prey_t = humans * m.geography.ecology.livestock_kg_per_capita / 1000.0 + m.geography.ecology.wild_prey_tonnes
    for i, p in enumerate(m.peoples):
        if isinstance(p, CreaturePeople):
            a = arche.get(p.archetype, {})
            if p.variant not in a.get("variants", []):
                err(f"peoples[{i}].variant", f"'{p.variant}' not a Registry variant of {p.archetype}")
            lo, hi = a.get("params", {}).get("mass_kg", [0, float("inf")])
            if not (lo <= p.params.mass_kg <= hi):
                err(f"peoples[{i}].params.mass_kg", f"outside Registry bounds [{lo}, {hi}]")
            mm_lo, mm_hi = reg_sys.get("S4.ecology", {}).get("params", {}).get("metabolism_multiplier", [0.1, 10.0])
            if not (mm_lo <= p.params.metabolism_multiplier <= mm_hi):
                err(f"peoples[{i}].params.metabolism_multiplier", f"outside Registry bounds [{mm_lo}, {mm_hi}]")
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
          f"({len(errors)} errors, {len(issues) - len(errors)} warnings) — next gate: burn-in (engine)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
