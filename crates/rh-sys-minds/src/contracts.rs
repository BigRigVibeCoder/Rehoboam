//! Contracts and frozen interfaces for T1.minds (WAVE-003 / SPEC-003).
//! Governed by: GOV-003, V-SAGA-16, V-SAGA-16b, ADR-0035

use rh_ids::{EventId, PersonId};

/// Fixed-point representation for stress and weights in plane A (q8_8 integer).
pub type StressQ8_8 = u16;

/// Typed mental break catalog (SPEC-003 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MentalBreakKind {
    /// Flee the settlement / court in despair.
    Fled = 1,
    /// Enter into a mutual despair pact.
    DespairPact = 2,
    /// Attempt violence / murder against inciting actor.
    MurderAttempt = 3,
    /// Burn or destroy holdings / grain reserves.
    HoldingsBurned = 4,
    /// Severe paranoia altering diplomatic weights.
    Paranoia = 5,
    /// Complete withdrawal and inactivity.
    Catatonia = 6,
}

/// Actor break state tracking machine (SPEC-003 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakState {
    /// Normal operation; accrues stress.
    Normal,
    /// Stress crossed threshold; delay timer active.
    Pending {
        /// Inciting cause event.
        cause: EventId,
        /// Tick at which break fires.
        fires_at: u64,
    },
    /// In refractory cooldown; zero new breaks permitted.
    Refractory {
        /// Tick until which actor is immune to new breaks.
        until: u64,
    },
}

/// Instant death observation payload for I-step (SPEC-003 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathObservation {
    /// The deceased actor.
    pub deceased: PersonId,
    /// Inciting death event.
    pub event_id: EventId,
    /// Instant tick of death.
    pub tick: u64,
}

/// Outcome of I-step grief saturation evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GriefAccrualOutcome {
    /// Total grief stress points added (q8_8).
    pub grief_applied_q8_8: StressQ8_8,
    /// Motive references created.
    pub motives_logged: u32,
}

/// Outcome of S-step break evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakEvaluationOutcome {
    /// No break triggered; actor remains in normal or refractory state.
    Unchanged { current_stress_q8_8: StressQ8_8 },
    /// Threshold crossed; break scheduled.
    Scheduled {
        kind: MentalBreakKind,
        delay_days: u32,
        scheduled_tick: u64,
        cause_event: EventId,
    },
    /// Break fired; verb program emitted and relief applied.
    Fired {
        kind: MentalBreakKind,
        verb: &'static str,
        relief_applied_q8_8: StressQ8_8,
        refractory_until_tick: u64,
    },
}

/// Explicit typed error domain for minds subsystem (GOV-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MindsError {
    /// Requested actor not registered in state model.
    ActorNotFound(PersonId),
    /// Invalid relationship query.
    InvalidRelation,
    /// Attempted to schedule break while refractory cooldown was active (V-SAGA-16b violation).
    RefractoryViolation { actor: PersonId, active_until: u64 },
    /// Fixed-point accounting equation mismatch (V-SAGA-16 identity drift).
    CorruptedAccounting { expected: u32, actual: u32 },
}

/// Instant observation interface for T1.minds (I-step).
pub trait MindsIStep {
    /// Observe deaths occurring at the current instant and saturate grief per kin bond.
    fn observe_instant_deaths(
        &mut self,
        deaths: &[DeathObservation],
    ) -> Result<GriefAccrualOutcome, MindsError>;
}

/// State evaluation and transition interface for T1.minds (S-step).
pub trait MindsSStep {
    /// Evaluate stress accumulation, check thresholds, and transition break state.
    fn evaluate_stress_and_breaks(
        &mut self,
        actor: PersonId,
        current_tick: u64,
    ) -> Result<BreakEvaluationOutcome, MindsError>;

    /// Assert bit-for-bit accounting invariant identity (V-SAGA-16).
    fn verify_accounting_invariants(
        &self,
        actor: PersonId,
    ) -> Result<(), MindsError>;
}
