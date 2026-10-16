// GENERATED from specs/systems/S5.daily_life.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "S5.daily_life";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Activity),
];
#[repr(u32)]
pub enum RngPurpose {
    ActivityBlock = 1,  // key: (person, day, block)
    EncounterOutcome = 2,  // key: (cell, day, block, person_a, person_b)
}
pub struct Params {
    pub blocks_per_day: Fixed,  // bounds checked against the Registry at load
    pub encounter_tie_rate: Fixed,  // bounds checked against the Registry at load
}
