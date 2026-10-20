// GENERATED from specs/systems/M2.demography.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "M2.demography";
pub const SPEC_VERSION: &str = "0.3.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Month, Phase::Vital),
    (Pulse::Day, Phase::Vital),
];
pub const DRAINS: &[(&str, Pulse, Phase)] = &[
    ("ScheduleDeath", Pulse::Day, Phase::Vital),
    ("Marry", Pulse::Day, Phase::Vital),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    DeathTime = 1,  // key: (person, month)
    Conception = 2,  // key: (person, month)
    MarriageTime = 3,  // key: (person, month)
    BirthSex = 4,  // key: (person)
    HusbandPick = 5,  // key: (person, month)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::DeathTime => "death_time",
            Self::Conception => "conception",
            Self::MarriageTime => "marriage_time",
            Self::BirthSex => "birth_sex",
            Self::HusbandPick => "husband_pick",
        }
    }
}
pub struct Params {
    pub e0: Fixed,  // bounds checked against the Registry at load
    pub brass_alpha: Fixed,  // bounds checked against the Registry at load
    pub brass_beta: Fixed,  // bounds checked against the Registry at load
    pub tfr: Fixed,  // bounds checked against the Registry at load
    pub eps_f: Fixed,  // bounds checked against the Registry at load
    pub eps_m: Fixed,  // bounds checked against the Registry at load
    pub elite_fertility_mult: Fixed,  // bounds checked against the Registry at load
    pub density_capacity: u64,  // bounds checked against the Registry at load
    pub density_mortality_slope: Fixed,  // bounds checked against the Registry at load
}
pub const ROLE_ACTOR: rh_core::RoleId = rh_core::RoleId(0);
pub const ROLE_TARGET: rh_core::RoleId = rh_core::RoleId(1);
/// The role constants by Registry name: registration compares each code with the Registry's.
pub const ROLES: &[(&str, rh_core::RoleId)] = &[
    ("actor", ROLE_ACTOR),
    ("target", ROLE_TARGET),
];
