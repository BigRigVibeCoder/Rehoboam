// GENERATED from worlds/smoke/systems/X1.smoke_weather.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "X1.smoke_weather";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Year, Phase::Inputs),
    (Pulse::Hour, Phase::Environment),
];
pub const DRAINS: &[(&str, Pulse, Phase)] = &[
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    DriftStep = 1,  // key: (region, hour)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::DriftStep => "drift_step",
        }
    }
}
pub struct Params {
}
