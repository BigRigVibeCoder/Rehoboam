//! `rh-sys-demography`: the system `M2.demography` (`specs/systems/M2.demography.odd.toml`,
//! frozen 0.2.0).
//!
//! Plane A, kind `system`. SPR-012 builds the rung-0 slice: ageing and death of founders, no
//! births.
//! - [`schedule`]: the rung-0 mortality schedule in fixed point, `[algorithm.mortality_rung0]`
//!   S0–S9 and S11 (T-05).
//!
//! REF: `CODEX/05_PROJECT/SPR-012_WalkingSkeletonB.md` T-05, T-07; SPR-011 rulings 5–8, 17.
#![forbid(unsafe_code)]

pub mod schedule;
