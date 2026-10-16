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
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::GossipNoise => "gossip_noise",
        }
    }
}
pub struct Params {
}
