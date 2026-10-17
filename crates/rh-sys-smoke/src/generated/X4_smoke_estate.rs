// GENERATED from worlds/smoke/systems/X4.smoke_estate.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "X4.smoke_estate";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Activity),
    (Pulse::Day, Phase::Contagion),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    GossipNoise = 1,  // key: (world, day)
    SettleFee = 2,  // key: (person, day)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::GossipNoise => "gossip_noise",
            Self::SettleFee => "settle_fee",
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
