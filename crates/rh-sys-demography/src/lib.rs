//! `rh-sys-demography`: the system `M2.demography` (`specs/systems/M2.demography.odd.toml`,
//! frozen; 0.2.1 since EVO-001, which changed no rung-0 mechanics).
//!
//! Plane A, kind `system`. SPR-012 builds the rung-0 slice: ageing and death of founders, no
//! births.
//! - [`schedule`]: the rung-0 mortality schedule in fixed point, `[algorithm.mortality_rung0]`
//!   S0–S9 and S11 (T-05);
//! - [`demography`]: the system, bound to Month/Vital, with its death timers, [`apply_death`] and
//!   its invariants (T-07).
//!
//! REF: `CODEX/05_PROJECT/SPR-012_WalkingSkeletonB.md` T-05, T-07; SPR-011 rulings 5–8, 17.
#![forbid(unsafe_code)]

pub mod contracts;
pub mod demography;
pub mod schedule;

pub use contracts::{DeathDrainOutcome, ScheduleDeathDrain, ScheduleDeathDrainStub};
pub use demography::{Demography, DemographyError, apply_death};

use rh_registry::WorldSpec;

impl Demography {
    /// M2 for a compiled world: its e₀ and β (`WorldSpec::demography`, ruling 9). `None` when the
    /// world does not name M2 or the build lacks it (the compiler then fills no parameters).
    ///
    /// # Errors
    /// [`DemographyError`].
    pub fn from_world(spec: &WorldSpec) -> Result<Option<Self>, DemographyError> {
        spec.demography
            .map(|p| {
                Self::new(
                    p.e0,
                    None,
                    p.brass_beta,
                    p.tfr,
                    p.elite_fertility_mult,
                    spec.calendar,
                )
            })
            .transpose()
    }
}

/// The `mutant-*` Cargo features compiled into this build (BCK-002 A-52): empty in every build a
/// world or a baseline may come from. `rh-cli run` and `bin/regen_baseline.sh` refuse a build where
/// it is not; `python/tests/test_mutant_features.py` checks that every `mutant-*` feature of the
/// crate is listed here.
pub const MUTANTS: &[&str] = &[
    #[cfg(feature = "mutant-resample-hit-k1")]
    "mutant-resample-hit-k1",
    #[cfg(feature = "mutant-resample-day-k0")]
    "mutant-resample-day-k0",
];
