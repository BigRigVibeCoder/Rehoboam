// GENERATED from specs/systems/S7.vows.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "S7.vows";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Vital),
    (Pulse::Day, Phase::Agenda),
    (Pulse::Month, Phase::Agenda),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    GrudgeForm = 1,  // key: (person, event)
    BreakMode = 2,  // key: (vow, month)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::GrudgeForm => "grudge_form",
            Self::BreakMode => "break_mode",
        }
    }
}
pub struct Params {
    pub vow_priority_base: Fixed,  // bounds checked against the Registry at load
    pub vow_priority_cap: Fixed,  // bounds checked against the Registry at load
    pub max_active_vows: Fixed,  // bounds checked against the Registry at load
    pub strain_rate: Fixed,  // bounds checked against the Registry at load
    pub strain_decay: Fixed,  // bounds checked against the Registry at load
    pub resolve_base: Fixed,  // bounds checked against the Registry at load
    pub resolve_slope: Fixed,  // bounds checked against the Registry at load
    pub grudge_on_harm: Fixed,  // bounds checked against the Registry at load
    pub broken_vow_opinion: Fixed,  // bounds checked against the Registry at load
}
pub const ROLE_ACTOR: rh_core::RoleId = rh_core::RoleId(0);
pub const ROLE_TARGET: rh_core::RoleId = rh_core::RoleId(1);
/// The role constants by Registry name: registration compares each code with the Registry's.
pub const ROLES: &[(&str, rh_core::RoleId)] = &[
    ("actor", ROLE_ACTOR),
    ("target", ROLE_TARGET),
];
pub const ATTR_VOW_PUBLIC: &str = "vow_public";
pub const MOTIVE_VOW_OF: &str = "VOW_OF";
