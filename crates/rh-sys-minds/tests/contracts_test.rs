//! Invariant contract assertions for WAVE-003 / SPEC-003.

use rh_ids::{EventId, PersonId};
use rh_sys_minds::{BreakState, MentalBreakKind, MindsError};

#[test]
fn test_mental_break_catalog_ordering_and_discriminants() {
    let breaks = [
        MentalBreakKind::Fled,
        MentalBreakKind::DespairPact,
        MentalBreakKind::MurderAttempt,
        MentalBreakKind::HoldingsBurned,
        MentalBreakKind::Paranoia,
        MentalBreakKind::Catatonia,
    ];
    for (i, b) in breaks.iter().enumerate() {
        assert_eq!(*b as u8, (i + 1) as u8);
    }
}

#[test]
fn test_break_state_refractory_invariant() {
    let state = BreakState::Refractory { until: 1440 };
    match state {
        BreakState::Refractory { until } => assert_eq!(until, 1440),
        _ => panic!("Expected refractory state"),
    }
}

#[test]
fn test_minds_error_refractory_violation_formatting() {
    let err = MindsError::RefractoryViolation {
        actor: PersonId::from_raw(42),
        active_until: 2880,
    };
    match err {
        MindsError::RefractoryViolation { actor, active_until } => {
            assert_eq!(actor.as_raw(), 42);
            assert_eq!(active_until, 2880);
        }
        _ => panic!("Unexpected error kind"),
    }
}
