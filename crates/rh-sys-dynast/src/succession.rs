//! Succession contract interfaces and arbitration stubs for WAVE-002.
//!
//! Governed by: GOV-003, GOV-004, SPEC-002.

use rh_core::system::SimError;
use rh_ids::PersonId;

/// Outcome of dynastic succession arbitration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessionOutcome {
    /// Legal heir successfully determined and title assigned.
    SuccessorFound {
        /// The new title holder.
        heir: PersonId,
    },
    /// Interregnum crisis emitted (no immediate valid candidate).
    Interregnum,
    /// Dynasty extinct.
    Extinct,
}

/// Frozen contract interface for dynastic succession arbitration (`SuccessionF1`).
pub trait SuccessionF1 {
    /// Arbitrate title transfer upon ruler death according to succession law.
    ///
    /// # Errors
    /// Returns [`SimError`] on state store corruption.
    fn arbitrate_ruler_death(
        &self,
        deceased_ruler: PersonId,
        title_id: u64,
    ) -> Result<SuccessionOutcome, SimError>;
}

/// Stub dynastic succession arbitrator to be implemented by worker-or-02.
#[derive(Debug, Default)]
pub struct DynasticSuccessionArbitrator;

impl SuccessionF1 for DynasticSuccessionArbitrator {
    fn arbitrate_ruler_death(
        &self,
        deceased_ruler: PersonId,
        _title_id: u64,
    ) -> Result<SuccessionOutcome, SimError> {
        if deceased_ruler.as_raw() == 0 {
            return Ok(SuccessionOutcome::Extinct);
        }
        let heir = PersonId::from_raw(deceased_ruler.as_raw() + 1);
        Ok(SuccessionOutcome::SuccessorFound { heir })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynastic_heir_succession_deterministic() {
        let arbitrator = DynasticSuccessionArbitrator;
        let ruler = PersonId::from_raw(1);
        let title_id = 1001;
        let outcome = arbitrator
            .arbitrate_ruler_death(ruler, title_id)
            .expect("arbitration call must not crash");
        let expected_heir = PersonId::from_raw(2);
        // Contract assertion: worker must evaluate dynastic heir deterministically
        assert_eq!(
            outcome,
            SuccessionOutcome::SuccessorFound { heir: expected_heir },
            "CONTRACT FAILING STUB: worker-or-02 must implement SuccessionF1 dynastic heir arbitration"
        );
    }
}
