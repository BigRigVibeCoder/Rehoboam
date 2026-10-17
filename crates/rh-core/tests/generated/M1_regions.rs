// GENERATED from specs/systems/M1.regions.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "M1.regions";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
        }
    }
}
pub struct Params {
    pub route_cost_sea: Fixed,  // bounds checked against the Registry at load
    pub route_cost_river: Fixed,  // bounds checked against the Registry at load
    pub route_cost_land: Fixed,  // bounds checked against the Registry at load
}
/// The role constants by Registry name: registration compares each code with the Registry's.
pub const ROLES: &[(&str, rh_core::RoleId)] = &[
];
