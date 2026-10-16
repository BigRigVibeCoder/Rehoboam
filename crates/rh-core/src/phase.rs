//! The phase order inside one instant (SPR-005 T-02).
//!
//! The twelve phases of `05.1` §4.2, numbered 1 to 11 with `5a` (`Activity`, ADR-0031, ADR-0033)
//! between `Vital` and `Contagion`. The order is part of the determinism contract: changing it
//! changes every history, so it is a C3 change and the snapshot test
//! (`tests/phase_table.rs`) pins it.
//!
//! The derived `Ord` is the execution order. [`Phase::index`] is the 0-based position (used in the
//! agenda key `(time, phase, entity_id, seq)` and in `EventId` order); [`Phase::label`] is the
//! number printed in `05.1` (`"5a"` for `Activity`).
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §4.2; ADR-0005 v1.1 item 5; ADR-0033
//! (phase 5a); `CODEX/05_PROJECT/SPR-005_CoreTimeAndScheduler.md` T-02.

use core::fmt;

/// A phase of one instant, in execution order (`05.1` §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Phase {
    /// 1. Apply journaled push inputs; cancel and re-sample timers in each input's scope.
    Inputs,
    /// 2. Climate, seasons, fields, ecology (M3, S4, physical rasters).
    Environment,
    /// 3. Agriculture yield, economy production, settlement growth (M3, M4, M1).
    Production,
    /// 4. Prices, trade, wages, taxes (M4).
    Exchange,
    /// 5. Fire due timers and resolve successions, through lifecycle queues (M2, M7, S3).
    Vital,
    /// 5a. Daily life (S5, ADR-0031): positions, then encounters as plane-A events.
    Activity,
    /// 6. SEIRS steps, testing, fear (M5).
    Contagion,
    /// 7. Legitimacy, elite dynamics, instability, revolts (M6).
    Polity,
    /// 8. Campaign moves, battles, sieges (M8).
    Military,
    /// 9. Named-actor decisions due at this instant (`05.1` §4.3).
    Agenda,
    /// 10. Belief, ideology and technology diffusion (S1, S2).
    Culture,
    /// 11. Invariants, aggregates, plane hashes, keyframe trigger.
    Bookkeeping,
}

impl Phase {
    /// Every phase, in execution order.
    pub const ALL: [Self; 12] = [
        Self::Inputs,
        Self::Environment,
        Self::Production,
        Self::Exchange,
        Self::Vital,
        Self::Activity,
        Self::Contagion,
        Self::Polity,
        Self::Military,
        Self::Agenda,
        Self::Culture,
        Self::Bookkeeping,
    ];

    /// The 0-based execution position.
    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// The number printed in `05.1` §4.2: `"1"` to `"11"`, and `"5a"` for `Activity`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inputs => "1",
            Self::Environment => "2",
            Self::Production => "3",
            Self::Exchange => "4",
            Self::Vital => "5",
            Self::Activity => "5a",
            Self::Contagion => "6",
            Self::Polity => "7",
            Self::Military => "8",
            Self::Agenda => "9",
            Self::Culture => "10",
            Self::Bookkeeping => "11",
        }
    }

    /// The name used in `05.1` and in the system specs (`[[schedule]] phase`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Inputs => "Inputs",
            Self::Environment => "Environment",
            Self::Production => "Production",
            Self::Exchange => "Exchange",
            Self::Vital => "Vital",
            Self::Activity => "Activity",
            Self::Contagion => "Contagion",
            Self::Polity => "Polity",
            Self::Military => "Military",
            Self::Agenda => "Agenda",
            Self::Culture => "Culture",
            Self::Bookkeeping => "Bookkeeping",
        }
    }

    /// The phase with this spec name, if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }

    /// The phase table as text, one `label name` line per phase in execution order: the form
    /// the snapshot test pins.
    #[must_use]
    pub fn table() -> String {
        let mut out = String::new();
        for p in Self::ALL {
            out.push_str(p.label());
            out.push(' ');
            out.push_str(p.name());
            out.push('\n');
        }
        out
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
