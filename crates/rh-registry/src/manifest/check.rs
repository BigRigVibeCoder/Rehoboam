//! Registry-aware validation of a structurally valid manifest: a port of
//! `validate_against_registry` in `specs/manifest/manifest_v0.py`, check for check and in the
//! same order, with the same paths and messages.

use std::collections::BTreeSet;
use std::fmt;

use super::types::{
    ActivityResolution, Artifact, Catalyst, CovertOutbreak, DynasticTension, FocusResolution,
    HorizonLabel, Manifest, People, Wager,
};
use crate::pyfmt::{float_repr, int_grouped, str_list_repr};
use crate::registry::{Bounds, Number, Registry};

/// Carbone & Gittleman (2002): kg of carnivore supported per kg of standing prey (R12 §10).
pub const CARBONE_RATIO: f64 = 0.009;
/// The heaviest species in Carbone & Gittleman's data; above it the ratio is extrapolated.
pub const CARBONE_MAX_MASS_KG: f64 = 310.0;

/// How serious an issue is: an error makes the manifest invalid; a warning does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// The manifest is invalid.
    Error,
    /// Worth knowing; the manifest stays valid.
    Warning,
}

/// One Registry-aware finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// Error or warning.
    pub level: Level,
    /// Dotted path into the manifest, as the reference prints it.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.level {
            Level::Error => "ERROR",
            Level::Warning => "WARNING",
        };
        write!(f, "[{level}] {}: {}", self.path, self.message)
    }
}

#[derive(Default)]
struct Out(Vec<Issue>);

impl Out {
    fn err(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.0.push(Issue {
            level: Level::Error,
            path: path.into(),
            message: message.into(),
        });
    }
    fn warn(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.0.push(Issue {
            level: Level::Warning,
            path: path.into(),
            message: message.into(),
        });
    }
}

fn bounds_text(b: &Bounds) -> String {
    format!("[{}, {}]", b.lo, b.hi)
}

/// Validates a manifest against the Registry; returns every issue in the reference's order.
#[must_use]
pub fn validate_against_registry(m: &Manifest, reg: &Registry) -> Vec<Issue> {
    let mut out = Out::default();
    if m.registry_version != reg.registry_version {
        out.warn(
            "registry_version",
            format!(
                "manifest pins {}, registry is {}",
                m.registry_version, reg.registry_version
            ),
        );
    }
    let tpl = template_and_horizon(m, reg, &mut out);
    systems_and_fidelity(m, reg, tpl, &mut out);
    climate_and_vocabularies(m, reg, &mut out);
    polities(m, reg, &mut out);
    let humans = human_population(m);
    if humans > 1_000_000 {
        out.warn(
            "peoples",
            format!(
                "{} persons: above the 1M target range (ADR-0028); expect batch-speed runs (docs/08)",
                int_grouped(humans)
            ),
        );
    }
    activity_resolution(m, humans, &mut out);
    cast_and_agents(m, reg, humans, &mut out);
    catalysts(m, humans, &mut out);
    creatures(m, reg, humans, &mut out);
    out.0
}

fn human_population(m: &Manifest) -> i128 {
    m.peoples
        .iter()
        .map(|p| match p {
            People::Human(h) => i128::from(h.population),
            People::Creature(_) => 0,
        })
        .sum()
}

#[allow(
    clippy::cast_precision_loss,
    reason = "Python mixes int and float the same way"
)]
fn as_f64(n: i128) -> f64 {
    n as f64
}

fn template_and_horizon<'r>(
    m: &Manifest,
    reg: &'r Registry,
    out: &mut Out,
) -> Option<&'r crate::registry::Template> {
    let w = &m.world;
    let Some(tpl) = reg.templates.get(&w.template) else {
        out.err(
            "world.template",
            format!("unknown template '{}'", w.template),
        );
        return None;
    };
    let [lo, hi] = tpl.horizon_years;
    if w.horizon.years > hi && w.horizon.label != HorizonLabel::Speculative {
        out.err(
            "world.horizon",
            format!(
                "template '{}' supports ≤{hi} historical years; set horizon.label='speculative' (and declare a \
                 tech/institution trajectory) for {}",
                w.template, w.horizon.years
            ),
        );
    }
    if w.horizon.years < lo {
        out.warn(
            "world.horizon",
            format!(
                "horizon {}y shorter than template minimum {lo}y",
                w.horizon.years
            ),
        );
    }
    Some(tpl)
}

fn systems_and_fidelity(
    m: &Manifest,
    reg: &Registry,
    tpl: Option<&crate::registry::Template>,
    out: &mut Out,
) {
    for (sid, params) in m.systems.iter() {
        let Some(def) = reg.systems.get(sid) else {
            out.err(format!("systems.{sid}"), "unknown system");
            continue;
        };
        for (p, &v) in params.iter() {
            match def.params.get(p) {
                None => out.err(format!("systems.{sid}.{p}"), "unknown parameter"),
                Some(b) if !b.contains(v) => out.err(
                    format!("systems.{sid}.{p}"),
                    format!(
                        "value {} outside Registry bounds {}",
                        float_repr(v),
                        bounds_text(b)
                    ),
                ),
                Some(_) => {}
            }
        }
    }
    for (sid, lvl) in m.world.fidelity_profile.iter() {
        let path = format!("world.fidelity_profile.{sid}");
        match reg.systems.get(sid) {
            None => out.err(&path, "unknown system"),
            Some(def) if !def.fidelity_levels.contains(lvl) => out.err(
                &path,
                format!(
                    "fidelity {lvl} not implemented (available: {})",
                    str_list_repr(&def.fidelity_levels)
                ),
            ),
            Some(_) => {}
        }
        let default = tpl.is_some_and(|t| t.default_systems.iter().any(|s| s == sid));
        if !m.systems.contains_key(sid) && !default {
            out.warn(
                &path,
                "fidelity set for a system that is neither a template default nor in `systems`",
            );
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "a check-for-check port of the reference, kept in its order"
)]
fn climate_and_vocabularies(m: &Manifest, reg: &Registry, out: &mut Out) {
    let c = &m.geography.climate;
    if let Some(m3) = reg.systems.get("M3.agriculture") {
        for (p, v) in [
            ("harvest_sigma", c.harvest_sigma),
            ("autocorr", c.autocorr),
            ("spatial_share", c.spatial_share),
            ("volcanic_rate", c.volcanic_rate),
        ] {
            if let Some(b) = m3.params.get(p)
                && !b.contains(v)
            {
                out.err(
                    format!("geography.climate.{p}"),
                    format!(
                        "value {} outside Registry bounds {}",
                        float_repr(v),
                        bounds_text(b)
                    ),
                );
            }
        }
    }
    if !reg
        .ids("heightmap_templates")
        .contains(m.geography.heightmap_template.as_str())
    {
        out.err(
            "geography.heightmap_template",
            format!("'{}' not in Registry", m.geography.heightmap_template),
        );
    }
    if !reg.ids("climate_zones").contains(c.zone.as_str()) {
        out.err(
            "geography.climate.zone",
            format!("'{}' not in Registry", c.zone),
        );
    }
    let (values, doctrines) = (reg.ids("values"), reg.ids("doctrine_tags"));
    for (i, p) in m.peoples.iter().enumerate() {
        let People::Human(h) = p else { continue };
        for (j, cu) in h.cultures.iter().enumerate() {
            for k in cu.value_bias.keys() {
                if !values.contains(k) {
                    out.err(
                        format!("peoples[{i}].cultures[{j}].value_bias.{k}"),
                        "not a Registry value",
                    );
                }
            }
        }
        for (j, fa) in h.faiths.iter().enumerate() {
            for d in &fa.doctrine_tags {
                if !doctrines.contains(d.as_str()) {
                    out.err(
                        format!("peoples[{i}].faiths[{j}].doctrine_tags"),
                        format!("'{d}' not in Registry"),
                    );
                }
            }
        }
    }
    if let Some(t) = &m.world.trajectory {
        for (kind, items) in [
            ("technologies", &t.technologies),
            ("institutions", &t.institutions),
        ] {
            let allowed = reg.ids(kind);
            for (j, ms) in items.iter().enumerate() {
                if !allowed.contains(ms.id.as_str()) {
                    out.err(
                        format!("world.trajectory.{kind}[{j}]"),
                        format!("'{}' not in Registry", ms.id),
                    );
                }
                if ms.by_year > m.world.horizon.years {
                    out.warn(
                        format!("world.trajectory.{kind}[{j}]"),
                        format!(
                            "milestone year {} beyond horizon {}",
                            ms.by_year, m.world.horizon.years
                        ),
                    );
                }
            }
        }
    }
    let style = std::slice::from_ref(&m.narrative.default_style_card);
    let vocab: [(&str, &[String], &str); 5] = [
        ("institutions", &m.institutions, "institutions"),
        ("technologies", &m.technologies, "technologies"),
        ("overlays", &m.overlays, "overlays"),
        (
            "narrative.pattern_packs",
            &m.narrative.pattern_packs,
            "pattern_packs",
        ),
        ("narrative.default_style_card", style, "style_cards"),
    ];
    for (path, vals, section) in vocab {
        let allowed = reg.ids(section);
        for v in vals {
            if !allowed.contains(v.as_str()) {
                out.err(path, format!("'{v}' not in Registry"));
            }
        }
    }
}

fn polities(m: &Manifest, reg: &Registry, out: &mut Out) {
    let (roles, govs, laws) = (
        reg.ids("roles"),
        reg.ids("governments"),
        reg.ids("succession_laws"),
    );
    for (i, pol) in m.polities.iter().enumerate() {
        if !govs.contains(pol.government.as_str()) {
            out.err(
                format!("polities[{i}].government"),
                format!("'{}' not in Registry", pol.government),
            );
        }
        if !laws.contains(pol.succession_law.as_str()) {
            out.err(
                format!("polities[{i}].succession_law"),
                format!("'{}' not in Registry", pol.succession_law),
            );
        }
        for r in pol.titles.keys() {
            if !roles.contains(r) {
                out.err(format!("polities[{i}].titles.{r}"), "unknown abstract role");
            }
        }
        for needed in ["authority", "successor"] {
            if !pol.titles.contains_key(needed) {
                out.err(
                    format!("polities[{i}].titles"),
                    format!("missing required role '{needed}'"),
                );
            }
        }
    }
}

fn activity_resolution(m: &Manifest, humans: i128, out: &mut Out) {
    let res = m.world.activity_resolution;
    let h = as_f64(humans);
    if res == Some(ActivityResolution::Minutes) && humans > 1_000 {
        let lo_h = h / 1000.0 * 5.0 / 3600.0;
        out.warn(
            "world.activity_resolution",
            format!(
                "'minutes' for {} persons: about {lo_h:.2} h per sim-year with simple activity updates, up to {:.0} h \
                 with Sims-like agents; a batch run (ADR-0031)",
                int_grouped(humans),
                lo_h * 100.0
            ),
        );
    }
    if res == Some(ActivityResolution::Blocks) && humans > 100_000 {
        out.warn(
            "world.activity_resolution",
            format!(
                "'blocks' for {} persons: about {:.1} h per simulated millennium, above PERF-1; consider focus \
                 regions (ADR-0031)",
                int_grouped(humans),
                h / 1000.0 * 0.06 * 1000.0 / 3600.0
            ),
        );
    }
    let mut seen = BTreeSet::new();
    for (i, f) in m.world.focus.iter().enumerate() {
        if seen.contains(f.settlement.as_str()) {
            out.err(
                format!("world.focus[{i}]"),
                format!("duplicate focus settlement '{}'", f.settlement),
            );
        }
        seen.insert(f.settlement.as_str());
        if res == Some(ActivityResolution::Minutes)
            || (res == Some(ActivityResolution::Blocks)
                && f.activity_resolution == FocusResolution::Blocks)
        {
            out.warn(
                format!("world.focus[{i}]"),
                "focus resolution is not finer than the world's; it has no effect",
            );
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "a check-for-check port of the reference, kept in its order"
)]
fn cast_and_agents(m: &Manifest, reg: &Registry, humans: i128, out: &mut Out) {
    let cast_ids: BTreeSet<&str> = m.cast.iter().map(|c| c.id.as_str()).collect();
    let hh_ids: BTreeSet<&str> = m.households.iter().map(|h| h.id.as_str()).collect();
    if cast_ids.len() != m.cast.len() {
        out.err("cast", "duplicate cast ids");
    }
    let named: i128 = i128::try_from(m.cast.len()).unwrap_or(i128::MAX)
        + m.households
            .iter()
            .map(|h| {
                i128::from(h.children.sons)
                    + i128::from(h.children.daughters)
                    + i128::from(h.servants)
            })
            .sum::<i128>();
    if named > humans {
        out.err(
            "cast",
            format!("cast, children and servants ({named}) exceed the human population ({humans})"),
        );
    }
    for (i, c) in m.cast.iter().enumerate() {
        if let Some(hh) = &c.household
            && !hh_ids.contains(hh.as_str())
        {
            out.err(
                format!("cast[{i}].household"),
                format!("unknown household '{hh}'"),
            );
        }
        for k in c.initial.keys() {
            match reg.attributes.get(k) {
                None => out.err(format!("cast[{i}].initial.{k}"), "not a Registry attribute"),
                Some(a) if a.args.len() > 2 => out.err(
                    format!("cast[{i}].initial.{k}"),
                    "only attributes of the person (optionally towards one deity) can be set",
                ),
                Some(_) => {}
            }
        }
    }
    let goods = reg.ids("goods");
    for (i, h) in m.households.iter().enumerate() {
        if !cast_ids.contains(h.head.as_str()) {
            out.err(
                format!("households[{i}].head"),
                format!("'{}' is not a cast member", h.head),
            );
        }
        for g in h.holdings.keys() {
            if !goods.contains(g) {
                out.err(
                    format!("households[{i}].holdings.{g}"),
                    "not a Registry good",
                );
            }
        }
    }
    let kinds = reg.ids("off_map_kinds");
    for (i, o) in m.off_map_actors.iter().enumerate() {
        if !kinds.contains(o.kind.as_str()) {
            out.err(
                format!("off_map_actors[{i}].kind"),
                format!("'{}' not in Registry", o.kind),
            );
        }
    }
    for (i, a) in m.agents.iter().enumerate() {
        if !reg.archetypes.contains_key(&a.archetype) {
            out.err(
                format!("agents[{i}].archetype"),
                format!("unknown archetype '{}'", a.archetype),
            );
        }
        for v in &a.verbs {
            if !reg.verbs.contains_key(v) {
                out.err(
                    format!("agents[{i}].verbs"),
                    format!("verb '{v}' not in Registry"),
                );
            }
        }
    }
    for (i, c) in m.catalysts.iter().enumerate() {
        let Catalyst::Wager(Wager {
            agent,
            protagonist,
            stages,
        }) = c
        else {
            continue;
        };
        // Python's `{a.id: a for a in m.agents}` keeps the last agent with a given id.
        let held = m.agents.iter().rev().find(|a| &a.id == agent);
        if held.is_none() {
            out.err(
                format!("catalysts[{i}].agent"),
                format!("unknown agent '{agent}'"),
            );
        }
        if !cast_ids.contains(protagonist.as_str()) {
            out.err(
                format!("catalysts[{i}].protagonist"),
                format!("'{protagonist}' is not a cast member"),
            );
        }
        let verbs: BTreeSet<&str> = held
            .map(|a| a.verbs.iter().map(String::as_str).collect())
            .unwrap_or_default();
        for (j, s) in stages.iter().enumerate() {
            let extra: BTreeSet<&str> = s
                .allow
                .iter()
                .map(String::as_str)
                .filter(|v| !verbs.contains(v))
                .collect();
            if !extra.is_empty() {
                let extra: Vec<&str> = extra.into_iter().collect();
                out.err(
                    format!("catalysts[{i}].stages[{j}].allow"),
                    format!("verbs not held by the agent: {}", str_list_repr(&extra)),
                );
            }
        }
    }
}

const TESTING_TECHNOLOGIES: [&str; 2] = ["blood_assay", "pcr_testing"];

fn catalysts(m: &Manifest, humans: i128, out: &mut Out) {
    let polity_ids: BTreeSet<&str> = m.polities.iter().map(|p| p.id.as_str()).collect();
    let testing = m
        .technologies
        .iter()
        .any(|t| TESTING_TECHNOLOGIES.contains(&t.as_str()));
    for (i, c) in m.catalysts.iter().enumerate() {
        match c {
            Catalyst::DynasticTension(DynasticTension { polity, .. })
            | Catalyst::Artifact(Artifact { polity, .. })
                if !polity_ids.contains(polity.as_str()) =>
            {
                out.err(
                    format!("catalysts[{i}].polity"),
                    format!("unknown polity '{polity}'"),
                );
            }
            _ => {}
        }
        if let Catalyst::Artifact(Artifact { effects, .. }) = c {
            for (j, e) in effects.iter().enumerate() {
                if !m.overlays.contains(&e.overlay) {
                    out.err(
                        format!("catalysts[{i}].effects[{j}]"),
                        format!("overlay '{}' not enabled in manifest.overlays", e.overlay),
                    );
                }
            }
        }
        if let Catalyst::CovertOutbreak(CovertOutbreak { seed_infected, .. }) = c {
            if !m.systems.contains_key("M5.contagion") {
                out.err(
                    format!("catalysts[{i}]"),
                    "covert_outbreak requires system M5.contagion",
                );
            }
            if !testing {
                out.warn(
                    format!("catalysts[{i}]"),
                    "covert outbreak without a testing technology: detection only via behavioural tells",
                );
            }
            if i128::from(*seed_infected) > (humans.div_euclid(100)).max(1) {
                out.warn(
                    format!("catalysts[{i}].seed_infected"),
                    "seed >1% of population: the 'covert' phase may be skipped",
                );
            }
            if testing {
                let m5 = m.systems.get("M5.contagion");
                for p in ["test_sensitivity", "test_specificity"] {
                    if !m5.is_some_and(|m5| m5.contains_key(p)) {
                        out.err(
                            format!("systems.M5.contagion.{p}"),
                            "a testing technology is declared: set test_sensitivity and test_specificity explicitly",
                        );
                    }
                }
            }
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "a check-for-check port of the reference, kept in its order"
)]
fn creatures(m: &Manifest, reg: &Registry, humans: i128, out: &mut Out) {
    let any_creature = m.peoples.iter().any(|p| matches!(p, People::Creature(_)));
    if any_creature && !m.systems.contains_key("S4.ecology") {
        out.err(
            "systems",
            "creature peoples require system S4.ecology (prey, grazing, predator energetics)",
        );
    }
    let eco = &m.geography.ecology;
    let prey_t = as_f64(humans) * eco.livestock_kg_per_capita / 1000.0 + eco.wild_prey_tonnes;
    let arche = reg.archetypes.get("apex_predator");
    let mass_bounds = arche
        .and_then(|a| a.params.get("mass_kg"))
        .copied()
        .unwrap_or(Bounds {
            lo: Number::Int(0),
            hi: Number::Float(f64::INFINITY),
        });
    let mm_bounds = reg
        .systems
        .get("S4.ecology")
        .and_then(|s| s.params.get("metabolism_multiplier"))
        .copied()
        .unwrap_or(Bounds {
            lo: Number::Float(0.1),
            hi: Number::Float(10.0),
        });
    for (i, p) in m.peoples.iter().enumerate() {
        let People::Creature(c) = p else { continue };
        let variants = arche.map(|a| a.variants.as_slice()).unwrap_or_default();
        if !variants.contains(&c.variant) {
            out.err(
                format!("peoples[{i}].variant"),
                format!("'{}' not a Registry variant of apex_predator", c.variant),
            );
        }
        let caps = arche.map(|a| a.capabilities.as_slice()).unwrap_or_default();
        for (cap, set) in [("flying", c.params.flying), ("bondable", c.params.bondable)] {
            if set && !caps.iter().any(|x| x == cap) {
                out.err(
                    format!("peoples[{i}].params.{cap}"),
                    "capability not permitted by archetype apex_predator",
                );
            }
        }
        if !mass_bounds.contains(c.params.mass_kg) {
            out.err(
                format!("peoples[{i}].params.mass_kg"),
                format!("outside Registry bounds {}", bounds_text(&mass_bounds)),
            );
        }
        let mm = c.params.metabolism_multiplier;
        if !mm_bounds.contains(mm) {
            out.err(
                format!("peoples[{i}].params.metabolism_multiplier"),
                format!("outside Registry bounds {}", bounds_text(&mm_bounds)),
            );
        }
        if c.params.mass_kg > CARBONE_MAX_MASS_KG {
            out.warn(
                format!("peoples[{i}].params.mass_kg"),
                format!(
                    "{:.0} kg is beyond the 0.14–310 kg range behind the Carbone ratio; the energetics bound is \
                     extrapolated (metabolism_multiplier = field metabolic rate relative to a mammalian carnivore of \
                     equal mass)",
                    c.params.mass_kg
                ),
            );
        }
        let predator_t = as_f64(i128::from(c.population)) * c.params.mass_kg / 1000.0;
        // Python's max(mm, 1e-9) returns mm unless 1e-9 is greater (so NaN stays NaN).
        let supportable_t = CARBONE_RATIO * prey_t / if 1e-9 > mm { 1e-9 } else { mm };
        if predator_t > supportable_t {
            out.err(
                format!("peoples[{i}]"),
                format!(
                    "energetics: {}×{:.0} kg = {predator_t:.0} t of predator, but standing prey {prey_t:.0} t supports \
                     only {supportable_t:.0} t at metabolism ×{} (Carbone ratio {}). Lower population, \
                     lower metabolism_multiplier, or declare wild_prey_tonnes.",
                    c.population,
                    c.params.mass_kg,
                    float_repr(mm),
                    float_repr(CARBONE_RATIO)
                ),
            );
        }
    }
}
