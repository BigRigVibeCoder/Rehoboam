//! `WorldSpec`: a world manifest compiled for plane A (SPR-011 T-04, ruling 9; `05.5` §3.2).
//!
//! The tool crate `rh-manifest` builds it (`rh_manifest::compile`) from a validated manifest and
//! the Registry; plane A reads it and never the manifest. It holds plane-A types only: integers,
//! `rh_core::fixed` values, typed ids and enums. Every authored number arrived as the exact
//! decimal of its literal rounded half to even, never through an `f64`. Names and ids with meaning
//! stay on the tool side (the flavour rule, `05.5` §3): systems are their `SystemId`, founders are
//! positions in manifest order.
//!
//! REF: `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md` T-04, rulings 3, 8, 9 and 10;
//! `docs/05_architecture/05.5_world_compiler_and_dsl.md` §3.2.

use rh_core::fixed::Q16_16;
use rh_core::time::Calendar;
use rh_ids::SystemId;

/// A world ready for plane A: the seed, the calendar, the horizon, the named systems, the
/// parameters of the built ones in fixed point, and the founders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSpec {
    /// `world.seed`, the `world_seed` of every draw (`05.1` §6 item 2).
    pub seed: u64,
    /// `world.calendar`.
    pub calendar: Calendar,
    /// `world.horizon.years`: the manifest's horizon. A run may pass it (ruling 16); the run
    /// header records both.
    pub horizon_years: u32,
    /// Every system the manifest names (its template's defaults, `world.fidelity_profile` and
    /// `systems`), ascending by `SystemId`, each built or not built in this engine (ruling 3).
    pub systems: Vec<NamedSystem>,
    /// `M2.demography`'s parameters, when the build has M2 and the manifest names it.
    pub demography: Option<DemographyParams>,
    /// The founders, in genesis order (ruling 10): the cast in manifest order, then each
    /// household's children (sons, then daughters) and servants in manifest order, then the rest
    /// of the human population.
    pub founders: Vec<Founder>,
}

/// One system a manifest names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NamedSystem {
    /// The Registry system id's hash (`rh_core::register::system_id`).
    pub id: SystemId,
    /// The fidelity level from `world.fidelity_profile` (`F0` = 0 ...), if the manifest sets one.
    pub fidelity: Option<u8>,
    /// Whether this engine build has the system. A `not_built` system is skipped, never stubbed
    /// (ruling 3); `rh-cli run --strict` refuses a manifest with one.
    pub built: bool,
}

/// `M2.demography`'s rung-0 parameters (SPR-011 rulings 7 and 8): each from the manifest, else
/// from the Registry's `defaults`. Rung 0 reads no other parameter; `brass_alpha` and any
/// `brass_beta` other than 1 are refused at compile time until P1.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DemographyParams {
    /// Life expectancy at birth, in years.
    pub e0: Q16_16,
    /// The Brass slope; exactly 1 at rung 0.
    pub brass_beta: Q16_16,
    /// Total fertility rate target (F2), in Q16_16.
    pub tfr: Q16_16,
    /// Fertility multiplier for house members (F2), in Q16_16.
    pub elite_fertility_mult: Q16_16,
}

/// A founder's sex: as the manifest gives it, else drawn by `rh-worldgen` (ruling 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sex {
    /// Female.
    Female,
    /// Male.
    Male,
}

/// Where a founder comes from in the manifest, which sets the age range `rh-worldgen` draws in
/// when no age is given (ruling 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FounderRole {
    /// A `cast` member.
    Cast,
    /// A household's son or daughter.
    Child,
    /// A household's servant.
    Servant,
    /// The rest of a people's `population`.
    Commoner,
}

/// One founder: who they are in the manifest's terms, with no name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Founder {
    /// Where the founder comes from.
    pub role: FounderRole,
    /// The sex, when the manifest gives it.
    pub sex: Option<Sex>,
    /// The age in whole years, when the manifest gives it.
    pub age_years: Option<u8>,
}
