//! The World Manifest v0 as Rust types: one struct per model of `specs/manifest/manifest_v0.py`,
//! with the same field names, defaults and bounds (the bounds are enforced in `coerce`).
//!
//! Flavour text (names, naming seeds, prose) is carried only so the expander can move it into the
//! flavour asset; no simulation crate may read it.

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

/// Whether a world's horizon is historical or speculative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizonLabel {
    /// Within what the template supports.
    Historical,
    /// Beyond it; needs a dated trajectory.
    Speculative,
}

/// The world-level activity resolution (ADR-0031).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityResolution {
    /// Life-course events only.
    LifeCourse,
    /// Activity blocks per day.
    Blocks,
    /// Every minute.
    Minutes,
}

/// A focus region's activity resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusResolution {
    /// Activity blocks per day.
    Blocks,
    /// Every minute.
    Minutes,
}

/// `Calendar`: days per year, months, seasons and the epoch label.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Calendar {
    pub days_per_year: i64,
    pub months: i64,
    pub seasons: i64,
    pub epoch_label: String,
}

impl Default for Calendar {
    fn default() -> Self {
        Calendar {
            days_per_year: 360,
            months: 12,
            seasons: 4,
            epoch_label: "Year".to_owned(),
        }
    }
}

/// `Horizon`: how many years the world runs, and whether that is historical.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Horizon {
    pub years: i64,
    pub label: HorizonLabel,
}

/// `Milestone`: a dated technology or institution.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Milestone {
    pub id: String,
    pub by_year: i64,
}

/// `Trajectory`: what a speculative world is expected to reach (05.5 §5).
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Trajectory {
    pub technologies: Vec<Milestone>,
    pub institutions: Vec<Milestone>,
}

/// `Focus`: a settlement where daily life is authoritative at a finer resolution.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Focus {
    pub settlement: String,
    pub activity_resolution: FocusResolution,
}

/// `WorldMeta`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct WorldMeta {
    pub id: String,
    pub display_name: String,
    pub template: String,
    pub seed: i64,
    pub calendar: Calendar,
    pub horizon: Horizon,
    pub trajectory: Option<Trajectory>,
    pub fidelity_profile: OrderedMap<String>,
    pub activity_resolution: Option<ActivityResolution>,
    pub focus: Vec<Focus>,
}

/// `Climate`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Climate {
    pub zone: String,
    pub harvest_sigma: f64,
    pub autocorr: f64,
    pub spatial_share: f64,
    pub volcanic_rate: f64,
}

/// `Ecology`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Ecology {
    pub livestock_kg_per_capita: f64,
    pub wild_prey_tonnes: f64,
}

impl Default for Ecology {
    fn default() -> Self {
        Ecology {
            livestock_kg_per_capita: 150.0,
            wild_prey_tonnes: 0.0,
        }
    }
}

/// `Geography`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Geography {
    pub raster_resolution: i64,
    pub region_count: i64,
    pub heightmap_template: String,
    pub climate: Climate,
    pub ecology: Ecology,
}

/// `Culture` (name and naming seed are flavour only).
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Culture {
    pub id: String,
    pub name: String,
    pub naming_seed: String,
    pub value_bias: OrderedMap<f64>,
}

/// `Faith`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Faith {
    pub id: String,
    pub name: String,
    pub doctrine_tags: Vec<String>,
}

/// `ClassShares`: commoner, burgher and elite shares summing to 1.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct ClassShares {
    pub commoner: f64,
    pub burgher: f64,
    pub elite: f64,
}

/// `HumanPeople`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct HumanPeople {
    pub population: i64,
    pub cultures: Vec<Culture>,
    pub faiths: Vec<Faith>,
    pub class_shares: ClassShares,
}

/// `CreatureParams`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct CreatureParams {
    pub mass_kg: f64,
    pub metabolism_multiplier: f64,
    pub flying: bool,
    pub bondable: bool,
}

/// `CreaturePeople` (archetype `apex_predator`).
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct CreaturePeople {
    pub variant: String,
    pub population: i64,
    pub params: CreatureParams,
}

/// A people, discriminated by `archetype`.
#[derive(Debug, Clone, PartialEq)]
pub enum People {
    /// `archetype: human`.
    Human(HumanPeople),
    /// `archetype: apex_predator`.
    Creature(CreaturePeople),
}

/// `Dynasty`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Dynasty {
    pub id: String,
    pub name: String,
    pub generations: i64,
}

/// `Polity`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Polity {
    pub id: String,
    pub name: String,
    pub government: String,
    pub succession_law: String,
    pub titles: OrderedMap<String>,
    pub dynasties: Vec<Dynasty>,
}

/// `DynasticTensionInitial`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct DynasticTensionInitial {
    pub opinion_successor_of_authority: i64,
    pub opinion_authority_of_successor: i64,
    pub fear_authority: f64,
    pub heir_age_years: i64,
}

/// `OverlayEffect`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct OverlayEffect {
    pub overlay: String,
    pub holder_delta: f64,
}

/// `WagerStage`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct WagerStage {
    pub allow: Vec<String>,
    pub spare_life: bool,
    pub unlock_when: Option<String>,
}

/// A catalyst, discriminated by `kind`. Catalysts are initial conditions, never forced outcomes.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub enum Catalyst {
    /// `kind: dynastic_tension`.
    DynasticTension {
        polity: String,
        initial: DynasticTensionInitial,
    },
    /// `kind: artifact`.
    Artifact {
        name: String,
        polity: String,
        holder_role: String,
        effects: Vec<OverlayEffect>,
    },
    /// `kind: covert_outbreak`.
    CovertOutbreak {
        seed_infected: i64,
        seed_region_hint: String,
        channel: String,
        start_day: i128,
    },
    /// `kind: climate_shock`.
    ClimateShock {
        year: i128,
        severity_sigma: f64,
        duration_years: i64,
    },
    /// `kind: wager` (ADR-0028 §3).
    Wager {
        agent: String,
        protagonist: String,
        stages: Vec<WagerStage>,
    },
}

/// `CastMember`: a named person the expander must create exactly.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct CastMember {
    pub id: String,
    pub name: String,
    pub sex: String,
    pub age_years: i64,
    pub household: Option<String>,
    pub flags: Vec<String>,
    pub initial: OrderedMap<f64>,
}

/// `Children`.
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Children {
    pub sons: i64,
    pub daughters: i64,
}

/// `Household`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Household {
    pub id: String,
    pub head: String,
    pub children: Children,
    pub servants: i64,
    pub holdings: OrderedMap<i128>,
}

/// `OffMapActor`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct OffMapActor {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub strength: f64,
}

/// `Agent`: an in-world intervening agent.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub archetype: String,
    pub verbs: Vec<String>,
    pub policy: String,
}

/// `Narrative`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Narrative {
    pub pattern_packs: Vec<String>,
    pub default_style_card: String,
}

/// `Presentation`.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Presentation {
    pub era_art: String,
    pub palette: String,
}

/// The World Manifest v0 root.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs, reason = "fields mirror manifest_v0.py")]
pub struct Manifest {
    pub manifest_version: i64,
    pub registry_version: String,
    pub world: WorldMeta,
    pub geography: Geography,
    pub peoples: Vec<People>,
    pub polities: Vec<Polity>,
    pub cast: Vec<CastMember>,
    pub households: Vec<Household>,
    pub off_map_actors: Vec<OffMapActor>,
    pub agents: Vec<Agent>,
    pub systems: OrderedMap<OrderedMap<f64>>,
    pub institutions: Vec<String>,
    pub technologies: Vec<String>,
    pub catalysts: Vec<Catalyst>,
    pub overlays: Vec<String>,
    pub narrative: Narrative,
    pub presentation: Presentation,
}
