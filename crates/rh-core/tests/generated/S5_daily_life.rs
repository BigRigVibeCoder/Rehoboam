// GENERATED from specs/systems/S5.daily_life.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "S5.daily_life";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Activity),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    ActivityBlock = 1,  // key: (person, day, block)
    EncounterOutcome = 2,  // key: (cell, day, block, person_a, person_b)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ActivityBlock => "activity_block",
            Self::EncounterOutcome => "encounter_outcome",
        }
    }
}
pub struct Params {
    pub blocks_per_day: Fixed,  // bounds checked against the Registry at load
    pub encounter_tie_rate: Fixed,  // bounds checked against the Registry at load
}
pub const ROLE_ACTOR: rh_core::RoleId = rh_core::RoleId(0);
pub const ROLE_TARGET: rh_core::RoleId = rh_core::RoleId(1);
/// The role constants by Registry name: registration compares each code with the Registry's.
pub const ROLES: &[(&str, rh_core::RoleId)] = &[
    ("actor", ROLE_ACTOR),
    ("target", ROLE_TARGET),
];
