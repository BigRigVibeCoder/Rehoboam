// GENERATED from specs/systems/T1.minds.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "T1.minds";
pub const SPEC_VERSION: &str = "0.3.3";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Vital),
    (Pulse::Day, Phase::Agenda),
    (Pulse::Month, Phase::Agenda),
];
pub const DRAINS: &[(&str, Pulse, Phase)] = &[
    ("OpinionModifier", Pulse::Day, Phase::Agenda),
    ("AgendaGoal", Pulse::Day, Phase::Agenda),
    ("ClearGoal", Pulse::Day, Phase::Agenda),
    ("ApplyThought", Pulse::Day, Phase::Agenda),
    ("ApplyInfluence", Pulse::Day, Phase::Agenda),
    ("Promote", Pulse::Day, Phase::Agenda),
    ("Demote", Pulse::Day, Phase::Agenda),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    TraitDraw = 1,  // key: (person, k)
    PollJitter = 2,  // key: (person, decision_type)
    IausSelect = 3,  // key: (person, decision_type, tick)
    ActionOrder = 4,  // key: (tick, person)
    BreakDelay = 5,  // key: (person, tick)
    BreakAct = 6,  // key: (person, tick)
    BreakOutcome = 7,  // key: (actor, target, tick)
    AttackOutcome = 8,  // key: (actor, target, tick)
    OutcomeDelay = 9,  // key: (person, tick)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::TraitDraw => "trait_draw",
            Self::PollJitter => "poll_jitter",
            Self::IausSelect => "iaus_select",
            Self::ActionOrder => "action_order",
            Self::BreakDelay => "break_delay",
            Self::BreakAct => "break_act",
            Self::BreakOutcome => "break_outcome",
            Self::AttackOutcome => "attack_outcome",
            Self::OutcomeDelay => "outcome_delay",
        }
    }
}
pub struct Params {
    pub stress_threshold: Fixed,  // bounds checked against the Registry at load
    pub stress_decay: Fixed,  // bounds checked against the Registry at load
    pub vow_conflict_gain: Fixed,  // bounds checked against the Registry at load
    pub break_delay_max_days: Fixed,  // bounds checked against the Registry at load
    pub grief_gain: Fixed,  // bounds checked against the Registry at load
    pub grief_instant_cap: Fixed,  // bounds checked against the Registry at load
    pub loyalty_conflict_gain: Fixed,  // bounds checked against the Registry at load
    pub dissonance_gain: Fixed,  // bounds checked against the Registry at load
    pub break_refractory_days: Fixed,  // bounds checked against the Registry at load
    pub attempt_success: Fixed,  // bounds checked against the Registry at load
    pub trait_sd: Fixed,  // bounds checked against the Registry at load
    pub iaus_top_k: Fixed,  // bounds checked against the Registry at load
    pub interrupt_daily_cap: Fixed,  // bounds checked against the Registry at load
    pub thought_cap: Fixed,  // bounds checked against the Registry at load
    pub thought_decay: Fixed,  // bounds checked against the Registry at load
    pub marriage_min_age: Fixed,  // bounds checked against the Registry at load
}
