// GENERATED from worlds/smoke/systems/X3.smoke_vital.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "X3.smoke_vital";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Month, Phase::Vital),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    DeathTime = 1,  // key: (person, month)
    Cure = 2,  // key: (person, month)
    Birth = 3,  // key: (person, month)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::DeathTime => "death_time",
            Self::Cure => "cure",
            Self::Birth => "birth",
        }
    }
}
pub struct Params {
}
pub const ROLE_TARGET: rh_core::RoleId = rh_core::RoleId(1);
/// The role constants by Registry name: registration compares each code with the Registry's.
pub const ROLES: &[(&str, rh_core::RoleId)] = &[
    ("target", ROLE_TARGET),
];
