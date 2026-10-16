// GENERATED from worlds/smoke/systems/X5.smoke_polity.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "X5.smoke_polity";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Year, Phase::Polity),
    (Pulse::Month, Phase::Polity),
    (Pulse::Season, Phase::Military),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    MusterPick = 1,  // key: (person, season)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::MusterPick => "muster_pick",
        }
    }
}
pub struct Params {
}
