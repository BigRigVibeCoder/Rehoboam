// GENERATED from specs/systems/M7.dynasty.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "M7.dynasty";
pub const SPEC_VERSION: &str = "0.3.1";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Polity),
    (Pulse::Month, Phase::Polity),
    (Pulse::Day, Phase::Agenda),
    (Pulse::Year, Phase::Polity),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    ExitMurder = 1,  // key: (ruler, month)
    ExitDeposition = 2,  // key: (ruler, month)
    ExitDay = 3,  // key: (ruler, month)
    MurderCulprit = 4,  // key: (ruler, month)
    ArbitrationTie = 5,  // key: (title, tick)
    RegrantTie = 6,  // key: (title, tick)
    Sortition = 7,  // key: (person, tick)
    InterregnumDelay = 8,  // key: (title, tick)
    Vocation = 9,  // key: (person)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ExitMurder => "exit_murder",
            Self::ExitDeposition => "exit_deposition",
            Self::ExitDay => "exit_day",
            Self::MurderCulprit => "murder_culprit",
            Self::ArbitrationTie => "arbitration_tie",
            Self::RegrantTie => "regrant_tie",
            Self::Sortition => "sortition",
            Self::InterregnumDelay => "interregnum_delay",
            Self::Vocation => "vocation",
        }
    }
}
pub struct Params {
    pub murder_hazard_base: Fixed,  // bounds checked against the Registry at load
    pub deposition_hazard_ref: Fixed,  // bounds checked against the Registry at load
    pub primogeniture_hr: Fixed,  // bounds checked against the Registry at load
    pub seniority_hr: Fixed,  // bounds checked against the Registry at load
    pub predecessor_killed_or: Fixed,  // bounds checked against the Registry at load
    pub predecessor_deposed_hr: Fixed,  // bounds checked against the Registry at load
    pub exec_constraint_hr: Fixed,  // bounds checked against the Registry at load
    pub majority_age: Fixed,  // bounds checked against the Registry at load
    pub arb_legal_weight: Fixed,  // bounds checked against the Registry at load
    pub arb_claim_weight: Fixed,  // bounds checked against the Registry at load
    pub arb_military_weight: Fixed,  // bounds checked against the Registry at load
    pub arb_treasury_weight: Fixed,  // bounds checked against the Registry at load
    pub contest_margin: Fixed,  // bounds checked against the Registry at load
    pub interregnum_max_days: Fixed,  // bounds checked against the Registry at load
    pub dormancy_degrees: Fixed,  // bounds checked against the Registry at load
    pub vocation_holy_orders: Fixed,  // bounds checked against the Registry at load
    pub vocation_mercenary: Fixed,  // bounds checked against the Registry at load
}
