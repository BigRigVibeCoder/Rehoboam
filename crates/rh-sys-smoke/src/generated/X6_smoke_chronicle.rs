// GENERATED from worlds/smoke/systems/X6.smoke_chronicle.odd.toml by lint_odd.py --rust. Do not edit; edit the spec.
pub const SYSTEM_ID: &str = "X6.smoke_chronicle";
pub const SPEC_VERSION: &str = "0.1.0";
pub const SCHEDULE: &[(Pulse, Phase)] = &[
    (Pulse::Month, Phase::Agenda),
    (Pulse::Day, Phase::Culture),
    (Pulse::Day, Phase::Bookkeeping),
];
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RngPurpose {
    NoteTone = 1,  // key: (world, month)
}
impl RngPurpose {
    /// The purpose's name in the spec's `[[rng]]` table.
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoteTone => "note_tone",
        }
    }
}
pub struct Params {
}
