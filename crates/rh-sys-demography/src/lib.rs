//! `rh-sys-demography`: the system `M2.demography` (`specs/systems/M2.demography.odd.toml`,
//! frozen 0.2.0).
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

pub mod demography;
pub mod schedule;

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
            .map(|p| Self::new(p.e0, None, p.brass_beta, spec.calendar))
            .transpose()
    }
}
