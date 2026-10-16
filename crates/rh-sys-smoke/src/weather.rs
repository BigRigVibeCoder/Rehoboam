//! `X1.smoke_weather` (`worlds/smoke/systems/X1.smoke_weather.odd.toml`): a Q16.16 temperature
//! per region that drifts every hour; the hour count of the year. The smoke world's Hour pulse and
//! its Inputs and Environment phases.
#![allow(
    dead_code,
    reason = "generated skeleton items (Params) a parameterless system does not use"
)]

use rh_core::access::{AccessSpec, ColumnWrite, Plane};
use rh_core::fixed::Q16_16;
use rh_core::system::{InvariantResult, PulseBinding, SimError, StepCtx, SystemModel};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};

use crate::common::{REGIONS, check, degrees, fixed, from_skeleton, purposes, region};

include!("generated/X1_smoke_weather.rs");
purposes!(DriftStep);

const TEMP: &str = "weather.temp";
const HOURS: &str = "weather.hours";
const MIN: i16 = -40;
const MAX: i16 = 60;
const HOURS_MAX: i64 = 24 * 366;

/// The weather system.
pub struct Weather;

impl Weather {
    fn drift(view: &mut SystemView<'_>, ctx: &StepCtx<'_>) -> Result<(), SimError> {
        let mean = Q16_16::from_int(10);
        let (lo, hi) = (Q16_16::from_int(MIN), Q16_16::from_int(MAX));
        let temps = view.write(TEMP)?;
        for (r, bits) in temps.iter_mut().enumerate().take(REGIONS) {
            let t = fixed(SYSTEM_ID, *bits)?;
            let u = ctx.rng(&RngPurpose::DriftStep, region(r), 0)?;
            // A step of (u mod 129 - 64)/256 degrees, minus a pull of (t - 10)/64 toward 10.
            let step = Q16_16::from_bits(i32::try_from(u % 129).unwrap_or(0) * 256 - 64 * 256);
            let pull = Q16_16::from_bits(t.saturating_sub(mean).to_bits() / 64);
            let next = t.saturating_add(step).saturating_sub(pull);
            *bits = i64::from(next.max(lo).min(hi).to_bits());
        }
        view.write(HOURS)?[0] += 1;
        Ok(())
    }
}

impl SystemModel for Weather {
    from_skeleton!();

    fn access(&self) -> AccessSpec {
        AccessSpec {
            writes: [TEMP, HOURS]
                .map(|column| ColumnWrite {
                    column,
                    plane: Plane::A,
                })
                .to_vec(),
            ..AccessSpec::default()
        }
    }

    fn step(
        &self,
        binding: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        match (binding.pulse, binding.phase) {
            (Pulse::Year, Phase::Inputs) => {
                view.write(HOURS)?[0] = 0;
                Ok(())
            }
            _ => Self::drift(view, ctx),
        }
    }

    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        let temps = world.get(TEMP).map_or(&[][..], Vec::as_slice);
        let hours = world
            .get(HOURS)
            .and_then(|h| h.first())
            .copied()
            .unwrap_or(-1);
        vec![
            check(
                "INV-X1-01",
                temps.len() == REGIONS
                    && temps
                        .iter()
                        .all(|&t| (degrees(MIN)..=degrees(MAX)).contains(&t)),
                || format!("temperatures out of [{MIN}, {MAX}]: {temps:?}"),
            ),
            check("INV-X1-02", (0..=HOURS_MAX).contains(&hours), || {
                format!("hour count {hours} out of [0, {HOURS_MAX}]")
            }),
        ]
    }
}
