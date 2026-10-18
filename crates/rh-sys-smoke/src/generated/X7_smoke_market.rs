// GENERATED from worlds/smoke/systems/X7.smoke_market.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "X7.smoke_market";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Environment),
    (Pulse::Day, Phase::Activity),
];
pub const DRAINS: &[(&str, Pulse, Phase)] = &[
    ("Audit", Pulse::Day, Phase::Activity),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    PriceStep = 1,  // key: (region, day)
    DefaultPick = 2,  // key: (person, day)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PriceStep => "price_step",
            Self::DefaultPick => "default_pick",
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
