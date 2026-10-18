// GENERATED from specs/systems/S6.artifacts.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "S6.artifacts";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Vital),
    (Pulse::Day, Phase::Agenda),
    (Pulse::Month, Phase::Culture),
];
pub const DRAINS: &[(&str, Pulse, Phase)] = &[
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    LossOnDeath = 1,  // key: (artifact, tick)
    TheftOutcome = 2,  // key: (thief, artifact, tick)
    SearchFind = 3,  // key: (seeker, artifact, month)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::LossOnDeath => "loss_on_death",
            Self::TheftOutcome => "theft_outcome",
            Self::SearchFind => "search_find",
        }
    }
}
pub struct Params {
    pub influence_base: Fixed,  // bounds checked against the Registry at load
    pub resist_slope: Fixed,  // bounds checked against the Registry at load
    pub corruption_threshold: Fixed,  // bounds checked against the Registry at load
    pub rumour_max_hops: Fixed,  // bounds checked against the Registry at load
    pub rumour_decay_years: Fixed,  // bounds checked against the Registry at load
    pub find_chance: Fixed,  // bounds checked against the Registry at load
    pub theft_success: Fixed,  // bounds checked against the Registry at load
    pub loss_on_death: Fixed,  // bounds checked against the Registry at load
}
