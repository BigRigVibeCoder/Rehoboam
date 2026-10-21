//! Frozen contract traits and types for WAVE-002 (`SPEC-002`).
//!
//! Governed by: GOV-003, GOV-004, SPEC-002, REV-P1a MAJ-4.

use rh_core::system::SimError;
use rh_events::EventRef;
use rh_ids::PersonId;

/// Outcome of processing a scheduled death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathDrainOutcome {
    /// Death successfully processed and event emitted.
    Processed {
        /// Victim person ID.
        victim: PersonId,
        /// Recorded cause code.
        cause: u32,
    },
    /// Person was already dead at the pulse.
    AlreadyDead,
    /// Pending death queued but not yet drained.
    Pending,
}

/// Frozen contract interface for draining scheduled deaths at Day/`Vital` pulse (`REV-P1a MAJ-4`).
pub trait ScheduleDeathDrain {
    /// Drain pending death commands and emit death events without event loss.
    ///
    /// # Errors
    /// Returns [`SimError`] if state store access fails.
    fn drain_pending_deaths(
        &mut self,
        victim: PersonId,
        cause: u32,
        cause_event: EventRef,
    ) -> Result<DeathDrainOutcome, SimError>;
}

/// Stub implementation to be completed by worker-or-01.
#[derive(Debug, Default)]
pub struct ScheduleDeathDrainStub;

impl ScheduleDeathDrain for ScheduleDeathDrainStub {
    fn drain_pending_deaths(
        &mut self,
        _victim: PersonId,
        _cause: u32,
        _cause_event: EventRef,
    ) -> Result<DeathDrainOutcome, SimError> {
        // Contract stub: fails until worker implements full Day/Vital drain handler
        Ok(DeathDrainOutcome::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vital_day_drain_emits_death_event() {
        let mut drain = ScheduleDeathDrainStub;
        let victim = PersonId::from_raw(42);
        let outcome = drain
            .drain_pending_deaths(victim, 1, EventRef::None)
            .expect("drain call must not crash");
        // Contract assertion: worker must implement drain to produce Processed
        assert_eq!(
            outcome,
            DeathDrainOutcome::Processed { victim, cause: 1 },
            "CONTRACT FAILING STUB: worker-or-01 must implement Day/Vital drain to emit Death event"
        );
    }
}
