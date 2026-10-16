//! The Mechanics Registry v0, typed (SPR-004 T-01).
//!
//! `specs/manifest/registry_v0.toml` is the closed vocabulary that manifests may reference
//! (ADR-0010). The sections the validators read are typed structures here, with unknown fields
//! refused; every other section loads through one generic path ([`Registry::sections`]), so the
//! Registry can grow (a new system, a new vocabulary) without a loader change. Systems are a
//! keyed map: no system id is hard-coded. Loading also checks the Registry's own consistency
//! (bounds ordered and finite, fidelity levels known, ids unique, act classes resolved), for
//! generic sections as far as their shape allows.
//!
//! Maps are `BTreeMap` (ordered by key): plane A never iterates a hash map (GOV-003 §2).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::pyfmt::float_repr;

/// A number as the Registry writes it. Integers and floats are kept apart so that messages
/// print bounds as the Python reference does (`[3, 12]`, not `[3.0, 12.0]`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Number {
    /// A TOML integer.
    Int(i64),
    /// A TOML float.
    Float(f64),
}

impl Number {
    /// The value as a float, for comparisons (Python compares int and float by value).
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "Registry integers are small; Python compares them as floats too"
    )]
    pub fn as_f64(self) -> f64 {
        match self {
            Number::Int(i) => i as f64,
            Number::Float(f) => f,
        }
    }
}

impl fmt::Display for Number {
    /// Python's `str()` of the number.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Number::Int(i) => write!(f, "{i}"),
            Number::Float(x) => f.write_str(&float_repr(*x)),
        }
    }
}

/// Inclusive parameter bounds `[lo, hi]`, written as a two-element TOML array.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Number>", into = "[Number; 2]")]
pub struct Bounds {
    /// Lowest allowed value.
    pub lo: Number,
    /// Highest allowed value.
    pub hi: Number,
}

impl TryFrom<Vec<Number>> for Bounds {
    type Error = String;

    /// Exactly two elements: a third is an error, never dropped (SPR-004 DEF-5).
    fn try_from(v: Vec<Number>) -> Result<Self, String> {
        match v.as_slice() {
            [lo, hi] => Ok(Bounds { lo: *lo, hi: *hi }),
            _ => Err(format!(
                "bounds must be [lo, hi], found {} elements",
                v.len()
            )),
        }
    }
}

impl From<Bounds> for [Number; 2] {
    fn from(b: Bounds) -> Self {
        [b.lo, b.hi]
    }
}

impl Bounds {
    /// Whether `v` lies inside the bounds; NaN never does (as `lo <= v <= hi` in Python).
    #[must_use]
    pub fn contains(&self, v: f64) -> bool {
        self.lo.as_f64() <= v && v <= self.hi.as_f64()
    }
}

/// Deserialises a two-element array, refusing any other length (SPR-004 DEF-5).
fn exactly_two<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[i64; 2], D::Error> {
    let v: Vec<i64> = Vec::deserialize(d)?;
    match v.as_slice() {
        [lo, hi] => Ok([*lo, *hi]),
        _ => Err(serde::de::Error::custom(format!(
            "expected [min, max], found {} elements",
            v.len()
        ))),
    }
}

/// A list of identifiers: `[section] ids = [...]`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdList {
    /// The identifiers, in Registry order.
    pub ids: Vec<String>,
}

/// One simulation system: `[systems."M2.demography"]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemDef {
    /// Implemented fidelity levels (`F0`..`F3`).
    pub fidelity_levels: Vec<String>,
    /// Hazards that sifting patterns may read (`completion_hazard`, 05.4 §2.3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hazards: Vec<String>,
    /// Parameters a manifest may set, each with its bounds.
    #[serde(default)]
    pub params: BTreeMap<String, Bounds>,
}

/// An archetype of being: `[archetypes.apex_predator]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archetype {
    /// The ledger its members live in (`persons`, `creatures`, `agents`).
    pub ledger: String,
    /// Variants a manifest may name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    /// Capabilities its members may have.
    pub capabilities: Vec<String>,
    /// Bounded creature parameters.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, Bounds>,
}

/// One item of a consumption basket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasketItem {
    /// Item identifier.
    pub id: String,
    /// Quantity per basket-year.
    pub qty: f64,
    /// Unit of `qty`.
    pub unit: String,
    /// The Registry good it is made from.
    pub good: String,
    /// Kilocalories per unit.
    pub kcal: f64,
    /// Grams of protein per unit.
    pub protein_g: f64,
    /// Reference price, grams of silver per unit.
    pub ref_price: f64,
    /// The substitute used in Mediterranean climate zones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mediterranean: Option<String>,
    /// The quantity used in Mediterranean climate zones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qty_mediterranean: Option<f64>,
}

/// A consumption basket: `[baskets.allen2001]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Basket {
    /// Baskets per family.
    pub family_baskets: f64,
    /// Rent as a markup on the family's baskets.
    pub rent_markup: f64,
    /// Working days in the welfare-ratio convention.
    pub days: i64,
    /// The basket's items.
    pub items: Vec<BasketItem>,
    /// Substitutes named by `mediterranean`.
    #[serde(default)]
    pub substitutes: Vec<BasketItem>,
}

/// A world template: `[templates.feudal_realm]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    /// Systems enabled by default.
    pub default_systems: Vec<String>,
    /// Supported historical horizon in years, `[min, max]` (exactly two elements).
    #[serde(deserialize_with = "exactly_two")]
    pub horizon_years: [i64; 2],
    /// Whether the template starts in the modern era.
    pub modern: bool,
}

/// An era-state variable (ADR-0027).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EraStateVar {
    /// The one system that owns it.
    pub owner: String,
    /// What it is indexed by (`region`, `polity`, `region_class`).
    pub scope: String,
    /// Its fixed-point type (`q8_8`, `q16_16`).
    #[serde(rename = "type")]
    pub ty: String,
    /// An extra index, such as `technology`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per: Option<String>,
}

/// Era labels: predicates over era state, for presentation and validation only.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eras {
    /// The era labels, in order.
    pub ids: Vec<String>,
    /// The last era this engine build supports.
    pub frontier: String,
}

/// An anchored attribute readable by sifting patterns (`attr(args)@eN`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attribute {
    /// `q8_8`, `int`, `bool` or `enum`.
    #[serde(rename = "type")]
    pub ty: String,
    /// Argument roles.
    pub args: Vec<String>,
    /// Enum values (only for `enum`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

/// A concrete verb that systems emit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verb {
    /// Its act classes.
    pub act_classes: Vec<String>,
    /// The system (or subsystem) that emits it.
    pub emitted_by: String,
    /// Whether it is an aggregate event.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub aggregate: bool,
    /// Act classes of past events it may reference (contingent unlocking).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
}

/// The Mechanics Registry: every section of `registry_v0.toml`.
///
/// Typed sections other than `registry_version` default to empty, as the Python reference
/// treats a missing section as empty. Any other top-level section lands in `sections`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(
    missing_docs,
    reason = "each field is the Registry section of the same name"
)]
pub struct Registry {
    pub registry_version: String,
    #[serde(default)]
    pub systems: BTreeMap<String, SystemDef>,
    #[serde(default)]
    pub archetypes: BTreeMap<String, Archetype>,
    #[serde(default)]
    pub governments: IdList,
    #[serde(default)]
    pub succession_laws: IdList,
    #[serde(default)]
    pub institutions: IdList,
    #[serde(default)]
    pub goods: IdList,
    #[serde(default)]
    pub baskets: BTreeMap<String, Basket>,
    #[serde(default)]
    pub off_map_kinds: IdList,
    #[serde(default)]
    pub inheritance_rules: IdList,
    #[serde(default)]
    pub technologies: IdList,
    #[serde(default)]
    pub act_classes: IdList,
    #[serde(default)]
    pub roles: IdList,
    #[serde(default)]
    pub channels: IdList,
    #[serde(default)]
    pub overlays: IdList,
    #[serde(default)]
    pub pattern_packs: IdList,
    #[serde(default)]
    pub style_cards: IdList,
    #[serde(default)]
    pub templates: BTreeMap<String, Template>,
    #[serde(default)]
    pub values: IdList,
    #[serde(default)]
    pub doctrine_tags: IdList,
    #[serde(default)]
    pub heightmap_templates: IdList,
    #[serde(default)]
    pub climate_zones: IdList,
    #[serde(default)]
    pub era_state: BTreeMap<String, EraStateVar>,
    #[serde(default)]
    pub eras: Eras,
    #[serde(default)]
    pub attributes: BTreeMap<String, Attribute>,
    #[serde(default)]
    pub verbs: BTreeMap<String, Verb>,
    /// Every other section, by name, loaded generically: an id list (`ids = [...]`) or a table
    /// of keyed entries. Plane-C presentation tables (`place_types`, `visual_*`) live here too;
    /// mechanics never read them.
    #[serde(flatten)]
    pub sections: BTreeMap<String, toml::Value>,
}

/// Why a Registry could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// The file could not be read.
    Io {
        /// The path.
        path: String,
        /// The OS error.
        message: String,
    },
    /// The TOML does not parse, or does not have the Registry's shape.
    Parse(String),
    /// The Registry parses but contradicts itself; one entry per problem.
    Inconsistent(Vec<String>),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Io { path, message } => {
                write!(f, "cannot read Registry {path}: {message}")
            }
            RegistryError::Parse(m) => write!(f, "Registry does not load: {m}"),
            RegistryError::Inconsistent(problems) => {
                write!(f, "Registry is inconsistent ({} problems):", problems.len())?;
                for p in problems {
                    write!(f, "\n  {p}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// The fidelity levels a system may declare (05.1 §3).
pub const FIDELITY_LEVELS: [&str; 4] = ["F0", "F1", "F2", "F3"];
/// The attribute types sifting understands (`registry_v0.toml` `[attributes]`).
pub const ATTRIBUTE_TYPES: [&str; 4] = ["q8_8", "int", "bool", "enum"];
/// The fixed-point types an era-state variable may have.
pub const ERA_STATE_TYPES: [&str; 2] = ["q8_8", "q16_16"];

impl Registry {
    /// Parses and checks a Registry from TOML text.
    ///
    /// # Errors
    /// [`RegistryError::Parse`] for bad TOML or an unknown section or field;
    /// [`RegistryError::Inconsistent`] when [`Registry::problems`] finds anything.
    pub fn from_toml(text: &str) -> Result<Self, RegistryError> {
        let reg: Registry =
            toml::from_str(text).map_err(|e| RegistryError::Parse(e.to_string()))?;
        let problems = reg.problems();
        if problems.is_empty() {
            Ok(reg)
        } else {
            Err(RegistryError::Inconsistent(problems))
        }
    }

    /// Reads, parses and checks a Registry file.
    ///
    /// # Errors
    /// As [`Registry::from_toml`], plus [`RegistryError::Io`].
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        let text = std::fs::read_to_string(path).map_err(|e| RegistryError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
        Self::from_toml(&text)
    }

    /// Serialises the Registry back to TOML (comments are not kept).
    ///
    /// # Errors
    /// A value TOML cannot represent (none in a loaded Registry).
    pub fn to_toml(&self) -> Result<String, RegistryError> {
        toml::to_string(self).map_err(|e| RegistryError::Parse(e.to_string()))
    }

    /// The identifiers of a section, as the Python reference's `_ids` gives them: the `ids`
    /// list of an id-list section, or the keys of a keyed section. Unknown names are empty.
    #[must_use]
    pub fn ids(&self, section: &str) -> BTreeSet<&str> {
        fn keys<V>(m: &BTreeMap<String, V>) -> BTreeSet<&str> {
            m.keys().map(String::as_str).collect()
        }
        match section {
            "systems" => keys(&self.systems),
            "archetypes" => keys(&self.archetypes),
            "baskets" => keys(&self.baskets),
            "templates" => keys(&self.templates),
            "era_state" => keys(&self.era_state),
            "attributes" => keys(&self.attributes),
            "verbs" => keys(&self.verbs),
            "eras" => self.eras.ids.iter().map(String::as_str).collect(),
            _ => match self.id_list(section) {
                Some(l) => l.ids.iter().map(String::as_str).collect(),
                None => self.generic_ids(section),
            },
        }
    }

    /// The ids of a generic section: its `ids` array of strings, or its keys.
    fn generic_ids(&self, section: &str) -> BTreeSet<&str> {
        let Some(toml::Value::Table(t)) = self.sections.get(section) else {
            return BTreeSet::new();
        };
        match t.get("ids") {
            Some(toml::Value::Array(a)) => a.iter().filter_map(toml::Value::as_str).collect(),
            _ => t.keys().map(String::as_str).collect(),
        }
    }

    fn id_list(&self, section: &str) -> Option<&IdList> {
        Some(match section {
            "governments" => &self.governments,
            "succession_laws" => &self.succession_laws,
            "institutions" => &self.institutions,
            "goods" => &self.goods,
            "off_map_kinds" => &self.off_map_kinds,
            "inheritance_rules" => &self.inheritance_rules,
            "technologies" => &self.technologies,
            "act_classes" => &self.act_classes,
            "roles" => &self.roles,
            "channels" => &self.channels,
            "overlays" => &self.overlays,
            "pattern_packs" => &self.pattern_packs,
            "style_cards" => &self.style_cards,
            "values" => &self.values,
            "doctrine_tags" => &self.doctrine_tags,
            "heightmap_templates" => &self.heightmap_templates,
            "climate_zones" => &self.climate_zones,
            _ => return None,
        })
    }

    /// The id-list sections, by name, in a fixed order.
    pub const ID_LIST_SECTIONS: [&'static str; 17] = [
        "governments",
        "succession_laws",
        "institutions",
        "goods",
        "off_map_kinds",
        "inheritance_rules",
        "technologies",
        "act_classes",
        "roles",
        "channels",
        "overlays",
        "pattern_packs",
        "style_cards",
        "values",
        "doctrine_tags",
        "heightmap_templates",
        "climate_zones",
    ];

    /// Internal consistency problems, in a stable order; empty for a sound Registry.
    ///
    /// Checked: the version is `MAJOR.MINOR.PATCH`; every bound is finite with `lo <= hi`;
    /// fidelity levels are `F0`..`F3`, non-empty and unique; id lists have no duplicates;
    /// template horizons are ordered; basket goods, verb act classes and the era frontier
    /// resolve; attribute and era-state types are known, and only enums carry values.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !is_semver(&self.registry_version) {
            out.push(format!(
                "registry_version '{}' is not MAJOR.MINOR.PATCH",
                self.registry_version
            ));
        }
        let mut bounds = |path: String, b: &Bounds| {
            let (lo, hi) = (b.lo.as_f64(), b.hi.as_f64());
            if !lo.is_finite() || !hi.is_finite() || lo > hi {
                out.push(format!(
                    "{path}: bounds [{}, {}] must be finite with lo <= hi",
                    b.lo, b.hi
                ));
            }
        };
        for (sid, s) in &self.systems {
            for (p, b) in &s.params {
                bounds(format!("systems.{sid}.params.{p}"), b);
            }
        }
        for (aid, a) in &self.archetypes {
            for (p, b) in &a.params {
                bounds(format!("archetypes.{aid}.params.{p}"), b);
            }
        }
        for (sid, s) in &self.systems {
            if s.fidelity_levels.is_empty() {
                out.push(format!("systems.{sid}: no fidelity levels"));
            }
            for l in &s.fidelity_levels {
                if !FIDELITY_LEVELS.contains(&l.as_str()) {
                    out.push(format!("systems.{sid}: unknown fidelity level '{l}'"));
                }
            }
            push_duplicates(
                &mut out,
                &format!("systems.{sid}.fidelity_levels"),
                &s.fidelity_levels,
            );
        }
        for name in Self::ID_LIST_SECTIONS {
            if let Some(l) = self.id_list(name) {
                push_duplicates(&mut out, name, &l.ids);
            }
        }
        push_duplicates(&mut out, "eras", &self.eras.ids);
        for (tid, t) in &self.templates {
            let [lo, hi] = t.horizon_years;
            if lo < 1 || lo > hi {
                out.push(format!(
                    "templates.{tid}.horizon_years: [{lo}, {hi}] must have 1 <= min <= max"
                ));
            }
        }
        self.reference_problems(&mut out);
        out
    }

    /// Cross-section references and type names (part of [`Registry::problems`]).
    fn reference_problems(&self, out: &mut Vec<String>) {
        let goods = self.ids("goods");
        for (bid, b) in &self.baskets {
            for item in b.items.iter().chain(&b.substitutes) {
                if !goods.contains(item.good.as_str()) {
                    out.push(format!(
                        "baskets.{bid}.{}: good '{}' not in [goods]",
                        item.id, item.good
                    ));
                }
            }
            for item in &b.items {
                if let Some(sub) = &item.mediterranean
                    && !b.substitutes.iter().any(|s| &s.id == sub)
                {
                    out.push(format!(
                        "baskets.{bid}.{}: substitute '{sub}' not in substitutes",
                        item.id
                    ));
                }
            }
        }
        let acts = self.ids("act_classes");
        self.generic_problems(&acts, out);
        for (vid, v) in &self.verbs {
            for a in v.act_classes.iter().chain(&v.references) {
                if !acts.contains(a.as_str()) {
                    out.push(format!("verbs.{vid}: act class '{a}' not in [act_classes]"));
                }
            }
        }
        if !self.eras.ids.is_empty() && !self.eras.ids.contains(&self.eras.frontier) {
            out.push(format!(
                "eras.frontier '{}' not in eras.ids",
                self.eras.frontier
            ));
        }
        for (aid, a) in &self.attributes {
            if !ATTRIBUTE_TYPES.contains(&a.ty.as_str()) {
                out.push(format!("attributes.{aid}: unknown type '{}'", a.ty));
            }
            if (a.ty == "enum") != a.values.is_some() {
                out.push(format!(
                    "attributes.{aid}: values are required for enum and only for enum"
                ));
            }
        }
        for (eid, e) in &self.era_state {
            if !ERA_STATE_TYPES.contains(&e.ty.as_str()) {
                out.push(format!(
                    "era_state.{eid}: type '{}' is not fixed-point",
                    e.ty
                ));
            }
        }
    }
}

impl Registry {
    /// Shape checks for generic sections: each is a table; an `ids` entry is an array of unique
    /// strings; any `act_classes` in a keyed entry resolves to `[act_classes]`.
    fn generic_problems(&self, acts: &BTreeSet<&str>, out: &mut Vec<String>) {
        for (name, value) in &self.sections {
            let Some(table) = value.as_table() else {
                out.push(format!(
                    "{name}: a top-level Registry entry must be a section (a table)"
                ));
                continue;
            };
            if let Some(ids) = table.get("ids") {
                let strings: Option<Vec<String>> = ids
                    .as_array()
                    .and_then(|a| a.iter().map(|v| v.as_str().map(str::to_owned)).collect());
                match strings {
                    Some(ids) => push_duplicates(out, name, &ids),
                    None => out.push(format!("{name}.ids: must be an array of strings")),
                }
            }
            for (key, entry) in table {
                let Some(classes) = entry.get("act_classes") else {
                    continue;
                };
                for c in classes.as_array().map(Vec::as_slice).unwrap_or_default() {
                    match c.as_str() {
                        Some(c) if acts.contains(c) => {}
                        _ => out.push(format!("{name}.{key}: act class {c} not in [act_classes]")),
                    }
                }
            }
        }
    }
}

fn push_duplicates(out: &mut Vec<String>, what: &str, ids: &[String]) {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.as_str()) {
            out.push(format!("{what}: duplicate id '{id}'"));
        }
    }
}

/// `^[0-9]+\.[0-9]+\.[0-9]+$` (ASCII digits, as the reference since SPR-004 GAP-4).
#[must_use]
pub fn is_semver(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs/manifest/registry_v0.toml")
    }

    #[test]
    fn registry_v0_loads_every_section() {
        let reg = Registry::load(&registry_path()).unwrap();
        assert_eq!(reg.registry_version, "0.1.0");
        assert!(reg.systems.contains_key("M2.demography"));
        assert!(reg.systems.contains_key("S5.daily_life"));
        assert_eq!(
            reg.systems["M8.war"].params["campaign_months"].lo,
            Number::Int(3)
        );
        assert!(
            reg.archetypes["apex_predator"]
                .variants
                .contains(&"dragon".to_owned())
        );
        for name in Registry::ID_LIST_SECTIONS {
            assert!(!reg.ids(name).is_empty(), "section [{name}] is empty");
        }
        assert_eq!(reg.baskets["allen2001"].items.len(), 12);
        assert_eq!(reg.templates["modern_metropolis"].horizon_years, [1, 50]);
        assert_eq!(
            reg.era_state["tech_adoption"].per.as_deref(),
            Some("technology")
        );
        assert_eq!(reg.eras.frontier, "agrarian");
        assert_eq!(
            reg.attributes["test_result"].values.as_ref().map(Vec::len),
            Some(3)
        );
        assert!(reg.verbs["REVENGE"].references.contains(&"harm".to_owned()));
        assert!(reg.verbs["FAMINE_DECLARED"].aggregate);
    }

    /// Sections the loader has no type for load generically, and every system entry loads
    /// through the same keyed map (no hard-coded system ids).
    #[test]
    fn untyped_sections_load_generically() {
        let reg = Registry::load(&registry_path()).unwrap();
        let text = std::fs::read_to_string(registry_path()).unwrap();
        let raw: toml::Table = toml::from_str(&text).unwrap();
        let typed: BTreeSet<&str> = [
            Registry::ID_LIST_SECTIONS.as_slice(),
            &[
                "registry_version",
                "systems",
                "archetypes",
                "baskets",
                "templates",
                "era_state",
                "eras",
                "attributes",
                "verbs",
            ],
        ]
        .concat()
        .into_iter()
        .collect();
        for name in raw.keys() {
            let generic = reg.sections.contains_key(name);
            assert_eq!(generic, !typed.contains(name.as_str()), "section [{name}]");
        }
        let systems: BTreeSet<&String> = raw["systems"].as_table().unwrap().keys().collect();
        assert_eq!(systems, reg.systems.keys().collect());
        let extra = "registry_version = \"0.1.0\"\n[act_classes]\nids = [\"harm\"]\n\
            [new_vocab]\nids = [\"a\", \"b\"]\n[new_table]\nx = { act_classes = [\"harm\"], w = 1.0 }\n";
        let reg = Registry::from_toml(extra).unwrap();
        assert_eq!(reg.ids("new_vocab"), BTreeSet::from(["a", "b"]));
        assert_eq!(reg.ids("new_table"), BTreeSet::from(["x"]));
        assert_eq!(Registry::from_toml(&reg.to_toml().unwrap()).unwrap(), reg);
    }

    /// T-01 acceptance: load, write back, load again, and get the same Registry.
    #[test]
    fn registry_v0_round_trips() {
        let reg = Registry::load(&registry_path()).unwrap();
        let text = reg.to_toml().unwrap();
        let again = Registry::from_toml(&text).unwrap();
        assert_eq!(reg, again);
        assert_eq!(
            again.to_toml().unwrap(),
            text,
            "serialisation is not stable"
        );
    }

    #[test]
    fn every_bound_is_ordered_and_contains_its_ends() {
        let reg = Registry::load(&registry_path()).unwrap();
        for s in reg.systems.values() {
            for b in s.params.values() {
                assert!(b.contains(b.lo.as_f64()) && b.contains(b.hi.as_f64()));
                assert!(!b.contains(f64::NAN));
            }
        }
    }

    #[test]
    fn malformed_generic_sections_and_unknown_fields_are_refused() {
        let bad = "registry_version = \"0.1.0\"\nstray = 1\n[spells]\nids = [\"a\", \"a\"]\n\
            [rites]\nx = { act_classes = [\"bless\"] }\n";
        let Err(RegistryError::Inconsistent(p)) = Registry::from_toml(bad) else {
            panic!("accepted")
        };
        assert_eq!(p.len(), 3, "{p:?}");
        let unknown_field = "registry_version = \"0.1.0\"\n[systems.\"M1.x\"]\nfidelity_levels = [\"F1\"]\nmagic = 1\n";
        assert!(matches!(
            Registry::from_toml(unknown_field),
            Err(RegistryError::Parse(_))
        ));
    }

    /// DEF-5: a third element is an error, not silently dropped.
    #[test]
    fn pairs_have_exactly_two_elements() {
        let reg = std::fs::read_to_string(registry_path()).unwrap();
        let three = reg.replace(
            "harvest_sigma = [0.04, 0.35]",
            "harvest_sigma = [0.04, 0.35, 99.0]",
        );
        assert!(
            matches!(Registry::from_toml(&three), Err(RegistryError::Parse(m)) if m.contains("3 elements"))
        );
        let horizon = reg.replace(
            "horizon_years = [50, 3000]",
            "horizon_years = [50, 3000, 9]",
        );
        assert!(
            matches!(Registry::from_toml(&horizon), Err(RegistryError::Parse(m)) if m.contains("3 elements"))
        );
        let one = reg.replace("harvest_sigma = [0.04, 0.35]", "harvest_sigma = [0.04]");
        assert!(Registry::from_toml(&one).is_err());
    }

    #[test]
    fn inconsistencies_are_named() {
        let text = "registry_version = \"0.1\"\n\
            [systems.\"M1.x\"]\nfidelity_levels = [\"F9\"]\nparams = { a = [2.0, 1.0], b = [0.0, inf] }\n\
            [goods]\nids = [\"grain\", \"grain\"]\n\
            [act_classes]\nids = [\"harm\"]\n\
            [verbs]\nX = { act_classes = [\"bless\"], emitted_by = \"M1.x\" }\n";
        let Err(RegistryError::Inconsistent(p)) = Registry::from_toml(text) else {
            panic!("accepted")
        };
        let all = p.join("\n");
        for needle in [
            "not MAJOR.MINOR.PATCH",
            "params.a: bounds [2.0, 1.0]",
            "params.b",
            "unknown fidelity level 'F9'",
            "goods: duplicate id 'grain'",
            "act class 'bless'",
        ] {
            assert!(all.contains(needle), "missing {needle:?} in:\n{all}");
        }
    }
}
