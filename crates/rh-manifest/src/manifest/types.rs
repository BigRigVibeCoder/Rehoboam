//! The World Manifest v0 as Rust types: one struct per model of `specs/manifest/manifest_v0.py`,
//! with the same names, fields, defaults and bounds.
//!
//! Bounds are enforced in `coerce` and declared here for the JSON Schema (SPR-004 T-04): the
//! schema emitted from these types is diffed against `specs/manifest/manifest_v0.schema.json`
//! (see `schema`), so a bound changed in one place and not the other fails the build.
//!
//! Flavour text (names, naming seeds, prose) is carried only so the expander can move it into the
//! flavour asset; no simulation crate may read it.
#![allow(
    missing_docs,
    reason = "fields mirror manifest_v0.py, which documents them"
)]

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::SerializeMap;

use super::coerce::SLUG_PATTERN;

/// A string-keyed map that keeps the manifest's key order (Python dicts are ordered), so that
/// diagnostics come out in the same order as the reference's. Lookups are linear: manifest maps
/// are small, and plane A never iterates a hash map.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrderedMap<V>(pub Vec<(String, V)>);

impl<V> OrderedMap<V> {
    /// The value for `key`, if present.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&V> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Whether `key` is present.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Entries in manifest order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Keys in manifest order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(k, _)| k.as_str())
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<V: Serialize> Serialize for OrderedMap<V> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            m.serialize_entry(k, v)?;
        }
        m.end()
    }
}

impl<V: JsonSchema> JsonSchema for OrderedMap<V> {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        format!("Map_of_{}", V::schema_name()).into()
    }

    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "object", "additionalProperties": g.subschema_for::<V>() })
    }
}

/// A number a manifest authors for a system parameter (`systems.<id>.<param>`): the float the
/// reference validates against the Registry bounds, and the exact decimal of the literal as
/// written, which is all that reaches plane A (`crate::exact`; `05.5` §3.2; SPR-011 ruling 9).
/// It serialises, and appears in the JSON Schema, as the float it validates as.
#[derive(Debug, Clone, PartialEq)]
pub struct Authored {
    /// The value as Python's `float()` gives it: what validation compares with the bounds.
    pub value: f64,
    /// The literal's exact decimal value, or why it has none (an infinity, NaN).
    pub exact: Result<crate::exact::Decimal, crate::exact::DecimalError>,
}

impl Serialize for Authored {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(self.value)
    }
}

impl JsonSchema for Authored {
    fn inline_schema() -> bool {
        f64::inline_schema()
    }

    fn schema_name() -> Cow<'static, str> {
        f64::schema_name()
    }

    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        f64::json_schema(g)
    }
}

// ----------------------------------------------------------------------------- schema helpers

/// Adds a discriminator property (`"kind": {"const": "wager"}`) to a model's schema, required,
/// as Pydantic's `Literal` tag field appears.
fn add_tag(schema: &mut Schema, field: &str, value: &str) {
    if let Some(props) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
        props.insert(
            field.to_owned(),
            serde_json::json!({ "type": "string", "const": value }),
        );
    }
    let required = schema
        .ensure_object()
        .entry("required")
        .or_insert_with(|| serde_json::json!([]));
    if let Some(r) = required.as_array_mut() {
        r.insert(0, serde_json::json!(field));
    }
}

macro_rules! tag_fn {
    ($name:ident, $field:literal, $value:literal) => {
        fn $name(schema: &mut Schema) {
            add_tag(schema, $field, $value);
        }
    };
}
tag_fn!(tag_human, "archetype", "human");
tag_fn!(tag_creature, "archetype", "apex_predator");
tag_fn!(tag_dynastic_tension, "kind", "dynastic_tension");
tag_fn!(tag_artifact, "kind", "artifact");
tag_fn!(tag_covert_outbreak, "kind", "covert_outbreak");
tag_fn!(tag_climate_shock, "kind", "climate_shock");
tag_fn!(tag_wager, "kind", "wager");

fn slug_keyed(value: &serde_json::Value) -> Schema {
    json_schema!({ "type": "object", "patternProperties": { SLUG_PATTERN: value } })
}

fn titles_schema(_: &mut SchemaGenerator) -> Schema {
    slug_keyed(&serde_json::json!({ "type": "string" }))
}

fn initial_schema(_: &mut SchemaGenerator) -> Schema {
    slug_keyed(&serde_json::json!({ "type": "number", "minimum": -1.0, "maximum": 1.0 }))
}

fn holdings_schema(_: &mut SchemaGenerator) -> Schema {
    slug_keyed(&serde_json::json!({ "type": "integer", "minimum": 0, "maximum": i64::MAX }))
}

fn nullable_slug(_: &mut SchemaGenerator) -> Schema {
    json_schema!({ "anyOf": [{ "type": "string", "pattern": SLUG_PATTERN }, { "type": "null" }] })
}

fn nullable_predicate(_: &mut SchemaGenerator) -> Schema {
    json_schema!({ "anyOf": [{ "type": "string", "maxLength": 200 }, { "type": "null" }] })
}

fn nullable_activity_resolution(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "anyOf": [
            { "type": "string", "enum": ["life_course", "blocks", "minutes"] },
            { "type": "null" }
        ]
    })
}

fn cast_flags(_: &mut SchemaGenerator) -> Schema {
    json_schema!({ "type": "array", "items": { "type": "string", "enum": ["protagonist", "deep"] } })
}

fn fidelity_profile(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "object",
        "additionalProperties": { "type": "string", "enum": ["F0", "F1", "F2", "F3"] }
    })
}

fn default_epoch_label() -> String {
    "Year".to_owned()
}
fn default_days_per_year() -> i64 {
    360
}
fn default_months() -> i64 {
    12
}
fn default_seasons() -> i64 {
    4
}
fn default_raster_resolution() -> i64 {
    512
}
fn default_region_count() -> i64 {
    600
}
fn default_harvest_sigma() -> f64 {
    0.2
}
fn default_autocorr() -> f64 {
    0.1
}
fn default_spatial_share() -> f64 {
    0.6
}
fn default_volcanic_rate() -> f64 {
    0.004
}
fn default_livestock() -> f64 {
    150.0
}
fn default_one_f64() -> f64 {
    1.0
}
fn default_half() -> f64 {
    0.5
}
fn default_generations() -> i64 {
    3
}
fn default_heir_age() -> i64 {
    20
}
fn default_true() -> bool {
    true
}
fn default_random() -> String {
    "random".to_owned()
}
fn default_hive() -> String {
    "hive".to_owned()
}
fn default_one_i64() -> i64 {
    1
}
fn default_policy() -> String {
    "deterministic".to_owned()
}

// ----------------------------------------------------------------------------- world meta

/// Whether a world's horizon is historical or speculative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(inline)]
#[allow(
    missing_docs,
    reason = "variant docs would turn the schema's enum into a oneOf; the values are self-explanatory"
)]
pub enum HorizonLabel {
    // Within what the template supports.
    Historical,
    // Beyond it; needs a dated trajectory.
    Speculative,
}

/// The world-level activity resolution (ADR-0031).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(inline)]
#[allow(
    missing_docs,
    reason = "variant docs would turn the schema's enum into a oneOf; the values are self-explanatory"
)]
pub enum ActivityResolution {
    // Life-course events only.
    LifeCourse,
    // Activity blocks per day.
    Blocks,
    // Every minute.
    Minutes,
}

/// A focus region's activity resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(inline)]
#[allow(
    missing_docs,
    reason = "variant docs would turn the schema's enum into a oneOf; the values are self-explanatory"
)]
pub enum FocusResolution {
    // Activity blocks per day.
    Blocks,
    // Every minute.
    Minutes,
}

/// `Calendar`: days per year, months, seasons and the epoch label.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Calendar {
    #[schemars(default = "default_days_per_year", extend("enum" = [360, 365]))]
    pub days_per_year: i64,
    #[schemars(default = "default_months", extend("const" = 12))]
    pub months: i64,
    #[schemars(default = "default_seasons", extend("enum" = [2, 4]))]
    pub seasons: i64,
    #[schemars(default = "default_epoch_label")]
    pub epoch_label: String,
}

impl Default for Calendar {
    fn default() -> Self {
        Calendar {
            days_per_year: default_days_per_year(),
            months: default_months(),
            seasons: default_seasons(),
            epoch_label: default_epoch_label(),
        }
    }
}

/// `Horizon`: how many years the world runs, and whether that is historical.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Horizon {
    #[schemars(range(min = 1, max = 5000))]
    pub years: i64,
    #[schemars(default = "historical")]
    pub label: HorizonLabel,
}

fn historical() -> HorizonLabel {
    HorizonLabel::Historical
}

/// `Milestone`: a dated technology or institution.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Milestone {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    #[schemars(range(min = 0, max = 5000))]
    pub by_year: i64,
}

/// `Trajectory`: what a speculative world is expected to reach (05.5 §5).
#[derive(Debug, Clone, PartialEq, Default, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Trajectory {
    #[schemars(default)]
    pub technologies: Vec<Milestone>,
    #[schemars(default)]
    pub institutions: Vec<Milestone>,
}

/// `Focus`: a settlement where daily life is authoritative at a finer resolution.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Focus {
    #[schemars(pattern(SLUG_PATTERN))]
    pub settlement: String,
    pub activity_resolution: FocusResolution,
}

/// `WorldMeta`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct WorldMeta {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    #[schemars(length(min = 1, max = 120))]
    pub display_name: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub template: String,
    #[schemars(range(min = 0, max = i64::MAX))]
    pub seed: i64,
    #[schemars(default)]
    pub calendar: Calendar,
    pub horizon: Horizon,
    #[schemars(default)]
    pub trajectory: Option<Trajectory>,
    #[schemars(default, schema_with = "fidelity_profile")]
    pub fidelity_profile: OrderedMap<String>,
    #[schemars(default, schema_with = "nullable_activity_resolution")]
    pub activity_resolution: Option<ActivityResolution>,
    #[schemars(default)]
    pub focus: Vec<Focus>,
}

// ----------------------------------------------------------------------------- geography

/// `Climate`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Climate {
    #[schemars(pattern(SLUG_PATTERN))]
    pub zone: String,
    #[schemars(default = "default_harvest_sigma")]
    pub harvest_sigma: f64,
    #[schemars(default = "default_autocorr")]
    pub autocorr: f64,
    #[schemars(default = "default_spatial_share")]
    pub spatial_share: f64,
    #[schemars(default = "default_volcanic_rate")]
    pub volcanic_rate: f64,
}

/// `Ecology`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Ecology {
    #[schemars(default = "default_livestock", range(min = 0.0, max = 2000.0))]
    pub livestock_kg_per_capita: f64,
    #[schemars(default, range(min = 0.0))]
    pub wild_prey_tonnes: f64,
}

impl Default for Ecology {
    fn default() -> Self {
        Ecology {
            livestock_kg_per_capita: default_livestock(),
            wild_prey_tonnes: 0.0,
        }
    }
}

/// `Geography`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Geography {
    #[schemars(
        default = "default_raster_resolution",
        extend("enum" = [64, 128, 256, 512, 1024, 2048, 4096])
    )]
    pub raster_resolution: i64,
    #[schemars(default = "default_region_count", range(min = 1, max = 5000))]
    pub region_count: i64,
    #[schemars(pattern(SLUG_PATTERN))]
    pub heightmap_template: String,
    pub climate: Climate,
    #[schemars(default)]
    pub ecology: Ecology,
}

// ----------------------------------------------------------------------------- peoples

/// `Culture` (name and naming seed are flavour only).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Culture {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    pub name: String,
    #[schemars(length(max = 200))]
    pub naming_seed: String,
    #[schemars(default, schema_with = "value_bias_schema")]
    pub value_bias: OrderedMap<f64>,
}

fn value_bias_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "object",
        "additionalProperties": { "type": "number", "minimum": -1.0, "maximum": 1.0 }
    })
}

/// `Faith`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Faith {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    pub name: String,
    #[schemars(default, inner(pattern(SLUG_PATTERN)))]
    pub doctrine_tags: Vec<String>,
}

/// `ClassShares`: commoner, burgher and elite shares summing to 1.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ClassShares {
    #[schemars(range(min = 0.0, max = 1.0))]
    pub commoner: f64,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub burgher: f64,
    #[schemars(range(min = 0.0, max = 1.0))]
    pub elite: f64,
}

/// `HumanPeople` (archetype `human`).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields, transform = tag_human)]
pub struct HumanPeople {
    #[schemars(range(min = 1, max = 10_000_000))]
    pub population: i64,
    #[schemars(length(min = 1))]
    pub cultures: Vec<Culture>,
    #[schemars(length(min = 1))]
    pub faiths: Vec<Faith>,
    pub class_shares: ClassShares,
}

/// `CreatureParams`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CreatureParams {
    pub mass_kg: f64,
    #[schemars(default = "default_one_f64")]
    pub metabolism_multiplier: f64,
    #[schemars(default)]
    pub flying: bool,
    #[schemars(default)]
    pub bondable: bool,
}

/// `CreaturePeople` (archetype `apex_predator`).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields, transform = tag_creature)]
pub struct CreaturePeople {
    #[schemars(pattern(SLUG_PATTERN))]
    pub variant: String,
    #[schemars(range(min = 1, max = 100_000))]
    pub population: i64,
    pub params: CreatureParams,
}

/// A people, discriminated by `archetype`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "archetype")]
pub enum People {
    /// `archetype: human`.
    #[serde(rename = "human")]
    Human(HumanPeople),
    /// `archetype: apex_predator`.
    #[serde(rename = "apex_predator")]
    Creature(CreaturePeople),
}

impl JsonSchema for People {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "People".into()
    }

    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "oneOf": [g.subschema_for::<HumanPeople>(), g.subschema_for::<CreaturePeople>()],
            "discriminator": {
                "propertyName": "archetype",
                "mapping": {
                    "human": "#/$defs/HumanPeople",
                    "apex_predator": "#/$defs/CreaturePeople"
                }
            }
        })
    }
}

// ----------------------------------------------------------------------------- polities

/// `Dynasty`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Dynasty {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    pub name: String,
    #[schemars(default = "default_generations", range(min = 1, max = 6))]
    pub generations: i64,
}

/// `Polity`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Polity {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    pub name: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub government: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub succession_law: String,
    /// Abstract role -> concrete title, e.g. authority -> `High_King`.
    #[schemars(schema_with = "titles_schema")]
    pub titles: OrderedMap<String>,
    #[schemars(default)]
    pub dynasties: Vec<Dynasty>,
}

// ----------------------------------------------------------------------------- catalysts

/// `DynasticTensionInitial`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct DynasticTensionInitial {
    #[schemars(default, range(min = -100, max = 100))]
    pub opinion_successor_of_authority: i64,
    #[schemars(default, range(min = -100, max = 100))]
    pub opinion_authority_of_successor: i64,
    #[schemars(default, range(min = 0.0, max = 1.0))]
    pub fear_authority: f64,
    #[schemars(default = "default_heir_age", range(min = 0, max = 80))]
    pub heir_age_years: i64,
}

/// `DynasticTension` (kind `dynastic_tension`).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields, transform = tag_dynastic_tension)]
pub struct DynasticTension {
    #[schemars(pattern(SLUG_PATTERN))]
    pub polity: String,
    pub initial: DynasticTensionInitial,
}

/// `OverlayEffect`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct OverlayEffect {
    #[schemars(pattern(SLUG_PATTERN))]
    pub overlay: String,
    #[schemars(range(min = -1.0, max = 1.0))]
    pub holder_delta: f64,
}

/// `Artifact` (kind `artifact`).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields, transform = tag_artifact)]
pub struct Artifact {
    pub name: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub polity: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub holder_role: String,
    #[schemars(length(min = 1))]
    pub effects: Vec<OverlayEffect>,
}

/// `CovertOutbreak` (kind `covert_outbreak`).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields, transform = tag_covert_outbreak)]
pub struct CovertOutbreak {
    #[schemars(range(min = 1, max = 10_000))]
    pub seed_infected: i64,
    #[schemars(
        default = "default_random",
        extend("enum" = ["capital", "port", "periphery", "random"])
    )]
    pub seed_region_hint: String,
    #[schemars(default = "default_hive", extend("const" = "hive"))]
    pub channel: String,
    #[schemars(default, range(min = 0, max = i64::MAX))]
    pub start_day: i64,
}

/// `ClimateShock` (kind `climate_shock`).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields, transform = tag_climate_shock)]
pub struct ClimateShock {
    #[schemars(range(min = 0, max = i64::MAX))]
    pub year: i64,
    #[schemars(range(min = 0.5, max = 4.0))]
    pub severity_sigma: f64,
    #[schemars(default = "default_one_i64", range(min = 1, max = 10))]
    pub duration_years: i64,
}

/// `WagerStage`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct WagerStage {
    #[schemars(length(min = 1))]
    pub allow: Vec<String>,
    #[schemars(default = "default_true")]
    pub spare_life: bool,
    #[schemars(default, schema_with = "nullable_predicate")]
    pub unlock_when: Option<String>,
}

/// `Wager` (kind `wager`; ADR-0028 §3).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields, transform = tag_wager)]
pub struct Wager {
    #[schemars(pattern(SLUG_PATTERN))]
    pub agent: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub protagonist: String,
    #[schemars(length(min = 1))]
    pub stages: Vec<WagerStage>,
}

/// A catalyst, discriminated by `kind`. Catalysts are initial conditions, never forced outcomes.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Catalyst {
    /// `kind: dynastic_tension`.
    DynasticTension(DynasticTension),
    /// `kind: artifact`.
    Artifact(Artifact),
    /// `kind: covert_outbreak`.
    CovertOutbreak(CovertOutbreak),
    /// `kind: climate_shock`.
    ClimateShock(ClimateShock),
    /// `kind: wager`.
    Wager(Wager),
}

impl JsonSchema for Catalyst {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "Catalyst".into()
    }

    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "oneOf": [
                g.subschema_for::<DynasticTension>(),
                g.subschema_for::<Artifact>(),
                g.subschema_for::<CovertOutbreak>(),
                g.subschema_for::<ClimateShock>(),
                g.subschema_for::<Wager>()
            ],
            "discriminator": {
                "propertyName": "kind",
                "mapping": {
                    "artifact": "#/$defs/Artifact",
                    "climate_shock": "#/$defs/ClimateShock",
                    "covert_outbreak": "#/$defs/CovertOutbreak",
                    "dynastic_tension": "#/$defs/DynasticTension",
                    "wager": "#/$defs/Wager"
                }
            }
        })
    }
}

// ----------------------------------------------------------------------------- cast, households, outside world, agents

/// `CastMember`: a named person the expander must create exactly.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CastMember {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    pub name: String,
    #[schemars(extend("enum" = ["male", "female"]))]
    pub sex: String,
    #[schemars(range(min = 0, max = 120))]
    pub age_years: i64,
    #[schemars(default, schema_with = "nullable_slug")]
    pub household: Option<String>,
    #[schemars(default, schema_with = "cast_flags")]
    pub flags: Vec<String>,
    #[schemars(default, schema_with = "initial_schema")]
    pub initial: OrderedMap<f64>,
}

/// `Children`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Children {
    #[schemars(default, range(min = 0, max = 40))]
    pub sons: i64,
    #[schemars(default, range(min = 0, max = 40))]
    pub daughters: i64,
}

/// `Household`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Household {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub head: String,
    #[schemars(default)]
    pub children: Children,
    #[schemars(default, range(min = 0, max = 100_000))]
    pub servants: i64,
    #[schemars(default, schema_with = "holdings_schema")]
    pub holdings: OrderedMap<i64>,
}

/// `OffMapActor`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct OffMapActor {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    pub name: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub kind: String,
    #[schemars(default = "default_half", range(min = 0.0, max = 1.0))]
    pub strength: f64,
}

/// `Agent`: an in-world intervening agent.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Agent {
    #[schemars(pattern(SLUG_PATTERN))]
    pub id: String,
    pub name: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub archetype: String,
    #[schemars(length(min = 1))]
    pub verbs: Vec<String>,
    #[schemars(default = "default_policy", extend("enum" = ["deterministic", "player"]))]
    pub policy: String,
}

// ----------------------------------------------------------------------------- narrative, presentation, root

/// `Narrative`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Narrative {
    #[schemars(length(min = 1), inner(pattern(SLUG_PATTERN)))]
    pub pattern_packs: Vec<String>,
    #[schemars(pattern(SLUG_PATTERN))]
    pub default_style_card: String,
}

/// `Presentation`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Presentation {
    #[schemars(pattern(SLUG_PATTERN))]
    pub era_art: String,
    #[schemars(pattern(SLUG_PATTERN))]
    pub palette: String,
}

/// The World Manifest v0 root.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Manifest {
    #[schemars(extend("const" = 0))]
    pub manifest_version: i64,
    #[schemars(pattern(super::coerce::SEMVER_PATTERN))]
    pub registry_version: String,
    pub world: WorldMeta,
    pub geography: Geography,
    #[schemars(length(min = 1))]
    pub peoples: Vec<People>,
    #[schemars(default)]
    pub polities: Vec<Polity>,
    #[schemars(default)]
    pub cast: Vec<CastMember>,
    #[schemars(default)]
    pub households: Vec<Household>,
    #[schemars(default)]
    pub off_map_actors: Vec<OffMapActor>,
    #[schemars(default)]
    pub agents: Vec<Agent>,
    /// System id -> {param: value}; params bounded by Registry.
    pub systems: OrderedMap<OrderedMap<Authored>>,
    #[schemars(default, inner(pattern(SLUG_PATTERN)))]
    pub institutions: Vec<String>,
    #[schemars(default, inner(pattern(SLUG_PATTERN)))]
    pub technologies: Vec<String>,
    #[schemars(default)]
    pub catalysts: Vec<Catalyst>,
    #[schemars(default, inner(pattern(SLUG_PATTERN)))]
    pub overlays: Vec<String>,
    pub narrative: Narrative,
    pub presentation: Presentation,
}
