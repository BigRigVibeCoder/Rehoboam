// GENERATED from specs/systems/S3.migration.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "S3.migration";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Day, Phase::Vital),
];
pub const DRAINS: &[(&str, Pulse, Phase)] = &[
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
}
/// The role constants by Registry name: registration compares each code with the Registry's.
pub const ROLES: &[(&str, rh_core::RoleId)] = &[
];
