// GENERATED from worlds/smoke/systems/X2.smoke_harvest.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "X2.smoke_harvest";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Season, Phase::Production),
    (Pulse::Month, Phase::Exchange),
];
pub const DRAINS: &[(&str, Pulse, Phase)] = &[
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    YieldNoise = 1,  // key: (region, season)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::YieldNoise => "yield_noise",
        }
    }
}
pub struct Params {
}
/// The role constants by Registry name: registration compares each code with the Registry's.
pub const ROLES: &[(&str, rh_core::RoleId)] = &[
];
